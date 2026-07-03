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
    - Example modules could be `storage`, `lsp` and `cli`
  - `sidecar` could contain a multi-project Java build implementing functionality over the JVM
    - Example project could be `integration-gradle`, `integration-maven`
  - `schema` could contain definitions between the `crates` and `sidecar`
    - Not decided yet what tech we could use here

## Architecture

Beans have the following layers:
 - *Ingestion layer*: Handles incoming data; this is a simple sounding, but fat layer. Whatever comes in (source file, JARs, JMOD, etc.) we need to break it down to atomic parts, and build a semantic, intermediate representation.
 - *Storage layer*: The storage layer takes care of persisting, with a stable ID, the incoming intermediate representations.
 - *Indexing layer*: Diagnostics need access information at lightning speed. To serve this use-case, we need to create queriable "lake" of symbols. This is one of the most important layers in Beans: it's performance can make or break the key objectives.
 - *Diagnostics layer*: the most complicated, yet distributed layer. The diagnostics layer takes care of understanding the intermediate representations, and help create the functionality required by the LSP and CLI functionalities.

### Model concepts

To move ahead, we need to lay down also some fundamental concepts.

#### Identity

One of our targets is to serialize and deserialize information, and their relationship as fast as possible.

One of the bigger challenge is the relationship aspect: we cannot serialize and deserialize plain references.
What we could do is to assign each of the objects a unique identifier, and serialize/deserialize references by their identifier.

We will call this `Id`. The benefit of beans is that this doesn't need to be complicated: a monotonically increasing integer is enough to uniquely identify each object.

#### Source

In Beans, `Source` could mean any _source_ of information. 
In the JVM ecosystem, you could have widely varied containers of informations:
 - *Source files*: the simplest formats of information, a singular source file like `.java`, `.kt`, `.scala`, `.groovy`, or `.clj`. Meanwhile the files can be complex, and contain technically limiteless amounts of definitions, they are still considered as a single, atomic unit of information.
 - *JARs*: technically a ZIP archive. JARs are tricky, because they can contains vastly different informations: compiled Java classes, source files, resources, and other metadata. 
 - *JMODs*: the JVMs module format, a container of compiled Java classes and other metadata.
 - ???

 There are potential shortcuts here, when it comes to caching: if a JARs or a file's hash is the same as a cached one, we don't need to do anything. Also this opens a door to future concepts like remote caching of indices.

Source processing is pure; it doesn't need to access any other information than the source itself, and the result is the intermediate representation of the source.

#### Indexes

Indexes are providing efficient access to certain key-value pairs.
We are going to go into much more detail about what indexes can store, but what's important is:
 - Indexes

#### Scope

A scope defines a set of sources that another source can consume. To understand why this is necessary, let's consider a project with multiple project: if they don't depend on each other, each of them could define their own `Foo.java`, which would be completely valid. When we index these sources, all of these `Foo` classes end up in the index, and we need a way to distinguish between them: the _scope_ will help specify exactly which `Foo` class belongs to which project.

In the JVM, there are two vastly different mechanisms to define scopes:
 - *Classpath scopes*: a scope defined by the classpath. 
  - The classpath is quite simple: a list of directories and JARs.
 - *Module scopes*: a scope defined by the module system path
  - This is much more involved than the classpath scope; we need to take into account what `module-info.java` files define, and what dependencies they declare.

#### Diagnostics

Diagnostics are where the magic happens.
We anticipate that diagnostics can get complex, and they will need to get various kinds of information from the index.

DX is key here: meanwhile we will have a handful of indices, we will have hundreds if not thousands of diagnostics; we should make the proper tradeoffs to make sure that diagnostics are performant yet comfortable to write.


### Data flow

The two ends of the system stand in a push and pull relationship:
 - File changes introduce a push: their intermediate representations are pushed to the index.
 - Diagnostics are tricky: they pull, but needs to be susceptible to push
  - When a diagnostics first fires, it will need to pull data from the indices
  - Upon conditions changing, the diagnostics will need to accept pushed data from the index
