#!/usr/bin/env sh
# Build the server and client, then open the VS Code extension dev host on a
# folder. Defaults to the workspace example; pass another one to override, e.g.
# `scripts/dev-vscode.sh extensions/vscode/sample`.
#
# The folder has to be the one holding beans.toml: the server takes the first
# workspace folder the client reports and looks for the descriptor there.
set -e
cd "$(dirname "$0")/.."

PROJECT="${1:-examples/beans}"

cargo build -p lsp
npm --prefix extensions/vscode run compile

# Point the server at a fresh JSONL trace and hand the same path to VS Code, so
# the child server inherits it. Truncated per launch; `tail -f` to watch it.
BEANS_TRACE="${TMPDIR:-/tmp}/beans-lsp.jsonl"
export BEANS_TRACE
: >"$BEANS_TRACE"
echo "LSP trace: $BEANS_TRACE"
echo "  watch with: tail -f $BEANS_TRACE"

code --new-window --disable-extensions \
  --extensionDevelopmentPath="$PWD/extensions/vscode" \
  "$PWD/$PROJECT"
