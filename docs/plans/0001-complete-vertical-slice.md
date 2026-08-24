# Plan 0001: Complete vertical slice

## Goal

Deliver one executable, self-explaining object space that exercises exact,
relative, and pinned resolution; multi-view query semantics; typed endpoint
explanation; watch deltas; guarded dry-run and apply; one-to-many
correspondence; and an equivalent schema-backed dynamic operation.

## Non-goals

- A production graph database, query parser, async runtime, or incremental
  index.
- Consumer-specific Setout, Layerstack, UI, or retrieval adapters.
- Stable publication promises, release artifacts, or licensing changes.
- Performance claims before a consumer workload exists.

## Public call-site target

The API should make the semantic choices visible without exposing evaluator
plumbing:

```rust,ignore
let mut basilica = Basilica::new(SpaceId::new(1));
let root = Locator::exact(
    basilica.id(),
    BasilicaView::Assembly,
    AbsoluteAddress::parse("/basilica")?,
);

let query = Query::many(root)
    .traverse(BasilicaAxis::Descendants)
    .filter(BasilicaPredicate::LoadAtLeast(100))
    .deduplicate(Deduplication::Occurrence)
    .order(Ordering::Stable)
    .cycles(CyclePolicy::SkipVisited(VisitIdentity::Occurrence))
    .budget(TraversalBudget::new(8, 128, 32, 512));

let mut watch = basilica.watch(query.clone())?;
let arch = basilica.query_many(&query)?.items()[0].clone();
let endpoint = Endpoint::new(arch, Load);
let explained = basilica.read_load(&endpoint)?;

let edit = SetLoad::new(endpoint, 80, Guard::at(
    explained.subject(),
    basilica.revision(),
    explained.value(),
));
let preview = basilica.transact(Transaction::dry_run([edit.clone()]))?;
let applied = basilica.transact(Transaction::apply([edit]))?;
let delta = watch.poll(&basilica)?;
```

The dynamic adapter constructs the same typed endpoint, guard, and transaction
after schema validation.

## Steps

1. Scaffold the workspace, core, reference, tooling, and tour packages.
2. Implement names, addresses, locators, typed identities, locations,
   endpoints, resolution outcomes, query IR, explained values, deltas,
   transaction vocabulary, and correspondence.
3. Turn the architectural laws that are representable in the nucleus into unit
   tests.
4. Implement the basilica and catalog spaces with scanning resolution and query
   execution.
5. Implement typed load explanation, atomic guarded edits, watch recomputation,
   and delta generation.
6. Implement the schema-backed dynamic adapter solely through typed host calls.
7. Build the tour and README narrative from the same public contracts.
8. Add CI and run all local gates, including Rust 1.88 and `no_std` targets.

## Risks and controls

- **Generic API inflation:** keep host traits out until a second host proves
  them; make the semantic result types generic instead.
- **Mirage completeness:** every lifecycle claim must appear in the executable
  tour and an assertion-backed test.
- **Silent rebinding:** pinned resolution has dedicated rebound and moved
  outcomes plus a regression test.
- **Partial mutation:** validate every operation against one snapshot before
  applying any change; test a failing multi-operation transaction.
- **Delta drift:** replay every emitted delta and compare it with full query
  recomputation.
- **Dependency creep:** use only `core`, `alloc`, and `std` in this slice.

## Validation checklist

- [ ] `typos`
- [ ] `taplo fmt --check --diff`
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-features`
- [ ] warning-denied rustdoc
- [ ] `x86_64-unknown-none` core check
- [ ] `wasm32-unknown-unknown` core check
- [ ] Rust 1.88 workspace check
- [ ] executable tour run
