# Playground

Run `scripts/dev-vscode.sh` from the repository root to open this folder with
the Beans development extension and `src/demo/Example.java`.

## Workspace configuration

[`beans.toml`](beans.toml) defines two project modules:

```text
demo → library
```

Demo's sources can resolve `library.Widget`; the library does not depend on
demo. Paths are relative to this directory. Each module has only `sources` and
`depends_on`; the optional `jdk_home` is shared by the entire project.

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
the project's `jdk_home`; unset or empty preserves the descriptor setting.
Relative paths resolve against this workspace directory. The selected JDK's
runtime artifacts are processed on Engine's event thread. Java 26 currently has
a known `cafebabe` decoder failure; for a working JDK indexing demo, use an older
installed mise JDK such as Temurin 11.

The SQLite database is stored at `.beans/index.sqlite` in this workspace. Startup
fails if it cannot be created. This directory is ignored by git. Hover and
Go to Definition run through Engine's SQLite-backed queries.

The development script enables `RUST_LOG=beans=debug`. Open **Output → Beans**
to see the server's stderr diagnostics:

```text
Loaded /…/examples/playground/beans.toml: modules=2
```

For a behavioral check, use **Go to Definition** on `Widget` in
`Widget imported;` in `Example.java`. It should lead to `src/library/Widget.java`.
The backend records module dependencies, but LSP queries currently use
unrestricted visibility. Removing `depends_on` does not yet restrict navigation.
Descriptor changes are not watched yet.

The other fields in `Example.java` demonstrate modeled type hovers, member
types, type arguments, arrays, and type-parameter navigation. Hover currently
shows the type as written, rather than resolved type information.

Without `beans.toml`, the server recursively discovers sources under the
workspace root. With a descriptor, it scans only the declared source roots.
