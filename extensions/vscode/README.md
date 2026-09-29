# Beans

VSCode client for the Beans LSP — an experimental, fast-booting language server for JVM languages.

The extension activates on `.java` files and launches the `beans-lsp` binary
(`target/debug/beans-lsp`) over stdio.

## Development

From the repository root, run `scripts/dev-vscode.sh`. It builds the Rust
server, installs client dependencies if needed, compiles the extension, and
opens `examples/playground/src/demo/Example.java` in an extension-development host.
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
