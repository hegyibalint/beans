#!/usr/bin/env sh
# Build Beans and open its VS Code extension-development host on a Java project.
set -eu
cd "$(dirname "$0")/.."

PROJECT="${1:-examples/playground}"
case "$PROJECT" in
  /*) ;;
  *) PROJECT="$PWD/$PROJECT" ;;
esac

cargo build -p beans-lsp
# Crate diagnostics go to stderr, displayed in VS Code's Output -> Beans.
export RUST_LOG="${RUST_LOG:-beans=debug}"
# target/ is ignored by git; the server log contains full document contents.
BEANS_LSP_LOG_PATH="${BEANS_LSP_LOG_PATH:-$PWD/target/beans-lsp.jsonl}"
export BEANS_LSP_LOG_PATH
printf 'Beans LSP traffic log: %s\n' "$BEANS_LSP_LOG_PATH"
if [ ! -d extensions/vscode/node_modules ]; then
  npm --prefix extensions/vscode ci
fi
npm --prefix extensions/vscode run compile

if [ "$PROJECT" = "$PWD/examples/playground" ]; then
  code --new-window --disable-extensions \
    --extensionDevelopmentPath="$PWD/extensions/vscode" \
    "$PROJECT" "$PROJECT/src/demo/Example.java"
else
  code --new-window --disable-extensions \
    --extensionDevelopmentPath="$PWD/extensions/vscode" "$PROJECT"
fi
