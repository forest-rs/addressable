// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Schema-backed dynamic adaptation for Addressable.
//!
//! Erasure is deliberately confined to this crate. [`ReferenceTool`] validates
//! schema names and dynamic value kinds, then reconstructs the same typed
//! endpoint, guard, and transaction used by ordinary Rust callers.
//!
//! # Dynamic workflow
//!
//! 1. Inspect [`ReferenceTool::schema`] for accepted view, facet, value-kind,
//!    and capability names.
//! 2. Construct a [`DynamicEndpoint`] and pass it to [`ReferenceTool::read`].
//! 3. Form a [`DynamicGuard`] directly from the returned
//!    [`DynamicExplanation`], then submit a [`DynamicTransaction`].
//!
//! The complete example on [`ReferenceTool`] is a tooling-only call path: it
//! never reaches around the adapter to recover typed state.

use std::{string::String, vec::Vec};

use addressable::{
    AbsoluteAddress, AddressError, Endpoint, Guard, Locator, Opinion, Resolution, Revision,
    Transaction, TransactionMode,
};
use addressable_reference::{
    Basilica, BasilicaLocation, BasilicaView, EditCapability, FeatureId, Load, LoadProvenance,
    ReadError, SetLoad, TransactionConflict,
};

/// Dynamic value kind declared by a tooling schema.
///
/// Read this from `FacetSchema::value_kind` or obtain it from
/// [`DynamicValue::kind`] before constructing a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DynamicKind {
    /// Signed 64-bit integer.
    Integer,
    /// Owned UTF-8 text.
    Text,
}

/// Value crossing the schema-backed tooling boundary.
///
/// This enum is not used by `addressable` or by typed reference storage.
/// Callers construct values according to `FacetSchema::value_kind`; read and
/// transaction reports return values in the same representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DynamicValue {
    /// Signed 64-bit integer.
    Integer(i64),
    /// Owned UTF-8 text.
    Text(String),
}

impl DynamicValue {
    /// Returns the schema kind of this value.
    #[must_use]
    pub const fn kind(&self) -> DynamicKind {
        match self {
            Self::Integer(_) => DynamicKind::Integer,
            Self::Text(_) => DynamicKind::Text,
        }
    }
}

/// Operation capability declared for one dynamic facet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolCapability {
    /// Read the effective value.
    Read,
    /// Explain candidates and provenance.
    Explain,
    /// Apply a guarded set operation.
    Set,
}

/// Schema for one named address-space view.
///
/// Returned as part of [`ReferenceTool::schema`]; tooling does not need to
/// construct this reference adapter's schema itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewSchema {
    /// Stable dynamic name.
    pub name: &'static str,
}

/// Schema for one addressable facet.
///
/// Returned as part of [`ReferenceTool::schema`]. Its name is accepted by
/// `DynamicEndpoint::facet`, and its value kind governs reads and sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FacetSchema {
    /// Stable dynamic name.
    pub name: &'static str,
    /// Accepted value kind.
    pub value_kind: DynamicKind,
    /// Supported operations.
    pub capabilities: &'static [ToolCapability],
}

/// Declared dynamic schema for one typed object-space adapter.
///
/// Obtain this from [`ReferenceTool::schema`] before constructing locators or
/// endpoint requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectSpaceSchema {
    /// Stable schema identity.
    pub name: &'static str,
    /// Named views accepted by dynamic locators.
    pub views: &'static [ViewSchema],
    /// Addressable facets exposed by the adapter.
    pub facets: &'static [FacetSchema],
}

const VIEWS: &[ViewSchema] = &[
    ViewSchema { name: "assembly" },
    ViewSchema { name: "dependency" },
];
const LOAD_CAPABILITIES: &[ToolCapability] = &[
    ToolCapability::Read,
    ToolCapability::Explain,
    ToolCapability::Set,
];
const FACETS: &[FacetSchema] = &[FacetSchema {
    name: "load",
    value_kind: DynamicKind::Integer,
    capabilities: LOAD_CAPABILITIES,
}];
const BASILICA_SCHEMA: ObjectSpaceSchema = ObjectSpaceSchema {
    name: "addressable.reference.basilica/v1",
    views: VIEWS,
    facets: FACETS,
};

/// Erased but schema-qualified locator.
///
/// Construct this from a runtime space id plus view and address syntax declared
/// by [`ObjectSpaceSchema`]. It becomes the owner of a [`DynamicEndpoint`] and
/// is validated by [`ReferenceTool::read`] or [`ReferenceTool::transact`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicLocator {
    /// Runtime space id.
    pub space: u64,
    /// Stable view name from [`ObjectSpaceSchema::views`].
    pub view: String,
    /// Textual exact address to parse into the typed representation.
    pub address: String,
}

/// Erased typed-facet endpoint.
///
/// Callers construct this from a [`DynamicLocator`] and a facet name from
/// `ObjectSpaceSchema::facets`. Pass it to [`ReferenceTool::read`] or include
/// it in a [`DynamicSet`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicEndpoint {
    /// Located owner recipe.
    pub owner: DynamicLocator,
    /// Stable facet name from [`ObjectSpaceSchema::facets`].
    pub facet: String,
}

/// Erased preconditions for one set operation.
///
/// Copy the referent, space, revision, and value from the
/// [`DynamicExplanation`] returned by [`ReferenceTool::read`]. The adapter
/// validates all four values before delegating to the typed transaction path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicGuard {
    /// Expected durable semantic identity.
    pub expected_referent: u64,
    /// Runtime space in which the revision was observed.
    pub expected_space: u64,
    /// Expected space-local revision sequence.
    pub expected_revision: u64,
    /// Expected typed value after schema recovery.
    pub expected_value: DynamicValue,
}

/// One erased guarded set operation.
///
/// Combine a [`DynamicEndpoint`], a schema-compatible proposed value, and a
/// [`DynamicGuard`], then include it in `DynamicTransaction::operations`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicSet {
    /// Addressed facet.
    pub endpoint: DynamicEndpoint,
    /// Proposed value.
    pub value: DynamicValue,
    /// Required preconditions.
    pub guard: DynamicGuard,
}

/// Snapshot-scoped dynamic transaction request.
///
/// Use the space and revision from the same [`DynamicExplanation`] that supplied
/// each operation's guard. Submit the request to [`ReferenceTool::transact`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicTransaction {
    /// Runtime space in which the target set was selected.
    pub selection_space: u64,
    /// Revision sequence at which the target set was selected.
    pub selection_revision: u64,
    /// Preview or commit mode.
    pub mode: TransactionMode,
    /// Guarded operations, applied atomically.
    pub operations: Vec<DynamicSet>,
}

/// One erased opinion in a structured explanation.
///
/// Produced inside `DynamicExplanation::opinions` by
/// [`ReferenceTool::read`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicOpinion {
    /// Typed value after erasure.
    pub value: DynamicValue,
    /// Stable provenance category.
    pub provenance: &'static str,
    /// Revision attached to authored provenance, when present.
    pub revision: Option<u64>,
}

/// Structured dynamic value explanation.
///
/// Produced by [`ReferenceTool::read`]. The subject, space, revision, and value
/// are exactly the observation needed to construct a [`DynamicGuard`]; opinions
/// and reason explain how the effective value was selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicExplanation {
    /// Durable semantic subject identity.
    pub subject: u64,
    /// Runtime space in which the value was observed.
    pub space: u64,
    /// Space-local revision at which the value was observed.
    pub revision: u64,
    /// Effective value.
    pub value: DynamicValue,
    /// Candidate opinions in typed domain strength order.
    pub opinions: Vec<DynamicOpinion>,
    /// Winning opinion index.
    pub winner: usize,
    /// Stable domain reason.
    pub reason: &'static str,
}

/// One effective dynamic change produced in a [`DynamicTransactionReport`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DynamicChange {
    /// Durable semantic subject identity.
    pub referent: u64,
    /// Previous effective integer.
    pub previous: i64,
    /// New effective integer.
    pub current: i64,
}

/// Dynamic transaction report produced by [`ReferenceTool::transact`].
///
/// Inspect [`Self::changes`] for effective changes and [`Self::undo`] for the
/// preconditions required by a future, separately guarded undo operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicTransactionReport {
    /// Preview or apply mode.
    pub mode: TransactionMode,
    /// Validated previous revision.
    pub revision_before: u64,
    /// Resulting revision.
    pub revision_after: u64,
    /// Effective changes.
    pub changes: Vec<DynamicChange>,
    /// Typed undo information recovered for the dynamic boundary.
    pub undo: Vec<DynamicUndo>,
}

/// Dynamic form of one typed authored-load undo record.
///
/// Returned in `DynamicTransactionReport::undo`; it is information for
/// constructing a future guarded request, not an immediately executable token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DynamicUndo {
    /// Durable semantic subject identity.
    pub referent: u64,
    /// Authored value that existed before the transaction.
    pub previous_authored: Option<i64>,
    /// Authored value that a future guarded undo must still observe.
    pub expected_authored: i64,
}

/// Schema-backed adapter around one typed basilica host.
///
/// This is the entry point for dynamic callers. Read an endpoint first, then
/// derive every transaction precondition from that observation.
///
/// ```
/// use addressable::{SpaceId, TransactionMode};
/// use addressable_reference::{Basilica, BasilicaSpace};
/// use addressable_tooling::{
///     DynamicEndpoint, DynamicGuard, DynamicLocator, DynamicSet,
///     DynamicTransaction, DynamicValue, ReferenceTool,
/// };
///
/// let mut basilica = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
/// let mut tool = ReferenceTool::new(&mut basilica);
/// let endpoint = DynamicEndpoint {
///     owner: DynamicLocator {
///         space: 1,
///         view: "assembly".into(),
///         address: "/basilica/nave/north_arch".into(),
///     },
///     facet: "load".into(),
/// };
/// let observed = tool.read(&endpoint).expect("dynamic read succeeds");
/// let report = tool
///     .transact(DynamicTransaction {
///         selection_space: observed.space,
///         selection_revision: observed.revision,
///         mode: TransactionMode::Apply,
///         operations: vec![DynamicSet {
///             endpoint,
///             value: DynamicValue::Integer(80),
///             guard: DynamicGuard {
///                 expected_referent: observed.subject,
///                 expected_space: observed.space,
///                 expected_revision: observed.revision,
///                 expected_value: observed.value,
///             },
///         }],
///     })
///     .expect("guarded dynamic transaction applies");
///
/// assert_eq!(report.changes[0].current, 80);
/// ```
#[derive(Debug)]
pub struct ReferenceTool<'a> {
    space: &'a mut Basilica,
}

impl<'a> ReferenceTool<'a> {
    /// Borrows a typed host through its dynamic tooling adapter.
    #[must_use]
    pub const fn new(space: &'a mut Basilica) -> Self {
        Self { space }
    }

    /// Returns the schema that governs every dynamic operation.
    #[must_use]
    pub const fn schema(&self) -> &'static ObjectSpaceSchema {
        &BASILICA_SCHEMA
    }

    /// Reads and explains an endpoint after recovering its typed schema.
    ///
    /// The returned explanation carries every observation needed to form a
    /// guarded set request against this value.
    pub fn read(&self, endpoint: &DynamicEndpoint) -> Result<DynamicExplanation, ToolError> {
        let endpoint = self.typed_endpoint(endpoint)?;
        let explained = self.space.read_load(&endpoint).map_err(ToolError::Read)?;
        let opinions = explained.opinions().iter().map(dynamic_opinion).collect();
        Ok(DynamicExplanation {
            subject: explained.subject().get(),
            space: self.space.id().get(),
            revision: self.space.revision().get(),
            value: DynamicValue::Integer(*explained.value()),
            opinions,
            winner: explained.winner(),
            reason: match explained.reason() {
                addressable_reference::LoadReason::AuthoredOverridesDefault => {
                    "authored-overrides-default"
                }
                addressable_reference::LoadReason::DefaultUsed => "default-used",
            },
        })
    }

    /// Validates and delegates a dynamic transaction to the typed host path.
    ///
    /// Use one prior [`Self::read`] result to populate the request's selection
    /// context and each operation guard, as shown in the [`ReferenceTool`]
    /// example.
    pub fn transact(
        &mut self,
        request: DynamicTransaction,
    ) -> Result<DynamicTransactionReport, ToolError> {
        if request.selection_space != self.space.id().get() {
            return Err(ToolError::WrongSpace {
                expected: self.space.id().get(),
                actual: request.selection_space,
            });
        }
        let mut operations = Vec::with_capacity(request.operations.len());
        for operation in &request.operations {
            if operation.guard.expected_space != self.space.id().get() {
                return Err(ToolError::WrongSpace {
                    expected: self.space.id().get(),
                    actual: operation.guard.expected_space,
                });
            }
            let endpoint = self.typed_endpoint(&operation.endpoint)?;
            let value = integer(&operation.value)?;
            let expected_value = integer(&operation.guard.expected_value)?;
            operations.push(SetLoad::new(
                endpoint,
                value,
                Guard::new(
                    FeatureId::new(operation.guard.expected_referent),
                    Revision::new(self.space.id(), operation.guard.expected_revision),
                    expected_value,
                    EditCapability::SetLoad,
                ),
            ));
        }
        let revision = Revision::new(self.space.id(), request.selection_revision);
        let transaction = match request.mode {
            TransactionMode::DryRun => Transaction::dry_run(revision, operations),
            TransactionMode::Apply => Transaction::apply(revision, operations),
        };
        let report = self
            .space
            .transact(transaction)
            .map_err(ToolError::Conflict)?;
        Ok(DynamicTransactionReport {
            mode: report.mode(),
            revision_before: report.revision_before().get(),
            revision_after: report.revision_after().get(),
            changes: report
                .changes()
                .iter()
                .map(|change| DynamicChange {
                    referent: change.referent().get(),
                    previous: change.previous(),
                    current: change.current(),
                })
                .collect(),
            undo: report
                .undo()
                .iter()
                .map(|undo| DynamicUndo {
                    referent: undo.referent().get(),
                    previous_authored: undo.previous_authored(),
                    expected_authored: undo.expected_authored(),
                })
                .collect(),
        })
    }

    fn typed_endpoint(
        &self,
        endpoint: &DynamicEndpoint,
    ) -> Result<Endpoint<BasilicaLocation, Load>, ToolError> {
        if endpoint.facet != "load" {
            return Err(ToolError::UnknownFacet(endpoint.facet.clone()));
        }
        if endpoint.owner.space != self.space.id().get() {
            return Err(ToolError::WrongSpace {
                expected: self.space.id().get(),
                actual: endpoint.owner.space,
            });
        }
        let view = match endpoint.owner.view.as_str() {
            "assembly" => BasilicaView::Assembly,
            "dependency" => BasilicaView::Dependency,
            _ => return Err(ToolError::UnknownView(endpoint.owner.view.clone())),
        };
        let address =
            AbsoluteAddress::parse(&endpoint.owner.address).map_err(ToolError::InvalidAddress)?;
        let locator = Locator::exact(self.space.id(), view, address);
        match self.space.resolve(&locator) {
            Resolution::Resolved(location) => Ok(Endpoint::new(location, Load)),
            _ => Err(ToolError::Unresolved),
        }
    }
}

fn integer(value: &DynamicValue) -> Result<i64, ToolError> {
    match value {
        DynamicValue::Integer(value) => Ok(*value),
        other => Err(ToolError::TypeMismatch {
            expected: DynamicKind::Integer,
            actual: other.kind(),
        }),
    }
}

fn dynamic_opinion(opinion: &Opinion<i64, LoadProvenance>) -> DynamicOpinion {
    match opinion.provenance() {
        LoadProvenance::Authored { revision } => DynamicOpinion {
            value: DynamicValue::Integer(*opinion.value()),
            provenance: "authored",
            revision: Some(revision.get()),
        },
        LoadProvenance::Default { .. } => DynamicOpinion {
            value: DynamicValue::Integer(*opinion.value()),
            provenance: "default",
            revision: None,
        },
    }
}

/// Failure returned by [`ReferenceTool::read`] or [`ReferenceTool::transact`].
///
/// Schema and value errors are rejected before delegation. Typed read and
/// transaction failures remain distinguishable in [`Self::Read`] and
/// [`Self::Conflict`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolError {
    /// The locator named another runtime space instance.
    WrongSpace {
        /// Adapted space id.
        expected: u64,
        /// Supplied dynamic space id.
        actual: u64,
    },
    /// The view name does not exist in the declared schema.
    UnknownView(String),
    /// Text could not be parsed as a typed exact address.
    InvalidAddress(AddressError),
    /// The typed locator did not resolve normally.
    Unresolved,
    /// The facet name does not exist in the declared schema.
    UnknownFacet(String),
    /// A dynamic value did not match the facet schema.
    TypeMismatch {
        /// Declared schema kind.
        expected: DynamicKind,
        /// Supplied dynamic kind.
        actual: DynamicKind,
    },
    /// Typed endpoint reading rejected stale or invalid context.
    Read(ReadError),
    /// The typed guarded transaction rejected the request atomically.
    Conflict(TransactionConflict),
}

#[cfg(test)]
mod tests {
    use addressable::{
        AbsoluteAddress, Endpoint, Guard, Locator, Resolution, SpaceId, Transaction,
        TransactionMode,
    };
    use addressable_reference::{
        Basilica, BasilicaSpace, BasilicaView, EditCapability, FeatureId, Load, LoadReason, SetLoad,
    };

    use super::{
        DynamicEndpoint, DynamicGuard, DynamicLocator, DynamicSet, DynamicTransaction,
        DynamicValue, ReferenceTool, ToolError,
    };

    #[test]
    fn dynamic_operation_is_equivalent_to_the_typed_path() {
        let mut typed_space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        let mut dynamic_space = typed_space.clone();
        let locator = Locator::exact(
            typed_space.id(),
            BasilicaView::Assembly,
            AbsoluteAddress::parse("/basilica/nave/north_arch").expect("valid address"),
        );
        let Resolution::Resolved(location) = typed_space.resolve(&locator) else {
            panic!("typed location resolves");
        };
        let endpoint = Endpoint::new(location, Load);
        let explained = typed_space
            .read_load(&endpoint)
            .expect("typed read succeeds");
        assert_eq!(explained.reason(), &LoadReason::AuthoredOverridesDefault);
        typed_space
            .transact(Transaction::apply(
                typed_space.revision(),
                [SetLoad::new(
                    endpoint,
                    80,
                    Guard::new(
                        FeatureId::new(3),
                        typed_space.revision(),
                        120,
                        EditCapability::SetLoad,
                    ),
                )],
            ))
            .expect("typed operation applies");

        let dynamic_endpoint = DynamicEndpoint {
            owner: DynamicLocator {
                space: dynamic_space.id().get(),
                view: "assembly".into(),
                address: "/basilica/nave/north_arch".into(),
            },
            facet: "load".into(),
        };
        let mut tool = ReferenceTool::new(&mut dynamic_space);
        assert_eq!(tool.schema().facets[0].name, "load");
        let before = tool.read(&dynamic_endpoint).expect("dynamic read succeeds");
        assert_eq!(before.value, DynamicValue::Integer(120));
        assert_eq!(before.space, 1);
        assert_eq!(before.revision, 0);
        let report = tool
            .transact(DynamicTransaction {
                selection_space: before.space,
                selection_revision: before.revision,
                mode: TransactionMode::Apply,
                operations: vec![DynamicSet {
                    endpoint: dynamic_endpoint.clone(),
                    value: DynamicValue::Integer(80),
                    guard: DynamicGuard {
                        expected_referent: before.subject,
                        expected_space: before.space,
                        expected_revision: before.revision,
                        expected_value: before.value,
                    },
                }],
            })
            .expect("dynamic operation delegates successfully");
        assert_eq!(report.revision_after, typed_space.revision().get());
        assert_eq!(report.changes[0].current, 80);

        let after = tool
            .read(&dynamic_endpoint)
            .expect("dynamic reread succeeds");
        let typed_locator = Locator::exact(
            typed_space.id(),
            BasilicaView::Assembly,
            AbsoluteAddress::parse("/basilica/nave/north_arch").expect("valid address"),
        );
        let Resolution::Resolved(typed_location) = typed_space.resolve(&typed_locator) else {
            panic!("typed location re-resolves");
        };
        let typed_after = typed_space
            .read_load(&Endpoint::new(typed_location, Load))
            .expect("typed reread succeeds");
        assert_eq!(after.value, DynamicValue::Integer(*typed_after.value()));
    }

    #[test]
    fn dynamic_selection_revision_cannot_cross_spaces() {
        let mut space = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
        let original = space.revision();
        let result = ReferenceTool::new(&mut space).transact(DynamicTransaction {
            selection_space: 2,
            selection_revision: 0,
            mode: TransactionMode::Apply,
            operations: vec![],
        });

        assert_eq!(
            result,
            Err(ToolError::WrongSpace {
                expected: 1,
                actual: 2,
            })
        );
        assert_eq!(space.revision(), original);
    }
}
