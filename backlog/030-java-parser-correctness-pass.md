---
status: pending
area: java
priority: medium
---

# Java parser correctness pass

## Description

A code-review pass over `beans-core/src/languages/java/` (post graph
migration) surfaced several pre-existing correctness issues in the Java
parser and its surrounding helpers. None blocked the migration; all
deserve a focused pass before bytecode-level consumers come online.

1. **`is_java_identifier_char` misses `$`.** Java identifiers may
   contain `$` (JLS §3.8). The current `is_java_identifier_char` in
   `syntax.rs` accepts only `[A-Za-z0-9_]`, so identifier scanning
   stops at `$`. Affects `word_at_position` and any consumer that uses
   it for token extraction.

2. **`word_at_position` is byte-indexed, not char-indexed.** The
   function takes a `col: u32` and treats it as a byte offset into the
   line. For multi-byte UTF-8 (Unicode identifier letters, non-ASCII
   text in comments before the cursor) the offset misaligns. Should
   convert via `char_indices` to a byte index before slicing.

3. **`SymbolKind::EnumConstant` is dead in the prototype Symbol
   stream.** The Java parser emits enum constants as
   `SymbolKind::Field` with synthetic `public static final` modifiers.
   The `EnumConstant` variant exists in `SymbolKind` but no producer
   sets it. Spec tests assert `SymbolKind::Field` for `RED`/`GREEN`
   etc., so changing the parser would require updating those tests.
   Decide on the canonical kind (probably `EnumConstant` per JLS
   §8.9.1) and update parser + tests together.

4. **`MethodParam.is_varargs` hardcoded to `false`.** The walker's
   `extract_formal_parameters` treats `formal_parameter` and
   `spread_parameter` (varargs) tree-sitter nodes uniformly and never
   sets `is_varargs = true`. Should detect `spread_parameter` and
   propagate.

5. **`PrimitiveKind::from_str` failure path silently falls back to
   `Simple`.** `types::TypeRef::to_core` calls `PrimitiveKind::from_str`
   on every `Primitive(s)` value; on `None` it falls back to
   `simple(s)`. A `Primitive` whose name doesn't parse should be a
   debug-only assertion failure (parser bug) rather than silently
   degrading.

6. **`AnnotationElement::default_value: Option<ConstantValue>` cannot
   represent every JLS §9.6.1/9.6.2 default form.** Class literals,
   enum constants, nested annotations, and arrays-of-those are all
   permitted defaults; `ConstantValue` covers only primitives + String.
   Either extend `ConstantValue` or introduce a dedicated
   `AnnotationDefaultValue` enum used by `AnnotationElement`.

7. **`TypeRef::erasure` collapses all `TypeVariable`s to
   `java.lang.Object`** without consulting the surrounding `TypeParam`
   list's bounds. Per JLS §4.6 the erasure of a type variable is the
   erasure of its leftmost bound (`Object` only when unbounded). Add
   either a doc note explaining the limitation or a context-aware
   `erasure_with_params` helper.

## Context

Surfaced during code review of the merged step 4+5 commit on
`feat/graph-migration` (Java parser folded into `beans-core`, fixture
ported to graph). Most of these issues predate the migration; one (#3)
is more visible now because the new payload model has a dedicated
`JavaEnumConstantNode` variant that the parser never produces.

## Acceptance criteria

- Each numbered item resolved or explicitly documented as a deliberate
  limitation with a comment naming the JLS section involved.
- `cargo test --workspace` green.
- No regression on the 338 spec tests (or, if a spec assertion changes
  due to (#3), the test update is part of the same commit).
- A test exists for each behavior change (varargs propagation, `$` in
  identifiers, multi-byte word extraction, etc.).

## Notes

Probably best done as a single commit per item rather than one big PR
— each is small and individually reviewable.
