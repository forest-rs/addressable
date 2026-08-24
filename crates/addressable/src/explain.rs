// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Generic typed value opinions and winning explanations.

use alloc::{boxed::Box, vec::Vec};

/// One typed candidate value and its domain-owned provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opinion<T, P> {
    value: T,
    provenance: P,
}

impl<T, P> Opinion<T, P> {
    /// Creates one value opinion.
    #[must_use]
    pub const fn new(value: T, provenance: P) -> Self {
        Self { value, provenance }
    }

    /// Returns the candidate value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns domain-owned provenance.
    #[must_use]
    pub const fn provenance(&self) -> &P {
        &self.provenance
    }
}

/// A winning typed value, alternatives, provenance, and domain-owned reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explained<S, T, P, R> {
    subject: S,
    opinions: Box<[Opinion<T, P>]>,
    winner: usize,
    reason: R,
}

impl<S, T, P, R> Explained<S, T, P, R> {
    /// Creates an explanation and validates its winner index.
    pub fn new(
        subject: S,
        opinions: impl IntoIterator<Item = Opinion<T, P>>,
        winner: usize,
        reason: R,
    ) -> Result<Self, ExplainError> {
        let opinions = opinions.into_iter().collect::<Vec<_>>().into_boxed_slice();
        if opinions.is_empty() {
            return Err(ExplainError::NoOpinions);
        }
        if winner >= opinions.len() {
            return Err(ExplainError::WinnerOutOfBounds);
        }
        Ok(Self {
            subject,
            opinions,
            winner,
            reason,
        })
    }

    /// Returns the explained semantic subject.
    #[must_use]
    pub const fn subject(&self) -> &S {
        &self.subject
    }

    /// Returns the effective winning value.
    #[must_use]
    pub fn value(&self) -> &T {
        self.opinions[self.winner].value()
    }

    /// Returns every typed opinion in domain-defined strength order.
    #[must_use]
    pub fn opinions(&self) -> &[Opinion<T, P>] {
        &self.opinions
    }

    /// Returns the winning opinion index.
    #[must_use]
    pub const fn winner(&self) -> usize {
        self.winner
    }

    /// Returns the domain-owned explanation reason.
    #[must_use]
    pub const fn reason(&self) -> &R {
        &self.reason
    }
}

/// Invalid construction of an [`Explained`] value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplainError {
    /// At least one opinion is required.
    NoOpinions,
    /// The winner index did not identify an opinion.
    WinnerOutOfBounds,
}
