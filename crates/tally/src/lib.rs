//! `tally` — 行指向データの度数集計。
//!
//! 学習用の題材だが、構成は実務の CLI と同じにしてある。
//!
//! - [`core`][]: I/O を持たない純粋な集計ロジック。ユニットテストはここに集中する。
//! - [`error`][]: ライブラリ境界の具体的なエラー型（`thiserror`）。
//! - [`cli`][]: 引数定義（`clap`）。`main.rs` から分離してテスト可能にしてある。
//!
//! バイナリ（`main.rs`）は「引数を読む・I/O を開く・結果を出す」だけを担い、
//! ロジックを持たない。これにより `cargo test` の大半がプロセス起動なしで回る。

pub mod cli;
pub mod core;
pub mod error;

pub use error::{Result, TallyError};
