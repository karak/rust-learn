//! 5 章「`Result` と `?`、`Option` との違い」の実例。
//!
//! `tally` から、入力を 1 行ずつ読む部分と、`--strict` の有無で
//! 「フィールドが無い行」の扱いを変える部分を抜き出したもの。
//! 本物は `crates/tally-core/src/count.rs` と `crates/tally-core/src/select.rs` にある。
//!
//! 本物は行を JSON として読むが、ここでは 4 章と同じく `level=warn` のような
//! 空白区切りの `名前=値` を読む。入力はファイルではなくバイト列（`&[u8]`）から読む。
//! ファイルを開く失敗まで持ち込まずに、読む途中の失敗を扱えるからである。
//!
//! 1 章から 4 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::collections::HashMap;
use std::io::{self, BufRead};

/// 集計の失敗。
///
/// 読み込みの失敗と、行の中身の失敗を、どちらも 1 つの型で返す。
#[derive(Debug)]
enum CountError {
    /// 入力を読めなかった。中身が UTF-8 でない場合もここに入る。
    Read(io::Error),
    /// `--strict` のとき、指定したフィールドを持たない行があった。
    MissingField { line_no: usize },
}

/// 1 行から `name=値` の値を探す。
///
/// 見つからないのは失敗ではないので、`Result` ではなく `Option` で返す。
/// 4 章の `Picked` と同じ形である。
fn find_field(line: &str, name: &str) -> Option<String> {
    for token in line.split_whitespace() {
        if token.starts_with(name) && token[name.len()..].starts_with('=') {
            return Some(token[name.len() + 1..].to_owned());
        }
    }
    None
}

/// キーごとの件数。
struct Tally {
    field: String,
    /// `true` なら、フィールドの無い行で集計を止める。
    strict: bool,
    counts: HashMap<String, u64>,
    skipped: usize,
}

impl Tally {
    fn new(field: &str, strict: bool) -> Self {
        Self {
            field: field.to_owned(),
            strict,
            counts: HashMap::new(),
            skipped: 0,
        }
    }

    /// 1 行ぶん数える。
    ///
    /// 同じ「見つからない」が、`strict` によって飛ばす行にも失敗にもなる。
    /// `Option` を `Result` に変えるかどうかを、呼んだ側の事情で決めている。
    fn push_line(&mut self, line: &str, line_no: usize) -> Result<(), CountError> {
        match find_field(line, &self.field) {
            Some(key) => *self.counts.entry(key).or_insert(0) += 1,
            None if self.strict => return Err(CountError::MissingField { line_no }),
            None => self.skipped += 1,
        }
        Ok(())
    }

    /// 件数の降順に並べて返す。同数ならキーの昇順。
    fn sorted(&self) -> Vec<(&str, u64)> {
        let mut entries: Vec<(&str, u64)> = self
            .counts
            .iter()
            .map(|(key, &count)| (key.as_str(), count))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        entries
    }
}

/// 入力を最後まで読んで数える。
///
/// `?` は、失敗ならその場で `Err` を返し、成功なら中身を取り出して先へ進む。
/// `read_line` の失敗は `io::Error` なので、`CountError::Read` で包んでから `?` に渡す。
/// `push_line` は最初から `CountError` を返すので、そのまま `?` を付けられる。
///
/// `&[u8]` から読むと、読んだぶんだけ `input` の先頭が進む。だから `mut` が要る。
fn count_all(mut input: &[u8], tally: &mut Tally) -> Result<(), CountError> {
    let mut line = String::new();
    let mut line_no = 0;
    loop {
        line.clear();
        if input.read_line(&mut line).map_err(CountError::Read)? == 0 {
            break;
        }
        line_no += 1;
        tally.push_line(line.trim_end(), line_no)?;
    }
    Ok(())
}

fn main() {
    let input = b"level=info user=alice\nlevel=warn\nuser=bob\nlevel=info\n";

    for strict in [false, true] {
        let mut tally = Tally::new("level", strict);
        match count_all(input, &mut tally) {
            Ok(()) => {
                for (key, count) in tally.sorted() {
                    println!("{count}\t{key}");
                }
                println!("飛ばした行\t{}", tally.skipped);
            }
            Err(CountError::MissingField { line_no }) => {
                println!("{line_no} 行目に level がありません");
            }
            Err(CountError::Read(source)) => println!("読めません: {source}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CountError, Tally, count_all, find_field};
    use std::io;

    #[test]
    fn 見つからないことは_none_で返す() {
        assert_eq!(find_field("level=warn", "level"), Some("warn".to_owned()));
        assert_eq!(find_field("level=", "level"), Some(String::new()));
        assert_eq!(find_field("user=bob", "level"), None);
        assert_eq!(find_field("levels=3", "level"), None);
    }

    #[test]
    fn 厳しくなければフィールドの無い行を飛ばして続ける() {
        let mut tally = Tally::new("level", false);

        let result = count_all(b"level=info\nuser=bob\nlevel=info\n", &mut tally);

        assert!(result.is_ok());
        assert_eq!(tally.sorted(), vec![("info", 2)]);
        assert_eq!(tally.skipped, 1);
    }

    #[test]
    fn 厳しければフィールドの無い行で止まり行番号を返す() {
        let mut tally = Tally::new("level", true);

        let result = count_all(b"level=info\nuser=bob\nlevel=warn\n", &mut tally);

        match result {
            Err(CountError::MissingField { line_no }) => assert_eq!(line_no, 2),
            other => panic!("2 行目で止まるはずが {other:?}"),
        }
        // `?` がその場で返したので、3 行目は数えていない。
        assert_eq!(tally.sorted(), vec![("info", 1)]);
    }

    #[test]
    fn utf_8_でない入力は読み込みの失敗になる() {
        let mut tally = Tally::new("level", false);

        let result = count_all(b"level=info\n\xff\n", &mut tally);

        match result {
            Err(CountError::Read(source)) => assert_eq!(source.kind(), io::ErrorKind::InvalidData),
            other => panic!("読み込みの失敗になるはずが {other:?}"),
        }
        assert_eq!(tally.sorted(), vec![("info", 1)]);
    }
}
