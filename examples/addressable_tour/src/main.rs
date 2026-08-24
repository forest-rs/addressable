// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable complete vertical slice for Addressable.

use addressable::{
    AbsoluteAddress, CyclePolicy, Deduplication, Endpoint, Guard, Locator, Pinned, Query,
    Resolution, ResultOrdering, SpaceId, Transaction, TransactionMode, TraversalBudget,
    VisitIdentity,
};
use addressable_reference::{
    Basilica, BasilicaAxis, BasilicaPredicate, BasilicaSpace, BasilicaView, Catalog, CatalogSpace,
    EditCapability, FeatureKind, Load, SetLoad,
};
use addressable_tooling::{
    DynamicEndpoint, DynamicGuard, DynamicLocator, DynamicSet, DynamicTransaction, DynamicValue,
    ReferenceTool,
};

fn main() {
    let mut basilica = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
    let catalog = Catalog::new(SpaceId::<CatalogSpace>::new(2));

    let north_locator = Locator::exact(
        basilica.id(),
        BasilicaView::Assembly,
        AbsoluteAddress::parse("/basilica/nave/north_arch").expect("valid exact address"),
    );
    let south_locator = Locator::relative(
        basilica.id(),
        BasilicaView::Assembly,
        AbsoluteAddress::parse("/basilica/nave").expect("valid relative base"),
        addressable::RelativeAddress::parse("south_arch").expect("valid relative path"),
    );
    let Resolution::Resolved(north) = basilica.resolve(&north_locator) else {
        panic!("north arch must resolve");
    };
    let Resolution::Resolved(south) = basilica.resolve(&south_locator) else {
        panic!("south arch must resolve");
    };
    let relative_document = south_locator.to_string();
    let decoded_relative = relative_document
        .parse::<addressable_reference::BasilicaLocator>()
        .expect("canonical relative locator must parse");
    assert_eq!(
        decoded_relative, south_locator,
        "relative locator serialization must round-trip"
    );
    assert_eq!(
        north.referent(),
        south.referent(),
        "shared arches must retain one semantic referent"
    );
    assert_ne!(
        north.occurrence(),
        south.occurrence(),
        "north and south appearances must remain distinct"
    );

    let pinned = Pinned::new(north_locator, *north.referent(), basilica.revision());
    let pinned_document = pinned.to_string();
    let decoded_pin = pinned_document
        .parse::<Pinned<BasilicaSpace, BasilicaView, addressable_reference::FeatureId>>()
        .expect("canonical pin must parse");
    assert_eq!(
        decoded_pin, pinned,
        "pinned locator serialization must round-trip"
    );
    assert!(
        matches!(basilica.resolve_pinned(&pinned), Resolution::Resolved(_)),
        "an unchanged pin must resolve normally"
    );

    let loaded_arches = Query::many(basilica.root_locator())
        .traverse(BasilicaAxis::Descendants)
        .filter(BasilicaPredicate::Kind(FeatureKind::Arch))
        .filter(BasilicaPredicate::LoadAtLeast(100))
        .deduplicate(Deduplication::Occurrence)
        .order(ResultOrdering::Stable)
        .cycles(CyclePolicy::SkipVisited(VisitIdentity::Occurrence))
        .budget(TraversalBudget::new(8, 128, 32, 512));
    let results = basilica
        .query_many(&loaded_arches)
        .expect("typed assembly query succeeds");
    assert_eq!(
        results.items().len(),
        2,
        "occurrence deduplication must preserve both arches"
    );

    let dependency_query = Query::many(basilica.root_locator())
        .traverse(BasilicaAxis::ToView(BasilicaView::Dependency))
        .traverse(BasilicaAxis::Descendants)
        .deduplicate(Deduplication::Occurrence)
        .order(ResultOrdering::Stable)
        .cycles(CyclePolicy::SkipVisited(VisitIdentity::Occurrence))
        .budget(TraversalBudget::new(8, 128, 32, 512));
    assert_eq!(
        basilica
            .query_many(&dependency_query)
            .expect("cycle policy terminates")
            .items()
            .len(),
        3,
        "dependency traversal must visit nave, arch, and vault exactly once"
    );

    let mut watch = basilica
        .watch(loaded_arches.clone())
        .expect("occurrence watch starts");
    let mut replayed = watch.snapshot().clone();
    let arch = results.items()[0].clone();
    let endpoint = Endpoint::new(arch.clone(), Load);
    let explained = basilica
        .read_load(&endpoint)
        .expect("typed load can be explained");
    let handle = basilica
        .resolved_handle(&arch)
        .expect("runtime handle resolves");
    assert_eq!(
        *explained.value(),
        120,
        "the authored load must win over the default"
    );

    let edit = SetLoad::new(
        endpoint,
        80,
        Guard::new(
            *arch.referent(),
            basilica.revision(),
            *explained.value(),
            EditCapability::SetLoad,
        ),
    );
    let preview = basilica
        .transact(Transaction::dry_run(basilica.revision(), [edit.clone()]))
        .expect("dry run validates");
    assert_eq!(
        preview.mode(),
        TransactionMode::DryRun,
        "preview must not be reported as an apply"
    );
    let applied = basilica
        .transact(Transaction::apply(basilica.revision(), [edit]))
        .expect("guarded transaction applies");
    assert_eq!(applied.changes().len(), 1, "one referent value must change");
    assert_eq!(
        applied.undo().len(),
        1,
        "the applied change must carry undo information"
    );

    let delta = watch.poll(&basilica).expect("watch advances coherently");
    replayed.apply(&delta).expect("delta replays");
    assert_eq!(
        &replayed,
        watch.snapshot(),
        "delta replay must equal full recomputation"
    );
    assert!(
        watch.snapshot().entries().is_empty(),
        "both occurrences must leave the load-filtered query"
    );

    let correspondence = basilica.correspond_to_catalog(*arch.referent(), &catalog);
    assert!(
        correspondence.is_ambiguous(),
        "one shared feature must map to two catalog results"
    );
    assert_eq!(
        correspondence.targets().len(),
        2,
        "both result occurrences must retain correspondence evidence"
    );

    let dynamic_endpoint = DynamicEndpoint {
        owner: DynamicLocator {
            space: basilica.id().get(),
            view: "assembly".into(),
            address: "/basilica/nave/north_arch".into(),
        },
        facet: "load".into(),
    };
    let dynamic_space = basilica.id().get();
    let current_revision = basilica.revision().get();
    let dynamic_report = {
        let mut tool = ReferenceTool::new(&mut basilica);
        assert_eq!(
            tool.schema().name,
            "addressable.reference.basilica/v1",
            "dynamic calls must be governed by the declared schema"
        );
        assert_eq!(
            tool.read(&dynamic_endpoint)
                .expect("dynamic explanation succeeds")
                .value,
            DynamicValue::Integer(80),
            "dynamic reads must agree with the typed effective value"
        );
        tool.transact(DynamicTransaction {
            selection_space: dynamic_space,
            selection_revision: current_revision,
            mode: TransactionMode::Apply,
            operations: vec![DynamicSet {
                endpoint: dynamic_endpoint,
                value: DynamicValue::Integer(120),
                guard: DynamicGuard {
                    expected_referent: arch.referent().get(),
                    expected_space: dynamic_space,
                    expected_revision: current_revision,
                    expected_value: DynamicValue::Integer(80),
                },
            }],
        })
        .expect("dynamic operation delegates to typed transaction")
    };
    assert_eq!(
        dynamic_report.changes[0].current, 120,
        "dynamic set must delegate to the typed transaction"
    );
    assert_eq!(
        dynamic_report.undo.len(),
        1,
        "dynamic callers must receive typed undo information"
    );

    println!(
        "Addressable slice complete: referent {}, occurrences 2, edge ids {}, slot {}, revisions 0→{}, catalog targets {}, dynamic revision {}",
        arch.referent().get(),
        basilica.edge_ids().len(),
        handle.handle().get(),
        applied.revision_after().get(),
        correspondence.targets().len(),
        dynamic_report.revision_after,
    );
}
