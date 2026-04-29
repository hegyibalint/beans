---
status: completed
area: core
priority: high
---

# Land the JVM model foundation

## Description

Initial migration of the beans-core data model, covering the work that was
identified as Tiers 1-3 in the (now retired) MODEL_EVOLUTION.md tracker.
Replaces string-typed fields with structured types and adds the symbol
extensions needed for annotation processing and modern Java features.

Concrete deliverables that landed in this batch:

- `TypeRef` enum with `TypeParam`, `PrimitiveKind` (widening + boxing),
  `erasure()`, and `substitute()` (`beans-core/src/type_ref.rs`).
- `SymbolKind::EnumConstant`, `Modifier::NonSealed`, `RelationKind::Permits`,
  and `Relation.type_args: Vec<TypeRef>`.
- Migration of `Signature` fields from `String` to `TypeRef` / `TypeParam`
  for methods, fields, and classes; addition of `Method.throws`,
  `MethodParam.is_varargs`, `Field.constant_value`, `Field.initialized`.
- New `Signature::Record` and `Signature::AnnotationElement` variants with
  `RecordComponent` and `ConstantValue` supporting types.
- `Symbol.annotations: Vec<AnnotationInstance>` plus the `AnnotationInstance`
  / `AnnotationValue` types in `beans-core/src/annotation.rs`.

## Context

This work is the model layer the rest of the architecture is built on top
of. It is recorded as a single completed item rather than a dozen for
historical reference; the per-item breakdown is in the git history of the
model migration commit and was previously tracked in MODEL_EVOLUTION.md
(retired).

Note that the SymbolTable layer that originally consumed this model is
itself being replaced by the graph engine (see ADR-0006 through ADR-0011).
Some of the per-symbol fields in this work are surfaced through node
payloads in the new architecture rather than through `Symbol`/`SymbolTable`.

## Acceptance criteria

- All listed types and fields exist in `beans-core` and compile.
- `cargo test --workspace` passes.
- The Java parser populates the new fields where the parser work has been
  done (parser items are tracked separately in this backlog).
