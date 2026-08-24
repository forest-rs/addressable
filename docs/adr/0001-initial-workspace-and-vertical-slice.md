# ADR 0001: Initial workspace and complete vertical slice

- Status: accepted
- Date: 2026-08-24

## Context

Addressable must prove the complete address-resolve-query-watch-patch loop
without turning its durable vocabulary into a host-owned graph database. It
must also keep schema-backed erasure out of the ordinary typed API.

## Fences

This `addressable` crate owns portable structured addressing and semantic
contracts; it explicitly does not own object storage, indexing, scheduling, or
domain values.

This `addressable_reference` crate owns a small scanning object-space host and
its conformance proof; it explicitly does not redefine durable addressing or
dynamic tool schemas.

This `addressable_tooling` crate owns schema-backed erased inspection and
operation adaptation; it explicitly does not bypass the host's typed guarded
operations.

The `addressable_tour` example owns the executable narrative; it explicitly
does not provide reusable production behavior.

## Invariants

1. Space, referent, occurrence, endpoint, revision, and runtime handle identity
   remain distinct types.
2. Exact and relative text parse into structured addresses; strings are never
   the resolved representation.
3. A pinned locator never returns ordinary success for a different referent.
4. Query cardinality is visible in the query type, while ordering,
   deduplication, cycle behavior, and budgets remain explicit values.
5. Watch deltas replay to the same observable snapshot as full recomputation.
6. Guarded transactions validate atomically and expose dry-run and undo data.
7. Correspondence preserves one-to-many outcomes and evidence.
8. Dynamic reads and writes recover a declared schema and delegate to the same
   typed host methods used by Rust callers.
9. Runtime-local handles cannot be formatted or parsed as durable addresses.

## Options considered

1. **One crate.** Smallest package count, but `std` execution and dynamic values
   would contaminate the portable vocabulary boundary.
2. **Core plus reference runtime.** Preserves portability, but placing erased
   tooling in the runtime would make an open-world adapter look like ordinary
   host API.
3. **Core, reference runtime, and tooling adapter.** Adds two real dependency
   seams and is the chosen design. The executable example remains a separate
   non-production workspace package.

## Decision

Use the third option with one-way dependencies:

```text
addressable <- addressable_reference <- addressable_tooling <- addressable_tour
```

The reference domain is a small basilica model. One semantic arch referent has
two assembly occurrences. An explicit axis crosses from the assembly view into
a cyclic dependency view. A typed load endpoint has authored/default opinions,
can be read with structured explanation, and can be changed only by a guarded
transaction. A scanning watch recomputes and emits coherent structural deltas.
A second catalog space demonstrates one-to-many correspondence. The tooling
adapter exposes the same load operation through a small declared dynamic
schema.

The core stays dependency-free and always `no_std + alloc`. The reference and
tooling crates are honestly `std`-dependent. All packages begin unpublished.

## Cardinality decision

`One`, `Optional`, and `Many` are marker types on `Query`. Reference execution
methods accept the corresponding query type and return the corresponding
shape. Dynamic tooling may erase that marker only after validating its schema.
This combines compile-time call-site guidance with a representable runtime
contract.

## Revision and space identity decision

`SpaceId<S>` is a caller/host-assigned typed `u64`; it does not require a global
allocator or atomics. `Revision` is a local monotonic value meaningful only
with its space. Locations and resolved handles carry both. Durable addresses
carry a space marker at compile time, while locators carry the runtime space
identity required when several instances coexist.

## Explanation and erasure decision

The core owns a generic winning-opinion shape. Domains own typed value and
provenance payloads. The tooling crate owns the deliberately small dynamic
value set used by its declared schema; that enum does not enter the typed core
or reference storage.

## Consequences and extension points

- Hosts may replace scanning with indexes without changing query or delta
  meaning.
- Domain axes, predicates, facets, values, and provenance remain generic.
- A future generic dynamic protocol can replace the reference-specific adapter
  once a second real adapter proves its common shape.
- Async runtimes, serialization frameworks, and hash maps are not dependencies
  of the nucleus.
