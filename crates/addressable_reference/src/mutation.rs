// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed load operation and atomic guarded transaction execution.

use std::vec::Vec;

use addressable::{Endpoint, Guard, Revision, Transaction, TransactionMode};

use crate::{
    Basilica, BasilicaLocation, BasilicaSpace, EditCapability, FeatureId, Load, ReadError,
};

/// Typed operation that authors one effective load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetLoad {
    endpoint: Endpoint<BasilicaLocation, Load>,
    value: i64,
    guard: Guard<BasilicaSpace, FeatureId, i64, EditCapability>,
}

impl SetLoad {
    /// Creates a guarded load operation.
    #[must_use]
    pub const fn new(
        endpoint: Endpoint<BasilicaLocation, Load>,
        value: i64,
        guard: Guard<BasilicaSpace, FeatureId, i64, EditCapability>,
    ) -> Self {
        Self {
            endpoint,
            value,
            guard,
        }
    }

    /// Returns the typed endpoint.
    #[must_use]
    pub const fn endpoint(&self) -> &Endpoint<BasilicaLocation, Load> {
        &self.endpoint
    }

    /// Returns the proposed effective load.
    #[must_use]
    pub const fn value(&self) -> i64 {
        self.value
    }

    /// Returns all mutation preconditions.
    #[must_use]
    pub const fn guard(&self) -> &Guard<BasilicaSpace, FeatureId, i64, EditCapability> {
        &self.guard
    }
}

/// One effective value change reported by a transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadChange {
    referent: FeatureId,
    previous: i64,
    current: i64,
}

impl LoadChange {
    /// Returns the changed semantic referent.
    #[must_use]
    pub const fn referent(self) -> FeatureId {
        self.referent
    }

    /// Returns the previous effective load.
    #[must_use]
    pub const fn previous(self) -> i64 {
        self.previous
    }

    /// Returns the new effective load.
    #[must_use]
    pub const fn current(self) -> i64 {
        self.current
    }
}

/// Undo information for one applied authored opinion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UndoLoad {
    referent: FeatureId,
    previous_authored: Option<i64>,
    expected_authored: i64,
}

impl UndoLoad {
    /// Returns the referent whose authored opinion can be restored.
    #[must_use]
    pub const fn referent(self) -> FeatureId {
        self.referent
    }

    /// Returns the authored state that existed before the transaction.
    #[must_use]
    pub const fn previous_authored(self) -> Option<i64> {
        self.previous_authored
    }

    /// Returns the authored value that an undo operation must still observe.
    #[must_use]
    pub const fn expected_authored(self) -> i64 {
        self.expected_authored
    }
}

/// Successful dry-run or applied transaction report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionReport {
    mode: TransactionMode,
    revision_before: Revision<BasilicaSpace>,
    revision_after: Revision<BasilicaSpace>,
    changes: Vec<LoadChange>,
    undo: Vec<UndoLoad>,
}

impl TransactionReport {
    /// Returns whether state was previewed or applied.
    #[must_use]
    pub const fn mode(&self) -> TransactionMode {
        self.mode
    }

    /// Returns the revision validated by the transaction.
    #[must_use]
    pub const fn revision_before(&self) -> Revision<BasilicaSpace> {
        self.revision_before
    }

    /// Returns the resulting revision. A dry run retains the previous revision.
    #[must_use]
    pub const fn revision_after(&self) -> Revision<BasilicaSpace> {
        self.revision_after
    }

    /// Returns effective value changes in operation order.
    #[must_use]
    pub fn changes(&self) -> &[LoadChange] {
        &self.changes
    }

    /// Returns sufficient authored-state information for a separately guarded undo.
    #[must_use]
    pub fn undo(&self) -> &[UndoLoad] {
        &self.undo
    }
}

/// Atomic transaction conflict. No operation is observable when this is returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransactionConflict {
    /// The bulk selection snapshot is no longer current.
    SelectionRevision {
        /// Transaction selection revision.
        expected: Revision<BasilicaSpace>,
        /// Current space revision.
        actual: Revision<BasilicaSpace>,
    },
    /// An endpoint could not be used at the current revision.
    Endpoint {
        /// Operation index.
        operation: usize,
        /// Read-side validation failure.
        error: ReadError,
    },
    /// Endpoint and guard name different referents.
    ReferentMismatch {
        /// Operation index.
        operation: usize,
    },
    /// The operation's guard was established at another revision.
    GuardRevision {
        /// Operation index.
        operation: usize,
        /// Guard revision.
        expected: Revision<BasilicaSpace>,
        /// Current revision.
        actual: Revision<BasilicaSpace>,
    },
    /// Effective value changed since the guard was established.
    ValueMismatch {
        /// Operation index.
        operation: usize,
        /// Guarded value.
        expected: i64,
        /// Current effective value.
        actual: i64,
    },
    /// The referent does not permit this edit capability.
    CapabilityUnavailable {
        /// Operation index.
        operation: usize,
    },
    /// Several operations target the same referent in one atomic batch.
    DuplicateTarget {
        /// Later conflicting operation index.
        operation: usize,
    },
    /// Guarded semantic identity no longer exists.
    MissingReferent {
        /// Operation index.
        operation: usize,
    },
}

#[derive(Clone, Copy, Debug)]
struct PreparedLoad {
    referent: FeatureId,
    previous_effective: i64,
    previous_authored: Option<i64>,
    current: i64,
}

impl Basilica {
    /// Atomically validates and previews or applies typed load operations.
    pub fn transact(
        &mut self,
        transaction: Transaction<BasilicaSpace, SetLoad>,
    ) -> Result<TransactionReport, TransactionConflict> {
        if transaction.selection_revision() != self.revision {
            return Err(TransactionConflict::SelectionRevision {
                expected: transaction.selection_revision(),
                actual: self.revision,
            });
        }

        let mut prepared = Vec::new();
        for (operation, edit) in transaction.operations().iter().enumerate() {
            self.validate_location(edit.endpoint.owner())
                .map_err(|error| TransactionConflict::Endpoint { operation, error })?;
            if edit.endpoint.owner().referent() != edit.guard.expected_referent() {
                return Err(TransactionConflict::ReferentMismatch { operation });
            }
            if edit.guard.expected_revision() != self.revision {
                return Err(TransactionConflict::GuardRevision {
                    operation,
                    expected: edit.guard.expected_revision(),
                    actual: self.revision,
                });
            }
            if *edit.guard.required_capability() != EditCapability::SetLoad {
                return Err(TransactionConflict::CapabilityUnavailable { operation });
            }
            let feature = self
                .feature(*edit.guard.expected_referent())
                .ok_or(TransactionConflict::MissingReferent { operation })?;
            if !feature.editable {
                return Err(TransactionConflict::CapabilityUnavailable { operation });
            }
            let current = feature.effective_load();
            if current != *edit.guard.expected_value() {
                return Err(TransactionConflict::ValueMismatch {
                    operation,
                    expected: *edit.guard.expected_value(),
                    actual: current,
                });
            }
            if prepared
                .iter()
                .any(|prior: &PreparedLoad| prior.referent == feature.id)
            {
                return Err(TransactionConflict::DuplicateTarget { operation });
            }
            prepared.push(PreparedLoad {
                referent: feature.id,
                previous_effective: current,
                previous_authored: feature.authored_load,
                current: edit.value,
            });
        }

        let changes = prepared
            .iter()
            .filter(|change| change.previous_effective != change.current)
            .map(|change| LoadChange {
                referent: change.referent,
                previous: change.previous_effective,
                current: change.current,
            })
            .collect::<Vec<_>>();
        let undo = prepared
            .iter()
            .filter(|change| change.previous_authored != Some(change.current))
            .map(|change| UndoLoad {
                referent: change.referent,
                previous_authored: change.previous_authored,
                expected_authored: change.current,
            })
            .collect::<Vec<_>>();

        let revision_before = self.revision;
        if transaction.mode() == TransactionMode::Apply && !undo.is_empty() {
            let revision_after = self.revision.next();
            for change in &prepared {
                let feature = self
                    .feature_mut(change.referent)
                    .expect("validated referent must remain present during atomic apply");
                feature.authored_load = Some(change.current);
                feature.authored_revision = revision_after;
            }
            self.revision = revision_after;
        }

        Ok(TransactionReport {
            mode: transaction.mode(),
            revision_before,
            revision_after: self.revision,
            changes,
            undo,
        })
    }
}

#[cfg(test)]
mod tests {
    use addressable::{
        AbsoluteAddress, Endpoint, Guard, Locator, Resolution, SpaceId, Transaction,
        TransactionMode,
    };

    use crate::{
        Basilica, BasilicaSpace, BasilicaView, EditCapability, FeatureId, Load, SetLoad,
        TransactionConflict,
    };

    fn endpoint(space: &Basilica, address: &str) -> Endpoint<crate::BasilicaLocation, Load> {
        let locator = Locator::exact(
            space.id(),
            BasilicaView::Assembly,
            AbsoluteAddress::parse(address).expect("valid test address"),
        );
        let Resolution::Resolved(location) = space.resolve(&locator) else {
            panic!("test endpoint should resolve");
        };
        Endpoint::new(location, Load)
    }

    #[test]
    fn dry_run_reports_without_mutating_then_apply_advances_once() {
        let mut space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        let endpoint = endpoint(&space, "/basilica/nave/north_arch");
        let edit = SetLoad::new(
            endpoint,
            80,
            Guard::new(
                FeatureId::new(3),
                space.revision(),
                120,
                EditCapability::SetLoad,
            ),
        );
        let preview = space
            .transact(Transaction::dry_run(space.revision(), [edit.clone()]))
            .expect("dry run should validate");
        assert_eq!(preview.mode(), TransactionMode::DryRun);
        assert_eq!(space.revision(), addressable::Revision::initial(space.id()));
        assert_eq!(
            space.feature(FeatureId::new(3)).unwrap().effective_load(),
            120
        );

        let applied = space
            .transact(Transaction::apply(space.revision(), [edit]))
            .expect("apply should validate");
        assert_eq!(applied.mode(), TransactionMode::Apply);
        assert_eq!(applied.changes()[0].current(), 80);
        assert_eq!(applied.undo()[0].previous_authored(), Some(120));
        assert_eq!(space.revision(), addressable::Revision::new(space.id(), 1));
    }

    #[test]
    fn failed_batch_has_no_partial_observable_effect() {
        let mut space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        let arch = endpoint(&space, "/basilica/nave/north_arch");
        let vault = endpoint(&space, "/basilica/nave/vault");
        let edits = [
            SetLoad::new(
                arch,
                80,
                Guard::new(
                    FeatureId::new(3),
                    space.revision(),
                    120,
                    EditCapability::SetLoad,
                ),
            ),
            SetLoad::new(
                vault,
                160,
                Guard::new(
                    FeatureId::new(4),
                    space.revision(),
                    999,
                    EditCapability::SetLoad,
                ),
            ),
        ];

        assert!(matches!(
            space.transact(Transaction::apply(space.revision(), edits)),
            Err(TransactionConflict::ValueMismatch { operation: 1, .. })
        ));
        assert_eq!(space.revision(), addressable::Revision::initial(space.id()));
        assert_eq!(
            space.feature(FeatureId::new(3)).unwrap().effective_load(),
            120
        );
    }

    #[test]
    fn transaction_revision_cannot_cross_space_instances() {
        let first = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        let mut second = Basilica::new(SpaceId::<BasilicaSpace>::new(2));
        let original = second.revision();

        let result = second.transact(Transaction::apply(
            first.revision(),
            core::iter::empty::<SetLoad>(),
        ));

        assert!(matches!(
            result,
            Err(TransactionConflict::SelectionRevision { .. })
        ));
        assert_eq!(second.revision(), original);
    }
}
