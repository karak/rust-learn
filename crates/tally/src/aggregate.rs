//! 並列化ポリシー。**入力の種類も集計の中身も知らない。**
//!
//! ここが答えるのは 1 つの問いだけ —
//! **独立したジョブの列をどう走らせ、どう畳み、どの失敗を報告するか。**
//! ジョブが何を読むか（ファイル・標準入力・メモリ上の文字列）は関心の外にある。
//!
//! | 問い | 置き場所 |
//! | --- | --- |
//! | 1 単位をどう集計するか | [`tally_core::tally_reader`] |
//! | 集計をどう合流させるか | [`tally_core::Counter::merge`] |
//! | **単位の列をどう走らせ、どう畳むか** | **このモジュール** |
//!
//! # 切り出せる状態を保つ
//!
//! このモジュールは `tally` の中にあるが、**CLI の関心事を持たない。**
//! `clap` も `regex` も知らず、ファイルも開かない。
//! **`tally` 以外の消費者が現れたら、別クレートへ移せる**
//! （移行の条件は [ADR-0007] 論点 5）。
//!
//! **「移せるように書く」は守り忘れられるので、検査にしてある** —
//! `scripts/check-module-deps.sh` が CI で回る。規則の正本は
//! `crates/tally/docs/layout.md` の層の表。
//!
//! [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md

use std::num::NonZeroUsize;

use rayon::prelude::*;
use tally_core::Counter;

use crate::error::{CliError, CliErrorKind};

/// ジョブの走らせ方。
///
/// **trait ではなく enum である。** 戦略の実装は 2 つともこのクレートにあり、
/// 外部の利用者が独自の戦略を差す必要はまだ無い。
/// enum なら [`aggregate_all`] の `match` が網羅性検査を受けるので、
/// 戦略を足したときに実装漏れがコンパイルエラーになる（[ADR-0007] 論点 5）。
///
/// **コンパイル時の構成（feature flag）にしていない。**
/// 実行時の値なら **両方が常にコンパイルされ、常にテストされる。**
///
/// [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Execution {
    /// 1 つずつ順に走らせる。**`rayon` を経由しない。**
    Sequential,
    /// `rayon` で並列に走らせる。
    Parallel {
        /// スレッド数。`None` なら rayon の既定（論理コア数、
        /// または環境変数 `RAYON_NUM_THREADS`）。
        ///
        /// **`Some` のときは専用のスレッドプールを立てる。**
        /// 大域のプールを設定する形（`build_global`）にしないのは、
        /// 一度しか呼べず、**測定のために 2 通りを続けて走らせられない**ため。
        threads: Option<NonZeroUsize>,
    },
}

/// ジョブの列を 1 つの集計に畳む。
///
/// `new_counter` は **合流の単位元**を作る。`Counter` の方針（`strict`）を
/// 呼び出し側が決めるため、`Counter::new()` を内部で作らない
/// （[`tally_core::tally_reader`] が `counter` を受け取るのと同じ理由）。
///
/// # どの失敗を報告するか
///
/// **引数順で最も左の失敗**を返す（[ADR-0007] 論点 4）。
/// 並列に走らせると「最初に見つかった失敗」は実行ごとに変わりうるので、
/// **順序を利用者に見える形（引数の順番）で決めている。**
///
/// **[`Execution::Sequential`] は短絡してよい。** 最初に出会う失敗が
/// 最小添字の失敗と一致するためで、**両者は同じ失敗を返す。**
/// [`Execution::Parallel`] は全ジョブを走らせてから選ぶ。
///
/// # 部分的な集計は返らない
///
/// 1 つでも失敗すれば `Err` になる。それまでに合流した分は捨てられる。
///
/// [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md
pub fn aggregate_all<J, N>(
    jobs: &[J],
    new_counter: N,
    execution: Execution,
) -> Result<Counter, CliError>
where
    J: Fn() -> Result<Counter, CliError> + Sync,
    N: Fn() -> Counter + Sync,
{
    match execution {
        Execution::Sequential => {
            let mut merged = new_counter();
            for job in jobs {
                // **`?` で短絡する。** 引数順に走っているので、
                // 最初に出会う失敗が「最も左の失敗」である。
                merged.merge(job()?);
            }
            Ok(merged)
        }
        Execution::Parallel { threads: None } => merge_in_order(run_parallel(jobs), new_counter),
        Execution::Parallel {
            threads: Some(threads),
        } => {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads.get())
                .build()
                .map_err(|source| CliError::new(CliErrorKind::threads(threads, source), None))?;
            // **`install` の中で並列イテレータを回す**と、このプールのスレッドが使われる。
            merge_in_order(pool.install(|| run_parallel(jobs)), new_counter)
        }
    }
}

/// 全ジョブを並列に走らせ、**引数順のまま**結果を並べる。
///
/// **`try_reduce` を使わない。** 短絡するが、rayon は分割点が非決定的なので
/// 「どの失敗が返るか」が実行ごとに変わりうる（[ADR-0007] 論点 4）。
///
/// [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md
fn run_parallel<J>(jobs: &[J]) -> Vec<Result<Counter, CliError>>
where
    J: Fn() -> Result<Counter, CliError> + Sync,
{
    jobs.par_iter().map(|job| job()).collect()
}

/// 添字の小さい順に合流する。**最初に見つかった `Err` が「最も左の失敗」。**
fn merge_in_order<N>(
    results: Vec<Result<Counter, CliError>>,
    new_counter: N,
) -> Result<Counter, CliError>
where
    N: Fn() -> Counter,
{
    let mut merged = new_counter();
    for result in results {
        merged.merge(result?);
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tally_core::{Key, Selector, tally_reader};

    use crate::error::{CliErrorKind, InputName};

    const BOTH: [Execution; 3] = [
        Execution::Sequential,
        Execution::Parallel { threads: None },
        // **スレッド数を指定した経路も必ず通す。** 専用プールを立てる枝が
        // 検査されないと、`-j N` が黙って壊れる。
        Execution::Parallel {
            threads: NonZeroUsize::new(2),
        },
    ];

    /// メモリ上の文字列を 1 単位として集計するジョブ。
    ///
    /// **ファイルを作らない。** 並列化ポリシーは入力の種類を知らないので、
    /// テストも入力を作らずに書ける（ADR-0007 論点 5 が 5d を採った理由）。
    fn job(input: &'static str) -> impl Fn() -> Result<Counter, CliError> + Sync {
        move || {
            let mut counter = Counter::new();
            tally_reader(
                &mut counter,
                input.as_bytes(),
                &Selector::new(Key::WholeLine),
                |_| true,
            )
            .map_err(|source| {
                CliError::new(
                    CliErrorKind::Input {
                        name: InputName::Stdin,
                        source,
                    },
                    None,
                )
            })?;
            Ok(counter)
        }
    }

    /// 必ず失敗するジョブ。失敗した入力の名前で、どれが報告されたかを見分ける。
    fn failing(name: &'static str) -> impl Fn() -> Result<Counter, CliError> + Sync {
        move || {
            let mut counter = Counter::new().strict(true);
            tally_reader(
                &mut counter,
                "{\"other\":1}\n".as_bytes(),
                &Selector::new(Key::JsonField("lvl".to_owned())),
                |_| true,
            )
            .map_err(|source| {
                CliError::new(
                    CliErrorKind::Input {
                        name: InputName::Path(name.into()),
                        source,
                    },
                    None,
                )
            })?;
            Ok(counter)
        }
    }

    fn failed_input(err: &CliError) -> String {
        match err.kind() {
            CliErrorKind::Input { name, .. } => name.to_string(),
            other => panic!("集計の失敗を期待した: {other:?}"),
        }
    }

    #[test]
    fn ジョブの集計が合流する() {
        for execution in BOTH {
            let jobs = [job("a\nb\n"), job("a\nc\n")];
            let counter = aggregate_all(&jobs, Counter::new, execution).expect("成功するはず");

            let report = counter.report(None);
            assert_eq!(report.total, 4, "{execution:?}");
            assert_eq!(report.entries[0].key, "a", "{execution:?}");
            assert_eq!(report.entries[0].count, 2, "{execution:?}");
        }
    }

    #[test]
    fn 逐次と並列は同じ結果を返す() {
        let jobs = [job("a\nb\na\n"), job("c\na\n"), job("b\n")];
        let sequential = aggregate_all(&jobs, Counter::new, Execution::Sequential)
            .expect("成功するはず")
            .report(None);
        let parallel = aggregate_all(&jobs, Counter::new, Execution::Parallel { threads: None })
            .expect("成功するはず")
            .report(None);

        assert_eq!(sequential, parallel);
    }

    #[test]
    fn ジョブが無ければ空の集計になる() {
        for execution in BOTH {
            let jobs: [fn() -> Result<Counter, CliError>; 0] = [];
            let report = aggregate_all(&jobs, Counter::new, execution)
                .expect("成功するはず")
                .report(None);
            assert_eq!(report.total, 0, "{execution:?}");
            assert!(report.entries.is_empty(), "{execution:?}");
        }
    }

    #[test]
    fn 報告されるのは引数順で最初の失敗() {
        for execution in BOTH {
            let jobs = [failing("first.log"), failing("second.log")];
            let err = aggregate_all(&jobs, Counter::new, execution).expect_err("失敗するはず");
            assert_eq!(failed_input(&err), "first.log", "{execution:?}");
        }
    }

    #[test]
    fn 引数の順序を入れ替えると報告も入れ替わる() {
        // 「常に同じものが出る」だけでは、規則が **引数順** であることを示せない。
        for execution in BOTH {
            let jobs = [failing("second.log"), failing("first.log")];
            let err = aggregate_all(&jobs, Counter::new, execution).expect_err("失敗するはず");
            assert_eq!(failed_input(&err), "second.log", "{execution:?}");
        }
    }

    #[test]
    fn 成功と失敗が混ざっても失敗する() {
        // 部分的な集計は返らない（ADR-0007 論点 4）。
        for execution in BOTH {
            let jobs: [Box<dyn Fn() -> Result<Counter, CliError> + Sync>; 3] = [
                Box::new(job("a\n")),
                Box::new(failing("broken.log")),
                Box::new(job("b\n")),
            ];
            let err = aggregate_all(&jobs, Counter::new, execution).expect_err("失敗するはず");
            assert_eq!(failed_input(&err), "broken.log", "{execution:?}");
        }
    }

    proptest::proptest! {
        /// **分割のしかたによらず、逐次と並列と一括が一致する**（段階 6 の完了条件）。
        ///
        /// 例を数個並べるだけでは、マージの結合則の破れを拾えない。
        /// **ジョブが閉包なので、ファイルを作らずに性質テストが書ける**
        /// （ADR-0007 論点 5 が 5d を採った実利）。
        #[test]
        fn 分割のしかたによらず結果は同じ(
            groups in proptest::collection::vec(
                proptest::collection::vec("[a-c]{1,2}", 0..5usize),
                0..5usize,
            )
        ) {
            let texts: Vec<String> = groups
                .iter()
                .map(|lines| lines.iter().map(|line| format!("{line}\n")).collect())
                .collect();

            let jobs: Vec<_> = texts
                .iter()
                .map(|text| {
                    move || {
                        let mut counter = Counter::new();
                        tally_reader(
                            &mut counter,
                            text.as_bytes(),
                            &Selector::new(Key::WholeLine),
                            |_| true,
                        )
                        .expect("メモリ上の入力は読み取りに失敗しない");
                        Ok(counter)
                    }
                })
                .collect();

            // 一括で集計した基準。
            let joined: String = texts.concat();
            let mut whole = Counter::new();
            tally_reader(
                &mut whole,
                joined.as_bytes(),
                &Selector::new(Key::WholeLine),
                |_| true,
            )
            .expect("メモリ上の入力は読み取りに失敗しない");
            let expected = whole.report(None);

            for execution in BOTH {
                let actual = aggregate_all(&jobs, Counter::new, execution)
                    .expect("成功するはず")
                    .report(None);
                proptest::prop_assert_eq!(&actual, &expected, "{:?} で結果が変わった", execution);
            }
        }
    }

    #[test]
    fn 単位元の方針が合流後に残る() {
        for execution in BOTH {
            let jobs = [job("a\n")];
            let counter = aggregate_all(&jobs, || Counter::new().strict(true), execution)
                .expect("成功するはず");

            // strict な単位元に合流したので、その後の行は厳格に扱われる。
            let mut counter = counter;
            counter
                .push_line(
                    &Selector::new(Key::JsonField("lvl".to_owned())),
                    "{\"other\":1}",
                    1,
                )
                .expect_err("strict が残っているはず");
        }
    }
}
