# Local forest-rs conventions

This note records the repository conventions discovered before Addressable was
scaffolded. The comparison was performed on 2026-08-24 against `exedra`,
`portolan`, `layerstack`, `understory`, `overstory`, and `inkstone` in the local
forest-rs checkout. Those repositories are references and were not modified.

## Governing tenets

The older forest-rs tenets found in sibling `AGENTS.md` files emphasize durable
modularity, incremental work, introspection, explicit behavior, replaceable
subsystems, and calm interfaces. Their definition of done adds strict
formatting, Clippy, rustdoc, public documentation, deterministic tests, and
durable ADRs for public semantic decisions.

Addressable adopts those tenets without importing a sibling's issue tracker.
This repository currently has neither Beads nor `tk` state, so the initial work
is owned by the durable plan and ADR in `docs/`.

## Workspace baseline

- Rust edition 2024.
- Rust 1.88 is the conservative shared MSRV. Newer siblings have moved to 1.92,
  but Layerstack, Understory, Portolan, and Inkstone still prove 1.88.
- Cargo resolver 2.
- The intended repository metadata remains `Apache-2.0 OR MIT`, as already
  stated in the bootstrap README. This slice does not add or alter license
  texts.
- Internal dependencies are centralized in `[workspace.dependencies]`, use
  `default-features = false`, and carry versions when publication is intended.
- Initial packages are `publish = false`; publication is an owner decision.
- Production crates have no dev-dependencies. Executable examples are separate
  top-level workspace crates.

## Portability and features

Foundational vocabulary is `#![no_std]` with `alloc`. A genuinely `std`-owned
runtime is expressed as a separate crate rather than a cosmetic feature. The
core has no production dependencies. CI checks the core on
`x86_64-unknown-none` and `wasm32-unknown-unknown` as well as native targets.

## Lints and formatting

The workspace uses the current Linebender-style lint set seen in Exedra and
Portolan, including `unsafe_code = "deny"`, `missing_docs = "warn"`, and the
Cargo metadata lints. The expected local gates are:

```sh
typos
taplo fmt --check --diff
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo check -p addressable --target x86_64-unknown-none
cargo check -p addressable --target wasm32-unknown-unknown
cargo +1.88 check --workspace --all-targets --all-features
```

CI follows the representative forest-rs matrix: formatting and repository
policy, strict Clippy, tests and doctests, rustdoc, MSRV, and explicit `no_std`
targets. The initial workspace has no need for `cargo-hack` because the core has
no optional feature matrix.

## Architectural evidence from siblings

- Layerstack keeps structured paths distinct from interned runtime `PathId`s.
- Setout distinguishes durable fingerprints from evaluation-local handles.
- Exedra uses typed generational handles and documents their runtime-local
  meaning.
- Understory uses typed property endpoints and local monotonic revisions.
- Portolan makes live result-entry identity and source capabilities explicit.
- Core crates consistently keep host runtimes, demos, and heavy adapters outside
  their foundational package.

These conventions support, rather than conflict with, the bootstrap
architecture. The only resolved ambiguity is packaging: the first slice uses
three production crates because each represents a demonstrated portability or
open-world dependency boundary, not a roadmap phase.
