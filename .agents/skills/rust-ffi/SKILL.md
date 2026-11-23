---
name: rust-ffi
description: Call C or C++ from Rust (or vice versa) correctly — string/representation mismatches, error-handling conventions, ownership across the FFI boundary, and choosing between bindgen (C) and cxx (C++). Use when writing an extern "C" binding, wrapping a C/C++ library, converting between C strings and Rust &str, or reviewing an FFI boundary for soundness.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/unsafe-deep-dive/ffi/**,
  src/android/interoperability/{with-c,cpp}/** (platform-neutral parts only)
---

# Rust FFI

For the mechanics of `unsafe extern "C"` declarations and calling unsafe functions in general,
see `rust-unsafe-fundamentals`. This skill is about the FFI boundary itself: what differs
between Rust and C/C++, and how to bridge it soundly rather than just make it compile.

## Interop strategy: go through the C ABI, not directly

Rust and C++ (or any two languages) generally **cannot** share data structures and call each
other's functions directly — their type layouts, calling conventions, and runtime models
don't agree. The practical path both directions go through is the **C ABI** as a lowest common
denominator:

```text
Rust  <----->  C ABI  <----->  C++
```

This is why C interop (raw `extern "C"`, or the `bindgen` tool) is comparatively
straightforward, while C++ interop needs a bridging layer (the `cxx` crate, or hand-written
`extern "C"` shims) to translate C++'s richer type system down to something C-ABI-compatible
and back up on the other side. (Fully automatic high-fidelity interop across the *entire* type
system — e.g. Crubit, Zngur — exists but is experimental; don't expect it to eliminate this
boundary today.)

## Rust ↔ C: what actually differs

| Concern | Rust | C |
|---|---|---|
| Errors | `Result<T, E>`, `Option<T>` | Sentinel return values, out-parameters, global `errno` |
| Strings | `&str`/`String` — UTF-8, length carried alongside the pointer | `char*` — NUL-terminated, encoding unspecified |
| Nullability | Explicit: absence is `Option<T>`, a plain `&T` is never null | Any pointer can be null; nothing in the type says so |
| Ownership | Enforced by the type system (affine types + lifetimes) | Convention only — comments, naming, documentation |
| Callbacks | `Fn`/`FnMut`/`FnOnce` closures (capture state automatically) | Function pointer + a separate `void* userdata` you thread manually |
| Panics/unwinding | Stack unwinding (or abort, depending on panic strategy) | No concept of unwinding — a Rust panic crossing into C is UB |

Practical consequences at the boundary:
- **Every pointer arriving from C must be checked before use** — wrap it as `Option<NonNull<T>>`
  (or explicitly null-check) before dereferencing; there is no way around this cost, since C
  gives you no compile-time guarantee.
- **Never let a Rust panic unwind across an `extern "C"` boundary** — catch it first
  (`std::panic::catch_unwind`) and convert to whatever error convention the C side expects
  (sentinel value, out-parameter). An uncaught panic crossing into C is undefined behavior.
- **Ownership must be documented and enforced by convention on the C side** — decide, and
  clearly document, which side is responsible for freeing any pointer that crosses the
  boundary, and never let both sides believe they own it (double-free) or neither (leak).
- **Callbacks need manual `userdata` threading** — a Rust closure captures its environment
  automatically; a C function pointer can't, so you pass a raw `void*`/`*mut c_void` alongside
  the function pointer and cast it back on the Rust side. Getting this cast wrong is a common,
  hard-to-diagnose source of UB.

## String representations differ — never assume compatibility

Three different "text" representations you'll meet at an FFI boundary, and how to bridge each
correctly:

```rust
let c_repr    = b"Hello, C\0";           // C: NUL-terminated, no separate length
let cc_repr   = (b"Hello, C++\0", 10u32); // C++: often pointer + explicit length
let rust_repr = (b"Hello, Rust", 11);     // Rust: pointer + length, no NUL required
```

```rust
// C → Rust: find the NUL, validate UTF-8 (or accept it may not be valid)
let c: &str = unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap() };

// Rust → C: Rust strings aren't NUL-terminated by default — use CString/CStr,
// or the `c"..."` literal (Rust 1.77+, any edition) which appends the NUL for you:
// c"Rust" == b"Rust\0"
```

Checklist:
- Never assume a Rust `&str`/`String` is safe to hand to C as a `char*` without going through
  `CString`/`CStr` — Rust strings can contain interior NUL bytes (silently truncating on the C
  side) and aren't NUL-terminated.
- Never assume a C `char*` is valid UTF-8 — `CStr::to_str()` returns `Result`; don't
  `.unwrap()` it in code that must handle arbitrary/untrusted C strings.
- A "string" from C++ might carry an explicit length instead of being NUL-terminated (e.g. a
  `std::string_view`-shaped ABI) — check which convention the specific API actually uses
  rather than assuming C's NUL-termination applies everywhere.

## Semantic mismatches beyond representation

Some C APIs have semantics that don't map onto anything expressible in Rust's type system at
all — not just "needs a manual check," but genuinely inexpressible:

```rust
unsafe extern "C" {
    fn ctime(t: *const libc::time_t) -> *const c_char;   // returns a pointer into a
}                                                          // buffer *shared and reused
                                                            // across calls
```

`ctime`'s returned buffer is overwritten by the *next* call to `ctime` from *any* caller —
neither `'static` (it's not permanently valid) nor a normal borrowed lifetime `'a` (it doesn't
end when this call's stack frame does; it ends whenever some other call to `ctime` happens,
which Rust's lifetime system has no way to model) correctly describes it. When you hit an API
shaped like this: copy the data out (e.g. `.to_str().unwrap().to_string()`) immediately after
the call and before any other call that could invalidate the shared buffer, rather than trying
to hold a reference to it — the API's own contract, not Rust, is what makes any borrow of it
unsound past the very next call.

## Choosing a binding tool

| Tool | For | What it does |
|---|---|---|
| Hand-written `unsafe extern "C"` | Small, stable C APIs; full control needed | You declare each signature yourself — see `rust-unsafe-fundamentals` for `safe`/`unsafe` per-function marking |
| `bindgen` | Larger/evolving C headers | Auto-generates raw Rust bindings from a `.h` file. **The generated bindings are themselves unsafe and ergonomically raw** — the course's explicit advice: wrap `bindgen`'s output in your own safe Rust module/API rather than exposing it directly to callers. Don't skip this wrapping step just because generation was automatic. |
| `cxx` crate | C++ interop specifically | You declare shared signatures/types once in a `#[cxx::bridge]` module (an `extern "Rust"` block and an `extern "C++"` block); `cxx` generates matching, checked bindings on both sides, plus a real (checked, not just documented) contract for which types can safely cross the boundary. Use `cargo expand ::ffi` to see the generated Rust glue, and `target/cxxbridge` for the generated C++ glue when debugging a mismatch. |

**Don't hand-write C++ FFI bindings** the way you might for C — C++'s name mangling, templates,
exceptions, and non-C-ABI-stable class layout make that much more failure-prone than C; use
`cxx` (or an equivalent bridging crate) rather than trying to replicate its safety guarantees
by hand.

## Checklist for reviewing an FFI boundary

1. Every pointer entering from the foreign side is null-checked / wrapped in `Option`/`NonNull`
   before dereferencing.
2. Every panic that could originate in code reachable from an `extern "C"` entry point is
   caught (`catch_unwind`) and converted, not allowed to unwind across the boundary.
3. String conversions go through `CStr`/`CString` explicitly in both directions — no
   implicit "this pointer is basically a string" assumptions.
4. Ownership of every pointer/handle crossing the boundary is documented: which side frees it,
   and when.
5. Any C API returning a pointer into a shared/reused internal buffer (like `ctime`) has its
   data copied out immediately, not held as a borrow.
6. `bindgen` output is wrapped in a hand-written safe API before being exposed to the rest of
   the codebase, not called directly everywhere it's needed.
