#!/usr/bin/env sh
# Build the server and client, then open the VS Code extension dev host on the sample.
set -e
cd "$(dirname "$0")/.."

cargo build -p lsp
npm --prefix extensions/vscode run compile

code --new-window --disable-extensions \
  --extensionDevelopmentPath="$PWD/extensions/vscode" \
  "$PWD/extensions/vscode/sample"
