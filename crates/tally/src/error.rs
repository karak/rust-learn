//! CLI 固有の失敗と、そこから終了コードを決める規則。
//!
//! **ライブラリの失敗（[`TallyError`]）とは型を分けている**（[ADR-0004] 論点 5）。
//! cargo の `CliError`、jj の `CommandError` と同じ形。
//!
//! # なぜ `anyhow` を使わないのか
//!
//! **`CLAUDE.md` の「ライブラリ層は `thiserror`、バイナリ層は `anyhow`」を
//! ここでは破っている。理由を書く。**
//!
//! `anyhow::Error` は不透明なので、「エラー → 終了コード」を
//! **網羅性検査を受ける `match`** として書けない。ADR-0004 論点 5 が
//! `anyhow` を退けた決め手はそこにあり、`anyhow` を context のためだけに
//! 残すと **エラーの表現が 2 系統**になって、どちらに文脈を足すのかが
//! そのつど揺れる。よって一本化した。
//!
//! `anyhow` から失ったものは 2 つで、どちらも代替がある。
//!
//! | 失ったもの | 代替 |
//! | --- | --- |
//! | `.context("...")` による文脈の後付け | 文脈をバリアントとして型に持つ |
//! | `{:#}` によるチェーンの 1 行表示 | [`one_line`] |
//! | `Error::chain()` | [`one_line`] と同じ走査（`source()` を辿るだけ） |
//!
//! # 終了コード
//!
//! `crates/tally/docs/output-format.md` の契約は `0` / `1` / `2` の 3 つ。
//! **`2`（引数の誤り）はここを通らない** — `Cli::parse()` が
//! [`run`][crate] より前に返すため、clap 側に残る。
//!
//! [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md

use std::error::Error;
use std::fmt;
use std::io;

use tally_core::{LineErrorKind, TallyError};

/// 成功。
pub const EXIT_OK: u8 = 0;
/// 実行時エラー。
pub const EXIT_FAILURE: u8 = 1;

/// CLI が返す失敗。
///
/// **`hint` を必ず添える型**である。示唆が無い失敗のほうが多いので大半は
/// `None` になるが、[`CliError::new`] が引数で要求することで
/// **「この失敗に対して利用者は何ができるか」を打鍵時に問う**
/// （[ADR-0004] 論点 6）。
///
/// **`Default` も「hint 省略版のコンストラクタ」も置かない。**
/// 既定化した瞬間、その問いかけが消えて価値がゼロになる。
///
/// [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md
#[derive(Debug)]
pub struct CliError {
    kind: CliErrorKind,
    hint: Option<&'static str>,
}

/// 失敗の内訳。
///
/// **閉じている。** バリアントを足すと [`CliError::exit_code`] と
/// [`hint_for`] の `match` がコンパイルエラーになる。それが狙い。
#[derive(Debug, thiserror::Error)]
pub enum CliErrorKind {
    /// 集計そのものの失敗。
    ///
    /// **`#[error(transparent)]` にして文脈を足していない。** `anyhow` 時代は
    /// `.context("{path} の集計に失敗しました")` を被せていたが、
    /// 開けなかった場合は [`TallyError::OpenInput`] が既に path を持っており、
    /// **同じ path が 2 度出る**。入力は最大 1 つなので、
    /// 読み取り途中の失敗で path が出ないことは受け入れる
    /// （複数入力を扱うようになったら見直す）。
    #[error(transparent)]
    Tally(#[from] TallyError),

    /// 集計結果を書き出せなかった。
    ///
    /// パイプの下流が先に閉じた場合（`tally big.log | head`）もここに来る。
    /// [`CliError::exit_code`] がそれを成功として扱う。
    #[error("集計結果を書き出せません")]
    Write(#[source] io::Error),
}

impl CliError {
    /// **`hint` を引数で必ず受ける。** 省略できるコンストラクタを置かない理由は
    /// [`CliError`] のドキュメントを参照。
    #[must_use]
    pub fn new(kind: CliErrorKind, hint: Option<&'static str>) -> Self {
        Self { kind, hint }
    }

    /// 何が起きたか。
    #[must_use]
    pub fn kind(&self) -> &CliErrorKind {
        &self.kind
    }

    /// 利用者が次に何をすればよいか。**stderr にのみ出す。**
    ///
    /// stdout はデータ専用という契約（`crates/tally/docs/output-format.md`）を変えない。
    #[must_use]
    pub fn hint(&self) -> Option<&'static str> {
        self.hint
    }

    /// 終了コード。**純粋関数。**
    ///
    /// これがあることで、終了コードの決定規則を**プロセスを起動せずに**
    /// 網羅できる。`main` に述語を散らす形（`anyhow` 時代）では、
    /// 網羅の数がプロセス起動の数に比例し、かつ
    /// **「テストを書き忘れた」ことが検出されなかった**（[ADR-0004] 論点 5）。
    ///
    /// [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match &self.kind {
            // パイプの下流が先に閉じた場合（`tally big.log | head`）。
            // これは異常ではないので、静かに成功終了する。Unix ツールの作法。
            CliErrorKind::Write(source) if is_broken_pipe(source) => EXIT_OK,
            CliErrorKind::Tally(_) | CliErrorKind::Write(_) => EXIT_FAILURE,
        }
    }
}

impl fmt::Display for CliError {
    /// `kind` の文をそのまま出す。**hint は含めない**
    /// （表示するかは呼び出し側の判断で、`Display` に混ぜると
    /// ログに落としたときにも付いて回る）。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl Error for CliError {
    /// `kind` を飛ばしてその原因を指す。理由は `tally_core::error` の
    /// 「`Display` と `source()` の合成規則」と同じ。
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.kind.source()
    }
}

/// 失敗に対して利用者ができることがあれば返す。
///
/// **範囲を絞る**（[ADR-0004] 論点 6）。
///
/// - 書くのは **「利用者の操作で解決できる失敗」** だけ
/// - **入力そのものが壊れている場合には書かない。**
///   `InvalidJson` に対する「JSON を直せ」は情報量がゼロ
/// - 示唆の質が悪いと害になる。「`--strict` を外せ」は、
///   入力の検査が目的の利用者にとっては誤った助言 — だから範囲を絞る
///
/// [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md
#[must_use]
pub fn hint_for(err: &TallyError) -> Option<&'static str> {
    match err {
        // 開けない・読めないは利用者の操作で直せるが、
        // **`tally` の使い方を変えて直るものではない。** パスや権限の話なので、
        // メッセージ本体（path を含む）以上に言えることが無い。
        TallyError::OpenInput { .. } | TallyError::Read(_) => None,
        TallyError::Line(line) => match &line.kind {
            LineErrorKind::MissingField { .. } => {
                Some("--strict を外すと、この行はスキップされます")
            }
            // `UnsupportedFieldType` と `InvalidJson` は入力そのものの問題。
            //
            // **`_` を書かざるをえないのは `LineErrorKind` が
            // `#[non_exhaustive]` だから**（ADR-0004 論点 2 の帰結）。
            // 別クレートからは網羅性検査を受けられないので、
            // バリアントが増えたときここは黙って `None` になる。
            _ => None,
        },
    }
}

/// エラーチェーンを 1 行に連結する。`anyhow` の `{:#}` 相当。
///
/// **`anyhow` を落としたので手で書いている。** `anyhow::Error::chain()` は
/// [`Error::source`] を辿るだけの糖衣で、型が決まっていれば依存なしに書ける。
#[must_use]
pub fn one_line(err: &(dyn Error + 'static)) -> String {
    let mut shown = Vec::new();
    let mut current = Some(err);
    while let Some(err) = current {
        shown.push(err.to_string());
        current = err.source();
    }
    shown.join(": ")
}

/// エラーチェーンのどこかに `BrokenPipe` があるか。
///
/// 直接の `io::Error` だけを見ると足りない。`serde_json` を経由した書き出しでは
/// `serde_json::Error` から `io::Error` に戻る経路があり、
/// **包まれた状態で届きうる。**
fn is_broken_pipe(err: &(dyn Error + 'static)) -> bool {
    let mut current = Some(err);
    while let Some(err) = current {
        let broken = err
            .downcast_ref::<io::Error>()
            .is_some_and(|io_err| io_err.kind() == io::ErrorKind::BrokenPipe);
        if broken {
            return true;
        }
        current = err.source();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tally_core::LineError;

    fn write_err(kind: io::ErrorKind) -> CliError {
        CliError::new(CliErrorKind::Write(io::Error::from(kind)), None)
    }

    fn line_err(kind: LineErrorKind) -> TallyError {
        TallyError::Line(LineError::new(1, "{\"other\":1}", kind))
    }

    fn missing_field() -> TallyError {
        line_err(LineErrorKind::MissingField {
            field: "lvl".into(),
        })
    }

    // --- 終了コード ---
    //
    // **プロセスを起動せずに網羅する。** これが CLI 固有のエラー型を
    // 持った目的（ADR-0004 論点 5）。統合テストは配線の確認だけに残す。

    #[test]
    fn 集計の失敗は終了コード_1() {
        let err = CliError::new(CliErrorKind::Tally(missing_field()), None);
        assert_eq!(err.exit_code(), EXIT_FAILURE);
    }

    #[test]
    fn 入力を開けない場合も終了コード_1() {
        let err = CliError::new(
            CliErrorKind::Tally(TallyError::OpenInput {
                path: "/nope".into(),
                source: io::Error::from(io::ErrorKind::NotFound),
            }),
            None,
        );
        assert_eq!(err.exit_code(), EXIT_FAILURE);
    }

    #[test]
    fn 読み取りの失敗も終了コード_1() {
        let err = CliError::new(
            CliErrorKind::Tally(TallyError::Read(io::Error::from(
                io::ErrorKind::InvalidData,
            ))),
            None,
        );
        assert_eq!(err.exit_code(), EXIT_FAILURE);
    }

    #[test]
    fn 書き出しの失敗は終了コード_1() {
        assert_eq!(
            write_err(io::ErrorKind::StorageFull).exit_code(),
            EXIT_FAILURE
        );
    }

    #[test]
    fn パイプの下流が閉じた場合は成功扱い() {
        // `tally big.log | head` は異常ではない。Unix ツールの作法。
        assert_eq!(write_err(io::ErrorKind::BrokenPipe).exit_code(), EXIT_OK);
    }

    #[test]
    fn 包まれた_broken_pipe_も成功扱い() {
        // serde_json を経由した書き出しでは io::Error が包まれて届きうる。
        // 直接の kind() だけを見る実装ではここが落ちる。
        #[derive(Debug)]
        struct Wrapped(io::Error);
        impl fmt::Display for Wrapped {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("包んだ")
            }
        }
        impl Error for Wrapped {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                Some(&self.0)
            }
        }

        let inner = io::Error::other(Wrapped(io::Error::from(io::ErrorKind::BrokenPipe)));
        let err = CliError::new(CliErrorKind::Write(inner), None);
        assert_eq!(err.exit_code(), EXIT_OK);
    }

    // --- hint ---

    #[test]
    fn 欠損フィールドには_strict_の示唆が付く() {
        let hint = hint_for(&missing_field()).expect("示唆があるはず");
        assert!(hint.contains("--strict"), "実際: {hint}");
    }

    #[test]
    fn 壊れた入力には示唆を付けない() {
        // 「JSON を直せ」は情報量がゼロ。
        let unsupported = line_err(LineErrorKind::UnsupportedFieldType {
            field: "tags".into(),
        });
        assert_eq!(hint_for(&unsupported), None);
    }

    #[test]
    fn 入力を開けない場合には示唆を付けない() {
        let err = TallyError::OpenInput {
            path: "/nope".into(),
            source: io::Error::from(io::ErrorKind::NotFound),
        };
        assert_eq!(hint_for(&err), None);
    }

    #[test]
    fn hint_は既定で無い() {
        assert_eq!(write_err(io::ErrorKind::StorageFull).hint(), None);
    }

    // --- 表示 ---

    #[test]
    fn 表示に_hint_は混ざらない() {
        // hint を出すかは呼び出し側の判断。Display に混ぜると
        // ログへ落としたときにも付いて回る。
        let err = CliError::new(
            CliErrorKind::Tally(missing_field()),
            Some("この文は Display に出てはいけない"),
        );
        assert!(
            !err.to_string()
                .contains("この文は Display に出てはいけない")
        );
    }

    #[test]
    fn one_line_はチェーンを連結する() {
        let err = CliError::new(
            CliErrorKind::Write(io::Error::from(io::ErrorKind::StorageFull)),
            None,
        );
        let shown = one_line(&err);
        assert!(shown.starts_with("集計結果を書き出せません: "), "{shown}");
        // 原因（io::Error の文）が続く。
        assert!(shown.len() > "集計結果を書き出せません: ".len(), "{shown}");
    }

    #[test]
    fn one_line_は集計の失敗に文脈を足さない() {
        // `#[error(transparent)]` なので、CLI 側の層は何も被せない。
        let inner = missing_field();
        let expected = inner.to_string();
        let err = CliError::new(CliErrorKind::Tally(inner), None);
        assert!(one_line(&err).starts_with(&expected), "{}", one_line(&err));
    }
}
