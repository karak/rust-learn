//! 14 章「型で不変条件を守る（newtype・RAII ガード・typestate）」の実例。
//!
//! 引数の取り違えを防ぐ newtype、入口で一度だけ検証する経路型、空にならない列、
//! `Drop` で元に戻す一時変更ガード、所有権の消費で手順を縛る typestate を見る。
//!
//! 1 章から 13 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::mem;
use std::num::NonZeroUsize;
use std::ops::Deref;

/// 状態コードごとの件数。
pub type Counts = BTreeMap<u16, u64>;

/// ホスト名を表す newtype。`Endpoint` と同じ `String` だが、別の型として区別される。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host(String);

impl Host {
    /// ホスト名を作る。
    #[must_use]
    // `impl Into<String>` は「`String` に変換できる型なら何でも」を受ける書き方で、
    // `&str` でも `String` でも呼べる。
    pub fn new(host: impl Into<String>) -> Self {
        Self(host.into())
    }

    /// 文字列スライスとして返す。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// パスを表す newtype。`Host` と取り違えるとコンパイルエラーになる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint(String);

impl Endpoint {
    /// パスを作る。
    #[must_use]
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self(endpoint.into())
    }

    /// 文字列スライスとして返す。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// ホスト名とパスから URL を組み立てる。
#[must_use]
pub fn format_url(host: &Host, endpoint: &Endpoint) -> String {
    format!("https://{}{}", host.as_str(), endpoint.as_str())
}

/// フィールド経路の書式違反。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldPathError {
    /// 先頭が `/` で始まっていない。
    MissingLeadingSlash,
    /// 空のセグメントを含んでいる（例: `/` や `/req//status`）。
    EmptySegment,
}

impl fmt::Display for FieldPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingLeadingSlash => f.write_str("経路は '/' で始まる必要があります"),
            Self::EmptySegment => f.write_str("空のセグメントは含められません"),
        }
    }
}

impl Error for FieldPathError {}

/// 検証済みのフィールド経路（例: `/req/status`）。
///
/// 構築に成功した時点で、先頭が `/` であり、空でないセグメントを 1 つ以上持つことが決まっている。
/// `Deref<Target = str>` は実装するが `DerefMut` は実装しないため、外から検証を迂回して書き換えられない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPath {
    raw: Box<str>,
    first: Box<str>,
    rest: Box<[Box<str>]>,
}

impl FieldPath {
    /// 文字列を検証して `FieldPath` を作る。
    pub fn new(raw: &str) -> Result<Self, FieldPathError> {
        let Some(body) = raw.strip_prefix('/') else {
            return Err(FieldPathError::MissingLeadingSlash);
        };
        let mut parts = body.split('/');
        let first = parts.next().unwrap_or("");
        if first.is_empty() {
            return Err(FieldPathError::EmptySegment);
        }
        let mut rest = Vec::new();
        for part in parts {
            if part.is_empty() {
                return Err(FieldPathError::EmptySegment);
            }
            rest.push(Box::from(part));
        }
        Ok(Self {
            raw: Box::from(raw),
            first: Box::from(first),
            rest: rest.into_boxed_slice(),
        })
    }

    /// 先頭のセグメントを返す。構造上必ず 1 つ以上あるので `Option` にならない。
    #[must_use]
    pub fn first_segment(&self) -> &str {
        &self.first
    }

    /// 全セグメントを順に返す。
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        std::iter::once(&*self.first).chain(self.rest.iter().map(Deref::deref))
    }
}

impl Deref for FieldPath {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

/// 要素を 1 つ以上持つ列。
///
/// 先頭と残りを別フィールドに分けることで、「空の列」を型の上で作れなくしている。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmpty<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmpty<T> {
    /// 先頭 1 要素だけで作る。
    #[must_use]
    pub fn new(first: T) -> Self {
        Self {
            first,
            rest: Vec::new(),
        }
    }

    /// `Vec<T>` から作る。空なら `None`。
    #[must_use]
    pub fn from_vec(values: Vec<T>) -> Option<Self> {
        let mut iter = values.into_iter();
        let first = iter.next()?;
        Some(Self {
            first,
            rest: iter.collect(),
        })
    }

    /// 末尾に要素を足す。
    pub fn push(&mut self, value: T) {
        self.rest.push(value);
    }

    /// 先頭の要素を返す。空になりえないので `Option` が要らない。
    #[must_use]
    pub fn first(&self) -> &T {
        &self.first
    }

    /// 要素数を返す。必ず 1 以上なので `NonZeroUsize` になる。
    #[must_use]
    pub fn len(&self) -> NonZeroUsize {
        NonZeroUsize::MIN.saturating_add(self.rest.len())
    }
}

/// スコープのあいだだけ値を差し替え、`Drop` で元に戻す RAII ガード。
///
/// `#[must_use]` により、戻り値を受け取らずに捨てるとコンパイラが警告する。
#[must_use = "ガードを変数に束縛しないと、その場ですぐ元に戻ります"]
#[derive(Debug)]
pub struct OverrideGuard<'a, T> {
    target: &'a RefCell<T>,
    previous: Option<T>,
}

impl<'a, T> OverrideGuard<'a, T> {
    /// `target` の中身を `replacement` に差し替え、スコープ終了時に元へ戻すガードを返す。
    pub fn new(target: &'a RefCell<T>, replacement: T) -> Self {
        let previous = mem::replace(&mut *target.borrow_mut(), replacement);
        Self {
            target,
            previous: Some(previous),
        }
    }
}

impl<T> Drop for OverrideGuard<'_, T> {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            *self.target.borrow_mut() = previous;
        }
    }
}

/// typestate の第 1 段階: まだ入力を読み込んでいない状態。
///
/// この型には `aggregate` も `counts` も生えていないため、読み込み前に集計は呼べない。
///
/// 読み込む前に集計を呼ぶと、コンパイルが通らない。
///
/// ```compile_fail,E0599
/// let _ = book_ch14::Draft::new().aggregate();
/// ```
///
/// `load` が `self` を消費するので、同じ `Draft` を二度読み込むこともできない。
///
/// ```compile_fail,E0382
/// let draft = book_ch14::Draft::new();
/// let _loaded = draft.load(vec![200]);
/// let _again = draft.load(vec![404]);
/// ```
// 本文は `Default` を検証の迂回路として戒めているが、`Draft` は守る条件を持たない
// 最初の状態なので、既定値で作れても飛ばせる手順が無い。
#[derive(Debug, Default)]
pub struct Draft {
    _private: (),
}

impl Draft {
    /// 下書き状態のパイプラインを作る。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 状態コードの列を読み込み、次の状態 `Loaded` へ進む。
    ///
    /// `self` を値で受け取って消費するため、元の `Draft` は以後使えない。
    #[must_use]
    pub fn load(self, codes: Vec<u16>) -> Loaded {
        Loaded { codes }
    }
}

/// typestate の第 2 段階: 入力を読み込み終えた状態。
#[derive(Debug)]
pub struct Loaded {
    codes: Vec<u16>,
}

impl Loaded {
    /// 読み込んだ状態コードを集計し、完了状態 `Aggregated` へ進む。
    #[must_use]
    pub fn aggregate(self) -> Aggregated {
        let mut counts = Counts::new();
        for code in self.codes {
            *counts.entry(code).or_insert(0) += 1;
        }
        Aggregated { counts }
    }
}

/// typestate の第 3 段階: 集計が終わった状態。
#[derive(Debug)]
pub struct Aggregated {
    counts: Counts,
}

impl Aggregated {
    /// 集計結果を返す。
    #[must_use]
    pub fn counts(&self) -> &Counts {
        &self.counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newtype_でホスト名とパスから_url_を組み立てられる() {
        let host = Host::new("example.com");
        let endpoint = Endpoint::new("/health");
        assert_eq!(format_url(&host, &endpoint), "https://example.com/health");
    }

    #[test]
    fn field_path_は入口で不正な綴りを弾く() {
        assert_eq!(
            FieldPath::new("req/status"),
            Err(FieldPathError::MissingLeadingSlash)
        );
        assert_eq!(FieldPath::new("/"), Err(FieldPathError::EmptySegment));
        assert_eq!(
            FieldPath::new("/req//status"),
            Err(FieldPathError::EmptySegment)
        );

        let path = FieldPath::new("/req/status").expect("正しい経路");
        assert_eq!(&*path, "/req/status");
        assert_eq!(path.first_segment(), "req");
        let segs: Vec<&str> = path.segments().collect();
        assert_eq!(segs, vec!["req", "status"]);
    }

    #[test]
    fn non_empty_は空の_vec_から作れず先頭を直接返せる() {
        assert_eq!(NonEmpty::<u16>::from_vec(Vec::new()), None);

        let from_vec = NonEmpty::from_vec(vec![200u16, 404, 500]).expect("空でない");
        assert_eq!(*from_vec.first(), 200);
        assert_eq!(from_vec.len().get(), 3);

        let mut list = NonEmpty::new(200u16);
        list.push(404);
        assert_eq!(*list.first(), 200);
        assert_eq!(list.len().get(), 2);
    }

    #[test]
    fn override_guard_はスコープを抜けると元の値に戻す() {
        let level = RefCell::new("info");
        {
            let _guard = OverrideGuard::new(&level, "debug");
            assert_eq!(*level.borrow(), "debug");
        }
        assert_eq!(*level.borrow(), "info");
    }

    #[test]
    fn override_guard_を外側から先に捨てると内側が覚えた値が残る() {
        let level = RefCell::new("info");
        let outer = OverrideGuard::new(&level, "debug");
        let inner = OverrideGuard::new(&level, "trace");
        drop(outer);
        drop(inner);
        assert_eq!(*level.borrow(), "debug");
    }

    #[test]
    #[should_panic(expected = "RefCell already borrowed")]
    fn override_guard_は借りたままの_refcell_に作ると_panic_する() {
        let level = RefCell::new("info");
        let _shared = level.borrow();
        let _guard = OverrideGuard::new(&level, "debug");
    }

    #[test]
    fn typestate_で読み込みから集計まで順に進む() {
        let aggregated = Draft::new().load(vec![200, 404, 200]).aggregate();
        assert_eq!(*aggregated.counts(), Counts::from([(200, 2), (404, 1)]));
    }
}
