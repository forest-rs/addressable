# Architectural nucleus

This document records the shape Addressable is trying to preserve before local
implementation work begins. It is a starting constitution, not a frozen API.

The initial crate boundary and complete executable slice are now implemented as
recorded in
[`adr/0001-initial-workspace-and-vertical-slice.md`](adr/0001-initial-workspace-and-vertical-slice.md).
The mature architecture below remains the design target; the scanning host is a
conformance execution, not a reduction of the mandate to its first evaluator.

## 1. Vocabulary

### Address space

An address space defines the roots, identity types, views, schemas, revisions,
resolution behavior, and supported navigation relationships of one host-owned
world. An address is never meaningful without its space.

Type-level space markers should prevent accidental interchange between domains
where possible. Runtime space identity is still needed when multiple instances
of one typed space coexist.

### Referent and occurrence

A referent is the semantic thing. An occurrence is one contextual appearance of
that thing reached through a particular parent or incoming edge in a particular
view and revision.

Equality and deduplication must state which one they mean. Two occurrences may
share a referent. The same locator text in two spaces or revisions may not denote
the same referent.

### Location, address, locator, and query

- A `Location` is resolved contextual occurrence information.
- An exact `Address` is canonical, singular, durable data when its domain can
  provide such a form.
- A `Locator` is a resolution recipe. It may be relative, policy-bearing, or
  capable of reporting more than simple presence or absence.
- A `Query` is an executable expression that can select zero or more located
  results. It is not generally valid as a key.
- A `Pinned` reference combines a locator with an expected identity, revision,
  or fingerprint and must report rebinding rather than silently accepting it.

### Endpoint and edge

An endpoint combines a located owner with a typed facet such as a property,
attribute, port, event, or field. Reading, watching, explaining, and editing
operate on endpoints without requiring the address serialization to encode
value-resolution policy.

Relationships may be first-class located values. Setout relations and ports,
Layerstack arcs, and Overstory bindings all carry semantics that disappear if
represented only as implicit traversal.

### Resolved handle

Arena indices, interned path IDs, generational element IDs, dense graph slots,
and registry-local property IDs are efficient resolved capabilities. They
remain host-owned and may be cached in located values, but are not serialized as
durable world identity unless their host explicitly guarantees it.

## 2. Multiple views

One address space may expose several named views over overlapping referents.

Examples include:

| Domain | Views |
|---|---|
| Setout | namespace, dependency, relation participation, claim provenance |
| Layerstack | composed stage, authored specs, composition arcs, opinions |
| Overstory | logical tree, template parts, presentation tree, bindings, accessibility |
| Portolan | host subject space, retrieval projections, live result occurrences |

Only some views form rooted trees with canonical addresses. Other views are
relations traversed by explicit typed axes. Changing view is an operation, not
an undocumented reinterpretation of `/`.

## 3. Resolution

Resolution should preserve evidence and distinguish outcomes such as:

- resolved as expected;
- absent;
- malformed or unsupported locator;
- ambiguous;
- stale revision;
- locator now denotes another referent;
- expected referent moved and was found elsewhere;
- partially resolved;
- view or capability unavailable;
- traversal or work budget exceeded.

Callers choose rebinding policy explicitly. Convenient APIs may collapse rich
outcomes only where doing so is safe and obvious.

## 4. Query model

Queries are typed ASTs before they are strings. A textual language, if added,
parses into the same representation used by Rust builders, agents, UI tools,
and serializers.

Domains supply axes and node or endpoint tests. Shared query semantics cover:

- result kind and cardinality (`Exact`, `Optional`, `Many`, or equivalent);
- occurrence versus referent deduplication;
- stable semantic order, traversal order, document order, or explicitly
  unordered results;
- whether edges, occurrences, referents, endpoints, or values are returned;
- cycle behavior and visitation identity;
- depth, node, result, and general work budgets;
- diagnostic plans and explanations.

Shared IR does not imply one evaluator. Setout may execute over dense slots,
Layerstack over namespace and composition indexes, Overstory over retained tree
indexes and binding tables, and Portolan through its retrieval pipeline.

## 5. Values, provenance, and explanation

An endpoint identifies what can be read; a value view specifies what is being
asked for. Candidate views include effective, local, authored, default, or all
opinions, but each domain owns the valid set and its precedence rules.

The common protocol should make it possible for tooling to ask:

- What is the effective value?
- What alternatives or opinions contributed?
- Why did this one win?
- Which source, rule, claim, layer, style, animation, or binding supplied it?
- What would be affected by changing it?

Explanation is structured data with addressable subjects and provenance, not
only formatted prose.

## 6. Live results

A live query maintains located results across revisions and emits deltas such
as additions, removals, updates, moves, and rebindings. Delta semantics must say
whether identity is by occurrence, referent, or result-entry identity.

Space identity and live-query identity are independent replay preconditions. A
numerically equal revision in another space is not the same revision, and two
subscriptions in one space are not interchangeable merely because their
current entries happen to match.

Important law: applying a coherent delta stream to the previous result set must
produce the same observable result as recomputing the query at the new revision.

Hosts may implement incrementality differently. Addressable standardizes result
and delta meaning rather than forcing one invalidation engine or async runtime.

## 7. Guarded edits

Addressed mutation uses typed operations and explicit preconditions:

- expected referent or endpoint identity;
- expected revision or value fingerprint;
- required capability;
- cardinality requirement;
- optional dry-run and impact analysis.

Transactions report applied changes, conflicts, rebases, and undo information.
Bulk query-targeted edits must make their selection snapshot and failure policy
explicit. No privileged agent mutation path bypasses normal invariants,
history, invalidation, or explanation.

## 8. Correspondence between spaces

Compilation, composition, retrieval, and presentation create partial mappings
between address spaces. These correspondences may be zero-to-one, one-to-many,
many-to-one, revision-sensitive, or lossy.

They should be first-class and explainable rather than encoded as matching
strings. A useful end-to-end chain might be:

```text
Layerstack authored spec
    -> composed stage endpoint
    -> Setout quantity and selected claim
    -> Exedra assembly or geometry occurrence
    -> Overstory property or presentation fragment
    -> Portolan live result and affordance
```

Composition of correspondences must preserve ambiguity and provenance, and
each second mapping leg must begin at the target produced by the first leg.

## 9. Typed and dynamic boundaries

Ordinary Rust users should see typed nodes, axes, endpoints, values, and
operations. Erasure belongs at boundaries that genuinely require open-world
behavior: inspectors, persisted query documents, scripting, plugin protocols,
and agent tool schemas.

The dynamic layer should be schema-backed and capable of recovering type and
capability information. It must not force the typed core to store every value in
one universal enum.

## 10. Laws worth making executable

The initial implementation should turn these into tests or conformance cases:

1. Parsing and formatting canonical exact addresses round-trip.
2. Normalization is idempotent.
3. Joining and relativizing paths obey their documented inverse laws.
4. A pinned locator never silently resolves to a different referent.
5. Occurrence equality does not imply or erase referent equality.
6. Query ordering and deduplication are deterministic when requested.
7. Traversal terminates under declared cycle and budget policy.
8. Live delta replay agrees with full recomputation.
9. Failed guarded transactions have no partial observable effect.
10. Correspondence composition preserves ambiguity and provenance.
11. Typed and dynamic execution agree for representable queries and operations.
12. Durable serialization never leaks runtime-local slots accidentally.

Property tests and fuzzing should target parsing, normalization, rebasing,
query guards, delta application, and transaction preconditions.

## 11. Packaging posture

The repository may become a small workspace, but crate boundaries should follow
dependency and portability boundaries rather than roadmap years. A plausible
shape to evaluate locally is:

- `addressable`: `no_std` plus `alloc` vocabulary, exact addressing, resolution
  contracts, typed query IR, and semantic result types;
- a `std` reference/runtime crate for in-memory indexes, watches, transactions,
  and conformance fixtures;
- an erased/schema tooling crate only when a real inspector or agent adapter
  demonstrates the boundary;
- consumer adapters living with the consumer unless a dependency-neutral
  integration crate is clearly warranted.

This is not yet a decision. Inspect the old forest-rs tenets and current sibling
practice before fixing the workspace shape.

## 12. Initial complete vertical slice

The first executable object space should be deliberately small but exercise the
whole lifecycle:

1. construct referents shared across multiple occurrences;
2. serialize and resolve exact, relative, and pinned locators;
3. query through at least two named views with explicit cycle and dedup policy;
4. read and explain a typed endpoint;
5. watch a query;
6. apply a guarded transaction;
7. observe coherent deltas;
8. cross a correspondence into a second small space;
9. perform an equivalent operation through the dynamic tooling boundary.

That prevents later features from discovering that the foundational identity
model was too small while keeping the first implementation finite.
