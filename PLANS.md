# Addressable 0.1 release evidence

## Goal

Prepare `addressable` and `addressable_tree` for a first `0.1.0` release only
after their public contracts have survived independent consumer use. Bring
forward API corrections that concrete consumers expose, then package the two
reusable crates without publishing, tagging, or landing consumer branches.

## Fence

Addressable owns durable addressing and shared interaction semantics; it
explicitly does not own consumer storage, UI state, presence reduction,
collaboration algorithms, or composition policy.

## Non-goals

- Publishing crates, creating a release, or merging consumer work.
- Generalizing the reference-specific tooling adapter without a second real
  schema-backed adapter.
- Adding a textual query language, universal evaluator, collaboration model,
  or speculative revision branches.
- Counting an adapter as evidence merely because its types compile.

## Evidence sequence

1. Audit the two proposed release crates for public invariants, documentation,
   package contents, and commitments that known near-term work would overturn.
2. Adapt Overstory's retained inspection tree to `addressable_tree`. Require the
   integration to replace traversal or selection glue and demonstrate rich
   resolution, pins, or budgeted queries against real snapshots.
3. Adapt a Layerstack composed stage without moving interned path, storage, or
   composition ownership. Require it to expose a genuine tree-host seam and to
   preserve Layerstack's path semantics.
4. Explore a Ruthere presence facet that carries current Addressable focus. Keep
   presence and collaboration separate, and retain this consumer only if it
   exercises locator or pin semantics rather than adding decorative wrapping.
5. Fold only consumer-earned corrections into Addressable. Record meaningful
   public semantic changes in an ADR and migration note.
6. Prepare release notes and package metadata for `addressable` and
   `addressable_tree`; keep the reference, tooling, and tour packages
   unpublished.
7. Run package inspection plus the full workspace gates, review the resulting
   API and consumer diffs, and open draft or review-ready PRs without merging.

## Evidence so far

- Overstory can borrow a retained inspection snapshot at its existing revision
  and resolve, query, budget, pin, and recover generational handles. The adapter
  is additive and does not replace Overstory's topology-aware live outline
  patch, so it is evidence for the core API rather than a consumer PR yet.
- Layerstack can borrow a composed `Stage` without leaking `PathId` into durable
  identity. It exposed the need for host-reported predicate work: composed
  field matching now charges one unit to find the field stack plus one per
  opinion.
- Ruthere can carry a typed pin as an application-owned presence facet through
  its real visibility, replacement, cursor, and expiry behavior. It exposed
  unsafe manual pin construction and the need to state that current `SpaceId`
  text is runtime-scoped. Addressable does not absorb Ruthere presence or any
  collaboration algorithm.

## Risks

- A UI-tree or USD-tree adapter may accidentally make Addressable own labels,
  storage, or domain traversal policy. Keep those decisions in the host.
- Runtime `SpaceId` values may be mistaken for durable cross-process space
  names when locators enter presence or tooling payloads. Either make that
  lifetime explicit or add a consumer-earned durable envelope before release.
- Existing reference and tooling crates demonstrate breadth but are not yet
  reusable production boundaries. Do not publish them or describe their
  reference-specific schemas as a stable generic protocol.
- Consumer branches can become dependency tangles. Use adapters and examples,
  preserve one-way dependencies, and avoid cross-consumer coupling.
