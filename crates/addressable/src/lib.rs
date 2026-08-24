// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed vocabulary for addressable structured object spaces.
//!
//! `addressable` keeps durable addresses, contextual locations, semantic
//! identities, runtime handles, query policy, live deltas, guarded edits, and
//! correspondence distinct. It deliberately owns no storage engine or domain
//! value enum.
//!
//! The crate is always `no_std` and uses `alloc` for owned structured values.
//!
//! ```
//! use addressable::{AbsoluteAddress, RelativeAddress};
//!
//! enum BasilicaSpace {}
//!
//! let nave = AbsoluteAddress::<BasilicaSpace>::parse("/basilica/nave")?;
//! let arch = RelativeAddress::<BasilicaSpace>::parse("../transept/arch")?;
//! let target = nave.join(&arch)?;
//! assert_eq!(target.to_string(), "/basilica/transept/arch");
//! # Ok::<(), addressable::AddressError>(())
//! ```

#![no_std]

extern crate alloc;

mod address;
mod correspondence;
mod edit;
mod explain;
mod identity;
mod live;
mod query;
mod resolution;

pub use address::{
    AbsoluteAddress, AddressError, Locator, LocatorKind, LocatorParseError, Name, NameError,
    Pinned, PinnedParseError, RelativeAddress,
};
pub use correspondence::{ComposedEvidence, Correspondence, CorrespondenceTarget};
pub use edit::{FailurePolicy, Guard, Transaction, TransactionMode};
pub use explain::{ExplainError, Explained, Opinion};
pub use identity::{Endpoint, Located, Location, ResolvedHandle, Revision, SpaceId};
pub use live::{
    DeltaError, LiveQueryId, QueryChange, QueryDelta, QuerySnapshot, ResultEntry, ResultIdentity,
};
pub use query::{
    Cardinality, CardinalityKind, CyclePolicy, Deduplication, Many, One, Optional, Query,
    QueryError, QueryResults, QuerySemantics, QueryStats, QueryStep, ResultOrdering,
    TraversalBudget, VisitIdentity,
};
pub use resolution::{BudgetDimension, BudgetExceeded, PartialReason, Resolution};
