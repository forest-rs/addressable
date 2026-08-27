# addressable_tree

`addressable_tree` executes Addressable locators and queries over a host-owned
rooted tree or forest. Hosts keep their storage, indexes, node values, domain
predicates, and runtime handles; the reusable runtime owns revision context,
rich resolution, tree traversal, cardinality, ordering, deduplication, cycle
policy, and budgets.

Implement `TreeHost` for the domain storage type, construct `TreeRuntime` with a
host-assigned `SpaceId`, and use the runtime's `resolve`, `resolve_pinned`,
`query_many`, `query_one`, `query_optional`, and `resolved_handle` methods.

The crate is always `no_std + alloc`, owns no storage engine, and depends only
on `addressable`.
