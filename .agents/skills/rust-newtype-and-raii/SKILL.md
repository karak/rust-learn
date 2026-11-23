---
name: rust-newtype-and-raii
description: Wrap a value in a newtype to enforce an invariant or prevent argument mix-ups, or use Drop to guarantee cleanup / enforce that an API is finalized correctly. Use when designing a type that must always be valid once constructed, when a resource (lock, file, transaction, connection) must always be released or finalized, or when reviewing whether a Drop-based guarantee actually holds.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/idiomatic/leveraging-the-type-system/{newtype-pattern,raii}/**
---

# Rust Newtype Pattern & RAII

`rust-patterns` mentions the newtype pattern in passing (~18 lines) for type-safety and has no
RAII section. This skill is for actually **designing** a newtype whose invariant needs to be
airtight, or a `Drop`-based guarantee (a lock, a transaction, a temp file) that must hold even
under panics and early returns.

## Newtype pattern

### Problem 1: semantic confusion between same-typed arguments

```rust
// Both &str — the compiler can't stop you from swapping them at a call site.
fn login(username: &str, password: &str) -> Result<(), LoginError> { ... }
login(password, username);  // compiles, silently wrong — bug or vulnerability
```

Wrapping each in a distinct newtype makes the swap a compile error instead of a runtime bug:

```rust
struct Username(String);
struct Password(String);
fn login(username: &Username, password: &Password) -> Result<(), LoginError> { ... }
login(password, username);  // ❌ won't compile
```

Use this whenever a function takes two-or-more arguments of the *same underlying type* that
are not interchangeable — IDs, coordinates in different units, currency amounts in different
currencies, distinct kinds of strings.

### Problem 2: enforcing an invariant at construction ("parse, don't validate")

A newtype plus a private field plus a fallible constructor makes "every value of this type is
valid" a property the compiler helps hold, not just documentation:

```rust
pub struct Username(String);   // field is NOT pub

impl Username {
    pub fn new(username: String) -> Result<Self, InvalidUsername> {
        if username.is_empty() { return Err(InvalidUsername::CannotBeEmpty); }
        if username.len() > 32 { return Err(InvalidUsername::TooLong { len: username.len() }); }
        Ok(Self(username))
    }
    pub fn as_str(&self) -> &str { &self.0 }   // read-only escape hatch
}
```

Because the tuple field isn't `pub`, code outside the defining module cannot construct a
`Username` except through `new()`, so every `Username` that exists anywhere has already passed
validation. This is strictly stronger than validating at each call site: once construction is
gated, there is no path to an invalid instance, ever.

### Problem 3 (the audit step, don't skip it): is it *actually* encapsulated?

A private field is necessary but not sufficient. **Audit the entire trait/method surface for
anything that hands out mutable access to the interior** — that's the actual bypass surface,
not just `pub` fields:

```rust
impl std::ops::Deref for Username { type Target = str; fn deref(&self) -> &str { &self.0 } }
impl std::ops::DerefMut for Username {                       // ‼️ breaks encapsulation
    fn deref_mut(&mut self) -> &mut str { &mut self.0 }
}
```

`DerefMut` here hands back `&mut str`, letting any caller mutate the wrapped string directly
and bypass every check in `new()`. When reviewing or designing a validating newtype, check
specifically for:
- `DerefMut`, `AsMut`, `BorrowMut`, or any hand-written method returning `&mut` to the field
- A public field, even `pub(crate)` if untrusted code can be in-crate
- Any method that returns an owned copy of the interior in a way that could be mutated and
  put back (e.g. `into_inner()` + a `From<InnerType>` that skips validation)

If none of the type's own module code needs field access outside `new`/accessor methods,
**put the newtype in its own module** — that shrinks the "trusted" surface that can bypass
`new()` down to exactly that module's contents, instead of the type's entire defining crate.

## RAII (`Drop`-based guarantees)

RAII ties resource release (or any cleanup) to a value going out of scope — the release
happens automatically and can't be forgotten by a caller, including on early return or panic
unwinding.

### Drop guards — release-on-drop for an already-acquired resource

```rust
impl Drop for MutexGuard<'_> {
    fn drop(&mut self) { self.mutex.is_locked = false; }   // released, no explicit unlock() exists
}
```

The point: **there is no `unlock()` method to forget to call.** The guard IS the "currently
locked" state; when it's dropped (end of scope, early `return`, or unwind through a panic),
the resource is released. This is the shape behind `MutexGuard`, `RefCell`'s `Ref`/`RefMut`,
and most "acquire returns a token" APIs — see also `rust-typestate-and-tokens` for tokens used
to encode more than just resource release.

**`Mutex`/interior mutability connection**: `lock()` takes `&self` but returns a `MutexGuard`
providing `&mut T` — this works because the `Mutex` manages its own borrowing rules
internally (interior mutability) rather than the compiler's static borrow checker managing
them. `MutexGuard` implements `Deref`/`DerefMut` so it's used like `&mut T` directly.

### Drop bombs — enforcing that an API step isn't skipped

When a value **must** be finalized through a specific method (`commit()`, `close()`,
`rollback()`) before being dropped, panic in `Drop` if that never happened:

```rust
struct Transaction { active: bool }
impl Transaction {
    fn start() -> Self { Self { active: true } }
    fn commit(mut self) -> io::Result<()> {
        writeln!(io::stdout(), "COMMIT")?;
        self.active = false;
        Ok(())
    }
}
impl Drop for Transaction {
    fn drop(&mut self) {
        if self.active { panic!("Transaction dropped without commit!"); }
    }
}
```

**Why this pattern exists at all**: `commit()` needs to return a `Result` (committing can
fail), but `Drop::drop` cannot return anything — so you can't simply do the real finalization
work in `Drop`. The bomb instead makes forgetting to call `commit()`/`rollback()` a loud panic
rather than a silently-skipped step.

**Defusing the bomb on the success path** — once `commit()` has run, prevent the panic from
firing on the subsequent (now-harmless) drop, in one of two ways:
- A runtime flag (`self.active = false`, checked in `Drop`, as above) — simple, but costs a
  field and a branch.
- `std::mem::forget(self)` inside the finalizing method — skips `Drop::drop` from ever running
  at all, so the bomb (and everything else in `Drop`) never fires:
  ```rust
  fn commit(self) -> io::Result<()> {
      writeln!(io::stdout(), "COMMIT")?;
      std::mem::forget(self);   // Drop::drop() will never run for this value
      Ok(())
  }
  ```
  **Caution**: `forget` skips *all* cleanup the type's `Drop` would otherwise do, not just the
  bomb — safe here because `Transaction` owns no heap memory or resources, but forgetting a
  type that *does* own resources (a `Vec`, a file handle, an `Rc`) leaks them. Only use
  `forget` to defuse a bomb on a type that owns nothing else worth cleaning up, or that has
  already released everything else by that point.

`std::mem::forget(t)` and `std::mem::drop(t)` have the identical `fn(T)` signature but opposite
effects: `forget` wraps `t` in `ManuallyDrop` so `Drop::drop` never runs (used both for drop
bombs and, more commonly, for intentionally leaking); `drop` is just `fn drop<T>(_x: T) {}` —
taking ownership and immediately letting it fall out of scope, which is what triggers the
normal `Drop::drop` call. Reach for `drop(x)` to end a borrow/lock early (see
`rust-ownership-and-lifetimes`); reach for `forget` only when you specifically need to opt out
of destructor behavior.

### Scope guards — ad-hoc cleanup without a custom type

The `scopeguard` crate gives you a single-use, closure-based `Drop` without hand-rolling a
type:

```rust
let cleanup = scopeguard::guard(path, |path| { let _ = fs::remove_file(path); });
// ... do the risky work ...
if success {
    ScopeGuard::into_inner(cleanup);  // cancel the cleanup — keep the file
}
// otherwise the guard runs on drop (including on panic) and deletes it
```

Set the guard up **immediately after** acquiring the resource, before any code that could
fail — that ordering is what makes it exception/early-return-safe. `ScopeGuard::into_inner`
disarms it on the success path (analogous to `mem::forget` above, but scoped to just this
crate's API instead of skipping all of `Drop`).

### `Drop` needing ownership of a field: the `Option` workaround

`Drop::drop(&mut self)` only gives you `&mut self`, never `self` by value — so you can't move
a field out of `self` to call a method that consumes it (e.g. `Handle::close(self)`).
Workaround: wrap the field in `Option` so `.take()` can move it out through the `&mut self`:

```rust
struct File(Option<Handle>);
impl Drop for File {
    fn drop(&mut self) {
        let handle = self.0.take().unwrap();  // move out via Option::take
        handle.close();                        // now we own it, can call close(self)
    }
}
```

Trade-off: every other method now has to go through the `Option` even though `None` can't
logically occur outside `drop` — an ergonomics cost for enabling this one operation.

### `Drop` is not guaranteed to run — design around it, don't assume it

Drop can be **skipped** entirely:
- `std::process::exit(0)` terminates immediately with no unwinding — no destructors run.
  Consider `#![deny(clippy::exit)]` if a codebase must guarantee cleanup.
- `std::mem::forget` (see above) explicitly skips it.
- A reference cycle (`Rc`/`Arc` cycle without a `Weak` break, or leaked via `Box::leak`) means
  the value is never dropped because nothing ever reaches a zero refcount.
- Process abort/crash (including a panic inside another `Drop` during unwinding, which
  aborts) skips remaining drops.

**Design implication**: never make a safety-critical invariant (e.g. "this always releases an
OS-level lock," "this file is always deleted") depend *solely* on `Drop` running if the
consequence of it not running is severe — pair it with an external mechanism (OS-level cleanup
on process exit, a supervisory process, a lock lease with a timeout) when the stakes justify
it. For most application-level cleanup (closing a file, releasing an in-process mutex), `Drop`
running is a fine assumption — this caveat matters most for cross-process or persistent
resources.

## Quick checklist

- **Newtype**: private field + fallible constructor enforces the invariant; then audit for
  `DerefMut`/`AsMut`/public fields that leak mutable access before trusting the invariant.
- **RAII**: pick a drop guard when the resource just needs releasing; a drop bomb when a
  specific finalizing method must be called; a scope guard for one-off, closure-based cleanup.
- Defuse bombs with a flag (cheap, needs a field) or `mem::forget` (needs the type to own
  nothing else worth cleaning up).
- Don't rely on `Drop` alone for correctness if `process::exit`, a panic-during-unwind abort,
  or a reference cycle could plausibly happen in that code path.
