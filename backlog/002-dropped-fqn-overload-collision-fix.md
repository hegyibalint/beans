---
status: dropped
area: core
priority: low
---

# Fix FQN overload collision in SymbolTable

## Description

Originally tracked as Tier 4 of the model evolution work: `SymbolTable`'s
FQN-to-symbol map is `HashMap<String, SymbolId>`, which silently drops
overloaded methods because they share the same FQN. The proposed fix was
`HashMap<String, Vec<SymbolId>>`.

## Context

Dropped because the `SymbolTable` layer is being replaced wholesale by the
graph/registry architecture described in ADR-0006 through ADR-0011 and
ADR-0019. Overloads in the new model are represented as multiple nodes
producing the same registry key, with the registry storing all providers
(ADR-0013). The collision problem does not exist in the new design because
there is no FQN-keyed flat map to collide in.

If a similar issue surfaces in a registry implementation during the graph
migration, it will be tracked as a fresh backlog item against that
registry, not against the legacy `SymbolTable`.

## Acceptance criteria

N/A — dropped. The migration to registries makes this item moot.
