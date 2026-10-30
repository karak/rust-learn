//! 1 章の演習の答え。
//!
//! `cargo t -p book-ch01` で確かめられる。
//! テストからしか呼ばないので、モジュールごと `#[cfg(test)]` に置いてある。

use std::collections::HashMap;

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
/// 本体の `merge` はこの複製をしていない。
fn merge_borrowed(into: &mut HashMap<String, u64>, other: &HashMap<String, u64>) {
    for (key, count) in other {
        *into.entry(key.clone()).or_insert(0) += count;
    }
}

#[cfg(test)]
mod tests {
    use super::{Labeled, merge_borrowed};
    use std::collections::HashMap;

    #[test]
    fn 値で受け取るビルダはつなげて書ける() {
        let built = Labeled::new().label("app.log").push("info").push("warn");

        assert_eq!(built.describe(), "app.log: 2");
    }

    #[test]
    fn 借りて受け取ると元のカウンタが残る() {
        let mut left = HashMap::from([("info".to_owned(), 2_u64)]);
        let right = HashMap::from([("info".to_owned(), 1_u64), ("error".to_owned(), 1)]);

        merge_borrowed(&mut left, &right);

        assert_eq!(left.get("info"), Some(&3));
        assert_eq!(left.get("error"), Some(&1));
        // 借りただけなので right は生きている。値で受け取る merge ではこう書けない。
        assert_eq!(right.len(), 2);
    }
}
