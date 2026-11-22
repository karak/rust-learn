//! 11 章の実行ファイル。
//!
//! `lib.rs` とは別のクレートなので、`#[non_exhaustive]` の付いた `LineErrorKind` を
//! `match` するには `_` の腕が要る。

use book_ch11::{CountError, LineErrorKind, count_codes, one_line};

fn main() {
    match count_codes("code=200\ncode=404 path=/a\ncode=200\n".as_bytes()) {
        Ok(counts) => println!("{counts:?}"),
        Err(err) => println!("{}", one_line(&err)),
    }

    let inputs: [&[u8]; 3] = [
        b"code=200\nlevel=warn\n",
        b"code=200\ncode=x\n",
        b"code=200\n\xff\n",
    ];
    for input in inputs {
        if let Err(err) = count_codes(input) {
            println!("{}", one_line(&err));
            println!("  {err:?}");
            println!("  分類: {}", classify(&err));
        }
    }
}

/// 失敗を分類する。
///
/// `CountError` は閉じているので、2 つのバリアントを挙げれば網羅になる。
/// `LineErrorKind` は開いているので、全部を挙げても `_` が要る。
fn classify(err: &CountError) -> &'static str {
    match err {
        CountError::Read(_) => "読み取り",
        CountError::Line(line) => match line.kind {
            LineErrorKind::MissingCode => "欄が無い",
            LineErrorKind::BadCode(_) => "数でない",
            _ => "その他の行の失敗",
        },
    }
}
