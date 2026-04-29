---
status: pending
area: java
priority: medium
---

# Parse Java annotations and constant initializers

## Description

Populate `Symbol.annotations` and `Field.constant_value` from the parse
tree:

- For each declarable symbol (class, method, field, parameter, type
  parameter, etc.) collect declared annotations as `AnnotationInstance`
  values, including their element values.
- For fields, when the initializer is a compile-time constant
  expression, evaluate it into a `ConstantValue` and store it on
  `Field.constant_value`. Also set `Field.initialized = true`.

Element-value evaluation has to handle the JLS-defined constant forms:
literal expressions, references to other constants (resolved through
the symbol model), array initializers, nested annotations, and class
literals.

## Context

Required for annotation-aware features (annotation completion,
`@Retention`/`@Target` validation, `@FunctionalInterface` checking),
switch-statement duplicate-label detection (which uses
`Field.constant_value`), and definite-assignment analysis on blank
finals (which uses `Field.initialized`).

The model side landed with backlog 001; this item is the parser
adoption.

Constant evaluation has subtle edges (signed vs. unsigned shifts,
floating-point NaN, integer overflow). Test against JLS-derived cases.

## Acceptance criteria

- A class annotated `@Deprecated` exposes the annotation in
  `Symbol.annotations` with the FQN resolved.
- `@FunctionalInterface` is detected on declarations that have it.
- A `final int N = 1 + 2;` field has `constant_value = Some(Int(3))`.
- A field with an explicit initializer has `initialized = true`; one
  without does not.
- Fixture tests cover annotations on each symbol kind and at least a
  few constant-expression shapes.
