//! 1 章「所有権と move」の実例。
//!
//! `tally` の `Counter` から、所有権の話に要る部分だけを抜き出したもの。
//! 本物は `crates/tally-core/src/count.rs` にある。

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

    fn push(&mut self, key: &str) {
        *self.counts.entry(key.to_owned()).or_insert(0) += 1;
        self.total += 1;
    }

    /// 別のカウンタを取り込む。
    ///
    /// `other` を借りずに受け取るのは、キーの `String` をそのまま引き取るため。
    /// 借りて受け取ると、キーごとに新しい `String` を作ることになる。
    fn merge(&mut self, other: Tally) {
        for (key, count) in other.counts {
            *self.counts.entry(key).or_insert(0) += count;
        }
        // counts は上で move したが、total はまだ読める。
        self.total += other.total;
    }

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
    let mut left = Tally::new();
    for line in ["info", "warn", "info"] {
        left.push(line);
    }

    let mut right = Tally::new();
    for line in ["error", "info"] {
        right.push(line);
    }

    left.merge(right);
    // この行から先、right は使えない。
    // println!("{right:?}"); と書くと E0382 で落ちる。

    for (key, count) in left.sorted() {
        println!("{count}\t{key}");
    }
    println!("total\t{}", left.total);
}

#[cfg(test)]
mod tests {
    use super::Tally;

    #[test]
    fn 合流させると件数が足し合わされる() {
        let mut left = Tally::new();
        left.push("info");
        left.push("info");

        let mut right = Tally::new();
        right.push("info");
        right.push("error");

        left.merge(right);

        assert_eq!(left.sorted(), vec![("info", 3), ("error", 1)]);
        assert_eq!(left.total, 4);
    }

    #[test]
    fn 空のカウンタを合流させても何も変わらない() {
        let mut left = Tally::new();
        left.push("info");

        left.merge(Tally::new());

        assert_eq!(left.sorted(), vec![("info", 1)]);
        assert_eq!(left.total, 1);
    }
}
