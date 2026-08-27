# Project status

## Current state

The complete initial vertical slice landed on `main` on 2026-08-24. No crate
has been tagged or published.

The workspace contains five packages:

- `addressable`: dependency-free, always `no_std + alloc` semantic vocabulary;
- `addressable_tree`: reusable `no_std + alloc` resolution and typed query
  execution over host-owned rooted trees;
- `addressable_reference`: a `std` scanning basilica host and second catalog
  space;
- `addressable_tooling`: schema-backed dynamic adaptation through typed host
  calls;
- `addressable_tour`: a separate executable proof crate.

The crate decision, fences, invariants, and resolved bootstrap questions are in
[`docs/adr/0001-initial-workspace-and-vertical-slice.md`](docs/adr/0001-initial-workspace-and-vertical-slice.md).
The first consumer-derived execution boundary is in
[`docs/adr/0002-tree-runtime-from-exedra.md`](docs/adr/0002-tree-runtime-from-exedra.md).
The local forest-rs convention survey is in
[`docs/CONVENTIONS.md`](docs/CONVENTIONS.md).

## What is real

The reference slice exercises every lifecycle item required by the initial
architecture:

1. One arch referent has distinct north and south assembly occurrences.
2. Exact, relative, and pinned locators have canonical round-trip documents and
   resolve with rich outcomes; pinned rebinding has a regression test.
3. Typed queries cross explicitly between assembly and dependency views. Query
   cardinality is restricted to the sealed `One`, `Optional`, and `Many`
   markers. Ordering, deduplication, cycle policy, and four work budgets are
   explicit. Pure assembly queries execute through `addressable_tree`; the
   dependency graph retains its custom evaluator and contains a real cycle.
4. A typed `Load` endpoint returns effective value, alternatives, provenance,
   and a domain-owned winning reason.
5. A scanning watch maintains occurrence-identified query results under an
   explicit host-assigned live-query identity.
6. Atomic guarded transactions support dry-run and apply, require referent,
   revision, value, and capability preconditions, and return undo information.
7. Query deltas are replayed and compared with full recomputation. Replay
   rejects another space, live-query stream, or cross-space transition without
   partial effect.
8. One arch referent maps to two independently addressable catalog results while
   retaining correspondence evidence.
9. The dynamic adapter declares its view/facet/value schema, reconstructs typed
   endpoints and guards, and delegates to the same transaction method. A read
   returns the space and revision needed to form its guard without reaching
   around the adapter. Typed and dynamic operation equivalence is tested,
   including undo data.

The tour runs all nine points through public APIs:

```sh
cargo run -p addressable_tour --locked
```

The public rustdoc now describes the lifecycle of caller-created and
host-produced types, links each pivotal result to its producing and consuming
operations, and reserves doctests for real workflows and static laws. The tour
presents the same lifecycle as five narrated chapters.

## Deliberately simple execution

The contracts are real; the first execution is intentionally modest:

- resolution and query execution scan small vectors;
- watches recompute synchronously when polled;
- the reference mutation vocabulary currently authors one integer load facet;
- the first watcher supports occurrence identity only and rejects other live
  identities explicitly;
- the catalog correspondence is in-memory and deterministic;
- the tooling schema is reference-specific until a second real adapter proves a
  generic protocol;
- there is no textual query language, async runtime, persistent journal, or
  production index;
- `addressable_tree` deliberately covers rooted canonical-address trees only;
  other relationship views keep specialized evaluators;
- tree hosts yield projected nodes lazily, can index referent occurrences, and
  use ordered sets for cycle detection and deduplication;
- runtime extraction preserves the revision needed by `resume`, while
  validated in-place commits advance the clock without cloning a whole host.

These are replaceable host choices, not placeholders in the core semantic
types. No production or development dependencies were added.

## Validation evidence

The repository is green under the local definition of done:

```sh
typos
taplo fmt --check --diff
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --locked --no-deps --document-private-items
cargo check -p addressable -p addressable_tree --locked --target x86_64-unknown-none
cargo check -p addressable -p addressable_tree --locked --target wasm32-unknown-unknown
cargo +1.88 check --workspace --all-targets --all-features --locked
cargo +1.88 check -p addressable -p addressable_tree --locked --target x86_64-unknown-none
cargo run -p addressable_tour --locked
```

Results: 26 unit tests and 16 doctests pass; strict Clippy and warning-denied
rustdoc pass; native stable, Rust 1.88, bare-metal `no_std`, and WebAssembly
core checks pass; repository formatting, typo, SPDX-header, and whitespace
checks pass.

## Repository decisions retained by the owner

All packages remain `publish = false`. No merge, release, publication, or
sibling-repository edit was performed. The repository includes the standard
forest-rs Apache-2.0 and MIT license texts matching its workspace metadata.

## Next architectural evidence

The Exedra consumer exposed a genuine shared host seam, and the Basilica
assembly view now proves it against a second storage model. An outline/tree UI
is a credible next consumer: occurrence-aware rows, stable addresses, ordering,
budgets, and multiple views already fit. Pull live row deltas, lazy ranges, or
other execution machinery forward only when that consumer supplies concrete
behavior and deletion.
