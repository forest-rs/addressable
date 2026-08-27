// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reusable Addressable execution over host-owned rooted trees.
//!
//! This crate owns revisioned locator and tree-query execution. It explicitly
//! does not own node storage, indexes, domain predicates, values, or mutation
//! policy. A host implements [`TreeHost`], then binds one host value to a
//! runtime [`SpaceId`] through [`TreeRuntime::new`].
//!
//! ```
//! use addressable::{AbsoluteAddress, Query, Resolution, SpaceId};
//! use addressable_tree::{PredicateMatch, TreeAxis, TreeHost, TreeNode, TreeRuntime};
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! enum View { Instances }
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! enum Predicate { Any }
//! #[derive(Clone)]
//! enum Space {}
//!
//! #[derive(Clone, Debug)]
//! struct Host;
//!
//! impl TreeHost for Host {
//!     type Space = Space;
//!     type View = View;
//!     type Referent = u64;
//!     type Occurrence = u64;
//!     type Handle = u32;
//!     type Predicate = Predicate;
//!
//!     fn supports_view(&self, view: &View) -> bool {
//!         *view == View::Instances
//!     }
//!
//!     fn node_at(
//!         &self,
//!         _view: &View,
//!         address: &AbsoluteAddress<Space>,
//!     ) -> Option<TreeNode<Space, u64, u64, u32>> {
//!         (address.depth() == 0).then(|| {
//!             TreeNode::new(1, 1, AbsoluteAddress::root(), Some(0))
//!         })
//!     }
//!
//!     fn nodes(&self, view: &View) -> impl Iterator<Item = TreeNode<Space, u64, u64, u32>> {
//!         self.node_at(view, &AbsoluteAddress::root()).into_iter()
//!     }
//!
//!     fn children(
//!         &self,
//!         _view: &View,
//!         _occurrence: &u64,
//!     ) -> impl Iterator<Item = TreeNode<Space, u64, u64, u32>> {
//!         core::iter::empty()
//!     }
//!
//!     fn parent(&self, _view: &View, _occurrence: &u64) -> Option<TreeNode<Space, u64, u64, u32>> {
//!         None
//!     }
//!
//!     fn matches(
//!         &self,
//!         _node: &TreeNode<Space, u64, u64, u32>,
//!         predicate: &Predicate,
//!     ) -> PredicateMatch {
//!         PredicateMatch::new(*predicate == Predicate::Any, 1)
//!     }
//! }
//!
//! let runtime = TreeRuntime::new(SpaceId::<Space>::new(7), Host);
//! let root = runtime.root_locator(View::Instances);
//! assert!(matches!(runtime.resolve(&root), Resolution::Resolved(_)));
//!
//! let results = runtime
//!     .query_many(&Query::many(root).traverse(TreeAxis::Children))?;
//! assert!(results.items().is_empty());
//! # Ok::<(), addressable::QueryError>(())
//! ```

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

use alloc::{collections::BTreeSet, collections::VecDeque, vec, vec::Vec};
use core::fmt;

pub use addressable::Measured;
use addressable::{
    AbsoluteAddress, BudgetDimension, BudgetExceeded, Cardinality, CyclePolicy, Deduplication,
    Location, Locator, Many, One, Optional, Pinned, Query, QueryError, QueryResults,
    QuerySemantics, QueryStats, QueryStep, Resolution, ResolvedHandle, ResultOrdering, Revision,
    SpaceId, VisitIdentity,
};

/// One host-projected tree node with durable identities and optional runtime handle.
///
/// Hosts construct these in [`TreeHost`] methods. [`TreeRuntime`] turns them
/// into revision-scoped [`Location`] values and [`ResolvedHandle`] values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode<S, R, O, H> {
    referent: R,
    occurrence: O,
    address: AbsoluteAddress<S>,
    handle: Option<H>,
}

impl<S, R, O, H> TreeNode<S, R, O, H> {
    /// Projects one current node from host storage.
    #[must_use]
    pub const fn new(
        referent: R,
        occurrence: O,
        address: AbsoluteAddress<S>,
        handle: Option<H>,
    ) -> Self {
        Self {
            referent,
            occurrence,
            address,
            handle,
        }
    }

    /// Returns durable semantic referent identity.
    #[must_use]
    pub const fn referent(&self) -> &R {
        &self.referent
    }

    /// Returns durable contextual occurrence identity.
    #[must_use]
    pub const fn occurrence(&self) -> &O {
        &self.occurrence
    }

    /// Returns the canonical exact address in the projected view.
    #[must_use]
    pub const fn address(&self) -> &AbsoluteAddress<S> {
        &self.address
    }

    /// Returns a host-local runtime accelerator when this node has one.
    #[must_use]
    pub const fn handle(&self) -> Option<&H> {
        self.handle.as_ref()
    }
}

/// Result of evaluating one host-owned predicate against a projected node.
///
/// [`TreeRuntime`] adds `work` to
/// [`QueryStats::work_units`](addressable::QueryStats::work_units). Hosts choose
/// a stable unit meaningful for their domain: a constant in-memory comparison
/// commonly costs one, while resolving an opinion stack or consulting an
/// external index may cost more. The work value may be zero for a cached result
/// that performs no observable host work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PredicateMatch {
    /// Whether the node satisfies the predicate.
    pub matched: bool,
    /// Host-defined work performed to decide the match.
    pub work: u32,
}

impl PredicateMatch {
    /// Reports a predicate result and its host-defined work.
    #[must_use]
    pub const fn new(matched: bool, work: u32) -> Self {
        Self { matched, work }
    }
}

/// Host-owned projection required by [`TreeRuntime`].
///
/// The host retains storage and indexing choices. Every method observes one
/// coherent host value. `node_at` must return at most one node for an exact
/// address; `nodes` must enumerate every node in the view; `children` and
/// `parent` must describe the same rooted forest. Iterators yield nodes in
/// deterministic traversal order.
pub trait TreeHost: Sized {
    /// Type-level address-space marker.
    type Space: Clone;
    /// Named domain view.
    type View: Clone + Eq;
    /// Durable semantic referent identity.
    type Referent: Clone + Ord;
    /// Durable contextual occurrence identity.
    type Occurrence: Clone + Ord;
    /// Runtime-local node accelerator.
    type Handle: Clone;
    /// Domain-owned node predicate.
    type Predicate;

    /// Reports whether this host projects the named rooted-tree view.
    fn supports_view(&self, view: &Self::View) -> bool;

    /// Projects the node at one canonical exact address.
    fn node_at(
        &self,
        view: &Self::View,
        address: &AbsoluteAddress<Self::Space>,
    ) -> Option<HostNode<Self>>;

    /// Iterates every node in one view in deterministic traversal order.
    fn nodes<'a>(&'a self, view: &'a Self::View) -> impl Iterator<Item = HostNode<Self>> + 'a;

    /// Iterates current occurrences of one durable referent.
    ///
    /// The default scans [`Self::nodes`] lazily. Indexed hosts should override
    /// this so pinned movement detection does not inspect unrelated nodes.
    fn occurrences_of<'a>(
        &'a self,
        view: &'a Self::View,
        referent: &'a Self::Referent,
    ) -> impl Iterator<Item = HostNode<Self>> + 'a {
        self.nodes(view)
            .filter(move |node| node.referent() == referent)
    }

    /// Iterates the direct children of one occurrence in deterministic order.
    fn children<'a>(
        &'a self,
        view: &'a Self::View,
        occurrence: &'a Self::Occurrence,
    ) -> impl Iterator<Item = HostNode<Self>> + 'a;

    /// Projects the parent of one occurrence, if it has one.
    fn parent(&self, view: &Self::View, occurrence: &Self::Occurrence) -> Option<HostNode<Self>>;

    /// Evaluates one domain predicate and reports the work it performed.
    ///
    /// The runtime charges the returned [`PredicateMatch::work`] against the
    /// query's
    /// [`TraversalBudget::max_work`](addressable::TraversalBudget::max_work).
    /// A constant in-memory comparison normally reports one work unit.
    fn matches(&self, node: &HostNode<Self>, predicate: &Self::Predicate) -> PredicateMatch;
}

impl<H: TreeHost> TreeHost for &H {
    type Space = H::Space;
    type View = H::View;
    type Referent = H::Referent;
    type Occurrence = H::Occurrence;
    type Handle = H::Handle;
    type Predicate = H::Predicate;

    fn supports_view(&self, view: &Self::View) -> bool {
        H::supports_view(self, view)
    }

    fn node_at(
        &self,
        view: &Self::View,
        address: &AbsoluteAddress<Self::Space>,
    ) -> Option<HostNode<Self>> {
        H::node_at(self, view, address)
    }

    fn nodes<'a>(&'a self, view: &'a Self::View) -> impl Iterator<Item = HostNode<Self>> + 'a {
        H::nodes(self, view)
    }

    fn occurrences_of<'a>(
        &'a self,
        view: &'a Self::View,
        referent: &'a Self::Referent,
    ) -> impl Iterator<Item = HostNode<Self>> + 'a {
        H::occurrences_of(self, view, referent)
    }

    fn children<'a>(
        &'a self,
        view: &'a Self::View,
        occurrence: &'a Self::Occurrence,
    ) -> impl Iterator<Item = HostNode<Self>> + 'a {
        H::children(self, view, occurrence)
    }

    fn parent(&self, view: &Self::View, occurrence: &Self::Occurrence) -> Option<HostNode<Self>> {
        H::parent(self, view, occurrence)
    }

    fn matches(&self, node: &HostNode<Self>, predicate: &Self::Predicate) -> PredicateMatch {
        H::matches(self, node, predicate)
    }
}

/// Shared rooted-tree navigation axes.
///
/// Append these through [`Query::traverse`], then execute the resulting
/// [`TreeQuery`] through the matching method on [`TreeRuntime`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeAxis {
    /// Direct children of every current occurrence.
    Children,
    /// Recursive descendants of every current occurrence.
    Descendants,
    /// Direct parent of every current occurrence.
    Parent,
}

/// Resolved location produced by a tree runtime over host `H`.
pub type TreeLocation<H> = Location<
    <H as TreeHost>::Space,
    <H as TreeHost>::View,
    <H as TreeHost>::Referent,
    <H as TreeHost>::Occurrence,
>;

/// One node projected by tree host `H`.
pub type HostNode<H> = TreeNode<
    <H as TreeHost>::Space,
    <H as TreeHost>::Referent,
    <H as TreeHost>::Occurrence,
    <H as TreeHost>::Handle,
>;

/// Revision-scoped runtime handle produced for tree host `H`.
pub type TreeHandle<H> = ResolvedHandle<<H as TreeHost>::Space, <H as TreeHost>::Handle>;

/// Locator accepted by a tree runtime over host `H`.
pub type TreeLocator<H> = Locator<<H as TreeHost>::Space, <H as TreeHost>::View>;

/// Rich resolution result produced by a tree runtime over host `H`.
pub type TreeResolution<H> = Resolution<
    <H as TreeHost>::Space,
    TreeLocation<H>,
    <H as TreeHost>::Referent,
    AbsoluteAddress<<H as TreeHost>::Space>,
>;

/// Typed query executed by a tree runtime over host `H`.
pub type TreeQuery<H, C = Many> = Query<TreeLocator<H>, TreeAxis, <H as TreeHost>::Predicate, C>;

/// Revisioned Addressable execution over one host-owned tree value.
///
/// Construct this with [`Self::new`]. It exposes immutable host access so no
/// mutation can bypass its revision. Domain code validates an operation, then
/// applies it through [`Self::commit`]. [`Self::into_host`] and
/// [`Self::from_revision`] preserve the clock when ownership must cross a
/// boundary. Hosts that already own a revision use the same constructor to
/// lend a coherent snapshot to the runtime.
#[derive(Clone, Debug)]
pub struct TreeRuntime<H: TreeHost> {
    id: SpaceId<H::Space>,
    revision: Revision<H::Space>,
    host: H,
}

struct AdvanceRevision<'a, S> {
    revision: &'a mut Revision<S>,
    next: Revision<S>,
}

impl<S> Drop for AdvanceRevision<'_, S> {
    fn drop(&mut self) {
        *self.revision = self.next;
    }
}

impl<H: TreeHost> TreeRuntime<H> {
    /// Binds a host value to one runtime address-space identity.
    ///
    /// `id` must identify a genuinely new space instance. Restore an extracted
    /// instance with [`Self::from_revision`]; reusing its id with `new` would
    /// restart the clock and could make stale observations appear current.
    #[must_use]
    pub fn new(id: SpaceId<H::Space>, host: H) -> Self {
        Self {
            id,
            revision: Revision::initial(id),
            host,
        }
    }

    /// Binds a host value at an existing host-owned revision.
    ///
    /// Use this for a host snapshot that already owns its revision, including
    /// the values returned by [`Self::into_host`]. The revision already carries
    /// the owning [`SpaceId`], so a mismatched pair cannot be supplied.
    #[must_use]
    pub const fn from_revision(revision: Revision<H::Space>, host: H) -> Self {
        Self {
            id: revision.space(),
            revision,
            host,
        }
    }

    /// Returns this runtime address-space identity.
    #[must_use]
    pub const fn id(&self) -> SpaceId<H::Space> {
        self.id
    }

    /// Returns the revision governing every current result.
    #[must_use]
    pub const fn revision(&self) -> Revision<H::Space> {
        self.revision
    }

    /// Borrows the host-owned tree value immutably.
    #[must_use]
    pub const fn host(&self) -> &H {
        &self.host
    }

    /// Recovers the revision and host value, consuming the live runtime.
    ///
    /// Pass both values to [`Self::from_revision`] to restore the same clock.
    /// Mutation should normally remain inside the runtime through
    /// [`Self::commit`].
    #[must_use]
    pub fn into_host(self) -> (Revision<H::Space>, H) {
        (self.revision, self.host)
    }

    /// Applies one already-validated host mutation and advances once.
    ///
    /// The closure is infallible by design: domain code validates fallible
    /// preconditions before entering the commit. The returned revision governs
    /// the closure's result and every subsequent runtime observation. The
    /// revision also advances if the closure unwinds after partially mutating
    /// the host, so a caught panic cannot leave changed data at the old clock.
    pub fn commit<T>(&mut self, mutation: impl FnOnce(&mut H) -> T) -> (Revision<H::Space>, T) {
        let next = self.revision.next();
        let value = {
            let _advance = AdvanceRevision {
                revision: &mut self.revision,
                next,
            };
            mutation(&mut self.host)
        };
        (self.revision, value)
    }

    /// Replaces the complete host value and advances the revision once.
    ///
    /// Domain transaction code calls this only after validating every
    /// precondition and successfully applying every operation to a private
    /// replacement value. Dry runs and no-op batches retain the current host.
    pub fn replace_host(&mut self, host: H) -> Revision<H::Space> {
        let next = self.revision.next();
        self.host = host;
        self.revision = next;
        self.revision
    }

    /// Returns an exact locator for `/` in one supported tree view.
    #[must_use]
    pub fn root_locator(&self, view: H::View) -> TreeLocator<H> {
        Locator::exact(self.id, view, AbsoluteAddress::root())
    }

    /// Resolves an exact or relative locator through the host projection.
    #[must_use]
    pub fn resolve(&self, locator: &TreeLocator<H>) -> TreeResolution<H> {
        if locator.space() != self.id || !self.host.supports_view(locator.view()) {
            return Resolution::UnsupportedLocator;
        }
        let Ok(address) = locator.to_absolute() else {
            return Resolution::UnsupportedLocator;
        };
        self.host
            .node_at(locator.view(), &address)
            .map(|node| self.location(locator.view().clone(), node))
            .map_or(Resolution::Absent, Resolution::Resolved)
    }

    /// Resolves a pin without silently accepting staleness or rebinding.
    #[must_use]
    pub fn resolve_pinned(
        &self,
        pinned: &Pinned<H::Space, H::View, H::Referent>,
    ) -> TreeResolution<H> {
        let locator = pinned.locator();
        if locator.space() != self.id || !self.host.supports_view(locator.view()) {
            return Resolution::UnsupportedLocator;
        }
        let Ok(address) = locator.to_absolute() else {
            return Resolution::UnsupportedLocator;
        };

        if let Some(node) = self.host.node_at(locator.view(), &address) {
            let location = self.location(locator.view().clone(), node);
            if location.referent() != pinned.expected_referent() {
                return Resolution::Rebound {
                    expected: pinned.expected_referent().clone(),
                    actual: location.referent().clone(),
                    resolved: location,
                };
            }
            if pinned.expected_revision() != self.revision {
                return Resolution::StaleRevision {
                    expected: pinned.expected_revision(),
                    actual: self.revision,
                };
            }
            return Resolution::Resolved(location);
        }

        let moved = self
            .host
            .occurrences_of(locator.view(), pinned.expected_referent())
            .map(|node| self.location(locator.view().clone(), node))
            .collect::<Vec<_>>();
        match moved.as_slice() {
            [] => Resolution::Absent,
            [location] => Resolution::Moved {
                from: address,
                to: location.address().clone(),
                resolved: location.clone(),
            },
            _ => Resolution::Ambiguous(moved.into_boxed_slice()),
        }
    }

    /// Executes a many-result tree query.
    pub fn query_many(
        &self,
        query: &TreeQuery<H, Many>,
    ) -> Result<QueryResults<TreeLocation<H>>, QueryError> {
        self.execute(query)
    }

    /// Executes a tree query requiring exactly one result.
    pub fn query_one(
        &self,
        query: &TreeQuery<H, One>,
    ) -> Result<Measured<TreeLocation<H>>, QueryError> {
        self.execute(query)?.require_one()
    }

    /// Executes a tree query allowing zero or one result.
    pub fn query_optional(
        &self,
        query: &TreeQuery<H, Optional>,
    ) -> Result<Measured<Option<TreeLocation<H>>>, QueryError> {
        self.execute(query)?.require_optional()
    }

    /// Resolves a location to its revision-scoped host-local handle.
    pub fn resolved_handle(
        &self,
        location: &TreeLocation<H>,
    ) -> Result<TreeHandle<H>, TreeReadError<H::Space>> {
        let node = self.validate_location(location)?;
        let handle = node.handle().cloned().ok_or(TreeReadError::NoHandle)?;
        Ok(ResolvedHandle::new(self.revision, handle))
    }

    /// Validates a location and recovers its current host projection.
    ///
    /// Domain endpoint reads use this before consuming node identity or a
    /// runtime handle.
    pub fn validate_location(
        &self,
        location: &TreeLocation<H>,
    ) -> Result<HostNode<H>, TreeReadError<H::Space>> {
        if location.space() != self.id {
            return Err(TreeReadError::WrongSpace);
        }
        if !self.host.supports_view(location.view()) {
            return Err(TreeReadError::WrongView);
        }
        if location.revision() != self.revision {
            return Err(TreeReadError::StaleRevision {
                expected: location.revision(),
                actual: self.revision,
            });
        }
        let node = self
            .host
            .node_at(location.view(), location.address())
            .ok_or(TreeReadError::MissingOccurrence)?;
        if node.occurrence() != location.occurrence() {
            return Err(TreeReadError::Moved);
        }
        if node.referent() != location.referent() {
            return Err(TreeReadError::Rebound);
        }
        Ok(node)
    }

    fn location(&self, view: H::View, node: HostNode<H>) -> TreeLocation<H> {
        Location::new(
            view,
            self.revision,
            node.referent,
            node.occurrence,
            node.address,
        )
    }

    fn execute<C: Cardinality>(
        &self,
        query: &TreeQuery<H, C>,
    ) -> Result<QueryResults<TreeLocation<H>>, QueryError> {
        let locator = query.start();
        if locator.space() != self.id || !self.host.supports_view(locator.view()) {
            return Err(QueryError::StartDidNotResolve);
        }
        let address = locator
            .to_absolute()
            .map_err(|_| QueryError::StartDidNotResolve)?;
        let start = self
            .host
            .node_at(locator.view(), &address)
            .ok_or(QueryError::StartDidNotResolve)?;
        let semantics = query.semantics();
        let mut stats = QueryStats::default();
        stats.charge(semantics.budget, 1, 1, 0)?;
        let mut frontier = vec![start];

        for step in query.steps() {
            frontier = match step {
                QueryStep::Traverse(axis) => {
                    self.traverse(locator.view(), &frontier, *axis, semantics, &mut stats)?
                }
                QueryStep::Filter(predicate) => {
                    let mut filtered = Vec::with_capacity(frontier.len());
                    for node in frontier {
                        let predicate_match = self.host.matches(&node, predicate);
                        stats.charge_work(semantics.budget, predicate_match.work)?;
                        if predicate_match.matched {
                            filtered.push(node);
                        }
                    }
                    filtered
                }
            };
        }

        deduplicate::<H>(&mut frontier, semantics.deduplication);
        if semantics.ordering == ResultOrdering::Stable {
            frontier.sort_by(|left, right| left.address().cmp(right.address()));
        }
        let result_count = u32::try_from(frontier.len()).unwrap_or(u32::MAX);
        if result_count > semantics.budget.max_results {
            return Err(QueryError::BudgetExceeded(BudgetExceeded::new(
                BudgetDimension::Results,
                semantics.budget.max_results,
                result_count,
            )));
        }
        Ok(QueryResults::new(
            frontier
                .into_iter()
                .map(|node| self.location(locator.view().clone(), node)),
            stats,
        ))
    }

    fn traverse(
        &self,
        view: &H::View,
        frontier: &[HostNode<H>],
        axis: TreeAxis,
        semantics: QuerySemantics,
        stats: &mut QueryStats,
    ) -> Result<Vec<HostNode<H>>, QueryError> {
        let mut output = Vec::new();
        for node in frontier {
            match axis {
                TreeAxis::Children => {
                    for child in self.host.children(view, node.occurrence()) {
                        stats.charge(semantics.budget, 1, 1, 1)?;
                        output.push(child);
                    }
                }
                TreeAxis::Descendants => {
                    self.push_descendants(view, node, semantics, stats, &mut output)?;
                }
                TreeAxis::Parent => {
                    if let Some(parent) = self.host.parent(view, node.occurrence()) {
                        stats.charge(semantics.budget, 1, 1, 1)?;
                        output.push(parent);
                    }
                }
            }
        }
        Ok(output)
    }

    fn push_descendants(
        &self,
        view: &H::View,
        start: &HostNode<H>,
        semantics: QuerySemantics,
        stats: &mut QueryStats,
        output: &mut Vec<HostNode<H>>,
    ) -> Result<(), QueryError> {
        let mut queue = VecDeque::from_iter(
            self.host
                .children(view, start.occurrence())
                .map(|node| (node, 1_u32)),
        );
        let mut visited_occurrences = BTreeSet::from([start.occurrence().clone()]);
        let mut visited_referents = BTreeSet::from([start.referent().clone()]);

        while let Some((node, depth)) = queue.pop_front() {
            let revisited = match semantics.cycle_policy {
                CyclePolicy::Error | CyclePolicy::SkipVisited(VisitIdentity::Occurrence) => {
                    !visited_occurrences.insert(node.occurrence().clone())
                }
                CyclePolicy::SkipVisited(VisitIdentity::Referent) => {
                    !visited_referents.insert(node.referent().clone())
                }
            };
            if revisited {
                if semantics.cycle_policy == CyclePolicy::Error {
                    return Err(QueryError::Cycle);
                }
                continue;
            }
            visited_occurrences.insert(node.occurrence().clone());
            visited_referents.insert(node.referent().clone());
            stats.charge(semantics.budget, 1, 1, depth)?;

            let next_depth = depth.saturating_add(1);
            queue.extend(
                self.host
                    .children(view, node.occurrence())
                    .map(|child| (child, next_depth)),
            );
            output.push(node);
        }
        Ok(())
    }
}

/// Failure to consume a tree runtime's location or handle projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeReadError<S> {
    /// The location belongs to another runtime space instance.
    WrongSpace,
    /// The location names a view this host does not project as a tree.
    WrongView,
    /// The location was resolved at an earlier runtime revision.
    StaleRevision {
        /// Revision carried by the stale value.
        expected: Revision<S>,
        /// Current runtime revision.
        actual: Revision<S>,
    },
    /// The exact address no longer has a node.
    MissingOccurrence,
    /// The occurrence now has another exact address.
    Moved,
    /// The exact address now denotes another referent.
    Rebound,
    /// The projected node has no runtime-local handle.
    NoHandle,
}

impl<S> fmt::Display for TreeReadError<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongSpace => "tree location belongs to another space",
            Self::WrongView => "tree location uses an unsupported view",
            Self::StaleRevision { .. } => "tree location is stale",
            Self::MissingOccurrence => "tree occurrence is absent",
            Self::Moved => "tree occurrence moved",
            Self::Rebound => "tree address rebound to another referent",
            Self::NoHandle => "tree node has no runtime handle",
        })
    }
}

impl<S: fmt::Debug> core::error::Error for TreeReadError<S> {}

fn deduplicate<H: TreeHost>(frontier: &mut Vec<HostNode<H>>, identity: Deduplication) {
    match identity {
        Deduplication::None => {}
        Deduplication::Occurrence => {
            let mut seen = BTreeSet::new();
            frontier.retain(|node| seen.insert(node.occurrence().clone()));
        }
        Deduplication::Referent => {
            let mut seen = BTreeSet::new();
            frontier.retain(|node| seen.insert(node.referent().clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec, vec::Vec};
    use core::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use addressable::{
        AbsoluteAddress, Deduplication, Locator, Pinned, Query, QueryError, RelativeAddress,
        Resolution, Revision, SpaceId, TraversalBudget,
    };

    use super::{PredicateMatch, TreeAxis, TreeHost, TreeNode, TreeReadError, TreeRuntime};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Space {}

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum View {
        Instances,
        Unsupported,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Predicate {
        Referent(u64),
        Expensive,
    }

    #[derive(Clone, Debug)]
    struct StoredNode {
        occurrence: u64,
        referent: u64,
        parent: Option<u64>,
        address: AbsoluteAddress<Space>,
    }

    #[derive(Clone, Debug)]
    struct Host {
        nodes: Vec<StoredNode>,
        node_lookups: Cell<u32>,
        predicate_work: Cell<u32>,
    }

    impl Host {
        fn new() -> Self {
            Self {
                nodes: vec![
                    stored(0, 0, None, "/"),
                    stored(1, 1, Some(0), "/root"),
                    stored(2, 10, Some(1), "/root/a"),
                    stored(3, 10, Some(1), "/root/b"),
                    stored(4, 20, Some(2), "/root/a/leaf"),
                ],
                node_lookups: Cell::new(0),
                predicate_work: Cell::new(0),
            }
        }

        fn project(node: &StoredNode) -> TreeNode<Space, u64, u64, u64> {
            TreeNode::new(
                node.referent,
                node.occurrence,
                node.address.clone(),
                Some(node.occurrence),
            )
        }
    }

    impl TreeHost for Host {
        type Space = Space;
        type View = View;
        type Referent = u64;
        type Occurrence = u64;
        type Handle = u64;
        type Predicate = Predicate;

        fn supports_view(&self, view: &View) -> bool {
            *view == View::Instances
        }

        fn node_at(
            &self,
            _view: &View,
            address: &AbsoluteAddress<Space>,
        ) -> Option<TreeNode<Space, u64, u64, u64>> {
            self.node_lookups
                .set(self.node_lookups.get().saturating_add(1));
            self.nodes
                .iter()
                .find(|node| node.address == *address)
                .map(Self::project)
        }

        fn nodes(&self, _view: &View) -> impl Iterator<Item = TreeNode<Space, u64, u64, u64>> {
            self.nodes.iter().map(Self::project)
        }

        fn children<'a>(
            &'a self,
            _view: &'a View,
            occurrence: &'a u64,
        ) -> impl Iterator<Item = TreeNode<Space, u64, u64, u64>> + 'a {
            self.nodes
                .iter()
                .filter(|node| node.parent == Some(*occurrence))
                .map(Self::project)
        }

        fn parent(&self, _view: &View, occurrence: &u64) -> Option<TreeNode<Space, u64, u64, u64>> {
            let parent = self
                .nodes
                .iter()
                .find(|node| node.occurrence == *occurrence)?
                .parent?;
            self.nodes
                .iter()
                .find(|node| node.occurrence == parent)
                .map(Self::project)
        }

        fn matches(
            &self,
            node: &TreeNode<Space, u64, u64, u64>,
            predicate: &Predicate,
        ) -> PredicateMatch {
            match predicate {
                Predicate::Referent(referent) => {
                    PredicateMatch::new(node.referent() == referent, 1)
                }
                Predicate::Expensive => {
                    self.predicate_work
                        .set(self.predicate_work.get().saturating_add(4));
                    PredicateMatch::new(true, 4)
                }
            }
        }
    }

    fn stored(occurrence: u64, referent: u64, parent: Option<u64>, address: &str) -> StoredNode {
        StoredNode {
            occurrence,
            referent,
            parent,
            address: AbsoluteAddress::parse(address).expect("static test address is valid"),
        }
    }

    fn runtime() -> TreeRuntime<Host> {
        TreeRuntime::new(SpaceId::new(7), Host::new())
    }

    #[test]
    fn exact_relative_and_pinned_resolution_share_one_host_projection() {
        let runtime = runtime();
        let exact = Locator::exact(
            runtime.id(),
            View::Instances,
            AbsoluteAddress::parse("/root/a").expect("valid exact address"),
        );
        let relative = Locator::relative(
            runtime.id(),
            View::Instances,
            AbsoluteAddress::parse("/root").expect("valid base"),
            RelativeAddress::parse("a").expect("valid relative address"),
        );
        let Resolution::Resolved(exact_location) = runtime.resolve(&exact) else {
            panic!("exact locator resolves");
        };
        let Resolution::Resolved(relative_location) = runtime.resolve(&relative) else {
            panic!("relative locator resolves");
        };
        assert_eq!(
            exact_location, relative_location,
            "exact and relative recipes reach one location"
        );

        let pin = Pinned::from_location(&exact_location);
        assert!(
            matches!(runtime.resolve_pinned(&pin), Resolution::Resolved(_)),
            "fresh pin resolves normally"
        );
        assert!(
            matches!(
                runtime.resolve(&Locator::exact(
                    runtime.id(),
                    View::Unsupported,
                    AbsoluteAddress::root(),
                )),
                Resolution::UnsupportedLocator
            ),
            "unsupported view is explicit"
        );
    }

    #[test]
    fn queries_apply_tree_axes_cardinality_and_referent_deduplication() {
        let runtime = runtime();
        let all = runtime
            .query_many(
                &Query::many(runtime.root_locator(View::Instances)).traverse(TreeAxis::Descendants),
            )
            .expect("descendant query succeeds");
        assert_eq!(all.items().len(), 4, "root has four descendants");

        runtime.host().node_lookups.set(0);
        let shared = runtime
            .query_many(
                &Query::many(runtime.root_locator(View::Instances))
                    .traverse(TreeAxis::Descendants)
                    .filter(Predicate::Referent(10))
                    .deduplicate(Deduplication::Referent),
            )
            .expect("referent query succeeds");
        assert_eq!(
            shared.items().len(),
            1,
            "referent deduplication collapses two occurrences"
        );
        assert_eq!(
            runtime.host().node_lookups.get(),
            1,
            "filters reuse projected frontier nodes"
        );

        let one = runtime
            .query_one(
                &Query::one(runtime.root_locator(View::Instances))
                    .traverse(TreeAxis::Descendants)
                    .filter(Predicate::Referent(20)),
            )
            .expect("one leaf matches");
        assert_eq!(
            one.value().address().to_string(),
            "/root/a/leaf",
            "one-result query retains exact context"
        );
    }

    #[test]
    fn predicate_evaluation_reports_host_defined_work() {
        let runtime = runtime();
        let results = runtime
            .query_many(
                &Query::many(runtime.root_locator(View::Instances))
                    .traverse(TreeAxis::Descendants)
                    .filter(Predicate::Expensive),
            )
            .expect("query stays within the default budget");

        assert_eq!(
            results.stats().work_units,
            5 + runtime.host().predicate_work.get(),
            "query statistics must report the work the host performed"
        );
    }

    #[test]
    fn budget_failure_and_host_replacement_are_revision_safe() {
        let mut runtime = runtime();
        let query = Query::many(runtime.root_locator(View::Instances))
            .traverse(TreeAxis::Descendants)
            .budget(TraversalBudget::new(1, 10, 10, 10));
        assert!(
            matches!(
                runtime.query_many(&query),
                Err(QueryError::BudgetExceeded(_))
            ),
            "depth limit stops recursive traversal"
        );

        let locator = Locator::exact(
            runtime.id(),
            View::Instances,
            AbsoluteAddress::parse("/root/a").expect("valid address"),
        );
        let Resolution::Resolved(location) = runtime.resolve(&locator) else {
            panic!("handle target resolves");
        };
        let handle = runtime
            .resolved_handle(&location)
            .expect("projected node has a handle");
        let old_revision = runtime.revision();
        runtime.replace_host(Host::new());
        assert_eq!(
            runtime.revision(),
            old_revision.next(),
            "whole-host commit advances exactly once"
        );
        assert!(
            matches!(
                runtime.validate_location(&location),
                Err(TreeReadError::StaleRevision { .. })
            ),
            "old location is stale"
        );
        assert_ne!(
            handle.revision(),
            runtime.revision(),
            "old handle remains tied to its observation revision"
        );
    }

    #[test]
    fn commit_and_reconstruction_preserve_the_revision_clock() {
        let mut runtime = runtime();
        let locator = Locator::exact(
            runtime.id(),
            View::Instances,
            AbsoluteAddress::parse("/root/a").expect("valid address"),
        );
        let Resolution::Resolved(location) = runtime.resolve(&locator) else {
            panic!("pin target resolves");
        };
        let pin = Pinned::from_location(&location);

        runtime.commit(|host| host.nodes.reverse());
        assert!(matches!(
            runtime.resolve_pinned(&pin),
            Resolution::StaleRevision { .. }
        ));

        let (revision, host) = runtime.into_host();
        let reconstructed = TreeRuntime::from_revision(revision, host);
        assert_eq!(reconstructed.revision(), revision);
        assert!(matches!(
            reconstructed.resolve_pinned(&pin),
            Resolution::StaleRevision { .. }
        ));
    }

    #[test]
    fn caught_commit_panic_cannot_preserve_the_old_revision() {
        let mut runtime = runtime();
        let revision = runtime.revision();

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            runtime.commit(|host| {
                host.nodes.reverse();
                panic!("mutation failed after writing");
            });
        }));

        assert!(outcome.is_err());
        assert_eq!(runtime.revision(), revision.next());
    }

    #[test]
    fn exhausted_revision_rejects_host_changes_before_mutation() {
        let space = SpaceId::new(7);
        let revision = Revision::new(space, u64::MAX);
        let mut runtime = TreeRuntime::from_revision(revision, Host::new());
        let node_count = runtime.host().nodes.len();

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            runtime.commit(|host| host.nodes.clear());
        }));

        assert!(outcome.is_err());
        assert_eq!(runtime.revision(), revision);
        assert_eq!(runtime.host().nodes.len(), node_count);

        let mut replacement = Host::new();
        replacement.nodes.clear();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            runtime.replace_host(replacement);
        }));

        assert!(outcome.is_err());
        assert_eq!(runtime.revision(), revision);
        assert_eq!(runtime.host().nodes.len(), node_count);
    }
}
