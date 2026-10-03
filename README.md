# Beans

> [!NOTE]
> Beans is in very early stages of development. Don't expect anything to work right now.
> The aim is currently to work out the fundamentals that can support the whole idea.

Beans is an experiment in building a shared semantic foundation for languages on the _JVM_, exposed through libraries, developer tools, and a _language server_.

Beans was created because we believe that there is value in separating your editors and tools from where semantic information comes from. The LSP is a great example of succeeding at this separation: whether you are using VS Code, Emacs, Helix, or your own editor, language servers make information equally accessible to all.

But there are already existing language server implementations serving each language, and they will probably do a better job, so why bother? The problem is that in an ecosystem like the JVM, interoperability is _everything_. However, sharing semantic models is not part of the LSP; coordinating multiple language servers must happen out of band.

The idea with Beans is to move one step up the abstraction ladder: what if we start looking at JVM languages as components using the abstraction of the JVM? By incorporating major JVM languages piece by piece, and using the JVM as a common denominator, we could exploit cross-language interoperability, just like, for example, IntelliJ IDEA does.

## Key features

### Native first

Surprisingly, Beans is not written in a JVM language, but in a native language, Rust[^1]. The JVM might be an excellent VM, but building tooling above it is a pain; startup performance is not great, and with language servers you are expecting abrupt starts and stops[^2]. Also, finding or provisioning a JVM is a complex process, which is much better handled in a sane programming language than in Bash. By going native, Beans could potentially be distributed and used as a single binary file[^3][^4].

That doesn't mean that the JVM is absent from Beans; different functionalities like project import still require a running JVM. What's different, though, is that these "sidecar" JVMs can be spawned and managed by a possibly shorter-lived, ephemeral language server.

### Startup performance optimized

Beans is designed to supply you with information _right now_, even if it's stale. Having a stale but consistent model serving code completion is better than having no code completion. Do you like to wait for minutes after a branch change, or after opening the IDE?

To support this, persistence is a key feature in Beans; with persisted models, the language server can start almost immediately. As imports and other side functionalities catch up with your project, your models will catch up with reality, but you can still start working right now.

### Reusability

Beans is not primarily a language server. It is a language processing and indexing engine that also implements an LSP as a facet. Beans is designed to be reusable as a library and embeddable in binaries. Tools using the library can share the persisted models. This could allow interesting use cases like building CLI tools that have an immediate snapshot of your project when you use them.

The idea is to allow a community to form around Beans by making it convenient for third-party tools or libraries to consume its knowledge.

### Debugging the language server

Diagnostic logs go to stderr, with a UTC timestamp and the emitting crate's name:

```text
2026-01-01T12:00:00.123Z [beans-workspace-toml] Loaded /project/beans.toml: units=2
```

The default level is `warn`. Set `RUST_LOG=beans=info` to include informational
messages from all Beans crates, or `RUST_LOG=beans=debug` for debug messages.
Crate-specific filters use Rust's underscore names, for example
`RUST_LOG=beans_workspace_toml=debug`. Existing `log::info!`, `log::debug!`, etc.
calls supply the crate name automatically; no per-call prefix is needed.

Set `BEANS_LSP_LOG_PATH=/path/to/beans-lsp.jsonl` in the **language server process's** environment, then restart the server. Beans appends one JSON object per line for every incoming LSP request/notification/response and every outgoing response. Each entry includes a Unix `time_ms`, `pid`, `direction`, and the decoded `message`, including request IDs, parameters, results, and errors. Leave the variable unset to disable logging. The parent directory must already exist; an unwritable path prevents the server from starting.

**The log contains document contents and potentially other private client data.** Keep it in a private location and remove or redact it before sharing. Logging does not write to stdout, which is reserved for the LSP transport.