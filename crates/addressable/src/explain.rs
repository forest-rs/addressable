// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Generic typed value opinions and winning explanations.

use alloc::{boxed::Box, vec::Vec};

/// One typed candidate value and its domain-owned provenance.
///
/// Hosts construct opinions inside an [`Explained`] result. Callers normally
/// inspect the complete ordered slice through [`Explained::opinions`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opinion<T, P> {
    value: T,
    provenance: P,
}

impl<T, P> Opinion<T, P> {
    /// Creates one value opinion on behalf of an explaining host.
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
///
/// A host produces this while reading or explaining a typed subject according
/// to domain policy. Callers use [`Self::value`] for the effective value and
/// inspect [`Self::opinions`] plus [`Self::reason`] when they need the evidence
/// behind it. The subject lets callers associate the explanation with durable
/// semantic identity instead of relying only on the route used to read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explained<S, T, P, R> {
    subject: S,
    opinions: Box<[Opinion<T, P>]>,
    winner: usize,
    reason: R,
}

impl<S, T, P, R> Explained<S, T, P, R> {
    /// Creates an explanation on behalf of a host and validates its winner index.
    ///
    /// `opinions` must be in the host's documented strength order. `winner`
    /// identifies the effective opinion; Addressable does not choose it or
    /// interpret the domain-owned reason and provenance values.
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

#[cfg(test)]
mod tests {
    use super::{ExplainError, Explained, Opinion};

    #[test]
    fn explanation_requires_an_opinion() {
        let result = Explained::<&str, i64, &str, &str>::new("arch", [], 0, "policy");

        assert_eq!(result, Err(ExplainError::NoOpinions));
    }

    #[test]
    fn explanation_requires_a_valid_winner() {
        let result = Explained::new("arch", [Opinion::new(120_i64, "authored")], 1, "policy");

        assert_eq!(result, Err(ExplainError::WinnerOutOfBounds));
    }

    #[test]
    fn explanation_returns_the_selected_value_and_evidence() {
        let explanation = Explained::new(
            "arch",
            [
                Opinion::new(40_i64, "default"),
                Opinion::new(120_i64, "authored"),
            ],
            1,
            "authored overrides default",
        )
        .expect("winner identifies an opinion");

        assert_eq!(explanation.subject(), &"arch");
        assert_eq!(explanation.value(), &120);
        assert_eq!(explanation.opinions()[1].provenance(), &"authored");
        assert_eq!(explanation.reason(), &"authored overrides default");
    }
}
