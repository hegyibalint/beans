---
status: pending
area: clojure
priority: low
---

# Add Clojure language module skeleton

## Description

Create the Clojure language module as a feature-gated submodule of
`beans-core` per ADR-0019.

Initial scope:

- Namespace declarations (`ns` forms).
- `def`, `defn`, `defmacro`, `defprotocol`, `defrecord`, `deftype`.
- `import` and `:require` forms in namespace declarations.
- `gen-class` directives (these produce JVM-visible classes).

Macro expansion is out of scope for the skeleton. The parser captures
the surface syntax; macro-aware analysis (if we ever do it) is a
separate item.

## Context

Clojure is the most JVM-divergent language we target. Most beans
features (overload resolution, sealed types, etc.) have no analogue in
Clojure. The value of supporting Clojure is mainly cross-language
projection: a Clojure namespace declared with `gen-class` produces a
JVM class that Java/Kotlin code may reference, and we want those
references to resolve.

Per ADR-0004, the Clojure model is its own thing; the JVM projection
covers only what `gen-class` and `defrecord`/`deftype` make visible.

Lowest priority among the language modules because the audience overlap
with our other supported languages is the smallest.

## Acceptance criteria

- `cargo build --features clojure` succeeds.
- A simple `.clj` file with a namespace and a few `def`/`defn` forms
  produces nodes.
- A `gen-class` form produces a node visible to a Java fixture by FQN
  through the JVM projection.
- A regression fixture covers `ns`, `defn`, and `gen-class`.
