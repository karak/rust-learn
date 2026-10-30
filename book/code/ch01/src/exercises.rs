//! 1 章の演習の答え。
//!
//! `cargo test -p book-ch01` で確かめられる。
//! テストからしか呼ばないので、モジュールごと `#[cfg(test)]` に置いてある。

use super::Tally;

/// 演習 1 の答え。
///
/// `self` を値で受け取り、値で返す。
/// 呼び手はカウンタを手放し、設定済みのカウンタを受け取り直す。
/// `&mut self` で書いても動くが、そのときは `.label("a").push("b")` と
/// つなげられない。
#[derive(Debug, Default)]
struct Labeled {
    tally: Tally,
    label: String,
}

impl Labeled {
    fn new() -> Self {
        Self::default()
    }

    fn label(mut self, label: &str) -> Self {
        self.label = label.to_owned();
        self
    }

    fn push(mut self, key: &str) -> Self {
        self.tally.push(key);
        self
    }

    fn describe(&self) -> String {
        format!("{}: {}", self.label, self.tally.total)
    }
}

/// 演習 2 の答え。
///
/// 借りて受け取る版。`other` は呼び出し後も生きている。
/// 代わりにキーごとに `String` を作り直す必要がある。
/// 値で受け取る `merge` はこの複製をしていない。
impl Tally {
    fn merge_borrowed(&mut self, other: &Tally) {
        for (key, count) in &other.counts {
            *self.counts.entry(key.clone()).or_insert(0) += count;
        }
        self.total += other.total;
    }
}

#[cfg(test)]
mod tests {
    use super::Labeled;
    use crate::Tally;

    #[test]
    fn 値で受け取るビルダはつなげて書ける() {
        let built = Labeled::new().label("app.log").push("info").push("warn");

        assert_eq!(built.describe(), "app.log: 2");
    }

    #[test]
    fn 借りて受け取ると元のカウンタが残る() {
        let mut left = Tally::new();
        left.push("info");
        left.push("info");

        let mut right = Tally::new();
        right.push("info");
        right.push("error");

        left.merge_borrowed(&right);

        assert_eq!(left.sorted(), vec![("info", 3), ("error", 1)]);
        // 借りただけなので right は生きている。値で受け取る merge ではこう書けない。
        assert_eq!(right.total, 2);
    }
}
