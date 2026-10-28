//! 入力を開いて 1 単位を集計する。**I/O の境界はここ。**
//!
//! [`crate::aggregate`]（層 2）はファイルを知らないので、
//! **「ジョブの中身」はここが受け持つ。**
//! 逆にこのモジュールは `clap` を知らない — 引数の形とは独立している。
//!
//! # なぜ `main.rs` ではないのか
//!
//! **ベンチとテストから呼べないため。** `main.rs` の項目は binary クレートの中にあり、
//! `benches/` からも `tests/` からも見えない。`main.rs` がロジックを持たない
//! という規約（`crates/tally/docs/layout.md`）の具体的な効き方がこれである。

use std::fs::File;
use std::io::{self, BufRead, BufReader, Stdin, StdinLock};
use std::path::{Path, PathBuf};

use tally_core::{Counter, Selector, TallyError, tally_reader};

use crate::error::{CliError, CliErrorKind, InputName, hint_for};

/// 集計の出どころ。**開き方と呼び名だけを持つ。**
///
/// 読み手の型を **関連型**にしているのは、**1 つの出どころに読み手が 1 つ**だから
/// （[ADR-0012] 論点 4）。型パラメータ（`trait Input<R: BufRead>`）にすると
/// 同じ出どころに複数の読み手を実装でき、**2 つ目の実装を足した瞬間に
/// 呼び出し側の推論が `E0283` で落ちる。** 関連型なら 2 つ目をそもそも書けない。
///
/// **関連型を 2 つ（`type Error`）にはしない。** 標準入力の [`Input::open`] は
/// 決して失敗しないが、[ADR-0012] 論点 4e がその形を却下した。
///
/// **退出条件**: 実装が 1 つに減ったら、この trait を畳んで直書きに戻す
/// （[ADR-0012] 論点 3）。
///
/// [ADR-0012]: ../../../docs/adr/0012-stage-10-associated-types.md
pub trait Input {
    /// この出どころから読む型。**実装ごとに別の具体型に解決する。**
    ///
    /// `Box<dyn BufRead>` を通さないので、[`tally_input`] は出どころごとに
    /// 単相化され、行ごとの間接呼び出しが残らない。
    type Reader: BufRead;

    /// 読み手を用意する。**ここが唯一の「開く」操作。**
    fn open(&self) -> Result<Self::Reader, CliError>;

    /// 診断に出す呼び名。
    ///
    /// **確保を伴いうる**ので、[`tally_input`] は失敗の経路でしか呼ばない。
    fn name(&self) -> InputName;
}

/// **newtype を挟まず `std::path::Path` にそのまま実装する。**
/// 孤児ルールは `Input` が自クレートの trait なので満たす。
/// 包む型を足しても、`Path` が既に持っている意味が増えない。
///
/// **`PathBuf` ではなく `Path` 側**なので [`tally_input`] に `?Sized` が要るが、
/// 代わりに `&Path` と `&PathBuf`（`as_path()`）の両方から呼べる。
impl Input for Path {
    type Reader = BufReader<File>;

    fn open(&self) -> Result<Self::Reader, CliError> {
        // **開くのはここ。** `tally_core` はファイルを開かないので、
        // 開けなかった失敗も CLI の型になる（ADR-0007 論点 3）。
        File::open(self)
            .map(BufReader::new)
            .map_err(|source| open_error(self, source))
    }

    fn name(&self) -> InputName {
        InputName::Path(self.to_path_buf())
    }
}

/// 標準入力を丸ごと集計する。**並列化しない** — 単位が 1 つしかない。
///
/// **こちらも newtype を挟まない。** 出どころを表す型が std にあるので、
/// `StdinInput` のような包む型を足す理由が無い。
impl Input for Stdin {
    type Reader = StdinLock<'static>;

    /// **決して失敗しない。** それでも `Result` を返すのは、
    /// 「失敗しない」を型に出す案（`type Error = Infallible`）を
    /// [ADR-0012] 論点 4e が却下したため。`Into<CliError>` 境界を入れると
    /// `?` が自動で包む経路が戻り、**「この失敗に対して利用者は何ができるか」を
    /// 問う瞬間が消える**（ADR-0004 論点 6）。
    ///
    /// [ADR-0012]: ../../../docs/adr/0012-stage-10-associated-types.md
    fn open(&self) -> Result<Self::Reader, CliError> {
        // **ロックが `'static` なのは、`Stdin` が大域にあるため。**
        // 借用元の寿命に縛られないので、関連型に寿命引数が要らない。
        Ok(self.lock())
    }

    fn name(&self) -> InputName {
        InputName::Stdin
    }
}

/// ファイルを開いて丸ごと集計する。
///
/// `counter` は方針（`strict`）を持った空のカウンタ。
/// **失敗には必ず入力の名前が乗る**（[ADR-0007] 論点 3）。
///
/// **出どころごとに単相化される。** 読み手の型は [`Input::Reader`] から来るので、
/// `Box<dyn BufRead>` を経由しない（[ADR-0012] 論点 4）。
///
/// [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md
/// [ADR-0012]: ../../../docs/adr/0012-stage-10-associated-types.md
pub fn tally_input<I, F>(
    input: &I,
    mut counter: Counter,
    selector: &Selector,
    keep: F,
) -> Result<Counter, CliError>
where
    // **`?Sized` は `Path` のため。** 既定の `Sized` 境界のままでは、
    // unsized な `Path` に実装しても `&Path` を渡せない。
    I: Input + ?Sized,
    F: Fn(&str) -> bool,
{
    let reader = input.open()?;
    tally_reader(&mut counter, reader, selector, keep)
        .map_err(|source| input_error(input.name(), source))?;
    Ok(counter)
}

/// 開けなかった失敗。
///
/// **hint は付けない。** 開けない理由（パス・権限）に対して、
/// `tally` の使い方を変えてできることは無い。
fn open_error(path: &Path, source: io::Error) -> CliError {
    CliError::new(
        CliErrorKind::Open {
            path: PathBuf::from(path),
            source,
        },
        None,
    )
}

/// 集計の失敗に入力の名前を被せる。
///
/// **包む責任は呼び出し側にある**（ADR-0007 論点 3）。包み忘れても型検査は通るので、
/// **`tally_reader` を呼ぶ経路をこのモジュールに集約する。**
///
/// **hint もここで打つ。** `From<TallyError> for CliError` を実装して `?` に任せると、
/// 「この失敗に対して利用者は何ができるか」を問う瞬間が消える（ADR-0004 論点 6）。
fn input_error(name: InputName, source: TallyError) -> CliError {
    let hint = hint_for(&source);
    CliError::new(CliErrorKind::Input { name, source }, hint)
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::fs;

    use tally_core::Key;
    use tempfile::TempDir;

    use super::*;

    /// 1 ファイルだけの作業場。**`TempDir` を返すのは、落とすと中身が消えるため。**
    fn 一時ファイル(body: &str) -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("一時ディレクトリを作れる");
        let path = dir.path().join("a.log");
        fs::write(&path, body).expect("書ける");
        (dir, path)
    }

    fn 行全体() -> Selector {
        Selector::new(Key::WholeLine)
    }

    #[test]
    fn 読み手の型は実装ごとに別物に解決する() {
        // **束縛の型注釈がコンパイル時の主張。** 関連型が別の型に解決していれば、
        // この 2 行は型検査で落ちる。
        // **値を作らないのは、標準入力を開くと大域のロックを取るため。**
        let _: fn(&Path) -> Result<BufReader<File>, CliError> = <Path as Input>::open;
        let _: fn(&Stdin) -> Result<StdinLock<'static>, CliError> = <Stdin as Input>::open;

        // **上の 2 行は「それぞれがこの型だ」としか言っていない。**
        // 2 つが別物であることは、同一性を比べて初めて示せる。
        // 同じ型に解決していたら、関連型にした意味が無い。
        assert_ne!(
            TypeId::of::<<Path as Input>::Reader>(),
            TypeId::of::<<Stdin as Input>::Reader>(),
            "2 つの実装が同じ読み手に解決している"
        );
    }

    #[test]
    fn ファイルを開いて集計する() {
        let (_dir, path) = 一時ファイル("a\nb\na\n");

        let counter =
            tally_input(path.as_path(), Counter::new(), &行全体(), |_| true).expect("集計できる");

        let report = counter.report(None);
        assert_eq!(report.total, 3);
        assert_eq!(report.entries[0].key, "a");
        assert_eq!(report.entries[0].count, 2);
    }

    #[test]
    fn 述語で落とした行は集計に入らない() {
        let (_dir, path) = 一時ファイル("a\nb\na\n");

        let counter = tally_input(path.as_path(), Counter::new(), &行全体(), |line| {
            line == "a"
        })
        .expect("集計できる");

        let report = counter.report(None);
        // **落ちた行は total にも skipped にも入らない**（`tally_reader` の契約）。
        assert_eq!(report.total, 2);
        assert_eq!(report.skipped, 0);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].key, "a");
    }

    #[test]
    fn 開けない入力は_open_の失敗になる() {
        // 開けなかった失敗は `Input` を通らない（ADR-0007 論点 3）。
        let missing = Path::new("/存在しないはずのディレクトリ/a.log");

        let err =
            tally_input(missing, Counter::new(), &行全体(), |_| true).expect_err("開けないはず");

        match err.kind() {
            CliErrorKind::Open { path, .. } => assert_eq!(path, missing),
            other => panic!("開けない失敗を期待した: {other:?}"),
        }
    }

    #[test]
    fn 集計の失敗には開いた入力の名前が乗る() {
        // **名前は `Input::name` から来る。** 統合前は関数ごとに直書きしていた。
        let (_dir, path) = 一時ファイル("{\"other\":1}\n");
        let selector = Selector::new(Key::JsonField("lvl".to_owned()));

        let err = tally_input(
            path.as_path(),
            Counter::new().strict(true),
            &selector,
            |_| true,
        )
        .expect_err("strict なので失敗するはず");

        match err.kind() {
            CliErrorKind::Input { name, .. } => {
                assert_eq!(name, &InputName::Path(path.clone()));
            }
            other => panic!("集計の失敗を期待した: {other:?}"),
        }
    }

    #[test]
    fn 標準入力の呼び名は開かずに取れる() {
        // `name` は失敗の経路でしか要らないので、開く操作と独立している。
        assert_eq!(io::stdin().name(), InputName::Stdin);
    }
}
