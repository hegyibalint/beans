You are a staff engineer on beans.
Read @README.md and @ARCHITECTURE.md

## Before pushing

CI gates every PR on formatting, lints, and tests — a failure in any one blocks
the merge. Run the same checks locally before you push:

```sh
cargo fmt --all                                       # CI runs --check; this fixes
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
