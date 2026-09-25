# Beans playground

A small, standalone Java project for trying Beans in VS Code. Keep new
examples here as Beans grows: add Java files under `src/demo/` (or another
package directory under `src/`) and describe what to try below. No Maven,
Gradle or dependency download is needed.

From the repository root, run `scripts/dev-vscode.sh`. It builds the language
server, compiles the VS Code extension and opens this folder with
`src/demo/Example.java` in an Extension Development Host. You need VS Code
(`code` on PATH), Rust/Cargo and Node.js/npm. See
[`extensions/vscode/README.md`](../../extensions/vscode/README.md) for setup and
troubleshooting. Edit the files in the host; you do not need to save to see
updated hover content.

## Current demo: type hovers

In `src/demo/Example.java`, hover over these *type uses*:

- `Labelled` in the type parameter bound, and `Box` and `String` after `extends`;
- `Box`, `String`, `Slot` and `Integer` in the `nested` field type;
- `Box` and `Number` in the wildcard field type;
- `T` in `T label`, and `int` in `int[] counts`.

Beans currently shows types **as written**, not resolved declarations or
documentation. Hovering `Slot` shows `Box<String>.Slot<Integer>`; hovering
`Integer` shows `Integer`. Only the type identifier under the cursor is
highlighted. Hovering `class`, field names, dots, `?`, angle or array brackets
should not produce a Beans hover.

The nested parameterized type and wildcard are legal Java type forms (JLS §4.5
and §4.5.1); `int[]` is an array type (JLS §10.1). The JDK types in this
example do not require extra dependencies; Beans does not yet resolve them.
