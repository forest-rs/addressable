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
//! # Vocabulary lifecycle
//!
//! Addressable separates values a caller prepares from contextual values a
//! host produces:
//!
//! 1. A caller prepares a [`Locator`], [`Pinned`] locator, or [`Query`].
//! 2. A host resolves or executes it, producing a [`Location`] or
//!    [`QueryResults`].
//! 3. A located owner and a domain facet form an [`Endpoint`]. A host can read
//!    that endpoint as an [`Explained`] value or represent a revision-scoped
//!    runtime accelerator with [`ResolvedHandle`].
//! 4. A caller turns the identity, revision, and value it observed into a
//!    [`Guard`], then submits typed operations in a [`Transaction`].
//! 5. A live-query host produces [`QuerySnapshot`] and [`QueryDelta`] values;
//!    consumers replay deltas with [`QuerySnapshot::apply`].
//!
//! Constructors on contextual result types are public for host
//! implementations. Ordinary callers usually obtain those types from the
//! domain host rather than constructing them directly.
//!
//! # Structured addresses
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
