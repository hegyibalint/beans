import * as path from "path";
import { commands, ExtensionContext, Uri, window, workspace } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

const JVM_SCHEME = "beans+jvm";

let client: LanguageClient;

export async function activate(context: ExtensionContext): Promise<void> {
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

  await client.start();

  context.subscriptions.push(
    // vscode-languageclient 9 does not register LSP 3.18 content providers.
    workspace.registerTextDocumentContentProvider(JVM_SCHEME, {
      async provideTextDocumentContent(uri, token) {
        const { text } = await client.sendRequest<{ text: string }>(
          "workspace/textDocumentContent", { uri: uri.toString() }, token,
        );
        return text;
      },
    }),
    commands.registerCommand("beans.preview.jvmString", async () => {
      const uri = Uri.from({ scheme: JVM_SCHEME, path: "/java/lang/String.class" });
      const document = await workspace.openTextDocument(uri);
      await window.showTextDocument(document);
    }),
  );
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
