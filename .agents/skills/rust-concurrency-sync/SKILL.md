---
name: rust-concurrency-sync
description: Fix Rust thread and shared-state concurrency issues — Send/Sync compile errors, choosing a channel type, Arc<Mutex<T>> deadlocks, and mutex poisoning. Use when spawning OS threads, sharing state across threads, a type "cannot be sent between threads safely," or a multi-threaded program hangs/deadlocks.
source: |
  Adapted from Comprehensive Rust (https://google.github.io/comprehensive-rust/),
  © Google LLC, CC-BY-4.0. Code samples Apache-2.0. Source pin: 351fafa (2026-08-05).
  Paths: src/concurrency/{threads,channels,send-sync,shared-state,sync-exercises}/**
---

# Rust Threads & Shared State

`rust-patterns` has a short `Arc<Mutex<T>>`/channels section for the default idiom. This
skill is for when threading **is** the task: a `Send`/`Sync` compile error to fix, a channel
type to choose, or a hang/deadlock to diagnose. For `async`/`await` concurrency instead of OS
threads, see `rust-async`.

## Threads: spawned vs. scoped

`thread::spawn` closures must be `'static` — they **cannot borrow** from the spawning
function's stack, because the spawned thread might outlive it:

```rust
// Does not compile: closure would borrow `s`, but the thread could outlive `foo`
fn foo() {
    let s = String::from("Hello");
    thread::spawn(|| dbg!(s.len()));
}
```

Two fixes:
- **Own the data**: `move ||` the value into the closure, or clone/wrap it (`Arc`) if it's
  needed elsewhere too.
- **`thread::scope`**: guarantees every thread spawned inside the scope closure is joined
  before the scope returns, so borrows from the enclosing stack frame are sound:

```rust
thread::scope(|scope| {
    scope.spawn(|| dbg!(s.len()));  // borrowing `s` is fine here
});
```

Normal borrowing rules still apply inside a scope: one thread may hold `&mut`, or any number
may hold `&`, never both at once.

**`thread::spawn` doesn't keep `main` alive** — if `main` returns first, unjoined spawned
threads are simply cut off (not even guaranteed to finish). Capture the `JoinHandle` and call
`.join()` if you need the thread's work to complete, or its return value:

```rust
let handle = thread::spawn(|| { /* ... */ 42 });
let result: thread::Result<i32> = handle.join(); // Err(_) if the thread panicked
```

Thread panics are isolated — a panic in a spawned thread does **not** unwind or crash `main`;
it surfaces only as `Err` from `.join()`. If you never call `.join()`, a spawned thread's
panic is silently swallowed from the caller's point of view.

## Diagnosing `Send`/`Sync` errors

- **`Send`**: safe to *move* a value to another thread (its destructor may then run there).
- **`Sync`**: safe to *access* `&T` from multiple threads at once. Formally: `T: Sync` iff
  `&T: Send`.

When the compiler says a type "cannot be sent between threads safely" or "cannot be shared
between threads safely," identify which quadrant the type is in before reaching for a fix:

| | `Send` | `!Send` |
|---|---|---|
| **`Sync`** | Most types: primitives, `String`, `Vec<T>`, `Option<T>`, `Box<T>`, `Arc<T>` (thread-safe refcount), `Mutex<T>`, atomics, `mpsc::Sender<T>` (1.72+) | `MutexGuard<T>` — OS mutex primitives must be released on the thread that acquired them, but the guarded data can be read from other threads if `T: Sync` |
| **`!Sync`** | `mpsc::Receiver<T>`, `Cell<T>`, `RefCell<T>` — movable, but their interior mutability isn't safe for concurrent *shared* access | `Rc<T>` (non-atomic refcount — races if shared), raw pointers `*const T`/`*mut T` (assumed to have special concurrency concerns) |

**Fixes by symptom:**
- Type contains `Rc<T>` and needs to cross threads → swap for `Arc<T>`.
- Type contains `RefCell<T>` and needs to be `Sync` → swap for `Mutex<T>`/`RwLock<T>` (see
  below); or restructure so each thread owns its own copy instead of sharing.
- A closure captures a non-`Send` value (e.g. a raw pointer, an `Rc`, a `MutexGuard` held
  across an `.await`/thread boundary) → don't carry the guard/handle itself across the
  boundary; extract the data you need first, or restructure ownership.
- Generic types are `Send`/`Sync` automatically when their type parameters are — if a generic
  wrapper isn't inferred as `Send`, check whether one of its fields is the actual culprit.

## Channels — pick the right kind

`std::sync::mpsc` (Multi-Producer, Single-Consumer): `Sender<T>`/`SyncSender<T>` implement
`Clone` (many producers); `Receiver<T>` does not (single consumer). `send`/`recv` return
`Result` — `Err` means the other end was dropped and the channel is closed, not that the
operation itself failed transiently.

| Kind | Constructor | `send()` behavior | Use when |
|---|---|---|---|
| Unbounded | `mpsc::channel()` | Never blocks; allocates as needed | Producer rate is bounded some other way, or backpressure isn't a concern — otherwise risks unbounded memory growth |
| Bounded | `mpsc::sync_channel(n)` | Blocks the sending thread once the buffer of size `n` is full | You want built-in backpressure between producer and consumer |
| Rendezvous | `mpsc::sync_channel(0)` | Every `send()` blocks until a `recv()` is ready for it | Strict hand-off synchronization between exactly two threads |

A blocked `send()` on a bounded/rendezvous channel with no reader is a common source of
"the program just hangs" — if nothing is calling `recv()` (or the receiver was dropped without
draining), a full bounded channel blocks its sender forever.

## Shared mutable state: `Arc<Mutex<T>>`

- **`Arc<T>`**: thread-safe shared *ownership* via atomic refcounting (the `Rc` for multiple
  threads). `Arc::clone` is cheap (an atomic increment) but not free like a raw pointer copy —
  don't clone in a hot loop if it can be hoisted. `Arc<T>: Send + Sync` iff `T: Send + Sync`.
  Reference cycles (`Arc` pointing to itself transitively) leak, same as `Rc` — use
  `std::sync::Weak` to break them.
- **`Mutex<T>`**: mutual exclusion *and* interior mutability — `Mutex<T>: Sync` whenever
  `T: Send`, via a blanket impl, which is how a `&Mutex<T>` shared across threads lets any one
  of them get `&mut T`. The `MutexGuard` returned by `.lock()` ties the `&mut T`'s lifetime to
  the lock being held; it cannot outlive the lock.
- **Combine them**: `Arc<Mutex<T>>` is the standard way to share *mutable* state across
  threads — `Arc` handles shared ownership, `Mutex` handles safe mutation. They're orthogonal
  concerns; wrapping one without the other doesn't give you both properties.

```rust
let v = Arc::new(Mutex::new(vec![10, 20, 30]));
let mut handles = Vec::new();
for i in 0..5 {
    let v = Arc::clone(&v);              // clone the handle, not the data
    handles.push(thread::spawn(move || {
        let mut guard = v.lock().unwrap();
        guard.push(10 * i);
    }));                                  // guard drops here — lock released promptly
}
handles.into_iter().for_each(|h| h.join().unwrap());
```

**Scope the guard as tightly as possible.** Holding a `MutexGuard` across unrelated work
(I/O, another lock acquisition, a long computation) is the single biggest source of
contention and deadlocks. Prefer an explicit block to force early drop:

```rust
{
    let mut guard = v.lock().unwrap();
    guard.push(40);
}   // lock released here, not at the end of the enclosing function
```

**Why `.lock()` returns `Result`, and what to do about it**: if the thread holding the mutex
*panicked* while it held the lock, the mutex becomes **poisoned** — a signal that the
protected data might be in an inconsistent state. `.lock()` on a poisoned mutex returns
`Err(PoisonError)`. Don't reflexively `.unwrap()` this away in code that must survive a
peer panic — recover with `.into_inner()` on the error if the data is known-safe to reuse, or
propagate the poisoning as a real failure.

## Deadlock diagnosis (dining philosophers pattern)

The canonical deadlock: N resources arranged in a cycle, each task needs two adjacent
resources, and every task acquires them in the same *relative* order (left-then-right). If
all tasks grab their "left" resource simultaneously, every task then blocks forever waiting
for its "right" resource, which its neighbor is holding. Rust's ownership/borrow checker does
**not** catch this — it's a runtime resource-ordering bug, not a memory-safety one.

Checklist when a multi-threaded program hangs instead of crashing:
1. Look for **any point where two or more locks/resources are acquired in an order that isn't
   globally consistent** across all threads — the classic fix is a fixed global acquisition
   order (e.g. always lock the lower-ID resource first).
2. Check for a `MutexGuard` held across a blocking call (channel `recv()`, another `.lock()`,
   `.join()`) — that's a lock held longer than intended, not necessarily a cycle, but the same
   symptom.
3. Check for a bounded/rendezvous channel `send()` with no live reader (see Channels above).
4. If reproducing is hard, add logging around every lock acquire/release with thread IDs
   (`thread::current().id()`) to reconstruct the actual acquisition order that occurred.
