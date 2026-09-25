Ingress:

- Add a project import subsystem that recognizes Maven, Gradle, SBT, etc.,
  offers an import choice through LSP, and supplies the resulting classpath.
  Until then, ingest accepted files across the workspace with no classpath.
- Respect gitignored files during the fallback workspace walk when needed.

Navigation:

- Expand hover beyond modeled type positions and show resolved information
  rather than just echoing the type as written in source.
- Expand `goto_definition` beyond named field type components to type parameters
  (including declaration-name spans), nested type arguments, supertype clauses,
  array elements, package-qualified names, and other reference kinds.
- Supply revision-consistent target text beyond workspace-indexed source files;
  JVM-only candidates also need source mapping.

Resolution:

- Complete `lookup_external` with single-static and static-on-demand type imports,
  module imports, import-conflict and accessibility checks, and inherited member
  types across files. Refine package/type-boundary handling for qualified names.
- Map JVM candidate names to canonical Java names and provide classpath-aware JVM
  lookup when that vertical has data.
- Resolve an omitted type-parameter bound as `java.lang.Object` once platform-backed lookup is wired.
- Preserve static field and method context in the Java model, then extend type-parameter usage validation to JLS §6.5.5.1's static-context rule.
- Preserve recoverable hierarchy-branch failures, including unresolved type-parameter bounds and circular inheritance, alongside candidates found through other branches.
- Think about how to move some basic operations that we do nowadays with `iter_enclosing_types` and friends.
  Probably these could be exposed as member methods on the File itself.