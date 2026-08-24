// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A second small result space and evidence-bearing basilica correspondence.

use std::vec::Vec;

use addressable::{
    AbsoluteAddress, Correspondence, CorrespondenceTarget, Location, Locator, Resolution, Revision,
    SpaceId,
};

use crate::{Basilica, BasilicaSpace, FeatureId, OccurrenceId};

/// Type marker for the catalog result space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CatalogSpace {}

/// The catalog's named result view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CatalogView {
    /// Ranked retrieval result occurrences.
    Results,
}

/// Durable semantic identity of one catalog result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CatalogEntryId(u64);

impl CatalogEntryId {
    /// Creates a catalog result identity.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the catalog-assigned value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Contextual occurrence identity of one ranked catalog result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CatalogOccurrenceId(u64);

impl CatalogOccurrenceId {
    /// Creates a result occurrence identity.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the catalog-assigned value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Resolved catalog result occurrence.
pub type CatalogLocation = Location<CatalogSpace, CatalogView, CatalogEntryId, CatalogOccurrenceId>;

/// View-qualified catalog locator.
pub type CatalogLocator = Locator<CatalogSpace, CatalogView>;

/// Rich catalog resolution outcome.
pub type CatalogResolution =
    Resolution<CatalogSpace, CatalogLocation, CatalogEntryId, AbsoluteAddress<CatalogSpace>>;

/// Provenance for one basilica-to-catalog mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CatalogEvidence {
    source_space: SpaceId<BasilicaSpace>,
    source_occurrence: OccurrenceId,
    reason: &'static str,
}

impl CatalogEvidence {
    /// Returns the basilica instance observed by cataloging.
    #[must_use]
    pub const fn source_space(self) -> SpaceId<BasilicaSpace> {
        self.source_space
    }

    /// Returns the assembly occurrence that produced this result occurrence.
    #[must_use]
    pub const fn source_occurrence(self) -> OccurrenceId {
        self.source_occurrence
    }

    /// Returns stable ranking evidence.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        self.reason
    }
}

#[derive(Clone, Debug)]
struct CatalogEntry {
    id: CatalogEntryId,
    occurrence: CatalogOccurrenceId,
    subject: FeatureId,
    source_occurrence: OccurrenceId,
    address: AbsoluteAddress<CatalogSpace>,
}

/// Deterministic second address space containing ranked basilica results.
#[derive(Clone, Debug)]
pub struct Catalog {
    id: SpaceId<CatalogSpace>,
    revision: Revision<CatalogSpace>,
    entries: Vec<CatalogEntry>,
}

impl Catalog {
    /// Constructs deterministic catalog results for the reference basilica.
    #[must_use]
    pub fn new(id: SpaceId<CatalogSpace>) -> Self {
        Self {
            id,
            revision: Revision::initial(id),
            entries: vec![
                entry(1, 1, 3, 3, "/results/north_arch"),
                entry(2, 2, 3, 4, "/results/south_arch"),
                entry(3, 3, 4, 5, "/results/vault"),
            ],
        }
    }

    /// Returns the runtime catalog identity.
    #[must_use]
    pub const fn id(&self) -> SpaceId<CatalogSpace> {
        self.id
    }

    /// Returns the current catalog revision.
    #[must_use]
    pub const fn revision(&self) -> Revision<CatalogSpace> {
        self.revision
    }

    /// Resolves one exact or relative result locator.
    #[must_use]
    pub fn resolve(&self, locator: &CatalogLocator) -> CatalogResolution {
        if locator.space() != self.id || *locator.view() != CatalogView::Results {
            return Resolution::UnsupportedLocator;
        }
        let Ok(address) = locator.to_absolute() else {
            return Resolution::UnsupportedLocator;
        };
        self.entries
            .iter()
            .find(|entry| entry.address == address)
            .map_or(Resolution::Absent, |entry| {
                Resolution::Resolved(self.location(entry))
            })
    }

    fn location(&self, entry: &CatalogEntry) -> CatalogLocation {
        CatalogLocation::new(
            CatalogView::Results,
            self.revision,
            entry.id,
            entry.occurrence,
            entry.address.clone(),
        )
    }
}

impl Basilica {
    /// Maps one semantic basilica feature into zero or more catalog result occurrences.
    #[must_use]
    pub fn correspond_to_catalog(
        &self,
        referent: FeatureId,
        catalog: &Catalog,
    ) -> Correspondence<FeatureId, CatalogLocation, CatalogEvidence> {
        Correspondence::new(
            referent,
            catalog
                .entries
                .iter()
                .filter(|entry| entry.subject == referent)
                .map(|entry| {
                    CorrespondenceTarget::new(
                        catalog.location(entry),
                        CatalogEvidence {
                            source_space: self.id(),
                            source_occurrence: entry.source_occurrence,
                            reason: "catalog/ranked-basilica-occurrence",
                        },
                    )
                }),
        )
    }
}

fn entry(
    id: u64,
    occurrence: u64,
    subject: u64,
    source_occurrence: u64,
    address: &str,
) -> CatalogEntry {
    CatalogEntry {
        id: CatalogEntryId::new(id),
        occurrence: CatalogOccurrenceId::new(occurrence),
        subject: FeatureId::new(subject),
        source_occurrence: OccurrenceId::new(source_occurrence),
        address: AbsoluteAddress::parse(address).expect("static catalog address must be valid"),
    }
}

#[cfg(test)]
mod tests {
    use addressable::{AbsoluteAddress, Locator, Resolution, SpaceId};

    use crate::{Basilica, BasilicaSpace, Catalog, CatalogSpace, CatalogView, FeatureId};

    #[test]
    fn correspondence_preserves_one_to_many_occurrences_and_evidence() {
        let basilica = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        let catalog = Catalog::new(SpaceId::<CatalogSpace>::new(2));
        let correspondence = basilica.correspond_to_catalog(FeatureId::new(3), &catalog);
        assert!(correspondence.is_ambiguous());
        assert_eq!(correspondence.targets().len(), 2);
        assert_ne!(
            correspondence.targets()[0].target().occurrence(),
            correspondence.targets()[1].target().occurrence()
        );
        assert_eq!(
            correspondence.targets()[0].provenance().source_space(),
            basilica.id()
        );
    }

    #[test]
    fn catalog_is_an_independently_resolvable_space() {
        let catalog = Catalog::new(SpaceId::<CatalogSpace>::new(2));
        let locator = Locator::exact(
            catalog.id(),
            CatalogView::Results,
            AbsoluteAddress::parse("/results/north_arch").expect("valid result address"),
        );
        assert!(matches!(catalog.resolve(&locator), Resolution::Resolved(_)));
    }
}
