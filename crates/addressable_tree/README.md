# addressable_tree

`addressable_tree` executes Addressable locators and typed queries over a
host-owned rooted tree or forest. The crate is always `no_std + alloc` and
depends only on `addressable`.

The host keeps its storage, indexes, node values, domain predicates, mutation
policy, and runtime handles. `TreeRuntime` owns the shared behavior:

- exact, relative, and pinned resolution with exhaustive outcomes;
- children, descendants, and parent traversal;
- explicit cardinality, ordering, deduplication, cycle policy, and budgets;
- revision validation and revision-scoped resolved handles.

## Host and runtime lifecycle

Implement `TreeHost` for the domain storage type. Its methods project
`TreeNode` values lazily; they do not transfer storage ownership. Predicate
evaluation returns `PredicateMatch`, including the domain work charged against
the query budget.

Bind a new host instance with `TreeRuntime::new`. A host snapshot that already
owns a revision uses `TreeRuntime::from_revision`. Apply validated in-place
mutations through `TreeRuntime::commit`, which advances the revision even if a
mutation unwinds after changing the host. `into_host` returns both the host and
the revision needed to reconstruct that same runtime identity.

```rust
use addressable::{Query, Resolution, SpaceId};
use addressable_tree::{TreeAxis, TreeRuntime};
# use addressable::{AbsoluteAddress};
# use addressable_tree::{PredicateMatch, TreeHost, TreeNode};
# enum Space {}
# #[derive(Clone, Copy, PartialEq, Eq)] enum View { Outline }
# enum Predicate { Any }
# struct Host;
# impl TreeHost for Host {
#     type Space = Space;
#     type View = View;
#     type Referent = u64;
#     type Occurrence = u64;
#     type Handle = u64;
#     type Predicate = Predicate;
#     fn supports_view(&self, view: &View) -> bool { *view == View::Outline }
#     fn node_at(&self, _: &View, address: &AbsoluteAddress<Space>) -> Option<TreeNode<Space, u64, u64, u64>> {
#         (address.depth() == 0).then(|| TreeNode::new(1, 1, AbsoluteAddress::root(), Some(1)))
#     }
#     fn nodes(&self, view: &View) -> impl Iterator<Item = TreeNode<Space, u64, u64, u64>> { self.node_at(view, &AbsoluteAddress::root()).into_iter() }
#     fn children(&self, _: &View, _: &u64) -> impl Iterator<Item = TreeNode<Space, u64, u64, u64>> { core::iter::empty() }
#     fn parent(&self, _: &View, _: &u64) -> Option<TreeNode<Space, u64, u64, u64>> { None }
#     fn matches(&self, _: &TreeNode<Space, u64, u64, u64>, _: &Predicate) -> PredicateMatch { PredicateMatch::new(true, 1) }
# }

let runtime = TreeRuntime::new(SpaceId::<Space>::new(1), Host);
let root = runtime.root_locator(View::Outline);
assert!(matches!(runtime.resolve(&root), Resolution::Resolved(_)));

let children = runtime.query_many(
    &Query::many(root).traverse(TreeAxis::Children),
)?;
assert!(children.items().is_empty());
# Ok::<(), addressable::QueryError>(())
```

## Minimum supported Rust version

This crate has been verified to compile with Rust 1.88 and later.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
