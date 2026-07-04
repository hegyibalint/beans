# Beans

Beans is an experimental LSP for JVM languages.
The realization is that LSPs cannot communicate between each other easily; a separate Java or Kotlin LSP would have a hard time sharing information about. This impacts cross language usability, which is key in the JVM ecosystem: 
 - an Android application is often a mix of Kotlin, and Java
 - Groovy is still used for testing
 - Other languages like Scala and Clojure are great, because even if they are a different language, they can use the wider ecosystem

 The idea of this project to grow a core LSP that could eventually support as many languages as we can.

 Beans is unconventional, because it uses Rust to implement the core diagnostics engine.
 Java applications are slow and cumbersome to start.
 The idea is that with a Rust diagnostics engine—that is simple and fast to start—we can create a very snappy, fast to boot LSP.

 ## Key objectives

 - *Startup speed*: Beans should be extremely fast to start, even if we have a temporal dip in correctness. It's better for the developer to have something immediately when they switch a branch, than wait 10 minutes for indexing.
 - *Persistence*: Beans should be designed from the ground with persistence in mind.
 - *Multi-faceted*: Beans could not only be an LSP; with fast startup, and database under its feet, we could make CLI tools serving multiple use-cases: linting, formatting, querying.

 ## Layout

 The project is a primarily Cargo project with a parallel Gradle build embedded in the project.
 The project follows the large-scale project template:

 ```
 beans/
 ├── Cargo.toml
 ├── crates/
 │   ├── ...
 ├── sidecar/
 │       ...
 ├── extensions/
 │       ...
 ├── schema/
 └── xtask/
 ```

 This project layout ensures good separation between all the different verticals:
  - `crates` could contain majority of the implementation
    - Example modules could be `storage`, `lsp` and `cli`, `platform-jvm` and language specific crates like `lang-java` or `lang-kotlin`
  - `sidecar` could contain a multi-project Java build implementing functionality over the JVM
    - Example project could be `integration-gradle`, `integration-maven`
  - `schema` could contain definitions between the `crates` and `sidecar`
    - Not decided yet what tech we could use here

### Concepts

#### Identity

One of our targets is to serialize and deserialize information, and their relationship as fast as possible.

One of the bigger challenge is the relationship aspect: we cannot serialize and deserialize plain references.
What we could do is to assign each of the objects a unique identifier, and serialize/deserialize references by their identifier.

We will call this `Id`. The benefit of beans is that this doesn't need to be complicated: a monotonically increasing integer is enough to uniquely identify each object.

#### Source

In Beans, `Source` could mean any _source_ of information. 
In the JVM ecosystem, you could have widely varied containers of informations:
 - *Source files*: the simplest formats of information, a singular source file like `.java`, `.kt`, `.scala`, `.groovy`, or `.clj`. While the files can be complex, and contain technically limiteless amounts of definitions, they are still considered as a single, atomic unit of information.
 - *JARs*: technically a ZIP archive. JARs are tricky, because they can contains vastly different informations: compiled Java classes, source files, resources, and other metadata. 
 - *JMODs*: the JVMs module format, a container of compiled Java classes and other metadata.
 - ???

 There are potential shortcuts here, when it comes to caching: if a JARs or a file's hash is the same as a cached one, we don't need to do anything. Also this opens a door to future concepts like remote caching of indices.

Source processing is pure; it doesn't need to access any other information than the source itself, and the result is the intermediate representation of the source.

#### Revision

Files, and the environment changes; when they do, it's important that we don't apply already stale data.
A solution to this problem is to assign each `Source` a revision number, and only apply changes from a newer revision.
Without a revision, we run a chance that stale diagnostics could surface themselves.
Take the following example

```
t=0ms   keystroke → world becomes R42
t=1ms   diagnostics for the open files start on worker threads, reading R42
t=10ms  another keystroke arrives → R43 wants to exist
t=31ms  the R42 diagnostic run would have finished
```

Without a revision, the second keystroke would mutate the state under the running R42 computation (it would read a mix of R42 and R43 that never existed), and the run's results would be published as if they described the latest state.

The revision is a global counter. 
When sources change, the revision is incremented.
Each source carries the revision number it was last updated with.
This means that on the source-level, revision numbers can have large gaps between them.

#### Scope

A scope defines a set of sources that another source can consume. To understand why this is necessary, let's consider a project with multiple project: if they don't depend on each other, each of them could define their own `Foo.java`, which would be completely valid. When we index these sources, all of these `Foo` classes end up in the index, and we need a way to distinguish between them: the _scope_ will help specify exactly which `Foo` class belongs to which project.

In the JVM, there are two vastly different mechanisms to define scopes:
 - *Classpath scopes*: a scope defined by the classpath. 
  - The classpath is quite simple: a list of directories and JARs.
 - *Module scopes*: a scope defined by the module system path
  - This is much more involved than the classpath scope; we need to take into account what `module-info.java` files define, and what dependencies they declare.

## Layers

Beans can be composed down to some major layers:
 - Translation
 - Projection
 - Storage and indexing
 - Semantic
 - Language server

### Translation

The first layer, handling incoming sources of data.
Data can come in various forms: source files, JARs, JMOD, etc.

The translation layer ingests these sources, if needed, breaks them down into atomic parts, and builds the intermediate representation needed by the rest of the system.

This intermediate representation is rich; the aim is that this model can support all the required diagnostics _of that language_. E.g. a Scala IR could be used to provide detailed diagnostics for Scala code.

The translation layer is pure: files in, IRs out. The translation layer doesn't require any state or side effects to operate.

### Projection

After we have the intermediate representation, the projection layer takes these rich models, and creates a common JVM IR that can be used by the rest of the system.

Languages interact with each other through the projection layer, which provides a common IR that can be used to reason about their code.

Just like the translation layer, the projection layer is pure too: it takes IRs in, and produces a JVM IR out.

### Storage and indexing

The storage layer is responsible for persisting the IRs, and offer various indexing capabilities.
This is a crucial layer, that makes or breaks our performance and speed goals.

The storage system would serve as a "lake" of symbols.
All IRs, regardless what scope they belong to (see the Scope concept above) would be dumped into this storage system.

Then, indices are built on top of the symbol storage system, to allow fast, scope-aware lookup and retrieval of symbols.

Storage by definition is not pure; it requires side effects to persist data to disk.

### Semantic

The semantic layer consumes the IRs, and builds high-level semantic models over them.
This is the layer that the language-server uses to provide diagnostics and code completion.

The semantic layer is by default pure, with an exception:
 - Semantic computations themselves are pure; they take the lake and indices as input and return a result.
 - To cut redundant work, computations are wrapped in a memoization cache. That cache is the layer's only mutable state.

### Language server

The language server layer is responsible for implementing the LSP protocol, and to communicate with the semantic layer.

The language server's job is quite thin and limited. 
Diagnostics, code navigation, quick actions, and other LSP features are handled by the semantic layer, as it has a better understanding of the code.

### Data Flow

Beans' data flow model should be similar to many other LSPs (rust-analyzer, IntelliJ, Roslyn, etc...).

There are two forces that play in here:
 - What happens when a file is opened
 - What happens when a file has changed

#### Opened Files

Diagnostics are only computed for files that are open in the editor.
For completeness, we need to index all sources in the workspace, otherwise the semantic layer cannot resolve references across files. 
But we don't need to go further than create and project the IRs for most of the sources.

#### Changed Files

As mentioned, we need to index all relevant files in the workspace to resolve references across files.
But the world moves under our feet: dependencies, and external libraries can change. Non open files can change as well.
This means that we need to listen, and partially re-index the workspace when files change.
