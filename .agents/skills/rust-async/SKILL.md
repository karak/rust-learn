---
name: rust-async
description: Debug Rust async/await bugs — an executor that silently runs futures sequentially, a hang at an await point, a cancelled future losing state, or an async trait that won't compile with dyn. Use when working with tokio, futures, join!/select!, or diagnosing why "concurrent" async code isn't actually concurrent.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/concurrency/{async,async-control-flow,async-pitfalls,async-exercises}/**
---

# Rust Async

`rust-patterns` has a brief Tokio snippet for the default idiom. This skill is for the actual
pitfalls that produce real bugs in async code — lead with those, since they're why this skill
exists separately from `rust-concurrency-sync` (OS threads).

## Mental model (needed to make sense of every pitfall below)

- An `async fn`/block compiles to an anonymous type implementing `Future`, holding all its
  local state (including references between its own locals across `.await` points).
- **Futures are inert.** Unlike a JS `Promise`, nothing happens — not even a timer starting —
  until an **executor** polls the future. Constructing a future and dropping it without
  `.await`ing or spawning it does nothing.
- A **task** is a top-level future the executor schedules; tasks are cooperatively scheduled
  onto a pool of OS threads and are *not* 1:1 with threads — many tasks share one thread.
  Concurrency within a single task happens by polling multiple nested futures (e.g. via
  `select!`), corresponding loosely to nested calls.
- Rust has no built-in runtime — you pick one (Tokio is the ecosystem default; `smol` for
  lightweight use). A **runtime** = an executor (runs futures) + a reactor (drives I/O).

## Pitfall 1: blocking the executor (silently serializes "concurrent" work)

Most executors run tasks cooperatively on a limited thread pool. Any task that blocks its
thread — a CPU-bound loop, or a **synchronous** blocking call — prevents the executor from
polling every other task on that thread, defeating the whole point of `.await`ing them
concurrently.

```rust
// BUG: std::thread::sleep blocks the OS thread; join_all "runs" 10 sleeps
// one after another instead of concurrently.
async fn sleep_ms(_id: u64, duration_ms: u64) {
    std::thread::sleep(Duration::from_millis(duration_ms));  // ← blocking!
}
join_all((1..=10).map(|t| sleep_ms(t, t * 10))).await;
```

This bug is easiest to spot with the `current_thread` Tokio flavor (single OS thread — the
serialization becomes obvious in timing), but **the bug exists in multi-threaded runtimes
too**, just less visibly (it starves whichever other tasks land on the same worker thread).

Fixes, in order of preference:
1. Use the async equivalent (`tokio::time::sleep(...).await` instead of `std::thread::sleep`,
   async file/network I/O instead of sync).
2. If the blocking call is unavoidable (a sync library, genuine CPU-bound work), move it off
   the executor with `tokio::task::spawn_blocking(...)`, which runs it on a dedicated blocking
   thread pool and hands back a future you can `.await`.

## Pitfall 2: `Pin` — why a future "cannot be moved" won't compile

An async block's generated `Future` type can contain **self-references** — a local variable
holding a pointer to another local in the same block, valid across an `.await` (e.g. a
borrowed slice into a buffer that's also a field of the same generated struct). If that
struct were moved to a new memory address, the internal pointer would now point at the *old*
address — dangling.

To prevent this, futures are only ever polled through a **pinned** pointer: `Pin<&mut Self>`
in `Future::poll`. `Pin<P>` wraps a pointer and refuses operations that would move the
pointee, once it's pinned.

Practical implications:
- You will hit this mainly as a **compile error** when trying to move a value out of a
  `Pin<&mut T>`, store a manually-constructed future in a struct field and move that struct,
  or hold a `&mut` future across code that also tries to relocate it.
- Common triggers: hand-rolling a `Future` impl instead of using `async fn`/blocks; storing
  a boxed future (`Pin<Box<dyn Future<...>>>`) and then moving the containing struct without
  going through the `Pin` API; passing a future by value into something expecting `Unpin`.
- Most application-level async code never needs to think about `Pin` directly — it only
  surfaces when writing custom combinators, executors, or self-referential structures. If you
  hit it unexpectedly in ordinary `async fn` code, the more common root cause is trying to
  manually implement `Future` or store a `dyn Future` without boxing/pinning it correctly, not
  a fundamental need for unsafe pinning. For the general (non-async) unsafe mechanics of `Pin`
  — `Unpin`, `PhantomPinned`, self-referential structs — see `rust-pinning`.

## Pitfall 3: cancellation — dropping a future loses everything past its last completed `await`

**Dropping a future *is* cancellation.** It can happen at any `.await` point: `select!`
dropping the losing branches, a timeout wrapper, an outer future being dropped for any
reason. Once dropped, a future can never be polled again — any work after the current
suspension point simply never runs, and any local state it held is lost (its `Drop` impls
still run, but no further `.await`s complete).

This means an `async fn` is **not** transactional across `.await` points by default. Code
that assumes "if I called `foo().await` and it returned, the whole operation completed" is
wrong if `foo` can be cancelled mid-flight by whatever's driving it (a `select!` racing it
against something else, a caller wrapping it in `timeout(...)`).

Checklist for cancellation-safety when writing or reviewing async code:
- After every `.await` inside a function whose future might be dropped externally (almost
  any function called from `select!` or wrapped in a timeout), ask: **"if execution stops
  right here and never resumes, is any invariant broken?"** — a partially-sent message, a
  lock that won't be released, a state machine left mid-transition.
- Prefer operations that are atomic from the caller's perspective, or that can be safely
  retried/resumed from scratch, over ones with meaningful partial progress.
- If a `select!` branch does something that must complete once started (e.g. writing to a
  socket), don't let dropping that branch's future skip cleanup — either make the operation
  itself cancel-safe (many Tokio APIs document this explicitly — check the docs for
  "Cancel safety" sections), or move the non-cancellable part outside the `select!` (e.g.
  spawn it as its own task, and `select!` on a completion signal instead of the operation
  itself).
- Never assume `some_future.await` running to completion is guaranteed just because it's
  in a "happy path" — anything driven by `select!`, `timeout`, or an executor shutdown can
  cut it off at any suspension point.

## Pitfall 4: async fn in traits — dyn compatibility

`async fn` in a trait desugars to a method returning `-> impl Future<Output = ...>`
(stabilized 1.75). Two consequences that will surface as confusing compile errors:

- **Return-position `impl Trait` captures all in-scope generic lifetimes** — some borrowing
  patterns that would be fine in a plain `fn` don't type-check as an async trait method.
- **Async trait methods are not object-safe** — you cannot build a `Box<dyn Trait>` /
  `dyn Trait` over a trait with `async fn` methods natively.

If you need `dyn Trait` with async methods, use the `async_trait` crate's `#[async_trait]`
macro (on both the trait definition and every impl) — it boxes the futures for you, at the
cost of an allocation per call and losing some borrowing flexibility. Don't reach for it by
default on traits that never need dynamic dispatch; prefer plain `async fn` in a generic
(`impl Trait` / type-parameter) bound when you control all call sites.

## `join!` vs `select!`

| | Waits for | Use when |
|---|---|---|
| `join!` / `futures::future::join_all` | **All** futures to complete, collects all results | You need every result and don't care about order of completion (parallel HTTP fetches, batch of independent queries) |
| `select!` | **Whichever** future completes **first**, runs that arm, drops the rest | Racing an operation against a timeout, an "event or shutdown signal" loop, first-response-wins |

- `join_all`'s risk: if **any** one future never resolves, the whole `join_all` stalls forever
  — a single hung request blocks the batch. Wrap individual futures in a timeout if that's a
  real risk.
- `select!`'s risk is Pitfall 3 above: the branches that don't win are **dropped**, i.e.
  cancelled — make sure that's actually safe for each arm before relying on `select!` in a
  loop (a common "actor" pattern: `select!` on an inbound-message channel plus a periodic
  timer, looped forever).

## Quick diagnosis table

| Symptom | Likely cause | See |
|---|---|---|
| Futures that should run concurrently run one after another | Blocking the executor | Pitfall 1 |
| "cannot move out of...", "value does not implement Unpin" | Improper handling of `Pin` | Pitfall 2, `rust-pinning` |
| Data loss / half-done work after a `select!` or timeout | Cancellation dropped a future mid-flight | Pitfall 3 |
| `the trait ... cannot be made into an object` on an async trait | Async fn + `dyn Trait` | Pitfall 4 |
| Program hangs at an `.await` | Nothing will ever complete that future — check for a channel with no sender/receiver, a lock never released, or a `join_all` waiting on one stuck future | Pitfall 1 / `rust-concurrency-sync` |
