# java-inspect

A standalone utility for comparing tree-sitter's Java syntax tree with Beans'
`JavaFile` model. Run these commands from the workspace root.

## Inspect a Java file

```bash
cargo run -q -p java-inspect -- path/to/Example.java
```

For example:

```bash
cargo run -q -p java-inspect -- tools/java-inspect/examples/anonymous-class.java
```

The utility prints the Java source, tree-sitter's indented S-expression, and
the parsed `JavaFile` model:

```text
== source: tools/java-inspect/examples/anonymous-class.java ==
class Example {
    ...
}

== tree-sitter: tools/java-inspect/examples/anonymous-class.java ==
(program
  (class_declaration
    name: (identifier)
    body: (class_body
      ...
    )
  )
)

== JavaFile: tools/java-inspect/examples/anonymous-class.java ==
JavaFile {
  ...
  declarations:
    0: Type(
        ...
    )
  lexical_scopes:
    0: JavaLexicalScope {
        ...
    }
}
```

Declarations and lexical scopes are prefixed with their arena index so their
IDs can be followed through the model.

The utility includes `lang-java`'s model and parser source directly. It does
not compile the rest of the production library, so it remains usable while
resolution and other language services are in progress.

## Included examples

```bash
cargo run -q -p java-inspect -- tools/java-inspect/examples/nested-types.java
cargo run -q -p java-inspect -- tools/java-inspect/examples/object-creation.java
cargo run -q -p java-inspect -- tools/java-inspect/examples/anonymous-class.java
cargo run -q -p java-inspect -- tools/java-inspect/examples/lambda.java
cargo run -q -p java-inspect -- tools/java-inspect/examples/type-kinds.java
```

To inspect all examples:

```bash
for file in tools/java-inspect/examples/*.java; do
    cargo run -q -p java-inspect -- "$file"
done
```

For repeated use, build once and invoke the binary directly:

```bash
cargo build -q -p java-inspect
target/debug/java-inspect tools/java-inspect/examples/anonymous-class.java
```
