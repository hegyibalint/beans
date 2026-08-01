# Beans

VSCode client for the Beans LSP — an experimental, fast-booting language server for JVM languages.

The extension activates on `.java` files and launches the `beans-lsp` binary
(`target/debug/beans-lsp`) over stdio.

## Development

One-time setup: run `npm install` in this directory.

## Demo VSCode

The `scripts/dev-vscode.sh` script builds the server and client and opens the extension dev host on `examples/beans`, which is a simple project showing testcasing Beans. See its README for what to try.

The script can open any directory; just pass the dir after the script:

```sh
scripts/dev-vscode.sh extensions/vscode/sample
```
