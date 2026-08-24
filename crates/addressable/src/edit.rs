// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Guard and transaction vocabulary for addressed mutation.

use alloc::vec::Vec;

use crate::Revision;

/// Preconditions required before applying an addressed mutation.
///
/// A caller forms a guard after selecting and reading a target. All four fields
/// must describe that same observation: the resolved semantic referent, the
/// revision that governed selection and reading, the value or fingerprint then
/// read, and any capability required by the domain operation.
///
/// A guard does not identify an endpoint or perform a mutation. A domain
/// operation pairs it with its endpoint and proposed value, a [`Transaction`]
/// carries one or more operations, and the receiving host validates every
/// precondition at submission. A guard is evidence, not a lock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guard<S, I, V, C = ()> {
    expected_referent: I,
    expected_revision: Revision<S>,
    expected_value: V,
    required_capability: C,
}

impl<S, I, V> Guard<S, I, V, ()> {
    /// Creates a guard for a domain that requires no separate capability token.
    ///
    /// This omits only the capability token; the host must still validate the
    /// referent, revision, and observed value.
    #[must_use]
    pub const fn at(
        expected_referent: I,
        expected_revision: Revision<S>,
        expected_value: V,
    ) -> Self {
        Self::new(expected_referent, expected_revision, expected_value, ())
    }
}

impl<S, I, V, C> Guard<S, I, V, C> {
    /// Creates a guarded-mutation precondition from one coherent observation.
    #[must_use]
    pub const fn new(
        expected_referent: I,
        expected_revision: Revision<S>,
        expected_value: V,
        required_capability: C,
    ) -> Self {
        Self {
            expected_referent,
            expected_revision,
            expected_value,
            required_capability,
        }
    }

    /// Returns expected semantic referent identity.
    #[must_use]
    pub const fn expected_referent(&self) -> &I {
        &self.expected_referent
    }

    /// Returns the revision at which selection and reading occurred.
    #[must_use]
    pub const fn expected_revision(&self) -> Revision<S> {
        self.expected_revision
    }

    /// Returns the value fingerprint or typed value observed by the caller.
    #[must_use]
    pub const fn expected_value(&self) -> &V {
        &self.expected_value
    }

    /// Returns the required host capability.
    #[must_use]
    pub const fn required_capability(&self) -> &C {
        &self.required_capability
    }
}

/// Whether a transaction is previewed or committed.
///
/// Selected by [`Transaction::dry_run`] or [`Transaction::apply`] and returned
/// in domain transaction reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionMode {
    /// Validate and report impact without changing state.
    DryRun,
    /// Validate atomically and then commit.
    Apply,
}

/// Behavior when one operation in a transaction conflicts.
///
/// Read this from [`Transaction::failure_policy`]. Addressable currently
/// exposes only all-or-nothing execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailurePolicy {
    /// No operation is observable unless every operation validates.
    Atomic,
}

/// A snapshot-scoped collection of typed operations.
///
/// Build operations from guarded endpoints, then choose [`Self::dry_run`] or
/// [`Self::apply`] using the revision at which the target set was selected.
/// Operations should retain their own target and [`Guard`]; `O` remains typed
/// so each host can define the mutations it supports.
///
/// Addressable packages the request but does not execute it. The receiving
/// host validates the transaction, enforces [`FailurePolicy::Atomic`], and
/// defines its typed success report and conflict errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction<S, O> {
    selection_revision: Revision<S>,
    mode: TransactionMode,
    failure_policy: FailurePolicy,
    operations: Vec<O>,
}

impl<S, O> Transaction<S, O> {
    /// Creates an atomic dry run against one selection revision.
    #[must_use]
    pub fn dry_run(
        selection_revision: Revision<S>,
        operations: impl IntoIterator<Item = O>,
    ) -> Self {
        Self::new(selection_revision, TransactionMode::DryRun, operations)
    }

    /// Creates an atomic applying transaction against one selection revision.
    #[must_use]
    pub fn apply(selection_revision: Revision<S>, operations: impl IntoIterator<Item = O>) -> Self {
        Self::new(selection_revision, TransactionMode::Apply, operations)
    }

    fn new(
        selection_revision: Revision<S>,
        mode: TransactionMode,
        operations: impl IntoIterator<Item = O>,
    ) -> Self {
        Self {
            selection_revision,
            mode,
            failure_policy: FailurePolicy::Atomic,
            operations: operations.into_iter().collect(),
        }
    }

    /// Returns the revision against which targets were selected.
    #[must_use]
    pub const fn selection_revision(&self) -> Revision<S> {
        self.selection_revision
    }

    /// Returns preview or apply mode.
    #[must_use]
    pub const fn mode(&self) -> TransactionMode {
        self.mode
    }

    /// Returns all-or-nothing failure policy.
    #[must_use]
    pub const fn failure_policy(&self) -> FailurePolicy {
        self.failure_policy
    }

    /// Returns typed operations in caller order.
    #[must_use]
    pub fn operations(&self) -> &[O] {
        &self.operations
    }

    /// Consumes the transaction and returns its operations.
    #[must_use]
    pub fn into_operations(self) -> Vec<O> {
        self.operations
    }
}
