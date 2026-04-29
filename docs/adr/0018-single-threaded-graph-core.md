# ADR-0018: The graph core is single-threaded; parallelism lives at the file batch

## Status

Accepted

## Context

The semantic graph is a mutable, cyclic, self-referential data
structure with thousands to millions of nodes, dense cross-references,
push-stale notifications, and reentrant subscriber callbacks. The
question is whether mutation of this structure should be multi-threaded
(`Arc<Mutex<_>>` style) or single-threaded (`Rc<RefCell<_>>` style,
ADR-0015).

The argument for multi-threaded mutation is throughput: parsing N files
in parallel and concurrently inserting their nodes into the graph
should be faster than doing it serially. In a server context, multiple
LSP requests arriving simultaneously could each touch the graph in
parallel.

The arguments against are concrete:

- The graph is densely connected. A fine-grained lock per node has
  millions of locks and frequent contention; a coarse-grained lock has
  no parallelism. The middle (per-subtree, per-language) is hard to
  reason about and easy to deadlock.
- Subscriber notifications are inherently fan-out. A single registry
  update can notify dozens of subscribers, each of which may notify
  more. Coordinating this across threads requires either a global lock
  or a deferred-dispatch mechanism, both of which give up most of the
  parallelism benefit anyway.
- Re-entrancy under `Mutex` is harder than under `RefCell`. `RefCell`
  panics on re-entrant `borrow_mut` (clear bug, fix the snapshot-and-
  release pattern); `Mutex` deadlocks (process hangs, no clear
  signal).
- The actual bottlenecks are parsing and disk I/O, not graph mutation.
  Parsing scales naturally per-file. Disk I/O is OS-level. Graph
  insertion of an already-parsed file is cheap relative to the parse.

## Decision

The graph engine is **single-threaded for graph operations**.
Internally, mutation uses `Rc<RefCell<_>>` and friends. There is no
`Mutex`, no `RwLock`, and no `Arc` *within the graph*. Any code that
touches `NodeData`, `Registry`, or related state runs on a single
thread.

Parallelism happens at a coarser boundary: **file-batch parsing**.
When the workspace loads or refreshes, files are parsed in parallel
using `rayon`. The output of each parse is a `ParsedFile` value that
is `Send` and self-contained — no graph references. When parsing of a
batch finishes, the parsed values are submitted to the graph's owning
thread, which integrates them serially.

```
[ thread pool ]                   [ graph thread ]
  parse file A  ──┐
  parse file B  ──┼── batch ──>   integrate(A, B, C, ...)
  parse file C  ──┘                 (one at a time)
```

`RefCell` borrow violations are treated as **bugs to fix**, not
recoverable conditions. The fix path is the snapshot-and-release
pattern (ADR-0015): if a borrow is held during a notification dispatch,
copy what is needed, drop the borrow, then dispatch. Code that
violates `RefCell` invariants panics, and we fix the offending code.

The LSP server wraps each request handler in `std::panic::catch_unwind`
so that a panic in one handler returns an error to the client without
killing the server. Underlying state is presumed unaffected — handlers
do not perform multi-step mutations that could leave the graph in a
half-updated state, and the engine is single-threaded so a panic
unwinds cleanly through one stack.

## Consequences

**Positive.**

- The graph code is dramatically simpler. No locks, no atomics, no
  thinking about ordering across threads, no surprise deadlocks.
- Errors are loud. `RefCell` panics at the bug site. We never get the
  silent-corruption-on-Tuesday class of bugs that `Mutex` invites.
- Reasoning is local. Every graph operation runs in a single linear
  call stack on a single thread. Reading the code tells you what
  happens.
- Parallelism still exists where it matters: parsing N files
  concurrently is the workload that actually benefits from threads.
  The serial integration step is cheap compared to the parsing it
  follows.

**Negative.**

- The graph thread is a serialization point. If the workload is
  somehow integration-bound rather than parse-bound, this design will
  not scale through more cores. We have no evidence this is the
  workload, and substantial evidence the bottleneck is elsewhere.
- LSP requests serialize on the graph thread. A long-running query
  blocks shorter queries that arrive after it. The mitigation is
  query design — keep handlers short, push expensive work into
  parsing/precomputation so handlers are mostly cache lookups.
- Cross-process or cross-machine setups (remote indexing, future
  distributed workspace) would require a fresh design pass. The
  serialization format (ADR placeholder for the binary format) is
  built with this in mind, but the runtime is not. We accept this; the
  immediate goal is a fast local LSP.
- A panic in a notification callback does not run the remaining
  callbacks for that notification (ADR-0015). The LSP wrapper
  contains the blast radius, but a buggy callback can drop work for
  one event.

## Alternatives considered

**`Arc<Mutex<_>>` graph with multi-threaded mutation.** A standard
shape for thread-shared mutable state. Rejected because the graph is
densely connected and re-entrant; we expect either heavy contention on
a coarse lock or deadlocks on fine ones. The throughput we would gain
is throughput we do not need — the bottleneck is parsing, which we
parallelize at a coarser, cleaner boundary.

**`RwLock` for read-mostly access.** Better than `Mutex` for read-heavy
workloads, but the graph is not strictly read-mostly during indexing,
and during steady-state reads with no writes, single-threaded is just
as fast. Rejected for the same reasons as `Mutex`, with the additional
hazard of writer starvation.

**Per-language thread.** Each language's nodes live on a dedicated
thread; cross-language messages route through channels. Rejected
because the JVM interop layer is shared across languages and would
require explicit handoffs for every cross-language query, killing the
performance argument for the design and adding complexity nobody asked
for.

**Lock-free / actor model.** Avoids `Mutex` deadlocks by design.
Rejected because the graph is a mutable connected structure — the
shapes that fit lock-free patterns (queues, log structures, trees with
hand-over-hand locking) do not match what we have. Forcing it would
mean rebuilding the engine around an actor framework before we know
whether the constraint is real.

**`tokio` with `Send`-bound futures everywhere.** Removes thread
mutability concerns by sequencing on a runtime. Rejected because it
mandates `Send + 'static` on every closure that touches the graph,
which interacts poorly with `Rc<RefCell<_>>` and the borrow patterns
we use. We are not building an async system; we are building a
single-threaded engine with parallelism at the file boundary.
