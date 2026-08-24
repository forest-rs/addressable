# Migration from the bootstrap API

The initial slice deliberately tightened several public types before any crate
was published. Code written against the earlier bootstrap draft should make the
following mechanical changes.

## Space-typed revisions

`Revision` is now `Revision<S>` and contains its `SpaceId<S>`:

```rust
let initial = Revision::initial(space_id);
let later = Revision::new(space_id, 4);
```

Replace `Revision::INITIAL` with `Revision::initial(space_id)` and replace
`Revision::new(sequence)` with `Revision::new(space_id, sequence)`.
`Location::new` and `ResolvedHandle::new` no longer accept a separate space id;
they derive it from their revision. `Resolution`, `Guard`, and `Transaction`
gain the corresponding space marker parameter.

Pinned locator documents now serialize both the expected revision's runtime
space and sequence. Documents produced by the earlier unpublished bootstrap
format should be reparsed and re-emitted by an adapter that supplies the space
recorded in their embedded locator.

## Live-query scope

`QuerySnapshot`, `QueryDelta`, and `DeltaError` gain a space marker parameter.
Snapshot and delta constructors also take a host-assigned `LiveQueryId<S>`.
The reference `Basilica::watch` method now takes `&mut self` so it can allocate
that local id. Replay rejects mismatched live-query ids, mismatched space
revisions, and deltas that transition between spaces.

## Closed cardinality markers

`Query` cardinality must implement the sealed `Cardinality` trait. Use `One`,
`Optional`, or `Many`; custom marker types are no longer accepted. Generic code
can inspect `C::KIND` or `query.cardinality()`.

## Correspondence composition

The `Correspondence::compose` callback now returns an iterator of
`CorrespondenceTarget<U, Q>` values rather than another `Correspondence`:

```rust,ignore
first.compose(|source| {
    lookup(source).map(|target| [CorrespondenceTarget::new(target, evidence)])
})
```

The callback already receives the connecting source, so removing the redundant
second source makes disagreement between composition legs unrepresentable.

## Dynamic guarded requests

`DynamicTransaction` adds `selection_space`, and `DynamicGuard` adds
`expected_space`. Set both to the runtime space id from the dynamic locator.
The tooling adapter validates them before reconstructing typed revisions.
