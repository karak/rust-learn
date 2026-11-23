//! 12 章「並列化とその前提（`Send` / `Sync`）」の実例。
//!
//! 状態コードの列をスレッドに分けて数える。同じ答えを 3 通りの分け方で出す。
//! 何を共有し、どの失敗を返すかという問題は `tally` と同じで、本物は `crates/tally/src/aggregate.rs` にある。
//!
//! 1 章から 11 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::collections::BTreeMap;
use std::num::{NonZeroUsize, ParseIntError};
use std::panic;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

/// 状態コードごとの件数。
pub type Counts = BTreeMap<u16, u64>;

/// 1 本のスレッドで数える。ほかの関数は、これと同じ答えを返す。
#[must_use]
pub fn count(codes: &[u16]) -> Counts {
    let mut counts = Counts::new();
    for &code in codes {
        *counts.entry(code).or_insert(0) += 1;
    }
    counts
}

/// 表を足し合わせる。足す順番を変えても答えは変わらない。
fn merge(parts: impl IntoIterator<Item = Counts>) -> Counts {
    let mut merged = Counts::new();
    for part in parts {
        for (code, n) in part {
            *merged.entry(code).or_insert(0) += n;
        }
    }
    merged
}

/// `threads` 本に分けるときの、1 本あたりの要素数。空の列でも 1 以上にする。
fn chunk_len(len: usize, threads: NonZeroUsize) -> usize {
    len.div_ceil(threads.get()).max(1)
}

/// スレッドごとに別の表で数え、最後に足し合わせる。
///
/// スレッドどうしが共有するのは、読むだけの入力 `codes` だけである。
/// `thread::scope` の中のスレッドは、スコープの外の値を借りてよい。
#[must_use]
pub fn count_split(codes: &[u16], threads: NonZeroUsize) -> Counts {
    let parts: Vec<Counts> = thread::scope(|s| {
        let handles: Vec<_> = codes
            .chunks(chunk_len(codes.len(), threads))
            .map(|chunk| s.spawn(move || count(chunk)))
            .collect();
        handles
            .into_iter()
            // スレッドの panic は、呼んだ側でそのまま起こし直す。
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|payload| panic::resume_unwind(payload))
            })
            .collect()
    });
    merge(parts)
}

/// 1 つの表を `Mutex` で守り、全スレッドがそこへ書く。
///
/// 1 件ごとにロックを取るので、スレッドどうしが順番待ちをする。
#[must_use]
pub fn count_locked(codes: &[u16], threads: NonZeroUsize) -> Counts {
    let counts = Mutex::new(Counts::new());
    thread::scope(|s| {
        for chunk in codes.chunks(chunk_len(codes.len(), threads)) {
            let counts = &counts;
            s.spawn(move || {
                for &code in chunk {
                    // ロックを握ったまま panic したスレッドがあると、`lock` は `Err` を返す。
                    // その panic は `scope` の終わりで呼んだ側に伝わるので、ここでは中身を使い続ける。
                    let mut counts = counts.lock().unwrap_or_else(PoisonError::into_inner);
                    *counts.entry(code).or_insert(0) += 1;
                }
            });
        }
    });
    counts.into_inner().unwrap_or_else(PoisonError::into_inner)
}

/// `thread::spawn` で数える。
///
/// `spawn` のスレッドはこの関数より長く生きうるので、入力を借りられない。
/// 入力を `Arc` で共有し、各スレッドが `Arc` の複製を 1 つずつ持って、自分の範囲を数える。
#[must_use]
pub fn count_spawned(codes: &Arc<[u16]>, threads: NonZeroUsize) -> Counts {
    let size = chunk_len(codes.len(), threads);
    let handles: Vec<_> = (0..codes.len())
        .step_by(size)
        .map(|start| {
            let codes = Arc::clone(codes);
            thread::spawn(move || {
                let end = (start + size).min(codes.len());
                count(&codes[start..end])
            })
        })
        .collect();
    merge(handles.into_iter().map(|handle| {
        handle
            .join()
            .unwrap_or_else(|payload| panic::resume_unwind(payload))
    }))
}

/// 状態コードとして読めなかった行。
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("{line_no} 行目: 状態コードとして読めません")]
pub struct LineError {
    /// 何行目か。1 始まり。
    pub line_no: usize,
    /// 読めなかった理由。
    #[source]
    pub source: ParseIntError,
}

/// 行をスレッドに分けて読む。
///
/// # Errors
///
/// 読めない行があれば、**いちばん前の行**の失敗を返す。
/// どのスレッドが先に終わっても、返る失敗は変わらない。
pub fn parse_all(lines: &[&str], threads: NonZeroUsize) -> Result<Vec<u16>, LineError> {
    let size = chunk_len(lines.len(), threads);
    thread::scope(|s| {
        let handles: Vec<_> = lines
            .chunks(size)
            .enumerate()
            .map(|(index, chunk)| s.spawn(move || parse_chunk(chunk, index * size)))
            .collect();
        let mut codes = Vec::with_capacity(lines.len());
        // 前の塊から順に待つ。塊の中では最初の失敗で止まるので、
        // 最初に見つかる `Err` が、いちばん前の行の失敗である。
        for handle in handles {
            let part = handle
                .join()
                .unwrap_or_else(|payload| panic::resume_unwind(payload));
            codes.extend(part?);
        }
        Ok(codes)
    })
}

/// 1 つの塊を読む。`offset` は塊の先頭が全体の何番目か。
fn parse_chunk(chunk: &[&str], offset: usize) -> Result<Vec<u16>, LineError> {
    chunk
        .iter()
        .enumerate()
        .map(|(index, line)| {
            line.parse().map_err(|source| LineError {
                line_no: offset + index + 1,
                source,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODES: [u16; 7] = [200, 404, 200, 500, 404, 200, 301];

    fn threads(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).expect("1 以上")
    }

    #[test]
    fn どの分け方でも_1_本で数えた答えと同じ() {
        let expected = count(&CODES);
        assert_eq!(
            expected,
            Counts::from([(200, 3), (301, 1), (404, 2), (500, 1)])
        );
        for n in [1, 2, 3, 7, 100] {
            assert_eq!(count_split(&CODES, threads(n)), expected, "{n} 本");
            assert_eq!(count_locked(&CODES, threads(n)), expected, "{n} 本");
            assert_eq!(
                count_spawned(&Arc::from(CODES), threads(n)),
                expected,
                "{n} 本"
            );
        }
    }

    #[test]
    fn 空の列は空の表になる() {
        assert!(count_split(&[], threads(4)).is_empty());
        assert!(count_locked(&[], threads(4)).is_empty());
        assert!(count_spawned(&Arc::from([]), threads(4)).is_empty());
    }

    #[test]
    fn 読めればすべての行を順に返す() {
        let lines = ["200", "404", "500"];
        assert_eq!(parse_all(&lines, threads(2)), Ok(vec![200, 404, 500]));
        assert_eq!(parse_all(&[], threads(2)), Ok(Vec::new()));
    }

    #[test]
    fn 失敗はいつもいちばん前の行() {
        // 2 本に分けると、4 行目と 5 行目は別のスレッドで読まれる。
        // 後ろの塊は先頭で失敗する。どちらが先に終わっても 4 行目を返す。
        let lines = ["200", "200", "200", "x", "y", "200", "200", "200"];
        for _ in 0..100 {
            let err = parse_all(&lines, threads(2)).expect_err("読めない行がある");
            assert_eq!(err.line_no, 4);
        }
    }

    #[test]
    fn 後ろの塊だけが失敗しても行番号は全体で数える() {
        let lines = ["200", "404", "500", "x"];
        let err = parse_all(&lines, threads(2)).expect_err("4 行目が読めない");
        assert_eq!(err.line_no, 4);
        assert_eq!(err.to_string(), "4 行目: 状態コードとして読めません");
    }

    #[test]
    #[should_panic(expected = "スレッドの中で")]
    fn resume_unwind_で起こし直した_panic_は呼んだ側に届く() {
        thread::scope(|s| {
            let handle = s.spawn(|| panic!("スレッドの中で"));
            handle
                .join()
                .unwrap_or_else(|payload| panic::resume_unwind(payload));
        });
    }
}
