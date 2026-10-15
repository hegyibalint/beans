# Conference demo

A source-only Java workspace: no JDK or external libraries required.

Launch the Beans extension-development host from the repository root:

```sh
BEANS_WORKSPACE_TOML_JAVA_HOME="" scripts/dev-vscode.sh examples/conference
```

Open `src/demo/Example.java`.

- Named field types and generic arguments receive semantic type coloring, even when unresolved.
- Hover field types and use Go to Definition on `Widget`, `Box`, or `Generic`.
- Follow `Generic<Widget>` into `Generic.java`: its type parameter has a class bound
  and two interface bounds (`Widget & Readable & Writable`). The class implements
  both interfaces, and its fields demonstrate wildcard bounds and arrays.
- Use Go to Definition on a field's `T` inside `Generic.java` to reach its type parameter.
- `Missing` is deliberately undeclared, both as a field type and inside an array type argument.
- Change `Missing` to `Widget` to resolve those references.
- Alternatively, append `class Missing {}` to `Example.java`, then rename it to bring the errors back.

Unresolved field types display red squiggles. Editing any open Java file refreshes
diagnostics for all open Java files, so declaring `Missing` clears both errors.

For a cross-file version, create `src/demo/Missing.java` with:

```java
package demo;

class Missing {}
```

Open the new file so Beans receives it; filesystem watching is not implemented.
