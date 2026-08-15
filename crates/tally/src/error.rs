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

    /// `--strict` 指定時に、キーを取り出せない行を見つけた。
    ///
    /// **抜粋は `{:?}`（`str` の `Debug`）で出す。** 入力は信頼できず、
    /// 表示を撹乱する文字をそのまま stderr へ流すと、端末の表示を操作されうる
    /// （ログインジェクション）。`Debug` は非表示・非印字の文字を
    /// 短縮形（`\r` `\n` `\t`）か `\u{..}` にエスケープし、全体を引用符で囲むため、
    /// 追加依存なしにこれを防げる。
    ///
    /// **「JSON なら危険な文字は入らない」は誤り。** JSON がエスケープを要求するのは
    /// C0 制御文字（U+0000..=U+001F）だけで、それ以外は生で書ける。実際に
    /// U+202E（RTL override、表示順を反転させる）や U+007F（DEL）は
    /// 合法な JSON 文字列の中を素通りしてここへ到達する。**エスケープは必要。**
    ///
    /// なお **stdout 側では意図的にエスケープしていない。** stdout は
    /// 「集計結果というデータ」であり、他のツールへパイプで渡す前提でバイト透過に保つ。
    /// エスケープしてよいのは、人間が読む診断出力である stderr のほうだけ。
    /// この非対称は仕様であって漏れではない。
    /// **`String` ではなく `Box<str>` を使っている。**
    /// `Result<T, TallyError>` の大きさは最大バリアントで決まり、その `Result` は
    /// **1 行ごとに 3 段（`extract` → `select` → `push_line`）返される。**
    /// つまり **失敗しない行もこの大きさを払う。** エラー型を太らせると
    /// 成功パスに課税されるのが、Rust のエラー設計で見落としやすい点。
    ///
    /// `String` は容量フィールドのぶん 24 バイト、`Box<str>` は 16 バイト。
    /// 以後変更しない文字列なので容量は要らない。実測（aarch64）:
    /// このバリアントの payload は 56 → 40 バイト、`TallyError` 全体は 56 → 48 バイト。
    ///
    /// 段階 2 以前の 40 バイトまで戻すには payload ごと `Box` に入れる必要があるが、
    /// 一段の間接参照とパターンマッチの読みにくさに見合わないと判断した。
    #[error("{line_no} 行目からフィールド `{field}` を取り出せません: {snippet:?}")]
    MissingField {
        line_no: usize,
        field: Box<str>,
        /// 該当行の先頭を切り詰めたもの。
        snippet: Box<str>,
    },
}

/// このクレート共通の `Result`。
pub type Result<T, E = TallyError> = std::result::Result<T, E>;
