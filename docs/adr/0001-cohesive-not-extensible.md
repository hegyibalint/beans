# ADR-0001: Be cohesive, not extensible

## Status

Accepted

## Context

Beans is an LSP for JVM languages. The set of languages it targets is known
upfront and small: Java, Kotlin, Scala, Groovy, Clojure. There is no business
case for supporting a language outside that family — the value proposition
(a single index that spans every JVM language in your project) only holds
because all five compile to the same runtime, share a class file format, and
interoperate at the JVM level.

The dominant prior art (IntelliJ IDEA, Eclipse) is built the opposite way:
a generic platform with a plugin API, where each language is a third-party
extension. That model pays a steep tax. Plugins must communicate through
abstract interfaces. Core types (PSI, UAST in IntelliJ) have to be flexible
enough for languages that haven't been written yet. Cross-language features
require negotiating shared vocabulary across plugins maintained by different
teams. The architecture is shaped by the requirement that anyone can add a
language, even though in practice 99% of users only care about the four or
five that ship in the box.

We had to decide whether to follow that path — designing for extension by
unknown future languages — or commit to the five we know about and build
something tighter.

## Decision

Beans is **cohesive, not extensible**. The five target languages are baked
into the architecture at compile time. There is no plugin API, no dynamic
language registry, no generic abstraction for "any language that might
someday want to be a JVM language." Adding a sixth JVM language is a code
change in this repository, not a downstream extension point.

Concretely:

- `SymbolKind` is an enum that exhaustively enumerates the variants we need
  across all five languages (Class, Interface, Trait, Object, Defrecord,
  Namespace, ...). It is not open for extension.
- The `Language` trait exists, but only to share parsing dispatch across the
  language crates we ship. It is not a public extension point.
- The JVM is the universal interop layer. The model assumes class files,
  packages (or namespaces), and method dispatch on objects. Languages that
  don't fit that model are out of scope.

## Consequences

**Positive.**

- Adding a language quirk is straightforward: extend the enum, handle the
  variant, ship it. No negotiation across plugin boundaries.
- Cross-language features (rename across Java/Kotlin, find references across
  all five) are tractable because every language emits into the same shared
  symbol table. There is no plugin firewall to push information across.
- The code is smaller and faster. No trait objects for plugin dispatch, no
  dynamic registries, no serialization at extension boundaries.
- New contributors can read the whole system end to end. There is no plugin
  protocol to learn.

**Negative.**

- A new JVM language (say, a future entrant) requires a code change to the
  core. We accept this. New JVM languages appear roughly once a decade.
- We cannot serve users whose primary language is outside the JVM family.
  This is a feature, not a bug — those users have other tools.
- Refactors that change the universal model touch every language crate. That
  is fine when there are five of them; it would be intolerable with fifty.
  We are explicitly choosing the small-N regime.

## Alternatives considered

**Generic plugin platform (IntelliJ-style).** Would let third parties add
languages without modifying beans. Rejected because the cost — abstract
interfaces, dynamic dispatch, version skew between plugins, hard cross-
language features — is far greater than the benefit. We are not trying to
build a platform; we are trying to build a working LSP for five specific
languages. The plugin tax shows up in every line of code and we never get
the benefit because the language set is closed in practice.

**Support languages outside the JVM family.** Would broaden the user base
but would break the central premise — the shared JVM layer is what enables
cross-language navigation. Without it, beans would be a worse copy of
existing single-language LSPs that already serve those communities well.
Rejected because it would dilute the only thing beans does that nothing
else does.

**Hybrid: a generic core with a closed plugin set.** The worst of both
worlds. We pay the abstraction cost without the benefit of an open
ecosystem. Rejected.
