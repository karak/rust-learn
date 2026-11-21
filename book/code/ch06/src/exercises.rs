//! 6 章の演習の答え。
//!
//! `cargo test -p book-ch06` で確かめられる。

use std::collections::HashMap;

/// 演習 1: 0 から 100 までの値だけを持つ型。
///
/// フィールドを公開しないので、このモジュールとその子孫の外では `Percent(150)` と書けない。
/// 作る道は `Percent::new` だけで、そこで範囲を確かめる。
#[derive(Debug, PartialEq, Eq)]
struct Percent(u8);

impl Percent {
    fn new(value: u8) -> Option<Self> {
        if value <= 100 {
            Some(Self(value))
        } else {
            None
        }
    }

    fn get(&self) -> u8 {
        self.0
    }
}

/// 演習 2: 集計の途中経過。2 つを 1 つにまとめる。
#[derive(Debug, Default)]
struct Counts {
    counts: HashMap<String, u64>,
    skipped: usize,
}

impl Counts {
    /// `other` の中身を `self` に足す。
    ///
    /// `self` は書き換えるので `&mut self`。
    /// `other` は値で受け取るので、キーの `String` を複製せずに `self` へ移せる。
    /// 呼んだあとの `other` は呼び手の手元に残らない。
    fn merge(&mut self, other: Counts) {
        for (value, n) in other.counts {
            *self.counts.entry(value).or_insert(0) += n;
        }
        self.skipped += other.skipped;
    }
}

#[test]
fn 演習1_範囲の外は作れない() {
    assert_eq!(Percent::new(100).map(|p| p.get()), Some(100));
    assert_eq!(Percent::new(0).map(|p| p.get()), Some(0));
    assert_eq!(Percent::new(101), None);
}

#[test]
fn 演習2_2つの途中経過を足し合わせる() {
    let mut a = Counts::default();
    a.counts.insert("warn".to_owned(), 2);
    a.skipped = 1;

    let mut b = Counts::default();
    b.counts.insert("warn".to_owned(), 1);
    b.counts.insert("info".to_owned(), 3);
    b.skipped = 2;

    a.merge(b);
    assert_eq!(a.counts.get("warn"), Some(&3));
    assert_eq!(a.counts.get("info"), Some(&3));
    assert_eq!(a.skipped, 3);
}
