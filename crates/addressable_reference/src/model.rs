// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reference-domain identities and typed query vocabulary.

use std::{fmt, num::ParseIntError, str::FromStr, string::String};

use addressable::{AbsoluteAddress, Location, Locator, Many, Query, Resolution, Revision};

/// Type marker for the basilica address space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BasilicaSpace {}

/// Durable semantic identity of one basilica feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId(u64);

impl FeatureId {
    /// Creates a durable feature id from a domain-assigned value.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the domain-assigned value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for FeatureId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for FeatureId {
    type Err = ParseIntError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse().map(Self)
    }
}

/// Identity of one contextual appearance in one basilica view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OccurrenceId(u64);

impl OccurrenceId {
    /// Creates an occurrence id from a host-assigned value.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the host-assigned value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Identity of one addressable relationship occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(u64);

impl EdgeId {
    pub(crate) const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the host-assigned value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Runtime-local dense feature slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotHandle(u32);

impl SlotHandle {
    pub(crate) const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the runtime-local slot value for diagnostics.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Named view exposed by the basilica space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BasilicaView {
    /// Rooted assembly occurrences with canonical hierarchical addresses.
    Assembly,
    /// Relationship view over load dependencies, including a deliberate cycle.
    Dependency,
}

impl fmt::Display for BasilicaView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Assembly => "assembly",
            Self::Dependency => "dependency",
        })
    }
}

impl FromStr for BasilicaView {
    type Err = BasilicaViewParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "assembly" => Ok(Self::Assembly),
            "dependency" => Ok(Self::Dependency),
            _ => Err(BasilicaViewParseError),
        }
    }
}

/// A dynamic or persisted view name was not part of the basilica schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BasilicaViewParseError;

/// Semantic feature classification used by typed predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureKind {
    /// Complete building.
    Basilica,
    /// Nave assembly.
    Nave,
    /// Shared semantic arch feature.
    Arch,
    /// Vault supported by the arch.
    Vault,
    /// Altar assembly.
    Altar,
}

/// Typed navigation axes understood by [`Basilica`].
///
/// [`Basilica`]: crate::Basilica
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BasilicaAxis {
    /// Direct assembly children.
    Children,
    /// Recursive outgoing relationships in the current view.
    Descendants,
    /// Direct outgoing dependency relationships.
    Dependencies,
    /// Direct incoming dependency relationships.
    Dependents,
    /// Cross explicitly to every occurrence of the same referent in a named view.
    ToView(BasilicaView),
}

/// Typed node predicates understood by [`Basilica`].
///
/// [`Basilica`]: crate::Basilica
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BasilicaPredicate {
    /// Match every occurrence.
    Any,
    /// Match a semantic feature kind.
    Kind(FeatureKind),
    /// Match an effective load greater than or equal to a threshold.
    LoadAtLeast(i64),
    /// Match a case-sensitive substring in the feature name.
    NameContains(String),
}

/// Resolved occurrence type for the basilica space.
pub type BasilicaLocation = Location<BasilicaSpace, BasilicaView, FeatureId, OccurrenceId>;

/// View-qualified locator type for the basilica space.
pub type BasilicaLocator = Locator<BasilicaSpace, BasilicaView>;

/// Rich resolution outcome for a basilica occurrence.
pub type BasilicaResolution =
    Resolution<BasilicaSpace, BasilicaLocation, FeatureId, AbsoluteAddress<BasilicaSpace>>;

/// Typed basilica query, defaulting to many-result cardinality.
pub type BasilicaQuery<C = Many> = Query<BasilicaLocator, BasilicaAxis, BasilicaPredicate, C>;

/// Marker for the typed effective-load endpoint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Load;

/// Capability required to change a load endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditCapability {
    /// Author an effective load opinion.
    SetLoad,
}

/// Domain-owned provenance for one load opinion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoadProvenance {
    /// Explicitly authored load.
    Authored {
        /// Revision at which the opinion was authored.
        revision: Revision<BasilicaSpace>,
    },
    /// Schema-provided fallback.
    Default {
        /// Stable schema rule name.
        rule: &'static str,
    },
}

/// Domain-owned reason explaining the winning load opinion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoadReason {
    /// The authored opinion has greater strength than the default.
    AuthoredOverridesDefault,
    /// No authored opinion exists, so the default is effective.
    DefaultUsed,
}

#[derive(Clone, Debug)]
pub(crate) struct Feature {
    pub(crate) id: FeatureId,
    pub(crate) name: String,
    pub(crate) kind: FeatureKind,
    pub(crate) default_load: i64,
    pub(crate) authored_load: Option<i64>,
    pub(crate) authored_revision: Revision<BasilicaSpace>,
    pub(crate) editable: bool,
}

impl Feature {
    pub(crate) const fn effective_load(&self) -> i64 {
        match self.authored_load {
            Some(load) => load,
            None => self.default_load,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Occurrence {
    pub(crate) id: OccurrenceId,
    pub(crate) referent: FeatureId,
    pub(crate) view: BasilicaView,
    pub(crate) address: AbsoluteAddress<BasilicaSpace>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EdgeKind {
    Assembly,
    Dependency,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Edge {
    pub(crate) id: EdgeId,
    pub(crate) from: OccurrenceId,
    pub(crate) to: OccurrenceId,
    pub(crate) kind: EdgeKind,
}
