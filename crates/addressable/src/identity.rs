// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed space, location, endpoint, and resolved-handle identities.

use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use crate::AbsoluteAddress;

/// Runtime identity for one instance of a typed address space.
///
/// The marker `S` prevents ids from unrelated domain types from being mixed.
/// Values are assigned by the host; Addressable does not require a global id
/// generator or atomics. Hosts place the id in locators and revisions; callers
/// normally obtain it from a domain host rather than inventing it.
pub struct SpaceId<S> {
    raw: u64,
    marker: PhantomData<fn() -> S>,
}

impl<S> Copy for SpaceId<S> {}

impl<S> Clone for SpaceId<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> fmt::Debug for SpaceId<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("SpaceId").field(&self.raw).finish()
    }
}

impl<S> PartialEq for SpaceId<S> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<S> Eq for SpaceId<S> {}

impl<S> PartialOrd for SpaceId<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S> Ord for SpaceId<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<S> Hash for SpaceId<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<S> SpaceId<S> {
    /// Creates a typed space id from a host-assigned value.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self {
            raw,
            marker: PhantomData,
        }
    }

    /// Returns the host-assigned value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.raw
    }
}

/// Monotonic revision scoped to one typed address-space instance.
///
/// Keeping the [`SpaceId`] inside the value prevents equal numeric counters
/// from unrelated space instances from comparing as the same revision. A host
/// creates and advances revisions, returns them in contextual results, and
/// validates them when callers submit pins, guards, transactions, or deltas.
pub struct Revision<S> {
    space: SpaceId<S>,
    sequence: u64,
}

impl<S> Copy for Revision<S> {}

impl<S> Clone for Revision<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> fmt::Debug for Revision<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Revision")
            .field("space", &self.space)
            .field("sequence", &self.sequence)
            .finish()
    }
}

impl<S> PartialEq for Revision<S> {
    fn eq(&self, other: &Self) -> bool {
        self.space == other.space && self.sequence == other.sequence
    }
}

impl<S> Eq for Revision<S> {}

impl<S> PartialOrd for Revision<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S> Ord for Revision<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.space
            .cmp(&other.space)
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

impl<S> Hash for Revision<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.space.hash(state);
        self.sequence.hash(state);
    }
}

impl<S> Revision<S> {
    /// Creates the initial revision for one space instance.
    #[must_use]
    pub const fn initial(space: SpaceId<S>) -> Self {
        Self::new(space, 0)
    }

    /// Creates a revision from a space id and host-owned monotonic sequence.
    #[must_use]
    pub const fn new(space: SpaceId<S>, sequence: u64) -> Self {
        Self { space, sequence }
    }

    /// Returns the owning space instance.
    #[must_use]
    pub const fn space(self) -> SpaceId<S> {
        self.space
    }

    /// Returns the host-owned monotonic sequence.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.sequence
    }

    /// Returns the next revision in the same space, wrapping only after `u64::MAX`.
    #[must_use]
    pub const fn next(self) -> Self {
        Self::new(self.space, self.sequence.wrapping_add(1))
    }
}

/// Resolved contextual information for one occurrence.
///
/// `R` is durable referent identity and `O` is contextual occurrence identity.
/// The two are intentionally stored separately even when a domain happens to
/// use the same representation for both.
///
/// A host normally produces a location while resolving a
/// [`Locator`](crate::Locator) or executing a [`Query`](crate::Query). Callers
/// inspect its context, retain its durable identities, or combine it with a
/// typed facet using [`Endpoint`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Location<S, V, R, O> {
    view: V,
    revision: Revision<S>,
    referent: R,
    occurrence: O,
    address: AbsoluteAddress<S>,
}

impl<S, V, R, O> Location<S, V, R, O> {
    /// Creates resolved occurrence context on behalf of a domain host.
    #[must_use]
    pub const fn new(
        view: V,
        revision: Revision<S>,
        referent: R,
        occurrence: O,
        address: AbsoluteAddress<S>,
    ) -> Self {
        Self {
            view,
            revision,
            referent,
            occurrence,
            address,
        }
    }

    /// Returns the runtime address-space identity.
    #[must_use]
    pub const fn space(&self) -> SpaceId<S> {
        self.revision.space()
    }

    /// Returns the named domain view.
    #[must_use]
    pub const fn view(&self) -> &V {
        &self.view
    }

    /// Returns the revision against which this occurrence was resolved.
    #[must_use]
    pub const fn revision(&self) -> Revision<S> {
        self.revision
    }

    /// Returns durable semantic referent identity.
    #[must_use]
    pub const fn referent(&self) -> &R {
        &self.referent
    }

    /// Returns contextual occurrence identity.
    #[must_use]
    pub const fn occurrence(&self) -> &O {
        &self.occurrence
    }

    /// Returns the canonical exact address of this occurrence in its view.
    #[must_use]
    pub const fn address(&self) -> &AbsoluteAddress<S> {
        &self.address
    }

    /// Decomposes the location into its typed parts.
    #[must_use]
    pub fn into_parts(self) -> (V, Revision<S>, R, O, AbsoluteAddress<S>) {
        (
            self.view,
            self.revision,
            self.referent,
            self.occurrence,
            self.address,
        )
    }
}

/// A resolved referent value paired with the context through which it was found.
///
/// A host can use this as the return type of a read that yields the referent
/// value itself but must not discard the location, revision, view, or occurrence
/// through which it was obtained. Addressable does not produce this pair
/// automatically; a host chooses it when that return shape matches its API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located<T, L> {
    referent: T,
    location: L,
}

impl<T, L> Located<T, L> {
    /// Pairs a referent value with its resolved location on behalf of a host.
    #[must_use]
    pub const fn new(referent: T, location: L) -> Self {
        Self { referent, location }
    }

    /// Returns the semantic referent value.
    #[must_use]
    pub const fn referent(&self) -> &T {
        &self.referent
    }

    /// Returns the occurrence context.
    #[must_use]
    pub const fn location(&self) -> &L {
        &self.location
    }

    /// Decomposes the pair.
    #[must_use]
    pub fn into_parts(self) -> (T, L) {
        (self.referent, self.location)
    }
}

/// A typed addressable facet on a located owner.
///
/// Callers normally create an endpoint from a host-produced [`Location`] and a
/// domain-defined facet marker, then pass it to a matching host read, explain,
/// or edit API. The facet type prevents unrelated values from being read or
/// written through the same owner by accident.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Endpoint<L, F> {
    owner: L,
    facet: F,
}

impl<L, F> Endpoint<L, F> {
    /// Creates a typed endpoint.
    #[must_use]
    pub const fn new(owner: L, facet: F) -> Self {
        Self { owner, facet }
    }

    /// Returns the located owner.
    #[must_use]
    pub const fn owner(&self) -> &L {
        &self.owner
    }

    /// Returns the typed facet.
    #[must_use]
    pub const fn facet(&self) -> &F {
        &self.facet
    }

    /// Decomposes the endpoint.
    #[must_use]
    pub fn into_parts(self) -> (L, F) {
        (self.owner, self.facet)
    }
}

/// Efficient host-local capability resolved at one revision.
///
/// `H` may be an arena slot, generational handle, interned id, or another
/// runtime accelerator. This wrapper carries context but intentionally has no
/// textual serialization API.
///
/// A domain may produce this from a validated [`Location`] and accept it in
/// host-specific fast paths. Callers must reacquire it after the host revision
/// changes. Addressable itself defines no operation on `H` and deliberately
/// provides no persistence or automatic freshness check for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedHandle<S, H> {
    revision: Revision<S>,
    handle: H,
}

impl<S, H> ResolvedHandle<S, H> {
    /// Wraps a host-local handle at the revision where the host resolved it.
    #[must_use]
    pub const fn new(revision: Revision<S>, handle: H) -> Self {
        Self { revision, handle }
    }

    /// Returns the owning space instance.
    #[must_use]
    pub const fn space(&self) -> SpaceId<S> {
        self.revision.space()
    }

    /// Returns the revision at which the handle was resolved.
    #[must_use]
    pub const fn revision(&self) -> Revision<S> {
        self.revision
    }

    /// Returns the host-local handle.
    #[must_use]
    pub const fn handle(&self) -> &H {
        &self.handle
    }
}

#[cfg(test)]
mod tests {
    use super::{Location, Revision, SpaceId};
    use crate::AbsoluteAddress;

    #[derive(Debug, PartialEq, Eq)]
    struct Space;

    #[test]
    fn occurrence_equality_does_not_erase_referent_equality() {
        let address_a = AbsoluteAddress::<Space>::parse("/root/a").expect("valid path");
        let address_b = AbsoluteAddress::<Space>::parse("/root/b").expect("valid path");
        let a = Location::new(
            0_u8,
            Revision::initial(SpaceId::new(1)),
            7_u64,
            1_u64,
            address_a,
        );
        let b = Location::new(
            0_u8,
            Revision::initial(SpaceId::new(1)),
            7_u64,
            2_u64,
            address_b,
        );

        assert_eq!(
            a.referent(),
            b.referent(),
            "the semantic referent is shared"
        );
        assert_ne!(
            a.occurrence(),
            b.occurrence(),
            "occurrences remain distinct"
        );
        assert_ne!(a, b, "location equality includes occurrence context");
    }
}
