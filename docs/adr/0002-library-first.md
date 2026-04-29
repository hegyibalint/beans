# ADR-0002: Build a library, ship an LSP

## Status

Accepted

## Context

The most visible artifact beans produces is an LSP server. It is what most
users will interact with. It is also what shows up in the project README,
the IDE extensions, and the demos. The natural temptation when building an
LSP is to make the LSP server the center of the project — the main crate,
the home of the symbol table, the place where parsing is orchestrated —
and treat everything else as a helper.

We had to decide whether to do that, or whether to flip the relationship:
make the core (symbol model, parsing, indexing, resolution) the center, and
treat the LSP as one consumer among several possible consumers.

Several real use cases push toward the latter:

- **Batch analyzers.** "Run beans across this 10,000-file repo and tell me
  which Kotlin files reference deprecated Java APIs." This wants the index
  and the queries, not an LSP protocol.
- **CLIs.** A `beans find-refs com.example.Foo` command would be useful for
  scripts and CI.
- **IDE plugins that bypass LSP.** IntelliJ and Eclipse don't speak LSP
  natively in all cases; a native plugin might want direct access to the
  symbol table.
- **Embedding.** Other tools (linters, code generators, doc generators)
  could benefit from the index.

If the LSP server holds the symbol table, all of these become awkward —
they have to either spin up an LSP server and talk JSON-RPC to themselves,
or duplicate logic. If the symbol table lives in a library that the LSP
server depends on, all of these are straightforward.

## Decision

Beans is a **Rust library that happens to ship an LSP server**. The library
is the product. The LSP server is one application of the library.

Concretely:

- The dependency graph terminates at `beans-lsp`. It is a leaf. Nothing
  depends on it.
- No core module (`beans-core`, `beans-lang-*`, future `beans-jmod`)
  depends on `beans-lsp`. They do not import LSP types, do not know about
  `tower-lsp`, and do not assume their consumer speaks JSON-RPC.
- Public APIs in core crates are designed for direct use, not for being
  wrapped in a JSON-RPC layer. If the LSP needs something the library
  doesn't expose, the fix is to expose it as a library API, not to add an
  LSP-only path.
- A second consumer (CLI, batch analyzer) should be addable without
  refactoring core crates.

## Consequences

**Positive.**

- The architecture is naturally testable. The symbol table can be exercised
  in unit tests without an LSP harness. The fixture framework
  (`beans-test-harness`) is a direct beneficiary — it talks to the library,
  not to a server.
- New consumers (CLI, batch tools) are cheap to add. Each is a thin shell
  around the library.
- The library can evolve faster than the LSP protocol. New symbol kinds,
  new query types, and new analyses don't have to wait on LSP method
  definitions.
- Reasoning about layering is simple: if you find yourself wanting to
  import `beans-lsp` from a core crate, you have a design problem.

**Negative.**

- Some LSP-shaped conveniences (request cancellation, progress reporting,
  document version tracking) must be expressed in library terms or pushed
  into the LSP layer. Pushing them into the LSP layer is fine; expressing
  them in library terms requires care.
- The LSP server cannot drive design choices in the library. If LSP wants
  a feature the library doesn't naturally provide, we have to either
  generalize the library API (the right answer) or duplicate logic in the
  LSP (a smell).
- Public API surface in core crates is broader than it would be if only
  one consumer existed. We accept this — a slightly bigger API is the
  price of multi-consumer support.

## Alternatives considered

**LSP as the central crate.** The LSP server holds the symbol table, the
parsers are wired in directly, and any other consumer either spins up the
server or copies code. Rejected because it would make non-LSP consumers
(CLIs, batch analyzers) second-class. The LSP would be the only way to
get at the index, which is the project's actual product. It would also
couple the library evolution to the LSP protocol.

**Library and LSP as siblings, with a shared "engine" crate beneath.**
This is roughly what we have, just framed differently. The framing matters:
"library-first with LSP as a leaf consumer" makes the dependency direction
explicit. "Siblings under a shared engine" leaves room for the LSP to
accumulate logic that should live in the engine. We picked the framing
that makes drift visible.

**Multiple LSP servers (one per language).** Rejected upstream by the
project's premise — a single shared index across languages is the whole
point (see ADR-0001). Worth noting here because it would also force
library-first by accident: with multiple LSPs over one index, the index
has to be a library. We get the same property for principled reasons.
