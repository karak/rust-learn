//! 5 章の演習の答え。
//!
//! `cargo test -p book-ch05` で確かめられる。
//! テストからしか呼ばないので、モジュールごと `#[cfg(test)]` に置いてある。

use super::{CountError, Tally, find_field};
use std::io::BufRead;

/// 演習 1 の答え。
///
/// `?` は `Option` にも使える。`None` ならその場で `None` を返す。
/// 戻り値が `Option` の関数の中でだけ使える。
fn level_and_user(line: &str) -> Option<(String, String)> {
    let level = find_field(line, "level")?;
    let user = find_field(line, "user")?;
    Some((level, user))
}

/// 演習 2 の答え。本文の `count_all` を、`?` を使わずに書いたもの。
///
/// `?` 1 つが、`match` 1 つと早期の `return` に当たる。`push_line` に付けていた `?` なら、
/// `rustfmt` で整えると 4 行になる（`match` の行、`Ok` の腕、`Err` の腕、閉じ括弧）。
fn count_all_without_question(mut input: &[u8], tally: &mut Tally) -> Result<(), CountError> {
    let mut line = String::new();
    let mut line_no = 0;
    loop {
        line.clear();
        let read = match input.read_line(&mut line) {
            Ok(read) => read,
            Err(source) => return Err(CountError::Read(source)),
        };
        if read == 0 {
            break;
        }
        line_no += 1;
        match tally.push_line(line.trim_end(), line_no) {
            Ok(()) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{count_all_without_question, level_and_user};
    use crate::{CountError, Tally, count_all};

    #[test]
    fn 両方あれば組で返し片方でも無ければ_none() {
        assert_eq!(
            level_and_user("user=alice level=warn"),
            Some(("warn".to_owned(), "alice".to_owned()))
        );
        assert_eq!(level_and_user("level=warn"), None);
        assert_eq!(level_and_user("user=alice"), None);
    }

    #[test]
    fn 疑問符を使わない版も同じ結果になる() {
        let input: &[u8] = b"level=info\nuser=bob\nlevel=warn\nlevel=info\n";

        let mut with = Tally::new("level", false);
        let mut without = Tally::new("level", false);
        assert!(count_all(input, &mut with).is_ok());
        assert!(count_all_without_question(input, &mut without).is_ok());
        assert_eq!(with.sorted(), without.sorted());
        assert_eq!(with.skipped, without.skipped);

        let mut strict = Tally::new("level", true);
        match count_all_without_question(input, &mut strict) {
            Err(CountError::MissingField { line_no }) => assert_eq!(line_no, 2),
            other => panic!("2 行目で止まるはずが {other:?}"),
        }
    }
}
