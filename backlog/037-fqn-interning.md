# 037 — Intern FQN strings

Status: partially delivered (see Results)
Priority: high (sequence before #012 — JDK indexing multiplies the
string population ~10×)

## Results so far

- `Interner` (per-workspace, `Arc<str>`, RefCell/single-threaded) in
  beans-core; `Fqn` re-backed by `Arc<str>`; `ParsedJavaFile::intern`
  re-keys plans at the serial integrate boundary. **581 → 539 MB.**
- `Location.file` re-backed by `Arc<Path>` — producers mint one Arc
  per file, clone per location (no table needed). **539 → 493 MB.**
- Diagnostics/query latencies unchanged (28µs/file, ~2µs lookups);
  parse throughput unchanged; integrate +~45ms (the intern pass).

Remaining headroom (un-measured shares — the original prep step still
applies before going further): payload enum width × 368k arena slots,
`TypeRef` trees, per-header `name: String` allocations, boxed RAII
handles, hash-map capacity overhead.

## End-state: strong-form interning (`Symbol(u32)`)

The delivered slices are the weak form (shared buffers, content
equality). The recorded end-state — owner's design — is compiler-style
symbol interning: one central table owns all name bytes; `Fqn` becomes
a `Copy` `u32` into it; deduplication is definitional, keys hash
integers, `candidate_fqns` becomes `Vec<u32>`.

Costs to accept when delivering it (the regime):
- display requires table access everywhere (`as_str` not standalone) —
  prefer a global sharded table, rustc-style; permanence accepted
- the parallel parse phase mints symbols → the table must be `Sync`
  (the one concurrent structure in the core, ADR-0018 exception) or
  per-worker tables with a u32 remap at integrate
- all-or-nothing migration: every construction site goes through the
  table or doesn't compile (this is also the enforcement benefit)

Gate: profile at #012 scale (millions of JDK/dependency keys) showing
key-hashing/key-memory hot. The harness in
`beans/examples/index_workspace.rs` is the measurement vehicle.

## Motivation (measured)

gradle/master baseline (10,187 files, 34 MB source): **581 MB RSS** —
a ~17× blowup over source size, dominated by owned copies of
overlapping qualified-name text. Per declaration the dotted string is
owned by:

1. the Java payload header (`JavaDeclHeader.fqn`)
2. the `java.symbols` registry key (ADR-0007: identity lives in keys)
3. the `ProviderHandle` (RAII removal needs its key)
4. –6. the same three again for the JVM projection
7. `JavaUseHeader.candidate_fqns` — 4–6 candidate clones per type-use
8. every header's `name: String` — the last segment as its own
   allocation

Each copy is justified as a *field*; none is justified as a separate
*buffer*. (History: the simple-name index briefly added a 9th copy at
+138 MB; fixed by storing `NodeId`s — see `912b904`.)

## Approach

`Fqn` keeps its API, swaps its representation for a shared buffer —
`Arc<str>` via an intern table, or a symbol id into a per-`Beans`
string table (decide against measured shares, see prep). All copies
become pointer-width; equality/hash can become pointer/id-based as a
bonus.

Constraints:

- **Intern at integrate time.** Parsing is data-parallel with
  self-contained outputs (ADR-0005); a shared table must not be
  touched from rayon workers. The serial integrate step re-keys
  strings as payloads enter the graph — same place registration
  already happens.
- **Per-workspace table** (owned by `Beans`/registries side), not a
  global — multiple workspaces per process (ADR-0015 rationale).
- `name`/simple-name fields should become views into the interned
  buffer (or derived on demand via `Fqn::simple_name()`) rather than
  separate allocations.

## Prep (first step)

Extend `beans/examples/index_workspace.rs` to report string bytes by
category (payload fqns, payload names, candidate_fqns, registry keys,
handles) so the mechanism choice is made on shares, not guesses — and
so the win is measurable after.

## Acceptance

- gradle/master RSS reduced substantially (target: order of one-third
  off the 581 MB baseline; pin the real number after prep).
- No measurable regression in parse throughput (interning must not
  serialize the parallel phase) or lookup latency.
- Suite green; no public API change to `Fqn` consumers.
