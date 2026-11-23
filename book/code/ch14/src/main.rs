//! 14 章の実行ファイル。newtype、検証済み経路、`NonEmpty`、一時変更ガード、typestate の動きを見る。

use std::cell::RefCell;

use book_ch14::{Draft, Endpoint, FieldPath, Host, NonEmpty, OverrideGuard, format_url};

fn main() {
    let host = Host::new("example.com");
    let endpoint = Endpoint::new("/health");
    println!("url:       {}", format_url(&host, &endpoint));

    if let Ok(path) = FieldPath::new("/req/status") {
        let segments: Vec<&str> = path.segments().collect();
        println!("path:      {} -> {segments:?}", &*path);
    }

    let mut codes = NonEmpty::new(200u16);
    codes.push(404);
    println!("non_empty: first={}, len={}", codes.first(), codes.len());

    let level = RefCell::new("info");
    {
        let _guard = OverrideGuard::new(&level, "debug");
        println!("override:  inside={}", *level.borrow());
    }
    println!("override:  after={}", *level.borrow());

    let done = Draft::new().load(vec![200, 404, 200]).aggregate();
    println!("pipeline:  {:?}", done.counts());
}
