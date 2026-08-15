//! ライブラリ境界のエラー型。
//!
//! 方針: **ライブラリは `thiserror` で具体的な列挙型を返し、バイナリは `anyhow` で束ねる。**
//! 呼び出し側が「どのエラーか」で分岐できる余地を残すのがライブラリの責務であり、
//! `anyhow::Error` を公開 API に出した時点でその余地は失われる。

use std::path::PathBuf;

/// `tally` のライブラリ層で発生しうる失敗。
#[derive(Debug, thiserror::Error)]
pub enum TallyError {
    /// 入力ファイルを開けなかった。
    ///
    /// `#[source]` で元の `io::Error` を保持しているため、
    /// 呼び出し側は `std::error::Error::source()` で原因まで辿れる。
    #[error("入力を読めません: {path}")]
    OpenInput {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// 読み取り中の I/O 失敗。
    #[error("入力の読み取りに失敗しました")]
    Read(#[from] std::io::Error),

    /// `--field` 指定時に、行が JSON として解釈できなかった。
    ///
    /// 何行目かを持たせているのは、CLI のエラーメッセージとして
    /// 「どこを直せばよいか」が分からないと役に立たないため。
    #[error("{line_no} 行目を JSON として解釈できません")]
    InvalidJson {
        line_no: usize,
        #[source]
        source: serde_json::Error,
    },

    /// 抽出対象のフィールドが文字列でも数値でもなかった。
    #[error("{line_no} 行目のフィールド `{field}` は文字列・数値・真偽値ではありません")]
    UnsupportedFieldType { line_no: usize, field: String },
}

/// このクレート共通の `Result`。
pub type Result<T, E = TallyError> = std::result::Result<T, E>;
