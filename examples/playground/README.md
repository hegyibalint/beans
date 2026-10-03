# Playground

Run `scripts/dev-vscode.sh` from the repository root to open this folder with
the Beans development extension and `src/demo/Example.java`.

## Workspace configuration

[`beans.toml`](beans.toml) defines two units:

```text
demo → library
```

Demo's sources can resolve `library.Widget`; the library does not depend on
demo. Paths are relative to this directory. There is no configured JDK or
compiled classpath, since binary ingestion is not implemented yet.

The development script enables `RUST_LOG=beans=debug`. Open **Output → Beans**
to see the server's stderr diagnostics:

```text
Loaded /…/examples/playground/beans.toml: units=2
Indexed 2 workspace sources
```

For a behavioral check, use **Go to Definition** on `Widget` in
`Widget imported;` in `Example.java`. It should lead to `src/library/Widget.java`.
Remove demo's `depends_on` line and restart the server: the same query should
no longer resolve. Restore the line and restart afterward. Descriptor changes
are not watched yet.

The other fields in `Example.java` demonstrate modeled type hovers, member
types, type arguments, arrays, and type-parameter navigation. Hover currently
shows the type as written, rather than resolved type information.

See [TOML workspace documentation](../../docs/WORKSPACE.md) for the full format,
fallback behavior, and current limitations.
