---
status: pending
area: java
priority: medium
---

# Emit `Package` nodes from the Java walker

## Description

The Java walker reads a file's package declaration into
`ParseContext::package` (and surfaces it via `ParsedJavaFile::package`)
but never emits a `JvmNodePayload::Package` / `JavaNodePayload::Package`
into the plan. As a result `Registries::jvm.packages` is always empty
after parsing source files; only synthetic-from-elsewhere consumers
could populate it. The prototype walker had the same gap.

The visible symptom: `lookup_fqn` in
[`languages/java/resolve.rs`](../beans-core/src/languages/java/resolve.rs)
falls through to its `PackageKey` probe (lines ~166-169) on every call
and always misses because nothing ever provided. Spec tests pass today
because no test asserts a package-as-symbol resolution; that's
coincidence, not correctness.

## Context

Surfaced during code review of step 7 of the graph migration. The
walker rewrite landed without re-introducing this gap because it was
already absent in the prototype. Reviewer recommended deleting the
`make_package_payload` placeholder rather than carrying it as a doc
anchor — the clean state is "the gap is documented in this backlog
file, not in dead code."

## Acceptance criteria

- `parse_java_to_graph` emits exactly one `JvmNodePayload::Package`
  per file at the top of its plan, owner-keyed by the package FQN
  (empty string for the unnamed package).
- The package payload registers in
  [`Registries::jvm.packages`](../beans-core/src/jvm/registries.rs)
  via the existing `JvmPackageNode::on_created` impl, so
  `registries.jvm.packages.query(&PackageKey::new("com.example"))`
  returns the registered NodeId.
- Multiple files in the same package register one provider each (per
  ADR-0013 "registries store all providers; precedence is a resolution
  concern"); the language module's resolution rules pick one, and
  cross-file consumers see all of them.
- A spec test asserts that `lookup_fqn(&registries, "com.example")`
  resolves to a Package payload after a file in `com.example` is
  parsed. This is currently impossible without the fix.

## Implementation sketch

Inside `parse_java_to_graph`, after `ctx.package` is populated and
before the second pass over root children:

```rust
if !ctx.package.is_empty() {
    let pkg_fqn = Fqn::new(ctx.package.clone());
    ctx.plan.push(PendingNode {
        payload: NodePayload::Jvm(JvmNodePayload::Package(JvmPackageNode {
            header: JvmDeclHeader {
                name: ctx.package.split('.').next_back().unwrap_or("").to_string(),
                fqn: pkg_fqn,
                location: None,  // packages have no source location
                modifiers: Vec::new(),
                annotations: Vec::new(),
            },
        })),
        parent: None,
    });
}
```

The Java side may also want a `JavaNodePayload::Package` companion;
decide when implementing whether to emit a pair (mirroring every other
declaration) or just the JVM payload. A pair is more consistent with
ADR-0004's "each language node hard-links a JVM projection" but
requires a `JavaPackageNode::on_created` round-trip that's currently
fine.

## Notes

The walker rewrite in step 7 deliberately deleted the
`make_package_payload` placeholder rather than carrying it as a doc
anchor — keep the gap visible here, not in dead code.
