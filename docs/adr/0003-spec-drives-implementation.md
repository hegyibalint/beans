# ADR-0003: Treat the spec as the source of truth, not the code

## Status

Accepted

## Context

Beans currently has a working prototype: a Java parser, a symbol table, a
resolver, an LSP server, hundreds of fixture tests. It works for the cases
it was built for. It also has the shape of a thing that grew organically
from "let me get go-to-definition working in Java" rather than from a
worked-out architecture for five languages with a shared JVM projection.

The prototype is informative — it taught us what queries the symbol table
needs to answer, what the Language trait should look like, where bytecode
parsing fits. It is also, in places, the wrong shape for where we are
going. The current `SymbolTable` is a single universal store; ADR-0004
moves to per-language models with a shared JVM projection. The current
code uses sync Rust without a clear concurrency story; ADR-0005 commits
to rayon. Several modules carry assumptions from the Java-first prototype
that won't survive Kotlin.

The question is what role the existing code plays. Two postures are
possible:

1. **Code is precious.** Preserve what works. Migrate incrementally.
   Every change is a refactor on top of the prototype.
2. **Code is disposable.** The architecture is the source of truth. When
   the prototype doesn't match the architecture, change the prototype —
   even if it means deleting working code and rewriting it.

The first feels safer. It is also how prototypes turn into permanent
production code. The second is more disruptive but keeps the architecture
honest.

## Decision

The **spec drives the implementation**. The ADRs and the architecture
documents are the source of truth for the system's shape. The current
code is a prototype that informs the architecture but does not constrain
it.

Concretely:

- When implementation diverges from the spec, fix the implementation.
  Do not bend the spec to match what the code happens to do.
- If the spec is wrong (the implementation revealed a real problem with
  the design), update the spec — by writing a new ADR, not by silently
  letting the code drift.
- Working code is not a reason to keep code. Sentimental attachment
  ("but I wrote this last month") is not a design argument.
- "We can fix this later" is not a plan. If the architecture says the
  code should be a different shape, the code should be that shape now.

## Consequences

**Positive.**

- The architecture stays coherent. We do not accumulate "the way it
  actually works" as a hidden second spec that diverges from the
  written one.
- New contributors trust the documents. If the ADRs say something, the
  code reflects it. There is no unwritten "but actually" lore.
- Decisions are reversible at the ADR level, not the code level. To
  change direction, write an ADR; the code follows.
- We are willing to delete code. This keeps the system small.

**Negative.**

- We will throw away working code. That is wasteful in the short term.
- There is a real risk of churning the architecture without shipping —
  rewriting in pursuit of a perfect spec. We mitigate this by treating
  ADRs as commitments, not drafts: once accepted, an ADR is not casually
  revisited.
- "The spec says X" can become an argument against legitimate
  implementation feedback. We mitigate this by making it easy to update
  the spec — a new ADR is the right response to "the spec is wrong,"
  not silent code drift.
- Contributors who joined for the prototype may feel their work is
  being discarded. The honest answer is that prototypes are valuable
  precisely because they are disposable. The lessons stay; the code
  doesn't have to.

## Alternatives considered

**Incremental migration.** Keep the prototype and refactor it toward the
target architecture, change by change. Rejected because incremental
migration on a moving target tends to produce a hybrid that has neither
the simplicity of the old design nor the power of the new one. We have
seen this pattern in other JVM tooling — a "transitional" architecture
that lasts a decade. We would rather be honest: the new shape is the
shape; the old shape is informational.

**Spec follows code.** Document what is, not what should be. Rejected
because it makes the documents reactive instead of normative. ADRs that
just narrate existing code are not decisions; they are descriptions. We
want decisions. The architecture is a thing we choose, not a thing that
emerges.

**No spec at all.** Let the code be the spec. Rejected because the
project has cross-language ambitions that are difficult to pursue
without a coordinating document. With five languages and several
consumers, "read the code" stops scaling as a coordination mechanism.
