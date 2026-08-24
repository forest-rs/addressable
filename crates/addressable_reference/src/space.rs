// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Basilica construction, resolution, query execution, and typed reads.

use std::{collections::VecDeque, vec::Vec};

use addressable::{
    AbsoluteAddress, BudgetDimension, BudgetExceeded, Cardinality, CardinalityKind, CyclePolicy,
    Deduplication, Endpoint, Explained, Locator, Many, One, Opinion, Optional, Pinned, QueryError,
    QueryResults, QuerySemantics, QueryStats, QueryStep, Resolution, ResolvedHandle,
    ResultOrdering, Revision, SpaceId, TraversalBudget, VisitIdentity,
};

use crate::model::{
    BasilicaAxis, BasilicaLocation, BasilicaLocator, BasilicaPredicate, BasilicaQuery,
    BasilicaResolution, BasilicaSpace, BasilicaView, Edge, EdgeId, EdgeKind, Feature, FeatureId,
    FeatureKind, Load, LoadProvenance, LoadReason, Occurrence, OccurrenceId, SlotHandle,
};

/// Measured single-value or optional query output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measured<T> {
    value: T,
    stats: QueryStats,
}

impl<T> Measured<T> {
    /// Pairs a cardinality-shaped value with measured query work.
    #[must_use]
    pub const fn new(value: T, stats: QueryStats) -> Self {
        Self { value, stats }
    }

    /// Returns the cardinality-shaped value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns measured query work.
    #[must_use]
    pub const fn stats(&self) -> QueryStats {
        self.stats
    }

    /// Decomposes the measured value.
    #[must_use]
    pub fn into_parts(self) -> (T, QueryStats) {
        (self.value, self.stats)
    }
}

/// The complete scanning reference basilica space.
#[derive(Clone, Debug)]
pub struct Basilica {
    pub(crate) id: SpaceId<BasilicaSpace>,
    pub(crate) revision: Revision<BasilicaSpace>,
    pub(crate) next_live_query: u64,
    pub(crate) features: Vec<Feature>,
    pub(crate) occurrences: Vec<Occurrence>,
    pub(crate) edges: Vec<Edge>,
}

impl Basilica {
    /// Constructs the deterministic reference basilica.
    ///
    /// The arch referent appears at both `north_arch` and `south_arch` in the
    /// assembly view. The dependency view contains an arch/vault cycle.
    #[must_use]
    pub fn new(id: SpaceId<BasilicaSpace>) -> Self {
        let revision = Revision::initial(id);
        let features = vec![
            feature(
                1,
                "Basilica",
                FeatureKind::Basilica,
                0,
                None,
                revision,
                false,
            ),
            feature(2, "Nave", FeatureKind::Nave, 20, None, revision, false),
            feature(3, "Arch", FeatureKind::Arch, 100, Some(120), revision, true),
            feature(
                4,
                "Vault",
                FeatureKind::Vault,
                180,
                Some(200),
                revision,
                true,
            ),
            feature(5, "Altar", FeatureKind::Altar, 40, None, revision, false),
        ];
        let occurrences = vec![
            occurrence(1, 1, BasilicaView::Assembly, "/basilica"),
            occurrence(2, 2, BasilicaView::Assembly, "/basilica/nave"),
            occurrence(3, 3, BasilicaView::Assembly, "/basilica/nave/north_arch"),
            occurrence(4, 3, BasilicaView::Assembly, "/basilica/nave/south_arch"),
            occurrence(5, 4, BasilicaView::Assembly, "/basilica/nave/vault"),
            occurrence(6, 5, BasilicaView::Assembly, "/basilica/altar"),
            occurrence(10, 1, BasilicaView::Dependency, "/dependencies/basilica"),
            occurrence(11, 3, BasilicaView::Dependency, "/dependencies/arch"),
            occurrence(12, 4, BasilicaView::Dependency, "/dependencies/vault"),
            occurrence(13, 2, BasilicaView::Dependency, "/dependencies/nave"),
        ];
        let edges = vec![
            edge(1, 1, 2, EdgeKind::Assembly),
            edge(2, 1, 6, EdgeKind::Assembly),
            edge(3, 2, 3, EdgeKind::Assembly),
            edge(4, 2, 4, EdgeKind::Assembly),
            edge(5, 2, 5, EdgeKind::Assembly),
            edge(10, 10, 13, EdgeKind::Dependency),
            edge(11, 13, 11, EdgeKind::Dependency),
            edge(12, 11, 12, EdgeKind::Dependency),
            edge(13, 12, 11, EdgeKind::Dependency),
        ];
        Self {
            id,
            revision,
            next_live_query: 0,
            features,
            occurrences,
            edges,
        }
    }

    /// Returns the runtime identity of this address-space instance.
    #[must_use]
    pub const fn id(&self) -> SpaceId<BasilicaSpace> {
        self.id
    }

    /// Returns the current local revision.
    #[must_use]
    pub const fn revision(&self) -> Revision<BasilicaSpace> {
        self.revision
    }

    /// Iterates the identities of addressable relationship occurrences.
    ///
    /// Edge identity remains distinct from both endpoint occurrence ids.
    pub fn edge_ids(&self) -> impl ExactSizeIterator<Item = EdgeId> + '_ {
        self.edges.iter().map(|edge| edge.id)
    }

    /// Returns an exact locator for the assembly root.
    #[must_use]
    pub fn root_locator(&self) -> BasilicaLocator {
        Locator::exact(
            self.id,
            BasilicaView::Assembly,
            AbsoluteAddress::parse("/basilica").expect("static root address must be valid"),
        )
    }

    /// Resolves an exact or relative locator with rich outcome semantics.
    #[must_use]
    pub fn resolve(&self, locator: &BasilicaLocator) -> BasilicaResolution {
        if locator.space() != self.id {
            return Resolution::UnsupportedLocator;
        }
        let Ok(address) = locator.to_absolute() else {
            return Resolution::UnsupportedLocator;
        };
        self.occurrences
            .iter()
            .find(|occurrence| occurrence.view == *locator.view() && occurrence.address == address)
            .map_or(Resolution::Absent, |occurrence| {
                Resolution::Resolved(self.location(occurrence))
            })
    }

    /// Resolves a pinned locator without silently accepting rebinding.
    #[must_use]
    pub fn resolve_pinned(
        &self,
        pinned: &Pinned<BasilicaSpace, BasilicaView, FeatureId>,
    ) -> BasilicaResolution {
        let locator = pinned.locator();
        if locator.space() != self.id {
            return Resolution::UnsupportedLocator;
        }
        let Ok(address) = locator.to_absolute() else {
            return Resolution::UnsupportedLocator;
        };
        if let Some(occurrence) = self
            .occurrences
            .iter()
            .find(|occurrence| occurrence.view == *locator.view() && occurrence.address == address)
        {
            let location = self.location(occurrence);
            if occurrence.referent != *pinned.expected_referent() {
                return Resolution::Rebound {
                    expected: *pinned.expected_referent(),
                    actual: occurrence.referent,
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
            .occurrences
            .iter()
            .filter(|occurrence| {
                occurrence.view == *locator.view()
                    && occurrence.referent == *pinned.expected_referent()
            })
            .map(|occurrence| self.location(occurrence))
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

    /// Executes a many-result query.
    pub fn query_many(
        &self,
        query: &BasilicaQuery<Many>,
    ) -> Result<QueryResults<BasilicaLocation>, QueryError> {
        self.execute(query)
    }

    /// Executes a query that requires exactly one result.
    pub fn query_one(
        &self,
        query: &BasilicaQuery<One>,
    ) -> Result<Measured<BasilicaLocation>, QueryError> {
        let results = self.execute(query)?;
        let (items, stats) = results.into_parts();
        if items.len() != 1 {
            return Err(QueryError::Cardinality {
                expected: CardinalityKind::One,
                actual: items.len(),
            });
        }
        let item = items
            .into_vec()
            .pop()
            .expect("cardinality was checked as exactly one");
        Ok(Measured::new(item, stats))
    }

    /// Executes a query that allows zero or one result.
    pub fn query_optional(
        &self,
        query: &BasilicaQuery<Optional>,
    ) -> Result<Measured<Option<BasilicaLocation>>, QueryError> {
        let results = self.execute(query)?;
        let (items, stats) = results.into_parts();
        if items.len() > 1 {
            return Err(QueryError::Cardinality {
                expected: CardinalityKind::Optional,
                actual: items.len(),
            });
        }
        Ok(Measured::new(items.into_vec().pop(), stats))
    }

    /// Resolves a revision-scoped runtime feature slot.
    pub fn resolved_handle(
        &self,
        location: &BasilicaLocation,
    ) -> Result<ResolvedHandle<BasilicaSpace, SlotHandle>, ReadError> {
        self.validate_location(location)?;
        let index = self
            .features
            .iter()
            .position(|feature| feature.id == *location.referent())
            .ok_or(ReadError::MissingReferent)?;
        let slot = u32::try_from(index).map_err(|_| ReadError::MissingReferent)?;
        Ok(ResolvedHandle::new(self.revision, SlotHandle::new(slot)))
    }

    /// Reads and explains the effective typed load endpoint.
    pub fn read_load(
        &self,
        endpoint: &Endpoint<BasilicaLocation, Load>,
    ) -> Result<Explained<FeatureId, i64, LoadProvenance, LoadReason>, ReadError> {
        self.validate_location(endpoint.owner())?;
        let feature = self
            .feature(*endpoint.owner().referent())
            .ok_or(ReadError::MissingReferent)?;
        let default = Opinion::new(
            feature.default_load,
            LoadProvenance::Default {
                rule: "basilica/load/default",
            },
        );
        match feature.authored_load {
            Some(authored) => Explained::new(
                feature.id,
                [
                    Opinion::new(
                        authored,
                        LoadProvenance::Authored {
                            revision: feature.authored_revision,
                        },
                    ),
                    default,
                ],
                0,
                LoadReason::AuthoredOverridesDefault,
            )
            .map_err(|_| ReadError::InvalidExplanation),
            None => Explained::new(feature.id, [default], 0, LoadReason::DefaultUsed)
                .map_err(|_| ReadError::InvalidExplanation),
        }
    }

    pub(crate) fn feature(&self, id: FeatureId) -> Option<&Feature> {
        self.features.iter().find(|feature| feature.id == id)
    }

    pub(crate) fn feature_mut(&mut self, id: FeatureId) -> Option<&mut Feature> {
        self.features.iter_mut().find(|feature| feature.id == id)
    }

    pub(crate) fn occurrence(&self, id: OccurrenceId) -> Option<&Occurrence> {
        self.occurrences
            .iter()
            .find(|occurrence| occurrence.id == id)
    }

    pub(crate) fn location(&self, occurrence: &Occurrence) -> BasilicaLocation {
        BasilicaLocation::new(
            occurrence.view,
            self.revision,
            occurrence.referent,
            occurrence.id,
            occurrence.address.clone(),
        )
    }

    pub(crate) fn validate_location(&self, location: &BasilicaLocation) -> Result<(), ReadError> {
        if location.space() != self.id {
            return Err(ReadError::WrongSpace);
        }
        if location.revision() != self.revision {
            return Err(ReadError::StaleRevision {
                expected: location.revision(),
                actual: self.revision,
            });
        }
        let occurrence = self
            .occurrence(*location.occurrence())
            .ok_or(ReadError::MissingOccurrence)?;
        if occurrence.referent != *location.referent() {
            return Err(ReadError::Rebound);
        }
        Ok(())
    }

    fn execute<C: Cardinality>(
        &self,
        query: &BasilicaQuery<C>,
    ) -> Result<QueryResults<BasilicaLocation>, QueryError> {
        let Resolution::Resolved(start) = self.resolve(query.start()) else {
            return Err(QueryError::StartDidNotResolve);
        };
        let semantics = query.semantics();
        let mut stats = QueryStats::default();
        charge(&mut stats, semantics.budget, 1, 1, 0)?;
        let mut frontier = vec![start];

        for step in query.steps() {
            frontier = match step {
                QueryStep::Traverse(axis) => {
                    self.traverse(&frontier, *axis, semantics, &mut stats)?
                }
                QueryStep::Filter(predicate) => {
                    let inspected = u32::try_from(frontier.len()).unwrap_or(u32::MAX);
                    let filtered = frontier
                        .into_iter()
                        .filter(|location| self.matches(location, predicate))
                        .collect();
                    charge_work(&mut stats, semantics.budget, inspected)?;
                    filtered
                }
            };
        }

        deduplicate(&mut frontier, semantics.deduplication);
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
        Ok(QueryResults::new(frontier, stats))
    }

    fn traverse(
        &self,
        frontier: &[BasilicaLocation],
        axis: BasilicaAxis,
        semantics: QuerySemantics,
        stats: &mut QueryStats,
    ) -> Result<Vec<BasilicaLocation>, QueryError> {
        let mut output = Vec::new();
        for location in frontier {
            match axis {
                BasilicaAxis::Children => {
                    if *location.view() != BasilicaView::Assembly {
                        return Err(QueryError::UnsupportedStep);
                    }
                    self.push_direct(
                        location,
                        EdgeKind::Assembly,
                        false,
                        semantics.budget,
                        stats,
                        &mut output,
                    )?;
                }
                BasilicaAxis::Dependencies => {
                    if *location.view() != BasilicaView::Dependency {
                        return Err(QueryError::UnsupportedStep);
                    }
                    self.push_direct(
                        location,
                        EdgeKind::Dependency,
                        false,
                        semantics.budget,
                        stats,
                        &mut output,
                    )?;
                }
                BasilicaAxis::Dependents => {
                    if *location.view() != BasilicaView::Dependency {
                        return Err(QueryError::UnsupportedStep);
                    }
                    self.push_direct(
                        location,
                        EdgeKind::Dependency,
                        true,
                        semantics.budget,
                        stats,
                        &mut output,
                    )?;
                }
                BasilicaAxis::ToView(view) => {
                    for occurrence in self.occurrences.iter().filter(|occurrence| {
                        occurrence.view == view && occurrence.referent == *location.referent()
                    }) {
                        charge(stats, semantics.budget, 1, 1, 1)?;
                        output.push(self.location(occurrence));
                    }
                }
                BasilicaAxis::Descendants => {
                    self.push_descendants(location, semantics, stats, &mut output)?;
                }
            }
        }
        Ok(output)
    }

    fn push_direct(
        &self,
        location: &BasilicaLocation,
        kind: EdgeKind,
        reverse: bool,
        budget: TraversalBudget,
        stats: &mut QueryStats,
        output: &mut Vec<BasilicaLocation>,
    ) -> Result<(), QueryError> {
        for edge in self.edges.iter().filter(|edge| {
            edge.kind == kind
                && if reverse {
                    edge.to == *location.occurrence()
                } else {
                    edge.from == *location.occurrence()
                }
        }) {
            let target = if reverse { edge.from } else { edge.to };
            let occurrence = self.occurrence(target).ok_or(QueryError::UnsupportedStep)?;
            charge(stats, budget, 1, 1, 1)?;
            output.push(self.location(occurrence));
        }
        Ok(())
    }

    fn push_descendants(
        &self,
        start: &BasilicaLocation,
        semantics: QuerySemantics,
        stats: &mut QueryStats,
        output: &mut Vec<BasilicaLocation>,
    ) -> Result<(), QueryError> {
        let kind = match start.view() {
            BasilicaView::Assembly => EdgeKind::Assembly,
            BasilicaView::Dependency => EdgeKind::Dependency,
        };
        let mut queue = VecDeque::from([(*start.occurrence(), 0_u32)]);
        let mut visited_occurrences = vec![*start.occurrence()];
        let mut visited_referents = vec![*start.referent()];

        while let Some((current, depth)) = queue.pop_front() {
            for edge in self
                .edges
                .iter()
                .filter(|edge| edge.kind == kind && edge.from == current)
            {
                let occurrence = self
                    .occurrence(edge.to)
                    .ok_or(QueryError::UnsupportedStep)?;
                let next_depth = depth.saturating_add(1);
                if next_depth > semantics.budget.max_depth {
                    return Err(QueryError::BudgetExceeded(BudgetExceeded::new(
                        BudgetDimension::Depth,
                        semantics.budget.max_depth,
                        next_depth,
                    )));
                }
                let revisited = match semantics.cycle_policy {
                    CyclePolicy::Error => visited_occurrences.contains(&occurrence.id),
                    CyclePolicy::SkipVisited(VisitIdentity::Occurrence) => {
                        visited_occurrences.contains(&occurrence.id)
                    }
                    CyclePolicy::SkipVisited(VisitIdentity::Referent) => {
                        visited_referents.contains(&occurrence.referent)
                    }
                };
                if revisited {
                    if semantics.cycle_policy == CyclePolicy::Error {
                        return Err(QueryError::Cycle);
                    }
                    continue;
                }
                visited_occurrences.push(occurrence.id);
                visited_referents.push(occurrence.referent);
                charge(stats, semantics.budget, 1, 1, next_depth)?;
                output.push(self.location(occurrence));
                queue.push_back((occurrence.id, next_depth));
            }
        }
        Ok(())
    }

    fn matches(&self, location: &BasilicaLocation, predicate: &BasilicaPredicate) -> bool {
        let Some(feature) = self.feature(*location.referent()) else {
            return false;
        };
        match predicate {
            BasilicaPredicate::Any => true,
            BasilicaPredicate::Kind(kind) => feature.kind == *kind,
            BasilicaPredicate::LoadAtLeast(threshold) => feature.effective_load() >= *threshold,
            BasilicaPredicate::NameContains(fragment) => feature.name.contains(fragment),
        }
    }
}

/// Failure to read through a resolved endpoint or handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The location belongs to another runtime space instance.
    WrongSpace,
    /// The location was resolved at an earlier revision.
    StaleRevision {
        /// Location revision.
        expected: Revision<BasilicaSpace>,
        /// Current space revision.
        actual: Revision<BasilicaSpace>,
    },
    /// The contextual occurrence no longer exists.
    MissingOccurrence,
    /// The occurrence now denotes another referent.
    Rebound,
    /// The semantic referent no longer exists.
    MissingReferent,
    /// Host data violated the explanation constructor invariant.
    InvalidExplanation,
}

fn feature(
    id: u64,
    name: &str,
    kind: FeatureKind,
    default_load: i64,
    authored_load: Option<i64>,
    authored_revision: Revision<BasilicaSpace>,
    editable: bool,
) -> Feature {
    Feature {
        id: FeatureId::new(id),
        name: name.into(),
        kind,
        default_load,
        authored_load,
        authored_revision,
        editable,
    }
}

fn occurrence(id: u64, referent: u64, view: BasilicaView, address: &str) -> Occurrence {
    Occurrence {
        id: OccurrenceId::new(id),
        referent: FeatureId::new(referent),
        view,
        address: AbsoluteAddress::parse(address).expect("static address must be valid"),
    }
}

const fn edge(id: u64, from: u64, to: u64, kind: EdgeKind) -> Edge {
    Edge {
        id: EdgeId::new(id),
        from: OccurrenceId::new(from),
        to: OccurrenceId::new(to),
        kind,
    }
}

fn charge(
    stats: &mut QueryStats,
    budget: TraversalBudget,
    nodes: u32,
    work: u32,
    depth: u32,
) -> Result<(), QueryError> {
    stats.visited_nodes = stats.visited_nodes.saturating_add(nodes);
    stats.work_units = stats.work_units.saturating_add(work);
    stats.max_depth_reached = stats.max_depth_reached.max(depth);
    if depth > budget.max_depth {
        return Err(QueryError::BudgetExceeded(BudgetExceeded::new(
            BudgetDimension::Depth,
            budget.max_depth,
            depth,
        )));
    }
    if stats.visited_nodes > budget.max_nodes {
        return Err(QueryError::BudgetExceeded(BudgetExceeded::new(
            BudgetDimension::Nodes,
            budget.max_nodes,
            stats.visited_nodes,
        )));
    }
    if stats.work_units > budget.max_work {
        return Err(QueryError::BudgetExceeded(BudgetExceeded::new(
            BudgetDimension::Work,
            budget.max_work,
            stats.work_units,
        )));
    }
    Ok(())
}

fn charge_work(
    stats: &mut QueryStats,
    budget: TraversalBudget,
    work: u32,
) -> Result<(), QueryError> {
    charge(stats, budget, 0, work, stats.max_depth_reached)
}

fn deduplicate(frontier: &mut Vec<BasilicaLocation>, identity: Deduplication) {
    match identity {
        Deduplication::None => {}
        Deduplication::Occurrence => {
            let mut seen = Vec::new();
            frontier.retain(|location| {
                if seen.contains(location.occurrence()) {
                    false
                } else {
                    seen.push(*location.occurrence());
                    true
                }
            });
        }
        Deduplication::Referent => {
            let mut seen = Vec::new();
            frontier.retain(|location| {
                if seen.contains(location.referent()) {
                    false
                } else {
                    seen.push(*location.referent());
                    true
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use addressable::{
        AbsoluteAddress, CyclePolicy, Deduplication, Locator, Pinned, Query, ResultOrdering,
        SpaceId, TraversalBudget, VisitIdentity,
    };

    use super::Basilica;
    use crate::{BasilicaAxis, BasilicaPredicate, BasilicaSpace, BasilicaView, FeatureKind};

    #[test]
    fn exact_relative_and_pinned_resolution_preserve_identity() {
        let mut space = Basilica::new(SpaceId::new(7));
        let north_address =
            AbsoluteAddress::parse("/basilica/nave/north_arch").expect("valid north address");
        let exact = Locator::exact(space.id(), BasilicaView::Assembly, north_address.clone());
        let addressable::Resolution::Resolved(north) = space.resolve(&exact) else {
            panic!("north arch should resolve");
        };
        let relative = Locator::relative(
            space.id(),
            BasilicaView::Assembly,
            AbsoluteAddress::parse("/basilica/nave").expect("valid base"),
            addressable::RelativeAddress::parse("south_arch").expect("valid relative path"),
        );
        let addressable::Resolution::Resolved(south) = space.resolve(&relative) else {
            panic!("south arch should resolve");
        };
        assert_eq!(north.referent(), south.referent());
        assert_ne!(north.occurrence(), south.occurrence());

        let pinned = Pinned::new(exact, *north.referent(), space.revision());
        space
            .occurrences
            .iter_mut()
            .find(|occurrence| occurrence.address == north_address)
            .expect("test occurrence exists")
            .referent = crate::FeatureId::new(5);
        assert!(matches!(
            space.resolve_pinned(&pinned),
            addressable::Resolution::Rebound { .. }
        ));
    }

    #[test]
    fn queries_make_occurrence_and_referent_deduplication_explicit() {
        let space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        let occurrence_query = Query::many(space.root_locator())
            .traverse(BasilicaAxis::Descendants)
            .filter(BasilicaPredicate::Kind(FeatureKind::Arch))
            .deduplicate(Deduplication::Occurrence)
            .order(ResultOrdering::Stable)
            .cycles(CyclePolicy::SkipVisited(VisitIdentity::Occurrence));
        let referent_query = occurrence_query
            .clone()
            .deduplicate(Deduplication::Referent);

        assert_eq!(
            space
                .query_many(&occurrence_query)
                .expect("assembly query succeeds")
                .items()
                .len(),
            2
        );
        assert_eq!(
            space
                .query_many(&referent_query)
                .expect("referent query succeeds")
                .items()
                .len(),
            1
        );
    }

    #[test]
    fn explicit_view_crossing_terminates_a_dependency_cycle() {
        let space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        let query = Query::many(space.root_locator())
            .traverse(BasilicaAxis::ToView(BasilicaView::Dependency))
            .traverse(BasilicaAxis::Descendants)
            .deduplicate(Deduplication::Occurrence)
            .cycles(CyclePolicy::SkipVisited(VisitIdentity::Occurrence))
            .budget(TraversalBudget::new(8, 32, 16, 64));
        let results = space.query_many(&query).expect("cycle policy terminates");
        assert_eq!(results.items().len(), 3);

        let error_query = query.clone().cycles(CyclePolicy::Error);
        assert_eq!(
            space.query_many(&error_query),
            Err(addressable::QueryError::Cycle)
        );
    }
}
