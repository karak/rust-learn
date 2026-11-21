//! 6 章「構造体・メソッド・可視性」の実例。
//!
//! `tally` の集計器から、フィールドを隠した構造体と、検査してから作る型を抜き出したもの。
//! 本物は `crates/tally-core/src/count.rs` と `crates/tally-core/src/select.rs` にある。
//!
//! 本物と同じく、ライブラリ（このファイル）と実行ファイル（`main.rs`）を別のクレートに分けている。
//! `main.rs` からは、ここで `pub use` したものしか見えない。
//! 可視性の境界を、本物と同じ形で試せるようにするためである。
//!
//! 1 章から 5 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;
mod tally;

pub use tally::{FieldName, FieldNameError, MissingField, Report, Tally};
