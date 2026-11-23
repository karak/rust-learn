//! 12 章の演習の答え。
//!
//! `cargo test -p book-ch12` で確かめられる。

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::thread;

/// 演習 1: いちばん大きい値を、スレッドに分けて探す。空なら `None`。
///
/// 各スレッドが自分の塊の最大を返し、最後にその中の最大を取る。
/// `thread::scope` なので、`values` を借りたまま渡せる。
fn max_parallel(values: &[u64], threads: NonZeroUsize) -> Option<u64> {
    let size = values.len().div_ceil(threads.get()).max(1);
    thread::scope(|s| {
        let handles: Vec<_> = values
            .chunks(size)
            .map(|chunk| s.spawn(move || chunk.iter().copied().max()))
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().expect("数えるだけでは panic しない"))
            .max()
    })
}

/// 演習 2: 行を `thread::spawn` で分けて読み、語ごとの件数を 1 つの表に数え入れる。
///
/// `spawn` のスレッドは呼び手より長く生きうるので、入力も表も `Arc` で共有する。
/// 表は全スレッドが書くので、`Mutex` で包む。
fn count_words_spawned(lines: Vec<String>, threads: NonZeroUsize) -> BTreeMap<String, u64> {
    let lines: Arc<[String]> = Arc::from(lines);
    let counts = Arc::new(Mutex::new(BTreeMap::new()));
    let size = lines.len().div_ceil(threads.get()).max(1);
    let handles: Vec<_> = (0..lines.len())
        .step_by(size)
        .map(|start| {
            let lines = Arc::clone(&lines);
            let counts = Arc::clone(&counts);
            thread::spawn(move || {
                let end = (start + size).min(lines.len());
                for line in &lines[start..end] {
                    for word in line.split_whitespace() {
                        let mut counts =
                            counts.lock().expect("握ったまま panic するスレッドは無い");
                        *counts.entry(word.to_owned()).or_insert(0) += 1;
                    }
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().expect("数えるだけでは panic しない");
    }
    // すべてのスレッドを待ったので、`Arc` を持っているのはここだけになっている。
    Arc::into_inner(counts)
        .expect("ほかの持ち手は残っていない")
        .into_inner()
        .expect("握ったまま panic するスレッドは無い")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn threads(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).expect("1 以上")
    }

    #[test]
    fn いちばん大きい値を返す() {
        let values = [3, 41, 5, 9, 26, 5];
        for n in [1, 2, 4, 10] {
            assert_eq!(max_parallel(&values, threads(n)), Some(41), "{n} 本");
        }
    }

    #[test]
    fn 空なら_none() {
        assert_eq!(max_parallel(&[], threads(3)), None);
    }

    #[test]
    fn 語ごとに数える() {
        let lines = vec![
            "warn disk".to_owned(),
            "error disk".to_owned(),
            "warn net".to_owned(),
        ];
        for n in [1, 2, 5] {
            let counts = count_words_spawned(lines.clone(), threads(n));
            let expected = BTreeMap::from([
                ("disk".to_owned(), 2),
                ("error".to_owned(), 1),
                ("net".to_owned(), 1),
                ("warn".to_owned(), 2),
            ]);
            assert_eq!(counts, expected, "{n} 本");
        }
    }

    #[test]
    fn 行が無ければ空の表() {
        assert!(count_words_spawned(Vec::new(), threads(3)).is_empty());
    }
}
