//! ファイル単位の並列化を実測する。
//!
//! **合格条件と予測は `docs/curriculum.md` 段階 6 に、測る前に書いてある。**
//! 結果は `docs/stage-log.md` に書く。**このファイルに数値を書かない。**
//!
//! # 測り方の固定
//!
//! - **総行数を固定し、ファイル数だけを変える。** 仕事量とファイル数を同時に
//!   動かすと、何を測ったのか分からなくなる
//! - 逐次版と並列版は **同じ入力**に対して走らせ、同じ関数
//!   （[`tally::aggregate::aggregate_all`]）に [`Execution`] を変えて渡す。
//!   **測っている差が実行戦略だけであることを、構造で保証する**
//! - **ページキャッシュは温まった状態**である（同じファイルを繰り返し読む）。
//!   つまり **I/O ではなくメモリの読み出しと CPU を測っている。**
//!   冷えた状態の測定は `divan` の反復の中では作れない（curriculum の
//!   「検討する価値のあるもの」を参照）
//!
//! # 予測（curriculum の表と対応）
//!
//! | ベンチ | 予測 |
//! | --- | --- |
//! | `json` の 1 ファイル | B1: 速くならない（分割されない） |
//! | `json` の 10 ファイル | B2: 速くなる（陽性対照） |
//! | `unique` | B3: B2 より伸びない（合流の費用が効く） |
//! | `tiny` | B4: B2 より伸びない（open/close と分配が効く） |

// **`tests/` と同じ理由で明示的に許可する。** `benches/` は `#[cfg(test)]` ではないので、
// `clippy.toml` の `allow-expect-in-tests` が効かない。
#![allow(clippy::expect_used)]

use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use divan::Bencher;
use tally::aggregate::{Execution, aggregate_all};
use tally::input::tally_path;
use tally_core::{Counter, Key, Selector};

fn main() {
    divan::main();
}

/// 総行数。**ファイル数を変えてもこれは変えない。**
const TOTAL_LINES: usize = 20_000;

/// キーの散らばり方。合流（`Counter::merge`）の費用を左右する。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Keys {
    /// 4 種類だけ。合流は安い。
    Few,
    /// 全行で異なる。**合流が集計と同じ桁の費用になる。**
    Unique,
}

impl fmt::Display for Keys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Few => f.write_str("few"),
            Self::Unique => f.write_str("unique"),
        }
    }
}

/// 生成した入力。**プロセスの生存期間だけ存在する。**
struct Corpus {
    dir: tempfile::TempDir,
}

fn corpus() -> &'static Corpus {
    static CORPUS: OnceLock<Corpus> = OnceLock::new();
    CORPUS.get_or_init(|| Corpus {
        dir: tempfile::tempdir().expect("一時ディレクトリを作れる"),
    })
}

/// `files` 個のファイルに `lines` 行を分けて書き、そのパスを返す。
///
/// **同じ組み合わせなら作り直さない**（測定のたびに書き直すと、
/// 測っているものにファイル生成が混ざる）。
fn make_inputs(files: usize, lines: usize, keys: Keys) -> Vec<PathBuf> {
    let dir = corpus().dir.path().join(format!("{files}-{lines}-{keys}"));
    let paths: Vec<PathBuf> = (0..files).map(|i| dir.join(format!("{i}.log"))).collect();

    if !dir.exists() {
        fs::create_dir_all(&dir).expect("作れる");
        let per_file = lines.div_ceil(files);
        for (index, path) in paths.iter().enumerate() {
            let mut text = String::new();
            for line in 0..per_file {
                let serial = index * per_file + line;
                match keys {
                    Keys::Few => {
                        let level = ["info", "warn", "error", "debug"][serial % 4];
                        writeln!(text, "{{\"lvl\":\"{level}\"}}")
                            .expect("String への書き込みは失敗しない");
                    }
                    Keys::Unique => {
                        writeln!(text, "{{\"lvl\":\"k{serial}\"}}")
                            .expect("String への書き込みは失敗しない");
                    }
                }
            }
            fs::write(path, text).expect("書ける");
        }
    }
    paths
}

/// 入力を集計する。**ベンチの本体はこれだけ。**
fn run(paths: &[PathBuf], execution: Execution) {
    let selector = Selector::new(Key::JsonField("lvl".to_owned()));
    let jobs: Vec<_> = paths
        .iter()
        .map(|path| {
            let selector = &selector;
            move || tally_path(path, Counter::new(), selector, |_| true)
        })
        .collect();

    let counter = aggregate_all(&jobs, Counter::new, execution).expect("集計できる");
    divan::black_box(counter.report(None));
}

/// 総行数を固定して、ファイル数だけを変える（B1 / B2 / B4 の一部）。
#[divan::bench(args = [1, 10, 1000], sample_count = 20)]
fn 逐次(bencher: Bencher, files: usize) {
    let paths = make_inputs(files, TOTAL_LINES, Keys::Few);
    bencher.bench_local(|| run(&paths, Execution::Sequential));
}

#[divan::bench(args = [1, 10, 1000], sample_count = 20)]
fn 並列(bencher: Bencher, files: usize) {
    let paths = make_inputs(files, TOTAL_LINES, Keys::Few);
    bencher.bench_local(|| run(&paths, Execution::Parallel { threads: None }));
}

/// キーが全行で異なる場合（B3）。合流の費用が効く。
#[divan::bench(args = [10], sample_count = 20)]
fn 逐次_キーが全行で異なる(bencher: Bencher, files: usize) {
    let paths = make_inputs(files, TOTAL_LINES, Keys::Unique);
    bencher.bench_local(|| run(&paths, Execution::Sequential));
}

#[divan::bench(args = [10], sample_count = 20)]
fn 並列_キーが全行で異なる(bencher: Bencher, files: usize) {
    let paths = make_inputs(files, TOTAL_LINES, Keys::Unique);
    bencher.bench_local(|| run(&paths, Execution::Parallel { threads: None }));
}

/// 1 ファイルあたり数行しかない場合（B4）。open/close と分配が効く。
#[divan::bench(sample_count = 20)]
fn 逐次_小さなファイル_1000_個(bencher: Bencher) {
    let paths = make_inputs(1000, 5_000, Keys::Few);
    bencher.bench_local(|| run(&paths, Execution::Sequential));
}

#[divan::bench(sample_count = 20)]
fn 並列_小さなファイル_1000_個(bencher: Bencher) {
    let paths = make_inputs(1000, 5_000, Keys::Few);
    bencher.bench_local(|| run(&paths, Execution::Parallel { threads: None }));
}
