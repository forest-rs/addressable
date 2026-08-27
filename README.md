# Addressable

Addressable is the typed substrate for locating, navigating, observing,
explaining, and safely modifying things in structured object spaces.

It preserves distinctions that string paths and runtime handles usually erase:

- a referent is the semantic thing;
- an occurrence is where that thing appears in a particular view;
- an endpoint is a typed facet on a located owner;
- a space-typed revision says which instance and state were resolved;
- a resolved handle is a revision-scoped runtime token, never durable identity.

The same arch referent can therefore appear as north and south assembly
occurrences without being duplicated. A caller can query both, deduplicate by
referent when appropriate, read and explain a typed load endpoint, apply a
guarded edit, and observe a coherent live delta.

## Workspace

| Crate | Boundary |
|---|---|
| `addressable` | Dependency-free `no_std + alloc` vocabulary, structured addresses, query IR, live deltas, guards, and correspondence |
| `addressable_tree` | Reusable `no_std + alloc` resolution and query execution over host-owned rooted trees |
| `addressable_reference` | `std` scanning basilica and catalog spaces exercising the complete lifecycle |
| `addressable_tooling` | Schema-backed erased adapter that delegates to the typed reference API |
| `addressable_tour` | Separate executable proof; no example-only dependencies enter production crates |

Dependencies flow in one direction:

```text
addressable <- addressable_tree <- addressable_reference <- addressable_tooling <- addressable_tour
```

## Typed use

```rust
use addressable::{
    CyclePolicy, Deduplication, Endpoint, Guard, Query, SpaceId, Transaction,
    VisitIdentity,
};
use addressable_reference::{
    Basilica, BasilicaAxis, BasilicaPredicate, BasilicaSpace, EditCapability,
    FeatureKind, Load, SetLoad,
};

let mut space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
let query = Query::many(space.root_locator())
    .traverse(BasilicaAxis::Descendants)
    .filter(BasilicaPredicate::Kind(FeatureKind::Arch))
    .filter(BasilicaPredicate::LoadAtLeast(100))
    .deduplicate(Deduplication::Occurrence)
    .cycles(CyclePolicy::SkipVisited(VisitIdentity::Occurrence));

let mut watch = space.watch(query.clone()).expect("watch starts");
let arch = space
    .query_many(&query)
    .expect("query succeeds")
    .items()[0]
    .clone();
let endpoint = Endpoint::new(arch.clone(), Load);
let explained = space.read_load(&endpoint).expect("load reads");
let edit = SetLoad::new(
    endpoint,
    80,
    Guard::new(
        *arch.referent(),
        space.revision(),
        *explained.value(),
        EditCapability::SetLoad,
    ),
);

let preview = space
    .transact(Transaction::dry_run(space.revision(), [edit.clone()]))
    .expect("dry run validates");
let applied = space
    .transact(Transaction::apply(space.revision(), [edit]))
    .expect("edit applies");
let delta = watch.poll(&space).expect("watch advances");

assert_eq!(preview.changes().len(), 1);
assert_eq!(applied.changes().len(), 1);
assert!(!delta.changes().is_empty());
```

The full tour also resolves exact, relative, and pinned locators; crosses
explicitly into a cyclic dependency view; replays the live delta; maps one arch
referent to two catalog results with evidence; and performs an equivalent
guarded operation through the dynamic schema boundary:

```sh
cargo run -p addressable_tour
```

The executable presents those transitions as five named chapters, so it can be
read from top to bottom or run as a narrated overview. Focused rustdoc examples
on the workflow types use the same call paths and run as doctests.

## Semantic contracts

- Text is parsed into validated segmented addresses. It is not the in-memory
  location representation.
- Query cardinality is visible in `One`, `Optional`, and `Many` query types.
  Their marker trait is sealed; ordering, deduplication, cycle behavior, and
  traversal budgets are explicit.
- Pinned resolution reports stale, moved, or rebound outcomes instead of
  silently accepting a different referent.
- Guarded transactions validate every operation before applying any operation.
- Replaying a query delta produces the same snapshot as full recomputation;
  another space or live-query stream is rejected atomically.
- Correspondence preserves one-to-many mappings and provenance, and composed
  mapping legs cannot disagree about their connecting source.
- Dynamic tooling recovers a declared schema and uses the same typed guarded
  operations as Rust callers.

Addressable does not own consumer world state, a universal graph or value enum,
one storage engine, domain composition rules, or a privileged agent mutation
path.

See [`MANDATE.md`](MANDATE.md) for the durable purpose,
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the mature design target,
[`docs/adr/0001-initial-workspace-and-vertical-slice.md`](docs/adr/0001-initial-workspace-and-vertical-slice.md)
for the initial crate decision,
[`docs/adr/0002-tree-runtime-from-exedra.md`](docs/adr/0002-tree-runtime-from-exedra.md)
for the consumer-derived tree runtime, and [`STATUS.md`](STATUS.md) for current
state.
See [`docs/MIGRATION.md`](docs/MIGRATION.md) when updating code written against
the earlier bootstrap draft.

Addressable is available under the terms of either the
[Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your
option.
