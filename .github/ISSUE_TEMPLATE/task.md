---
name: Task
about: A unit of implementable work — feature, refactor, fix, or investigation.
title: ""
labels: []
---

<!--
Beans uses ONE common format for every issue (feature, refactor, fix). Fill in
all four sections below; delete the comment hints as you go.

Keep it terse — aim for 30–80 lines total. If the issue grows past that, it is
probably more than one task: split it.

Classify with labels — exactly one a: (kind) and one or more in: (component):
  a:   a:feature a:bug a:chore a:investigation a:documentation
  in:  in:core in:lsp
       in:lang-java in:lang-kotlin in:lang-scala in:lang-groovy in:lang-clojure
       in:ide-vscode
-->

## Current behavior

<!--
What happens today, or what is missing. For greenfield work this may be "X does
not exist yet"; for refactors, describe the current structure. Ground it in
observable behavior wherever you can — e.g. "completion after `svc.` returns
nothing" beats "member completion is unimplemented".
-->

## Expected behavior

<!--
What should happen instead. Concrete and specific — the state this issue moves
us to.
-->

## Context

<!--
Why this matters, what depends on it, what it depends on. Reference ADRs by
number (ADR-0014) and related issues by #number. Note any decisions already
made during planning so they are not relitigated.
-->

## Acceptance criteria

<!--
How we know it is done. Each item independently checkable; prefer testable
assertions ("`Jmod::open(..)` enumerates `java.lang.String`") over prose.
-->

- [ ] 
- [ ] `cargo test --workspace` passes
