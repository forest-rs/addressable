# Addressable tree runtime consumer slice

## Goal

Turn the first real Exedra consumer into subtraction by moving reusable rooted
tree resolution and query execution into a small `no_std + alloc`
`addressable_tree` crate. Exedra should retain its storage and domain policies
while deleting its custom path type, recursive path lookup, and consumer-local
assembly-query executor. Preserve the revision clock across extraction, and
make in-place commits the normal mutation path.

## Non-goals

- A compulsory storage engine or index.
- Generic endpoint value or mutation traits before a second domain proves them.
- Async execution, persistence, or a textual query language.

## Steps

1. Define a host-owned node projection trait and reusable tree runtime.
2. Prove exact, relative, pinned, cardinality, budget, deduplication, handle,
   suspend/resume, and in-place commit behavior in tests and rustdoc.
3. Make the Basilica assembly projection a second `TreeHost`; retain its
   dependency-specific evaluator only for graph and cross-view axes.
4. Replace Exedra's `InstancePath` machinery with structured Addressable exact
   addresses and a host-owned index.
5. Implement the small tree projection in `exedra_assembly`, retain its material
   explanation/edit policy, and remove Basilica-specific resolution/selection
   helpers.
6. Validate and submit the Addressable and Exedra changes as separate PRs.

## Risks

- A trait shaped too narrowly around Exedra. Prove the assembly seam against
  both Exedra and the Basilica reference domain while leaving Basilica's graph
  axes domain-owned.
- Runtime mutation bypassing revisions. Expose immutable host access, preserve
  the clock on extraction, and provide an in-place commit that advances once.
- Treating exact address text as domain storage. Hosts store structured
  `AbsoluteAddress` values; string forms remain serialization only.
