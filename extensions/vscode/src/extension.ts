import * as os from "os";
import * as path from "path";
import { ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  const command = context.asAbsolutePath(
    path.join("..", "..", "target", "debug", "beans-lsp"),
  );

  // The server writes a JSONL trace of raw protocol traffic when BEANS_TRACE
  // points at a file. Honor an override from the launch environment, otherwise
  // default to a predictable temp path so the trace always exists somewhere we
  // can find it.
  const trace =
    process.env.BEANS_TRACE ?? path.join(os.tmpdir(), "beans-lsp.jsonl");

  const serverOptions: ServerOptions = {
    command,
    transport: TransportKind.stdio,
    options: { env: { ...process.env, BEANS_TRACE: trace } },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "java" }],
  };

  client = new LanguageClient(
    "beans",
    "Beans",
    serverOptions,
    clientOptions,
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
