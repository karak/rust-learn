//! 4 章「enum と網羅的な `match`」の実例。
//!
//! `tally` から、出力形式（`Format`）と、行からキーを取り出す規則（`Key`）を抜き出したもの。
//! 本物は `crates/tally/src/format.rs` と `crates/tally-core/src/select.rs` にある。
//!
//! 本物の `Key` は行を JSON として読むが、ここでは `level=warn` のような
//! 空白区切りの `名前=値` を読む。JSON の解析を持ち込むと、章の話が enum から逸れるためである。
//!
//! 1 章から 3 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::collections::HashMap;

/// 出力形式。
///
/// 取りうる値はこの 3 つだけで、ほかの値を持つ `Format` は作れない。
enum Format {
    Text,
    Json,
    Csv,
}

impl Format {
    /// コマンドラインで指定するときの名前。
    ///
    /// `_` の腕を書かずに全部を挙げている。形式を足すと、ここがコンパイルエラーになる。
    fn name(&self) -> &str {
        match self {
            Format::Text => "text",
            Format::Json => "json",
            Format::Csv => "csv",
        }
    }
}

/// 行から何をキーとして取り出すか。
///
/// バリアントごとに持つ値が違う。`WholeLine` は何も持たず、`Field` は探す名前を持つ。
enum Key {
    WholeLine,
    Field(String),
}

/// 1 行から取り出した結果。
///
/// 「見つからなかった」を空文字列で表すと、値が空のフィールド（`level=`）と区別できない。
/// だから場合を分けて、型にする。
#[derive(Debug, PartialEq)]
enum Picked {
    Found(String),
    Missing,
}

impl Key {
    /// 1 行からキーを取り出す。
    ///
    /// `match self` の `self` は `&Key` なので、`name` は `&String` として束縛される。
    /// `Key` の中の `String` を持ち出さずに、借りたまま読んでいる。
    ///
    /// 取り出したキーは `String` にして返す。行を指す `&str` のまま返すには、
    /// 戻り値がどちらの引数を借りているかを書く記法が要る（10 章）。
    fn extract(&self, line: &str) -> Picked {
        match self {
            Key::WholeLine => Picked::Found(line.trim().to_owned()),
            Key::Field(name) => {
                for token in line.split_whitespace() {
                    // `level=warn` が `level` で始まり、その直後が `=` なら、残りが値である。
                    // 名前は ASCII に限らないが、`starts_with` が通った時点で
                    // `name.len()` は文字の境界にある。
                    if token.starts_with(name.as_str()) && token[name.len()..].starts_with('=') {
                        return Picked::Found(token[name.len() + 1..].to_owned());
                    }
                }
                Picked::Missing
            }
        }
    }
}

/// キーごとの件数。
struct Tally {
    key: Key,
    counts: HashMap<String, u64>,
    /// キーを取り出せずに飛ばした行の数。
    skipped: usize,
}

impl Tally {
    fn new(key: Key) -> Self {
        Self {
            key,
            counts: HashMap::new(),
            skipped: 0,
        }
    }

    fn push_line(&mut self, line: &str) {
        match self.key.extract(line) {
            Picked::Found(key) => *self.counts.entry(key).or_insert(0) += 1,
            Picked::Missing => self.skipped += 1,
        }
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

fn main() {
    let input = "level=info user=alice\nlevel=warn\nuser=bob\nlevel=info\n";

    for key in [Key::Field(String::from("level")), Key::WholeLine] {
        let mut tally = Tally::new(key);
        for line in input.lines() {
            tally.push_line(line);
        }
        for (key, count) in tally.sorted() {
            println!("{count}\t{key}");
        }
        println!("飛ばした行\t{}\n", tally.skipped);
    }

    for format in [Format::Text, Format::Json, Format::Csv] {
        println!("--format {}", format.name());
    }
}

#[cfg(test)]
mod tests {
    use super::{Format, Key, Picked, Tally};

    #[test]
    fn 名前は形式ごとに一つ決まる() {
        assert_eq!(Format::Text.name(), "text");
        assert_eq!(Format::Json.name(), "json");
        assert_eq!(Format::Csv.name(), "csv");
    }

    #[test]
    fn 行全体をキーにすると前後の空白を落とす() {
        assert_eq!(
            Key::WholeLine.extract("  warn  "),
            Picked::Found("warn".to_owned())
        );
    }

    #[test]
    fn フィールドの値と見つからない場合を区別する() {
        let key = Key::Field("level".to_owned());

        assert_eq!(
            key.extract("user=alice level=warn"),
            Picked::Found("warn".to_owned())
        );
        // 値が空でも「見つかった」であり、見つからなかったのとは別の場合になる。
        assert_eq!(key.extract("level="), Picked::Found(String::new()));
        assert_eq!(key.extract("user=alice"), Picked::Missing);
        // 名前が前方一致するだけのフィールドは別物である。
        assert_eq!(key.extract("levels=3"), Picked::Missing);
    }

    #[test]
    fn 見つからない行は数えずに飛ばす() {
        let mut tally = Tally::new(Key::Field("level".to_owned()));

        for line in ["level=info", "user=bob", "level=info", "level=warn"] {
            tally.push_line(line);
        }

        assert_eq!(tally.sorted(), vec![("info", 2), ("warn", 1)]);
        assert_eq!(tally.skipped, 1);
    }

    // 大きさはポインタの幅で変わるので、64 ビット環境に限って固定する。
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn 判別の印は_string_の使わない値に埋め込まれることがある() {
        // `Key` は `String` 1 つぶんの大きさに収まる。どのバリアントかの印を別に持たない。
        assert_eq!(size_of::<Key>(), size_of::<String>());

        // 2 つのバリアントがどちらも `String` を持つと、印を置く場所が別に要る。
        // `Result` は 5 章で扱う、成功と失敗の 2 つのバリアントを持つ標準ライブラリの enum である。
        assert_eq!(size_of::<Result<String, String>>(), 32);
    }
}
