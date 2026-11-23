//! 12 章の実行ファイル。3 通りの分け方が同じ答えを返すことと、返る失敗が決まっていることを見る。

use std::num::NonZeroUsize;
use std::sync::Arc;

use book_ch12::{count, count_locked, count_spawned, count_split, parse_all};

fn main() {
    let codes = [200, 404, 200, 500, 404, 200, 301];
    let threads = NonZeroUsize::new(3).unwrap_or(NonZeroUsize::MIN);

    println!("1 本:    {:?}", count(&codes));
    println!("split:   {:?}", count_split(&codes, threads));
    println!("locked:  {:?}", count_locked(&codes, threads));
    println!("spawned: {:?}", count_spawned(&Arc::from(codes), threads));

    match parse_all(&["200", "x", "404", "y"], threads) {
        Ok(codes) => println!("{codes:?}"),
        Err(err) => println!("{err}"),
    }
}
