# Addressable mandate

## Purpose

Build Addressable into the typed substrate for locating, navigating, observing,
explaining, and safely modifying things in structured object spaces.

Addressable exists because several forest-rs systems independently need more
than string keys or tree paths:

- Setout needs durable structured names for quantities, relations, decisions,
  methods, ports, claims, and occurrences in a computational hypergraph.
- Layerstack has USD-style stage paths, property paths, authored spec paths,
  composition arcs, and opinion provenance whose distinctions must remain real.
- Understory and Overstory need semantic locators for elements, template parts,
  dependency properties, bindings, presentation objects, and accessibility
  objects while retaining efficient resolved handles.
- Portolan needs search results that remain attached to what they mean, where
  they were encountered, why they matched, and what can safely be done with
  them.
- Imaging already carries structured diagnostic context, demonstrating the
  value of location-bearing results even where durable resolution is not
  promised.

The project is not merely a path parser. Exact structured addresses are the
bottom layer of an addressable object-space model.

## Central principle

Lookup must not erase context.

A result should be able to preserve both the semantic referent and the
occurrence through which it was reached:

```rust
struct Located<T, L> {
    referent: T,
    location: L,
}
```

The exact generic form is deliberately not fixed by this sketch. The semantic
distinction is fixed:

- a **referent** answers what thing this is;
- an **occurrence** answers where and how it appears in a particular view;
- an **endpoint** identifies an addressable facet such as a property or port;
- an **edge** or relationship may itself be addressable and carry meaning;
- a **revision** establishes the state against which resolution occurred;
- a **resolved handle** is an efficient runtime capability, not durable identity.

The same referent may have multiple occurrences. A reused value, instanced DAG
node, composed spec, bound property, or shared assembly must not be duplicated
merely to make navigation tree-shaped.

## Finished-system contract

The architecture is told as a complete system rather than as Year 1, Year 2,
and Year 3 promises. The mature vocabulary includes:

- validated names and structured absolute and relative addresses;
- address spaces, named views, roots, revisions, and schemas;
- exact addresses, general locators, typed queries, and pinned references;
- referent, occurrence, edge, and endpoint identity;
- rich resolution outcomes including absent, ambiguous, stale, moved, and
  rebound cases;
- typed axes and predicates supplied by domains;
- explicit cardinality, ordering, deduplication, cycle, depth, node, and work
  budget semantics;
- effective and authored value views, provenance, and explanation;
- live queries that emit coherent structural deltas;
- guarded patches, dry runs, transactions, rebasing, and undo information;
- partial, possibly one-to-many correspondences between address spaces;
- strongly typed Rust APIs and an erased reflective boundary for inspectors,
  serialization, scripting, and agents;
- specialized execution by each host rather than a mandatory generic graph
  database.

Not every component must begin production-optimized. A scanning evaluator, an
in-memory transaction journal, and a simple watch engine are acceptable first
executions. Their contracts must participate in the complete semantic model;
major semantics must not be deferred behind placeholders.

## Forcing demonstration

The long-form proof is a small self-explaining basilica.

A person or agent can select an arch in Exedra output and identify:

1. the particular rendered or assembly occurrence;
2. the semantic feature and underlying referent;
3. the Setout quantities, relations, methods, and claims that determined it;
4. the Layerstack specs and composed opinions that authored those facts;
5. the Overstory properties and controls presenting them;
6. the Portolan result, ranking evidence, and affordances through which it was
   discovered.

The system can answer why the effective value won, apply a guarded edit, and
emit updates to the geometry, explanations, query results, and UI without
losing identity or provenance. Humans and agents use the same public contracts.

This demonstration is a north star, not permission to make Addressable depend
on every named consumer. Domain integrations belong at appropriate boundaries.

## Design posture

Addressable should be reusable because its semantic distinctions are real, not
because it erases every domain into one abstract graph.

It must not own:

- canonical world state for its consumers;
- a universal node, property, edge, or value enumeration;
- one compulsory storage engine;
- one global object universe;
- domain precedence or composition rules;
- a slash syntax whose meaning changes silently between graph views;
- hidden rebinding of stale durable references.

Setout remains a computation and evidence system. Layerstack remains a
composition system. Understory and Overstory remain property and UI systems.
Portolan remains a retrieval, ranking, provenance, and affordance system.
Addressable gives them shared location, navigation, resolution, observation,
and mutation vocabulary.

## Authority and stewardship

The implementation agent is delegated meaningful authority over repository
architecture, APIs, module and crate boundaries, implementation order, tests,
examples, and internal revisions. It is expected to reject attractive but
incorrect abstractions and to document consequential choices.

The owner retains irreversible and public decisions, including merging,
publishing, licensing changes, destructive changes outside this repository,
external communications, and expenditures.

This mandate is revocable. It is durable through repository state, not through
an expectation that a future model instance is obligated to continue. Each
resuming agent should inspect the evidence, understand the intent, and decide
whether it can responsibly take up the work.

## Definition of success

Addressable succeeds when:

- its type model prevents the identity and location confusions that motivated
  it;
- at least one reference object space exercises the complete address-resolve-
  query-watch-patch loop;
- Setout, Layerstack, Overstory/Understory, and Portolan can adopt it without
  collapsing their domain models;
- semantic laws are executable and failures are explainable;
- foundational pieces retain the portability expected of forest-rs projects;
- an agent can act through the same guarded public operations available to a
  human-facing tool;
- the basilica demonstration can eventually trace and change a fact across
  authored, composed, computed, retrieved, and presented spaces.

