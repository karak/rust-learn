---
name: rust-polymorphism
description: Decide between generics and dyn Trait, understand why Rust has no inheritance, and restrict who can implement a trait (sealed traits, enums). Use when converting OOP-style inheritance designs to Rust, choosing static vs dynamic dispatch, hitting an orphan-rule error, downcasting a trait object, or deciding whether an API's extension points should be public.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/idiomatic/polymorphism/**,
  src/idiomatic/leveraging-the-type-system/extension-traits/**
---

# Rust Polymorphism

`rust-patterns` covers "accept generics, return concrete types" and trait objects briefly.
This skill goes deeper on the actual decisions: generics vs `dyn`, why/how to port an
inheritance-shaped design, and controlling who can extend a trait.

## The orphan rule (why you can't `impl` any trait for any type)

Rust forbids implementing a trait for a type when **neither** the trait nor the type is
"local" to your crate — otherwise two crates could each define a conflicting impl of the same
foreign trait for the same foreign type, and the whole ecosystem would have no way to resolve
which one applies.

```rust
// crate `mycoolnewdb`, depends on `database-traits` (defines DbConnection)
// and `postgresql-bindings` (defines PostgresqlConn) — neither is local here.
impl DbConnection for PostgresqlConn {}   // ❌ orphan rule violation
```

If you hit `error[E0117]: only traits defined in the current crate can be implemented for
types defined outside of the crate`, the fix is one of:
- Define a **newtype wrapper** around the foreign type in your crate (now the type is local)
  and implement the foreign trait for the wrapper instead.
- Define your **own trait** (now the trait is local) with the behavior you need, and implement
  it for the foreign type directly.
- If you control one of the two crates, move the trait or type there instead.

## Generics vs `dyn Trait` — decision guide

Both are ways to write one function/type that works across multiple concrete types, but with
different cost profiles:

| | Generics (`impl Trait` / `<T: Trait>`) | `dyn Trait` |
|---|---|---|
| Dispatch | Static — resolved at compile time | Dynamic — vtable lookup at runtime |
| Codegen | **Monomorphization**: a separate compiled copy per concrete type used | One shared copy in the binary |
| Binary size / compile time | Grows with number of instantiations — "pay for what you use," but can add up with many generic call sites | Fixed — one copy regardless of how many concrete types implement the trait |
| Runtime cost | Free after compilation — same as hand-writing each version | Vtable indirection per call, plus size overhead of the wide pointer |
| Value shapes allowed | All values must be the **same concrete type** at each call site (`T` is one type) | Can hold **heterogeneous** types behind the same interface (`Vec<Box<dyn Trait>>` with mixed concrete types) |
| Memory layout | `&T`/`T` — normal thin pointer or inline value | `&dyn Trait`/`Box<dyn Trait>` — a **wide pointer**: data pointer + vtable pointer, so `size_of::<&dyn Trait>()` is 2 words even though `size_of::<&i32>()` is 1 |

Default to generics; reach for `dyn Trait` specifically when you need a **heterogeneous
collection** (a `Vec` of different concrete types behind one interface) or need to avoid
per-type code bloat in a context where binary size matters more than the dispatch cost
(e.g. embedded or WASM targets, where monomorphization's binary-size cost is felt directly).
When compile time or binary size from monomorphization becomes a real, measured problem,
`dyn Trait` at a few well-chosen boundaries (not everywhere) is the standard fix.

**`Sized` / `?Sized`**: most types have a compile-time-known size (`Sized`, auto-implemented).
`dyn Trait`, `[T]`, and `str` are **dynamically sized** — their size is only known at runtime
and is carried alongside the pointer (hence the wide pointer above). This is why `dyn Trait`
can only be used behind a reference or pointer (`&dyn Trait`, `Box<dyn Trait>`), never as a
plain by-value type or generic parameter without `?Sized`.

### Limits and pitfalls of `dyn Trait`

- **Downcasting requires `Any`**: to get a concrete type back out of a `dyn Trait`, the trait
  needs `Any` as a supertrait (`trait Foo: Any {}`), and you still cast through `dyn Any`
  explicitly (`(value as &dyn Any).downcast_ref::<Concrete>()`), not through `dyn Foo`
  directly. That `as &dyn Any` cast relies on trait upcasting coercion, stable since Rust 1.86
  — on older toolchains, add `fn as_any(&self) -> &dyn Any { self }` to the trait instead and
  call that.
- **Pitfall: reaching for `dyn Trait` too early.** Coming from OOP, it's tempting to make
  everything dynamically dispatched by default. This has a real cost — every call pays vtable
  indirection, and worse, patterns like "downcast to compare/combine two trait objects of
  possibly-different concrete types" (an `AddDyn` trait with `fn add_dyn(&self, rhs: &dyn
  AddDyn)`, downcasting `rhs` before you can even add two numbers) show how far the fluency
  the compiler normally gives you can erode. If two values need to interact and you find
  yourself downcasting to make that interaction work at all, that's a strong signal the
  values should have been generic/concrete rather than trait objects in the first place.
- Trait objects are **not zero-cost** the way generics are: expect a vtable call per method
  invocation and the wide-pointer memory overhead above, on top of whatever the method
  actually does.

## Porting an inheritance-shaped design

Rust has no `struct Data: Base` inheritance. Use **composition** — a field of the base type,
plus separate `impl` blocks for the derived type's own methods and for any traits:

```rust
// Not valid Rust:
// struct Data: Id { name: String }

// Idiomatic:
struct Data {
    id: Id,        // composed, not inherited
    name: String,
}
impl Data { /* Data's own methods */ }
impl SomeTrait for Data { /* trait impls kept separate, explicit */ }
```

**Why Rust deliberately omits inheritance**: class inheritance makes subtypes implicitly
interchangeable with their base (heterogeneous by default), which causes real problems for
things like equality/comparison across the hierarchy; and it splits "what makes up a type and
how it behaves" across a hierarchy of parent/child definitions, making both fields and
effective method behavior (which override wins) hard to determine by reading one place.
Composition keeps every field and every trait impl visible and local to the type that has it.

**Trade-off to know about**: composition costs field-access ergonomics — `data.id.value`
instead of `data.value` — there's no automatic "inherited field" promotion. This is
deliberate; don't try to paper over it with `Deref` to the inner type purely for ergonomics
(that's a well-known anti-pattern — `Deref` should model "is essentially a" for smart-pointer-
like types, not be used to fake field inheritance).

**Deriving traits on a composed type**: derive macros (`#[derive(Clone)]`, etc.) require
*every* field's type (or every variant's types, for enums) to already implement that trait —
composing in a field whose type doesn't implement the trait you're trying to derive will fail
to compile, with a somewhat oblique error pointing at the field, not the struct.

## Restricting who can implement a trait

Two techniques for "this API is built around a fixed, known set of implementors — don't let
downstream crates add more":

**Sealed traits** — hide the real trait behind a supertrait defined in a private module:

```rust
mod sealed {
    pub trait Sealed {}
    impl Sealed for String {}
    impl Sealed for Vec<u8> {}
}
pub trait ApiTrait: sealed::Sealed { /* public methods */ }
impl ApiTrait for String {}
impl ApiTrait for Vec<u8> {}
```

Downstream crates can *use* `ApiTrait` (call its methods, take `impl ApiTrait` bounds) but
cannot `impl ApiTrait for TheirType`, because they cannot name — let alone implement —
`sealed::Sealed`. Use this when the trait's contract is either still evolving (you want the
freedom to add methods without it being a breaking change for external implementors) or is
high-risk to implement incorrectly (e.g. cryptographic primitives, safety-critical protocol
state).

**Sealing with an enum** — when the actual polymorphism need is "one of a small, fixed set of
variants," skip traits/dispatch entirely:

```rust
pub enum GetSource {
    WebUrl(String),
    BytesMap(BTreeMap<String, Vec<u8>>),
}
impl GetSource {
    fn get(&self, url: &str) -> Option<&Vec<u8>> {
        match self { Self::WebUrl(s) => ..., Self::BytesMap(m) => m.get(url) }
    }
}
```

Compared to a sealed trait: an enum makes the valid set of "types" **explicit and enumerable**
at every call site (users construct a specific variant, and `match` makes the exhaustive set
visible), at the cost of every addition to the set being a breaking change to the enum's
definition (new variant = downstream `match`es without a wildcard arm stop compiling — which
is often exactly the point, if you want callers to consciously handle new cases). Prefer an
enum over a sealed trait when the set of "shapes" really is fixed and small, and callers
benefit from exhaustiveness checking; prefer a sealed trait when you want room to add
implementations from *inside* your own crate over time without every internal call site
needing a new `match` arm.

## Extension traits — adding methods to types you don't own

The orphan rule (above) blocks `impl ForeignTrait for ForeignType`, but you can still attach
new methods to a foreign type by defining your **own** trait and implementing it for that
type — this is legal because the trait is local, even though the type isn't:

```rust
mod ext {
    pub trait StrExt {
        fn is_palindrome(&self) -> bool;
    }
    impl StrExt for str {
        fn is_palindrome(&self) -> bool { self.chars().eq(self.chars().rev()) }
    }
}
use ext::StrExt as _;   // must be in scope to call the method
assert!("dad".is_palindrome());
```

Conventions and constraints:
- Name it `<Type>Ext` by convention, signaling it's meant for extension, not as an interface
  others implement.
- The impl must live in the **same crate** as the trait definition (orphan rule still applies
  to the impl itself — only the trait being local makes this legal at all).
- The trait must be **imported into scope** wherever its methods are called — `use
  ext::StrExt as _;` is enough (the `as _` avoids a name conflict if you never refer to the
  trait by name).

**Extension trait vs. free function** — prefer the extension trait when:
- **Discoverability** matters: IDEs/`rust-analyzer` suggest extension methods after typing
  `.` on a value of the foreign type; a free function `is_palindrome(s)` isn't discovered that
  way.
- **Method chaining** matters: `data.iter().filter(...).map(...)` reads left-to-right; the
  free-function equivalent nests inside-out (`map(filter(iter(data), ...), ...)`). This is why
  `Iterator`'s combinators are trait methods, not free functions.
- The behavior needs to be usable as a **generic bound** or through `dyn Trait` — only traits
  can appear in `T: SomeTrait` or `dyn SomeTrait`; a free function can't.

Prefer a free function when there's no foreign type to attach to (it's your own type, or
there's no natural receiver), or when the extra trait-definition ceremony isn't buying any of
the above.

**Name-conflict resolution** — two situations to know how Rust resolves, because they produce
counter-intuitive-looking results if you haven't seen them before:
- **Extension method vs. inherent method, same name**: if the foreign type later gains an
  *inherent* method with the same name your extension trait already defined, **the inherent
  method always wins** — Rust's method resolution prioritizes inherent methods over trait
  methods unconditionally, silently changing which implementation your existing call sites
  run. This is a real hazard when extending a type from an external crate that might add
  methods in a future version — there's no compiler warning when this shadowing happens.
- **Two extension traits, same method name, both in scope**: this is a **compile error**
  (ambiguous method call) — the compiler will not silently pick one. You must disambiguate
  with fully-qualified syntax: `Ext1::is_palindrome(&s)` instead of `s.is_palindrome()`.

## Quick decision checklist

1. Modeling "is-a" from an OOP design? → compose a field, don't look for inheritance syntax.
2. Need one function over many types, all the same concrete type per call? → generic.
3. Need a collection of genuinely different concrete types behind one interface? → `dyn Trait`,
   accept the vtable/wide-pointer cost.
4. About to downcast a `dyn Trait` to make two trait objects interact? → stop, reconsider
   whether generics/concrete types were the right tool from the start.
5. Trait implementable by external crates only in ways you don't want? → seal it (module trick)
   or replace it with an enum if the valid set is small and fixed.
