//! 13 章「`Drop`・`Deref` とスマートポインタ、内部可変性」の実例。
//!
//! 状態コードの選別・ラベル・共有の集計表を通して、`Drop`、`Box`、`Deref`、`Rc` / `Weak`、
//! `Cell` / `RefCell` の役割と組み合わせ方を見る。
//!
//! 1 章から 12 章のコードとは独立している。章を飛ばして読んでも動く。

#[cfg(test)]
mod exercises;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// 状態コードごとの件数。
pub type Counts = BTreeMap<u16, u64>;

/// 破棄されたときに、渡されたログへ自分の名前を残すガード。
///
/// `Drop` が走る順番や、`drop(x)` で早めに手放したときの動きを確かめるために使う。
#[derive(Debug)]
pub struct TraceGuard<'a> {
    name: &'static str,
    log: &'a RefCell<Vec<&'static str>>,
}

impl<'a> TraceGuard<'a> {
    /// 名前と記録先のログを受け取ってガードを作る。
    #[must_use]
    pub fn new(name: &'static str, log: &'a RefCell<Vec<&'static str>>) -> Self {
        Self { name, log }
    }
}

impl Drop for TraceGuard<'_> {
    fn drop(&mut self) {
        self.log.borrow_mut().push(self.name);
    }
}

/// 状態コードを選ぶ条件の木。
///
/// 自分自身を子に持つ再帰的な型なので、`Box` でヒープに置いて大きさを確定させる。
#[derive(Debug, PartialEq, Eq)]
pub enum Filter {
    /// 指定した状態コードに一致する。
    Code(u16),
    /// 2 つの条件を両方満たす。
    And(Box<Self>, Box<Self>),
    /// 内の条件を満たさない。
    Not(Box<Self>),
}

impl Filter {
    /// 状態コードが条件に合うかを返す。
    #[must_use]
    pub fn matches(&self, code: u16) -> bool {
        match self {
            Self::Code(target) => code == *target,
            Self::And(left, right) => left.matches(code) && right.matches(code),
            Self::Not(inner) => !inner.matches(code),
        }
    }
}

/// 変更しない名前を `Box<str>` で持つラベル。
///
/// `Deref<Target = str>` を実装しているため、`&Label` をそのまま `&str` として渡せる。
/// `DerefMut` は実装しないので、参照外し経由で中身を書き換えることはできない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    text: Box<str>,
}

impl Label {
    /// 文字列からラベルを作る。
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self {
            text: Box::from(text),
        }
    }
}

impl Deref for Label {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

/// 複数の記録係から共有される集計表。
///
/// 1 本のスレッドの中で持ち主を分けるために `Rc` を使い、
/// 共有参照のまま表を書き換えるために `RefCell` で包んでいる。
#[derive(Debug, Default, Clone)]
pub struct SharedCounter {
    counts: Rc<RefCell<Counts>>,
}

impl SharedCounter {
    /// 空の共有集計表を作る。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 同じ集計表を指す記録係を作る。
    #[must_use]
    pub fn recorder(&self, name: &str) -> Recorder {
        Recorder {
            label: Label::new(name),
            counts: Rc::clone(&self.counts),
            recorded: Cell::new(0),
        }
    }

    /// いま集計表を持っている持ち主の数を返す。
    #[must_use]
    pub fn owners(&self) -> usize {
        Rc::strong_count(&self.counts)
    }

    /// 現在の集計表のコピーを返す。
    #[must_use]
    pub fn snapshot(&self) -> Counts {
        self.counts.borrow().clone()
    }
}

/// 共有の集計表に件数を足す記録係。
///
/// 自分が記録した回数 `recorded` は `Cell<u64>` に持っているので、
/// `&self` のまま 1 足せる。
#[derive(Debug)]
pub struct Recorder {
    label: Label,
    counts: Rc<RefCell<Counts>>,
    recorded: Cell<u64>,
}

impl Recorder {
    /// 記録係の名前を返す。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.label
    }

    /// 状態コードを 1 件数える。
    pub fn record(&self, code: u16) {
        *self.counts.borrow_mut().entry(code).or_insert(0) += 1;
        self.recorded.set(self.recorded.get() + 1);
    }

    /// この記録係が数えた件数を返す。
    #[must_use]
    pub fn recorded(&self) -> u64 {
        self.recorded.get()
    }
}

/// 親への参照を `Weak` で持つ木の節。
///
/// 親から子へは `Rc` で所有し、子から親へは `Weak` で指すことで循環参照を防ぐ。
#[derive(Debug)]
pub struct Node {
    /// 節の名前。
    pub name: &'static str,
    parent: RefCell<Weak<Self>>,
    children: RefCell<Vec<Rc<Self>>>,
}

impl Node {
    /// 子を持たない節を作る。
    #[must_use]
    pub fn new(name: &'static str) -> Rc<Self> {
        Rc::new(Self {
            name,
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(Vec::new()),
        })
    }

    /// `parent` に `child` をぶら下げ、子から親へは `Weak` を張る。
    pub fn attach(parent: &Rc<Self>, child: &Rc<Self>) {
        *child.parent.borrow_mut() = Rc::downgrade(parent);
        parent.children.borrow_mut().push(Rc::clone(child));
    }

    /// 親がまだ生きていれば、親の名前を返す。
    #[must_use]
    pub fn parent_name(&self) -> Option<&'static str> {
        self.parent.borrow().upgrade().map(|p| p.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ローカル変数は宣言の逆順に_drop_される() {
        let log = RefCell::new(Vec::new());
        {
            let _a = TraceGuard::new("a", &log);
            let _b = TraceGuard::new("b", &log);
        }
        assert_eq!(*log.borrow(), vec!["b", "a"]);
    }

    #[test]
    fn 構造体のフィールドは宣言の順に_drop_される() {
        struct Pair<'a> {
            _first: TraceGuard<'a>,
            _second: TraceGuard<'a>,
        }

        let log = RefCell::new(Vec::new());
        {
            let _pair = Pair {
                _first: TraceGuard::new("first", &log),
                _second: TraceGuard::new("second", &log),
            };
        }
        assert_eq!(*log.borrow(), vec!["first", "second"]);
    }

    #[test]
    fn drop_でスコープの途中でも手放せる() {
        let log = RefCell::new(Vec::new());
        {
            let a = TraceGuard::new("a", &log);
            let _b = TraceGuard::new("b", &log);
            drop(a);
            log.borrow_mut().push("middle");
        }
        assert_eq!(*log.borrow(), vec!["a", "middle", "b"]);
    }

    #[test]
    fn 再帰的な_filter_で状態コードを選べる() {
        let not_ok = Filter::Not(Box::new(Filter::Code(200)));
        let only_404 = Filter::And(Box::new(not_ok), Box::new(Filter::Code(404)));
        assert!(!only_404.matches(200));
        assert!(only_404.matches(404));
        assert!(!only_404.matches(500));
    }

    #[test]
    fn label_は_deref_で_str_として渡せる() {
        fn is_api(name: &str) -> bool {
            name.starts_with("api-")
        }

        let label = Label::new("api-east");
        assert!(is_api(&label));
        assert_eq!(label.len(), 8);
    }

    #[test]
    fn 複数の記録係が同じ表に数え入れる() {
        let counter = SharedCounter::new();
        assert_eq!(counter.owners(), 1);

        let east = counter.recorder("east");
        let west = counter.recorder("west");
        assert_eq!(counter.owners(), 3);

        east.record(200);
        west.record(200);
        west.record(404);

        assert_eq!(east.name(), "east");
        assert_eq!(east.recorded(), 1);
        assert_eq!(west.recorded(), 2);
        assert_eq!(counter.snapshot(), Counts::from([(200, 2), (404, 1)]));

        drop(east);
        drop(west);
        assert_eq!(counter.owners(), 1);
    }

    #[test]
    fn weak_なら親が消えたあと_none_になる() {
        let child = Node::new("child");
        {
            let parent = Node::new("parent");
            Node::attach(&parent, &child);
            assert_eq!(child.parent_name(), Some("parent"));
            assert_eq!(Rc::strong_count(&parent), 1);
            assert_eq!(Rc::weak_count(&parent), 1);
        }
        assert_eq!(child.parent_name(), None);
    }

    #[test]
    #[should_panic(expected = "RefCell already borrowed")]
    fn refcell_の二重の可変借用は実行時に_panic_する() {
        let cell = RefCell::new(0u64);
        let _first = cell.borrow_mut();
        let _second = cell.borrow_mut();
    }

    #[test]
    #[should_panic(expected = "RefCell already borrowed")]
    fn borrow_を持ったままの_borrow_mut_も同じ文面で_panic_する() {
        let cell = RefCell::new(0u64);
        let _shared = cell.borrow();
        let _exclusive = cell.borrow_mut();
    }

    /// 互いを `Rc` で指す 2 つの値。`Drop` が走ったかを外の旗で見る。
    struct Cyclic {
        other: RefCell<Option<Rc<Cyclic>>>,
        dropped: Rc<std::cell::Cell<u32>>,
    }

    impl Drop for Cyclic {
        fn drop(&mut self) {
            self.dropped.set(self.dropped.get() + 1);
        }
    }

    #[test]
    fn rc_の循環は外の変数が消えても_drop_が走らない() {
        let dropped = Rc::new(std::cell::Cell::new(0));
        {
            let a = Rc::new(Cyclic {
                other: RefCell::new(None),
                dropped: Rc::clone(&dropped),
            });
            let b = Rc::new(Cyclic {
                other: RefCell::new(None),
                dropped: Rc::clone(&dropped),
            });
            *a.other.borrow_mut() = Some(Rc::clone(&b));
            *b.other.borrow_mut() = Some(Rc::clone(&a));
        }
        // 循環が残したカウントのせいで、どちらの `Drop` も走らない（このテストは意図してリークする）。
        assert_eq!(dropped.get(), 0);
    }

    #[test]
    fn try_borrow_mut_なら_panic_せずに_err_で受け取れる() {
        let cell = RefCell::new(0u64);
        let first = cell.borrow();
        assert!(cell.try_borrow_mut().is_err());
        drop(first);
        assert!(cell.try_borrow_mut().is_ok());
    }
}
