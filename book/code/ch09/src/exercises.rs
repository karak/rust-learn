//! 9 章の演習の答え。
//!
//! `cargo test -p book-ch09` で確かめられる。

use super::{Bucket, Level, buckets, tally};

/// 演習 1: いちばん多い欄の名前を返す。同数なら名前の小さいほう。空なら `None`。
///
/// `tally` と同じ境界を書けば、`tally` を呼べる。
fn most_common<T: Bucket>(items: &[T]) -> Option<String> {
    tally(items)
        .into_iter()
        // `max_by` は等しいとき後ろを選ぶので、名前の比較は逆向きにする。
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(name, _)| name)
}

/// 演習 2: 応答時間（ミリ秒）。100 未満を `fast`、それ以外を `slow` に数え、1000 以上を失敗とする。
struct Latency(u32);

impl Bucket for Latency {
    fn bucket(&self) -> String {
        if self.0 < 100 { "fast" } else { "slow" }.to_owned()
    }

    fn is_error(&self) -> bool {
        self.0 >= 1000
    }
}

#[test]
fn 演習1_いちばん多い欄を返す() {
    // 3 つとも 1 件ずつなので、名前の小さい `2xx` になる。
    assert_eq!(most_common(&[404_u16, 200, 503]), Some("2xx".to_owned()));
    assert_eq!(most_common(&[200_u16, 404, 200]), Some("2xx".to_owned()));
    assert_eq!(
        most_common(&[Level::Warn, Level::Info, Level::Warn]),
        Some("warn".to_owned())
    );
    assert_eq!(most_common::<u16>(&[]), None);
}

#[test]
fn 演習2_自分の型も同じ列に並べられる() {
    let slow = Latency(1500);
    assert!(slow.is_error());
    assert!(!Latency(999).is_error());
    assert!(Latency(1000).is_error());
    assert_eq!(
        buckets(&[&Latency(50), &slow, &404_u16, &Level::Warn]),
        vec!["fast", "slow", "4xx", "warn"]
    );
    assert_eq!(Latency(99).bucket(), "fast");
    assert_eq!(Latency(100).bucket(), "slow");
}
