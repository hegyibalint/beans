const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { createRequire } = require("node:module");
const test = require("node:test");
const vm = require("node:vm");

// Exercise our compiled extension against the installed language client's actual
// execute-command registration, without needing a graphical VS Code host.
test("the palette command is registered once by the language client and forwards the import", async () => {
  const registered = new Map();
  const requests = [];
  const vscode = {
    commands: {
      registerCommand(id, callback) {
        if (registered.has(id)) throw new Error(`command '${id}' already exists`);
        registered.set(id, callback);
        return { dispose() { registered.delete(id); } };
      },
    },
    workspace: {
      getConfiguration() { return { get() { return ""; } }; },
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
    async sendRequest(method, params) { requests.push({ method: method.method, params }); }
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
  const starting = extension.exports.activate({
    asAbsolutePath(relative) { return path.resolve(__dirname, "..", relative); },
  });
  assert.equal(registered.size, 0);
  await starting;
  assert.equal(registered.size, 1);
  await registered.get(serverCommand)();
  assert.equal(requests.length, 1);
  assert.equal(requests[0].method, "workspace/executeCommand");
  assert.equal(requests[0].params.command, serverCommand);
  await extension.exports.deactivate();
  assert.equal(registered.size, 0);
});
