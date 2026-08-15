//! 集計のコア。**I/O を一切持たない**ように切り出してある。
//!
//! この分離が学習上いちばん重要な点。`impl BufRead` を受け取る関数と、
//! `&str` を受け取る純粋関数を分けておくと、テストがファイルシステムに依存しなくなる。

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::BufRead;

use crate::error::{Result, TallyError};

/// 各行からどの値を取り出すか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// 行全体（前後の空白を除去したもの）をキーにする。
    WholeLine,
    /// 行を JSON オブジェクトとして解釈し、指定フィールドをキーにする。
    JsonField(String),
}

impl Key {
    /// 1 行からキーを取り出す。
    ///
    /// 戻り値が `Option` なのは「そのフィールドを持たない行」を
    /// エラーではなくスキップとして扱うため。ログ集計では欠損は日常的で、
    /// そこで全体を失敗させると使い物にならない。
    ///
    /// `Cow` を返しているのは、`WholeLine` の場合に借用のまま返せるから。
    /// ここで無条件に `String` を作ると、行数ぶんのアロケーションが増える。
    fn extract<'a>(&self, line: &'a str, line_no: usize) -> Result<Option<Cow<'a, str>>> {
        match self {
            Self::WholeLine => Ok(Some(Cow::Borrowed(line.trim()))),
            Self::JsonField(field) => {
                let value: serde_json::Value = serde_json::from_str(line)
                    .map_err(|source| TallyError::InvalidJson { line_no, source })?;

                let Some(found) = value.get(field) else {
                    return Ok(None);
                };

                let rendered = match found {
                    serde_json::Value::String(s) => Cow::Owned(s.clone()),
                    serde_json::Value::Number(n) => Cow::Owned(n.to_string()),
                    serde_json::Value::Bool(b) => Cow::Owned(b.to_string()),
                    serde_json::Value::Null => return Ok(None),
                    _ => {
                        return Err(TallyError::UnsupportedFieldType {
                            line_no,
                            field: field.clone(),
                        });
                    }
                };
                Ok(Some(rendered))
            }
        }
    }

    /// 対象の JSON フィールド名。行全体をキーにする場合は `None`。
    ///
    /// `WholeLine` は値を取り出せないことが無いため、`--strict` の対象にならない。
    /// その区別をここで型に落としている。
    fn field_name(&self) -> Option<&str> {
        match self {
            Self::WholeLine => None,
            Self::JsonField(name) => Some(name),
        }
    }
}

/// 大小無視のために小文字化する。**変換が不要なら借用のまま返す。**
///
/// 判定に `char::is_uppercase()` を使わないのが要点。
/// `'ǅ'` (U+01C5) は Unicode 上 Titlecase であり `is_uppercase()` は `false` を返すが、
/// `to_lowercase()` では `'ǆ'` に変わる。「大文字か」ではなく
/// **「小文字化で変化するか」** を直接見る必要がある。
fn needs_lowering(value: &str) -> bool {
    value.chars().any(|c| {
        let mut lowered = c.to_lowercase();
        // to_lowercase() は 1 文字とは限らない（'İ' U+0130 は 2 文字に伸びる）。
        // 「1 文字に収まり、かつ元と同じ」ときだけ変換不要と判定する。
        match (lowered.next(), lowered.next()) {
            (Some(first), None) => first != c,
            _ => true,
        }
    })
}

/// 小文字化した値を返す。呼び出し側のアロケーションを最小化する。
fn fold_case(value: Cow<'_, str>) -> Cow<'_, str> {
    if !needs_lowering(&value) {
        // 大半の行がここを通る。借用は借用のまま、所有は所有のまま、追加確保なし。
        return value;
    }

    if value.is_ascii() {
        // ASCII に限れば小文字化で長さが変わらないため、その場で書き換えられる。
        // 元が Cow::Owned なら into_owned() は確保を伴わないので、
        // to_lowercase() と違って **2 度目のアロケーションを避けられる**。
        let mut owned = value.into_owned();
        owned.make_ascii_lowercase();
        return Cow::Owned(owned);
    }

    // 非 ASCII は長さが変わりうるので、新しい String を組み立てるほかない。
    Cow::Owned(value.to_lowercase())
}

/// エラーメッセージに載せる行の抜粋の長さ（文字数）。
///
/// 表示上の契約なので公開している。バイト数ではなく **文字数** である点に注意。
pub const SNIPPET_CHARS: usize = 40;

/// エラーメッセージ用に行の先頭を切り詰める。
///
/// **バイトではなく文字で数える。** `&line[..SNIPPET_CHARS]` と書くと、
/// 日本語のようなマルチバイト文字の途中に当たった瞬間に panic する。
///
/// `char_indices().nth(n)` が返すバイト位置は **必ず文字境界** なので、
/// そこで切る限りスライスは安全。文字を 1 つずつ `String` に積むより、
/// 境界を求めて一度にコピーするほうが確保回数が少ない。
fn snippet(line: &str) -> String {
    match line.char_indices().nth(SNIPPET_CHARS) {
        Some((boundary, _)) => {
            let mut out = String::with_capacity(boundary + '…'.len_utf8());
            out.push_str(&line[..boundary]);
            out.push('…');
            out
        }
        // 切り詰めが不要なら省略記号も付けない。
        None => line.to_owned(),
    }
}

/// 「どこからキーを取り、どう正規化し、欠損をどう扱うか」。
///
/// [`Key`][] を包むだけの薄い型だが、抽出（どこから）と正規化（どう揃えるか）と
/// 厳格さ（欠損を許すか）を分けておくと、後続のフラグ（`--filter`）を足すときに
/// 各所のシグネチャを壊さずに済む。段階 2 の `--strict` 追加では、
/// 実際に `select` の呼び出し側を 1 箇所も変えずに済んだ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub key: Key,
    pub ignore_case: bool,
    /// キーを取り出せない行をエラーにするか。
    pub strict: bool,
}

impl Selector {
    /// 1 行からキーを取り出し、必要なら正規化する。
    pub fn select<'a>(&self, line: &'a str, line_no: usize) -> Result<Option<Cow<'a, str>>> {
        let Some(value) = self.key.extract(line, line_no)? else {
            // `--strict` は「取り出せたか否か」で一貫させる。
            // キーの不在も値が null の場合も、区別せず失敗にする。
            return match (self.strict, self.key.field_name()) {
                (true, Some(field)) => Err(TallyError::MissingField {
                    line_no,
                    field: field.to_owned(),
                    snippet: snippet(line),
                }),
                _ => Ok(None),
            };
        };
        Ok(Some(if self.ignore_case {
            fold_case(value)
        } else {
            value
        }))
    }
}

/// 集計結果の 1 行。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Entry {
    pub key: String,
    pub count: u64,
}

/// 集計結果全体。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Report {
    pub entries: Vec<Entry>,
    /// キーを取り出せずスキップした行数。
    pub skipped: usize,
    /// 読んだ行の総数（空行を除く）。
    pub total: usize,
}

/// 度数カウンタ。
#[derive(Debug, Default)]
pub struct Counter {
    counts: HashMap<String, u64>,
    skipped: usize,
    total: usize,
}

impl Counter {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 1 行を取り込む。空行は総数にも数えない。
    pub fn push_line(&mut self, selector: &Selector, line: &str, line_no: usize) -> Result<()> {
        if line.trim().is_empty() {
            return Ok(());
        }
        self.total += 1;

        match selector.select(line, line_no)? {
            Some(extracted) => {
                // entry API を使うと、存在チェックと挿入で 2 回ハッシュを引かずに済む。
                // `into_owned()` はここで初めて確定的にアロケーションする。
                *self.counts.entry(extracted.into_owned()).or_insert(0) += 1;
            }
            None => self.skipped += 1,
        }
        Ok(())
    }

    /// 上位 `limit` 件を返す。`None` なら全件。
    ///
    /// 件数の降順、同数ならキーの昇順。**同数時のタイブレークを決めておかないと
    /// `HashMap` の反復順に依存して出力が実行ごとに変わり、スナップショットテストが壊れる。**
    #[must_use]
    pub fn report(&self, limit: Option<usize>) -> Report {
        let mut entries: Vec<Entry> = self
            .counts
            .iter()
            .map(|(key, &count)| Entry {
                key: key.clone(),
                count,
            })
            .collect();

        entries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));

        if let Some(limit) = limit {
            entries.truncate(limit);
        }

        Report {
            entries,
            skipped: self.skipped,
            total: self.total,
        }
    }
}

/// `BufRead` を丸ごと集計する。I/O に触れるのはこの関数だけ。
pub fn tally_reader<R: BufRead>(
    reader: R,
    selector: &Selector,
    limit: Option<usize>,
) -> Result<Report> {
    let mut counter = Counter::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        counter.push_line(selector, &line, index + 1)?;
    }
    Ok(counter.report(limit))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 既存テスト用。大小無視なしの `Selector` を作る。
    fn sel(key: Key) -> Selector {
        Selector {
            key,
            ignore_case: false,
            strict: false,
        }
    }

    /// 大小無視ありの `Selector`。
    fn sel_ci(key: Key) -> Selector {
        Selector {
            key,
            ignore_case: true,
            strict: false,
        }
    }

    /// `--strict` ありの `Selector`。
    fn sel_strict(key: Key) -> Selector {
        Selector {
            key,
            ignore_case: false,
            strict: true,
        }
    }

    fn tally_str(input: &str, key: &Key) -> Report {
        tally_reader(input.as_bytes(), &sel(key.clone()), None).expect("集計に成功するはず")
    }

    /// 1 行だけ通してキーを取り出す。`Cow` の借用/所有を検査するために使う。
    ///
    /// 参照が 2 つあるため省略規則では出力の寿命が決まらない（E0106）。
    /// 戻り値が借用しうるのは `line` の側なので、明示的に紐づける。
    fn select_one<'a>(selector: &Selector, line: &'a str) -> Cow<'a, str> {
        selector
            .select(line, 1)
            .expect("抽出に成功するはず")
            .expect("値が存在するはず")
    }

    #[test]
    fn 行全体をキーにして度数を数える() {
        let report = tally_str("a\nb\na\n", &Key::WholeLine);
        assert_eq!(
            report.entries,
            vec![
                Entry {
                    key: "a".to_owned(),
                    count: 2
                },
                Entry {
                    key: "b".to_owned(),
                    count: 1
                },
            ]
        );
        assert_eq!(report.total, 3);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn 空行は総数に含めない() {
        let report = tally_str("a\n\n   \na\n", &Key::WholeLine);
        assert_eq!(report.total, 2);
        assert_eq!(report.entries.len(), 1);
    }

    #[test]
    fn 同数のときはキーの昇順で安定する() {
        // HashMap の反復順に依存していれば、この assert はいずれ落ちる。
        let report = tally_str("b\nc\na\n", &Key::WholeLine);
        let keys: Vec<&str> = report.entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn json_フィールドを抽出する() {
        let input = r#"{"lvl":"info"}
{"lvl":"error"}
{"lvl":"info"}"#;
        let report = tally_str(input, &Key::JsonField("lvl".to_owned()));
        assert_eq!(report.entries[0].key, "info");
        assert_eq!(report.entries[0].count, 2);
    }

    #[test]
    fn フィールドがない行はスキップ扱いになる() {
        let input = "{\"lvl\":\"info\"}\n{\"other\":1}\n";
        let report = tally_str(input, &Key::JsonField("lvl".to_owned()));
        assert_eq!(report.skipped, 1);
        assert_eq!(report.total, 2);
    }

    #[test]
    fn json_として壊れている行は行番号つきで失敗する() {
        let input = "{\"lvl\":\"info\"}\nnot json\n";
        let err = tally_reader(
            input.as_bytes(),
            &sel(Key::JsonField("lvl".to_owned())),
            None,
        )
        .expect_err("2 行目で失敗するはず");
        assert!(
            matches!(err, TallyError::InvalidJson { line_no: 2, .. }),
            "実際のエラー: {err:?}"
        );
    }

    #[test]
    fn limit_は件数の多い順に切り詰める() {
        let report = tally_reader("a\na\nb\nc\n".as_bytes(), &sel(Key::WholeLine), Some(2))
            .expect("集計に成功するはず");
        assert_eq!(report.entries.len(), 2);
        assert_eq!(report.entries[0].key, "a");
    }

    // --- 段階 1: --ignore-case ---

    #[test]
    fn ignore_case_で大文字小文字が同じキーに畳まれる() {
        let report = tally_reader(
            "Info\nINFO\ninfo\n".as_bytes(),
            &sel_ci(Key::WholeLine),
            None,
        )
        .expect("集計に成功するはず");
        assert_eq!(
            report.entries,
            vec![Entry {
                key: "info".to_owned(),
                count: 3
            }]
        );
    }

    #[test]
    fn ignore_case_なしなら大文字小文字は区別される() {
        let report = tally_str("Info\ninfo\n", &Key::WholeLine);
        assert_eq!(report.entries.len(), 2, "実際: {:?}", report.entries);
    }

    #[test]
    fn 小文字化で変化しない値は借用のまま返る() {
        // 段階 1 の主眼。to_lowercase() を無条件に呼ぶ実装にすると、
        // Cow::Owned になってこのテストが落ちる。
        let selector = sel_ci(Key::WholeLine);
        let extracted = select_one(&selector, "already lower");
        assert!(
            matches!(extracted, Cow::Borrowed(_)),
            "不要なアロケーションが発生している: {extracted:?}"
        );
    }

    #[test]
    fn 小文字化が必要な値だけが所有値になる() {
        let selector = sel_ci(Key::WholeLine);
        let extracted = select_one(&selector, "HAS Upper");
        assert_eq!(extracted.as_ref(), "has upper");
        assert!(matches!(extracted, Cow::Owned(_)));
    }

    #[test]
    fn タイトルケース文字も畳まれる() {
        // 'ǅ' (U+01C5) は Unicode 上 Titlecase であり to_lowercase() では 'ǆ' に変わる。
        // まず「is_uppercase() では検出できない」という前提自体を固定しておく。
        // この assert が落ちたら、needs_lowering の実装根拠が変わったということ。
        assert!(
            !'ǅ'.is_uppercase(),
            "前提が崩れている: 'ǅ' が Uppercase 扱い"
        );

        let selector = sel_ci(Key::WholeLine);
        assert_eq!(select_one(&selector, "ǅ").as_ref(), "ǆ");
    }

    #[test]
    fn 小文字化で長さが変わる文字も壊れない() {
        // 'İ' (U+0130) の小文字化は 2 文字（'i' + 合成用ドット）に伸びる。
        // ASCII 前提の in-place 変換で処理すると壊れる。
        let selector = sel_ci(Key::WholeLine);
        let extracted = select_one(&selector, "İ");
        assert_eq!(
            extracted.chars().count(),
            2,
            "実際: {:?}",
            extracted.as_ref()
        );
    }

    // --- 段階 2: --strict ---

    /// `strict` で失敗させ、`MissingField` の中身を取り出す。
    fn missing_field_err(input: &str, field: &str) -> (usize, String) {
        let err = tally_reader(
            input.as_bytes(),
            &sel_strict(Key::JsonField(field.to_owned())),
            None,
        )
        .expect_err("strict なので失敗するはず");
        match err {
            TallyError::MissingField {
                line_no, snippet, ..
            } => (line_no, snippet),
            other => panic!("MissingField を期待したが {other:?}"),
        }
    }

    #[test]
    fn strict_でフィールドが無い行はエラーになる() {
        let input = "{\"lvl\":\"info\"}\n{\"other\":1}\n";
        let (line_no, _) = missing_field_err(input, "lvl");
        assert_eq!(line_no, 2, "2 行目で失敗するはず");
    }

    #[test]
    fn strict_では_null_値もエラーになる() {
        // 「取り出せたか否か」で一貫させる設計なので、null も欠損と同じ扱い。
        let input = "{\"lvl\":null}\n";
        let (line_no, _) = missing_field_err(input, "lvl");
        assert_eq!(line_no, 1);
    }

    #[test]
    fn strict_なしなら従来どおりスキップされる() {
        let input = "{\"lvl\":\"info\"}\n{\"other\":1}\n";
        let report = tally_reader(
            input.as_bytes(),
            &sel(Key::JsonField("lvl".to_owned())),
            None,
        )
        .expect("strict でなければ成功するはず");
        assert_eq!(report.skipped, 1);
    }

    #[test]
    fn strict_でも取り出せる行だけなら成功する() {
        let input = "{\"lvl\":\"info\"}\n{\"lvl\":\"warn\"}\n";
        let report = tally_reader(
            input.as_bytes(),
            &sel_strict(Key::JsonField("lvl".to_owned())),
            None,
        )
        .expect("全行取り出せるので成功するはず");
        assert_eq!(report.total, 2);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn 行全体がキーなら_strict_でも失敗しない() {
        // WholeLine は None を返さないため、strict は無効。
        let report = tally_reader("a\nb\n".as_bytes(), &sel_strict(Key::WholeLine), None)
            .expect("成功するはず");
        assert_eq!(report.total, 2);
    }

    #[test]
    fn 抜粋は先頭_40_文字に切り詰められる() {
        let long = "x".repeat(100);
        let input = format!("{{\"a\":1,\"pad\":\"{long}\"}}\n");
        let (_, snippet) = missing_field_err(&input, "lvl");
        assert_eq!(
            snippet.chars().count(),
            SNIPPET_CHARS + 1,
            "40 文字 + 省略記号のはず: {snippet:?}"
        );
        assert!(snippet.ends_with('…'), "省略記号が無い: {snippet:?}");
    }

    #[test]
    fn 抜粋がマルチバイト境界で切れても_panic_しない() {
        // `&line[..40]` で切ると UTF-8 の途中に当たって panic する入力。
        let ja = "あ".repeat(60);
        let input = format!("{{\"a\":\"{ja}\"}}\n");
        let (_, snippet) = missing_field_err(&input, "lvl");
        assert_eq!(snippet.chars().count(), SNIPPET_CHARS + 1);
    }

    #[test]
    fn 短い行には省略記号が付かない() {
        let input = "{\"a\":1}\n";
        let (_, snippet) = missing_field_err(input, "lvl");
        assert_eq!(snippet, "{\"a\":1}");
    }

    #[test]
    fn 抜粋の制御文字は生のまま表示されない() {
        // 入力は信頼できない。復帰 (CR) が stderr にそのまま流れると、
        // 端末ではカーソルが行頭へ戻り、直前の出力を上書きできてしまう。
        //
        // CR は JSON の空白として妥当なので、**この行は JSON としては正しい**。
        // 逆に生の ESC を含む行は JSON として不正なので、この経路には到達しない
        // （JSON 文字列は U+0000..=U+001F のエスケープを要求する）。
        let input = "{\"a\":\r1}\n";
        let err = tally_reader(
            input.as_bytes(),
            &sel_strict(Key::JsonField("lvl".to_owned())),
            None,
        )
        .expect_err("strict なので失敗するはず");
        assert!(
            matches!(err, TallyError::MissingField { .. }),
            "MissingField を期待したが {err:?}"
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
    fn ignore_case_は値に効きフィールド名には効かない() {
        // フィールド名の一致は厳密なまま。"Lvl" は "lvl" とは一致せずスキップされる。
        let input = "{\"Lvl\":\"INFO\"}\n{\"lvl\":\"INFO\"}\n";
        let report = tally_reader(
            input.as_bytes(),
            &sel_ci(Key::JsonField("lvl".to_owned())),
            None,
        )
        .expect("集計に成功するはず");
        assert_eq!(report.skipped, 1);
        assert_eq!(
            report.entries,
            vec![Entry {
                key: "info".to_owned(),
                count: 1
            }]
        );
    }
}
