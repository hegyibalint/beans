# Beans LSP — Direction & Roadmap

## Vision

Beans is a **multi-language LSP** for JVM languages:
- Java (current focus)
- Kotlin (stage 1)
- Groovy (stage 1)
- Scala (stage 2)
- Clojure (stage 3)

## Why One LSP

Individual language LSPs cannot communicate with each other. A Kotlin LSP and a Java LSP are separate processes with separate indexes and no shared state. This means:

- **Rename a Java interface method** → the Kotlin implementation doesn't update.
- **Find references on a Java class** → misses the Groovy test that calls it.
- **Navigate from a Kotlin file into a Java definition** → either doesn't work, or each LSP reimplements Java understanding independently.

Real-world JVM projects mix languages constantly — Java + Kotlin (Android, Spring), Java + Groovy (Gradle, Spock tests), Scala + Java (big data). Every boundary between languages is a blind spot for separate LSPs.

An integrated platform with a **single shared index across all JVM languages** eliminates these blind spots. One LSP that understands Java, Kotlin, Groovy, Scala, and Clojure can navigate, refactor, and diagnose across all of them — something no combination of individual LSPs can do.

---

## Design Philosophy

**Beans is cohesive, not extensible.** Unlike JetBrains (IntelliJ) or Eclipse, which must support an open plugin ecosystem with generic abstractions, Beans knows its full set of target languages upfront. All are JVM languages. This is a major architectural advantage:

- **The core model is purpose-built for JVM semantics** — no need to generalize for non-JVM languages.
- **Language-specific constructs map to shared types** — Kotlin's `data class`, Scala's `case class`, Groovy's `@Immutable` all map to the same model, because we know the full set of variants ahead of time.
- **Clojure is the known outlier** — `defrecord`, `defprotocol`, namespaces instead of packages. Since we know this upfront, the model accommodates it from day one rather than bolting it on later.
- **No extension point overhead** — no trait objects for plugin dispatch, no dynamic registries. Just an enum of known languages. Simpler, faster, easier to reason about.

### Lessons from JetBrains

- **Stub indices, not full parsing, for cross-file resolution.** IntelliJ uses lightweight, cached structural summaries (stubs). We should distinguish between "full parse" (open file) and "stub parse" (everything else) for performance.
- **UAST for the JVM family.** JetBrains built a Unified AST specifically for JVM languages that share concepts. Our core model serves the same role.
- **The JDK model is shared infrastructure.** Class files are parsed once and made available to all language parsers.

---

## Primary Milestone: Cross-Language Navigation

The first major milestone is **cross-language code navigation** — go-to-definition, find references, find usages across language boundaries. This is the killer feature that no existing tool outside IntelliJ provides.

The typical flow is **source → source**: a developer is in a `.kt` file, clicks on `MyService`, and jumps to `MyService.java` — or vice versa. `.class` files only matter for stdlib/dependencies where source isn't available.

This milestone is:
- **High value** — immediately useful, nothing else does this in a lightweight LSP
- **Feasible** — doesn't require understanding method bodies or deep type inference, just declarations, references, and import resolution
- **Architecture-proving** — if the shared model + index works for navigation across 2 languages, completions and diagnostics follow naturally

For the first demo: a project with Java + Kotlin files, where you can go-to-definition from Kotlin into Java source and back.

---

## User Stories

### S1 — Go-to-Definition (User Code)
> As a developer, I want to click on a type, method, or field reference and jump to its definition in my project — even if the definition is in a different JVM language.

### S2 — Go-to-Definition (Stdlib)
> As a developer, I want to click on `List` or `String` and navigate to a representation of that stdlib type.

### S3 — Find References
> As a developer, I want to find all usages of a type, method, or field across my entire project, regardless of which JVM language each file is written in.

### S4 — Member Completion
> As a developer, when I type `myList.` I want to see methods and fields on that type, whether the type is defined in Java, Kotlin, or the stdlib.

### S5 — Import Completion
> As a developer, when I type `import java.util.` I want suggestions for available types.

### S6 — Hover Info
> As a developer, when I hover over a type or method, I want its signature.

### S7 — Diagnostics
> As a developer, I want a warning when I reference a method that doesn't exist on a type.

---

## Phased Roadmap

**Strategy: full-stack Java first.** Get parse → index → LSP navigation working end-to-end for Java before adding other languages. The model lives in `beans-core` from the start so generalizing later is a refactor, not a rewrite.

### Phase 1 — `beans-core`: Symbol Model
Define the shared JVM symbol model in its own crate. The Symbol type, SymbolKind enum (exhaustive across all five target languages from day one), and the symbol table with basic indexing.

### Phase 2 — Complete Java Parser
Implement class, interface, enum, method, and field parsing in `beans-lang-java`, emitting `beans-core` symbols. This is the largest current gap.

### Phase 3 — Java Semantic Analysis
The symbol table knows *what exists*. The semantic layer understands *what's happening inside a method body*. This is where the two-layer architecture takes shape.

Built on-demand for the open file, consults the symbol table for external types, and tracks:
- What variables are in scope at the cursor position, and what types they have
- What type an expression evaluates to (`a.b().c` → resolve step by step)
- Control flow (reachable code, initialized variables)

This layer is **language-specific** — Java's type rules differ from Kotlin's, Scala's type system is far more complex, Clojure is dynamically typed. Getting the boundary between shared symbol table and language-specific semantic analysis right for Java will define the architecture for all later languages.

This is what unlocks:
- Member completion on local variables (`list.` → needs to know the type of `list`)
- Diagnostics / linting (type mismatches, unreachable code, uninitialized variables)
- Inline type hints

### Phase 4 — LSP Server
Wire Java parsing + symbol table + semantic analysis into the LSP server. Navigation first, then completion and diagnostics.

### Phase 5 — JMOD / Class File Parsing
Parse JDK stdlib from `.jmod` / `.class` files into the same symbol table. Enables go-to-definition and completion for stdlib types.

### Phase 6 — Second Language (Kotlin)
Second source parser, proving the cross-language architecture. Kotlin + Java is the most common multi-language JVM project shape. At this point `beans-core` should require minimal changes — if it does, that's a design signal.

---

## Current Status

| Component | Status |
|-----------|--------|
| `beans-core` symbol model | Done |
| Symbol table (multi-indexed arena) | Done |
| Language trait | Done |
| Resolution (imports, same-package, wildcard, static, compound) | Done |
| Java parser (class, interface, enum, record, annotation, members) | Done |
| `JavaLanguage` implementation | Done |
| LSP server (go-to-def, hover, references, document symbols) | Done |
| Fixture test framework (`beans-test-harness`) | Done |
| Java semantic analysis (type inference, flow) | Not started |
| JMOD / .class reader | Not started |
| Kotlin parser | Not started |

### Fixture Test Framework

The project uses a fixture-driven test framework (`beans-test-harness`) to encode expected behavior from the JLS and other language specs at scale. Source files contain cursor markers; assertions are written in Rust with a chainable API. The framework dispatches per file extension via the `Language` trait, supporting multi-language interop tests from day one. See `docs/FIXTURE.md` for details.
