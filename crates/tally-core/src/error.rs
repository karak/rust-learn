//! 失敗の型。
//!
//! 方針: **`thiserror` で具体的な列挙型を返す。** 呼び出し側が「どのエラーか」で
//! 分岐できる余地を残すのがライブラリの責務であり、`anyhow::Error` のような
//! 不透明な型を公開 API に出した時点でその余地は失われる。
//!
//! 形の決定過程は [ADR-0004] が正本。要点だけ:
//!
//! - **行に紐づく失敗は [`LineError`] に括り出す。** 共有する文脈（行番号・抜粋）を
//!   構造体のフィールドに、判別と個別データを内側の [`LineErrorKind`] に置く。
//!   「行番号を持つが抜粋を持たない」状態が **型として存在しなくなる**
//! - [`TallyError`] は **閉じている**（`#[non_exhaustive]` を付けない）。
//!   分類の骨格なので、増えたら消費者の終了コード規則が網羅性検査に落ちてほしい
//! - [`LineErrorKind`] は **開いている**。ルールが増えるたびに伸びる側なので、
//!   バリアント追加で消費者を壊さない
//! - [`JsonError`] は不透明な newtype。**`serde_json` を公開依存にしない**
//!
//! # `Display` と `source()` の合成規則
//!
//! **ADR-0004 が決めずに実装へ持ち越した唯一の項目。ここで決める。**
//!
//! 2 段の型（[`LineError`] と [`LineErrorKind`]）を素朴に `#[source]` で繋ぐと、
//! **エラーチェーンを連結して表示したときに同じ文が 2 度出る。**
//! 外側の `Display` に内側の文を埋め込んでおきながら、`source()` でも
//! その内側を指すことになるため。規則はこう置く。
//!
//! | 層 | `Display` が描くもの | `source()` が指すもの |
//! | --- | --- | --- |
//! | [`TallyError::Line`] | **何も足さない**（`#[error(transparent)]`） | 内側の [`LineError`] |
//! | [`LineError`] | 行番号 + `kind` の文 + 抜粋（**1 行で自己完結する**） | `kind` を**飛ばして** `kind` の原因 |
//! | [`LineErrorKind`] | 自分の分の文だけ | [`JsonError`]（`InvalidJson` のときだけ） |
//!
//! **`LineError` の `source()` が `kind` を飛ばすのが要点。** これにより
//! `LineError::to_string()` は単体で読めるまま、チェーンの連結表示は重複しない。
//! `#[source]` を付ける素朴な形では両立しないので、[`std::error::Error`] を手書きしている。
//!
//! `LineError` の 1 行が自己完結する側を選んだ理由: 抜粋のエスケープ
//! （制御文字を生のまま stderr へ流さない）は **単一のエラーの `to_string()`**
//! で成り立っていなければならない。消費者がチェーンを辿らずに 1 件だけ
//! ログへ落とす経路は必ずあり、そこで防御が消えては意味がない。
//!
//! [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md

use std::error::Error;
use std::fmt;

/// このクレート共通の `Result`。
///
/// 既定のエラー型は [`TallyError`] だが、**行に紐づく処理は [`LineError`] を返す。**
/// そちらは `Result<_, LineError>` と明示的に書く（既定に頼ると
/// 「この関数は I/O で失敗しない」という表明が読み手に届かない）。
pub type Result<T, E = TallyError> = std::result::Result<T, E>;

/// 集計中に起きうる失敗の分類。
///
/// **閉じている。** バリアントを足すと、消費者側の
/// 「エラー → 終了コード」の `match` がコンパイルエラーになる。
/// これは望ましい挙動として受け入れている（[ADR-0004] 論点 2）。
///
/// ```
/// use tally_core::{Counter, Key, Selector, TallyError, tally_reader};
///
/// let selector = Selector::new(Key::JsonField("lvl".to_owned()));
/// let mut counter = Counter::new().strict(true);
/// let err = tally_reader(
///     &mut counter,
///     "{\"other\":1}\n".as_bytes(),
///     &selector,
///     |_| true,
/// )
/// .expect_err("strict なので失敗する");
///
/// // 行に紐づく失敗は 1 段剥がすと、必ず行番号と抜粋を持っている。
/// let TallyError::Line(line) = err else {
///     panic!("行の失敗を期待した")
/// };
/// assert_eq!(line.line_no, 1);
/// ```
///
/// [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md
/// **「開けなかった」はここに無い。** このクレートはファイルを開かないので、
/// 開く失敗は開いた側（CLI）の語彙である（[ADR-0007] 論点 3 で移した）。
///
/// [ADR-0007]: ../../../docs/adr/0007-multi-input-aggregation.md
#[derive(Debug, thiserror::Error)]
pub enum TallyError {
    /// 読み取り中の I/O 失敗。
    ///
    /// 不正な UTF-8 もここに来る（[`std::io::BufRead::lines`] が
    /// `InvalidData` を返す）。
    #[error("入力の読み取りに失敗しました")]
    Read(#[from] std::io::Error),

    /// 行に紐づく失敗。**必ず行番号と抜粋を持つ。**
    ///
    /// `#[error(transparent)]` なので、この層は表示に何も足さない。
    #[error(transparent)]
    Line(#[from] LineError),
}

/// 行に紐づく失敗。**行番号と抜粋を必ず持つ。**
///
/// 共有する文脈をここに、判別と個別データを [`kind`][Self::kind] に置いている。
/// 各バリアントに行番号を持たせる形と違い、**「行番号を持つが抜粋を持たない」
/// 状態が型として存在しない。**
///
/// 構築は [`LineError::new`] を通す。抜粋の生成箇所をそこ 1 箇所に集約してある
/// （[ADR-0005] 論点 5）。
///
/// ```
/// use tally_core::{LineError, LineErrorKind};
///
/// let err = LineError::new(
///     2,
///     "{\"other\":1}",
///     LineErrorKind::MissingField {
///         field: "lvl".into(),
///     },
/// );
///
/// assert_eq!(err.line_no, 2);
/// // 1 行で自己完結する: 行番号・原因・抜粋が揃う。
/// let shown = err.to_string();
/// assert!(shown.contains("2 行目"), "{shown}");
/// assert!(shown.contains("`lvl`"), "{shown}");
/// assert!(shown.contains("other"), "{shown}");
/// ```
///
/// [ADR-0005]: ../../../docs/adr/0005-selector-public-api.md
#[derive(Debug)]
pub struct LineError {
    /// 入力の何行目か。**1 始まり。** `--filter` で除外した行も数に入る。
    pub line_no: usize,
    /// 該当行の先頭を切り詰めたもの。表示時に [`str`] の `Debug` でエスケープされる。
    pub snippet: Box<str>,
    /// 何が起きたか。
    pub kind: LineErrorKind,
}

impl LineError {
    /// 行番号・行そのもの・原因から組み立てる。**抜粋はここで作る。**
    ///
    /// 行を丸ごと受け取って内部で切り詰めるのは、**抜粋の作り方を
    /// 呼び出し側に散らさないため。** 切り詰めの規則（長さ・省略記号・
    /// 文字境界の扱い）が複数箇所に散ると、片方だけ直る。
    #[must_use]
    pub fn new(line_no: usize, line: &str, kind: LineErrorKind) -> Self {
        Self {
            line_no,
            snippet: snippet(line),
            kind,
        }
    }
}

impl fmt::Display for LineError {
    /// **抜粋は `{:?}`（[`str`] の `Debug`）で出す。** 入力は信頼できず、
    /// 表示を撹乱する文字をそのまま stderr へ流すと端末の表示を操作されうる
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} 行目: {}: {:?}",
            self.line_no, self.kind, self.snippet
        )
    }
}

impl Error for LineError {
    /// **`kind` を飛ばして、その原因を指す。**
    ///
    /// `kind` の文は [`Display`][fmt::Display] に埋め込んである。ここで `kind` 自身を
    /// 返すと、チェーンを連結して表示したときに同じ文が 2 度出る。
    /// 規則の全体はモジュールドキュメントの表を参照。
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.kind.source()
    }
}

/// 行の失敗の内訳。
///
/// **開いている**（`#[non_exhaustive]`）。抽出規則が増えるたびに伸びる側なので、
/// バリアント追加で消費者を壊さない。別クレートから `match` する側は `_ =>` を書く
/// （[ADR-0004] 論点 2）。定義元クレートの中では属性が無効なので、
/// **こちらの網羅性検査は保たれる。**
///
/// ```
/// use tally_core::LineErrorKind;
///
/// let kind = LineErrorKind::UnsupportedFieldType {
///     field: "tags".into(),
/// };
/// assert!(kind.to_string().contains("`tags`"));
/// ```
///
/// [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum LineErrorKind {
    /// 行を JSON として解釈できなかった。
    #[error("JSON として解釈できません")]
    InvalidJson {
        /// 解釈できなかった原因。**`serde_json` の型は露出しない。**
        #[source]
        source: JsonError,
    },

    /// 抽出対象のフィールドが文字列・数値・真偽値のいずれでもなかった。
    ///
    /// 「値が無い」のではなく「集計に使えない形をしている」ので、
    /// **`strict` に関わらず常に失敗にする。**
    #[error("フィールド `{field}` は文字列・数値・真偽値ではありません")]
    UnsupportedFieldType {
        /// 対象のフィールド名。
        field: Box<str>,
    },

    /// キーを取り出せなかった。フィールドが無い場合と値が `null` の場合の両方。
    ///
    /// これが失敗になるのは [`Counter::strict`][crate::Counter::strict] が
    /// 有効なときだけ。既定ではスキップとして数える。
    #[error("フィールド `{field}` を取り出せません")]
    MissingField {
        /// 取り出せなかったフィールド名。
        field: Box<str>,
    },
}

/// JSON として解釈できなかった原因。
///
/// **内部表現は公開しない。** [`Display`][fmt::Display] と [`Error`] だけを通す
/// 不透明な newtype である（[ADR-0004] 論点 4）。
///
/// # なぜ包むのか
///
/// `LineErrorKind::InvalidJson { source: serde_json::Error }` と書くと
/// **`serde_json` が公開依存になる。** `serde_json` が major を上げたら
/// このクレートも major を上げることになり、版を固定した消費者は
/// **自分のコードが 1 行も変わっていないのに移行が要る。**
///
/// **enum のバリアントのフィールドは常に公開**なので、
/// ペイロードの型自体を不透明にするしか手がない。
///
/// # 代償
///
/// 消費者が `serde_json::Error` として取り出す手段が無い（`downcast` もできない）。
/// **その要求が出たら判断を見直す。**
///
/// [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md
#[derive(Debug)]
pub struct JsonError(serde_json::Error);

impl JsonError {
    /// **`From` 実装ではなく `pub(crate)` の関数にしている。**
    /// `impl From<serde_json::Error> for JsonError` を公開すると、
    /// トレイト実装のシグネチャに `serde_json` の型が現れ、
    /// 隠したはずの公開依存が復活する。
    pub(crate) fn new(source: serde_json::Error) -> Self {
        Self(source)
    }
}

impl fmt::Display for JsonError {
    /// **`serde_json` の「at line 1 column N」をそのまま通さない。**
    ///
    /// このクレートは 1 行ずつ渡すので `serde_json` の言う行番号は常に 1 になり、
    /// [`LineError`] が出す「N 行目」と衝突して読める。位置は列だけ言い直す。
    ///
    /// 単位が「バイト」なのは `serde_json` の数え方に合わせたもの。
    /// 文字数に直すには元の行が必要で、ここには無い。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let full = self.0.to_string();
        // 理由の文自体に " at line " が入ることは無いが、末尾から探すほうが安全。
        let reason = full
            .rfind(" at line ")
            .map_or(full.as_str(), |at| &full[..at]);
        match self.0.column() {
            // I/O 由来・入力終端由来の失敗は位置を持たない。
            0 => f.write_str(reason),
            column => write!(f, "{reason}（{column} バイト目）"),
        }
    }
}

impl Error for JsonError {
    /// **`serde_json::Error` を返さない。** ここで返すと
    /// 消費者が `downcast_ref::<serde_json::Error>()` で取り出せてしまい、
    /// 型に出さずに公開依存が成立する。
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// エラーメッセージに載せる行の抜粋の長さ（文字数）。バイト数ではない。
///
/// 公開していないのは、**抜粋の長さは表示の都合であって契約ではない**ため。
/// 公開 API は semver の対象なので、必要になるまで surface を広げない。
const SNIPPET_CHARS: usize = 40;

/// エラーメッセージ用に行の先頭を切り詰める。
///
/// **バイトではなく文字で数える。** `&line[..SNIPPET_CHARS]` と書くと、
/// 日本語のようなマルチバイト文字の途中に当たった瞬間に panic する。
///
/// ここでの「文字」は `char`（Unicode スカラ値）であって書記素クラスタではない。
/// 結合文字や ZWJ 絵文字の途中で切れることはある（panic はしないが表示は崩れる）。
/// 書記素まで見るには `unicode-segmentation` が要り、診断表示に見合わない。
///
/// `char_indices().nth(n)` が返すバイト位置は **必ず文字境界** なので、
/// そこで切る限りスライスは安全。しかも `nth` は高々 41 文字で打ち切るため、
/// 長い行を最後まで走査しない。
///
/// `[..].concat()` は連結後の長さを先に合計してから 1 回だけ確保する
/// （`format!` と違い再確保しない）。戻り値を `Box<str>` にしているのは、
/// 以後変更しない文字列だから。`String` は容量フィールドのぶん 24 バイトだが
/// `Box<str>` は 16 バイトで、これが [`LineError`] の大きさに効く。
///
/// **呼び出し箇所は [`LineError::new`] だけ。** 切り詰めの規則が散ると片方だけ直る。
fn snippet(line: &str) -> Box<str> {
    match line.char_indices().nth(SNIPPET_CHARS) {
        Some((boundary, _)) => [&line[..boundary], "…"].concat().into_boxed_str(),
        // 切り詰めが不要なら省略記号も付けない。
        None => line.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `serde_json` に実際に失敗させて [`JsonError`] を得る。
    fn json_err(line: &str) -> JsonError {
        let parsed = serde_json::from_str::<serde_json::Value>(line);
        JsonError::new(parsed.expect_err("壊れた JSON のはず"))
    }

    /// `anyhow` の `{:#}` 相当。`source()` を辿って `": "` で連結する。
    ///
    /// **`anyhow` を使わずに手で書いている。** 連結表示で文が重複しないことは
    /// この連結でしか観測できず、`anyhow::Error::chain()` は
    /// `std::error::Error::source()` を辿るだけの糖衣なので、
    /// 依存を足さずに同じものが書ける。
    fn chain(err: &(dyn Error + 'static)) -> String {
        // `iter::successors(Some(err), |e| e.source())` と書きたくなるが通らない。
        // クロージャの引数は `&&dyn Error` になり、戻り値の寿命が
        // **借用そのものの寿命**に縛られる（E0521 相当）。素朴なループにする。
        let mut shown = Vec::new();
        let mut current = Some(err);
        while let Some(err) = current {
            shown.push(err.to_string());
            current = err.source();
        }
        shown.join(": ")
    }

    // --- 抜粋の切り詰め ---

    #[test]
    fn 抜粋は先頭_40_文字に切り詰められる() {
        // 長さだけ見ると「別の 40 文字」を取る実装（`skip(1).take(40)` 等）も
        // 通ってしまうため、文字列全体を突き合わせる。
        let line = "x".repeat(100);
        let head: String = line.chars().take(SNIPPET_CHARS).collect();
        assert_eq!(snippet(&line).as_ref(), format!("{head}…"));
    }

    #[test]
    fn 抜粋がマルチバイト境界で切れても_panic_しない() {
        // `&line[..40]` で切ると UTF-8 の途中に当たって panic する入力。
        let line = "あ".repeat(60);
        let head: String = line.chars().take(SNIPPET_CHARS).collect();
        assert_eq!(snippet(&line).as_ref(), format!("{head}…"));
    }

    #[test]
    fn 短い行には省略記号が付かない() {
        assert_eq!(snippet("{\"a\":1}").as_ref(), "{\"a\":1}");
    }

    #[test]
    fn ちょうど_40_文字の行には省略記号が付かない() {
        // 境界。`nth(SNIPPET_CHARS)` が `None` を返す最長の入力。
        let line = "x".repeat(SNIPPET_CHARS);
        assert_eq!(snippet(&line).as_ref(), line);
    }

    // --- Display と source() の合成規則 ---

    #[test]
    fn line_error_の表示は行番号と原因と抜粋で自己完結する() {
        let err = LineError::new(
            7,
            "{\"other\":1}",
            LineErrorKind::MissingField {
                field: "lvl".into(),
            },
        );
        assert_eq!(
            err.to_string(),
            "7 行目: フィールド `lvl` を取り出せません: \"{\\\"other\\\":1}\""
        );
    }

    #[test]
    fn 抜粋の制御文字は生のまま表示されない() {
        // 入力は信頼できない。復帰 (CR) が stderr にそのまま流れると、
        // 端末ではカーソルが行頭へ戻り、直前の出力を上書きできてしまう。
        //
        // CR は JSON の空白として妥当なので、**この行は JSON としては正しい**。
        //
        // 注意: JSON がエスケープを要求するのは C0 制御文字（U+0000..=U+001F）だけ。
        // U+202E（RTL override）や U+007F（DEL）は合法な JSON 文字列を素通りして
        // ここへ到達する。「JSON なら安全」は成り立たない。
        let err = LineError::new(
            1,
            "{\"a\":\r1}",
            LineErrorKind::MissingField { field: "b".into() },
        );
        let msg = err.to_string();
        assert!(
            !msg.contains('\r'),
            "生の制御文字がメッセージに含まれている: {msg:?}"
        );
        assert!(
            msg.contains("\\r"),
            "エスケープされた形で含まれるはず: {msg}"
        );
    }

    #[test]
    fn 単一のエラーの表示だけでエスケープが成立している() {
        // **チェーンを辿らずに 1 件だけログへ落とす経路でも防御が効くこと。**
        // 抜粋の描画を `source()` 側へ落とすとこの検査が壊れる。
        let err = LineError::new(
            1,
            "{\"a\":\u{202e}1}",
            LineErrorKind::MissingField { field: "b".into() },
        );
        // to_string() は source() を辿らない。
        let single = err.to_string();
        assert!(
            !single.contains('\u{202e}'),
            "RTL override が生で載っている: {single:?}"
        );
        assert!(single.contains("\\u{202e}"), "{single}");
    }

    #[test]
    fn source_は_kind_を飛ばして原因を指す() {
        let err = LineError::new(
            1,
            "not json",
            LineErrorKind::InvalidJson {
                source: json_err("not json"),
            },
        );

        // kind 自身ではなく、その原因（JsonError）が返る。
        let source = err.source().expect("原因があるはず");
        assert!(
            source.downcast_ref::<JsonError>().is_some(),
            "JsonError を期待したが {source}"
        );
        // kind を返していたら、ここが LineErrorKind になる。
        assert!(source.downcast_ref::<LineErrorKind>().is_none());
    }

    #[test]
    fn 原因を持たない_kind_なら_source_は_none() {
        let err = LineError::new(
            1,
            "{}",
            LineErrorKind::MissingField {
                field: "lvl".into(),
            },
        );
        assert!(err.source().is_none());
    }

    #[test]
    fn 連結表示で同じ文が二度出ない() {
        // これが `#[source] kind` を使わずに `Error` を手書きした理由。
        let err = LineError::new(
            2,
            "not json",
            LineErrorKind::InvalidJson {
                source: json_err("not json"),
            },
        );
        let joined = chain(&err);
        assert_eq!(
            joined.matches("JSON として解釈できません").count(),
            1,
            "kind の文が重複している: {joined}"
        );
        // 抜粋も 1 度だけ。
        assert_eq!(joined.matches("not json").count(), 1, "{joined}");
    }

    #[test]
    fn tally_error_の_line_は表示に何も足さない() {
        let line = LineError::new(
            3,
            "{}",
            LineErrorKind::MissingField {
                field: "lvl".into(),
            },
        );
        let expected = line.to_string();
        assert_eq!(TallyError::Line(line).to_string(), expected);
    }

    #[test]
    fn line_error_は_from_で_tally_error_になる() {
        // `?` による合流がこの `From` に乗る。
        let err: TallyError = LineError::new(
            1,
            "{}",
            LineErrorKind::MissingField {
                field: "lvl".into(),
            },
        )
        .into();
        assert!(matches!(err, TallyError::Line(_)));
    }

    // --- JsonError の表示 ---

    #[test]
    fn serde_json_の文言は_at_line_で終わる() {
        // **前提を固定する。** JsonError の Display はこの形を刻んで組み直すので、
        // serde_json 側が変わったらここが落ちて気づける。
        let raw = serde_json::from_str::<serde_json::Value>("not json")
            .expect_err("壊れた JSON のはず")
            .to_string();
        assert!(raw.contains(" at line 1 column "), "実際: {raw}");
    }

    #[test]
    fn json_error_は_serde_の行番号を出さない() {
        // 1 行ずつ渡すので serde の行番号は常に 1 になり、
        // LineError の「N 行目」と衝突して読める。
        let shown = json_err("not json").to_string();
        assert!(!shown.contains("at line"), "行番号が残っている: {shown}");
        assert!(shown.contains("バイト目"), "列が出ていない: {shown}");
    }

    #[test]
    fn json_error_は理由を保っている() {
        let shown = json_err("{\"a\":}").to_string();
        assert!(shown.contains("expected value"), "実際: {shown}");
    }

    #[test]
    fn json_error_は_serde_json_error_を_downcast_させない() {
        // ここで取り出せてしまうと、型に出さずに公開依存が成立する。
        let err = json_err("not json");
        assert!(err.source().is_none());
    }
}
