# Scala 3 Language Specification

**Source:** https://github.com/scala/scala3/tree/main/docs/_spec
**Date:** 2026-03-25

## Chapters

1. [Lexical Syntax](01-lexical-syntax.md)
2. [Identifiers, Names, and Scopes](02-identifiers-names-and-scopes.md)
3. [Types](03-types.md)
4. [Basic Definitions](04-basic-definitions.md)
5. [Classes and Objects](05-classes-and-objects.md)
6. [Expressions](06-expressions.md)
7. [Implicits](07-implicits.md)
8. [Pattern Matching](08-pattern-matching.md)
9. [Top-Level Definitions](09-top-level-definitions.md)
10. [XML Expressions and Patterns](10-xml-expressions-and-patterns.md)
11. [Annotations](11-annotations.md)
12. [The Scala Standard Library](12-the-scala-standard-library.md)
13. [Syntax Summary](13-syntax-summary.md)

## Appendices

- [A1 — Deprecated](A1-deprecated.md)
- [A2 — Scala 2 Compatibility](A2-scala-2-compatibility.md)
- [A3 — To Be Deprecated](A3-to-be-deprecated.md)

## Scala 3 Applied Reference (integrated into spec)

Changes already reflected in the spec chapters above.

- **Changed Features:** [imports](APPLIEDreference/changed-features/imports.md), [interpolation escapes](APPLIEDreference/changed-features/interpolation-escapes.md), [match syntax](APPLIEDreference/changed-features/match-syntax.md), [operators](APPLIEDreference/changed-features/operators.md), [wildcards](APPLIEDreference/changed-features/wildcards.md)
- **Contextual:** [given imports](APPLIEDreference/contextual/given-imports.md)
- **Dropped Features:** [auto-apply](APPLIEDreference/dropped-features/auto-apply.md), [class shadowing](APPLIEDreference/dropped-features/class-shadowing.md), [delayed init](APPLIEDreference/dropped-features/delayed-init.md), [do-while](APPLIEDreference/dropped-features/do-while.md), [overview](APPLIEDreference/dropped-features/dropped-features.md), [early initializers](APPLIEDreference/dropped-features/early-initializers.md), [existential types](APPLIEDreference/dropped-features/existential-types.md), [22 limit](APPLIEDreference/dropped-features/limit22.md), [macros](APPLIEDreference/dropped-features/macros.md), [nonlocal returns](APPLIEDreference/dropped-features/nonlocal-returns.md), [procedure syntax](APPLIEDreference/dropped-features/procedure-syntax.md), [symbol literals](APPLIEDreference/dropped-features/symlits.md), [this qualifier](APPLIEDreference/dropped-features/this-qualifier.md), [weak conformance](APPLIEDreference/dropped-features/weak-conformance.md), [wildcard init](APPLIEDreference/dropped-features/wildcard-init.md)
- **Enums:** [ADTs](APPLIEDreference/enums/adts.md), [index](APPLIEDreference/enums/enums-index.md), [enums](APPLIEDreference/enums/enums.md)
- **New Types:** [intersection types](APPLIEDreference/new-types/intersection-types.md), [type lambdas](APPLIEDreference/new-types/type-lambdas.md), [union types](APPLIEDreference/new-types/union-types.md)
- **Other New Features:** [control syntax](APPLIEDreference/other-new-features/control-syntax.md), [kind polymorphism](APPLIEDreference/other-new-features/kind-polymorphism.md), [opaque types](APPLIEDreference/other-new-features/opaques.md), [trait parameters](APPLIEDreference/other-new-features/trait-parameters.md)

## Scala 3 TODO Reference (pending spec integration)

Features documented but not yet integrated into the formal spec chapters.

- **Overview:** [overview](TODOreference/overview.md), [features classification](TODOreference/features-classification.md)
- **Changed Features:** [overview](TODOreference/changed-features/changed-features.md), [compiler plugins](TODOreference/changed-features/compiler-plugins.md), [eta expansion](TODOreference/changed-features/eta-expansion.md) ([spec](TODOreference/changed-features/eta-expansion-spec.md)), [implicit conversions](TODOreference/changed-features/implicit-conversions.md) ([spec](TODOreference/changed-features/implicit-conversions-spec.md)), [implicit resolution](TODOreference/changed-features/implicit-resolution.md), [lazy vals init](TODOreference/changed-features/lazy-vals-init.md), [main functions](TODOreference/changed-features/main-functions.md), [numeric literals](TODOreference/changed-features/numeric-literals.md), [overload resolution](TODOreference/changed-features/overload-resolution.md), [pattern bindings](TODOreference/changed-features/pattern-bindings.md), [pattern matching](TODOreference/changed-features/pattern-matching.md), [structural types](TODOreference/changed-features/structural-types.md) ([spec](TODOreference/changed-features/structural-types-spec.md)), [type checking](TODOreference/changed-features/type-checking.md), [type inference](TODOreference/changed-features/type-inference.md), [vararg splices](TODOreference/changed-features/vararg-splices.md)
- **Contextual:** [overview](TODOreference/contextual/contextual.md), [by-name context parameters](TODOreference/contextual/by-name-context-parameters.md), [context bounds](TODOreference/contextual/context-bounds.md), [context functions](TODOreference/contextual/context-functions.md) ([spec](TODOreference/contextual/context-functions-spec.md)), [conversions](TODOreference/contextual/conversions.md), [derivation](TODOreference/contextual/derivation.md) ([macro](TODOreference/contextual/derivation-macro.md)), [extension methods](TODOreference/contextual/extension-methods.md), [givens](TODOreference/contextual/givens.md), [multiversal equality](TODOreference/contextual/multiversal-equality.md), [relationship to implicits](TODOreference/contextual/relationship-implicits.md), [right-associative extension methods](TODOreference/contextual/right-associative-extension-methods.md), [type classes](TODOreference/contextual/type-classes.md), [using clauses](TODOreference/contextual/using-clauses.md)
- **Dropped Features:** [package objects](TODOreference/dropped-features/package-objects.md), [type projection](TODOreference/dropped-features/type-projection.md), [XML](TODOreference/dropped-features/xml.md)
- **Experimental:** [CanThrow](TODOreference/experimental/canthrow.md), [capture checking](TODOreference/experimental/cc.md), [erased defs](TODOreference/experimental/erased-defs.md) ([spec](TODOreference/experimental/erased-defs-spec.md)), [explicit nulls](TODOreference/experimental/explicit-nulls.md), [fewer braces](TODOreference/experimental/fewer-braces.md), [main annotation](TODOreference/experimental/main-annotation.md), [named type args](TODOreference/experimental/named-typeargs.md) ([spec](TODOreference/experimental/named-typeargs-spec.md)), [numeric literals](TODOreference/experimental/numeric-literals.md), [overview](TODOreference/experimental/overview.md), [tupled function](TODOreference/experimental/tupled-function.md)
- **Language Versions:** [binary compatibility](TODOreference/language-versions/binary-compatibility.md), [language versions](TODOreference/language-versions/language-versions.md), [source compatibility](TODOreference/language-versions/source-compatibility.md)
- **Metaprogramming:** [overview](TODOreference/metaprogramming/metaprogramming.md), [compile-time ops](TODOreference/metaprogramming/compiletime-ops.md), [inline](TODOreference/metaprogramming/inline.md), [macros](TODOreference/metaprogramming/macros.md) ([spec](TODOreference/metaprogramming/macros-spec.md)), [reflection](TODOreference/metaprogramming/reflection.md), [simple SMP](TODOreference/metaprogramming/simple-smp.md), [staging](TODOreference/metaprogramming/staging.md), [TASTy inspect](TODOreference/metaprogramming/tasty-inspect.md)
- **New Types:** [overview](TODOreference/new-types/new-types.md), [dependent function types](TODOreference/new-types/dependent-function-types.md) ([spec](TODOreference/new-types/dependent-function-types-spec.md)), [match types](TODOreference/new-types/match-types.md), [polymorphic function types](TODOreference/new-types/polymorphic-function-types.md)
- **Other New Features:** [overview](TODOreference/other-new-features/other-new-features.md), [creator applications](TODOreference/other-new-features/creator-applications.md), [experimental defs](TODOreference/other-new-features/experimental-defs.md), [export](TODOreference/other-new-features/export.md), [indentation](TODOreference/other-new-features/indentation.md), [Matchable](TODOreference/other-new-features/matchable.md), [open classes](TODOreference/other-new-features/open-classes.md), [parameter untupling](TODOreference/other-new-features/parameter-untupling.md) ([spec](TODOreference/other-new-features/parameter-untupling-spec.md)), [safe initialization](TODOreference/other-new-features/safe-initialization.md), [targetName](TODOreference/other-new-features/targetName.md), [threadUnsafe annotation](TODOreference/other-new-features/threadUnsafe-annotation.md), [transparent traits](TODOreference/other-new-features/transparent-traits.md), [type test](TODOreference/other-new-features/type-test.md)
