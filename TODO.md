Resolution:

- Preserve static field and method context in the Java model, then extend type-parameter usage validation to JLS §6.5.5.1's static-context rule.
- Preserve recoverable inheritance-branch failures, including circular inheritance, alongside any candidates found through other branches.
- Think about how to move some basic operations that we do nowadays with `iter_enclosing_types` and friends.
  Probably these could be exposed as member methods on the File itself.