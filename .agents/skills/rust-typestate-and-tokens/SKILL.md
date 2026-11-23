---
name: rust-typestate-and-tokens
description: Design compile-time state machines (typestate pattern) and proof-of-permission token types in Rust. Use when a value's valid operations depend on which step of a protocol it's in (builders, serializers, connection handshakes), when you want "calling this without permission" to be a compile error instead of a runtime check, or when indexes/handles need to be proven valid without repeated bounds checks.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/idiomatic/leveraging-the-type-system/{typestate-pattern,token-types}/**
---

# Rust Typestate Pattern & Token Types

Two related techniques for moving a runtime check into the type system so misuse becomes a
compile error: **typestate** (encode *which step of a protocol* a value is in) and **tokens**
(a value that exists only as *proof* something was checked). Neither is in `rust-patterns`.

## Typestate pattern

Encode part of a value's runtime state in its type, so each state exposes only the operations
valid for it — the previous state's methods are consumed and simply don't exist on the next
type.

```rust
struct Serializer { output: String }
struct SerializeStruct { serializer: Serializer }

impl Serializer {
    fn serialize_struct(mut self, name: &str) -> SerializeStruct {
        writeln!(&mut self.output, "{name} {{").unwrap();
        SerializeStruct { serializer: self }
    }
    fn finish(self) -> String { self.output }
}
impl SerializeStruct {
    fn serialize_field(mut self, key: &str, value: &str) -> Self {
        writeln!(&mut self.serializer.output, "  {key}={value};").unwrap();
        self
    }
    fn finish_struct(mut self) -> Serializer { /* closes the struct, returns to Serializer */ }
}
```

`Serializer::default().serialize_struct("User").finish()` — calling `finish()` before
`finish_struct()` isn't a runtime "wrong state" error, it's a **method that doesn't exist on
that type**, caught at compile time. Each transition method takes `self` by value, consuming
the current state so it can't be reused after moving to the next one.

**Design recipe**:
1. One struct per distinct state in the protocol.
2. Each transition is a method that takes `self` by value and returns the *next* state's
   type — this both advances state and prevents the old state from being reused.
3. Only implement, on each state's type, the operations that are actually valid in that state.
   The absence of a method is the enforcement mechanism — don't add a runtime `match` on an
   internal enum "just in case," that defeats the purpose.

**Scaling to branching protocols** (not just a straight line): once a struct can appear at
multiple points (e.g. a serializer field can be a plain value, a nested struct, or an entry in
a list), a single "finish" method can't have one fixed return type — the correct next state
depends on *where* you are in the nesting, not just what type you're finishing. This is where
plain typestate structs stop scaling and you either (a) parameterize the state type generically
over "what comes after" (see `typestate-generics` in the course for a fully worked
serializer — same idea as below, applied to a whole grammar of states), or (b) fall back to a
runtime state check for just the branching part while keeping typestate for the parts that are
linear.

## Token types — proof-of-permission

A token is a value whose *existence* is the proof that something was checked — the function
requiring the token doesn't need to check again; being callable at all *is* the check.

```rust
mod admin {
    pub struct AdminToken(());                 // private field: can't construct outside this module
    pub fn get_admin(password: &str) -> Option<AdminToken> {
        if password == "Password123" { Some(AdminToken(())) } else { None }
    }
}
pub fn add_moderator(_: &admin::AdminToken, user: &str) { /* no permission check needed here */ }
```

`add_moderator` cannot be called without an `AdminToken`, and an `AdminToken` cannot be
constructed except by `get_admin` succeeding — so `add_moderator`'s signature *is* its access
control, enforced by the compiler rather than an `if !is_admin { return Err(...) }` at the top
of every privileged function. **The foundation of a useful token is that it cannot be
constructed arbitrarily** — a private tuple field (`AdminToken(())`) with no public
constructor other than the gated one is the standard way to ensure that.

**Tokens that carry data**: `MutexGuard` is a token-with-data — it's simultaneously "proof you
hold the lock" and a handle to the protected value via `Deref`/`DerefMut`. Unlike C++, where a
lock guard is just a flag the programmer must remember to check before touching shared data,
Rust's `MutexGuard` is the *only* way to reach the data at all — there is no path to the
`Mutex`'s contents that skips acquiring the guard. When designing your own "permission + data"
API, follow this shape: don't expose the protected data through any path that doesn't require
presenting the token.

## Branded types — tying a token to one specific instance

A plain token (like `ProvenIndex(usize)`, "this index is valid for *some* `Bytes`") can be
misused across instances — nothing stops passing a `ProvenIndex` obtained from one `Bytes`
into a different `Bytes`, which is either a logic bug or (if the receiving method skips
bounds-checking because it trusts the token, e.g. via `get_unchecked`) **undefined behavior**.

The fix is "branding": tag both the container and its tokens with a unique, per-instance
lifetime that the compiler will not unify across two different instances, so a token from one
container simply doesn't type-check against another.

```rust
struct InvariantLifetime<'id>(PhantomData<*mut &'id ()>);   // invariant over 'id — key detail
struct Bytes<'id>(Vec<u8>, InvariantLifetime<'id>);
struct ProvenIndex<'id>(usize, InvariantLifetime<'id>);

impl<'id> Bytes<'id> {
    // Construction goes through a closure, not a plain constructor —
    // this is what makes each call site's `'id` a fresh, distinct brand.
    fn new<T>(bytes: Vec<u8>, f: impl for<'a> FnOnce(Bytes<'a>) -> T) -> T {
        f(Bytes(bytes, InvariantLifetime::default()))
    }
    fn get_index(&self, ix: usize) -> Option<ProvenIndex<'id>> {
        if ix < self.0.len() { Some(ProvenIndex(ix, InvariantLifetime::default())) } else { None }
    }
    fn get_proven(&self, ix: &ProvenIndex<'id>) -> u8 {
        unsafe { *self.0.get_unchecked(ix.0) }   // sound: 'id ties this token to this Bytes only
    }
}
```

Key mechanics (this is genuinely advanced — reach for it only when the payoff is real, e.g.
eliminating a hot-path bounds check that profiling shows matters, or a `ghost-cell`/session-
type-style API):
- `PhantomData<*mut &'id ()>` (or an equivalent invariant marker) makes `'id`
  **invariant** — the compiler will not treat two different `'id`s as interchangeable the way
  it normally would treat lifetimes (which are usually *covariant*, i.e. freely shortenable/
  unifiable). Without invariance, the brand doesn't actually prevent cross-instance misuse.
- Construction happens inside a **closure taking a `for<'a> FnOnce(Bytes<'a>) -> T`**, not a
  plain `fn new() -> Bytes` — this "higher-ranked" closure is what forces each call to `new`
  to mint a genuinely fresh, unique `'id` that can't be confused with any other call's `'id`.
  This is the same shape `GhostCell`/`generativity` crates use.
- The practical upshot: `data_1.get_proven(&token_from_data_2)` fails to **compile**, not just
  to run correctly — the earlier plain-`ProvenIndex` version only panicked/UB'd at runtime.

## When to reach for which

| Need | Tool |
|---|---|
| "This value must go through steps A→B→C in order" | Typestate — one type per step |
| "This function should only be callable after some check passed" | Plain token — private-field struct returned only by the gated constructor |
| "This handle/index must not cross between multiple instances of the same type" | Branded token — invariant lifetime + closure-based construction |
| A resource genuinely needs runtime-checked exclusive access (not just a one-time permission check) | `RAII` drop guard (see `rust-newtype-and-raii`), which a token type can also serve as (e.g. `MutexGuard`) |

Don't reach for branding by default — it adds real complexity (HRTB closures, invariance,
`unsafe` in the "proven" fast path) that's only worth it when the alternative is a real,
measured cost (a bounds check in a hot loop, or a genuine soundness hole from cross-instance
handle confusion). For the common case, a plain typestate or token without branding is enough.
