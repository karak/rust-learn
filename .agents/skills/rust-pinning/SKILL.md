---
name: rust-pinning
description: Understand and correctly use Pin<Ptr> — what pinning actually prevents, Unpin vs !Unpin, PhantomPinned, and the specific ways Drop can accidentally move a pinned value. Use when hand-implementing Future, building a self-referential struct, seeing "cannot be unpinned" or an unsatisfied `Unpin` trait bound compile error, or implementing Drop for a !Unpin type.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/unsafe-deep-dive/pinning/**
---

# Rust Pinning

`Pin` trips up even experienced Rust users. This is deliberately a standalone skill (not
folded into `rust-async`, which links here) because the mechanics are general — they matter
any time a type must never move in memory, not only inside `async fn`.

## What a "move" actually is in Rust

Every move — even for a type that doesn't implement `Copy` — is a **bitwise memcpy** to a new
location, with the compiler considering the old location's contents "no longer valid" but
doing nothing to the actual bytes:

```rust
let a = DynamicBuffer::default();
let mut b = a;   // compiles to an actual memcpy of DynamicBuffer's bytes from `a`'s
                  // stack slot to `b`'s — verifiable in the generated LLVM IR/assembly
```

The implication that matters for pinning: **a value's memory address is never stable** by
default. Anything that stores a pointer to *its own* other field (a self-reference) breaks the
instant the containing value is moved — the pointer now points at the old, stale location.

## What pinning is (and isn't)

**Pinning prevents a value from being moved**, once it's behind `Pin<Ptr>`, for as long as it
doesn't implement `Unpin`. This matters for exactly one reason: it makes self-referential
structs (a struct holding a pointer into its own other field) sound to construct, which is
otherwise impossible in safe Rust — the compiler generates a self-referential struct
automatically whenever an `async fn`/block holds a reference across an `.await` point.

**`Pin` is a library type, not a language feature.** It doesn't change move semantics in the
language — it's a wrapper that relies on Rust's existing ownership system: `Pin<Ptr>` owns the
pointer it wraps and simply **never exposes a safe API that would let you move the pointee
out**. No new compiler magic; it's an API design constraint enforced by ownership, the same
way any other type enforces its own invariants by controlling its public surface.

```rust
// Simplified — the real std definition has a private field and extra bounds/methods; this
// captures the shape that matters here.
#[repr(transparent)]
pub struct Pin<Ptr> { pointer: Ptr }

impl<Ptr: Deref<Target: Unpin>> Pin<Ptr> {
    pub fn new(pointer: Ptr) -> Pin<Ptr> { ... }             // safe — only for Unpin targets
}
impl<Ptr: Deref> Pin<Ptr> {
    pub unsafe fn new_unchecked(pointer: Ptr) -> Pin<Ptr> { ... }  // required for !Unpin targets
}
```

`Pin::new()` only accepts pointers whose target already implements `Unpin` — for those, `Pin`
adds no actual guarantee (they were always safe to move), so wrapping them is a zero-cost,
purely-compile-time formality. For a `!Unpin` target, you must go through
`Pin::new_unchecked`, an `unsafe fn` — calling it is *you* asserting the value will genuinely
never move again for the rest of its life, which the compiler cannot verify for you.

**Why this feels awkward**: `Pin`'s design specifically favored a simpler implementation for
the compiler over ergonomics, because its original target audience was the small number of
people writing async runtimes, who could absorb that complexity so that ordinary `async fn`
users mostly never touch `Pin` directly. If you're hitting `Pin` errors in ordinary
application-level `async fn` code, that's usually a sign of manually implementing `Future` or
mishandling a boxed future — not a sign you need to become a `Pin` expert (see `rust-async`
Pitfall 2).

## `Unpin` — the "safe to move even when pinned" opt-out

`Unpin` is an **auto trait**: the compiler implements it automatically for almost every type,
meaning "I have no self-references, so moving me is always fine, `Pin` or no `Pin`." When a
type is `Unpin`, wrapping it in `Pin` doesn't actually restrict anything — `Pin<&mut T>` for
`T: Unpin` can still be freely moved via `Pin::get_mut`.

Only `!Unpin` types are genuinely constrained by `Pin` — that opt-out happens automatically
for:
- Compiler-generated `Future`s from `async fn`/blocks that hold cross-`.await` self-references
- Any type containing a `PhantomPinned` field (deliberate manual opt-out)
- Some FFI wrapper types around C++ objects that are inherently address-sensitive

```rust
// This is std's own (unstable) implementation, shown for how the opt-out works — `impl !Trait`
// (negative impl) needs #![feature(negative_impls)] and isn't available on stable Rust. You
// can't write this yourself; use `PhantomPinned` as a field instead, as shown below.
pub struct PhantomPinned;
impl !Unpin for PhantomPinned {}
```

Add a `_pin: PhantomPinned` field to any struct you're hand-writing that has (or will have)
internal self-references, to force it to be `!Unpin` — this is the deliberate, manual version
of what the compiler does automatically for generated futures.

## `Drop` on a `!Unpin` type — the sharpest edge

`Drop::drop` takes `&mut self` — and `&mut self` is, in general, enough to move out of fields
via `ptr::read`, `mem::replace`, `mem::take`, or plain reassignment. **None of those operations
know or care that the value is pinned.** Using any of them inside `Drop` for a `!Unpin` type
can silently violate the pinning guarantee, even though nothing about the code looks obviously
wrong:

```rust
struct SelfRef { data: String, ptr: *const String }
impl Drop for SelfRef {
    fn drop(&mut self) {
        // BAD: ptr::read bitwise-copies `self.data` out. `_dupe` now owns a
        // second copy of the same heap allocation `self.data` still owns.
        // When `_dupe` drops at the end of this function, that's a double free.
        let _dupe = unsafe { std::ptr::read(&self.data) };
    }
}
```

The bug isn't specific to `String`/double-free — it generalizes: **any operation that moves
data out of a `&mut self` inside `Drop` risks invalidating self-references or the pinning
guarantee**, because `Drop::drop`'s `&mut self` gives no compiler-enforced signal that "this
value must never actually be relocated." When implementing `Drop` for a type that contains
(or once contained, via a pointer) self-references:
- Never use `ptr::read`, `mem::replace`, `mem::take`, `mem::swap`, or a plain move-assignment
  on a field that participates in a self-reference.
- Drop fields in place (`ptr::drop_in_place` on a raw pointer to the field, if you must do it
  manually) rather than moving them out to drop them by value.
- If possible, structure the type so `Drop` only needs to release external resources (a raw
  allocation, an FFI handle) rather than touching the self-referential fields at all.

## Worked pattern: constructing a self-referential type behind `Pin`

```rust
pub struct SelfReferentialBuffer {
    data: [u8; 1024],
    cursor: *mut u8,          // points into `data`, above — this is the self-reference
    _pin: PhantomPinned,      // forces !Unpin
}

impl SelfReferentialBuffer {
    pub fn new() -> Pin<Box<Self>> {
        let buffer = SelfReferentialBuffer { data: [0; 1024], cursor: null_mut(), _pin: PhantomPinned };
        let mut pinned = Box::pin(buffer);          // heap-allocate and pin in one step
        unsafe {
            // SAFETY: we have exclusive access via `as_mut()`, and we're only
            // writing a pointer into `data`, not moving the struct itself.
            let mut_ref = Pin::get_unchecked_mut(pinned.as_mut());
            mut_ref.cursor = mut_ref.data.as_mut_ptr();   // now self-referential
        }
        pinned   // returned as Pin<Box<Self>> — caller can never move it out unsafely
    }
}
```

Key moves in this pattern:
1. Construct the value **without** the self-reference filled in yet (`cursor: null_mut()`).
2. `Box::pin(...)` heap-allocates it and immediately wraps it in `Pin` — the value never sits
   un-pinned at a temporary stack location that a self-reference could then invalidate.
3. Only *after* it's pinned do you use `Pin::get_unchecked_mut` (unsafe — you're promising not
   to move it) to fill in the self-referencing pointer.
4. Return `Pin<Box<Self>>`, never `Self` or `Box<Self>` — this is what stops every downstream
   caller from ever getting a movable, unwrapped value.

## Quick diagnosis

| Symptom | Meaning |
|---|---|
| `the trait bound `T: Unpin` is not satisfied` | Something (often `Pin::new`, vs. `Pin::new_unchecked`, or a generic bound) is requiring `Unpin` on a type that doesn't have it — usually a compiler-generated future or a type with `PhantomPinned` |
| "cannot move out of dereference of `Pin<...>`" | Correct enforcement working as intended — you need `Pin::get_mut`/`get_unchecked_mut`/`as_mut()` and to restructure so you don't need ownership of the pointee |
| Double-free / UB traced to a `Drop` impl on a self-referential type | Check for `ptr::read`/`mem::replace`/`mem::take`/reassignment on a field involved in the self-reference — see the `Drop` section above |
| Confusion about why `Pin` is even required for `async fn` | See the mental model in `rust-async` — the generated future struct can hold cross-`.await` self-references |
