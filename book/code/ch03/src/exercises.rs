//! 3 章の演習の答え。
//!
//! `cargo test -p book-ch03` で確かめられる。
//! テストからしか呼ばないので、モジュールごと `#[cfg(test)]` に置いてある。

use super::Tally;

impl Tally {
    /// 演習 1 の答え。
    ///
    /// `HashMap<String, _>` は `&str` のままで引ける。
    /// 既にあるキーなら、`String` を作らずに件数だけ増やして帰る。
    ///
    /// 代わりに、初めて見るキーではハッシュを 2 回計算する
    /// （`get_mut` で 1 回、`insert` で 1 回）。
    /// どちらが得かは、入力にキーが何度繰り返し現れるかで決まる。
    fn push_reusing(&mut self, key: &str) {
        self.total += 1;
        if let Some(count) = self.counts.get_mut(key) {
            *count += 1;
            return;
        }
        self.counts.insert(key.to_owned(), 1);
    }
}

/// 演習 2 の答え。
///
/// `char_indices()` が返すバイト位置は必ず文字の境界なので、そこで切る限り止まらない。
/// `nth` は `max + 1` 文字目で打ち切るので、長い行を最後まで読まない。
///
/// 返すのは `line` の一部を指す `&str` で、新しい文字列は作らない。
/// 引数の参照が 1 つだけなので、戻り値がどれを借りているかを書かずに済む。
fn head(line: &str, max: usize) -> &str {
    match line.char_indices().nth(max) {
        Some((boundary, _)) => &line[..boundary],
        None => line,
    }
}

#[cfg(test)]
mod tests {
    use super::head;
    use crate::Tally;

    const INPUT: &str = "info\nwarn\ninfo\nerror\ninfo\nwarn\n";

    #[test]
    fn 複製を省く版も同じ件数になる() {
        let mut copied = Tally::new();
        let mut reused = Tally::new();

        for line in INPUT.lines() {
            copied.push(line);
            reused.push_reusing(line);
        }

        assert_eq!(reused.sorted(), copied.sorted());
        assert_eq!(reused.total, copied.total);
    }

    #[test]
    fn 日本語の行でも文字の途中で切らない() {
        // 2 文字は 6 バイト。バイトで 2 と数えていれば「日」の途中で止まる。
        assert_eq!(head("日本語のログ", 2), "日本");
        assert_eq!(head("info", 2), "in");
    }

    #[test]
    fn 短い行はそのまま返す() {
        assert_eq!(head("info", 10), "info");
        assert_eq!(head("", 3), "");
    }

    #[test]
    fn 切り出した結果は元の行を指している() {
        let line = String::from("日本語のログ");

        let cut = head(&line, 2);

        assert_eq!(
            cut.as_ptr(),
            line.as_ptr(),
            "先頭から切るので先頭が一致する"
        );
    }
}
