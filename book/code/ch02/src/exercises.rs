//! 2 章の演習の答え。
//!
//! `cargo test -p book-ch02` で確かめられる。
//! テストからしか呼ばないので、モジュールごと `#[cfg(test)]` に置いてある。

use super::Tally;

impl Tally {
    /// 演習 1 の答え。
    ///
    /// 落とすキーを先に集めてしまい、そこで借用を終わらせる。
    /// `doomed` に入っているのは借りた `&String` ではなく複製した `String` なので、
    /// 次の `for` に入る時点で `counts` は誰にも借りられていない。
    ///
    /// 複製の分だけ `retain` より高くつく。
    /// それでも、閉包で表せない条件で落としたいときはこの形になる。
    fn prune_without_retain(&mut self, min: u64) {
        let doomed: Vec<String> = self
            .counts
            .iter()
            .filter(|&(_, &count)| count < min)
            .map(|(key, _)| key.clone())
            .collect();

        for key in doomed {
            self.counts.remove(&key);
        }
    }

    /// 演習 2 の答え。
    ///
    /// キーを借りずに複製して返す。
    /// 戻り値がカウンタを借りていないので、受け取ったまま `push` を呼べる。
    /// 代わりにキーの数だけ `String` を確保する。
    fn sorted_owned(&self) -> Vec<(String, u64)> {
        let mut entries: Vec<(String, u64)> = self
            .counts
            .iter()
            .map(|(key, &count)| (key.clone(), count))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        entries
    }
}

#[cfg(test)]
mod tests {
    use crate::Tally;

    fn sample() -> Tally {
        let mut tally = Tally::new();
        for line in ["info", "warn", "info", "error", "info", "warn"] {
            tally.push(line);
        }
        tally
    }

    #[test]
    fn 先にキーを集める版も_retain_版と同じ結果になる() {
        let mut collected = sample();
        let mut retained = sample();

        collected.prune_without_retain(2);
        retained.prune(2);

        assert_eq!(collected.sorted(), retained.sorted());
        assert_eq!(collected.total, retained.total);
    }

    #[test]
    fn 複製して返せば受け取ったまま書き換えられる() {
        let mut tally = sample();

        let snapshot = tally.sorted_owned();
        // snapshot は tally を借りていないので、持ったまま push できる。
        // 借りて返す sorted ではこう書けない。
        tally.push("info");

        assert_eq!(snapshot[0], ("info".to_owned(), 3));
        assert_eq!(tally.sorted()[0], ("info", 4));
    }
}
