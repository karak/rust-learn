//! 10 章の演習の答え。
//!
//! `cargo test -p book-ch10` で確かめられる。

/// 演習 1: `key<sep>value` の形の行を、キーと値に分ける。区切りが無ければ `None`。
///
/// 戻り値が借りるのは `line` だけなので、`'a` は `line` にだけ付ける。
/// `sep` にも付けると、呼び手は区切りの文字列を結果より長く持ち続けなければならない。
fn split_key<'a>(line: &'a str, sep: &str) -> Option<(&'a str, &'a str)> {
    line.split_once(sep)
}

/// 演習 2: 文字列を 1 行ずつ切り出す。
///
/// `next_line` の戻り値は `self` ではなく、元の文字列の寿命 `'a` に結ぶ。
/// 省略規則に任せると `&mut self` に結ばれ、切り出した行を持ったまま次の行を取れない。
/// `str::lines` と違い、行末の `\r` は取り除かない。
struct Lines<'a> {
    rest: &'a str,
}

impl<'a> Lines<'a> {
    fn new(text: &'a str) -> Self {
        Self { rest: text }
    }

    fn next_line(&mut self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        let (line, rest) = self.rest.split_once('\n').unwrap_or((self.rest, ""));
        self.rest = rest;
        Some(line)
    }
}

#[test]
fn 演習1_区切りの文字列が消えても結果は使える() {
    let line = String::from("lvl: warn");
    let parts;
    {
        let sep = String::from(": ");
        parts = split_key(&line, &sep);
    }
    assert_eq!(parts, Some(("lvl", "warn")));
    assert_eq!(split_key("lvl warn", ": "), None);
    assert_eq!(split_key(": warn", ": "), Some(("", "warn")));
}

#[test]
fn 演習2_切り出した行を持ったまま次の行を取れる() {
    let mut lines = Lines::new("a\n\nb\n");
    let first = lines.next_line();
    let second = lines.next_line();
    let third = lines.next_line();
    assert_eq!((first, second, third), (Some("a"), Some(""), Some("b")));
    assert_eq!(lines.next_line(), None);
}

#[test]
fn 演習2_標準の_lines_と同じ行に割る() {
    for text in ["", "a", "a\n", "a\n\nb", "\n\n"] {
        let mut lines = Lines::new(text);
        let mut ours = Vec::new();
        while let Some(line) = lines.next_line() {
            ours.push(line);
        }
        assert_eq!(ours, text.lines().collect::<Vec<_>>(), "{text:?}");
    }
}
