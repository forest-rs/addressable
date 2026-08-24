# Project status

## Current state

The GitHub repository was created on 2026-08-24 with an empty `README.md` on
`main`. This bootstrap material was prepared from the originating ChatGPT Work
conversation before moving development into a local Codex or local Work session.

No Rust workspace, crate layout, public API, CI configuration, license files, or
release policy has been committed yet. That is intentional: the cloud session
could inspect GitHub but could not access the owner's local forest-rs checkout
and old tenets.

## Why this branch exists

The originating conversation developed a mature architectural direction for
Addressable and then encountered a product boundary: a cloud Work conversation
could continue on desktop, but could not see `/Users/bruce/Development/forest-rs`
or become the same repository-bound local Codex session.

This branch is the durable bridge across that brief break. The local session is
not expected to reconstruct intent from chat history.

## First actions for the local session

1. Open or clone `forest-rs/addressable` under
   `/Users/bruce/Development/forest-rs/addressable` and check out this branch.
2. Read `AGENTS.md`, `MANDATE.md`, and `docs/ARCHITECTURE.md` completely.
3. Discover and read applicable ancestor instructions and the old forest-rs
   tenets. Search the local forest-rs tree rather than assuming they are public
   or current.
4. Inspect representative current CI, metadata, lint, formatting, licensing,
   MSRV, feature, and `no_std` practice in sibling projects. At minimum compare
   `exedra`, `portolan`, `layerstack`, `understory`, `overstory`, and `inkstone`.
5. Record the resulting project conventions and any conflict with this bootstrap
   architecture before scaffolding.
6. Decide the smallest honest initial crate/workspace boundary that supports the
   complete vertical slice in `docs/ARCHITECTURE.md`.
7. Implement, test, and document the vertical slice. Use a branch and keep
   changes reversible. Do not merge or publish without the owner's decision.

Useful local discovery commands include:

```sh
rg --files /Users/bruce/Development/forest-rs \
  | rg '(^|/)(AGENTS\.md|.*[Tt][Ee][Nn][Ee][Tt].*|ci\.yml|Cargo\.toml|taplo\.toml|clippy\.toml)$'

rg -n -i 'tenet|no_std|msrv|wasm32v1-none|cargo hack|cargo semver|rustdoc' \
  /Users/bruce/Development/forest-rs
```

Prefer narrower searches after locating likely files; the tree contains many
repositories and generated build output may be large.

## Important unresolved decisions

- Exact crate/module boundaries after applying local forest-rs conventions.
- The smallest sufficient type representation for owned and borrowed names,
  paths, locations, and occurrences.
- Whether query cardinality belongs in static types, builders, execution
  methods, or a combination.
- Revision and space identity requirements in `no_std` contexts.
- The division between shared explanation vocabulary and domain-defined
  explanation payloads.
- The erased/schema boundary needed by Portolan and agents.
- Which consumer provides the first real adapter after the reference space.

The implementation agent owns these choices within the mandate and should use
evidence to decide rather than asking the owner to settle routine architecture.

