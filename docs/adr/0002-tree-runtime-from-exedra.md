# ADR 0002: Extract a reusable rooted-tree runtime from Exedra evidence

- Status: accepted
- Date: 2026-08-27

## Context

The first real consumer, `exedra_assembly`, already owns a rooted instance
forest, stable occurrence paths, part referents, and runtime-local handles. A
consumer-local implementation of Addressable resolution and queries duplicated
the reference host's assembly orchestration while leaving Exedra's old path
resolution and selection helpers intact. That integration added vocabulary and
code but did not replace the narrower API it was meant to supersede.

The architecture anticipated specialized host evaluation and prohibited a
compulsory graph store. It also allowed a genuine consumer to reveal a shared
host seam. Rooted canonical-address trees are now such a seam: the host can own
storage and indexes while a reusable evaluator owns exact/relative/pinned
resolution and the common query policies.

## Decision

Add `addressable_tree`, an always `no_std + alloc` crate depending only on
`addressable`. It owns `TreeRuntime`, `TreeHost`, `TreeNode`, and `TreeAxis`.

`TreeHost` projects host-owned nodes by view, exact address, occurrence,
parent/child relationships, predicate matching, and optional runtime handles.
It does not prescribe storage, indexing, domain values, or mutation. The
runtime binds a host value to a `SpaceId` and revision. Hosts expose lazy node
iterators and may override referent occurrence lookup with an index. Referent
and occurrence identities are ordered so cycle detection and deduplication use
`BTreeSet` rather than quadratic vector scans. The runtime implements:

- exact and relative locator resolution;
- pinned staleness, rebinding, movement, and ambiguity;
- children, descendants, and parent queries;
- cardinality, ordering, occurrence/referent deduplication, cycle policy, and
  traversal budgets;
- validated revision-scoped runtime handles.

Cardinality shaping through `Measured` and budget charging through `QueryStats`
remain in `addressable`, because both tree and specialized graph evaluators use
those host-independent query laws.

The runtime exposes immutable host access and an infallible in-place commit
that advances its revision once. Extraction returns the revision with the host,
and `resume` restores that clock. Domain transactions remain responsible for
validating fallible preconditions before commit. A drop guard advances the
clock during unwinding as well, so a caught panic cannot expose a partially
mutated host at its old revision.

## Consequences

- Exedra can store `AbsoluteAddress<AssemblySpace>` directly, delete its custom
  path traversal, and implement only a small node projection.
- The Basilica assembly view is a second `TreeHost` and executes pure tree
  queries through the same runtime. Its dependency and cross-view axes retain
  their specialized evaluator.
- Non-tree relationship views retain specialized evaluators; this runtime does
  not force the reference dependency graph into a tree abstraction.
- The dependency-free `addressable` semantic nucleus remains unchanged.
- Watches can later recompute the same typed tree query through this runtime,
  but live-query scheduling is not pulled into this slice.
