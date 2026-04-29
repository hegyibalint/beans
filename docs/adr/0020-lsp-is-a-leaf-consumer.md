# ADR-0020: Keep beans-lsp a leaf consumer of beans-core

## Status

Accepted

## Context

ADR-0002 committed beans to being a library that ships an LSP
server, rather than an LSP server with a library hidden inside it.
The dependency graph terminates at `beans-lsp` — no other crate
depends on it. That principle is easy to state and easy to violate
once code starts being written. The temptation is real:

- A test wants to assert on document-symbol output, so it imports
  the LSP module that builds document symbols. Now the test crate
  depends on `beans-lsp`.
- A new consumer (a CLI) needs to format hover text. The formatter
  lives in `beans-lsp` because that's where it was first written.
  The CLI either depends on `beans-lsp` or duplicates the
  formatter.
- A node type is "really" only meaningful in the LSP context (e.g.,
  a diagnostic view), so it gets defined in `beans-lsp` for
  cleanliness. Now the union of all node types is split across two
  crates.

Each of these is locally reasonable. Together they erode the
library-first property and turn `beans-lsp` into a de facto core
crate by accident. ADR-0019 collapsed the workspace into one
library crate plus thin consumers; that move only pays off if the
consumers stay thin.

## Decision

`beans-lsp` is a leaf in the dependency graph. Nothing depends on
it. It contains only:

- LSP wire-protocol handling (request/response serialization,
  notification routing, server lifecycle).
- Debouncing and request scheduling specific to the LSP protocol.
- The mapping between `beans-core` types and LSP types
  (`SymbolKind` to `lsp_types::SymbolKind`, source ranges to LSP
  ranges, etc.).

Everything else lives in `beans-core`:

- All node types — including ones that look LSP-shaped (diagnostic
  payloads, document-symbol projections, hover content). They are
  domain types that the LSP renders, not LSP types.
- All formatting and resolution logic. If hover text needs
  formatting, the formatter lives in `beans-core` and is called
  from `beans-lsp`. A future `beans-cli` calls the same formatter.
- All registries, all rules, all parsing, all indexing.

The operational test: if a non-LSP consumer (CLI, batch analyzer,
embedding library user) cannot get to a piece of functionality
without depending on `beans-lsp`, that piece is in the wrong crate.
This is the same direction-of-dependency check ADR-0002 stated; this
ADR makes it concrete for the post-ADR-0019 layout.

## Consequences

**Positive.**

- The library is testable end-to-end without an LSP harness. Tests
  in `beans-core` can assert on the same data structures the LSP
  server consumes. The fixture framework (ADR-TBD on testing) is a
  direct beneficiary.
- A future `beans-cli` is mechanically straightforward: depend on
  `beans-core`, wire CLI arguments to library calls. No code is
  trapped behind the LSP boundary.
- Drift becomes visible. If a change starts pulling logic into
  `beans-lsp`, it shows up in code review as new files in the wrong
  crate. The boundary is one place; reviewers know where to look.
- Diagnostic payloads, document symbols, and hover content are all
  reusable. Tools that want to render hover text in a non-LSP
  context (a doc generator, a code-review bot) get the same output.

**Negative.**

- "All node types live in `beans-core`" sometimes feels heavy. A
  diagnostic node payload that is only ever rendered through LSP
  still lives in core. We accept the asymmetry: the library is
  closed under "things consumers need," and consumers do need
  diagnostics.
- LSP-specific optimizations have less room. If a fast path only
  matters for the LSP request flow, it has to be either expressed
  generically (so other consumers can use it) or kept narrow inside
  `beans-lsp`. We have not hit a case where this hurts; if we do,
  we revisit.
- Contributors who model their mental picture on "the LSP server is
  the program" find this counterintuitive at first. Onboarding
  needs to start from `beans-core`.

## Alternatives considered

**LSP as the central crate, with `beans-core` as a helper.** This
is the natural shape for many LSP projects: the server is the
binary, and the helper crate exists to keep the server file from
being 30,000 lines. Rejected for the reasons in ADR-0002 — non-LSP
consumers (CLIs, batch tools, embedders) become second-class, and
the library evolution is driven by LSP protocol concerns rather
than by what the index actually needs to express. ADR-0019 reduced
the crate count, but it did not change this direction.

**LSP and CLI as siblings under a shared `beans-engine` crate, with
LSP-shaped types living in the LSP sibling.** Splits node types
across two crates: domain nodes in the engine, presentation nodes
in the LSP. Rejected because the split is wrong — a "diagnostic"
is a domain concept (a finding about source code), not a
presentation concept. The fact that it is rendered as an LSP
diagnostic is a transport detail. If we want to render the same
diagnostic in a CLI's text output or a doc generator's HTML, we
need it in the shared crate.

**Allow `beans-lsp` to host types as long as nothing imports them
from outside.** A weaker version of leaf-consumer: the LSP can hold
its own types provided no other crate references them. Rejected
because it does not survive the next consumer. As soon as a CLI
wants to render the same diagnostic, the type either moves (a
backwards-incompatible refactor) or is duplicated. Better to put
it in the right place to begin with. The "no other crate imports
it" rule sounds enforceable but in practice means "no crate
imports it *yet*," and the rule erodes the moment a second
consumer arrives.
