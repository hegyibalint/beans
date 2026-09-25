Ingress:

- Reconcile open-document overlays with source-file storage and classpath
  visibility. Disk-ingested source and an open document with the same `file:`
  URI now share an identity, but closing the document removes its model rather
  than restoring the on-disk version.

Navigation:

- Expand hover beyond modeled type positions and show resolved information
  rather than just echoing the type as written in source.
- Use the preserved Java type-declaration and named-reference byte ranges to
  implement `beans_lang_java::lsp::find_declaration` and dispatch it through `beans-engine`.
- Preserve progressively finer byte ranges as navigation expands to more Java
  declarations and reference kinds.

Resolution:

- Add direct-import, current-package, on-demand-import, module-import, and as-written qualified-name tiers after upward/root lookup.
- Resolve an omitted type-parameter bound as `java.lang.Object` once platform-backed lookup is wired.
- Preserve static field and method context in the Java model, then extend type-parameter usage validation to JLS §6.5.5.1's static-context rule.
- Preserve recoverable hierarchy-branch failures, including unresolved type-parameter bounds and circular inheritance, alongside candidates found through other branches.
- Think about how to move some basic operations that we do nowadays with `iter_enclosing_types` and friends.
  Probably these could be exposed as member methods on the File itself.