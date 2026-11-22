//! 8 章「イテレータとクロージャ」の実例。
//!
//! 7 章で `for` と `push` で書いた詰め替えと、名前を付けて渡した比べる関数を、
//! イテレータとクロージャで書き直す。本物の集計は `crates/tally-core/src/count.rs` にある。
//!
//! 1 章から 7 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::any::type_name_of_val;
use std::collections::{BTreeMap, HashMap};
use std::num::ParseIntError;

/// 行ごとに出現回数を数える。
fn count(lines: &[&str]) -> HashMap<String, u64> {
    let mut counts = HashMap::new();
    for &line in lines {
        *counts.entry(line.to_owned()).or_insert(0) += 1;
    }
    counts
}

/// 件数の降順、同数ならキーの昇順に並べる。
///
/// 7 章の `ranked` と同じ順序を、詰め替えは `collect` で、比べ方はクロージャで書く。
fn ranked(counts: &HashMap<String, u64>) -> Vec<(&str, u64)> {
    let mut out: Vec<(&str, u64)> = counts.iter().map(|(key, &n)| (key.as_str(), n)).collect();
    // `then_with` は、前の比較が `Equal` のときだけ後ろのクロージャを呼ぶ。
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    out
}

/// キーの昇順に並べ直す。
///
/// 戻り値の型が `BTreeMap` なので、`collect` は `BTreeMap` を作る。
fn by_key(counts: &HashMap<String, u64>) -> BTreeMap<&str, u64> {
    counts.iter().map(|(key, &n)| (key.as_str(), n)).collect()
}

/// 件数が `min` 以上のキーを、キーの昇順で返す。
///
/// `filter` に渡すクロージャは、引数の `min` を借りている。
fn at_least(counts: &HashMap<String, u64>, min: u64) -> Vec<&str> {
    let mut keys: Vec<&str> = counts
        .iter()
        .filter(|&(_, &n)| n >= min)
        .map(|(key, _)| key.as_str())
        .collect();
    // キーは重複しないので、安定かどうかで結果は変わらない。
    keys.sort_unstable();
    keys
}

/// 全件数の合計。
fn total(counts: &HashMap<String, u64>) -> u64 {
    counts.values().sum()
}

/// 数字の列を読む。1 つでも読めなければ、最初の失敗を返す。
///
/// `Result` へ `collect` すると、最初の `Err` で止まる。
fn parse_all(fields: &[&str]) -> Result<Vec<u64>, ParseIntError> {
    fields.iter().map(|field| field.parse::<u64>()).collect()
}

fn main() {
    let lines = ["warn", "info", "warn", "error", "info", "debug"];
    let counts = count(&lines);

    for (key, n) in ranked(&counts) {
        println!("{n} {key}");
    }
    println!("キーの順: {:?}", by_key(&counts));
    println!("2 件以上: {:?}", at_least(&counts, 2));
    println!("合計: {}", total(&counts));
    println!("{:?}", parse_all(&["3", "x", "5"]));

    // アダプタは、呼ばれた順に段を重ねた型の値になる。まだ何も反復していない。
    let doubled = [1_u64, 2, 3].iter().filter(|&&n| n >= 2).map(|n| n * 10);
    println!("{}", type_name_of_val(&doubled));

    // 要素は 1 つずつ、`map` と `filter` を通り抜けてから次へ進む。
    let picked: Vec<u64> = [1_u64, 2, 3]
        .iter()
        .map(|n| {
            println!("map {n}");
            n * 10
        })
        .filter(|n| {
            println!("filter {n}");
            *n >= 20
        })
        .collect();
    println!("{picked:?}");
}

#[cfg(test)]
mod tests {
    use super::{at_least, by_key, count, parse_all, ranked, total};
    use std::mem::size_of_val;

    #[test]
    fn 件数の降順で同数ならキーの昇順に並ぶ() {
        let counts = count(&["warn", "info", "warn", "error", "info", "debug"]);
        assert_eq!(
            ranked(&counts),
            vec![("info", 2), ("warn", 2), ("debug", 1), ("error", 1)]
        );
    }

    #[test]
    fn collect_は戻り値の型で作るものを変える() {
        let counts = count(&["b", "a", "b"]);
        let keys: Vec<&str> = by_key(&counts).into_keys().collect();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn 下限を満たすキーだけが残る() {
        let counts = count(&["warn", "info", "warn", "error", "info", "debug"]);
        assert_eq!(at_least(&counts, 2), vec!["info", "warn"]);
        assert_eq!(at_least(&counts, 3), Vec::<&str>::new());
    }

    #[test]
    fn 空の入力でも合計と並びが決まる() {
        let counts = count(&[]);
        assert_eq!(total(&counts), 0);
        assert!(ranked(&counts).is_empty());
    }

    #[test]
    fn result_への_collect_は最初の失敗で止まる() {
        assert_eq!(parse_all(&["1", "2"]), Ok(vec![1, 2]));
        assert_eq!(parse_all(&[]), Ok(vec![]));

        let mut tried = Vec::new();
        let parsed: Result<Vec<u64>, _> = ["1", "x", "y"]
            .iter()
            .map(|field| {
                tried.push(*field);
                field.parse::<u64>()
            })
            .collect();
        assert!(parsed.is_err());
        // `y` には触れていない。
        assert_eq!(tried, vec!["1", "x"]);
    }

    #[test]
    fn 閉じ込めたものの大きさがクロージャの大きさになる() {
        let none = || 1;
        let min = 2_u64;
        let by_ref = |n: u64| n >= min;
        let s = String::from("warn");
        let by_move = move || s.len();
        assert_eq!(size_of_val(&none), 0);
        assert_eq!(size_of_val(&by_ref), size_of::<&u64>());
        assert_eq!(size_of_val(&by_move), size_of_val(&String::new()));
        assert_eq!((none(), by_ref(3), by_move()), (1, true, 4));
    }

    #[test]
    fn move_で写した_copy_型は元の変数と別になる() {
        let mut n = 1_u64;
        let mut add = move || {
            n += 1;
            n
        };
        assert_eq!((add(), add()), (2, 3));
        assert_eq!(n, 1);
    }

    #[test]
    fn クロージャはフィールド単位で借りる() {
        struct Tally {
            keys: Vec<String>,
            total: u64,
        }
        let mut t = Tally {
            keys: vec![String::from("warn")],
            total: 0,
        };
        let mut bump = || t.total += 1;
        // `bump` が借りているのは `t.total` だけなので、`t.keys` は読める。
        assert_eq!(t.keys.len(), 1);
        bump();
        assert_eq!(t.total, 1);
    }
}
