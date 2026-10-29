---
title: Editor Setup
description: Use the Beans language server in your editor.
---

First, follow the [installation instructions](/beans/getting-started/) to get
Beans and its dependencies.

## Visual Studio Code

The VS Code extension currently runs from the source checkout. Install VS Code's
`code` command and run from the repository root:

```sh
mise install
scripts/dev-vscode.sh
```

This builds Beans and opens the example Java project in a separate VS Code
window with the development extension loaded.

## Other editors

Beans communicates over standard input/output using the Language Server
Protocol. Point your editor's LSP client at the `beans-lsp` executable and
configure it for Java files. Editor-specific instructions will follow.
