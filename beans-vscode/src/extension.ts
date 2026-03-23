import { workspace, ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext): void {
  const config = workspace.getConfiguration("beans");
  const configuredPath = config.get<string>("serverPath", "");

  const serverCommand = configuredPath || "beans-lsp";

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

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
