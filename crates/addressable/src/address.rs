// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validated names, structured addresses, locators, and pinned references.

use alloc::{boxed::Box, string::ToString, vec::Vec};
use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    str::FromStr,
};

use crate::{Revision, SpaceId};

/// One validated address segment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(Box<str>);

impl Name {
    /// Validates and owns one address segment.
    pub fn new(value: impl AsRef<str>) -> Result<Self, NameError> {
        let value = value.as_ref();
        if value.is_empty() {
            return Err(NameError::Empty);
        }
        if value == "." || value == ".." {
            return Err(NameError::Reserved);
        }
        if value.contains('/') {
            return Err(NameError::ContainsSlash);
        }
        if value.contains('\0') {
            return Err(NameError::ContainsNul);
        }
        Ok(Self(Box::from(value)))
    }

    /// Returns the validated segment text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Failure while validating one [`Name`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    /// A segment cannot be empty.
    Empty,
    /// `.` and `..` are reserved for relative navigation.
    Reserved,
    /// A segment cannot contain the `/` separator.
    ContainsSlash,
    /// A segment cannot contain a NUL character.
    ContainsNul,
}

/// A normalized, structured absolute address in typed space `S`.
pub struct AbsoluteAddress<S> {
    segments: Box<[Name]>,
    marker: PhantomData<fn() -> S>,
}

impl<S> Clone for AbsoluteAddress<S> {
    fn clone(&self) -> Self {
        Self {
            segments: self.segments.clone(),
            marker: PhantomData,
        }
    }
}

impl<S> fmt::Debug for AbsoluteAddress<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("AbsoluteAddress")
            .field(&self.to_string())
            .finish()
    }
}

impl<S> PartialEq for AbsoluteAddress<S> {
    fn eq(&self, other: &Self) -> bool {
        self.segments == other.segments
    }
}

impl<S> Eq for AbsoluteAddress<S> {}

impl<S> PartialOrd for AbsoluteAddress<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S> Ord for AbsoluteAddress<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.segments.cmp(&other.segments)
    }
}

impl<S> Hash for AbsoluteAddress<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.segments.hash(state);
    }
}

impl<S> AbsoluteAddress<S> {
    /// Returns the root address (`/`).
    #[must_use]
    pub fn root() -> Self {
        Self {
            segments: Box::default(),
            marker: PhantomData,
        }
    }

    /// Builds an address from already validated segments.
    #[must_use]
    pub fn from_names(names: impl IntoIterator<Item = Name>) -> Self {
        Self {
            segments: names.into_iter().collect::<Vec<_>>().into_boxed_slice(),
            marker: PhantomData,
        }
    }

    /// Parses and normalizes an absolute textual address.
    ///
    /// `.` components are removed and `..` components cancel a preceding
    /// segment. Traversal above the root is rejected.
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        if !text.starts_with('/') {
            return Err(AddressError::NotAbsolute);
        }
        if text == "/" {
            return Ok(Self::root());
        }

        let mut names = Vec::new();
        for component in text[1..].split('/') {
            match component {
                "" => return Err(AddressError::EmptySegment),
                "." => {}
                ".." => {
                    names.pop().ok_or(AddressError::TraversesAboveRoot)?;
                }
                value => names.push(Name::new(value).map_err(AddressError::InvalidName)?),
            }
        }
        Ok(Self::from_names(names))
    }

    /// Returns the number of name segments.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Returns the validated segments.
    #[must_use]
    pub fn segments(&self) -> &[Name] {
        &self.segments
    }

    /// Returns the parent, or `None` for the root.
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        let (_, parents) = self.segments.split_last()?;
        Some(Self::from_names(parents.iter().cloned()))
    }

    /// Resolves a structured relative address against this address.
    pub fn join(&self, relative: &RelativeAddress<S>) -> Result<Self, AddressError> {
        let keep = self
            .segments
            .len()
            .checked_sub(
                usize::try_from(relative.upward)
                    .expect("u32 relative depth must fit this platform's usize"),
            )
            .ok_or(AddressError::TraversesAboveRoot)?;
        let names = self.segments[..keep]
            .iter()
            .chain(relative.segments.iter())
            .cloned();
        Ok(Self::from_names(names))
    }

    /// Computes a normalized relative address from `base` to `self`.
    #[must_use]
    pub fn relative_to(&self, base: &Self) -> RelativeAddress<S> {
        let common = self
            .segments
            .iter()
            .zip(base.segments.iter())
            .take_while(|(left, right)| left == right)
            .count();
        let upward = u32::try_from(base.segments.len() - common)
            .expect("address depth exceeds representable relative depth");
        RelativeAddress {
            upward,
            segments: self.segments[common..].to_vec().into_boxed_slice(),
            marker: PhantomData,
        }
    }
}

impl<S> fmt::Display for AbsoluteAddress<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            return formatter.write_str("/");
        }
        for name in &self.segments {
            write!(formatter, "/{name}")?;
        }
        Ok(())
    }
}

/// A normalized structured address relative to an explicit base.
pub struct RelativeAddress<S> {
    upward: u32,
    segments: Box<[Name]>,
    marker: PhantomData<fn() -> S>,
}

impl<S> Clone for RelativeAddress<S> {
    fn clone(&self) -> Self {
        Self {
            upward: self.upward,
            segments: self.segments.clone(),
            marker: PhantomData,
        }
    }
}

impl<S> fmt::Debug for RelativeAddress<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("RelativeAddress")
            .field(&self.to_string())
            .finish()
    }
}

impl<S> PartialEq for RelativeAddress<S> {
    fn eq(&self, other: &Self) -> bool {
        self.upward == other.upward && self.segments == other.segments
    }
}

impl<S> Eq for RelativeAddress<S> {}

impl<S> PartialOrd for RelativeAddress<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S> Ord for RelativeAddress<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.upward
            .cmp(&other.upward)
            .then_with(|| self.segments.cmp(&other.segments))
    }
}

impl<S> Hash for RelativeAddress<S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.upward.hash(state);
        self.segments.hash(state);
    }
}

impl<S> RelativeAddress<S> {
    /// Returns the identity relative address (`.`).
    #[must_use]
    pub fn current() -> Self {
        Self {
            upward: 0,
            segments: Box::default(),
            marker: PhantomData,
        }
    }

    /// Parses and normalizes relative text.
    ///
    /// Parent components cancel preceding child components before increasing
    /// the stored upward count.
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        if text.starts_with('/') {
            return Err(AddressError::NotRelative);
        }
        if text.is_empty() || text == "." {
            return Ok(Self::current());
        }

        let mut upward = 0_u32;
        let mut names = Vec::new();
        for component in text.split('/') {
            match component {
                "" => return Err(AddressError::EmptySegment),
                "." => {}
                ".." => {
                    if names.pop().is_none() {
                        upward = upward.checked_add(1).ok_or(AddressError::DepthOverflow)?;
                    }
                }
                value => names.push(Name::new(value).map_err(AddressError::InvalidName)?),
            }
        }
        Ok(Self {
            upward,
            segments: names.into_boxed_slice(),
            marker: PhantomData,
        })
    }

    /// Returns the number of parents traversed before descending.
    #[must_use]
    pub const fn upward(&self) -> u32 {
        self.upward
    }

    /// Returns the child segments after parent traversal.
    #[must_use]
    pub fn segments(&self) -> &[Name] {
        &self.segments
    }
}

impl<S> fmt::Display for RelativeAddress<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.upward == 0 && self.segments.is_empty() {
            return formatter.write_str(".");
        }
        let mut needs_separator = false;
        for _ in 0..self.upward {
            if needs_separator {
                formatter.write_str("/")?;
            }
            formatter.write_str("..")?;
            needs_separator = true;
        }
        for name in &self.segments {
            if needs_separator {
                formatter.write_str("/")?;
            }
            name.fmt(formatter)?;
            needs_separator = true;
        }
        Ok(())
    }
}

/// Failure while parsing, normalizing, or joining an address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressError {
    /// An absolute address did not begin with `/`.
    NotAbsolute,
    /// A relative address began with `/`.
    NotRelative,
    /// Two separators produced an empty segment.
    EmptySegment,
    /// One name segment was invalid.
    InvalidName(NameError),
    /// Normalization or joining attempted to traverse above the root.
    TraversesAboveRoot,
    /// Relative parent depth exceeded `u32`.
    DepthOverflow,
}

/// The structured recipe carried by a [`Locator`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LocatorKind<S> {
    /// A canonical exact address.
    Exact(AbsoluteAddress<S>),
    /// A relative address with an explicit exact base.
    Relative {
        /// Exact base against which `path` is interpreted.
        base: AbsoluteAddress<S>,
        /// Normalized path relative to `base`.
        path: RelativeAddress<S>,
    },
}

/// A view-qualified resolution recipe in one runtime space instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locator<S, V> {
    space: SpaceId<S>,
    view: V,
    kind: LocatorKind<S>,
}

impl<S, V> Locator<S, V> {
    /// Creates an exact locator.
    #[must_use]
    pub const fn exact(space: SpaceId<S>, view: V, address: AbsoluteAddress<S>) -> Self {
        Self {
            space,
            view,
            kind: LocatorKind::Exact(address),
        }
    }

    /// Creates a relative locator with an explicit exact base.
    #[must_use]
    pub const fn relative(
        space: SpaceId<S>,
        view: V,
        base: AbsoluteAddress<S>,
        path: RelativeAddress<S>,
    ) -> Self {
        Self {
            space,
            view,
            kind: LocatorKind::Relative { base, path },
        }
    }

    /// Returns the runtime space instance.
    #[must_use]
    pub const fn space(&self) -> SpaceId<S> {
        self.space
    }

    /// Returns the named view.
    #[must_use]
    pub const fn view(&self) -> &V {
        &self.view
    }

    /// Returns the structured locator recipe.
    #[must_use]
    pub const fn kind(&self) -> &LocatorKind<S> {
        &self.kind
    }

    /// Materializes this recipe as an exact address.
    pub fn to_absolute(&self) -> Result<AbsoluteAddress<S>, AddressError> {
        match &self.kind {
            LocatorKind::Exact(address) => Ok(address.clone()),
            LocatorKind::Relative { base, path } => base.join(path),
        }
    }
}

impl<S, V> fmt::Display for Locator<S, V>
where
    V: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let view = self.view.to_string();
        write!(formatter, "{}:{}:{}:", self.space.get(), view.len(), view)?;
        match &self.kind {
            LocatorKind::Exact(address) => write!(formatter, "E:{address}"),
            LocatorKind::Relative { base, path } => {
                let base = base.to_string();
                write!(formatter, "R:{}:{}:{path}", base.len(), base)
            }
        }
    }
}

impl<S, V> FromStr for Locator<S, V>
where
    V: FromStr,
{
    type Err = LocatorParseError<V::Err>;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (space, rest) = text
            .split_once(':')
            .ok_or(LocatorParseError::InvalidSyntax)?;
        let space = space
            .parse::<u64>()
            .map_err(|_| LocatorParseError::InvalidSpace)?;
        let (view, rest) = take_length_prefixed(rest).ok_or(LocatorParseError::InvalidSyntax)?;
        let view = view.parse().map_err(LocatorParseError::InvalidView)?;
        if let Some(address) = rest.strip_prefix(":E:") {
            return AbsoluteAddress::parse(address)
                .map(|address| Self::exact(SpaceId::new(space), view, address))
                .map_err(LocatorParseError::InvalidAddress);
        }
        let rest = rest
            .strip_prefix(":R:")
            .ok_or(LocatorParseError::InvalidSyntax)?;
        let (base, path) = take_length_prefixed(rest).ok_or(LocatorParseError::InvalidSyntax)?;
        let path = path
            .strip_prefix(':')
            .ok_or(LocatorParseError::InvalidSyntax)?;
        let base = AbsoluteAddress::parse(base).map_err(LocatorParseError::InvalidAddress)?;
        let path = RelativeAddress::parse(path).map_err(LocatorParseError::InvalidAddress)?;
        Ok(Self::relative(SpaceId::new(space), view, base, path))
    }
}

/// Failure to parse a canonical [`Locator`] document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocatorParseError<E> {
    /// Length prefixes or structural separators were malformed.
    InvalidSyntax,
    /// Runtime space identity was not a `u64`.
    InvalidSpace,
    /// The domain view name was invalid.
    InvalidView(E),
    /// An exact, base, or relative address was invalid.
    InvalidAddress(AddressError),
}

/// A locator pinned to expected semantic identity and revision.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Pinned<S, V, I> {
    locator: Locator<S, V>,
    expected_referent: I,
    expected_revision: Revision<S>,
}

impl<S, V, I> Pinned<S, V, I> {
    /// Pins a locator to identity observed at `expected_revision`.
    #[must_use]
    pub const fn new(
        locator: Locator<S, V>,
        expected_referent: I,
        expected_revision: Revision<S>,
    ) -> Self {
        Self {
            locator,
            expected_referent,
            expected_revision,
        }
    }

    /// Returns the underlying locator.
    #[must_use]
    pub const fn locator(&self) -> &Locator<S, V> {
        &self.locator
    }

    /// Returns expected semantic identity.
    #[must_use]
    pub const fn expected_referent(&self) -> &I {
        &self.expected_referent
    }

    /// Returns the revision at which the pin was established.
    #[must_use]
    pub const fn expected_revision(&self) -> Revision<S> {
        self.expected_revision
    }
}

impl<S, V, I> fmt::Display for Pinned<S, V, I>
where
    V: fmt::Display,
    I: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let locator = self.locator.to_string();
        let identity = self.expected_referent.to_string();
        write!(
            formatter,
            "{}:{}{}:{}:{}:{}",
            locator.len(),
            locator,
            identity.len(),
            identity,
            self.expected_revision.space().get(),
            self.expected_revision.get()
        )
    }
}

impl<S, V, I> FromStr for Pinned<S, V, I>
where
    V: FromStr,
    I: FromStr,
{
    type Err = PinnedParseError<V::Err, I::Err>;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (locator, rest) = take_length_prefixed(text).ok_or(PinnedParseError::InvalidSyntax)?;
        let locator = locator.parse().map_err(PinnedParseError::InvalidLocator)?;
        let (identity, revision) =
            take_length_prefixed(rest).ok_or(PinnedParseError::InvalidSyntax)?;
        let revision = revision
            .strip_prefix(':')
            .ok_or(PinnedParseError::InvalidSyntax)?;
        let (revision_space, revision) = revision
            .split_once(':')
            .ok_or(PinnedParseError::InvalidSyntax)?;
        let identity = identity
            .parse()
            .map_err(PinnedParseError::InvalidIdentity)?;
        let revision_space = revision_space
            .parse::<u64>()
            .map_err(|_| PinnedParseError::InvalidRevision)?;
        let revision = revision
            .parse::<u64>()
            .map_err(|_| PinnedParseError::InvalidRevision)?;
        Ok(Self::new(
            locator,
            identity,
            Revision::new(SpaceId::new(revision_space), revision),
        ))
    }
}

/// Failure to parse a canonical [`Pinned`] document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinnedParseError<V, I> {
    /// Length prefixes or structural separators were malformed.
    InvalidSyntax,
    /// The embedded locator was invalid.
    InvalidLocator(LocatorParseError<V>),
    /// Expected semantic identity could not be recovered.
    InvalidIdentity(I),
    /// Expected revision was not a `u64`.
    InvalidRevision,
}

fn take_length_prefixed(text: &str) -> Option<(&str, &str)> {
    let (length, rest) = text.split_once(':')?;
    let length = length.parse::<usize>().ok()?;
    let value = rest.get(..length)?;
    Some((value, &rest[length..]))
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::{AbsoluteAddress, AddressError, Locator, Pinned, RelativeAddress};
    use crate::{Revision, SpaceId};

    #[derive(Debug, PartialEq, Eq)]
    struct Space;

    #[test]
    fn exact_address_parse_and_format_round_trip() {
        let path = AbsoluteAddress::<Space>::parse("/basilica/./nave/side/../arch")
            .expect("address should normalize");
        assert_eq!(path.to_string(), "/basilica/nave/arch");
        assert_eq!(
            AbsoluteAddress::<Space>::parse(&path.to_string()).expect("canonical text parses"),
            path
        );
    }

    #[test]
    fn normalization_is_idempotent() {
        let once = RelativeAddress::<Space>::parse("chapel/../nave/./arch")
            .expect("relative address should normalize");
        let twice = RelativeAddress::<Space>::parse(&once.to_string())
            .expect("canonical relative address parses");
        assert_eq!(once, twice);
        assert_eq!(once.to_string(), "nave/arch");
    }

    #[test]
    fn join_and_relativize_are_inverse() {
        let base = AbsoluteAddress::<Space>::parse("/basilica/nave").expect("valid base");
        let target =
            AbsoluteAddress::<Space>::parse("/basilica/transept/arch").expect("valid target");
        let relative = target.relative_to(&base);
        assert_eq!(relative.to_string(), "../transept/arch");
        assert_eq!(base.join(&relative).expect("relative joins"), target);
    }

    #[test]
    fn traversal_above_root_is_rejected() {
        assert_eq!(
            AbsoluteAddress::<Space>::parse("/../outside"),
            Err(AddressError::TraversesAboveRoot)
        );
    }

    #[test]
    fn exact_relative_and_pinned_locator_documents_round_trip() {
        let exact = Locator::<Space, u8>::exact(
            SpaceId::new(7),
            2,
            AbsoluteAddress::parse("/basilica/nave/a:|#~rch")
                .expect("delimiter characters belong to the address data"),
        );
        let exact_text = exact.to_string();
        assert_eq!(
            exact_text
                .parse::<Locator<Space, u8>>()
                .expect("locator parses"),
            exact
        );

        let relative = Locator::<Space, u8>::relative(
            SpaceId::new(7),
            2,
            AbsoluteAddress::parse("/basilica/nave").expect("valid base"),
            RelativeAddress::parse("../transept/arch").expect("valid relative path"),
        );
        let relative_text = relative.to_string();
        assert_eq!(
            relative_text
                .parse::<Locator<Space, u8>>()
                .expect("relative locator parses"),
            relative
        );

        let pinned = Pinned::new(relative, 42_u64, Revision::new(SpaceId::new(7), 9));
        let pinned_text = pinned.to_string();
        assert_eq!(
            pinned_text
                .parse::<Pinned<Space, u8, u64>>()
                .expect("pinned locator parses"),
            pinned
        );
    }
}
