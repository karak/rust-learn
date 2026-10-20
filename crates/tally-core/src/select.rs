//! キーの抽出と正規化。**「参加する行がどうキーを産むか」だけを扱う。**
//!
//! 「どの行が参加するか」（フィルタ）はここには来ない。上流の
//! [`tally_reader`][crate::tally_reader] が述語で受ける。

use std::borrow::Cow;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::error::{JsonError, JsonPointerError, LineError, LineErrorKind};

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
    /// 行を JSON として解釈し、[JSON Pointer][rfc] の指す値をキーにする。
    ///
    /// **`JsonField` の格上げではなく別のバリアントである。** ポインタは `/` で
    /// 始まるので、`JsonField("lvl")` と `JsonPointer("/lvl")` は綴りが違う
    /// （[ADR-0005] 論点 3）。
    ///
    /// ```
    /// use tally_core::{JsonPointer, Key};
    ///
    /// let key = Key::JsonPointer(JsonPointer::new("/req/status").expect("正しいポインタ"));
    /// assert_ne!(key, Key::WholeLine);
    /// ```
    ///
    /// [rfc]: https://www.rfc-editor.org/rfc/rfc6901
    /// [ADR-0005]: ../../../docs/adr/0005-selector-public-api.md
    JsonPointer(JsonPointer),
}

/// 検証済みの [JSON Pointer][rfc]。
///
/// **構築に成功した時点で、綴りは正しい。** 不正なポインタを持つ値は存在しない
/// （[ADR-0010] 論点 2）。
///
/// # なぜ newtype なのか
///
/// `serde_json` の `Value::pointer` は **先頭が `/` でないと無条件に `None` を返す。**
/// 生の `String` で受けると、`--pointer lvl` のような綴り間違いが
/// 「そのポインタを持たない行」と区別できず、**度数 0 という結果が静かに出る。**
///
/// # 2 つの表現を持っている
///
/// 利用者が書いた綴り（[`as_str`][Self::as_str]）と、解釈済みのセグメント列を
/// 両方持つ。**前者は診断に、後者は行ごとの照合に使う。**
/// `regex` の [`Regex::as_str`][regex] が元のパターンを返しながら、
/// 照合はコンパイル済みのプログラムで行うのと同じ形である。
///
/// ```
/// use tally_core::JsonPointer;
///
/// let pointer = JsonPointer::new("/req/status").expect("正しいポインタ");
/// assert_eq!(pointer.as_str(), "/req/status");
///
/// // `~1` は `/` を表す（RFC 6901）。綴りは保たれる。
/// let escaped = JsonPointer::new("/a~1b").expect("正しいポインタ");
/// assert_eq!(escaped.as_str(), "/a~1b");
/// ```
///
/// [rfc]: https://www.rfc-editor.org/rfc/rfc6901
/// [regex]: https://docs.rs/regex/latest/regex/struct.Regex.html#method.as_str
/// [ADR-0010]: ../../../docs/adr/0010-json-pointer-key.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPointer {
    /// 利用者が書いた綴り。**エラーの文面と `Display` に使う。**
    raw: Box<str>,
    /// 解釈済みの最初のセグメント。`~0` と `~1` は解いてある。
    ///
    /// **残りと分けて持つのは、「セグメントは 1 つ以上ある」を型で表すため。**
    /// 1 本の `Box<[_]>` にすると、走査のたびに空かどうかを問い直すことになる。
    ///
    /// **行ごとに解き直さない。** 1 行につきセグメントの数だけ確保することになる。
    first: Box<str>,
    /// 2 つ目以降のセグメント。
    rest: Box<[Box<str>]>,
}

impl JsonPointer {
    /// 綴りを検証して組み立てる。
    ///
    /// # Errors
    ///
    /// 空、先頭が `/` でない、`-` セグメントを含む、`~` の後ろが `0` でも `1` でもない。
    /// それぞれの理由は [`JsonPointerError`] を参照。
    pub fn new(raw: &str) -> Result<Self, JsonPointerError> {
        if raw.is_empty() {
            return Err(JsonPointerError::Empty);
        }
        let Some(body) = raw.strip_prefix('/') else {
            return Err(JsonPointerError::NotRooted);
        };

        // **`split('/')` は空のセグメントを落とさない。** `/a//b` の真ん中は
        // 「空文字列というキー」であり、RFC 6901 ではこれが正しい。
        // **`/` だけなら要素は 1 つ（空文字列）**で、これも「空というキー」である。
        let mut parts = body.split('/');
        let first = unescape_segment(parts.next().unwrap_or(""))?;
        let rest = parts.map(unescape_segment).collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            raw: raw.into(),
            first,
            rest: rest.into_boxed_slice(),
        })
    }

    /// 利用者が書いた綴りを返す。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// 走査用の seed を作る。**公開しない。** 内部表現である。
    fn seed(&self) -> PickPath<'_> {
        PickPath {
            head: &self.first,
            rest: &self.rest,
        }
    }
}

/// 1 セグメントのエスケープを解く。
///
/// **`~` を含まないセグメントは確保せずに済ませる。** ほとんどのポインタは
/// エスケープを含まない。
fn unescape_segment(segment: &str) -> Result<Box<str>, JsonPointerError> {
    if segment == "-" {
        return Err(JsonPointerError::DashSegment);
    }
    if !segment.contains('~') {
        return Ok(segment.into());
    }

    let mut unescaped = String::with_capacity(segment.len());
    let mut chars = segment.chars();
    while let Some(c) = chars.next() {
        if c != '~' {
            unescaped.push(c);
            continue;
        }
        match chars.next() {
            Some('0') => unescaped.push('~'),
            Some('1') => unescaped.push('/'),
            _ => return Err(JsonPointerError::BadEscape),
        }
    }
    Ok(unescaped.into())
}

impl fmt::Display for JsonPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl FromStr for JsonPointer {
    type Err = JsonPointerError;

    /// **`clap` の値パーサがこれを使う。** `--filter` の `Regex` と同じ形で、
    /// 引数解釈の時点で検証させるための実装である。
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for JsonPointer {
    fn as_ref(&self) -> &str {
        &self.raw
    }
}

impl Deref for JsonPointer {
    type Target = str;

    /// **`Deref` を実装してよいのは、これが「文字列に検証を足したもの」だから。**
    /// 文字列として読む操作は全部そのまま通してよく、
    /// **検証を迂回して中身を書き換える経路が無い**（`DerefMut` は実装しない）。
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
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
            // **`JsonField` の経路と `match` の形が同じで、`Unsupported` の
            // 行き先だけが違う。** 共通化していないのは、上の各腕に段階 7 の
            // 測定の記録が付いているためで、動かすと記録の文脈が切れる。
            Self::JsonPointer(pointer) => {
                let mut de = serde_json::Deserializer::from_str(line);
                let picked = pointer
                    .seed()
                    .deserialize(&mut de)
                    .and_then(|picked| de.end().map(|()| picked))
                    .map_err(|source| LineErrorKind::InvalidJson {
                        source: JsonError::new(source),
                    })?;

                match picked {
                    Picked::Borrowed(found) => Ok(Some(Cow::Borrowed(found))),
                    Picked::Owned(found) => Ok(Some(Cow::Owned(found))),
                    Picked::Number(found) => Ok(Some(Cow::Owned(found.to_string()))),
                    Picked::Bool(found) => Ok(Some(Cow::Owned(found.to_string()))),
                    Picked::Missing => Ok(None),
                    Picked::Unsupported => Err(LineErrorKind::UnsupportedPointerType {
                        pointer: pointer.as_str().into(),
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

/// 1 行から **ポインタの指す 1 つの値だけ** を取り出す seed。
///
/// **[`PickField`] の入れ子版である。** 違いは 2 つ。
///
/// 1. **自分自身を `next_value_seed` に渡す。** セグメントを 1 つ削って再帰する
/// 2. **配列を辿る。** セグメントが RFC 6901 の添字なら、その位置だけ拾う
///
/// `serde_json::Value` を作らない方針は [`PickField`] と同じ
/// （[ADR-0010] 論点 1）。
///
/// **いま照合しているセグメント（`head`）と、その先（`rest`）を分けて持つ。**
/// 「セグメントが尽きた」は `rest` が空であることで表し、
/// **「セグメントが 1 つも無い」という状態は作れない。**
///
/// [ADR-0010]: ../../../docs/adr/0010-json-pointer-key.md
struct PickPath<'p> {
    head: &'p str,
    rest: &'p [Box<str>],
}

impl PickPath<'_> {
    /// 1 つ先へ進んだ seed。**末端なら `None`。**
    fn descend(&self) -> Option<Self> {
        self.rest
            .split_first()
            .map(|(head, rest)| Self { head, rest })
    }
}

/// セグメントを配列の添字として読む。
///
/// **RFC 6901 は先頭 0 を許さない。** `01` はオブジェクトのキーとしては有効なので、
/// ここで弾いても「キー `01` を持つオブジェクト」は辿れる。
/// `usize::from_str` が `+1` を通すので、**数字だけであることも見る。**
fn array_index(segment: &str) -> Option<usize> {
    if segment.is_empty() || !segment.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if segment.len() > 1 && segment.starts_with('0') {
        return None;
    }
    segment.parse().ok()
}

impl<'de> DeserializeSeed<'de> for PickPath<'_> {
    type Value = Picked<'de>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for PickPath<'_> {
    type Value = Picked<'de>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON の値")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut found = Picked::Missing;
        // **全キーを走査する。** 打ち切ると後続の構文エラーを見逃す（`PickField` と同じ）。
        while let Some(matched) = map.next_key_seed(MatchKey { field: self.head })? {
            if matched {
                found = match self.descend() {
                    Some(deeper) => map.next_value_seed(deeper)?,
                    None => map.next_value_seed(PickValue)?,
                };
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(found)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let wanted = array_index(self.head);
        let mut found = Picked::Missing;
        let mut position = 0usize;

        // **添字が無くても最後まで読む。** `PickField` が配列を読み捨てるのと同じ理由で、
        // 途中でやめると構文検査が甘くなる。
        loop {
            if Some(position) == wanted {
                let picked = match self.descend() {
                    Some(deeper) => seq.next_element_seed(deeper)?,
                    None => seq.next_element_seed(PickValue)?,
                };
                let Some(picked) = picked else {
                    break;
                };
                found = picked;
            } else if seq.next_element::<IgnoredAny>()?.is_none() {
                break;
            }
            position += 1;
        }
        Ok(found)
    }

    // --- 走査できない形は「その値が無い」と同じ扱い（`PickField` に揃える）---

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

    /// **空白の定義は Unicode の `White_Space`。**
    ///
    /// 段階 7 で ASCII の速い経路（`str::trim_ascii`）を試したときに、
    /// **垂直タブ（`0x0B`）の扱いが違う**ことが分かった。
    /// `trim_ascii` は `WhatWG` の定義で、垂直タブを空白と見なさない。
    /// 最適化自体は測って効果が出なかったので戻したが、
    /// **この落とし穴はテストとして残す**（次に同じ置き換えを試す人のため）。
    #[test]
    fn 空白除去は_ascii_でも非_ascii_でも同じ規則() {
        // ASCII の空白 5 種 + 空白。
        assert_eq!(
            select_one(&Selector::new(Key::WholeLine), " \t\r\n\u{0b}\u{0c}a\t ").as_ref(),
            "a"
        );
        // 非 ASCII の空白（U+3000 IDEOGRAPHIC SPACE）も落ちる。
        assert_eq!(
            select_one(&Selector::new(Key::WholeLine), "\u{3000}あ\u{3000}").as_ref(),
            "あ"
        );
        // 非 ASCII を含むが空白は ASCII の行。
        assert_eq!(
            select_one(&Selector::new(Key::WholeLine), "  あ  ").as_ref(),
            "あ"
        );
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

    // --- JSON Pointer の検証（ADR-0010 論点 2）---

    fn pointer(raw: &str) -> Selector {
        Selector::new(Key::JsonPointer(
            JsonPointer::new(raw).expect("正しいポインタのはず"),
        ))
    }

    #[test]
    fn 先頭が斜線でないポインタは組み立てられない() {
        // ここを通すと、綴り間違いが「そのポインタを持たない行」と区別できず、
        // 度数 0 という結果が静かに出る。newtype がある主な理由。
        assert_eq!(JsonPointer::new("lvl"), Err(JsonPointerError::NotRooted));
    }

    #[test]
    fn 空のポインタは組み立てられない() {
        assert_eq!(JsonPointer::new(""), Err(JsonPointerError::Empty));
    }

    #[test]
    fn 末尾を指す記号は組み立てられない() {
        // RFC 6901 の `-` は「配列の最後の次」で、必ず存在しない位置を指す。
        assert_eq!(
            JsonPointer::new("/items/-"),
            Err(JsonPointerError::DashSegment)
        );
    }

    #[test]
    fn 不正なエスケープは組み立てられない() {
        assert_eq!(JsonPointer::new("/a~2b"), Err(JsonPointerError::BadEscape));
        assert_eq!(JsonPointer::new("/a~"), Err(JsonPointerError::BadEscape));
    }

    #[test]
    fn 綴りはそのまま保たれる() {
        // 内部ではエスケープを解いているが、診断に使うのは利用者が書いた綴り。
        let p = JsonPointer::new("/a~1b").expect("正しいポインタ");
        assert_eq!(p.as_str(), "/a~1b");
        assert_eq!(p.to_string(), "/a~1b");
    }

    // --- JSON Pointer の抽出（ADR-0010 論点 1 と論点 3）---

    #[test]
    fn 入れ子のフィールドを取り出せる() {
        assert_eq!(
            select_one(&pointer("/req/status"), r#"{"req":{"status":"500"}}"#).as_ref(),
            "500"
        );
    }

    #[test]
    fn 配列の添字を辿れる() {
        assert_eq!(
            select_one(&pointer("/items/1"), r#"{"items":["a","b","c"]}"#).as_ref(),
            "b"
        );
    }

    #[test]
    fn 添字が範囲を超えたら取り出せない() {
        assert_eq!(
            pointer("/items/9")
                .select(r#"{"items":["a"]}"#, 1)
                .expect("読める"),
            None
        );
    }

    #[test]
    fn 先頭に零の付いた添字は配列の添字にならない() {
        // RFC 6901 は先頭 0 を添字として認めない。オブジェクトのキーとしては有効なので、
        // 配列に対しては空振りし、キー "01" を持つオブジェクトなら辿れる。
        assert_eq!(
            pointer("/items/01")
                .select(r#"{"items":["a","b"]}"#, 1)
                .expect("読める"),
            None
        );
        assert_eq!(
            select_one(&pointer("/items/01"), r#"{"items":{"01":"x"}}"#).as_ref(),
            "x"
        );
    }

    #[test]
    fn エスケープを含むキーを指せる() {
        // `~1` は `/`、`~0` は `~`。
        assert_eq!(
            select_one(&pointer("/a~1b"), r#"{"a/b":"x"}"#).as_ref(),
            "x"
        );
        assert_eq!(
            select_one(&pointer("/a~0b"), r#"{"a~b":"x"}"#).as_ref(),
            "x"
        );
    }

    #[test]
    fn 途中の段が辿れない行はスキップになる() {
        // 途中が無い場合も、途中がスカラの場合も、区別せず「その値が無い」とする。
        for line in [r#"{"req":{}}"#, r#"{"req":"ok"}"#, r"{}", "[1,2]"] {
            assert_eq!(
                pointer("/req/status").select(line, 1).expect("読める"),
                None,
                "失敗した行: {line}"
            );
        }
    }

    #[test]
    fn 値が_null_の行はスキップになる() {
        assert_eq!(
            pointer("/req/status")
                .select(r#"{"req":{"status":null}}"#, 1)
                .expect("読める"),
            None
        );
    }

    #[test]
    fn 集計に使えない形はエラーになる() {
        let err = pointer("/req/status")
            .select(r#"{"req":{"status":[1]}}"#, 1)
            .expect_err("配列は集計に使えないので失敗するはず");
        assert!(
            matches!(
                err.kind,
                LineErrorKind::UnsupportedPointerType { ref pointer }
                    if pointer.as_ref() == "/req/status"
            ),
            "実際: {:?}",
            err.kind
        );
    }

    #[test]
    fn ポインタでも借用のまま返る() {
        // 段階 7 の改善と同じ性質を新しい経路でも保つ（ADR-0010 論点 1）。
        // エスケープが無ければ入力を借用したまま返す。
        let extracted = select_one(&pointer("/req/status"), r#"{"req":{"status":"500"}}"#);
        assert!(matches!(extracted, Cow::Borrowed(_)), "{extracted:?}");
    }

    #[test]
    fn ポインタの経路でも末尾のゴミを見逃さない() {
        // `de.end()` を忘れると通ってしまう行。
        let err = pointer("/lvl")
            .select(r#"{"lvl":"a"} ゴミ"#, 1)
            .expect_err("末尾にゴミがあるので失敗するはず");
        assert!(
            matches!(err.kind, LineErrorKind::InvalidJson { .. }),
            "実際: {:?}",
            err.kind
        );
    }

    #[test]
    fn 重複したキーは後勝ちになる() {
        // `PickField` と同じ規則（`Value` が `BTreeMap` で後勝ちだったのに揃えてある）。
        assert_eq!(
            select_one(&pointer("/lvl"), r#"{"lvl":"a","lvl":"b"}"#).as_ref(),
            "b"
        );
    }
}
