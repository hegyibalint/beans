# Beans hover playground

Open `src/demo/Example.java` using `scripts/dev-vscode.sh` from the repository root.

Hover over `Base`, `Outer`, `String`, `Inner`, `Integer`, or `int`. Beans
currently displays the full type as written (including type arguments), while
highlighting the identifier under the cursor. It does **not** resolve the type
or display documentation yet. Hovering over `class`,
`member`, dots, angle brackets, or array brackets should produce no Beans
hover. Editing and saving the file is not required: the server receives full
in-memory document changes.

This is a valid, standalone Java source file (JLS §4.5 permits nested
parameterized types). It does not require a build tool or dependency download.
