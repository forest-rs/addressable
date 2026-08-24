// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Scanning reference object spaces for Addressable.
//!
//! [`Basilica`] is deliberately small, but it is a real host for the complete
//! lifecycle: structured resolution, multi-view queries, typed explanation,
//! live deltas, guarded transactions, and correspondence into [`Catalog`].
//! It uses linear scans so the semantic contracts remain visible.
//!
//! ```
//! use addressable::{CyclePolicy, Deduplication, Query, SpaceId, VisitIdentity};
//! use addressable_reference::{
//!     Basilica, BasilicaAxis, BasilicaPredicate, BasilicaSpace, FeatureKind,
//! };
//!
//! let space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
//! let query = Query::many(space.root_locator())
//!     .traverse(BasilicaAxis::Descendants)
//!     .filter(BasilicaPredicate::Kind(FeatureKind::Arch))
//!     .deduplicate(Deduplication::Occurrence)
//!     .cycles(CyclePolicy::SkipVisited(VisitIdentity::Occurrence));
//! let arches = space.query_many(&query)?;
//! assert_eq!(arches.items().len(), 2);
//! # Ok::<(), addressable::QueryError>(())
//! ```

mod catalog;
mod model;
mod mutation;
mod space;
mod watch;

pub use catalog::{
    Catalog, CatalogEntryId, CatalogEvidence, CatalogLocation, CatalogLocator, CatalogOccurrenceId,
    CatalogResolution, CatalogSpace, CatalogView,
};
pub use model::{
    BasilicaAxis, BasilicaLocation, BasilicaLocator, BasilicaPredicate, BasilicaQuery,
    BasilicaResolution, BasilicaSpace, BasilicaView, BasilicaViewParseError, EdgeId,
    EditCapability, FeatureId, FeatureKind, Load, LoadProvenance, LoadReason, OccurrenceId,
    SlotHandle,
};
pub use mutation::{LoadChange, SetLoad, TransactionConflict, TransactionReport, UndoLoad};
pub use space::{Basilica, Measured, ReadError};
pub use watch::{BasilicaWatch, WatchError};
