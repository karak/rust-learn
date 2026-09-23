//! `tally` — 行指向データの度数集計 CLI。
//!
//! **集計そのものは [`tally_core`] にある。** このクレートが持つのは
//! CLI の関心事だけ。
//!
//! - [`aggregate`] — 並列化ポリシー（**入力の種類を知らない**。[ADR-0007] 論点 5）
//! - [`cli`] — 引数定義（`clap`）と、引数から `tally_core` の型への変換
//! - [`input`] — 入力を開いて 1 単位を集計する（**I/O の境界**）
//! - [`mod@format`] — 集計結果の整形
//! - [`error`] — CLI 固有の失敗、終了コード、hint
//!
//! 貫かれている不変条件は 2 つ。
//!
//! - **バイナリ（`main.rs`）はロジックを持たない。**
//!   「引数を読む・I/O を開く・結果を出す」だけを担う
//! - **依存の向きは `tally` → [`tally_core`] の一方向。**
//!   `tally_core` は `clap` にも `anyhow` にも依存しない
//!
//! これにより、テストの大半がプロセス起動なしで回る。
//!
//! **どこに何を置くか、なぜそこなのかは `crates/tally/docs/layout.md` が正本。**
//! **モジュール間で許される依存（層）もそこにある。**
//! `scripts/check-module-deps.sh` が機械的に検査する。
//!
//! [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md

pub mod aggregate;
pub mod cli;
pub mod error;
pub mod format;
pub mod input;
