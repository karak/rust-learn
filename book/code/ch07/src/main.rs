//! 7 章「コレクションと順序」の実例。
//!
//! 行を数えて `HashMap` に入れ、表示する前に並べ直す。
//! 本物の集計は `crates/tally-core/src/count.rs` にある。
//!
//! `cargo run -p book-ch07` を何度か走らせると、`HashMap` の行と「件数だけ」の行の順序が変わりうる。
//! 既定のハッシュ関数が、マップを作るたびに違う種を使うからである。
//!
//! 1 章から 6 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::cmp::{Ordering, Reverse};
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};

/// 行ごとに出現回数を数える。
fn count(lines: &[&str]) -> HashMap<String, u64> {
    let mut counts = HashMap::new();
    for &line in lines {
        // `entry` は 1 回の探索で「既にある」か「まだ無い」かを返す。
        // C++ の `m[k]++` と違い、無いときに何を入れるかを呼び手が書く。
        match counts.entry(line.to_owned()) {
            Entry::Occupied(mut seen) => *seen.get_mut() += 1,
            Entry::Vacant(new) => {
                new.insert(1);
            }
        }
    }
    counts
}

/// 件数の降順、同数ならキーの昇順に並べる。
///
/// `HashMap` の反復順は実行ごとに変わりうるので、出力の順序はここで決める。
fn ranked(counts: &HashMap<String, u64>) -> Vec<(&str, u64)> {
    // タプルは前の要素から順に比べる。件数を `Reverse` で包むと、件数だけが逆順になる。
    let mut keyed = Vec::with_capacity(counts.len());
    for (key, &n) in counts {
        keyed.push((Reverse(n), key.as_str()));
    }
    // キーは `HashMap` の中で重複しないので、等しい要素は無い。
    // 安定な `sort` でも不安定な `sort_unstable` でも、結果は同じになる。
    keyed.sort();

    let mut out = Vec::with_capacity(keyed.len());
    for (Reverse(n), key) in keyed {
        out.push((key, n));
    }
    out
}

/// 件数の降順だけで並べる。**同数のときの順序を決めていない。**
///
/// 同数の要素は、`HashMap` から取り出した順に残る（`sort_by` は安定なので）。
/// その順序が実行ごとに変わりうるので、出力も変わりうる。比べるために置いてある。
fn ranked_by_count_only(counts: &HashMap<String, u64>) -> Vec<(&str, u64)> {
    let mut out = Vec::with_capacity(counts.len());
    for (key, &n) in counts {
        out.push((key.as_str(), n));
    }
    out.sort_by(by_count_desc);
    out
}

/// `ranked_by_count_only` の比べ方。件数しか見ない。
fn by_count_desc(a: &(&str, u64), b: &(&str, u64)) -> Ordering {
    b.1.cmp(&a.1)
}

/// キーの昇順に並べ直す。
fn by_key(counts: &HashMap<String, u64>) -> BTreeMap<&str, u64> {
    let mut sorted = BTreeMap::new();
    for (key, &n) in counts {
        sorted.insert(key.as_str(), n);
    }
    sorted
}

fn main() {
    let lines = ["warn", "info", "warn", "error", "info", "debug"];
    let counts = count(&lines);

    println!("HashMap:  {counts:?}");
    println!("BTreeMap: {:?}", by_key(&counts));
    println!("件数だけ: {:?}", ranked_by_count_only(&counts));
    for (key, n) in ranked(&counts) {
        println!("{n} {key}");
    }
}

#[cfg(test)]
mod tests {
    use super::{by_count_desc, by_key, count, ranked, ranked_by_count_only};
    use std::cmp::Reverse;
    use std::collections::HashMap;

    #[test]
    fn 件数の降順で同数ならキーの昇順に並ぶ() {
        let counts = count(&["warn", "info", "warn", "error", "info", "debug"]);
        assert_eq!(
            ranked(&counts),
            vec![("info", 2), ("warn", 2), ("debug", 1), ("error", 1)]
        );
    }

    #[test]
    fn 件数だけで並べても件数の順序は守られる() {
        // 守られないのは同数の中の順序だけである。
        let counts = count(&["warn", "info", "warn", "error", "info", "debug"]);
        let mut seen = Vec::new();
        for (_, n) in ranked_by_count_only(&counts) {
            seen.push(n);
        }
        assert_eq!(seen, vec![2, 2, 1, 1]);
        assert_eq!(
            by_count_desc(&("a", 1), &("b", 1)),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn 等しい要素が無ければ安定でも不安定でも同じ順序になる() {
        let mut stable = vec![(Reverse(1_u64), "b"), (Reverse(2), "c"), (Reverse(1), "a")];
        let mut unstable = stable.clone();
        stable.sort();
        unstable.sort_unstable();
        assert_eq!(stable, unstable);
    }

    #[test]
    fn キーの比較はバイト列の順で大文字が先に来る() {
        let counts = count(&["error", "Warn", "info"]);
        assert_eq!(
            ranked(&counts),
            vec![("Warn", 1), ("error", 1), ("info", 1)]
        );
    }

    #[test]
    fn 空の入力からは空の並びが出る() {
        let counts = count(&[]);
        assert!(ranked(&counts).is_empty());
        assert!(by_key(&counts).is_empty());
    }

    #[test]
    fn btreemap_はキーの昇順で反復する() {
        let counts = count(&["warn", "info", "warn", "error"]);
        let mut keys = Vec::new();
        for (key, _) in by_key(&counts) {
            keys.push(key);
        }
        assert_eq!(keys, vec!["error", "info", "warn"]);
    }

    #[test]
    fn 入れる順序が違っても中身が同じならマップは等しい() {
        assert_eq!(count(&["a", "b", "a"]), count(&["b", "a", "a"]));
    }

    #[test]
    fn or_insert_は_match_と同じ数え方をする() {
        let lines = ["warn", "info", "warn"];
        let mut short: HashMap<String, u64> = HashMap::new();
        for &line in &lines {
            *short.entry(line.to_owned()).or_insert(0) += 1;
        }
        assert_eq!(short, count(&lines));
    }

    #[test]
    fn get_は範囲の外や無いキーで_none_を返す() {
        let counts = count(&["warn"]);
        let top = ranked(&counts);
        assert_eq!(top.first(), Some(&("warn", 1)));
        assert_eq!(top.get(10), None);
        assert_eq!(counts.get("info"), None);
        assert_eq!(counts.get("warn"), Some(&1));
    }

    #[test]
    fn total_cmp_は_nan_を含む_f64_も並べられる() {
        // NaN は符号で両端に分かれる。負の NaN は先頭、正の NaN は末尾に来る。
        let mut v = [3.0_f64, f64::NAN, 1.0, -f64::NAN, -0.0, 0.0];
        v.sort_by(f64::total_cmp);
        assert!(v[0].is_nan() && v[0].is_sign_negative());
        assert!(v[1] == 0.0 && v[1].is_sign_negative());
        assert!(v[2] == 0.0 && v[2].is_sign_positive());
        assert_eq!(&v[3..5], &[1.0, 3.0]);
        assert!(v[5].is_nan() && v[5].is_sign_positive());
    }
}
