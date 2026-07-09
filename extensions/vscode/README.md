# Beans

VSCode client for the Beans LSP — an experimental, fast-booting language server for JVM languages.

The extension activates on `.java` files and launches the `beans-lsp` binary
(`target/debug/beans-lsp`) over stdio.

## Development

One-time setup: run `npm install` in this directory.

Then, from the repo root, `scripts/dev-vscode.sh` builds the server and client
and opens the extension dev host on the `sample/` folder.
