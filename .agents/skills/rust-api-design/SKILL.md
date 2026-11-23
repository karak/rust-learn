---
name: rust-api-design
description: Design predictable Rust public APIs — method naming conventions, doc comment structure, and which std traits to implement. Use when writing a library crate's public surface, reviewing API ergonomics, naming a constructor/conversion/accessor method, or deciding whether a type needs Debug/Display/Eq/Ord/Hash/Clone/Copy/From/TryFrom.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/idiomatic/foundations-api-design/**
---

# Rust API Design

For general idiom defaults, see `rust-patterns` first. This skill is for the specific job of
designing or reviewing a **public API surface**: what to name things, what to document, and
which standard traits a type should implement.

## Doc comments: what, not how or where

Doc comments describe the **contract** a caller can rely on — not the implementation, and not
where the API is currently used. Both leak details that change without the doc being updated.

```rust
// Bad — documents implementation, which is irrelevant and goes stale fast
/// Saves a `User` record to the Postgres database.
///
/// This function opens a new connection and begins a transaction. It checks
/// if a user with the given ID exists with a `SELECT` query...
pub fn save_user(user: &User) -> Result<(), db::Error> { ... }

// Good — documents the guarantee
/// Atomically saves a user record.
///
/// # Errors
///
/// Returns `db::Error::DuplicateUsername` if the user (keyed by
/// `user.username`) already exists.
pub fn save_user(user: &User) -> Result<(), db::Error> { ... }
```

If implementation details genuinely matter to the caller, it's because of an effect or
invariant they need to know about (e.g. "blocks the calling thread," "not safe for
concurrent calls") — document *that*, not the mechanism producing it.

**Avoid redundancy** — the item's name and signature already are documentation. Don't restate
them:

```rust
// Redundant — the signature already says this
/// Parses an ipv4 from a str. Returns an option for failure modes.
fn parse_ip_addr_v4(input: &str) -> Option<IpAddrV4> { ... }

// Redundant — the field name already says this
struct BusinessAsset {
    /// The customer id.
    customer_id: u64,
}

// Don't lead with the type/function's own name either
/// `ServerSynchronizer` is an orchestrator that sends local edits...
struct ServerSynchronizer { ... }
// Better: state the purpose directly
/// Sends local edits to the server and reconciles conflicts.
struct ServerSynchronizer { ... }
```

Rule of thumb: ask "what does the reader still not know after reading the name and type
signature?" — document only that gap. Don't explain `Result`, the `?` operator, or other
language/stdlib basics; assume an intermediate Rust reader.

**Name-drop and signpost, don't over-explain.** Readers skim; put the load-bearing keyword
near the start of a sentence so scanning finds it, and link out (`[term][anchor]`) to let an
unfamiliar reader self-serve rather than inlining a tutorial:

```rust
/// A parsed representation of a [MARC 21 record leader][leader].
///
/// [leader]: https://www.loc.gov/marc/bibliographic/bdleader.html
pub struct Leader { ... }
```

**Library vs. application docs are different jobs.** Library code has many users, a broad
problem scope, and a stable API — it can afford elaborate docs, including some restated
signature info, because the ROI is high. Application code has few users, a narrow purpose,
and churns often — keep its docs terse; heavy doc-coverage lints (`#![warn(missing_docs)]`)
are appropriate for library crates, not generally for application code.

## Naming conventions

Rust doesn't have a `new` keyword — naming *is* the API's grammar. Getting these conventions
right is what makes an API guessable without reading docs.

| Prefix/suffix | Meaning | Example |
|---|---|---|
| `new` | The canonical constructor | `Vec::new()`, `Box::new(x)` |
| `with_capacity`, `with_*` (constructor) | Constructor that sets one specific parameter, defaults elsewhere | `Vec::with_capacity(n)` |
| `with_*` (copy-and-set) | Non-consuming: copy self, change one thing, return new value — "like `self` but with X different" | `path.with_extension("mov")` |
| `with(closure)` | Takes a closure to configure something before consuming/returning | builder-style config methods |
| `is_*` | Boolean predicate about the value | `Vec::is_empty()`, `f32::is_nan()` |
| `[method]_mut` | Same access as `[method]` but returns `&mut` | `get` / `get_mut`, `iter` / `iter_mut` |
| `try_*` | Fallible version, returns `Result` with a **specific** error type | `TryFrom::try_from`, `Receiver::try_recv` |
| `from` | Constructor implying a type **conversion** — prefer over hand-rolled `into` (see below) | `Duration::from_days(n)` |
| `into_*` | Consumes `self`, returns another owned type | `IntoIterator::into_iter`, `Box<str>::into_string` |
| `to_*` | Non-consuming conversion, borrows `self` and returns an owned value (or takes `self` by value if `Copy`) | `str::to_owned()`, `str::to_uppercase()` |
| `as_*` / `_ref` | Cheap reference-to-reference reinterpretation, no allocation | `Rc::as_ref`, `Option::as_ref`, `Option::as_slice` |
| `by` | Method takes a custom projection/comparator closure | `slice::sort_by`, `sort_by_key` |

**`From`/`Into` — prefer implementing `From<T>` on your type, not `Into<Yours>` for `T`.**
A `From` impl gives you the corresponding `Into` for free (blanket impl), but not the reverse.
For function parameters, prefer `impl Into<String>` as the bound over `where String: From<T>`
— it reads more clearly at the call site as "accepts anything convertible into a String."
Same rule for the fallible pair: implement `TryFrom<T>`, not `TryInto`.

**Getting these names wrong has a real cost**: an `into_foo` that borrows instead of
consuming, or a `to_foo` that's actually expensive/allocating on a type callers expect to be
free, breaks the implicit contract these prefixes carry and will surprise every caller who's
used the standard library.

## Choosing which traits to implement

Default to only what the type's actual usage needs — implementing a trait is a promise about
behavior, not a checkbox.

| Trait | Derivable? | Implement when… | Watch out for |
|---|---|---|---|
| `Debug` | ✅ | Almost always — needed for `{:?}`, `dbg!`, assertions in tests. | — |
| `Display` | ❌ (hand-write) | The type has a genuine user-facing string form. | Don't implement just to reuse `{}` in logs — that's what `Debug`/`{:?}` is for. |
| `PartialEq` | ✅ | Value has a meaningful equality notion. | "Partial" means some values may not compare as expected (e.g. `f32::NAN != f32::NAN`) — not that it panics. |
| `Eq` | ✅ | Equality is *total* — every value equals itself, no `NaN`-like exceptions. Requires `PartialEq`. | Can't manually diverge: deriving both keeps them consistent. |
| `PartialOrd`/`Ord` | ✅ | Values have a meaningful order, e.g. for sorting or `BTreeMap` keys. | Derived order follows declaration order — struct fields top-to-bottom, enum variants top-to-bottom. `Ord` requires `Eq` + `PartialOrd`; keep both consistent. |
| `Hash` | ✅ | Type is used as a `HashMap`/`HashSet` key. Requires `Eq`. | `Hash` and `Eq` must agree: if `a == b` then `hash(a) == hash(b)` — don't hand-write one without the other, or derive both. |
| `Clone` | ✅ | Duplicating the value (or bumping an `Rc`/`Arc` refcount) is a valid operation. | Skip it if duplicating would violate an invariant the type exists to enforce (unique handles, single-owner resources — see `rust-newtype-and-raii`). |
| `Copy` | ✅ | Type is small "plain data" that should behave like a primitive (numeric wrappers, small enums). | Requires `Clone`; **always derive both together**, never hand-write `Clone` alongside a derived `Copy` — a custom `clone()` could diverge from what an implicit bitwise copy does. Impossible on types with `Drop` or non-`Copy` fields — that's a hint the type is not "plain data." |
| `From<T>` | ❌ | An infallible, non-panicking conversion from `T` exists. | Implement `From`, not `Into` (see above). |
| `TryFrom<T>` | ❌ | The conversion from `T` can fail. | Give it a specific, meaningful `Error` type — not `()` or a generic string. |
| `Serialize`/`Deserialize` (serde) | via `#[derive(Serialize, Deserialize)]` | Type crosses a serialization boundary (config, wire format, storage). | Changing field names/order is a wire-format break once this is derived — treat it as part of the public contract. |

## Quick checklist for a new public item

1. Name it using the table above — if it converts, copies, checks a condition, or borrows
   mutably, there's almost certainly a convention that already fits.
2. Write the doc comment as *contract*: what's guaranteed, what errors mean, link out to
   unfamiliar terms — not how it's implemented or where it's called from.
3. Derive `Debug` unless there's a specific reason not to (e.g. it would leak secrets — hand-
   write a redacting `Debug` instead).
4. Walk the traits table and implement only what the type's real usage requires; don't derive
   `Clone`/`Copy` reflexively on a type meant to represent a unique resource or handle.
5. If it's library code with many callers, invest in doc quality; if it's application code,
   keep docs terse and expect them to churn with the code.
