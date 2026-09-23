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
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use tally_core::{Counter, Selector, TallyError, tally_reader};

use crate::error::{CliError, CliErrorKind, InputName, hint_for};

/// ファイルを開いて丸ごと集計する。
///
/// `counter` は方針（`strict`）を持った空のカウンタ。
/// **失敗には必ず入力の名前が乗る**（[ADR-0007] 論点 3）。
///
/// [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md
pub fn tally_path<F>(
    path: &Path,
    mut counter: Counter,
    selector: &Selector,
    keep: F,
) -> Result<Counter, CliError>
where
    F: Fn(&str) -> bool,
{
    // **開くのはここ。** `tally_core` はファイルを開かないので、
    // 開けなかった失敗も CLI の型になる（ADR-0007 論点 3）。
    let file = File::open(path).map_err(|source| open_error(path, source))?;
    tally_reader(&mut counter, BufReader::new(file), selector, keep)
        .map_err(|source| input_error(InputName::Path(path.to_path_buf()), source))?;
    Ok(counter)
}

/// 標準入力を丸ごと集計する。**並列化しない** — 単位が 1 つしかない。
pub fn tally_stdin<F>(
    mut counter: Counter,
    selector: &Selector,
    keep: F,
) -> Result<Counter, CliError>
where
    F: Fn(&str) -> bool,
{
    let stdin = io::stdin();
    tally_reader(&mut counter, stdin.lock(), selector, keep)
        .map_err(|source| input_error(InputName::Stdin, source))?;
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
