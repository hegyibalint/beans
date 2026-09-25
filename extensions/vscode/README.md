# Beans

VSCode client for the Beans LSP — an experimental, fast-booting language server for JVM languages.

The extension activates on `.java` files and launches the `beans-lsp` binary
(`target/debug/beans-lsp`) over stdio.

## Development

From the repository root, run `scripts/dev-vscode.sh`. It builds the Rust
server, installs client dependencies if needed, compiles the extension, and
opens `examples/hover-playground/src/demo/Showcase.java` in an extension-development host.
See the [playground README](../../examples/hover-playground/README.md) for hover targets.

Pass another project directory to open it instead:

```sh
scripts/dev-vscode.sh /path/to/project
```

Check **Output → Beans** and **Help → Toggle Developer Tools** if the extension
fails to start. The server currently shows the type as written in source on hover, not
resolved type information.
