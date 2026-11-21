//! 3 章「`String` と `&str`、スライス」の実例。
//!
//! `tally` の `Counter` から、文字列の受け渡しに要る部分だけを抜き出したもの。
//! 本物は `crates/tally-core/src/count.rs` にある。
//!
//! 本物は `BufRead::read_line` で 1 本のバッファを使い回すが、ここでは
//! `str::lines()` で入力全体を割る。I/O の失敗の扱いは 5 章まで出さないためである。
//! どちらも「1 つの持ち主を、行ごとの `&str` で眺める」点は同じ。
//!
//! 1 章・2 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::collections::HashMap;

/// キーごとの件数。
#[derive(Debug, Default)]
struct Tally {
    counts: HashMap<String, u64>,
    total: usize,
}

impl Tally {
    fn new() -> Self {
        Self::default()
    }

    /// 1 行ぶん数える。
    ///
    /// キーは `&str` で借りて受け取るが、`String` は呼ぶたびに作る。
    /// `entry` は所有したキーを要求するからである。
    /// マップは呼び手より長く生きるので、借りたままでは入れられない。
    /// 既に数えたキーで `String` を作らずに済ませる方法は、演習 1 で見る。
    fn push(&mut self, key: &str) {
        *self.counts.entry(key.to_owned()).or_insert(0) += 1;
        self.total += 1;
    }

    /// 入力全体を行に割って数える。空白だけの行は数えない。
    ///
    /// `lines()` が返す各行も、`trim()` の結果も、`input` の一部を指す `&str` である。
    /// ここまでは新しい文字列を 1 つも作っていない。
    fn push_all(&mut self, input: &str) {
        for line in input.lines() {
            let key = line.trim();
            if key.is_empty() {
                continue;
            }
            self.push(key);
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
    let input = String::from("info\nwarn\n  info\n\n日本語のログ\ninfo\n");

    let mut tally = Tally::new();
    // `&String` を渡しているが、受け取る側は `&str` である。
    tally.push_all(&input);
    // 文字列リテラルは最初から `&str` なので、そのまま渡せる。
    tally.push("warn");

    for (key, count) in tally.sorted() {
        // len() はバイト数、chars().count() は文字数。日本語の行で食い違う。
        println!(
            "{count}\t{key}\t({} バイト, {} 文字)",
            key.len(),
            key.chars().count()
        );
    }
    println!("total\t{}", tally.total);
}

#[cfg(test)]
mod tests {
    use super::Tally;

    #[test]
    fn 前後の空白を落とした行が同じキーになり空白だけの行は数えない() {
        let mut tally = Tally::new();

        tally.push_all("info\n  info  \n\n   \nwarn\n");

        assert_eq!(tally.sorted(), vec![("info", 2), ("warn", 1)]);
        assert_eq!(tally.total, 3);
    }

    #[test]
    fn lines_が返す行は元の文字列の中を指している() {
        let input = String::from("info\nwarn\n");
        let inside = input.as_bytes().as_ptr_range();

        for line in input.lines() {
            // 複製されていれば、別の場所に確保されたバイト列を指すはずである。
            assert!(inside.contains(&line.as_ptr()), "{line:?} が複製されている");
        }
    }

    #[test]
    fn len_はバイト数で文字数ではない() {
        assert_eq!("info".len(), 4);
        assert_eq!("日本語".len(), 9, "UTF-8 では 1 文字 3 バイト");
        assert_eq!("日本語".chars().count(), 3);
    }

    #[test]
    #[should_panic(expected = "is not a char boundary")]
    fn 文字の途中で切ると実行時に止まる() {
        let s = String::from("日本");
        // 0..1 は「日」の 3 バイトの途中である。
        std::hint::black_box(&s[..1]);
    }

    // 大きさはポインタの幅で変わるので、64 ビット環境に限って固定する。
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn string_は3語で_str_への参照は2語() {
        // String: 先頭へのポインタ、容量、長さ。
        assert_eq!(size_of::<String>(), 24);
        // &str: 先頭へのポインタ、長さ。容量は持たない。
        assert_eq!(size_of::<&str>(), 16);
    }
}
