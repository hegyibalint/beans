Resolution:

- Add direct-import, current-package, on-demand-import, module-import, and as-written qualified-name tiers after upward/root lookup.
- Resolve an omitted type-parameter bound as `java.lang.Object` once platform-backed lookup is wired.
- Preserve static field and method context in the Java model, then extend type-parameter usage validation to JLS §6.5.5.1's static-context rule.
- Preserve recoverable hierarchy-branch failures, including unresolved type-parameter bounds and circular inheritance, alongside candidates found through other branches.
- Think about how to move some basic operations that we do nowadays with `iter_enclosing_types` and friends.
  Probably these could be exposed as member methods on the File itself.