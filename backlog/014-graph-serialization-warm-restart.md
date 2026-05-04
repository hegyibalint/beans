---
status: dropped
area: graph
priority: medium
---

# Serialize the graph for warm restart

## Description

Superseded by [035-snapshot-fast-restart-strategy](035-snapshot-fast-restart-strategy.md).

This item described warm restart in terms of the old graph-level
stable-vs-volatile distinction (ADR-0011) and graph-level pull-recompute
(ADR-0009/0010). Both have been reversed by ADR-0027; the warm-restart
strategy now lives at the artifact boundary, not the graph node boundary.
The replacement item captures the updated design.

The original notes are kept here for historical context.
