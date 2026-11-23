---
name: rust-unsafe-soundness
description: Reason rigorously about whether unsafe Rust is sound — derive and document safety preconditions, tell encapsulated vs exposed unsafe apart, spot "crying wolf" unsafe markers, and use MaybeUninit for uninitialized/partially-initialized memory. Use when auditing an unsafe function for soundness, documenting an unsafe function's safety preconditions, or deciding whether a safe wrapper around unsafe code is actually sound for every possible caller input.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/unsafe-deep-dive/{introduction,safety-preconditions,rules-of-the-game,initialization,memory-lifecycle}/**
---

# Rust Unsafe Soundness

`rust-unsafe-fundamentals` covers how to write everyday unsafe correctly. This skill is for
the harder, rarer job: **proving** an unsafe function is sound for every possible input, not
just the inputs you tested — the difference between "it worked when I ran it" and "it cannot
be misused into UB."

## Soundness — the actual definition

> A function is **sound** if it cannot trigger undefined behavior for *any* input that
> satisfies its documented safety preconditions.

The critical, easy-to-miss part: soundness is a claim about **all possible inputs the type
system allows**, not about the inputs your tests happen to exercise. A function can pass every
test you write and still be unsound, if there exists *some* other legal-looking call that
triggers UB. This is why "it works" is not evidence of soundness — you need to reason about
the full space of legal calls, not a sample of them.

The precondition-satisfying side of the contract is the **caller's** job (the human writing
the call site, since the compiler doesn't check preconditions); making sure the function
*behaves* when preconditions are satisfied is the **implementer's** job. A sound function
holds up its end regardless of what the compiler does or doesn't check.

## Three shapes of a function with unsafe inside it — know which one you're looking at

The same "copy bytes from source to dest" logic, in three shapes with very different
soundness properties:

**1. Encapsulated unsafe (safe function, sound)**

```rust
pub fn copy(dest: &mut [u8], source: &[u8]) {
    let len = dest.len().min(source.len());
    let mut i = 0;
    while i < len {
        // SAFETY: `i` < len ≤ source.len(), so in-bounds
        let new = unsafe { source.get_unchecked(i) };
        // SAFETY: `i` < len ≤ dest.len(), so in-bounds
        let old = unsafe { dest.get_unchecked_mut(i) };
        *old = *new;
        i += 1;
    }
}
```

Signature is entirely safe (`&mut [u8]`, `&[u8]`) — **there is no legal call that can violate
the internal unsafe blocks' preconditions**, because `len` is derived from both slices' actual
lengths inside the function itself, not supplied by the caller. This is the target shape:
unsafe internals, safe and foolproof exterior. A safe function containing unsafe blocks is
sound exactly when it's *impossible* for any caller, using only safe Rust, to drive it into
violating those blocks' preconditions.

**2. Exposed unsafe (safe-looking signature, actually unsound)**

```rust
pub fn copy(dest: &mut [u8], source: *const u8) {   // ⚠️ safe fn, raw pointer parameter
    let source = {
        let mut len = 0;
        let mut end = source;
        while unsafe { *end != 0 } { len += 1; end = unsafe { end.add(1) }; }
        unsafe { std::slice::from_raw_parts(source, len + 1) }
    };
    // ...
}
```

**This function is `pub fn` — not `unsafe fn` — but its correctness depends entirely on
`source` being a valid NUL-terminated pointer.** Nothing in its safe-looking signature
prevents a caller from passing a dangling, misaligned, or non-NUL-terminated pointer and
triggering UB with perfectly ordinary, safe-looking calling code. **The bug here is not the
unsafe blocks inside — it's that the function's *signature* makes an unsafely-strong
assumption about a `*const u8` parameter without either (a) being marked `unsafe fn` to push
that responsibility onto the caller, or (b) validating the assumption before trusting it.**

**The critical lesson**: a function passing your tests proves nothing about soundness — this
exact function works fine for well-formed inputs and is UB for a whole class of others. When
reviewing any function that touches raw pointers, unions, or manual memory management,
check first whether its **signature** (not just its body) is honest about what it requires.
If a plain, safe-looking `fn` takes a raw pointer, a length, an index, or anything else whose
validity can't be checked by the type system, that's a signal the function should either be
`unsafe fn` with a `# Safety` doc section, or should validate the input at runtime before
trusting it.

**3. "Crying wolf" (marked unsafe, but has no real preconditions)**

```rust
pub unsafe fn copy(dest: &mut [u8], source: &[u8]) {   // unsafe fn — but why?
    for (dest, src) in dest.iter_mut().zip(source) { *dest = *src; }
}
```

Marked `unsafe fn`, but the body is 100% safe Rust operating on safe references — there is
**no precondition a caller needs to uphold** beyond what the type system already guarantees.
This is a "crying wolf" function: it makes every caller write an `unsafe { }` block and a
safety comment justifying... nothing. This has a real cost beyond ceremony: it trains callers
(and reviewers) to treat `unsafe` blocks as boilerplate rather than a genuine signal requiring
scrutiny, which erodes the value of `unsafe` everywhere else in the codebase. If a function's
body doesn't actually need any of the five unsafe capabilities (see `rust-unsafe-fundamentals`)
to do its job, don't mark it `unsafe` — that marking should be reserved for functions with a
real, documented, caller-checkable precondition.

## Deriving and documenting safety preconditions

Common categories of precondition to check for when writing or auditing a `# Safety` section
(not exhaustive, but covers most real unsafe code):

| Category | What to check |
|---|---|
| **Validity** | Is every bit pattern present actually a legal value of the type? (e.g. a `bool` must be `0` or `1`, a reference must never be null) |
| **Alignment** | Does every pointer dereference respect the target type's required alignment? |
| **Aliasing** | If manually constructing `&mut T` from a pointer, is it provably the *only* live reference to that memory — no other `&T`/`&mut T` alive concurrently? |
| **Initialization** | Is every byte that will be read actually initialized before the read? (see `MaybeUninit` below) |
| **In-bounds** | Does every index/pointer offset stay within the bounds of the allocation it was derived from? |
| **Lifetimes** | Does a reference or pointer ever get used after the memory it points to could have been freed/moved? |
| **Pointer provenance** | Is a raw pointer actually derived from a valid allocation, not synthesized from an arbitrary integer? (`usize as *const T` from an arbitrary integer is not generally sound) |

**User-defined types get to declare their own preconditions**, beyond this generic list — if
your type has an invariant unsafe code elsewhere relies on (e.g. "this `Vec` field is always
sorted," "this handle is always paired with an open resource"), document it in a `# Safety`
section the same way, so future maintainers of *that type* know what they must preserve.

**Writing the doc**: every `unsafe fn` needs a `# Safety` section stating its preconditions in
terms the caller can actually check at their call site (not vague — "the pointer must be
valid" is weaker than "the pointer must be non-null, aligned for `T`, and point to a
live allocation of at least `len` elements for the duration of the call"). Every `unsafe { }`
block calling into it needs a `// SAFETY: ...` comment explaining why, at *this specific call
site*, those preconditions hold.

## `MaybeUninit<T>` — the sound way to handle uninitialized memory

Safe Rust cannot refer to potentially-uninitialized data — every `T` is assumed fully valid
and initialized. But data has to start out uninitialized somewhere (a fresh buffer, memory
just received from an allocator or an external source). `MaybeUninit<T>` is the type-system
bridge: think of it as `Option<T>` where the "`None`" state is "uninitialized memory, safe
only to *write*, never to read."

**Common use case — partial initialization without paying to zero a buffer you'll overwrite
anyway**:

```rust
let mut buf = [const { MaybeUninit::<u8>::uninit() }; 2048];   // no zeroing cost
let len = external_data.len().min(buf.len());   // `zip` below stops at buf.len() bytes —
                                                 // `len` must match, or the SAFETY claim is false
for (dest, src) in buf.iter_mut().zip(external_data) {
    dest.write(*src);                     // write() is always safe on MaybeUninit
}
// SAFETY: we just initialized exactly the first `len` bytes of `buf` above.
let init: &[u8] = unsafe {
    std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), len)
};
```

Rules to hold onto:
- `MaybeUninit::write()` is always safe — writing to uninitialized memory can't be UB.
- **Reading is where the danger is** — you must independently track (in your own code, the
  compiler won't) exactly which bytes have actually been written, and never construct a
  reference or slice that includes any byte that hasn't been. Reading uninitialized memory as
  a concrete type (not through `MaybeUninit`) is UB even if you never observe an obviously
  "wrong" value.
- Prefer `ptr::write` (or `MaybeUninit::write`) over plain assignment (`*ptr = value`) when
  writing into memory that might be uninitialized — plain assignment runs the old value's
  `Drop` first, which is UB if there was no valid old value to drop.

## Checklist when auditing an unsafe function

1. **Read the signature, not just the body.** Does every safe-typed parameter actually carry
   enough information for the type system to guarantee what the body assumes? If it takes a
   raw pointer/index/length and isn't itself `unsafe fn`, where does that assumption get
   validated?
2. **Is `unsafe fn`/`unsafe { }` present exactly where a real, non-compiler-checkable
   precondition exists** — not more (crying wolf), not less (exposed unsafe)?
3. Walk the precondition categories table above against the actual pointer/memory operations
   in the body.
4. If the function passes its tests, explicitly ask: **"is there an input the type signature
   permits that isn't covered by my tests, and would it violate a precondition?"** — this is
   the question "it works" doesn't answer.
5. For any `MaybeUninit` usage: can you point to exactly which bytes are known-initialized at
   the moment of every read, and why?
