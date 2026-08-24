// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Rich resolution outcomes.

use alloc::{boxed::Box, string::String};

use crate::Revision;

/// A resolution outcome that preserves absence, ambiguity, staleness, movement,
/// and rebinding.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Resolution<S, T, I, A> {
    /// The locator resolved without violating its policy.
    Resolved(T),
    /// Nothing currently occupies the requested location.
    Absent,
    /// The locator admitted several valid occurrences.
    Ambiguous(Box<[T]>),
    /// The required view or operation is unsupported by this host.
    UnsupportedLocator,
    /// Resolution was attempted against a newer or otherwise incompatible revision.
    StaleRevision {
        /// Revision required by the caller.
        expected: Revision<S>,
        /// Current host revision.
        actual: Revision<S>,
    },
    /// The locator now denotes a different semantic referent.
    Rebound {
        /// Referent identity pinned by the caller.
        expected: I,
        /// Referent identity currently at the locator.
        actual: I,
        /// Current resolved occurrence, returned as evidence rather than success.
        resolved: T,
    },
    /// The expected referent was found at a different exact address.
    Moved {
        /// Address carried by the locator.
        from: A,
        /// Current address of the expected referent.
        to: A,
        /// Occurrence at the new address.
        resolved: T,
    },
    /// Resolution returned usable partial results with an explicit reason.
    Partial {
        /// Resolved portion.
        resolved: Box<[T]>,
        /// Why resolution stopped.
        reason: PartialReason,
    },
    /// The host does not expose a required capability in this view.
    CapabilityUnavailable(String),
    /// A declared traversal budget was exhausted.
    BudgetExceeded(BudgetExceeded),
}

impl<S, T, I, A> Resolution<S, T, I, A> {
    /// Returns the ordinary resolved value, if and only if no exceptional
    /// resolution state occurred.
    #[must_use]
    pub fn resolved(self) -> Option<T> {
        match self {
            Self::Resolved(value) => Some(value),
            _ => None,
        }
    }
}

/// Why otherwise valid resolution is partial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PartialReason {
    /// A required subspace was unavailable.
    SubspaceUnavailable,
    /// Some but not all candidates were authorized.
    CapabilityUnavailable,
    /// Work stopped at a declared budget.
    BudgetExceeded(BudgetExceeded),
}

/// The budget dimension that stopped work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetDimension {
    /// Maximum traversal depth.
    Depth,
    /// Maximum visited nodes.
    Nodes,
    /// Maximum returned results.
    Results,
    /// Maximum host-defined work units.
    Work,
}

/// Evidence that one declared traversal budget was exceeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetExceeded {
    dimension: BudgetDimension,
    limit: u32,
    observed: u32,
}

impl BudgetExceeded {
    /// Records a budget limit and the first observed value beyond it.
    #[must_use]
    pub const fn new(dimension: BudgetDimension, limit: u32, observed: u32) -> Self {
        Self {
            dimension,
            limit,
            observed,
        }
    }

    /// Returns the exhausted budget dimension.
    #[must_use]
    pub const fn dimension(self) -> BudgetDimension {
        self.dimension
    }

    /// Returns the declared limit.
    #[must_use]
    pub const fn limit(self) -> u32 {
        self.limit
    }

    /// Returns the observed value that exceeded the limit.
    #[must_use]
    pub const fn observed(self) -> u32 {
        self.observed
    }
}
