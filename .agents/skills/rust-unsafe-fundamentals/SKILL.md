---
name: rust-unsafe-fundamentals
description: Write correct everyday unsafe Rust — dereferencing raw pointers, mutable statics, unions, unsafe functions, and unsafe extern "C" declarations — with proper safety comments. Use when writing your first unsafe block, calling into a C function, or reviewing whether an unsafe block's safety comment actually justifies the code. For advanced soundness reasoning and safety-precondition design, see rust-unsafe-soundness.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/unsafe-rust/**
---

# Rust Unsafe Fundamentals

`rust-patterns` has a short "when unsafe is/isn't acceptable" section. This skill is the
practical how-to for the five things `unsafe` actually unlocks, each with the specific
footguns the course calls out. For deriving/documenting safety preconditions rigorously and
reasoning about soundness end-to-end, see `rust-unsafe-soundness`; for `Pin`, see
`rust-pinning`; for calling C/C++ specifically, see `rust-ffi`.

## What `unsafe` actually is

Rust is two languages: **Safe Rust** (memory safety and no UB, always, mechanically enforced)
and **Unsafe Rust** (can trigger undefined behavior if you violate a precondition — the
compiler stops checking, you're now responsible). `unsafe` does **not** mean "this code is
wrong" — it means "some of the compiler's automatic safety checks are turned off here, and a
human has to have verified correctness instead." The five things `unsafe` unlocks, and only
these five:

1. Dereference raw pointers
2. Read or write mutable `static` variables
3. Access `union` fields
4. Call `unsafe fn`s, including `extern` functions
5. Implement `unsafe trait`s

Everything else in an `unsafe { }` block is still checked normally by the compiler — `unsafe`
doesn't turn off borrow checking or type checking generally, only these five specific things.

**House rule**: keep unsafe code small and isolated, wrap it in a safe abstraction at the
boundary, and document every precondition. Most unsafe you write should be invisible to
callers of the safe API around it.

## 1. Dereferencing raw pointers

Creating a raw pointer is safe; dereferencing one is not.

```rust
let mut x = 10;
let p1: *mut i32 = &raw mut x;
let p2 = p1 as *const i32;

// SAFETY: p1/p2 were derived from a local, so non-null, aligned, and pointing
// into a single stack allocation, which lives for this whole function. Not
// accessed through a reference or from another thread while these pointers
// are used.
unsafe {
    dbg!(*p1);
    *p1 = 6;
    dbg!(*p2);
}
```

**Classic mistake — mixing raw pointers with references to the same data**:

```rust
// UNSOUND. DO NOT DO THIS.
let r: &i32 = unsafe { &*p1 };
dbg!(r);
x = 50;              // mutating through a different path while `r` is alive
dbg!(r);             // UB: the value `r` points to changed underneath a shared reference
```

A shared reference (`&T`) carries a promise that its referent won't change while it's alive.
Deriving a reference from a raw pointer and then mutating the underlying value through *any*
other path (another pointer, a different variable) while that reference is still in scope
breaks the promise — this is UB even though nothing "crashes." Never let a `&T`/`&mut T`
derived from a raw pointer coexist with an independent write path to the same memory.

## 2. Mutable statics — a data race waiting to happen

Reading an **immutable** `static` needs no `unsafe`. A **mutable** static does, because
multiple threads reading/writing it concurrently without synchronization is a data race —
and the compiler can't verify single-threaded-only usage across your whole program, so it
requires you to assert it:

```rust
static mut COUNTER: u32 = 0;
fn add_to_counter(inc: u32) {
    // SAFETY: no other threads access COUNTER.
    unsafe { COUNTER += inc; }
}
```

This is sound *only* as long as the "no other threads access it" claim stays true — if this
function (or the static) is later called from a spawned thread, the safety comment's premise
silently breaks and nobody re-verifies it. **Prefer `AtomicU32`/`Mutex<u32>` etc. over a
mutable static** whenever there's any chance of multi-threaded access, now or later — that
moves the safety argument into the type system instead of a comment that can go stale.

## 3. Union fields — no compiler-tracked "active variant"

Unlike an `enum`, a `union` doesn't track which field is currently valid — reading the wrong
field is immediate UB, with no runtime check possible:

```rust
#[repr(C)]
union MyUnion { i: u8, b: bool }
let u = MyUnion { i: 42 };
println!("{}", unsafe { u.i });   // fine, `i` was written
println!("{}", unsafe { u.b });   // UB — `b` was never written, and 42 isn't a valid bool bit pattern
```

Unions exist almost exclusively for C interop (`#[repr(C)]` unions matching a C API). If the
actual goal is "reinterpret these bytes as a different type," prefer `std::mem::transmute` (or
better, a checked/safe wrapper like the `zerocopy` crate) over a hand-rolled union — those
tools at least centralize the safety argument instead of scattering unchecked field reads.

## 4. Unsafe functions and `extern "C"` declarations

Mark your own function `unsafe fn` when it has preconditions the compiler can't check, and
document them in a `# Safety` doc section — this is the contract callers must uphold:

```rust
/// Swaps the values pointed to by the given pointers.
///
/// # Safety
/// The pointers must be valid, properly aligned, and not otherwise accessed
/// for the duration of the function call.
unsafe fn swap(a: *mut u8, b: *mut u8) {
    unsafe { let t = *a; *a = *b; *b = t; }
}
```

**Edition note**: in the 2024 edition, an `unsafe fn`'s body does **not** implicitly act as an
`unsafe` block — you still need an explicit `unsafe { }` inside it for each operation that
needs it (2021 and earlier editions allowed the implicit form; don't rely on it in new code).

**`extern "C"` functions** must each be individually marked `safe` or `unsafe` inside an
`unsafe extern` block, based on whether *that specific function* has preconditions:

```rust
unsafe extern "C" {
    safe fn abs(input: i32) -> i32;              // no pointers, no preconditions — safe to call directly

    /// # Safety
    /// `s` must point to a valid, NUL-terminated, unmodified C string.
    unsafe fn strlen(s: *const c_char) -> usize;  // has preconditions — caller must uphold them
}
```

Don't reflexively mark every FFI declaration `unsafe` — if a specific C function genuinely has
no safety preconditions (takes and returns plain values, no pointers), declaring it `safe`
lets safe Rust call it directly without every call site needing its own `unsafe` block and
safety comment.

## 5. The caller's obligation: every `unsafe` block needs a safety comment

Calling an `unsafe fn` (or writing any `unsafe { }` block) means **you** are now asserting the
preconditions hold — and that assertion needs to be written down, not just true:

```rust
fn log_public_key(pk_ptr: *const u16) {
    // MISSING safety comment — this is the actual bug in this example, not a style nit.
    let pk: &[u16] = unsafe { std::slice::from_raw_parts(pk_ptr, PK_BYTE_LEN) };
    // ^ BUG: `PK_BYTE_LEN` (8) here is being passed as the *element* count, not
    // byte count — `from_raw_parts`'s second argument is always element count.
    // This reads 8 u16s (16 bytes) starting at pk_ptr, running past the 8-byte
    // `pk` field into the adjacent `sk` field. Undefined behavior, not just a
    // logic bug — it silently "works" until it doesn't.
}
```

Two lessons from this exact example, both real recurring mistakes:
- **`slice::from_raw_parts`'s length argument is element count, not byte count.** Confusing
  the two silently reads past the intended buffer.
- **The absence of a safety comment is itself the defect**, independent of whether the code
  happens to be correct — an `unsafe` block with no comment means nobody has actually written
  down (or therefore necessarily checked) why it's sound. Treat a missing safety comment as
  a required-fix in code review, the same as a missing precondition being violated.
- The function *taking* the raw pointer (`log_public_key`) should itself be `unsafe fn` here,
  since `pk_ptr` has preconditions the compiler can't check (validity, sufficient length) —
  wrapping unchecked pointer arithmetic in a plain safe `fn` pushes the safety obligation onto
  every caller silently, with no compiler-enforced signal that they need to think about it.

## Checklist for reviewing/writing unsafe code

1. Does every `unsafe fn` have a `# Safety` doc section stating its preconditions?
2. Does every `unsafe { }` block have a comment (conventionally `// SAFETY: ...`) explaining
   *why*, right now, at this call site, those preconditions are satisfied?
3. Is a mutable `static` actually single-threaded for the life of the program, or would
   `Atomic*`/`Mutex` be safer as the codebase grows?
4. Are raw pointers and references to the *same* memory ever alive at the same time? If so,
   is a write happening through one while a read/reference exists through the other?
5. For `from_raw_parts`/pointer arithmetic: is every length argument actually the *element*
   count it's documented to be, not bytes?
6. Is the unsafe block as small as possible, and is there a safe wrapper around it that
   upholds its preconditions internally so callers never need `unsafe` themselves?
