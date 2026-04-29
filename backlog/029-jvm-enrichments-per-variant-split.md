---
status: pending
area: core
priority: low
---

# Split JvmEnrichments into per-variant structs when second field lands

## Description

`JvmEnrichments` is currently a single struct repeated across every JVM
node payload variant (`JvmTypeNode`, `JvmMethodNode`, `JvmFieldNode`,
`JvmParameter`, but not `JvmConstructorNode`). It carries one field
today: `nullability: Option<NullabilityInfo>`.

ARCHITECTURE.md commits to additional enrichments — `property_origin`,
`has_defaults` — when their first cross-language consumer needs them.
Those won't apply uniformly:

- `property_origin` makes sense on `JvmField` and `JvmMethod` (Kotlin
  properties surface as both); meaningless on `JvmType` and
  `JvmParameter`.
- `has_defaults` makes sense on `JvmMethod` only.

Today's shared bag works because `nullability` is broadly applicable.
Once a non-uniform field lands, the bag stops carrying its weight: types
and parameters carry a `property_origin` slot that is forever `None`.

When the second enrichment field is added, refactor to per-variant
structs:
- `JvmTypeEnrichments` (nullability for the type as a whole, etc.)
- `JvmMethodEnrichments` (nullability + property_origin + has_defaults)
- `JvmFieldEnrichments` (nullability + property_origin)
- `JvmParameterEnrichments` (nullability)

`JvmConstructorNode` continues to carry no enrichment field by design;
constructor-parameter nullability lives on the parameter children.

## Context

Surfaced during code review of step 3 of the graph migration. Reviewer
recommended keeping the shared bag while there's only one field, then
splitting on demand rather than up front.

## Acceptance criteria

- Each variant carries the enrichment fields it can meaningfully
  populate, none of the ones it can't.
- Existing nullability tests and consumers continue to work.
- Doc-comments on each variant's enrichment struct explain what it
  carries and what's deliberately absent.
- No regression on cross-language nullability flow tests.

## Trigger

Open this when the *second* enrichment field is about to be added. Don't
do it preemptively.
