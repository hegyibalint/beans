# TODO

## Java

- [ ] Profile `TypeBound`'s `Vec` allocation costs before considering a different storage representation.

## Java resolution prototype

- [x] Resolve the three-file `examples/resolution` fixture using lexical scope, single imports, and inherited members.
- [x] Preserve blocked supertype lookups; stop ambiguous branches, continue independent branches, and guard lookup cycles.
- [ ] Retain successful supertype-reference outcomes too, including failures inside their arguments; keep declaration validation separate from member selection.
- [ ] Resolve package-prefixed references and static/on-demand/module imports.
- [ ] Resolve member types through type-parameter bounds and implicit enum/record supertypes.
- [ ] Complete cross-package protected access, static-context checks, and declaration/generic well-formedness checks.
- [ ] Reconcile JLS §6.3's supertype type-parameter scope wording with `javac 26` for `class Base {} class Use<Base> extends Base {}`. The prototype blocks this case rather than selecting the top-level class; `javac` rejects it as a type parameter used where a class is required.
- [ ] Represent method/block occurrence scopes and record-header reference locations.
- [ ] Design primitive, array, and void resolution outcomes; the prototype still resolves named references only.
- [ ] Migrate or retire the dormant `resolution/tests` suite once the prototype API stabilizes; it remains separate from `resolution/prototype_tests`.
