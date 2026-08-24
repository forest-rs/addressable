// Copyright 2026 the Addressable Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable complete vertical slice for Addressable.
//!
//! Each function is one chapter in the lifecycle. The assertions keep the tour
//! useful as conformance coverage, while the output explains the transitions a
//! caller would follow.

use addressable::{
    AbsoluteAddress, CyclePolicy, Deduplication, Endpoint, Guard, Locator, Pinned, Query,
    RelativeAddress, Resolution, ResultOrdering, SpaceId, Transaction, TransactionMode,
    TraversalBudget, VisitIdentity,
};
use addressable_reference::{
    Basilica, BasilicaAxis, BasilicaLocator, BasilicaPredicate, BasilicaQuery, BasilicaSpace,
    BasilicaView, Catalog, CatalogSpace, EditCapability, FeatureId, FeatureKind, Load, SetLoad,
};
use addressable_tooling::{
    DynamicEndpoint, DynamicGuard, DynamicLocator, DynamicSet, DynamicTransaction, DynamicValue,
    ReferenceTool,
};

fn main() {
    let mut basilica = Basilica::new(SpaceId::<BasilicaSpace>::new(1));
    let catalog = Catalog::new(SpaceId::<CatalogSpace>::new(2));

    let arch = addresses_and_identity(&basilica);
    let loaded_arches = queries_and_views(&basilica);
    let (slot, applied_revision) = explain_edit_and_watch(&mut basilica, &loaded_arches);
    let catalog_targets = correspondence(&basilica, &catalog, arch);
    let dynamic_revision = dynamic_tooling(&mut basilica);

    println!(
        "\nTour complete: referent {}, slot {slot}, typed revision {applied_revision}, catalog targets {catalog_targets}, dynamic revision {dynamic_revision}",
        arch.get(),
    );
}

fn addresses_and_identity(basilica: &Basilica) -> FeatureId {
    println!("1. Resolve structured addresses and preserve identity");

    let north_locator = Locator::exact(
        basilica.id(),
        BasilicaView::Assembly,
        AbsoluteAddress::parse("/basilica/nave/north_arch").expect("valid exact address"),
    );
    let south_locator = Locator::relative(
        basilica.id(),
        BasilicaView::Assembly,
        AbsoluteAddress::parse("/basilica/nave").expect("valid relative base"),
        RelativeAddress::parse("south_arch").expect("valid relative path"),
    );
    let Resolution::Resolved(north) = basilica.resolve(&north_locator) else {
        panic!("north arch must resolve");
    };
    let Resolution::Resolved(south) = basilica.resolve(&south_locator) else {
        panic!("south arch must resolve");
    };

    let relative_document = south_locator.to_string();
    let decoded_relative = relative_document
        .parse::<BasilicaLocator>()
        .expect("canonical relative locator must parse");
    assert_eq!(
        decoded_relative, south_locator,
        "relative locator serialization must round-trip",
    );
    assert_eq!(
        north.referent(),
        south.referent(),
        "shared arches must retain one semantic referent",
    );
    assert_ne!(
        north.occurrence(),
        south.occurrence(),
        "north and south appearances must remain distinct",
    );

    let pinned = Pinned::new(north_locator, *north.referent(), basilica.revision());
    let pinned_document = pinned.to_string();
    let decoded_pin = pinned_document
        .parse::<Pinned<BasilicaSpace, BasilicaView, FeatureId>>()
        .expect("canonical pin must parse");
    assert_eq!(
        decoded_pin, pinned,
        "pinned locator serialization must round-trip",
    );
    assert!(
        matches!(basilica.resolve_pinned(&pinned), Resolution::Resolved(_)),
        "an unchanged pin must resolve normally",
    );
    let edge_count = basilica.edge_ids().len();
    assert!(
        edge_count > 0,
        "relationship occurrences must have their own identities",
    );

    println!(
        "   north and south share referent {} at revision {}; {edge_count} relationships retain separate identities",
        north.referent().get(),
        north.revision().get(),
    );
    *north.referent()
}

fn queries_and_views(basilica: &Basilica) -> BasilicaQuery {
    println!("2. Query with explicit cardinality, identity, order, cycles, and budget");

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
        "occurrence deduplication must preserve both arches",
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
        "dependency traversal must visit nave, arch, and vault exactly once",
    );

    println!(
        "   found {} arch occurrences after visiting {} nodes",
        results.items().len(),
        results.stats().visited_nodes,
    );
    loaded_arches
}

fn explain_edit_and_watch(basilica: &mut Basilica, query: &BasilicaQuery) -> (u32, u64) {
    println!("3. Explain a typed endpoint, guard an edit, and replay its live delta");

    let mut watch = basilica
        .watch(query.clone())
        .expect("occurrence watch starts");
    let mut replayed = watch.snapshot().clone();
    let arch = basilica.query_many(query).expect("query succeeds").items()[0].clone();
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
        "the authored load must win over the default",
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
        "preview must not be reported as an apply",
    );
    let applied = basilica
        .transact(Transaction::apply(basilica.revision(), [edit]))
        .expect("guarded transaction applies");
    assert_eq!(applied.changes().len(), 1, "one referent value must change");
    assert_eq!(
        applied.undo().len(),
        1,
        "the applied change must carry undo information",
    );

    let delta = watch.poll(basilica).expect("watch advances coherently");
    replayed.apply(&delta).expect("delta replays");
    assert_eq!(
        &replayed,
        watch.snapshot(),
        "delta replay must equal full recomputation",
    );
    assert!(
        watch.snapshot().entries().is_empty(),
        "both occurrences must leave the load-filtered query",
    );

    println!(
        "   load 120 → 80 at revision {}; replay removed both watched occurrences",
        applied.revision_after().get(),
    );
    (handle.handle().get(), applied.revision_after().get())
}

fn correspondence(basilica: &Basilica, catalog: &Catalog, arch: FeatureId) -> usize {
    println!("4. Map a referent into another object space without losing evidence");

    let correspondence = basilica.correspond_to_catalog(arch, catalog);
    assert!(
        correspondence.is_ambiguous(),
        "one shared feature must map to two catalog results",
    );
    assert_eq!(
        correspondence.targets().len(),
        2,
        "both result occurrences must retain correspondence evidence",
    );

    println!(
        "   referent {} maps to {} catalog occurrences with separate provenance",
        arch.get(),
        correspondence.targets().len(),
    );
    correspondence.targets().len()
}

fn dynamic_tooling(basilica: &mut Basilica) -> u64 {
    println!("5. Repeat the guarded edit through the schema-backed dynamic boundary");

    let dynamic_endpoint = DynamicEndpoint {
        owner: DynamicLocator {
            space: basilica.id().get(),
            view: "assembly".into(),
            address: "/basilica/nave/north_arch".into(),
        },
        facet: "load".into(),
    };
    let mut tool = ReferenceTool::new(basilica);
    assert_eq!(
        tool.schema().name,
        "addressable.reference.basilica/v1",
        "dynamic calls must be governed by the declared schema",
    );
    let observed = tool
        .read(&dynamic_endpoint)
        .expect("dynamic explanation succeeds");
    assert_eq!(
        observed.value,
        DynamicValue::Integer(80),
        "dynamic reads must agree with the typed effective value",
    );
    let report = tool
        .transact(DynamicTransaction {
            selection_space: observed.space,
            selection_revision: observed.revision,
            mode: TransactionMode::Apply,
            operations: vec![DynamicSet {
                endpoint: dynamic_endpoint,
                value: DynamicValue::Integer(120),
                guard: DynamicGuard {
                    expected_referent: observed.subject,
                    expected_space: observed.space,
                    expected_revision: observed.revision,
                    expected_value: observed.value,
                },
            }],
        })
        .expect("dynamic operation delegates to typed transaction");
    assert_eq!(
        report.changes[0].current, 120,
        "dynamic set must delegate to the typed transaction",
    );
    assert_eq!(
        report.undo.len(),
        1,
        "dynamic callers must receive typed undo information",
    );

    println!(
        "   dynamic read supplied its own guard context; load restored to 120 at revision {}",
        report.revision_after,
    );
    report.revision_after
}
