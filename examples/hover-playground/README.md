# Beans hover playground

From the repository root, run `scripts/dev-vscode.sh` to build Beans and open
this folder and `src/demo/Showcase.java` in a VS Code Extension Development Host.
You need VS Code (`code` on PATH), Rust/Cargo and Node.js/npm. Alternatively,
open this folder in the host yourself after building `beans-lsp` and compiling
the extension (see [`extensions/vscode/README.md`](../../extensions/vscode/README.md)).

Try hovering over these *type uses* in `Showcase.java`:

- `Labelled` in the type parameter bound, and `Box` and `String` after `extends`;
- `Box`, `String`, `Slot` and `Integer` in the `nested` field type;
- `Box` and `Number` in the wildcard field type;
- `T` in `T label`, and `int` in `int[] counts`.

Beans currently displays the type **as written**, not the resolved declaration or
documentation. For instance, hovering `Slot` shows `Box<String>.Slot<Integer>`;
hovering `Integer` shows `Integer`. The hover range highlights just the type
identifier under the cursor. Hovering `class`, field names, dots, `?`, angle
or array brackets should not produce a Beans hover. Changes to the open file
are sent to the server without saving.

This is a standalone Java source file; it has no build-tool dependencies. The
nested parameterized type and wildcard are legal Java type forms (JLS §4.5 and
§4.5.1); `int[]` is an array type (JLS §10.1). It uses standard JDK names but
Beans does not yet resolve them.
