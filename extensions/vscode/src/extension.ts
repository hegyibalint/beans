import * as path from "path";
import { ExtensionContext, workspace } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: ExtensionContext): Promise<void> {
  const command = context.asAbsolutePath(
    path.join("..", "..", "target", "debug", "beans-lsp"),
  );

  const javaHome = workspace.getConfiguration("beans").get<string>("javaHome");
  const serverOptions: ServerOptions = {
    command,
    transport: TransportKind.stdio,
    options: {
      env: { ...process.env, ...(javaHome ? { JAVA_HOME: javaHome } : {}) },
    },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "java" }],
    initializationOptions: {
      sidecarJar: context.asAbsolutePath(
        path.join("..", "..", "sidecar", "build", "libs", "beans-sidecar.jar"),
      ),
    },
  };

  client = new LanguageClient(
    "beans",
    "Beans",
    serverOptions,
    clientOptions,
  );

  return client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
