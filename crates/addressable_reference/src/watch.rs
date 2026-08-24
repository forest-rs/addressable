// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Scanning live-query watch with coherent delta semantics.

use addressable::{
    Deduplication, DeltaError, LiveQueryId, Many, QueryDelta, QueryError, QuerySnapshot,
    ResultEntry, ResultIdentity,
};

use crate::{Basilica, BasilicaLocation, BasilicaQuery, BasilicaSpace, OccurrenceId};

/// A live many-result query tracked by occurrence identity.
#[derive(Clone, Debug)]
pub struct BasilicaWatch {
    query: BasilicaQuery<Many>,
    snapshot: QuerySnapshot<BasilicaSpace, OccurrenceId, BasilicaLocation>,
}

impl BasilicaWatch {
    /// Returns the typed query being watched.
    #[must_use]
    pub const fn query(&self) -> &BasilicaQuery<Many> {
        &self.query
    }

    /// Returns the latest complete snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> &QuerySnapshot<BasilicaSpace, OccurrenceId, BasilicaLocation> {
        &self.snapshot
    }

    /// Recomputes against the current revision and emits a coherent delta.
    ///
    /// The scanning implementation is replaceable; the delta contract is not.
    pub fn poll(
        &mut self,
        space: &Basilica,
    ) -> Result<QueryDelta<BasilicaSpace, OccurrenceId, BasilicaLocation>, WatchError> {
        let next = snapshot(space, self.snapshot.live_query(), &self.query)?;
        let delta = QueryDelta::between(&self.snapshot, &next).map_err(WatchError::Delta)?;
        self.snapshot = next;
        Ok(delta)
    }
}

impl Basilica {
    /// Starts a scanning live query.
    ///
    /// This first watcher deliberately accepts occurrence deduplication only,
    /// making its stable entry identity explicit.
    pub fn watch(&mut self, query: BasilicaQuery<Many>) -> Result<BasilicaWatch, WatchError> {
        if query.semantics().deduplication != Deduplication::Occurrence {
            return Err(WatchError::IdentityNotOccurrence);
        }
        let live_query = LiveQueryId::new(self.next_live_query);
        self.next_live_query = self
            .next_live_query
            .checked_add(1)
            .ok_or(WatchError::LiveQueryIdsExhausted)?;
        let snapshot = snapshot(self, live_query, &query)?;
        Ok(BasilicaWatch { query, snapshot })
    }
}

fn snapshot(
    space: &Basilica,
    live_query: LiveQueryId<BasilicaSpace>,
    query: &BasilicaQuery<Many>,
) -> Result<QuerySnapshot<BasilicaSpace, OccurrenceId, BasilicaLocation>, WatchError> {
    let results = space.query_many(query).map_err(WatchError::Query)?;
    Ok(QuerySnapshot::new(
        live_query,
        space.revision(),
        ResultIdentity::Occurrence,
        results
            .items()
            .iter()
            .cloned()
            .map(|location| ResultEntry::new(*location.occurrence(), location)),
    ))
}

/// Failure to start or advance a basilica watch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchError {
    /// This watcher requires occurrence-deduplicated results.
    IdentityNotOccurrence,
    /// The host cannot allocate another unique live-query identity.
    LiveQueryIdsExhausted,
    /// Full query recomputation failed.
    Query(QueryError),
    /// Snapshot differencing failed.
    Delta(DeltaError<BasilicaSpace>),
}

#[cfg(test)]
mod tests {
    use addressable::{
        CyclePolicy, Deduplication, Endpoint, Guard, Query, SpaceId, Transaction, VisitIdentity,
    };

    use crate::{
        Basilica, BasilicaAxis, BasilicaPredicate, BasilicaSpace, EditCapability, FeatureId, Load,
        SetLoad,
    };

    #[test]
    fn emitted_delta_replays_to_full_recomputation() {
        let mut space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        let query = Query::many(space.root_locator())
            .traverse(BasilicaAxis::Descendants)
            .filter(BasilicaPredicate::LoadAtLeast(100))
            .deduplicate(Deduplication::Occurrence)
            .cycles(CyclePolicy::SkipVisited(VisitIdentity::Occurrence));
        let mut watch = space.watch(query).expect("watch starts");
        let mut replayed = watch.snapshot().clone();
        let arch = replayed
            .entries()
            .iter()
            .find(|entry| *entry.value().referent() == FeatureId::new(3))
            .expect("arch result exists")
            .value()
            .clone();
        let edit = SetLoad::new(
            Endpoint::new(arch, Load),
            80,
            Guard::new(
                FeatureId::new(3),
                space.revision(),
                120,
                EditCapability::SetLoad,
            ),
        );
        space
            .transact(Transaction::apply(space.revision(), [edit]))
            .expect("guarded edit applies");

        let delta = watch.poll(&space).expect("watch advances");
        replayed.apply(&delta).expect("delta replays");
        assert_eq!(&replayed, watch.snapshot());
    }

    #[test]
    fn exhausted_live_query_ids_do_not_wrap() {
        let mut space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        space.next_live_query = u64::MAX;
        let query = Query::many(space.root_locator());

        assert!(matches!(
            space.watch(query),
            Err(super::WatchError::LiveQueryIdsExhausted)
        ));
        assert_eq!(space.next_live_query, u64::MAX);
    }
}
