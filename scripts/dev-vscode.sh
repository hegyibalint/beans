#!/usr/bin/env sh
# Build Beans and open its VS Code extension-development host on a Java project.
set -eu
cd "$(dirname "$0")/.."

PROJECT="${1:-examples/beans}"
case "$PROJECT" in
  /*) ;;
  *) PROJECT="$PWD/$PROJECT" ;;
esac

cargo build -p beans-lsp
if [ ! -d extensions/vscode/node_modules ]; then
  npm --prefix extensions/vscode ci
fi
npm --prefix extensions/vscode run compile

if [ "$PROJECT" = "$PWD/examples/beans" ]; then
  code --new-window --disable-extensions \
    --extensionDevelopmentPath="$PWD/extensions/vscode" \
    "$PROJECT" "$PROJECT/src/demo/Example.java"
else
  code --new-window --disable-extensions \
    --extensionDevelopmentPath="$PWD/extensions/vscode" "$PROJECT"
fi
