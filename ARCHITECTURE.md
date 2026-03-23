# Beans LSP — Architecture

## Crate Structure

```
beans-core            # Unified symbol model, symbol table, Language trait, resolution
beans-lang-java       # Java source → symbols (via tree-sitter-java), JavaLanguage impl
beans-lang-kotlin     # Kotlin source → symbols (planned, via tree-sitter-kotlin)
beans-jmod            # .class bytecode → symbols (planned, via cafebabe)
beans-lsp             # LSP server, orchestrates everything
beans-test-harness    # Fixture test framework (language-agnostic, no language deps)
beans-test-java       # Java spec tests (depends on beans-lang-java + harness)
beans-test-kotlin     # Kotlin spec tests (planned)
beans-test-interop    # Cross-language tests (planned, depends on all language crates)
```

**Production crates**: Each `beans-lang-*` crate implements the `Language` trait from `beans-core`, producing `Symbol` entries. `beans-jmod` does the same from bytecode. `beans-lsp` consumes them all through a unified symbol table.

**Test crates**: `beans-test-harness` provides the fixture framework with no language opinion. Per-language test crates (`beans-test-java`, etc.) wire in their language via a prelude and contain the spec tests. `beans-test-interop` (future) depends on all language crates to test cross-language scenarios.

---

## Symbol Model (`beans-core`)

The core abstraction is the **Symbol** — a unified representation of any named entity across all JVM languages. The model is designed around what the index needs to answer, not around OOP class hierarchies.

```
Symbol
  ├── id: SymbolId                  // arena index
  ├── fqn: String                   // "java.util.List", "com.app.MyService.doWork"
  ├── name: String                  // simple name: "List", "doWork"
  ├── kind: SymbolKind              // Class, Interface, Enum, Record, Protocol, Namespace, Function, Field, ...
  ├── location: Option<Location>    // source file + span (None for synthetic/bytecode)
  ├── modifiers: Vec<Modifier>      // public, static, abstract, final, ...
  ├── parent: Option<SymbolId>      // containing symbol (class for a method, package for a class)
  ├── children: Vec<SymbolId>       // members (methods, fields, inner classes)
  ├── relations: Vec<Relation>      // extends, implements, overrides, protocol-extends
  └── signature: Option<Signature>  // type-specific details (return type, parameters, generics, ...)
```

### SymbolKind

Exhaustive across all five target languages from day one. Adding a kind later is cheap (it's an enum variant), but designing the index queries around it matters.

```
SymbolKind:
  // Shared JVM
  Class, Interface, Enum, Record, Annotation,
  Method, Constructor, Field, Parameter,
  Package,

  // Kotlin-specific
  Object, CompanionObject, DataClass, SealedClass,

  // Scala-specific
  Trait, CaseClass, CaseObject,

  // Groovy-specific
  // (maps cleanly to Class/Interface — no unique kinds needed)

  // Clojure-specific
  Namespace, Function, Protocol, Multimethod, Defrecord, Deftype,
```

### Why Symbol, Not ClassDecl

A `ClassDecl` with methods and fields assumes OOP structure. This works for Java/Kotlin/Scala/Groovy but forces Clojure into a shape it doesn't fit. By modeling around Symbol:

- A Java class is a Symbol (kind: Class) with child Symbols (kind: Method, Field)
- A Clojure namespace is a Symbol (kind: Namespace) with child Symbols (kind: Function)
- The index doesn't care — it queries by FQN, kind, or relationship

Language parsers produce rich, language-specific ASTs internally but emit Symbols into the shared index.

---

## Language Trait

Each JVM language implements the `Language` trait (`beans-core/src/language.rs`). This is the contract between language-specific parsers and the rest of the system:

```rust
pub trait Language: Send + Sync {
    fn extensions(&self) -> &[&str];
    fn parse(&self, path: &Path, source: &str) -> Vec<Symbol>;
    fn extract_imports(&self, source: &str) -> Vec<Import>;
    fn extract_package(&self, source: &str) -> String;
    fn word_at_position(&self, source: &str, line: u32, col: u32) -> Option<String>;
}
```

The LSP server and test harness dispatch per file extension — a `.java` file is handled by `JavaLanguage`, a `.kt` file by `KotlinLanguage`, etc. This enables multi-language interop in both production and tests without language-specific branching in the core.

---

## Symbol Table

The symbol table is the hot path — every navigation request, completion, and diagnostic hits it. It's a **multi-indexed in-memory arena**.

### Storage

Symbols live in a flat `Vec<Symbol>` (arena). All indexes store `SymbolId` (a `usize` into the arena). This gives cache-friendly iteration and zero-cost cross-references.

### Indexes

| Index | Type | Serves |
|-------|------|--------|
| FQN → SymbolId | HashMap | go-to-definition, hover |
| Package → Vec\<SymbolId\> | HashMap | import completion |
| File → Vec\<SymbolId\> | HashMap | re-index on save, file outline |
| Kind → Vec\<SymbolId\> | HashMap | "find all interfaces", type hierarchy |
| Simple name → Vec\<SymbolId\> | HashMap | workspace symbol search, fuzzy find |
| Parent → Vec\<SymbolId\> | HashMap | member completion (`obj.`) |

### Incremental Updates

When a file is saved:
1. Look up all SymbolIds for that file (via File → Vec\<SymbolId\> index)
2. Remove those entries from all indexes
3. Re-parse the file
4. Insert new symbols and update all indexes

The rest of the workspace stays warm. This is the "location-based caching" — file path is the cache key.

---

## Parse Modes

### Full Parse
For the currently open file(s). Produces complete AST with:
- All symbols with source positions (line, column, span)
- Enough detail for diagnostics, hover, and inline hints

### Stub Parse
For all other workspace files. Lightweight structural summary:
- Type names, method signatures, field types
- No method bodies, no expressions, no control flow
- Enough to populate the symbol table for navigation and completion

### Bytecode Parse
For `.class` files from JMODs and dependencies:
- Same output as stub parse — symbols with signatures, no source positions
- `location` is `None` or points to a generated stub for display

---

## Data Flow

```
                     ┌─────────────────┐
  .java files ──────►│ beans-lang-java │──┐
                     └─────────────────┘  │
                     ┌─────────────────┐  │    ┌──────────────┐    ┌──────────┐
  .kt files ────────►│ beans-lang-kotlin│──┼───►│ Symbol Table │◄──►│ beans-lsp│
                     └─────────────────┘  │    └──────────────┘    └──────────┘
                     ┌─────────────────┐  │
  .class files ─────►│   beans-jmod   │──┘
                     └─────────────────┘
```

All parsers emit `Vec<Symbol>`. The symbol table ingests them and builds indexes. The LSP server queries the symbol table to serve requests.

---

## Stdlib / Dependency Resolution

For **go-to-definition on stdlib types** (e.g., clicking on `List`), there is no source file to jump to. The model is rendered into a stub `.java` representation on-the-fly for display in the editor. This is a **presentation concern**, not an indexing concern — the symbol table holds the same `Symbol` struct regardless of origin.
