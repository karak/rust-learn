//! キーの抽出と正規化。**「参加する行がどうキーを産むか」だけを扱う。**
//!
//! 「どの行が参加するか」（フィルタ）はここには来ない。上流の
//! [`tally_reader`][crate::tally_reader] が述語で受ける。

use std::borrow::Cow;
use std::fmt;

use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};

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
                // **`serde_json::Value` を作らない**（段階 7 の改善 2）。
                // `Value` は行の全フィールドを `BTreeMap<String, Value>` に積むので、
                // 1 つしか要らないのに残りも確保し、行ごとに捨てることになる。
                // 実測では構築 31.5% + 破棄 15.8% で、この関数の費用の大半だった。
                let mut de = serde_json::Deserializer::from_str(line);
                let picked = PickField { field }
                    .deserialize(&mut de)
                    // **`end()` を忘れない。** `serde_json::from_str` はこれを含むので、
                    // 省くと `{"lvl":"a"} ゴミ` のような行が通ってしまう。
                    .and_then(|picked| de.end().map(|()| picked))
                    .map_err(|source| LineErrorKind::InvalidJson {
                        source: JsonError::new(source),
                    })?;

                match picked {
                    // 引用符の中にエスケープが無ければ、**入力を借用したまま返る。**
                    // 以前は `Value::String` から必ず `clone()` していた。
                    Picked::Borrowed(found) => Ok(Some(Cow::Borrowed(found))),
                    Picked::Owned(found) => Ok(Some(Cow::Owned(found))),
                    // **`serde_json::Number` を経由する。** `f64` に落として
                    // `to_string()` すると `3.0` が `3` になり、出力が変わる。
                    Picked::Number(found) => Ok(Some(Cow::Owned(found.to_string()))),
                    Picked::Bool(found) => Ok(Some(Cow::Owned(found.to_string()))),
                    Picked::Missing => Ok(None),
                    Picked::Unsupported => Err(LineErrorKind::UnsupportedFieldType {
                        field: field.as_str().into(),
                    }),
                }
            }
        }
    }
}

/// 1 行から **指定した 1 フィールドだけ** を取り出す seed。
///
/// **`Deserialize` ではなく `DeserializeSeed`。** 探すフィールド名は実行時の値
/// （`--field` の引数）なので、型に埋め込めない。
/// C#/Java の「デシリアライザに引数を渡す」に相当するものが serde では seed である。
struct PickField<'f> {
    field: &'f str,
}

/// 取り出した値。**JSON の DOM を作らずに済む形だけを持つ。**
enum Picked<'a> {
    /// エスケープが無かったので、入力を借用している。
    Borrowed(&'a str),
    /// エスケープを解いたので所有している。
    Owned(String),
    /// **`f64` ではなく `Number`。** 表示が `serde_json` と一致する必要がある。
    Number(serde_json::Number),
    Bool(bool),
    /// フィールドが無い、値が `null`、または行がオブジェクトでない。
    Missing,
    /// 配列・オブジェクト。集計に使えない。
    Unsupported,
}

impl<'de> DeserializeSeed<'de> for PickField<'_> {
    type Value = Picked<'de>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // **`deserialize_map` ではなく `deserialize_any`。** 行がオブジェクトでない
        // 場合（`[1,2]` や `"文字列"`）は、以前の実装では `Value::get` が `None` を
        // 返してスキップされていた。エラーにすると振る舞いが変わる。
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for PickField<'_> {
    type Value = Picked<'de>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON の値")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut found = Picked::Missing;
        // **全キーを走査する。** 途中で打ち切ると、後続の構文エラーを見逃す。
        while let Some(matched) = map.next_key_seed(MatchKey { field: self.field })? {
            if matched {
                // **一致するたびに上書きする。** キーが重複した行では、
                // `Value`（`BTreeMap`）が後勝ちだったので、それに揃える。
                found = map.next_value_seed(PickValue)?;
            } else {
                // **値を読み捨てる。** 読まずに次のキーへ進むことはできない。
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(found)
    }

    // --- オブジェクト以外は「フィールドが無い」と同じ扱い ---

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(Picked::Missing)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(Picked::Missing)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(Picked::Missing)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(Picked::Missing)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(Picked::Missing)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Picked::Missing)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // **読み捨てる。** 途中でやめると構文検査が甘くなる。
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Picked::Missing)
    }
}

/// キーが探しているフィールドかどうかだけを返す seed。
///
/// **`String` にも `Cow` にもしない。** どちらも確保を伴うが、
/// ここで要るのは一致するかどうかの真偽値だけである。
struct MatchKey<'f> {
    field: &'f str,
}

impl<'de> DeserializeSeed<'de> for MatchKey<'_> {
    type Value = bool;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl Visitor<'_> for MatchKey<'_> {
    type Value = bool;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "オブジェクトのキー")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(value == self.field)
    }
}

/// 一致したフィールドの値を [`Picked`] として取り出す seed。
struct PickValue;

impl<'de> DeserializeSeed<'de> for PickValue {
    type Value = Picked<'de>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for PickValue {
    type Value = Picked<'de>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "集計に使える値")
    }

    /// **エスケープが無い文字列だけがここに来る。** 入力をそのまま借用できる。
    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Picked::Borrowed(value))
    }

    /// エスケープを解いた文字列。借用できないので確保する。
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Picked::Owned(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Picked::Owned(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Picked::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Picked::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        // JSON に NaN と無限大は書けないので `None` にはならないが、
        // **`unwrap()` を置かずに型で処理する**（このリポジトリの方針 4）。
        Ok(serde_json::Number::from_f64(value).map_or(Picked::Unsupported, Picked::Number))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Picked::Bool(value))
    }

    /// `null`。**エラーではなくスキップ**（`Key::extract` の doc を参照）。
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Picked::Missing)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Picked::Unsupported)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(Picked::Unsupported)
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

    /// **`Value` を経由しない実装でも、表示が変わらないこと。**
    ///
    /// `f64` に落として `to_string()` すると `3.0` が `3` になる。
    /// `serde_json::Number` を保つことでそれを防いでいる（段階 7 の改善 2）。
    #[test]
    fn 小数の表示は_serde_json_と同じ() {
        assert_eq!(select_one(&json("v"), "{\"v\":3.0}").as_ref(), "3.0");
        assert_eq!(select_one(&json("v"), "{\"v\":1.5}").as_ref(), "1.5");
        assert_eq!(select_one(&json("v"), "{\"v\":-0.25}").as_ref(), "-0.25");
    }

    /// エスケープが無ければ **入力を借用したまま返る**（段階 7 の改善 2）。
    #[test]
    fn エスケープの無い_json_文字列は借用のまま返る() {
        let extracted = json("v")
            .select("{\"v\":\"info\"}", 1)
            .expect("読める")
            .expect("値がある");
        assert!(matches!(extracted, Cow::Borrowed(_)), "{extracted:?}");
    }

    /// エスケープを解く必要があれば所有値になる。**解けていることも見る。**
    #[test]
    fn エスケープを含む_json_文字列は解かれて所有値になる() {
        let extracted = json("v")
            .select("{\"v\":\"a\\tb\"}", 1)
            .expect("読める")
            .expect("値がある");
        assert_eq!(extracted.as_ref(), "a\tb");
        assert!(matches!(extracted, Cow::Owned(_)), "{extracted:?}");
    }

    /// **行がオブジェクトでなければスキップ**（フィールドが無いのと同じ扱い）。
    ///
    /// `Value` 経由の実装では `Value::get` が `None` を返していた。
    /// seed の実装でエラーにすると振る舞いが変わるので、`deserialize_any` で受ける。
    #[test]
    fn オブジェクトでない行はスキップする() {
        for line in ["[1,2]", "\"文字列\"", "42", "null", "true"] {
            assert_eq!(
                json("lvl").select(line, 1).expect("読める"),
                None,
                "行: {line}"
            );
        }
    }

    /// キーが重複した行は **後勝ち**（`BTreeMap` に入れていたときと同じ）。
    #[test]
    fn 重複したキーは後勝ち() {
        assert_eq!(
            select_one(&json("v"), "{\"v\":\"first\",\"v\":\"second\"}").as_ref(),
            "second"
        );
    }

    /// 一致するキーの後ろに構文エラーがあっても見逃さない。
    #[test]
    fn 値を取り出せても後続の構文エラーは失敗にする() {
        let err = json("v")
            .select("{\"v\":\"ok\"} ゴミ", 3)
            .expect_err("末尾のゴミで失敗するはず");
        assert!(matches!(err.kind, LineErrorKind::InvalidJson { .. }));
        assert_eq!(err.line_no, 3);
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
