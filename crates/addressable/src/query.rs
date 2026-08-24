// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed query IR and explicit execution policy.

use alloc::{boxed::Box, vec::Vec};
use core::marker::PhantomData;

use crate::BudgetExceeded;

/// Marker for a query that must return exactly one result.
///
/// Select it with [`Query::one`] or [`Query::with_cardinality`], then call the
/// host's one-result execution method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct One;

/// Marker for a query that may return zero or one result.
///
/// Select it with [`Query::optional`] or [`Query::with_cardinality`], then call
/// the host's optional-result execution method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Optional;

/// Marker for a query that may return several results.
///
/// Select it with [`Query::many`] or [`Query::with_cardinality`], then call the
/// host's many-result execution or watch method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Many;

/// Runtime representation of the static cardinality marker.
///
/// Returned by [`Query::cardinality`] and used by hosts when reporting a
/// [`QueryError::Cardinality`] mismatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardinalityKind {
    /// Exactly one result is required.
    One,
    /// Zero or one result is allowed.
    Optional,
    /// Any result count within the result budget is allowed.
    Many,
}

mod sealed {
    #[expect(
        unnameable_types,
        reason = "the unnameable supertrait is what seals Cardinality"
    )]
    pub trait Sealed {}
}

/// Closed set of supported static query cardinalities.
///
/// This trait is sealed so every marker has a defined [`CardinalityKind`] and
/// hosts can exhaustively interpret the result contract.
pub trait Cardinality: sealed::Sealed {
    /// Runtime representation of this static result shape.
    const KIND: CardinalityKind;
}

impl sealed::Sealed for One {}
impl Cardinality for One {
    const KIND: CardinalityKind = CardinalityKind::One;
}

impl sealed::Sealed for Optional {}
impl Cardinality for Optional {
    const KIND: CardinalityKind = CardinalityKind::Optional;
}

impl sealed::Sealed for Many {}
impl Cardinality for Many {
    const KIND: CardinalityKind = CardinalityKind::Many;
}

/// One host-defined query operation.
///
/// Callers normally append these through [`Query::traverse`] and
/// [`Query::filter`]. Hosts inspect them through [`Query::steps`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryStep<A, P> {
    /// Traverse one typed domain axis.
    Traverse(A),
    /// Retain values matching one typed domain predicate.
    Filter(P),
}

/// Ordering promised by query execution.
///
/// Pass this to [`Query::order`]; hosts must honor the selected contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultOrdering {
    /// Preserve deterministic traversal order.
    Traversal,
    /// Sort by the host's stable semantic ordering.
    Stable,
    /// The caller does not rely on result order.
    Unordered,
}

/// Result deduplication identity.
///
/// Pass this to [`Query::deduplicate`] to choose whether repeated occurrences
/// or referents remain visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deduplication {
    /// Preserve every result entry, including repeats.
    None,
    /// Deduplicate by contextual occurrence identity.
    Occurrence,
    /// Deduplicate by semantic referent identity.
    Referent,
}

/// Identity used to detect revisitation during cyclic traversal.
///
/// Use this inside [`CyclePolicy::SkipVisited`], then pass the policy to
/// [`Query::cycles`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisitIdentity {
    /// A distinct occurrence is a distinct visit.
    Occurrence,
    /// Any occurrence of an already visited referent counts as revisitation.
    Referent,
}

/// Declared behavior when traversal encounters a cycle.
///
/// Pass this to [`Query::cycles`]. A host returns [`QueryError::Cycle`] when
/// `Error` is selected and revisitation occurs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CyclePolicy {
    /// Stop and return a cycle error.
    Error,
    /// Skip nodes already visited under the selected identity.
    SkipVisited(VisitIdentity),
}

/// Explicit upper bounds for query execution.
///
/// Construct all four limits with [`Self::new`] and pass them to
/// [`Query::budget`]. Hosts report the first exceeded dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraversalBudget {
    /// Maximum axis-traversal depth.
    pub max_depth: u32,
    /// Maximum number of nodes visited.
    pub max_nodes: u32,
    /// Maximum number of results produced.
    pub max_results: u32,
    /// Maximum host-defined work units.
    pub max_work: u32,
}

impl TraversalBudget {
    /// Creates a complete set of traversal limits.
    #[must_use]
    pub const fn new(max_depth: u32, max_nodes: u32, max_results: u32, max_work: u32) -> Self {
        Self {
            max_depth,
            max_nodes,
            max_results,
            max_work,
        }
    }
}

impl Default for TraversalBudget {
    fn default() -> Self {
        Self::new(64, 16_384, 4_096, 65_536)
    }
}

/// Shared policy that every query carries explicitly.
///
/// Callers normally set individual fields through the fluent [`Query`] methods.
/// Hosts obtain the complete copy through [`Query::semantics`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuerySemantics {
    /// Promised result ordering.
    pub ordering: ResultOrdering,
    /// Result identity used for deduplication.
    pub deduplication: Deduplication,
    /// Behavior on cyclic traversal.
    pub cycle_policy: CyclePolicy,
    /// Hard traversal limits.
    pub budget: TraversalBudget,
}

impl Default for QuerySemantics {
    fn default() -> Self {
        Self {
            ordering: ResultOrdering::Traversal,
            deduplication: Deduplication::Occurrence,
            cycle_policy: CyclePolicy::Error,
            budget: TraversalBudget::default(),
        }
    }
}

/// A typed query abstract syntax tree.
///
/// `L`, `A`, and `P` are the host's locator, axis, and predicate types. `C`
/// records result cardinality at the call site.
///
/// Callers construct a query with [`Self::many`], [`Query::one`], or
/// [`Query::optional`], add domain axes and predicates, select explicit
/// semantics, and pass it to the matching host execution method. Addressable
/// builds and exposes this portable query representation; the host owns its
/// execution.
///
/// ```
/// use addressable::{
///     CardinalityKind, CyclePolicy, Deduplication, Query, ResultOrdering,
///     VisitIdentity,
/// };
///
/// enum Axis { Descendants }
/// enum Predicate { IsArch }
/// let query = Query::many("/basilica")
///     .traverse(Axis::Descendants)
///     .filter(Predicate::IsArch)
///     .deduplicate(Deduplication::Occurrence)
///     .order(ResultOrdering::Stable)
///     .cycles(CyclePolicy::SkipVisited(VisitIdentity::Occurrence));
///
/// assert_eq!(query.cardinality(), CardinalityKind::Many);
/// assert_eq!(query.semantics().deduplication, Deduplication::Occurrence);
/// assert_eq!(query.semantics().ordering, ResultOrdering::Stable);
/// assert_eq!(
///     query.semantics().cycle_policy,
///     CyclePolicy::SkipVisited(VisitIdentity::Occurrence),
/// );
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query<L, A, P, C: Cardinality = Many> {
    start: L,
    steps: Vec<QueryStep<A, P>>,
    semantics: QuerySemantics,
    cardinality: PhantomData<fn() -> C>,
}

impl<L, A, P> Query<L, A, P, Many> {
    /// Starts a query that may return several results.
    #[must_use]
    pub fn many(start: L) -> Self {
        Self::new(start)
    }
}

impl<L, A, P> Query<L, A, P, One> {
    /// Starts a query that must return exactly one result.
    #[must_use]
    pub fn one(start: L) -> Self {
        Self::new(start)
    }
}

impl<L, A, P> Query<L, A, P, Optional> {
    /// Starts a query that may return zero or one result.
    #[must_use]
    pub fn optional(start: L) -> Self {
        Self::new(start)
    }
}

impl<L, A, P, C: Cardinality> Query<L, A, P, C> {
    fn new(start: L) -> Self {
        Self {
            start,
            steps: Vec::new(),
            semantics: QuerySemantics::default(),
            cardinality: PhantomData,
        }
    }

    /// Appends one typed traversal step.
    #[must_use]
    pub fn traverse(mut self, axis: A) -> Self {
        self.steps.push(QueryStep::Traverse(axis));
        self
    }

    /// Appends one typed predicate step.
    #[must_use]
    pub fn filter(mut self, predicate: P) -> Self {
        self.steps.push(QueryStep::Filter(predicate));
        self
    }

    /// Selects the result deduplication identity.
    #[must_use]
    pub const fn deduplicate(mut self, deduplication: Deduplication) -> Self {
        self.semantics.deduplication = deduplication;
        self
    }

    /// Selects result ordering.
    #[must_use]
    pub const fn order(mut self, ordering: ResultOrdering) -> Self {
        self.semantics.ordering = ordering;
        self
    }

    /// Selects cycle behavior.
    #[must_use]
    pub const fn cycles(mut self, cycle_policy: CyclePolicy) -> Self {
        self.semantics.cycle_policy = cycle_policy;
        self
    }

    /// Sets hard traversal limits.
    #[must_use]
    pub const fn budget(mut self, budget: TraversalBudget) -> Self {
        self.semantics.budget = budget;
        self
    }

    /// Changes only the static cardinality marker.
    ///
    /// Cardinality is closed over Addressable's three supported result shapes.
    ///
    /// ```compile_fail
    /// use addressable::{Many, Query};
    ///
    /// struct Unchecked;
    ///
    /// let query: Query<(), (), (), Many> = Query::many(());
    /// let _ = query.with_cardinality::<Unchecked>();
    /// ```
    #[must_use]
    pub fn with_cardinality<C2: Cardinality>(self) -> Query<L, A, P, C2> {
        Query {
            start: self.start,
            steps: self.steps,
            semantics: self.semantics,
            cardinality: PhantomData,
        }
    }

    /// Returns the runtime form of the static cardinality marker.
    #[must_use]
    pub const fn cardinality(&self) -> CardinalityKind {
        C::KIND
    }

    /// Returns the start locator.
    #[must_use]
    pub const fn start(&self) -> &L {
        &self.start
    }

    /// Returns query steps in execution order.
    #[must_use]
    pub fn steps(&self) -> &[QueryStep<A, P>] {
        &self.steps
    }

    /// Returns shared execution policy.
    #[must_use]
    pub const fn semantics(&self) -> QuerySemantics {
        self.semantics
    }
}

/// Host-independent query failure returned by query execution.
///
/// Callers can distinguish an unresolved start, cardinality mismatch, cycle,
/// budget exhaustion, and unsupported domain step without parsing diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError {
    /// The start locator did not resolve to an ordinary result.
    StartDidNotResolve,
    /// Static cardinality was not satisfied by the result count.
    Cardinality {
        /// Required result shape.
        expected: CardinalityKind,
        /// Actual result count.
        actual: usize,
    },
    /// Traversal encountered a cycle under [`CyclePolicy::Error`].
    Cycle,
    /// Execution exhausted a declared budget.
    BudgetExceeded(BudgetExceeded),
    /// The query requested an axis or predicate unavailable in this view.
    UnsupportedStep,
}

/// Measured work performed by a query execution.
///
/// Hosts return this inside [`QueryResults`] or another cardinality-shaped
/// result. Callers can use it for diagnostics and explicit budget tuning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QueryStats {
    /// Nodes inspected, including nodes rejected by predicates.
    pub visited_nodes: u32,
    /// Host-defined work units charged.
    pub work_units: u32,
    /// Maximum traversal depth reached.
    pub max_depth_reached: u32,
}

/// Query items paired with measured execution work.
///
/// A host returns this from many-result query execution. Callers inspect
/// [`Self::items`] and can use [`Self::stats`] for budgeting or diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResults<T> {
    items: Box<[T]>,
    stats: QueryStats,
}

impl<T> QueryResults<T> {
    /// Creates a measured result collection on behalf of a query host.
    #[must_use]
    pub fn new(items: impl IntoIterator<Item = T>, stats: QueryStats) -> Self {
        Self {
            items: items.into_iter().collect::<Vec<_>>().into_boxed_slice(),
            stats,
        }
    }

    /// Returns result items.
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Returns measured query work.
    #[must_use]
    pub const fn stats(&self) -> QueryStats {
        self.stats
    }

    /// Decomposes the result.
    #[must_use]
    pub fn into_parts(self) -> (Box<[T]>, QueryStats) {
        (self.items, self.stats)
    }
}

#[cfg(test)]
mod tests {
    use super::{CardinalityKind, Many, One, Optional, Query};

    #[test]
    fn static_cardinality_has_one_runtime_kind() {
        let one: Query<(), (), (), One> = Query::one(());
        let optional: Query<(), (), (), Optional> = Query::optional(());
        let many: Query<(), (), (), Many> = Query::many(());

        assert_eq!(one.cardinality(), CardinalityKind::One);
        assert_eq!(optional.cardinality(), CardinalityKind::Optional);
        assert_eq!(many.cardinality(), CardinalityKind::Many);
    }
}
