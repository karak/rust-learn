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
use std::num::NonZeroUsize;
use std::path::PathBuf;

use tally_core::{LineErrorKind, TallyError};

/// 成功。
pub const EXIT_OK: u8 = 0;
/// 実行時エラー。
pub const EXIT_FAILURE: u8 = 1;

/// 入力の呼び名。**診断に出すためだけの型。**
///
/// 標準入力には path が無いので、`Option<PathBuf>` では
/// 「持つが空」という状態が生まれる。**それを型で潰している**
/// （[ADR-0004] 論点 1 が `LineError` で採ったのと同じ形）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputName {
    /// 標準入力。
    Stdin,
    /// ファイル。
    Path(PathBuf),
}

impl fmt::Display for InputName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stdin => f.write_str("標準入力"),
            // `Path` は `Display` を実装しないので `.display()` を通す。
            // thiserror の書式指定と違い、ここは素の `std::fmt` である。
            Self::Path(path) => write!(f, "{}", path.display()),
        }
    }
}

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
    /// 入力を開けなかった。
    ///
    /// **[`TallyError`] ではなくここにある。** `tally_core` はファイルを開かないので、
    /// 「開けなかった」は開いた側の語彙である（[ADR-0007] 論点 3）。
    ///
    /// **標準入力では起きない**ので、[`InputName`] ではなく [`PathBuf`] を直接持つ。
    ///
    /// `path` の書式指定に `.display()` を書いていないのは、**thiserror 2 が
    /// `Path` / `PathBuf` を特別扱いする**ため。1.x では `#[error("{}", path.display())]`
    /// と書く必要があった。
    ///
    /// [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md
    #[error("入力を読めません: {path}")]
    Open {
        /// 開こうとした対象。
        path: PathBuf,
        /// 元の I/O 失敗。
        #[source]
        source: io::Error,
    },

    /// 集計そのものの失敗。**どの入力で起きたかを必ず持つ。**
    ///
    /// **段階 5 では `#[error(transparent)]` で文脈を足していなかった。**
    /// 当時は `TallyError::OpenInput` が path を持っていたため、
    /// 文脈を被せると **同じ path が 2 度出た**。
    /// `OpenInput` を [`CliErrorKind::Open`] へ移した結果、その二重は消えた
    /// （[ADR-0007] 論点 3）。
    ///
    /// **名前が要るのは行番号のためでもある。** [`tally_core::LineError`] の
    /// 行番号は入力ごとに 1 から数えるので、
    /// **どの入力かが言えないと行番号まで意味を失う。**
    ///
    /// [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md
    #[error("{name} の集計に失敗しました")]
    Input {
        /// どの入力か。
        name: InputName,
        /// 元の失敗。
        #[source]
        source: TallyError,
    },

    /// 並列実行を準備できなかった。
    ///
    /// **スレッドを立てられない**（OS の上限、資源の枯渇）場合にここへ来る。
    /// 集計そのものは始まっていない。
    #[error("{requested} スレッドの並列実行を準備できません")]
    Threads {
        /// 要求したスレッド数。
        requested: NonZeroUsize,
        /// 元の失敗。**不透明な型で包んでいる**（[`ExecutionError`] を参照）。
        #[source]
        source: ExecutionError,
    },

    /// 集計結果を書き出せなかった。
    ///
    /// パイプの下流が先に閉じた場合（`tally big.log | head`）もここに来る。
    /// [`CliError::exit_code`] がそれを成功として扱う。
    #[error("集計結果を書き出せません")]
    Write(#[source] io::Error),
}

impl CliErrorKind {
    /// [`CliErrorKind::Threads`] を組み立てる。
    ///
    /// **`rayon` の型を引数で受けない。** 受けると `error` モジュールが
    /// `rayon` を知ることになり、層の規則（`crates/tally/docs/layout.md`）を破る。
    pub(crate) fn threads(
        requested: NonZeroUsize,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self::Threads {
            requested,
            source: ExecutionError(Box::new(source)),
        }
    }
}

/// 並列実行の準備に失敗した原因。**中身を公開しない。**
///
/// `rayon::ThreadPoolBuildError` をそのまま持つと、**`rayon` が公開依存になる**
/// （[ADR-0004] 論点 4 が `serde_json::Error` に対して採ったのと同じ判断）。
/// 加えて、このモジュールは層 1 であり **`rayon` を知らない**
/// （`crates/tally/docs/layout.md` の層の表）。
///
/// [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md
#[derive(Debug)]
pub struct ExecutionError(Box<dyn Error + Send + Sync>);

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for ExecutionError {
    /// **`self.0` を原因として指さない。** 指すと `Display` が二重に出る
    /// （`tally_core::error` の「`Display` と `source()` の合成規則」と同じ理由）。
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
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
            CliErrorKind::Open { .. }
            | CliErrorKind::Input { .. }
            | CliErrorKind::Threads { .. }
            | CliErrorKind::Write(_) => EXIT_FAILURE,
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
        // 読めないのは利用者の操作で直せるが、
        // **`tally` の使い方を変えて直るものではない。** パスや権限の話なので、
        // メッセージ本体（入力の名前を含む）以上に言えることが無い。
        // **開けなかった場合は `CliErrorKind::Open` になり、ここを通らない**
        // （ADR-0007 論点 3 で `OpenInput` を CLI へ移した）。
        TallyError::Read(_) => None,
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

    /// 標準入力の集計が失敗した形。**テストの大半は名前を問わない**ので、
    /// 既定を 1 つ決めて短く書けるようにする。
    fn input_kind(source: TallyError) -> CliErrorKind {
        CliErrorKind::Input {
            name: InputName::Stdin,
            source,
        }
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
        let err = CliError::new(input_kind(missing_field()), None);
        assert_eq!(err.exit_code(), EXIT_FAILURE);
    }

    #[test]
    fn 入力を開けない場合も終了コード_1() {
        // 開けない失敗は `Open`。**`Input` を通らない**（ADR-0007 論点 3）。
        let err = CliError::new(
            CliErrorKind::Open {
                path: "/nope".into(),
                source: io::Error::from(io::ErrorKind::NotFound),
            },
            None,
        );
        assert_eq!(err.exit_code(), EXIT_FAILURE);
    }

    #[test]
    fn 読み取りの失敗も終了コード_1() {
        let err = CliError::new(
            input_kind(TallyError::Read(io::Error::from(
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
    fn 読み取りの失敗には示唆を付けない() {
        // 開けない失敗は `CliErrorKind::Open` になり、`hint_for` を通らない
        // （ADR-0007 論点 3 で `OpenInput` を CLI へ移した）。
        let err = TallyError::Read(io::Error::from(io::ErrorKind::InvalidData));
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
            input_kind(missing_field()),
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
    fn one_line_は集計の失敗に入力の名前を前置する() {
        // 段階 5 では `#[error(transparent)]` で何も被せなかった。
        // 複数入力では **どの入力の何行目かが言えなくなる**ので、
        // 名前を前置する形に変えた（ADR-0007 論点 3）。
        let inner = missing_field();
        let expected = inner.to_string();
        let err = CliError::new(
            CliErrorKind::Input {
                name: InputName::Path("a.log".into()),
                source: inner,
            },
            None,
        );
        let shown = one_line(&err);
        assert!(shown.starts_with("a.log の集計に失敗しました: "), "{shown}");
        // 元の失敗（行番号と抜粋を含む）は落ちない。
        assert!(shown.ends_with(&expected), "{shown}");
    }

    #[test]
    fn 標準入力にも呼び名がある() {
        // path が無い入力を `Option<PathBuf>` の `None` で表すと、
        // 表示の場合分けが呼び出し側に漏れる。
        let err = CliError::new(input_kind(missing_field()), None);
        assert!(
            one_line(&err).starts_with("標準入力 の集計に失敗しました: "),
            "{}",
            one_line(&err)
        );
    }
}
