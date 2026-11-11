# Beans

VSCode client for the Beans LSP — an experimental, fast-booting language server for JVM languages.

The extension activates when the workspace contains a `.java` file or
`beans.toml`, when a Java file is opened, or when its import command is invoked.
It launches the `beans-lsp` binary (`target/debug/beans-lsp`) over stdio.
Activation starts Beans; Gradle import remains an explicit command.

## Gradle import

Run **Beans: Import Gradle project** from the Command Palette (Cmd+Shift+P on
macOS). The palette entry and server capability share the ID `beans.import.gradle`;
the language client registers its forwarding handler automatically. There is no
manual command wrapper or custom progress/success popup. Errors are logged in
**Output → Beans**. Other language requests continue while the import runs.
The extension requires workspace trust because Gradle executes build scripts.

The command registers compilation scopes only; it does not index dependencies.
Re-run it after source-set changes or adding new files. Set `beans.javaHome` to
an installed JDK compatible with the build, or inherit `JAVA_HOME`. Automatic JDK
installation is disabled. Restart the extension host after changing the JDK.

Run `npm --prefix extensions/vscode test` from the repository root for client
command regression checks. They exercise the installed language client's
execute-command registration and ensure the palette and server IDs agree.

## Development

From the repository root, run `scripts/dev-vscode.sh`. It builds the Rust
server with the `sidecar` feature, assembles the JVM sidecar, installs client
dependencies if needed, compiles the extension, and
opens `examples/playground/src/demo/Demo.java` in an extension-development host.
The script also enables LSP traffic logging at `target/beans-lsp.jsonl` (override
with `BEANS_LSP_LOG_PATH`). The log includes document contents; keep it private.
See the [playground README](../../examples/playground/README.md) for the current demos.

Pass another project directory to open it instead:

```sh
scripts/dev-vscode.sh /path/to/project
```

Check **Output → Beans** and **Help → Toggle Developer Tools** if the extension
fails to start. The server currently shows the type as written in source on hover, not
resolved type information.
