// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Partial, evidence-bearing correspondence between address spaces.

use alloc::{boxed::Box, vec::Vec};

/// One correspondence target and the evidence for that mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorrespondenceTarget<T, P> {
    target: T,
    provenance: P,
}

impl<T, P> CorrespondenceTarget<T, P> {
    /// Creates one evidence-bearing target.
    #[must_use]
    pub const fn new(target: T, provenance: P) -> Self {
        Self { target, provenance }
    }

    /// Returns the target in the destination space.
    #[must_use]
    pub const fn target(&self) -> &T {
        &self.target
    }

    /// Returns evidence for the mapping.
    #[must_use]
    pub const fn provenance(&self) -> &P {
        &self.provenance
    }
}

/// A partial, possibly one-to-many mapping from one source value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correspondence<F, T, P> {
    source: F,
    targets: Box<[CorrespondenceTarget<T, P>]>,
}

impl<F, T, P> Correspondence<F, T, P> {
    /// Creates a correspondence, including an empty partial result.
    #[must_use]
    pub fn new(source: F, targets: impl IntoIterator<Item = CorrespondenceTarget<T, P>>) -> Self {
        Self {
            source,
            targets: targets.into_iter().collect::<Vec<_>>().into_boxed_slice(),
        }
    }

    /// Returns the source value.
    #[must_use]
    pub const fn source(&self) -> &F {
        &self.source
    }

    /// Returns every destination and its evidence.
    #[must_use]
    pub fn targets(&self) -> &[CorrespondenceTarget<T, P>] {
        &self.targets
    }

    /// Returns whether the source has more than one destination.
    #[must_use]
    pub fn is_ambiguous(&self) -> bool {
        self.targets.len() > 1
    }

    /// Composes this mapping with a second mapping while retaining evidence
    /// from both legs and preserving multiplicity.
    ///
    /// The callback cannot replace the source it receives with an unrelated
    /// source value.
    ///
    /// ```compile_fail
    /// use addressable::{Correspondence, CorrespondenceTarget};
    ///
    /// let first = Correspondence::new(
    ///     "arch",
    ///     [CorrespondenceTarget::new("north", "assembly")],
    /// );
    /// let _: Result<_, ()> = first.compose(|_| {
    ///     Ok(Correspondence::new(
    ///         "unrelated",
    ///         [CorrespondenceTarget::new(1_u8, "catalog")],
    ///     ))
    /// });
    /// ```
    pub fn compose<U, Q, E, C>(
        &self,
        mut next: impl FnMut(&T) -> Result<C, E>,
    ) -> Result<Correspondence<F, U, ComposedEvidence<P, Q>>, E>
    where
        F: Clone,
        P: Clone,
        C: IntoIterator<Item = CorrespondenceTarget<U, Q>>,
    {
        let mut composed = Vec::new();
        for first in &self.targets {
            for target in next(&first.target)? {
                composed.push(CorrespondenceTarget::new(
                    target.target,
                    ComposedEvidence::new(first.provenance.clone(), target.provenance),
                ));
            }
        }
        Ok(Correspondence::new(self.source.clone(), composed))
    }
}

/// Evidence retained from both legs of correspondence composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedEvidence<A, B> {
    first: A,
    second: B,
}

impl<A, B> ComposedEvidence<A, B> {
    /// Pairs evidence from two mapping legs.
    #[must_use]
    pub const fn new(first: A, second: B) -> Self {
        Self { first, second }
    }

    /// Returns evidence from the first leg.
    #[must_use]
    pub const fn first(&self) -> &A {
        &self.first
    }

    /// Returns evidence from the second leg.
    #[must_use]
    pub const fn second(&self) -> &B {
        &self.second
    }
}

#[cfg(test)]
mod tests {
    use super::{Correspondence, CorrespondenceTarget};

    #[test]
    fn composition_preserves_ambiguity_and_both_provenance_legs() {
        let first = Correspondence::new(
            "arch",
            [
                CorrespondenceTarget::new("north", "assembly:north"),
                CorrespondenceTarget::new("south", "assembly:south"),
            ],
        );
        let composed = first
            .compose::<_, _, (), _>(|occurrence| {
                Ok([CorrespondenceTarget::new(
                    if *occurrence == "north" { 1 } else { 2 },
                    "catalog:result",
                )])
            })
            .expect("composition succeeds");

        assert!(composed.is_ambiguous());
        assert_eq!(composed.targets().len(), 2);
        assert_eq!(
            composed.targets()[0].provenance().first(),
            &"assembly:north"
        );
        assert_eq!(
            composed.targets()[0].provenance().second(),
            &"catalog:result"
        );
    }
}
