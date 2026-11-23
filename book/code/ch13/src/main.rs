//! 13 章の実行ファイル。`Drop` の順、再帰の `Filter`、`Deref`、共有の集計表の動きを見る。

use std::cell::RefCell;

use book_ch13::{Filter, Label, SharedCounter, TraceGuard};

fn main() {
    let log = RefCell::new(Vec::new());
    {
        let first = TraceGuard::new("first", &log);
        let _second = TraceGuard::new("second", &log);
        drop(first);
    }
    println!("drop:    {:?}", log.borrow());

    let filter = Filter::And(
        Box::new(Filter::Not(Box::new(Filter::Code(200)))),
        Box::new(Filter::Code(404)),
    );
    println!(
        "filter:  200={}, 404={}",
        filter.matches(200),
        filter.matches(404)
    );

    let label = Label::new("api-east");
    println!("label:   {} ({} bytes)", &*label, label.len());

    let counter = SharedCounter::new();
    let east = counter.recorder("east");
    let west = counter.recorder("west");
    east.record(200);
    west.record(200);
    west.record(404);
    println!(
        "counts:  {:?} (east={}, west={})",
        counter.snapshot(),
        east.recorded(),
        west.recorded()
    );
}
