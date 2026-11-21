# Beans

VSCode client for the Beans LSP — an experimental, fast-booting language server
for JVM languages.

The extension activates when the workspace contains a `.java` file or
`beans.toml`, when a Java or `beans+jvm:` document is opened, or when a Beans
command is invoked. It launches the `beans-lsp` binary (`target/debug/beans-lsp`)
over stdio.
Activation starts Beans; Gradle import remains an explicit command.

## Gradle import

Run **Beans: Import Gradle project** from the Command Palette (Cmd+Shift+P on
macOS) for cross-project type navigation and dependency details in hover. Re-run
it after source-set changes or adding new files. Errors are logged in **Output →
Beans**. The editor remains responsive while the import runs.

The extension requires workspace trust because Gradle executes build scripts.
The sidecar needs Java 17 or newer. Set `beans.javaHome` to an installed JDK or
inherit `JAVA_HOME`. If the project's Gradle version cannot run on that JDK,
configure a compatible daemon JVM in the project. Automatic JDK installation is
disabled. Restart the extension host after changing the JDK.

## Development

From the repository root, run `scripts/dev-vscode.sh`. It builds the Rust server
with the `sidecar` feature, assembles the JVM sidecar, installs client
dependencies if needed, compiles the extension, and opens
`examples/playground/src/demo/Demo.java` in an extension-development host. The
script also enables LSP traffic logging at `target/beans-lsp.jsonl` (override
with `BEANS_LSP_LOG_PATH`). The log includes document contents; keep it private.
See the [playground README](../../examples/playground/README.md) for the current
demos.

Pass another project directory to open it instead:

```sh
scripts/dev-vscode.sh /path/to/project
```

### JVM class preview

Run **Beans: Preview java.lang.String** in the Command Palette to open
`beans+jvm:/java/lang/String.class` as a read-only listing. The extension can
also open `beans+jvm:/<internal-name>.class` directly through VS Code's
`workspace.openTextDocument` API. These are editor URIs, not browser links.

The language client registers the scheme advertised by Beans and fetches the
listing with LSP 3.18's `workspace/textDocumentContent`. The server looks up the
internal name in the index. It won't pick between multiple copies, and open
listings don't refresh during reindexing. If `String` isn't indexed yet, retry
after JDK indexing finishes. Go-to-definition doesn't emit these URIs yet.

Run `npm --prefix extensions/vscode test` from the repository root for client
command and virtual document regression checks.

Check **Output → Beans** and **Help → Toggle Developer Tools** if the extension
fails to start. Unsupported or unresolved type occurrences still show their
written spelling on hover. Method-parameter navigation and multiple-definition
results are not implemented yet.
