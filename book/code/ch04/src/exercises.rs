//! 4 章の演習の答え。
//!
//! `cargo test -p book-ch04` で確かめられる。
//! テストからしか呼ばないので、モジュールごと `#[cfg(test)]` に置いてある。

use super::Picked;

/// 演習 1 の答え。
///
/// 本文の `Key` に `Column` を足したもの。バリアントはあとから別の場所で足せないので、
/// ここでは型ごと書き直してある。
///
/// 本文の `Key` に `Column` を足してビルドすると、`extract` の `match` が
/// `Key::Column(_)` を扱っていないと言って止まる。直す場所はそこだけである。
enum Key {
    WholeLine,
    Field(String),
    Column(usize),
}

impl Key {
    fn extract(&self, line: &str) -> Picked {
        match self {
            Key::WholeLine => Picked::Found(line.trim().to_owned()),
            Key::Field(name) => {
                for token in line.split_whitespace() {
                    if token.starts_with(name.as_str()) && token[name.len()..].starts_with('=') {
                        return Picked::Found(token[name.len() + 1..].to_owned());
                    }
                }
                Picked::Missing
            }
            // `index` は `&usize` として束縛される。`*index` で値を取り出して比べている。
            Key::Column(index) => {
                for (position, word) in line.split_whitespace().enumerate() {
                    if position == *index {
                        return Picked::Found(word.to_owned());
                    }
                }
                Picked::Missing
            }
        }
    }
}

/// 演習 2 の答えが返す区分。
#[derive(Debug, PartialEq)]
enum Bucket {
    Zero,
    One,
    Few,
    Many,
}

/// 演習 2 の答え。
///
/// `u64` の値をすべて、`_` を使わずに範囲で挙げている。
/// `2..=9` を `2..9` と書き間違えると、9 が漏れてコンパイルエラーになる。
/// `_` で受けていれば、9 は黙って `Many` に入る。
fn bucket(count: u64) -> Bucket {
    match count {
        0 => Bucket::Zero,
        1 => Bucket::One,
        2..=9 => Bucket::Few,
        10.. => Bucket::Many,
    }
}

#[cfg(test)]
mod tests {
    use super::{Bucket, Key, bucket};
    use crate::Picked;

    #[test]
    fn 列の番号で語を取り出す() {
        let key = Key::Column(1);

        assert_eq!(
            key.extract("2026-11-21 warn disk"),
            Picked::Found("warn".to_owned())
        );
        assert_eq!(key.extract("only"), Picked::Missing);
    }

    #[test]
    fn 足す前からあるバリアントの振る舞いは変わらない() {
        assert_eq!(
            Key::WholeLine.extract(" warn "),
            Picked::Found("warn".to_owned())
        );
        assert_eq!(
            Key::Field("level".to_owned()).extract("level=info"),
            Picked::Found("info".to_owned())
        );
    }

    #[test]
    fn 境界の値がそれぞれの区分に入る() {
        assert_eq!(bucket(0), Bucket::Zero);
        assert_eq!(bucket(1), Bucket::One);
        assert_eq!(bucket(2), Bucket::Few);
        assert_eq!(bucket(9), Bucket::Few);
        assert_eq!(bucket(10), Bucket::Many);
        assert_eq!(bucket(u64::MAX), Bucket::Many);
    }
}
