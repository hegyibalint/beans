import * as fs from "fs";
import * as path from "path";
import { workspace, window, ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

/**
 * Locate the beans-lsp binary, in priority order:
 * 1. The `beans.serverPath` setting, when set.
 * 2. A cargo-built binary next to this extension's source — covers the
 *    extension-development workflow where beans-vscode/ sits inside the
 *    beans repository (release preferred, debug as fallback).
 * 3. `beans-lsp` from PATH.
 */
function resolveServerCommand(context: ExtensionContext): string {
  const configured = workspace
    .getConfiguration("beans")
    .get<string>("serverPath", "");
  if (configured) {
    return configured;
  }

  const repoRoot = path.resolve(context.extensionPath, "..");
  for (const profile of ["release", "debug"]) {
    const candidate = path.join(repoRoot, "target", profile, "beans-lsp");
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return "beans-lsp";
}

export function activate(context: ExtensionContext): void {
  const serverCommand = resolveServerCommand(context);

  const serverOptions: ServerOptions = {
    command: serverCommand,
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "java" }],
  };

  client = new LanguageClient(
    "beans-lsp",
    "Beans LSP",
    serverOptions,
    clientOptions
  );

  client.start().catch((err) => {
    window.showErrorMessage(
      `Beans LSP failed to start (${serverCommand}): ${err}`
    );
  });
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
