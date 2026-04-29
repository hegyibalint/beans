---
status: pending
area: java
priority: high
---

# Convert Java parser to emit graph node payloads

## Description

Rewrite the output side of `beans-lang-java/src/parser.rs` so that each
`extract_*` function produces the new typed node payloads with hard and
dynamic links, instead of emitting `Symbol` records into a `SymbolTable`.

The walker itself (the recursive traversal of the tree-sitter tree, the
grammar-quirk handling, the position extraction) is preserved per
ADR-0021. What changes is:

- Allocate a node and set its typed payload fields directly from the
  parse tree.
- Replace `enclosing_stack: Vec<(usize, String)>` (using legacy SymbolId-
  equivalents) with a stack of node IDs.
- Record hard links to children as the walker descends.
- Emit dynamic links for type references and method calls; the registries
  resolve them later (ADR-0008).

Migrate function-by-function with a temporary adapter so the rest of the
walker continues to work during the migration. Delete the adapter when
the last function is converted (do not let it persist as a "compatibility
shim" — see ADR-0021's negative consequences).

## Context

This is the load-bearing migration from the legacy `Symbol`/`SymbolTable`
world to the graph engine. Until it lands, the new model and registries
are unused for actual Java source.

ADR-0019 places the Java module inside `beans-core` behind a feature
flag; this item should land that feature-gated module structure if it
is not already in place.

## Acceptance criteria

- `beans-lang-java::parser::parse_file` returns node payloads, not a
  `SymbolTable`.
- All existing Java fixture tests still pass.
- The temporary adapter is deleted before the item is marked completed.
- No code in `beans-core` depends on `SymbolTable` for new features.
