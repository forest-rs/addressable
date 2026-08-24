// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Coherent query snapshots and replayable structural deltas.

use alloc::{boxed::Box, vec::Vec};
use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::Revision;

/// Host-assigned identity of one live query within a typed space instance.
///
/// The id is interpreted together with the space carried by a [`Revision`].
/// Addressable does not prescribe allocation or require atomics.
/// Live-query hosts create and retain it; consumers receive it through
/// [`QuerySnapshot::live_query`] and [`QueryDelta::live_query`].
pub struct LiveQueryId<S> {
    raw: u64,
    marker: PhantomData<fn() -> S>,
}

impl<S> Copy for LiveQueryId<S> {}

impl<S> Clone for LiveQueryId<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> fmt::Debug for LiveQueryId<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("LiveQueryId")
            .field(&self.raw)
            .finish()
    }
}

impl<S> PartialEq for LiveQueryId<S> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<S> Eq for LiveQueryId<S> {}

impl<S> PartialOrd for LiveQueryId<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S> Ord for LiveQueryId<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<S> Hash for LiveQueryId<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<S> LiveQueryId<S> {
    /// Creates a live-query id from a host-assigned value.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self {
            raw,
            marker: PhantomData,
        }
    }

    /// Returns the host-assigned value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.raw
    }
}

/// Identity used to track live result entries.
///
/// A live-query host records this in snapshots and deltas according to the
/// query's deduplication contract. Consumers inspect it before replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultIdentity {
    /// Contextual occurrence identity.
    Occurrence,
    /// Semantic referent identity.
    Referent,
    /// A host-assigned result-entry identity.
    Entry,
}

/// One stable live-query result entry.
///
/// Live-query hosts construct entries; consumers use [`Self::key`] to track
/// stable identity and [`Self::value`] for the current located result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultEntry<K, T> {
    key: K,
    value: T,
}

impl<K, T> ResultEntry<K, T> {
    /// Creates a keyed result entry on behalf of a live-query host.
    #[must_use]
    pub const fn new(key: K, value: T) -> Self {
        Self { key, value }
    }

    /// Returns stable result identity.
    #[must_use]
    pub const fn key(&self) -> &K {
        &self.key
    }

    /// Returns the located result value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }
}

/// A complete live-query result at one revision.
///
/// A live-query host produces an initial snapshot and subsequent
/// [`QueryDelta`] values. Consumers retain the snapshot and call
/// [`Self::apply`] for each delta in order. Replay is atomic: an invalid delta
/// leaves the snapshot unchanged.
///
/// ```
/// use addressable::{
///     LiveQueryId, QueryDelta, QuerySnapshot, ResultEntry, ResultIdentity,
///     Revision, SpaceId,
/// };
///
/// #[derive(Debug, PartialEq, Eq)]
/// enum Space {}
/// let space = SpaceId::<Space>::new(1);
/// let stream = LiveQueryId::new(9);
/// let mut current = QuerySnapshot::new(
///     stream,
///     Revision::new(space, 3),
///     ResultIdentity::Entry,
///     [ResultEntry::new("north", 120_i64)],
/// );
/// let next = QuerySnapshot::new(
///     stream,
///     Revision::new(space, 4),
///     ResultIdentity::Entry,
///     [ResultEntry::new("north", 80_i64)],
/// );
/// let delta = QueryDelta::between(&current, &next)?;
/// current.apply(&delta)?;
///
/// assert_eq!(current, next);
/// # Ok::<(), addressable::DeltaError<Space>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuerySnapshot<S, K, T> {
    live_query: LiveQueryId<S>,
    revision: Revision<S>,
    identity: ResultIdentity,
    entries: Vec<ResultEntry<K, T>>,
}

impl<S, K, T> QuerySnapshot<S, K, T> {
    /// Creates a complete snapshot on behalf of a live-query host.
    #[must_use]
    pub fn new(
        live_query: LiveQueryId<S>,
        revision: Revision<S>,
        identity: ResultIdentity,
        entries: impl IntoIterator<Item = ResultEntry<K, T>>,
    ) -> Self {
        Self {
            live_query,
            revision,
            identity,
            entries: entries.into_iter().collect(),
        }
    }

    /// Returns the live query whose result this snapshot represents.
    #[must_use]
    pub const fn live_query(&self) -> LiveQueryId<S> {
        self.live_query
    }

    /// Returns the object-space revision.
    #[must_use]
    pub const fn revision(&self) -> Revision<S> {
        self.revision
    }

    /// Returns the declared live-entry identity.
    #[must_use]
    pub const fn identity(&self) -> ResultIdentity {
        self.identity
    }

    /// Returns ordered result entries.
    #[must_use]
    pub fn entries(&self) -> &[ResultEntry<K, T>] {
        &self.entries
    }
}

impl<S, K, T> QuerySnapshot<S, K, T>
where
    K: Clone + Eq,
    T: Clone + Eq,
{
    /// Applies one delta atomically.
    ///
    /// On error, `self` is unchanged.
    pub fn apply(&mut self, delta: &QueryDelta<S, K, T>) -> Result<(), DeltaError<S>> {
        if self.live_query != delta.live_query {
            return Err(DeltaError::LiveQueryMismatch);
        }
        if delta.from_revision.space() != delta.to_revision.space() {
            return Err(DeltaError::SpaceMismatch);
        }
        if self.revision != delta.from_revision {
            return Err(DeltaError::WrongRevision {
                expected: self.revision,
                actual: delta.from_revision,
            });
        }
        if self.identity != delta.identity {
            return Err(DeltaError::IdentityMismatch);
        }

        let mut entries = self.entries.clone();
        for change in &delta.changes {
            match change {
                QueryChange::Added { index, entry } => {
                    if *index > entries.len() {
                        return Err(DeltaError::IndexOutOfBounds { index: *index });
                    }
                    if entries.iter().any(|existing| existing.key == entry.key) {
                        return Err(DeltaError::DuplicateKey);
                    }
                    entries.insert(*index, entry.clone());
                }
                QueryChange::Removed { index, entry } => {
                    let Some(existing) = entries.get(*index) else {
                        return Err(DeltaError::IndexOutOfBounds { index: *index });
                    };
                    if existing != entry {
                        return Err(DeltaError::EntryMismatch);
                    }
                    entries.remove(*index);
                }
                QueryChange::Updated { index, old, new } => {
                    let Some(existing) = entries.get_mut(*index) else {
                        return Err(DeltaError::IndexOutOfBounds { index: *index });
                    };
                    if existing != old || old.key != new.key {
                        return Err(DeltaError::EntryMismatch);
                    }
                    *existing = new.clone();
                }
                QueryChange::Moved { key, from, to } => {
                    let Some(existing) = entries.get(*from) else {
                        return Err(DeltaError::IndexOutOfBounds { index: *from });
                    };
                    if &existing.key != key {
                        return Err(DeltaError::KeyMismatch);
                    }
                    let entry = entries.remove(*from);
                    if *to > entries.len() {
                        return Err(DeltaError::IndexOutOfBounds { index: *to });
                    }
                    entries.insert(*to, entry);
                }
                QueryChange::Rebound { index, old, new } => {
                    let Some(existing) = entries.get_mut(*index) else {
                        return Err(DeltaError::IndexOutOfBounds { index: *index });
                    };
                    if existing != old || old.key != new.key {
                        return Err(DeltaError::EntryMismatch);
                    }
                    *existing = new.clone();
                }
            }
        }

        self.entries = entries;
        self.revision = delta.to_revision;
        Ok(())
    }
}

/// One replayable structural change in a live query.
///
/// Hosts emit these through [`QueryDelta::changes`]. Consumers can render the
/// individual events or replay the whole delta with [`QuerySnapshot::apply`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryChange<K, T> {
    /// Insert a new entry at an ordered index.
    Added {
        /// New index.
        index: usize,
        /// New entry.
        entry: ResultEntry<K, T>,
    },
    /// Remove an entry from an ordered index.
    Removed {
        /// Previous index.
        index: usize,
        /// Previous entry, used to validate replay.
        entry: ResultEntry<K, T>,
    },
    /// Replace observable data while retaining result identity.
    Updated {
        /// Stable index at this point in the delta stream.
        index: usize,
        /// Previous entry.
        old: ResultEntry<K, T>,
        /// Replacement entry with the same key.
        new: ResultEntry<K, T>,
    },
    /// Move a stable entry in ordered results.
    Moved {
        /// Stable entry key.
        key: K,
        /// Previous index at this point in the delta stream.
        from: usize,
        /// New index.
        to: usize,
    },
    /// Keep result-entry identity while reporting a changed referent binding.
    Rebound {
        /// Stable index at this point in the delta stream.
        index: usize,
        /// Previous binding.
        old: ResultEntry<K, T>,
        /// New binding with the same result-entry key.
        new: ResultEntry<K, T>,
    },
}

/// A coherent revision-to-revision live-query delta.
///
/// Hosts emit deltas in revision order. Consumers inspect [`Self::changes`] or
/// replay the complete transition with [`QuerySnapshot::apply`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryDelta<S, K, T> {
    live_query: LiveQueryId<S>,
    from_revision: Revision<S>,
    to_revision: Revision<S>,
    identity: ResultIdentity,
    changes: Box<[QueryChange<K, T>]>,
}

impl<S, K, T> QueryDelta<S, K, T> {
    /// Creates a delta from ordered structural changes on behalf of a host.
    #[must_use]
    pub fn new(
        live_query: LiveQueryId<S>,
        from_revision: Revision<S>,
        to_revision: Revision<S>,
        identity: ResultIdentity,
        changes: impl IntoIterator<Item = QueryChange<K, T>>,
    ) -> Self {
        Self {
            live_query,
            from_revision,
            to_revision,
            identity,
            changes: changes.into_iter().collect::<Vec<_>>().into_boxed_slice(),
        }
    }

    /// Returns the live query whose transition this delta describes.
    #[must_use]
    pub const fn live_query(&self) -> LiveQueryId<S> {
        self.live_query
    }

    /// Returns the previous revision.
    #[must_use]
    pub const fn from_revision(&self) -> Revision<S> {
        self.from_revision
    }

    /// Returns the new revision.
    #[must_use]
    pub const fn to_revision(&self) -> Revision<S> {
        self.to_revision
    }

    /// Returns live-entry identity semantics.
    #[must_use]
    pub const fn identity(&self) -> ResultIdentity {
        self.identity
    }

    /// Returns replay-ordered changes.
    #[must_use]
    pub fn changes(&self) -> &[QueryChange<K, T>] {
        &self.changes
    }
}

impl<S, K, T> QueryDelta<S, K, T>
where
    K: Clone + Eq,
    T: Clone + Eq,
{
    /// Computes a deterministic delta between complete snapshots.
    pub fn between(
        before: &QuerySnapshot<S, K, T>,
        after: &QuerySnapshot<S, K, T>,
    ) -> Result<Self, DeltaError<S>> {
        if before.live_query != after.live_query {
            return Err(DeltaError::LiveQueryMismatch);
        }
        if before.revision.space() != after.revision.space() {
            return Err(DeltaError::SpaceMismatch);
        }
        if before.identity != after.identity {
            return Err(DeltaError::IdentityMismatch);
        }
        ensure_unique(&before.entries)?;
        ensure_unique(&after.entries)?;

        let mut working = before.entries.clone();
        let mut changes = Vec::new();

        for index in (0..working.len()).rev() {
            if !after
                .entries
                .iter()
                .any(|entry| entry.key == working[index].key)
            {
                let entry = working.remove(index);
                changes.push(QueryChange::Removed { index, entry });
            }
        }

        for (index, target) in after.entries.iter().enumerate() {
            if working
                .get(index)
                .is_some_and(|entry| entry.key == target.key)
            {
                if working[index] != *target {
                    let old = working[index].clone();
                    working[index] = target.clone();
                    changes.push(QueryChange::Updated {
                        index,
                        old,
                        new: target.clone(),
                    });
                }
                continue;
            }

            if let Some(from) = working.iter().position(|entry| entry.key == target.key) {
                let entry = working.remove(from);
                working.insert(index, entry);
                changes.push(QueryChange::Moved {
                    key: target.key.clone(),
                    from,
                    to: index,
                });
                if working[index] != *target {
                    let old = working[index].clone();
                    working[index] = target.clone();
                    changes.push(QueryChange::Updated {
                        index,
                        old,
                        new: target.clone(),
                    });
                }
            } else {
                working.insert(index, target.clone());
                changes.push(QueryChange::Added {
                    index,
                    entry: target.clone(),
                });
            }
        }

        debug_assert!(
            working == after.entries,
            "generated structural changes must reproduce the target entries"
        );
        Ok(Self::new(
            before.live_query,
            before.revision,
            after.revision,
            before.identity,
            changes,
        ))
    }
}

fn ensure_unique<S, K: Eq, T>(entries: &[ResultEntry<K, T>]) -> Result<(), DeltaError<S>> {
    for (index, entry) in entries.iter().enumerate() {
        if entries[..index]
            .iter()
            .any(|previous| previous.key == entry.key)
        {
            return Err(DeltaError::DuplicateKey);
        }
    }
    Ok(())
}

/// Failure to construct or atomically replay a live delta.
///
/// Returned by [`QueryDelta::between`] and [`QuerySnapshot::apply`]. On replay
/// failure, the destination snapshot remains unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeltaError<S> {
    /// Snapshot and delta belong to different live queries.
    LiveQueryMismatch,
    /// The delta attempts to cross runtime space instances.
    SpaceMismatch,
    /// The delta starts at a different revision from the snapshot.
    WrongRevision {
        /// Snapshot revision.
        expected: Revision<S>,
        /// Delta's declared previous revision.
        actual: Revision<S>,
    },
    /// Snapshot and delta use different live-entry identities.
    IdentityMismatch,
    /// A snapshot contains duplicate stable keys.
    DuplicateKey,
    /// A structural change named an unavailable index.
    IndexOutOfBounds {
        /// Invalid index.
        index: usize,
    },
    /// Replay evidence did not match the current entry.
    EntryMismatch,
    /// A move's stable key did not match its source index.
    KeyMismatch,
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::{DeltaError, LiveQueryId, QueryDelta, QuerySnapshot, ResultEntry, ResultIdentity};
    use crate::{Revision, SpaceId};

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum TestSpace {}

    #[test]
    fn replay_rejects_another_space_or_live_query_atomically() {
        let space = SpaceId::<TestSpace>::new(1);
        let second_space = SpaceId::<TestSpace>::new(2);
        let stream = LiveQueryId::<TestSpace>::new(10);
        let mut snapshot = QuerySnapshot::new(
            stream,
            Revision::new(space, 4),
            ResultIdentity::Entry,
            [ResultEntry::new(1_u8, "one")],
        );
        let original = snapshot.clone();

        let other_stream = QueryDelta::new(
            LiveQueryId::new(11),
            Revision::new(space, 4),
            Revision::new(space, 5),
            ResultIdentity::Entry,
            [],
        );
        assert_eq!(
            snapshot.apply(&other_stream),
            Err(DeltaError::LiveQueryMismatch)
        );
        assert_eq!(snapshot, original);

        let other_space = QueryDelta::new(
            stream,
            Revision::new(second_space, 4),
            Revision::new(second_space, 5),
            ResultIdentity::Entry,
            [],
        );
        assert!(matches!(
            snapshot.apply(&other_space),
            Err(DeltaError::WrongRevision { .. })
        ));
        assert_eq!(snapshot, original);

        let crossing_space = QueryDelta::new(
            stream,
            Revision::new(space, 4),
            Revision::new(second_space, 5),
            ResultIdentity::Entry,
            [],
        );
        assert_eq!(
            snapshot.apply(&crossing_space),
            Err(DeltaError::SpaceMismatch)
        );
        assert_eq!(snapshot, original);
    }

    #[test]
    fn delta_replay_agrees_with_full_recomputation() {
        let space = SpaceId::<TestSpace>::new(1);
        let stream = LiveQueryId::new(1);
        let before = QuerySnapshot::new(
            stream,
            Revision::new(space, 4),
            ResultIdentity::Occurrence,
            [
                ResultEntry::new(1_u8, "north"),
                ResultEntry::new(2_u8, "south"),
                ResultEntry::new(3_u8, "altar"),
            ],
        );
        let after = QuerySnapshot::new(
            stream,
            Revision::new(space, 5),
            ResultIdentity::Occurrence,
            [
                ResultEntry::new(2_u8, "south-updated"),
                ResultEntry::new(4_u8, "choir"),
                ResultEntry::new(1_u8, "north"),
            ],
        );
        let delta = QueryDelta::between(&before, &after).expect("snapshots have unique keys");
        let mut replayed = before;
        replayed.apply(&delta).expect("generated delta must replay");
        assert_eq!(replayed, after);
    }

    #[test]
    fn failed_replay_has_no_partial_effect() {
        let space = SpaceId::<TestSpace>::new(1);
        let stream = LiveQueryId::new(1);
        let mut snapshot = QuerySnapshot::new(
            stream,
            Revision::new(space, 1),
            ResultIdentity::Entry,
            [ResultEntry::new(1_u8, "one")],
        );
        let original = snapshot.clone();
        let delta = QueryDelta::new(
            stream,
            Revision::new(space, 1),
            Revision::new(space, 2),
            ResultIdentity::Entry,
            vec![
                super::QueryChange::Added {
                    index: 1,
                    entry: ResultEntry::new(2_u8, "two"),
                },
                super::QueryChange::Removed {
                    index: 8,
                    entry: ResultEntry::new(9_u8, "missing"),
                },
            ],
        );

        assert_eq!(
            snapshot.apply(&delta),
            Err(DeltaError::IndexOutOfBounds { index: 8 })
        );
        assert_eq!(snapshot, original);
    }
}
