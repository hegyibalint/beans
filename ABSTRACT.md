The Language Server Protocol solved the editors-times-languages explosion; you can write one server, which can serve many editors. But LSPs are not without a tradeoff: language servers are not designed to communicate between each other. This is a strong limitation in the JVM ecosystem, where interoperability between languages are taken for granted.

Beans is a new langauge server aiming to plant seeds for a fast, native, cross-language server core implemented in Rust. In this talk we will see a working prototype serving cross-file diagnostics. We will take a brief walk through the architecture that makes it extensible to the rest of the JVM family: the per-language rich source models, the projections into the JVM models through which languages interoperate, the decisions taken to make serialization and startup as fast as possible, and the plan how to implement build tool synchronization by using a sidecar JVM process.

Developers using Beans should experience a cross-language server experience that is fast, responsive, and extensible to the whole JVM family of languages.

- Introduction to LSPs
- Problem statement, why Rust
- Demo
- Core models and JVM projections
