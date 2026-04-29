# ADR-0005: Sync core with rayon for parallelism, no async runtime

## Status

Accepted

## Context

Beans has to process a lot of files (a real Android or Spring project is
tens of thousands of source files plus a JDK and a tower of dependencies)
and answer LSP requests with low latency. There are two natural ways to
get parallelism in Rust:

1. **Async (Tokio).** Cooperative scheduling. Operations are `async fn`,
   they `await` on dependencies, the runtime multiplexes many tasks onto
   a small thread pool.
2. **Threads (rayon, std).** Preemptive scheduling on OS threads. Work is
   chunked and dispatched to a thread pool; per-task code is plain
   synchronous Rust.

Async is the obvious choice when the workload is **I/O-bound and
fan-in**: many tasks waiting on network, database, or RPC, with relatively
small amounts of CPU per task. The LSP server itself looks a bit like
this — it accepts requests over JSON-RPC, sometimes makes file system
calls, occasionally waits for the client.

The actual workload of beans does not look like that. The shape of the
work is:

- **Indexing.** At workspace open, parse thousands of source files and
  ingest them into the index. Each file's parse is CPU-bound and
  independent of every other file. Massive fan-out at the source level.
- **Per-file processing.** For a single file, parsing → symbol
  extraction → resolution is sequential. There is no real opportunity
  to overlap stages within one file.
- **Queries.** Most LSP requests (go-to-definition, hover, document
  symbols) are CPU-bound lookups against in-memory indexes. They don't
  await anything. They return.
- **Cross-file dependencies.** When a query needs information that
  isn't yet computed, it doesn't block waiting for it. The query
  returns "unresolved" and the engine pushes a stale notification when
  the missing piece becomes available.

This is rayon's natural shape: a work-stealing pool over independent CPU
tasks. Nothing in the core graph awaits. There is nothing for `await` to
do — all the data is in memory, all the computations are synchronous,
and parallelism is exploited by chunking files across worker threads,
not by interleaving tasks on a single thread.

The LSP boundary is different. `tower-lsp` is async because LSP is a
JSON-RPC protocol over stdio, which is naturally an I/O-bound
read-loop-and-dispatch shape. That part is fine; we can have an async
shell calling into a sync core.

## Decision

**The core is sync. Parallelism comes from rayon. There is no async
runtime in the core graph.**

Concretely:

- `beans-core`, `beans-lang-*`, the symbol table, the resolver, and the
  query layer are all synchronous Rust. No `async fn`. No `.await`. No
  `tokio::spawn`.
- Bulk operations (workspace indexing, batch reparses, cross-file
  resolution sweeps) use `rayon::par_iter` and friends.
- Cross-file dependencies (a Java file references a Kotlin class that
  hasn't been parsed yet) are handled by **queries returning
  unresolved** plus **push-stale notifications** when the missing piece
  arrives. We do not block. We do not await. We return now and notify
  later.
- The LSP server (`beans-lsp`) may use async at its protocol boundary
  because `tower-lsp` is async. The LSP server calls into the core
  synchronously. No async colours leak into the core.

## Consequences

**Positive.**

- The core is plain Rust. Stack traces are real. Debugging is normal.
  No runtime to set up, no executor to choose, no `Send + 'static`
  bounds to wrestle with.
- Parallelism is straightforward: `rayon::par_iter` over a `Vec<File>`
  parses every file in parallel with no ceremony.
- No async/sync split inside the core. Functions are functions. They
  don't have a colour. They can be called from any context.
- Tests are simple. No async test harness. Fixture tests just call
  library functions.
- Fewer dependencies. Tokio and its ecosystem are large; not depending
  on them in core crates keeps build times down.

**Negative.**

- Long-running operations cannot be cancelled cooperatively the way
  async tasks can. We mitigate this with cancellation tokens checked
  at coarse boundaries (per-file, per-batch).
- I/O in the core is blocking. If the core ever needs to do meaningful
  I/O (network calls to a Maven repo, talking to a build tool over a
  socket), we either do it synchronously on a rayon thread (fine for
  occasional calls) or push it out of the core into a layer that can
  use async.
- Rayon's work-stealing pool is global by default. We have to be
  careful about pool configuration in embedded contexts (a host
  application that already runs rayon).
- The async LSP boundary calling into a sync core means we have to
  bridge — typically by running core operations on a blocking thread
  pool spawned from the async runtime. This is a known pattern but
  it is one more thing to get right.

## Alternatives considered

**Heavy async (Tokio everywhere).** Make the entire core async.
Rejected because the workload isn't I/O-bound. Async pays off when
tasks spend most of their time waiting; our tasks spend their time
computing. We would pay the async tax (function colouring, runtime
setup, harder debugging, `Send + 'static` constraints, harder testing)
for parallelism we already get from rayon.

**Explicit thread pools with `std::thread`.** Manage threads ourselves.
Rejected because rayon already does this well, with work stealing and
a tuned default. There is no benefit to reinventing it. The only
reason to do this would be if rayon's global pool became a problem for
embedding, in which case we configure rayon (it supports custom pools)
rather than abandon it.

**Single-threaded.** Sync, no rayon, do everything serially. Rejected
because workspace indexing on real projects (10,000+ source files plus
a JDK and dependency JARs) is a multi-second operation even with
parallelism; serially it would be tens of seconds, which is unusable
as a startup cost. Initial indexing and bulk reparses need parallelism.

**Hybrid: async for I/O at the edges, rayon for CPU in the middle, with
the core being whichever the caller picks.** Rejected because "whichever
the caller picks" means the core has to be written async (because async
is contagious) and we lose the simplicity of a sync core. The actual
hybrid we want is the one we picked: async at the LSP edge (because the
protocol is async), sync core, rayon inside the core. The async layer
calls into the sync layer; nothing the other way.
