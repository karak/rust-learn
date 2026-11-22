//! 8 章の演習の答え。
//!
//! `cargo test -p book-ch08` で確かめられる。

use std::collections::HashMap;

/// 演習 1: キーごとに、最初に現れた行の番号（1 から数える）を返す。
///
/// `HashMap` へ `collect` すると、同じキーは後から来た値で上書きされる。
/// 後ろから辿れば、最後に書かれるのは最初に現れた行の番号になる。
fn first_seen(lines: &[&str]) -> HashMap<String, usize> {
    lines
        .iter()
        .enumerate()
        .rev()
        .map(|(i, &line)| (line.to_owned(), i + 1))
        .collect()
}

/// 演習 2: 空でない行を残し、捨てた行の数も返す。
///
/// `filter` に渡すクロージャが `skipped` を書き換えるので、クロージャは `skipped` を
/// 可変で借りる。`collect` が終わるまで、ほかの場所から `skipped` を読めない。
fn non_empty(text: &str) -> (Vec<&str>, usize) {
    let mut skipped = 0;
    let kept = text
        .lines()
        .filter(|line| {
            let keep = !line.is_empty();
            if !keep {
                skipped += 1;
            }
            keep
        })
        .collect();
    (kept, skipped)
}

#[test]
fn 演習1_最初に現れた行の番号が残る() {
    let first = first_seen(&["b", "a", "b", "a", "c"]);
    assert_eq!(first.get("b"), Some(&1));
    assert_eq!(first.get("a"), Some(&2));
    assert_eq!(first.get("c"), Some(&5));
    assert!(first_seen(&[]).is_empty());
}

#[test]
fn 演習1_前から辿ると最後の番号が残る() {
    let last: HashMap<&str, usize> = ["b", "a", "b"]
        .iter()
        .enumerate()
        .map(|(i, &line)| (line, i + 1))
        .collect();
    assert_eq!(last.get("b"), Some(&3));
}

#[test]
fn 演習2_空行を捨てて数える() {
    assert_eq!(non_empty("a\n\nb\n\n"), (vec!["a", "b"], 2));
    assert_eq!(non_empty(""), (vec![], 0));
}
