# Beans

Beans is an experimental LSP for JVM languages. The realization is that LSPs cannot communicate between each other easily; a separate Java or Kotlin LSP would have a hard time sharing information about. This impacts cross language usability, which is key in the JVM ecosystem:
 - an Android application is often a mix of Kotlin, and Java
 - Groovy is still used for testing
 - Other languages like Scala and Clojure are great, because even if they are a different language, they can use the wider ecosystem

The idea of this project to grow a core LSP that could eventually support as many languages as we can.

Beans is unconventional, because it uses Rust to implement the core diagnostics engine. Java applications are slow and cumbersome to start. The idea is that with a Rust diagnostics engine—that is simple and fast to start—we can create a very snappy, fast to boot LSP.

## Key objectives

- *Startup speed*: Beans should be extremely fast to start, even if we have a temporal dip in correctness. It's better for the developer to have something immediately when they switch a branch, than wait 10 minutes for indexing.
- *Persistence*: Beans should be designed from the ground with persistence in mind.
- *Multi-faceted*: Beans could not only be an LSP; with fast startup, and database under its feet, we could make CLI tools serving multiple use-cases: linting, formatting, querying.

## Layout

The project is a primarily Cargo project with a parallel Gradle build embedded in the project. The project follows the large-scale project template:

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
 - `crates` contain majority of the implementation
   - Crate `core` defining global structs and helpers
   - Crate `lang-java` being the isolated Java language vertical
   - Crate `platform-jvm` being the isolated JVM platform vertical
   - Crate `platform-jvm` being the underlying common JVM platform vertical, used by all languages that run on the JVM
   - Crate `engine` as the frontend of the diagnostics engine, but not yet LSP specific
   - Crate `lsp` as the LSP frontend of the diagnostics engine; quite thin, as most of the work is done in `engine`
 - `sidecar` could contain a multi-project Java build implementing functionality over the JVM
   - Example project could be `integration-gradle`, `integration-maven`
 - `schema` could contain definitions between the `crates` and `sidecar`
   - Not decided yet what tech we could use here
 - `extensions` could contain IDE and code editor extensions.
   - Examples are extensions for VSCode, Zed, or vim

### Concepts

#### Revision

Files, and the environment changes; when they do, it's important that we don't apply already stale data. A solution to this problem is to register each model with a revision number. Revisions serve as a versioning mechanism, and allow us to detect when a model is stale.

The idea comes from database systems: MVCC (multi-version concurrency control) is a database design pattern that allows multiple versions of a data item to exist at the same time. This allows for concurrent reads and writes without locking, and ensures that readers always see a consistent snapshot of the data.

Without a revision, we run a chance that stale diagnostics could surface themselves. Take the following example

```
t=0ms   keystroke → world becomes R42
t=1ms   diagnostics for the open files start on worker threads, reading R42
t=10ms  another keystroke arrives → R43 wants to exist
t=31ms  the R42 diagnostic run would have finished
```

Without a revision, the second keystroke would mutate the state under the running R42 computation (it would read a mix of R42 and R43 that never existed), and the run's results would be published as if they described the latest state.

The revision is a global counter. When the environment changes, the revision is incremented. Models are stored with the revision they were written at, and requested _at_ a revision: a read sees the newest version not newer than the requested one.

A computation started at revision R sees the world as of R, even while newer revisions land. Its result can become _outdated_, but never _wrong_. Cancelling outdated work early is an optimization, not a correctness need.

#### Source

Source is an essential concept in Beans. A source is an atomic unit of information, that can be processed by the system:
 - *Source files*: the simplest formats of information, a singular source file like `.java`, `.kt`, `.scala`, `.groovy`, or `.clj`. While the files can be complex, and contain technically limitless amounts of definitions, they are still considered as a single, atomic unit of information.
 - *Class files*: compiled Java classes, stored in `.class` files.
 - *JARs*: technically a ZIP archive. JARs are tricky, because they can contain vastly different information: compiled Java classes, source files, resources, and other metadata.
 - *JMODs*: linked image format, a container of compiled Java classes and other metadata used by the JVM.
 - *Jimages*: the JVMs internal image format, a container of compiled Java classes and other metadata used by the JVM.

Some are simple: a source file is just a path pointing to a file on disk. But others are more complex: a JAR can contain multiple source files, and even other JARs. A JMOD can contain multiple JARs, and even other JMODs.

Sources can be tracked for change: a hash of the source file can be computed and stored.

#### Source model

The source model is the _processed_ representation of an atomic unit of _source_, ready for use by the rest of the system.

Source models can come in two flavors:
 - *Language specific models*: a model that is specific to a particular programming language. It's expected that language A will use language A's model. But it's not expected that B can use A's model.
 - *JVM models*: in order to solve the A to B compatibility problem, we use a common denominator model, the JVM model. The JVM model is a lossy, but common projection of the true source model. By using this model, a language's semantic engine has a chance to understand and work with other languages. For example, a Java language engine can understand and work, even if in a limited way, with a Kotlin class.

#### Scope

Let's consider a single opened file. This file can reference other files, and those files can reference other files, and so on. What does this file see?

In the JVM world, there are two answers to this question: the classpath, or the module path. In either case, a scope is a _selector_ that allows us to filter the symbols that are visible to a file. In practice, we will query registries and indices with a scope, and the registries will return only symbols that are visible to that scope.

## Processing

### Processing and analysing

When Beans receives a source, it will process it into a source model. The processing is done in multiple stages, and each stage can be run in parallel.

The first to phases are:
 - *Processing*: the source is processed into a source model.
 - *Projection*: the source model is projected into a JVM model.
 - *Analysing*: the source model is analysed, and diagnostics are produced.

Analysis strictly happens after _all_ processing is done. The idea here is that processing can span multiple files; think about indexing a lot of files after a synchronization. Until we didn't process all the files, we cannot meaningfully analyse them; what if we didn't yet process `Bar` used from `Foo`? The analysis would fail, and produce a false positive diagnostic.

Doing the analysis at the end allows us to make sure all information for that revision is available, and we can produce the most accurate diagnostics possible.

## Indexing

Indices are a key performance optimization for Beans. Indices are updated when processing and projection happens, making sure the analysis has fast access to the information it needs.

However, indices are not infallible. As files can come and go, index lines can become stale. This is fine; the registries are the source of truth, and the indices are just a cache.

Indices need maintenance for size and staleness, and Beans will have a background thread that will maintain the indices. The idea is very similar to a garbage collector, or database vacuuming: the background thread will scan the indices, and remove stale entries, and compact the indices to save space. One law the garbage collector has is that it will never remove an entry that is still reachable; we can only remove a version once a _newer version of the same key_ exists at or below the oldest revision that is still in use. After that, no read can reach the older one. If the surviving version is a deletion marker, the whole entry can go, and any index lines pointing at it with it.

## Diagnostics

One important thing is what pulls the diagnostics; this is very important, as it drives key decisions in the architecture. Beans is designed to pull diagnostics for open files only.

Wider-scoped, batched runs are still possible, but they are not the default. The idea is that the user is only interested in the files they are working on, and not the entire project.
