//! 13 章の演習の答え。
//!
//! `cargo test -p book-ch13` で確かめられる。

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

/// 演習 1: スコープの出入りをログに積むガード。
///
/// 作ったときに `"enter: 名前"` を積み、スコープを抜けて `Drop` が走るときに
/// `"leave: 名前"` を積む。途中で早期 `return` しても必ず出口が記録される。
struct ScopeLog<'a> {
    name: &'static str,
    log: &'a RefCell<Vec<String>>,
}

impl<'a> ScopeLog<'a> {
    fn enter(log: &'a RefCell<Vec<String>>, name: &'static str) -> Self {
        log.borrow_mut().push(format!("enter: {name}"));
        Self { name, log }
    }
}

impl Drop for ScopeLog<'_> {
    fn drop(&mut self) {
        self.log.borrow_mut().push(format!("leave: {}", self.name));
    }
}

/// 演習 2: 1 つの表を `Rc<RefCell<...>>` で共有して語を数え、自分の件数は `Cell` で持つ記録係。
struct WordRecorder {
    counts: Rc<RefCell<BTreeMap<String, u64>>>,
    own_count: Cell<usize>,
}

impl WordRecorder {
    fn new(counts: Rc<RefCell<BTreeMap<String, u64>>>) -> Self {
        Self {
            counts,
            own_count: Cell::new(0),
        }
    }

    fn record(&self, word: &str) {
        *self.counts.borrow_mut().entry(word.to_owned()).or_insert(0) += 1;
        self.own_count.set(self.own_count.get() + 1);
    }

    fn own_count(&self) -> usize {
        self.own_count.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_step(log: &RefCell<Vec<String>>, fail: bool) -> Result<(), &'static str> {
        let _guard = ScopeLog::enter(log, "parse");
        if fail {
            return Err("bad input");
        }
        log.borrow_mut().push("body".to_owned());
        Ok(())
    }

    #[test]
    fn 正常終了でも早期_return_でも_leave_が積まれる() {
        let log = RefCell::new(Vec::new());
        assert_eq!(run_step(&log, false), Ok(()));
        assert_eq!(run_step(&log, true), Err("bad input"));
        assert_eq!(
            *log.borrow(),
            vec![
                "enter: parse",
                "body",
                "leave: parse",
                "enter: parse",
                "leave: parse",
            ]
        );
    }

    #[test]
    fn 二つの記録係が同じ表に語を数え入れる() {
        let shared = Rc::new(RefCell::new(BTreeMap::new()));
        let a = WordRecorder::new(Rc::clone(&shared));
        let b = WordRecorder::new(Rc::clone(&shared));

        a.record("warn");
        b.record("warn");
        b.record("error");

        assert_eq!(a.own_count(), 1);
        assert_eq!(b.own_count(), 2);
        assert_eq!(
            *shared.borrow(),
            BTreeMap::from([("error".to_owned(), 1), ("warn".to_owned(), 2)])
        );
    }
}
