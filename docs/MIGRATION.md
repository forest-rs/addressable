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

`DynamicExplanation` now carries the `space` and `revision` at which its value
was observed. Dynamic callers should copy those fields into
`DynamicTransaction::{selection_space, selection_revision}` and
`DynamicGuard::{expected_space, expected_revision}` instead of reaching through
the adapter to typed host state.

## Tree runtime revisions and host iteration

`TreeRuntime::into_host` now returns `(Revision<S>, H)`. Restore that pair with
`TreeRuntime::resume(revision, host)`; do not call `new` with an id belonging to
an existing space instance. Apply already-validated mutations through
`TreeRuntime::commit` so the revision advances without cloning the whole host.

`TreeHost::nodes` and `TreeHost::children` now return iterators instead of
appending to output vectors. Referent and occurrence identities must implement
`Ord`, and hosts may override `occurrences_of` to use an index. A blanket
implementation makes `&H` a read-only tree host whenever `H` is one.

## Resolution and live change exhaustiveness

`Resolution` is now exhaustive and accepts an optional fifth type parameter
for the `CapabilityUnavailable` payload; it defaults to `String`. Match all
variants directly, and select a domain capability type when free-form text is
not appropriate.

`QueryChange::Rebound` was removed because generic snapshot differencing could
not produce it without typed referent evidence. Use `Updated` for observable
value changes; a future rebound event must carry enough typed identity for the
producer and replay logic to agree on its meaning.
