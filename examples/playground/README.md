# Playground

Run `scripts/dev-vscode.sh` from the repository root to open this folder with
the Beans development extension and `src/demo/Demo.java`.

## Workspace configuration

[`beans.toml`](beans.toml) defines one source unit containing `src`:

```text
src/
├── demo/
│   └── Demo.java
└── types/
    ├── ExternalImportedType.java
    ├── GenericType.java
    └── TypeParameters.java
```

`Demo.java` is the entry point; `types` contains the supporting declarations.
This demo focuses on source-level LSP features, not classpath configuration.
Paths are relative to this directory.

The development script sets `BEANS_WORKSPACE_TOML_JAVA_HOME` from
`mise where java`, using the JDK selected in the repository's `mise.toml`.
From the repository root:

```sh
mise install
scripts/dev-vscode.sh
# Optionally use a different JDK, or opt out of the override:
BEANS_WORKSPACE_TOML_JAVA_HOME="/path/to/jdk" scripts/dev-vscode.sh
BEANS_WORKSPACE_TOML_JAVA_HOME="" scripts/dev-vscode.sh
```

The script uses an isolated VS Code user-data directory (`target/vscode-dev`)
so your normal editor instance cannot supply a stale environment. Close all
Beans development windows before rerunning with changed environment variables. A nonempty value overrides
the unit's `jdk_home` setting; unset or empty preserves the descriptor settings.
Relative paths resolve against this workspace directory. For now, a selected JDK
triggers the warning about unavailable binary inputs rather than indexing it.

The development script enables `RUST_LOG=beans=debug`. Open **Output → Beans**
to see the server's stderr diagnostics:

```text
Loaded /…/examples/playground/beans.toml: units=1
Indexed 4 workspace sources
```

For a behavioral check, use **Go to Definition** on `ExternalImportedType` in
`ExternalImportedType imported;` in `Demo.java`. It should lead to
`src/types/ExternalImportedType.java`.

The fields in `Demo.java` progress from primitives and simple type navigation
to generics, member types, and nested arrays. Names such as `LocalType`,
`ExternalImportedType`, and `GenericType` describe the LSP use case. Follow `TypeParameters`
into `src/types/TypeParameters.java` for a class bound with multiple interface
bounds, a generic superclass, upper- and lower-bounded wildcards, and
type-parameter arrays. Hover currently shows the type as written, rather than
resolved type information.

`MissingType` is deliberately undeclared, both as a field type and inside an array
type argument. Replace it with `LocalType`, or append `class MissingType {}` to
`Demo.java`, to clear those diagnostics. Rename the declaration to bring them
back. Editing any open Java file refreshes diagnostics for all open Java files.
New files must be opened so Beans receives them; filesystem watching is not
implemented.

See [TOML workspace documentation](../../docs/WORKSPACE.md) for the full format,
fallback behavior, and current limitations.
