//! 11 章「エラー型の設計と公開 API」の実例。
//!
//! `code=404` の形の行から状態コードを数え、失敗を行番号つきで返す。
//! 失敗の型の組み方は `tally` と同じで、本物は `crates/tally-core/src/error.rs` にある。
//!
//! 6 章と同じく、ライブラリ（このファイル）と実行ファイル（`main.rs`）を別のクレートに分けている。
//! `main.rs` は、ここで公開したエラー型を、外のクレートとして `match` する。
//!
//! 1 章から 10 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead};
use std::num::ParseIntError;

/// 集計の失敗。
///
/// **閉じている**（`#[non_exhaustive]` を付けない）。
/// バリアントを足すと、使う側の網羅的な `match` がコンパイルエラーになる。
/// 失敗の大きな分け方は増えにくく、増えたら使う側に気づいてほしいからである。
#[derive(Debug, thiserror::Error)]
pub enum CountError {
    /// 入力を読めなかった。中身が UTF-8 でない場合もここに入る。
    #[error("入力の読み取りに失敗しました")]
    Read(#[from] io::Error),
    /// 行の中身の失敗。表示も原因も `LineError` のものをそのまま出す。
    #[error(transparent)]
    Line(#[from] LineError),
}

/// 行に紐づく失敗。行番号を必ず持つ。
///
/// `Display` と `Error` を手で書いている。
/// `thiserror` で `#[source]` を付ける形にすると、`kind` の文が連結表示で 2 度出る。
#[derive(Debug)]
pub struct LineError {
    /// 何行目か。1 始まり。
    pub line_no: usize,
    /// 何が起きたか。
    pub kind: LineErrorKind,
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} 行目: {}", self.line_no, self.kind)
    }
}

impl Error for LineError {
    /// `kind` を飛ばして、`kind` の原因を指す。
    ///
    /// `kind` の文は `Display` に埋め込んだので、`kind` 自身を指すと同じ文が 2 度出る。
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.kind.source()
    }
}

/// 行の失敗の内訳。
///
/// **開いている**（`#[non_exhaustive]`）。読み方の規則が増えるたびに伸びる側なので、
/// バリアントを足しても使う側を壊さない。
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum LineErrorKind {
    /// `code=` の欄が無い。
    #[error("欄 `code` がありません")]
    MissingCode,
    /// `code=` の値を状態コードとして読めない。
    #[error("状態コードとして読めません")]
    BadCode(#[from] ParseIntError),
}

/// 1 行から状態コードを読む。
///
/// 行番号を知らないので、行番号を持たない `LineErrorKind` を返す。
/// 行番号を付けるのは、数えている `count_codes` の側である。
fn parse_line(line: &str) -> Result<u16, LineErrorKind> {
    let value = line
        .split_whitespace()
        .find_map(|pair| pair.strip_prefix("code="))
        .ok_or(LineErrorKind::MissingCode)?;
    Ok(value.parse()?)
}

/// 状態コードごとの件数を数える。最初の失敗で止まる。
///
/// # Errors
///
/// 読めない入力は [`CountError::Read`]、読めない行は [`CountError::Line`] を返す。
pub fn count_codes(input: impl BufRead) -> Result<BTreeMap<u16, u64>, CountError> {
    let mut counts = BTreeMap::new();
    for (index, line) in input.lines().enumerate() {
        let line = line?;
        let code = parse_line(&line).map_err(|kind| LineError {
            line_no: index + 1,
            kind,
        })?;
        *counts.entry(code).or_insert(0) += 1;
    }
    Ok(counts)
}

/// 原因の連鎖を辿り、各段の表示を `": "` でつないで 1 行にする。
#[must_use]
pub fn one_line(err: &(dyn Error + 'static)) -> String {
    let mut shown = Vec::new();
    let mut current = Some(err);
    while let Some(err) = current {
        shown.push(err.to_string());
        current = err.source();
    }
    shown.join(": ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 状態コードごとに数える() {
        let counts = count_codes("code=200\ncode=404 path=/a\ncode=200\n".as_bytes())
            .expect("どの行も読めるはず");
        assert_eq!(counts, BTreeMap::from([(200, 2), (404, 1)]));
    }

    #[test]
    fn 欄の無い行は行番号つきで失敗する() {
        let err = count_codes("code=200\nlevel=warn\n".as_bytes()).expect_err("2 行目に欄が無い");
        let CountError::Line(line) = &err else {
            panic!("行の失敗を期待した: {err:?}");
        };
        assert_eq!(line.line_no, 2);
        assert!(matches!(line.kind, LineErrorKind::MissingCode));
        // 原因の無い失敗は、1 段だけで終わる。
        assert_eq!(one_line(&err), "2 行目: 欄 `code` がありません");
    }

    #[test]
    fn 連結しても同じ文は_2_度出ない() {
        let err = count_codes("code=200\ncode=x\n".as_bytes()).expect_err("2 行目が数でない");
        assert_eq!(
            one_line(&err),
            "2 行目: 状態コードとして読めません: invalid digit found in string"
        );
    }

    #[test]
    fn transparent_は表示も原因も内側のものを出す() {
        let err = count_codes("code=x\n".as_bytes()).expect_err("1 行目が数でない");
        let CountError::Line(line) = &err else {
            panic!("行の失敗を期待した: {err:?}");
        };
        assert_eq!(err.to_string(), line.to_string());
        // `source()` は `LineError` を指さず、`LineError` の原因（`ParseIntError`）を指す。
        let source = err.source().expect("原因がある");
        assert!(source.downcast_ref::<ParseIntError>().is_some());
    }

    #[test]
    fn utf_8_でない入力は読み取りの失敗になる() {
        let err = count_codes(&b"code=200\n\xff\n"[..]).expect_err("2 行目が UTF-8 でない");
        assert!(matches!(err, CountError::Read(_)), "{err:?}");
        assert_eq!(
            one_line(&err),
            "入力の読み取りに失敗しました: stream did not contain valid UTF-8"
        );
    }

    #[test]
    fn 範囲を超える数も読めない数として扱う() {
        let err = count_codes("code=70000\n".as_bytes()).expect_err("u16 に収まらない");
        assert_eq!(
            one_line(&err),
            "1 行目: 状態コードとして読めません: number too large to fit in target type"
        );
    }

    #[test]
    fn 空の入力は空の表を返す() {
        let counts = count_codes("".as_bytes()).expect("空なら失敗しない");
        assert!(counts.is_empty());
    }
}
