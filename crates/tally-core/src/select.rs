//! キーの抽出と正規化。**「参加する行がどうキーを産むか」だけを扱う。**
//!
//! 「どの行が参加するか」（フィルタ）はここには来ない。上流の
//! [`tally_reader`][crate::tally_reader] が述語で受ける。

use std::borrow::Cow;

use crate::error::{JsonError, LineError, LineErrorKind};

/// 各行からどの値を取り出すか。
///
/// # 公開の約束
///
/// **`Debug` / `Clone` / `PartialEq` / `Eq` の 4 つは公開の約束である。**
/// 消費者はこれらに依存してよい（[ADR-0005] 論点 4）。
///
/// トレイト実装の削除は、消費者が自分では埋められない種類の破壊的変更である。
/// 孤児ルールにより **他人の型に他人のトレイトを実装することはできない**ので、
/// `Eq` を外されたら消費者は `HashMap` のキーに使うのをやめるしかない。
/// 「必要になるまで surface を広げない」という方針がここに効かないのはこのため。
///
/// **バリアントは今後増える**（`#[non_exhaustive]`）。別クレートから `match` する側は
/// `_ =>` を書く。定義元クレートの中では属性が無効なので、
/// `Key::extract`（非公開）の網羅性検査は保たれる。
///
/// ```
/// use tally_core::Key;
///
/// let key = Key::JsonField("lvl".to_owned());
/// assert_eq!(key.clone(), key);
/// assert_ne!(key, Key::WholeLine);
/// ```
///
/// [ADR-0005]: ../../../docs/adr/0005-selector-public-api.md
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// 行全体（前後の空白を除去したもの）をキーにする。
    WholeLine,
    /// 行を JSON オブジェクトとして解釈し、指定フィールドをキーにする。
    ///
    /// **中身は `String` のまま。newtype で包まない**（[ADR-0005] 論点 3 の軸 D）。
    /// 包む案の守備範囲（不正な指定を型で弾く）は、実際の拡張候補を調べたら空だった —
    /// JSON Pointer は先頭 `/` を要求するので既存バリアントに畳めず**新しい
    /// バリアント**になり、JSONPath は戻り値が 0..n になって集計モデルそのものが変わる。
    ///
    /// [ADR-0005]: ../../../docs/adr/0005-selector-public-api.md
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
    ///
    /// **返すのは [`LineErrorKind`] であって [`LineError`] ではない。**
    /// 行番号と抜粋は [`Selector::select`] が 1 回だけ付ける
    /// （[ADR-0005] 論点 5）。そのため `line_no` を引数に取らない。
    ///
    /// [ADR-0005]: ../../../docs/adr/0005-selector-public-api.md
    fn extract<'a>(&self, line: &'a str) -> Result<Option<Cow<'a, str>>, LineErrorKind> {
        match self {
            Self::WholeLine => Ok(Some(Cow::Borrowed(line.trim()))),
            Self::JsonField(field) => {
                let value: serde_json::Value =
                    serde_json::from_str(line).map_err(|source| LineErrorKind::InvalidJson {
                        source: JsonError::new(source),
                    })?;

                let Some(found) = value.get(field) else {
                    return Ok(None);
                };

                let rendered = match found {
                    serde_json::Value::String(s) => Cow::Owned(s.clone()),
                    serde_json::Value::Number(n) => Cow::Owned(n.to_string()),
                    serde_json::Value::Bool(b) => Cow::Owned(b.to_string()),
                    serde_json::Value::Null => return Ok(None),
                    _ => {
                        return Err(LineErrorKind::UnsupportedFieldType {
                            field: field.as_str().into(),
                        });
                    }
                };
                Ok(Some(rendered))
            }
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

/// 「どこからキーを取り、どう正規化するか」。
///
/// # 境界
///
/// **入れてよいのは「参加する行がどうキーを産むか」だけ。**
/// 「どの行が参加するか」は上流（[`tally_reader`][crate::tally_reader] の述語）、
/// 「産まなかったときどうするか」は下流（[`Counter::strict`][crate::Counter::strict]）。
///
/// 段階 4 の `--filter` が前者、`strict` が後者で、**どちらもここには無い。**
/// `strict` は当初ここにあったが、[ADR-0005] 論点 1 で
/// 「抽出器の性質ではなく消費者の方針」と判断して [`Counter`][crate::Counter] へ移した。
///
/// # 構築
///
/// **フィールドは非公開で、ビルダーで組み立てる**（[ADR-0005] 論点 2）。
/// リテラル構築を封じておくと、**フィールドの追加が非破壊になる**
/// （Cargo Book の `struct-private-fields-with-private`）。
/// 非公開フィールドが 1 つでもあれば外部からリテラル構築できないので、
/// `#[non_exhaustive]` は冗長になる。付けていないのはそのため。
///
/// # 公開の約束
///
/// **`Debug` / `Clone` / `PartialEq` / `Eq` の 4 つは公開の約束である**
/// （理由は [`Key`] の同じ節を参照）。
///
/// ```
/// use tally_core::{Key, Selector};
///
/// let selector = Selector::new(Key::JsonField("lvl".to_owned())).ignore_case(true);
/// assert_eq!(selector.key(), &Key::JsonField("lvl".to_owned()));
///
/// let value = selector
///     .select("{\"lvl\":\"INFO\"}", 1)
///     .expect("JSON として読める")
///     .expect("lvl がある");
/// assert_eq!(value, "info");
/// ```
///
/// [ADR-0005]: ../../../docs/adr/0005-selector-public-api.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    key: Key,
    ignore_case: bool,
}

impl Selector {
    /// 既定は大小を区別する。
    ///
    /// ```
    /// use tally_core::{Key, Selector};
    ///
    /// let selector = Selector::new(Key::WholeLine);
    /// let value = selector.select("  Info  ", 1).expect("失敗しない");
    /// // WholeLine は前後の空白を除去する。大小はそのまま。
    /// assert_eq!(value.as_deref(), Some("Info"));
    /// ```
    #[must_use]
    pub fn new(key: Key) -> Self {
        Self {
            key,
            ignore_case: false,
        }
    }

    /// 大文字小文字を区別せずに集計する。
    ///
    /// **正規化されるのは取り出した値だけ。** [`Key::JsonField`] のフィールド名の
    /// 一致判定は厳密なままである。
    ///
    /// ```
    /// use tally_core::{Key, Selector};
    ///
    /// let selector = Selector::new(Key::JsonField("lvl".to_owned())).ignore_case(true);
    /// // 値は畳まれる。
    /// assert_eq!(
    ///     selector.select("{\"lvl\":\"INFO\"}", 1).expect("読める").as_deref(),
    ///     Some("info")
    /// );
    /// // フィールド名は畳まれない。"Lvl" は "lvl" と一致しない。
    /// assert_eq!(selector.select("{\"Lvl\":\"INFO\"}", 2).expect("読める"), None);
    /// ```
    #[must_use]
    pub fn ignore_case(mut self, yes: bool) -> Self {
        self.ignore_case = yes;
        self
    }

    /// どこからキーを取るか。
    #[must_use]
    pub fn key(&self) -> &Key {
        &self.key
    }

    /// 1 行からキーを取り出し、必要なら正規化する。
    ///
    /// # 戻り値
    ///
    /// - `Ok(Some(_))` — キーを取り出せた
    /// - **`Ok(None)` — 取り出せなかった。** [`Key::JsonField`] で
    ///   フィールドが無い場合と、値が `null` の場合。
    ///   **これを失敗として扱うかは呼び出し側の方針**であり、
    ///   [`Counter`][crate::Counter] が `strict` で決める。
    ///   `Selector` はここで判断しない
    /// - `Err(_)` — 行を JSON として解釈できない、または値が
    ///   文字列・数値・真偽値のいずれでもない
    ///
    /// エラー型が [`LineError`] であって [`TallyError`][crate::TallyError] でないのは、
    /// **この関数が I/O では失敗しえない**ことを表明するため。読み手が本文を
    /// 読まずに知れる（[ADR-0004] 論点 3）。
    ///
    /// `line_no` は診断のためだけに使う。抽出そのものには影響しない。
    ///
    /// ```
    /// use tally_core::{Key, Selector};
    ///
    /// let selector = Selector::new(Key::JsonField("lvl".to_owned()));
    ///
    /// // 取り出せた。
    /// assert_eq!(
    ///     selector.select("{\"lvl\":\"warn\"}", 1).expect("読める").as_deref(),
    ///     Some("warn")
    /// );
    /// // フィールドが無い。失敗ではなく None。
    /// assert_eq!(selector.select("{\"other\":1}", 2).expect("読める"), None);
    /// // 値が null も同じ扱い。
    /// assert_eq!(selector.select("{\"lvl\":null}", 3).expect("読める"), None);
    /// // JSON として壊れている行は失敗。行番号が付く。
    /// assert_eq!(selector.select("not json", 4).expect_err("壊れている").line_no, 4);
    /// ```
    ///
    /// [ADR-0004]: ../../../docs/adr/0004-error-type-shape.md
    pub fn select<'a>(
        &self,
        line: &'a str,
        line_no: usize,
    ) -> Result<Option<Cow<'a, str>>, LineError> {
        // 文脈（行番号・抜粋）を付けるのはここ 1 回だけ。`extract` は付けない。
        let Some(value) = self
            .key
            .extract(line)
            .map_err(|kind| LineError::new(line_no, line, kind))?
        else {
            return Ok(None);
        };

        Ok(Some(if self.ignore_case {
            fold_case(value)
        } else {
            value
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn json(field: &str) -> Selector {
        Selector::new(Key::JsonField(field.to_owned()))
    }

    // --- 構築 ---

    #[test]
    fn 既定は大小を区別する() {
        let selector = Selector::new(Key::WholeLine);
        assert_eq!(select_one(&selector, "Info").as_ref(), "Info");
    }

    #[test]
    fn ビルダーは元の値を壊さずに積める() {
        // `ignore_case` は self を取って返すので、鎖の途中の値は残らない。
        // 「既に組み立てた Selector を後から書き換える」手段が無いことが要点。
        let base = Selector::new(Key::WholeLine);
        let ci = base.clone().ignore_case(true);
        assert_ne!(base, ci);
        assert_eq!(base, Selector::new(Key::WholeLine));
    }

    #[test]
    fn key_は構築時のものを返す() {
        let selector = json("lvl");
        assert_eq!(selector.key(), &Key::JsonField("lvl".to_owned()));
    }

    // --- 抽出 ---

    #[test]
    fn whole_line_は前後の空白を落とす() {
        assert_eq!(
            select_one(&Selector::new(Key::WholeLine), "  a  ").as_ref(),
            "a"
        );
    }

    #[test]
    fn whole_line_は借用のまま返る() {
        // 無条件に String を作る実装にすると、行数ぶんの確保が増える。
        let extracted = select_one(&Selector::new(Key::WholeLine), "a");
        assert!(matches!(extracted, Cow::Borrowed(_)), "{extracted:?}");
    }

    #[test]
    fn json_の文字列と数値と真偽値を取り出せる() {
        assert_eq!(select_one(&json("v"), "{\"v\":\"s\"}").as_ref(), "s");
        assert_eq!(select_one(&json("v"), "{\"v\":12}").as_ref(), "12");
        assert_eq!(select_one(&json("v"), "{\"v\":true}").as_ref(), "true");
    }

    #[test]
    fn フィールドが無い行は失敗ではなく_none() {
        // 「取り出せなかった」を失敗にするかは Counter の方針。ここでは判断しない。
        assert_eq!(
            json("lvl").select("{\"other\":1}", 1).expect("読める"),
            None
        );
    }

    #[test]
    fn 値が_null_の行も_none() {
        // 「取り出せたか否か」で一貫させる設計なので、null も欠損と同じ扱い。
        assert_eq!(
            json("lvl").select("{\"lvl\":null}", 1).expect("読める"),
            None
        );
    }

    #[test]
    fn 集計に使えない形の値は_none_ではなく失敗() {
        // 「値が無い」のではなく「集計に使えない形をしている」ので、
        // strict に関わらず常に失敗にする。
        let err = json("v")
            .select("{\"v\":[1,2]}", 5)
            .expect_err("配列は失敗するはず");
        assert_eq!(err.line_no, 5);
        assert!(
            matches!(err.kind, LineErrorKind::UnsupportedFieldType { .. }),
            "{:?}",
            err.kind
        );
    }

    #[test]
    fn 壊れた_json_は失敗する() {
        let err = json("lvl").select("not json", 9).expect_err("壊れている");
        assert_eq!(err.line_no, 9);
        assert!(
            matches!(err.kind, LineErrorKind::InvalidJson { .. }),
            "{:?}",
            err.kind
        );
    }

    #[test]
    fn extract_は行番号を知らない() {
        // **文脈を付けるのは select だけ。** extract は LineErrorKind を返す。
        // 「行番号を 2 箇所で付ける」形にすると、片方が 0 始まりになる類の
        // 食い違いが生まれる。
        let kind = Key::JsonField("lvl".to_owned())
            .extract("not json")
            .expect_err("壊れている");
        assert!(matches!(kind, LineErrorKind::InvalidJson { .. }));
    }

    #[test]
    fn select_が付ける行番号は引数のものになる() {
        for line_no in [1, 42, usize::MAX] {
            let err = json("lvl")
                .select("not json", line_no)
                .expect_err("壊れている");
            assert_eq!(err.line_no, line_no);
        }
    }

    // --- 正規化 ---

    #[test]
    fn 小文字化で変化しない値は借用のまま返る() {
        // to_lowercase() を無条件に呼ぶ実装にすると、
        // Cow::Owned になってこのテストが落ちる。
        let selector = Selector::new(Key::WholeLine).ignore_case(true);
        let extracted = select_one(&selector, "already lower");
        assert!(
            matches!(extracted, Cow::Borrowed(_)),
            "不要なアロケーションが発生している: {extracted:?}"
        );
    }

    #[test]
    fn 小文字化が必要な値だけが所有値になる() {
        let selector = Selector::new(Key::WholeLine).ignore_case(true);
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

        let selector = Selector::new(Key::WholeLine).ignore_case(true);
        assert_eq!(select_one(&selector, "ǅ").as_ref(), "ǆ");
    }

    #[test]
    fn 小文字化で長さが変わる文字も壊れない() {
        // 'İ' (U+0130) の小文字化は 2 文字（'i' + 合成用ドット）に伸びる。
        // ASCII 前提の in-place 変換で処理すると壊れる。
        let selector = Selector::new(Key::WholeLine).ignore_case(true);
        let extracted = select_one(&selector, "İ");
        assert_eq!(
            extracted.chars().count(),
            2,
            "実際: {:?}",
            extracted.as_ref()
        );
    }

    #[test]
    fn ignore_case_は値に効きフィールド名には効かない() {
        // フィールド名の一致は厳密なまま。"Lvl" は "lvl" とは一致しない。
        let selector = json("lvl").ignore_case(true);
        assert_eq!(select_one(&selector, "{\"lvl\":\"INFO\"}").as_ref(), "info");
        assert_eq!(
            selector.select("{\"Lvl\":\"INFO\"}", 1).expect("読める"),
            None
        );
    }
}
