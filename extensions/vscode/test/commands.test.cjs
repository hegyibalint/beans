const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { createRequire } = require("node:module");
const test = require("node:test");
const vm = require("node:vm");

// Exercise our compiled extension against the installed language client's actual
// execute-command registration, without needing a graphical VS Code host.
test("Gradle import and beans+jvm documents work through VS Code registration", async () => {
  const registered = new Map();
  const providers = new Map();
  const requests = [];
  const shown = [];
  const stringUri = "beans+jvm:/java/lang/String.class";
  const nestedUri = "beans+jvm:/java/util/Map%24Entry.class";
  const listings = new Map([
    [stringUri, "package java.lang;\n\npublic class String {\n}\n"],
    [nestedUri, "package java.util;\n\npublic interface Map$Entry {\n}\n"],
  ]);
  const cancellationToken = { isCancellationRequested: false };
  const vscode = {
    commands: {
      registerCommand(id, callback) {
        if (registered.has(id)) throw new Error(`command '${id}' already exists`);
        registered.set(id, callback);
        return { dispose() { registered.delete(id); } };
      },
    },
    Uri: {
      from({ scheme, path }) {
        const encoded = path.split("/").map(encodeURIComponent).join("/");
        return { scheme, toString() { return `${scheme}:${encoded}`; } };
      },
    },
    workspace: {
      getConfiguration() { return { get() { return ""; } }; },
      registerTextDocumentContentProvider(scheme, provider) {
        providers.set(scheme, provider);
        return { dispose() { providers.delete(scheme); } };
      },
      async openTextDocument(uri) {
        const text = await providers.get(uri.scheme).provideTextDocumentContent(uri, cancellationToken);
        return { uri, text };
      },
    },
    window: {
      async showTextDocument(document) { shown.push(document); },
    },
  };
  const featurePath = require.resolve("vscode-languageclient/lib/common/executeCommand.js");
  const feature = { exports: {} };
  const realRequire = createRequire(featurePath);
  vm.runInNewContext(fs.readFileSync(featurePath, "utf8"), {
    exports: feature.exports,
    require(id) {
      if (id === "vscode") return vscode;
      // Capability construction is outside this registration test.
      if (id === "./features") return {};
      return realRequire(id);
    },
  }, { filename: featurePath });
  const { ExecuteCommandFeature } = feature.exports;
  const root = path.resolve(__dirname, "../../..");
  const commandsSource = fs.readFileSync(path.join(root, "crates/beans-lsp/src/server/commands.rs"), "utf8");
  const serverCommand = commandsSource.match(/const IMPORT_GRADLE: &str = "([^"]+)"/)[1];
  const manifest = JSON.parse(fs.readFileSync(path.resolve(__dirname, "../package.json"), "utf8"));
  assert.equal(serverCommand, "beans.import.gradle");
  assert.equal(manifest.contributes.commands[0].command, serverCommand);
  const previewCommand = "beans.preview.jvmString";
  assert.ok(manifest.contributes.commands.some(({ command }) => command === previewCommand));
  assert.ok(manifest.activationEvents.includes("onFileSystem:beans+jvm"));
  assert.equal(manifest.capabilities.untrustedWorkspaces.supported, false);
  let commandFeature;
  class LanguageClient {
    middleware = {};
    start() {
      return Promise.resolve().then(() => {
        commandFeature = new ExecuteCommandFeature(this);
        commandFeature.initialize({ executeCommandProvider: { commands: [serverCommand] } });
      });
    }
    async sendRequest(method, params, token) {
      requests.push({ method: method.method ?? method, params, token });
      if (method === "workspace/textDocumentContent") {
        if (!listings.has(params.uri)) throw new Error("JVM class is not indexed");
        return { text: listings.get(params.uri) };
      }
    }
    stop() { commandFeature.clear(); }
  }
  const extension = { exports: {} };
  const extensionPath = path.resolve(__dirname, "../out/extension.js");
  vm.runInNewContext(fs.readFileSync(extensionPath, "utf8"), {
    exports: extension.exports,
    process,
    require(id) {
      if (id === "vscode") return vscode;
      if (id === "vscode-languageclient/node") {
        return { LanguageClient, TransportKind: { stdio: 0 } };
      }
      return require(id);
    },
  }, { filename: extensionPath });
  const subscriptions = [];
  const starting = extension.exports.activate({
    asAbsolutePath(relative) { return path.resolve(__dirname, "..", relative); },
    subscriptions,
  });
  assert.equal(registered.size, 0);
  await starting;
  assert.equal(registered.size, 2);
  await registered.get(serverCommand)();
  assert.equal(requests[0].method, "workspace/executeCommand");
  assert.equal(requests[0].params.command, serverCommand);

  await registered.get(previewCommand)();
  assert.equal(requests.length, 2);
  assert.equal(requests[1].method, "workspace/textDocumentContent");
  assert.equal(requests[1].params.uri, stringUri);
  assert.equal(shown.length, 1);
  assert.equal(shown[0].uri.toString(), stringUri);
  assert.equal(shown[0].text, listings.get(stringUri));

  // A caller can open a different class directly, with no palette command.
  const jvmUri = (path) => vscode.Uri.from({ scheme: "beans+jvm", path });
  const nested = await vscode.workspace.openTextDocument(jvmUri("/java/util/Map$Entry.class"));
  assert.equal(nested.text, listings.get(nestedUri));
  assert.equal(requests[2].method, "workspace/textDocumentContent");
  assert.equal(requests[2].params.uri, nestedUri);
  assert.equal(requests[2].token, cancellationToken);
  await assert.rejects(vscode.workspace.openTextDocument(jvmUri("/Missing.class")), /not indexed/);

  await extension.exports.deactivate();
  subscriptions.forEach((subscription) => subscription.dispose());
  assert.equal(registered.size, 0);
  assert.equal(providers.size, 0);
});
