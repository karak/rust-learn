---
name: rust-ownership-and-lifetimes
description: Diagnose Rust borrow checker and lifetime errors, and choose between references, Cell/RefCell, Rc/Weak, and owned data. Use when rustc reports a borrow/lifetime error (E0502, E0499, E0597, E0106, "cannot borrow as mutable"), when adding lifetime annotations to a struct or function, or when deciding how a type should hold or share data it doesn't own.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/{borrowing,lifetimes,memory-management,smart-pointers}/**,
  src/idiomatic/leveraging-the-type-system/borrow-checker-invariants/**
---

# Rust Ownership, Borrowing & Lifetimes

For "which idiom should I default to," see the general `rust-patterns` skill first. This
skill is for when a borrow/lifetime/ownership decision **is** the task: a compile error to
fix, a struct that needs to hold borrowed data, or a choice between `&T`, `Rc`, `RefCell`,
and cloning.

## Borrow checker error triage

Rust's borrow checker enforces two independent rules. Almost every borrow error is a
violation of one of them:

1. **Outlives rule**: a reference cannot outlive the value it borrows.
2. **Aliasing rule** ("aliasing XOR mutability"): for a given value, at any point in time,
   you can have either (a) one or more shared `&T` references, or (b) exactly one exclusive
   `&mut T` reference — never both.

| Compiler message / code | Which rule | Typical fix |
|---|---|---|
| ``error[E0502]: cannot borrow `x` as mutable because it is also borrowed as immutable`` | Aliasing | End the shared borrow before creating the mutable one (see NLL below), or restructure so only one reference is live at a time. |
| ``error[E0499]: cannot borrow `x` as mutable more than once`` | Aliasing | You have two `&mut` at once — pass one by value/consume it, or scope them so they don't overlap. |
| ``error[E0597]: `x` does not live long enough`` / borrowed value dropped while still borrowed | Outlives | The referent is dropped before the reference is used. Extend the referent's lifetime (move it up a scope) or stop borrowing (clone, or restructure ownership). |
| `error[E0106]: missing lifetime specifier` | Elision failed | Rust's elision rules (below) couldn't infer one — annotate explicitly. |
| "cannot move out of `x` because it is borrowed" | Aliasing + move | You're trying to move a value while a reference to it is alive. Clone, or drop the reference first. |
| Iterator invalidation (`vec.push` while iterating `&vec`) | Aliasing | Mutating a collection while holding references into it can reallocate and dangle those references — collect changes into a separate buffer, then apply them after the loop, or use indices instead of references. |

**Key fact about the aliasing rule**: conflicting references must not *exist* at the same
point in the code — it does not matter whether they're ever dereferenced. If replacing an
intermediate `let r = &mut x;` with a direct mutation of `x` produces the same error, that's
confirmation you're looking at an aliasing violation, not something else.

**Non-lexical lifetimes (NLL)**: a reference's lifetime ends at its **last use**, not at the
end of its enclosing scope. This is why moving a `dbg!(shared_ref)` to *before* a later
`&mut` borrow is a legitimate, idiomatic fix — the compiler already tracks last-use, not
brace-nesting. If a fix isn't obvious, try reordering statements so the shared borrow's last
use happens before the exclusive borrow starts.

**Fields borrow independently, methods don't**: `s.field_a` and `s.field_b` can be borrowed
simultaneously and independently. But calling `s.some_method()` borrows the *whole* `s`,
which will conflict with any other live borrow of any field. If you hit a spurious-looking
aliasing error through a method call, check whether accessing the field(s) directly instead
resolves it (a well-known borrow-checker limitation, not a real aliasing violation).

## Lifetime elision — when you don't need annotations

Function signatures need lifetimes on every reference, but Rust elides them per three rules,
applied in order:

1. Each unannotated reference parameter gets its own distinct lifetime.
2. If there's exactly **one** input lifetime, it's assigned to all elided output lifetimes.
3. If one of the inputs is `&self`/`&mut self`, **its** lifetime is assigned to all elided
   output lifetimes (even with other reference params present).

```rust
fn only_args(a: &i32, b: &i32) {}           // rule 1: two independent lifetimes, no output
fn identity(a: &i32) -> &i32 { a }          // rule 2: one input → output gets it
impl Foo {
    fn get(&self, other: &i32) -> &i32 {    // rule 3: `&self`'s lifetime → output
        &self.0
    }
}
```

If none of the three rules resolves every elided lifetime, `E0106` fires and you must
annotate manually — this is not a case where the compiler is "inferring" incorrectly, it's
telling you the shape is genuinely ambiguous (e.g. two `&` params, no `self`, and a `&`
return).

## Lifetimes in structs

A struct that stores a reference must be annotated, and the annotation is a **contract**: the
struct cannot outlive the data it points to.

```rust
struct Highlight<'document> {
    slice: &'document str,
    color: HighlightColor,
}
```

`doc` must stay alive for as long as any `Highlight` built from it exists — the borrow
checker rejects the struct instance surviving past `doc`'s drop point, not just direct use of
`doc` after that point.

**Design guidance**: types with borrowed data give the caller a lightweight *view* but force
them to keep the original data alive and in scope, which propagates lifetime parameters
outward and makes the type harder to compose (especially inside other structs, return types,
or anything stored long-term). **Default to owning data** (`String` not `&str`, `Vec<T>` not
`&[T]`) unless the type is explicitly a short-lived, zero-copy view (e.g. a parser's token,
a slice adapter) — the course frames this as: use borrowed-data structs for “lightweight
views,” not as a default representation.

Returning a reference from a function extends the borrow of whatever argument it was derived
from — the caller can't invalidate that argument (e.g. mutate or move it) while the returned
reference is alive. With one reference argument, the returned reference is unambiguously tied
to it; with multiple, you generally need explicit lifetime annotations to say which.

## Choosing a data-sharing strategy

Work top-to-bottom; each option adds a cost the previous one didn't need.

1. **Owned value, moved or passed by `&`/`&mut`** — the default. No runtime cost, borrow
   checker verifies safety at compile time. Prefer this unless one of the below is genuinely
   needed.
2. **`Clone`** — when ownership genuinely needs to be duplicated and the type is cheap (or
   the code isn't hot). Rust makes clones explicit and visible in the diff, unlike C++'s
   implicit copy-on-`=`; that's a feature, not a compromise — don't reach for `Rc` just to
   avoid writing `.clone()`.
3. **`Cell<T>`** — shared (`&self`) get/set access to a `Copy`-able value with **zero runtime
   check**, because it never hands out a reference to the interior — only whole-value
   `get`/`set`. Use for small `Copy` types (counters, flags, cached numbers) behind a shared
   reference.
4. **`RefCell<T>`** — shared (`&self`) access that *does* need `&T`/`&mut T`-shaped borrows
   (via `.borrow()`/`.borrow_mut()`), enforced by a **runtime** counter instead of the
   compiler. Violating the aliasing rule at runtime panics (`already borrowed: BorrowMutError`)
   instead of failing to compile — treat every `.borrow_mut()` call as something that can
   panic if a `Ref`/`RefMut` from the same `RefCell` is still alive elsewhere, and scope
   borrows as tightly as possible (an explicit `{ }` block around a `borrow_mut()` is a common,
   idiomatic way to force the guard to drop before the next borrow).
5. **`Rc<T>`** — multiple **owners** of the same heap allocation, single-threaded, with a
   reference count. `Rc::clone` is O(1) (bumps a counter, doesn't deep-copy) and can be
   ignored when profiling for correctness issues, but it does not give you mutation — pair
   with `RefCell<T>` (`Rc<RefCell<T>>`) when shared *and* mutable access is required.
   `Rc::downgrade` produces a `Weak<T>` that doesn't keep the value alive — use it to break
   reference cycles (e.g. a parent→child `Rc` graph needs child→parent as `Weak`, or the
   cycle never gets dropped and leaks). For multi-threaded sharing, this becomes `Arc<T>`
   (+ `Mutex<T>`/`RwLock<T>` instead of `RefCell<T>`) — see `rust-concurrency-sync`.

## Using the aliasing rule as an API design tool

"Aliasing XOR mutability" isn't just a constraint to work around — it can *encode an
invariant in the type system* so misuse fails to compile instead of silently returning wrong
data.

```rust
pub struct Transaction<'a> {
    connection: &'a mut DatabaseConnection,
}
impl<'a> Transaction<'a> {
    pub fn new(connection: &'a mut DatabaseConnection) -> Self { Self { connection } }
    pub fn query(&mut self, _query: &str) { /* fire off async query */ }
    pub fn commit(self) { /* consume self, results now safe to read */ }
}
```

Because `Transaction::new` takes `&mut DatabaseConnection`, the connection is completely
locked out (no `db.results()`, no starting a second transaction) for as long as the
`Transaction` is alive. Only after `commit(self)` consumes the transaction does the borrow
end and `db.results()` become callable again. This turns "don't read results before the
transaction finishes" from a documentation comment into a compile error — see
`rust-typestate-and-tokens` for the more general pattern of encoding protocol state this way.

## Quick checklist when a borrow error won't go away

1. Identify which rule is violated (table above) — don't guess, read the message's blamed
   lines.
2. Check whether the fix is reordering (NLL: move the last-use earlier) before reaching for
   `clone()` or `RefCell`.
3. If it's a method-call-borrows-whole-struct false positive, try direct field access.
4. If the type genuinely needs to hand out a reference into itself while also being mutated
   elsewhere, that's the signal to introduce `Cell`/`RefCell`, or to redesign ownership
   (e.g. return an owned copy instead of a reference, use indices/handles instead of
   references, or restructure with a typestate like `Transaction` above).
5. Prefer owning data in struct fields; only add a lifetime parameter when the type is
   deliberately a short-lived, zero-copy view.
