//! 7 章の演習の答え。
//!
//! `cargo test -p book-ch07` で確かめられる。

use std::cmp::Reverse;
use std::collections::HashMap;

/// 演習 1: キーごとに、最初に現れた行の番号（1 から数える）を返す。
///
/// `or_insert` は、キーが既にあれば何もしない。だから最初の番号が残る。
/// `insert` で書くと上書きされ、最後に現れた行の番号が残る。
fn first_seen(lines: &[&str]) -> HashMap<String, usize> {
    let mut first = HashMap::new();
    let mut line_no = 0;
    for &line in lines {
        line_no += 1;
        first.entry(line.to_owned()).or_insert(line_no);
    }
    first
}

/// 演習 2: 件数の昇順、同数ならキーの降順に並べる。
///
/// 今度は件数をそのまま、キーを `Reverse` で包む。
fn ranked_ascending(counts: &HashMap<String, u64>) -> Vec<(&str, u64)> {
    let mut keyed = Vec::with_capacity(counts.len());
    for (key, &n) in counts {
        keyed.push((n, Reverse(key.as_str())));
    }
    keyed.sort();

    let mut out = Vec::with_capacity(keyed.len());
    for (n, Reverse(key)) in keyed {
        out.push((key, n));
    }
    out
}

#[test]
fn 演習1_最初に現れた行の番号が残る() {
    let first = first_seen(&["b", "a", "b", "a", "c"]);
    assert_eq!(first.get("b"), Some(&1));
    assert_eq!(first.get("a"), Some(&2));
    assert_eq!(first.get("c"), Some(&5));
}

#[test]
fn 演習1_insert_で書くと最後の番号が残る() {
    let mut last = HashMap::new();
    let mut line_no = 0;
    for &line in &["b", "a", "b"] {
        line_no += 1;
        last.insert(line, line_no);
    }
    assert_eq!(last.get("b"), Some(&3));
}

#[test]
fn 演習2_件数の昇順で同数ならキーの降順に並ぶ() {
    let mut counts = HashMap::new();
    for (key, n) in [("warn", 2), ("info", 2), ("debug", 1), ("error", 1)] {
        counts.insert(key.to_owned(), n);
    }
    assert_eq!(
        ranked_ascending(&counts),
        vec![("error", 1), ("debug", 1), ("warn", 2), ("info", 2)]
    );
}
