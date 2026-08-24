# Instructions for agents

Read `MANDATE.md`, `docs/ARCHITECTURE.md`, and `STATUS.md` before making
architectural or implementation changes.

This repository is intended to become a long-lived Rust foundation for the
forest-rs ecosystem. Treat the mature system described in the architecture as
the design target. Do not reduce the project to a string-path utility merely
because exact addresses are the first primitive needed by consumers.

## Local context

When the checkout is under `/Users/bruce/Development/forest-rs`, inspect the
applicable instructions, old forest-rs tenets, and current conventions in the
sibling repositories before scaffolding or changing CI. In particular, compare
representative current practice in `exedra`, `portolan`, `layerstack`,
`understory`, `overstory`, and `inkstone`. Sibling repositories are references;
do not modify them as part of Addressable work.

Repository-local and ancestor `AGENTS.md` instructions take precedence over
this file where their scope applies. Record significant architectural choices
and reversals in a decision log rather than allowing them to survive only in a
chat transcript.

## Engineering expectations

- Preserve strong Rust typing. Do not introduce a universal value enum into the
  ordinary typed API.
- Keep durable semantic identities and addresses distinct from arena slots,
  interned IDs, generational handles, and other runtime-local accelerators.
- Keep referent identity, occurrence identity, endpoint identity, and revision
  context distinguishable.
- Prefer `no_std` plus `alloc` for foundational crates where practical. Put
  genuinely `std`-dependent execution facilities behind honest boundaries.
- Treat textual syntax as parsing and serialization of structured data, not as
  the in-memory representation.
- Make cardinality, ordering, deduplication, traversal budgets, and cycle policy
  explicit in query APIs.
- Require preconditions for potentially stale addressed mutations. A pinned
  reference must never silently rebind.
- Favor executable semantic laws, property tests, conformance fixtures, fuzzing,
  and examples over claims that cannot be checked.
- Add crate boundaries only where they express a real dependency or portability
  boundary. Avoid both a monolith and a family of speculative empty crates.

## Delegated authority

Within this repository, exercise architectural judgment rather than waiting for
approval on every type or module name. It is acceptable to revise this initial
architecture when concrete implementation evidence demands it; explain the
reason and preserve the important semantic distinctions.

Stop for actions that are public, destructive, difficult to reverse, or outside
the repository's delegated scope: merging to `main`, publishing crates or
releases, changing licensing, changing sibling repositories, spending money,
or making commitments on behalf of the owner.

