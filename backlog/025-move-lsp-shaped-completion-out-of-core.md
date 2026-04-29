---
status: pending
area: lsp
priority: medium
---

# Move LSP-shaped completion result out of beans-core

## Description

`beans-core/src/completion.rs` defines `CompletionItem` and `CompletionItems`
in a shape that maps directly onto the LSP protocol's `CompletionItem`
(fields like `detail: String`, `params: Vec<(String, String)>`). This
violates the library-first principle (ADR-0002, ADR-0020): an LSP-shaped
output type lives in the core library that an LSP-agnostic CLI would also
depend on.

Split it into two types in two layers:

- **`beans-core`**: a neutral result type carrying node references and
  minimal metadata — what's visible at this cursor, by kind, with enough
  information for any consumer to format. Provisional name:
  `CompletionCandidate` or `CompletionResult`.
- **`beans-lsp`**: the LSP-shaped `CompletionItem` plus an adapter that
  takes the neutral result and produces the LSP wire format. Lives only
  in the LSP crate; nothing else imports it.

The fixture test harness (`beans-test-harness`) must keep depending only
on `beans-core` (it cannot import `beans-lsp` without breaking the leaf
property of ADR-0020). Tests assert on the neutral result.

## Context

The current placement is a prototype-era artefact. ADR-0003 marks code in
this category as disposable — it works, but the layering is wrong on
purpose-built terms. ADR-0021 ("preserve the tree-sitter walker, rewrite
the layers above it") explicitly puts this kind of code in the rewrite
pile.

This is most naturally done as part of the milestone where the graph
engine first powers completions through real registry queries. Doing it
earlier in isolation would mean keeping two parallel completion code
paths during the transition.

## Acceptance criteria

- `beans-core/src/completion.rs` removed (or replaced by a neutral
  result type with no LSP-specific fields).
- `beans-lsp` owns the LSP-shaped `CompletionItem`, with an adapter from
  the neutral type.
- `beans-test-harness` still compiles depending only on `beans-core`.
- All existing fixture tests still pass.
- `beans-core` has no module or symbol that mentions LSP-protocol
  concepts (`detail` strings, LSP wire shapes, etc.).
