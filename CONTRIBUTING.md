# Contributing

## Setup

Install stable Rust through rustup and install mise, then run from the repository root:

```sh
rustup component add rustfmt clippy
mise install
cargo test --workspace --locked
```

To enable the optional Git hooks, run once per clone:

```sh
git config --local core.hooksPath .githooks
```

This replaces any existing hooks directory configured for the clone and applies across its worktrees.
The pre-commit hook checks formatting in the working tree, not just staged changes.
If it fails, run `cargo fmt --all`, stage the fixes, and retry.
