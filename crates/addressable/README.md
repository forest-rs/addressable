# addressable

`addressable` is the typed, `no_std + alloc` vocabulary for locating,
inspecting, watching, and safely editing values in structured object spaces.
It keeps six things deliberately distinct:

- a semantic referent;
- one contextual occurrence of that referent;
- the exact address of that occurrence;
- the named view in which the address is meaningful;
- the runtime space instance and revision that were observed;
- a runtime-local handle used only as an accelerator.

The crate owns no storage engine, graph evaluator, async runtime, or universal
value type. A domain host constructs contextual values and implements the
resolution, query, read, watch, or edit operations it supports.

## From an observed location to a safe pin

A host normally returns a `Location` from resolution or query execution. Turn
that one observation into an exact pinned reference with
`Pinned::from_location`:

```rust
use addressable::{AbsoluteAddress, Location, Pinned, Revision, SpaceId};

enum DocumentSpace {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DocumentView {
    Outline,
}

let space = SpaceId::<DocumentSpace>::new(1);
let location = Location::new(
    DocumentView::Outline,
    Revision::new(space, 4),
    42_u64, // referent
    7_u64,  // occurrence
    AbsoluteAddress::parse("/chapter/section")?,
);
let pinned = Pinned::from_location(&location);

assert_eq!(pinned.expected_referent(), &42);
assert_eq!(pinned.expected_revision(), location.revision());
# Ok::<(), addressable::AddressError>(())
```

The host later resolves that pin to an exhaustive `Resolution`: resolved,
absent, ambiguous, stale, moved, rebound, unsupported, budget-limited, partial,
or capability-unavailable. It must never silently accept rebinding.

`SpaceId`, `Revision`, and the textual forms of `Locator` and `Pinned` are
runtime-scoped. Persist or exchange them only when the host preserves the same
space-id assignment. An `AbsoluteAddress` can be durable when its domain makes
that guarantee.

For reusable locator and query execution over a host-owned rooted tree, see
[`addressable_tree`](https://crates.io/crates/addressable_tree).

## Minimum supported Rust version

This crate has been verified to compile with Rust 1.88 and later.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
