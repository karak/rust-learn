//! 6 章の実行ファイル。
//!
//! `lib.rs` とは別のクレートなので、`book_ch06` が `pub use` したものしか使えない。
//! `Tally` のフィールドにも、`FieldName` の中の `String` にも、ここからは触れない。

use book_ch06::{FieldName, Tally};

fn main() {
    let field = match FieldName::new("level") {
        Ok(field) => field,
        Err(e) => {
            eprintln!("名前が正しくない: {e:?}");
            return;
        }
    };

    let mut tally = Tally::new(field).strict(false);
    for line in ["level=warn user=a", "level=info", "user=b", "level=warn"] {
        if let Err(e) = tally.push_line(line) {
            eprintln!("{} 行目にフィールドが無い", e.line_no);
            return;
        }
    }
    println!(
        "{}: warn={}",
        tally.field().as_str(),
        tally.count_of("warn")
    );

    let report = tally.finish();
    println!("skipped={}", report.skipped);
}
