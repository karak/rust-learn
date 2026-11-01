//! 2 章「借用」の実例。
//!
//! `tally` の `Counter` から、借用の話に要る部分だけを抜き出したもの。
//! 本物は `crates/tally-core/src/count.rs` にある。
//!
//! 1 章のコードとは独立している。章を飛ばして読んでも動く。

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

    /// 1 行ぶん数える。中身を書き換えるので `&mut self`。
    fn push(&mut self, key: &str) {
        *self.counts.entry(key.to_owned()).or_insert(0) += 1;
        self.total += 1;
    }

    /// 件数の降順に並べて返す。読むだけなので `&self`。
    ///
    /// 返した `&str` は `counts` の中の `String` を指している。
    /// そのため、この `Vec` を持っているあいだはカウンタ全体が借りられたままになる。
    fn sorted(&self) -> Vec<(&str, u64)> {
        let mut entries: Vec<(&str, u64)> = self
            .counts
            .iter()
            .map(|(key, &count)| (key.as_str(), count))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        entries
    }

    /// `min` 件に満たないキーを落とす。
    ///
    /// `retain` に渡した閉包の中では、`counts` を触れるのは `retain` 自身だけになる。
    /// 自分で反復しながら消そうとすると、同じ `counts` を二重に借りることになって通らない。
    ///
    /// `total` は減らさない。何行読んだかは、あとでキーを落としても変わらない。
    fn prune(&mut self, min: u64) {
        self.counts.retain(|_, count| *count >= min);
    }
}

fn main() {
    let mut tally = Tally::new();
    for line in ["info", "warn", "info", "error", "info", "warn"] {
        tally.push(line);
    }

    // sorted が返した Vec は tally を借りている。
    // 使い終えてから prune を呼ぶので、借用は重ならない。
    let before = tally.sorted();
    println!("{before:?}");

    tally.prune(2);

    for (key, count) in tally.sorted() {
        println!("{count}\t{key}");
    }
    println!("total\t{}", tally.total);
}

#[cfg(test)]
mod tests {
    use super::Tally;

    fn sample() -> Tally {
        let mut tally = Tally::new();
        for line in ["info", "warn", "info", "error", "info", "warn"] {
            tally.push(line);
        }
        tally
    }

    #[test]
    fn 件数の降順に並び同数ならキーの昇順になる() {
        assert_eq!(
            sample().sorted(),
            vec![("info", 3), ("warn", 2), ("error", 1)]
        );
    }

    #[test]
    fn 少ないキーを落としても読んだ行数は変わらない() {
        let mut tally = sample();

        tally.prune(2);

        assert_eq!(tally.sorted(), vec![("info", 3), ("warn", 2)]);
        assert_eq!(tally.total, 6, "落としたのはキーであって、読んだ行ではない");
    }
}
