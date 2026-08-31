This is the new `v0.3` branch; this is a complete rewrite and reideation of `main`, i.e. `v0.2`.
You should take minimal inspiration from the code itself, and rather focus on what was working and what became hard to manage.
See `TODO.md` for places where we accured friction.

## References

Before writing language-specific code, all relevant specification pages should be read and understood.

For copyright reasons, we don't commit MD-ified versions of the specifications, but executing `scripts/specs/update.py` provides access to:
- `docs/lang-specs/jls/se26/`, containing the JLS 26 in Markdown
- `docs/lang-specs/jvms/se26/`, containing the JVMS 26 in Markdown

Use these docs to drive the implementation, and provide citations to _why_ something is happening.

# Code style

Comments should be used very sparingly; this is an experimental project, things move, and I would like to add most of the comments strenthening my understanding.
I am not a Rust expert; be very critical of my code and suggest better ways to do things. I want to learn idiomatic Rust and best practices. 
I want to avoid cargo culting, so please explain why a change is better than what I have.

## Testing

Read the dedicated `docs/TESTING.md` to learn more about our testing policy.

## TODOs

The project is not yet using GH Issues as development is mostly single person and local.
For simplicity, there is a `TODO.md` file.
Make sure the file is maintained after a feature there is developed.