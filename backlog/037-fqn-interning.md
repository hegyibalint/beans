# 037 — Reduce graph memory (string interning + layout)

Status: largely delivered; remaining slices parked (see below)
Priority: was high (pre-#012); the load-bearing slice is now done

## Results

Measured on gradle/master (10,187 files, 34 MB source; arena floor is
the exact metric, RSS via `ps` is noisier):

- `Interner` (per-workspace, `Arc<str>`, RefCell/single-threaded);
  `Fqn` re-backed by `Arc<str>`; `ParsedJavaFile::intern` re-keys
  plans at the serial integrate boundary. **581 → 539 MB.**
- `Location.file` re-backed by `Arc<Path>`, one buffer per file. **539
  → 493 MB.**
- `Interner::purge()` (GC: retain entries with `strong_count > 1`),
  wired into the LSP reindex. Bounds name growth across an editing
  session — the GC the `Arc` form allows and `Symbol(u32)` can't.
- **Boxing the fat payload variants** (the big one): `JvmMethodNode`
  (248 B) / `JavaMethodNode` (232 B) set the `NodePayload` width every
  one of 368k slots paid, though most slots are 64–80 B use-sites.
  Boxed all decl variants both sides; kept TypeUse/Parameter/Import
  inline. **Arena floor 91 → 29 MB (width 248 → 80 B); RSS ~465 →
  ~360 MB.** This is the slice that defuses the dependencies-make-it-GB
  concern: #012's ~700k JDK decl nodes cost 80 B/slot in the slab, not
  248 B.

Latencies unchanged throughout (~30µs/file diagnostics, ~2µs lookups);
parse throughput unchanged; integrate +~?? ms for the intern pass.

## Parked (deferred to #012)

- **Intern `name` / `TypeRef` strings (original "step 1").** Anatomy
  shows only 6 MB name text + 2.3 MB TypeRef text — ~13 MB ceiling
  against a blast radius of hundreds of sites (every `Display`,
  `name == "lit"` comparison, construction). Confirmed *no* effect on
  simple-name lookups: the index keys derive from the FQN slice, not
  the `name` field, and lookup hashes the same bytes regardless of
  representation. Its real value is #012-scale (JDK names like `get`/
  `toString` repeat tens of thousands of times) and it belongs in the
  bytecode loader's own integrate path.
- **Better than interning `name`: delete it for decls.** `java_header`
  builds `name` and `fqn` from the same token, so for declaration
  nodes `header.name` is fully redundant with `fqn.simple_name()` (a
  free slice of the already-interned FQN). Dropping it reclaims the
  6 MB + 24 B × ~665k inline with no interner. Use-site headers keep
  `name` (source token, no single FQN). Deferred — pair with #012.

Other un-measured remaining headroom: boxed RAII handles (~18 MB
measured, plus per-handle allocation overhead), registry hash-map
capacity, per-node child/handle Vec backings. These are allocation-
count costs, not string costs; revisit with a fresh anatomy if RSS
becomes a problem at dependency scale.

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
