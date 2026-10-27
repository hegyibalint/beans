# Documentation site

Install and activate [mise](https://mise.jdx.dev/getting-started.html), then run from this directory:

```sh
mise install node
npm ci
npm run dev
```

Node.js and its bundled npm are pinned in the root `mise.toml`.
You can also launch the dev server with the **Docs** configuration in VS Code
when the repository root is open.

`npm run build` creates the static site in `dist/`.
`npm run preview` serves that build locally.

Published pages live in `src/content/docs/`, and images live in `src/assets/`.
The other documents in this directory are not published by the site.
