//! 11 章の演習の答え。
//!
//! `cargo test -p book-ch11` で確かめられる。

use std::error::Error;
use std::num::ParseIntError;

/// 演習 1: 状態コードとして読めなかった理由。
///
/// 数として読めない場合は `ParseIntError` を原因に持つ。`#[from]` なので `?` が包む。
/// 範囲の外の場合は原因を持たない。値は表示に埋め込む。
#[derive(Debug, thiserror::Error)]
enum CodeError {
    #[error("数として読めません")]
    NotNumber(#[from] ParseIntError),
    #[error("{0} は 100 から 599 の範囲にありません")]
    OutOfRange(u16),
}

/// 演習 1: 欄の値を状態コードとして読む。
fn parse_code(value: &str) -> Result<u16, CodeError> {
    let code: u16 = value.parse()?;
    if (100..=599).contains(&code) {
        Ok(code)
    } else {
        Err(CodeError::OutOfRange(code))
    }
}

/// 演習 2: 原因の連鎖をいちばん奥まで辿る。原因が無ければ `err` 自身を返す。
///
/// `'a` を書くのは、引数に寿命が 2 つあるからである。
/// 外側の `&` の寿命と、`dyn Error + 'static` の `'static` で、省略規則はどちらに結ぶかを決めない。
fn root_cause<'a>(err: &'a (dyn Error + 'static)) -> &'a (dyn Error + 'static) {
    let mut current = err;
    while let Some(source) = current.source() {
        current = source;
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CountError, count_codes};

    #[test]
    fn 範囲の中の数は読める() {
        assert_eq!(parse_code("100").ok(), Some(100));
        assert_eq!(parse_code("599").ok(), Some(599));
    }

    #[test]
    fn 数でなければ原因を持つ() {
        let err = parse_code("x").expect_err("数でない");
        assert!(matches!(err, CodeError::NotNumber(_)));
        assert_eq!(err.to_string(), "数として読めません");
        let source = err.source().expect("ParseIntError を原因に持つ");
        assert_eq!(source.to_string(), "invalid digit found in string");
    }

    #[test]
    fn 範囲の外なら値を表示し原因を持たない() {
        for (value, code) in [("99", 99), ("600", 600)] {
            let err = parse_code(value).expect_err("範囲の外");
            assert!(matches!(err, CodeError::OutOfRange(c) if c == code));
            assert_eq!(
                err.to_string(),
                format!("{code} は 100 から 599 の範囲にありません")
            );
            assert!(err.source().is_none());
        }
    }

    #[test]
    fn u16_に収まらない数は範囲の外ではなく数でない() {
        // 範囲を調べる前に、u16 への変換で失敗する。
        let err = parse_code("70000").expect_err("u16 に収まらない");
        assert!(matches!(err, CodeError::NotNumber(_)));
    }

    #[test]
    fn いちばん奥の原因まで辿る() {
        let err = count_codes("code=x\n".as_bytes()).expect_err("数でない");
        let root = root_cause(&err);
        assert!(root.downcast_ref::<ParseIntError>().is_some());
        assert_eq!(root.to_string(), "invalid digit found in string");
    }

    #[test]
    fn 原因が無ければ自分自身を返す() {
        let err = count_codes("level=warn\n".as_bytes()).expect_err("欄が無い");
        let root = root_cause(&err);
        assert!(root.downcast_ref::<CountError>().is_some());
        assert_eq!(root.to_string(), "1 行目: 欄 `code` がありません");
    }

    #[test]
    fn 読み取りの失敗は_io_error_まで辿る() {
        let err = count_codes(&b"\xff\n"[..]).expect_err("UTF-8 でない");
        let root = root_cause(&err);
        assert!(root.downcast_ref::<std::io::Error>().is_some());
    }
}
