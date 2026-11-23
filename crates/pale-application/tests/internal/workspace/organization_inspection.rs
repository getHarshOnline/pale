//! Actual portable admission, canonical record parity and source custody.

use super::*;
use crate::workspace::{
    compile_portable_workspace, CapturedWorkspaceDocument, CapturedWorkspaceSource,
};
use crate::{organization_fixtures as org, responsibility_fixtures as responsibility};

pub(super) fn local_responsibility(
    projection_count: usize,
    directional: bool,
) -> ResponsibilityDerivationV1 {
    let mut source =
        responsibility::responsibility_derivation("alpha", projection_count, directional);
    // This local declaration claims no publisher release or adopted import.
    // Omission preserves its reason/attribution; it is not admission evidence.
    source.publisher_module = responsibility::omitted_answer("local-publisher");
    source.adopter_module = responsibility::omitted_answer("local-adopter");
    source
}

pub(super) fn detail_responsibility() -> ResponsibilityDerivationV1 {
    let mut source = local_responsibility(2, true);
    // As in the owning composition fixture, these are declared body-free
    // occurrence references, not claims of admitted runtime occurrence bodies.
    source.genealogy_links.push(ResponsibilityGenealogyLinkV1 {
        link_id: responsibility::fixture_id("genealogy.inspection"),
        responsibilities: vec![source.derivation_id.clone()],
        projections: source
            .native_projections
            .iter()
            .map(|value| value.projection_id.clone())
            .collect(),
        occurrences: vec![
            responsibility::fixture_id("occurrence.request"),
            responsibility::fixture_id("occurrence.acceptance"),
        ],
    });
    source
}

pub(super) fn compile(
    organization: Option<&OrganizationalTranslationV1>,
    responsibility: Option<&ResponsibilityDerivationV1>,
) -> PortableWorkspaceUniverse {
    compile_result(organization, responsibility)
        .expect("real public portable compilation must admit the source fixture")
}

fn compile_result(
    organization: Option<&OrganizationalTranslationV1>,
    responsibility: Option<&ResponsibilityDerivationV1>,
) -> Result<PortableWorkspaceUniverse, crate::workspace::PortableWorkspaceFailure> {
    let documents = fixture_documents(organization, responsibility)
        .into_iter()
        .map(|(label, bytes)| CapturedWorkspaceDocument::new(label, bytes).unwrap())
        .collect();
    compile_portable_workspace(
        CapturedWorkspaceSource::new(
            CapturedWorkspaceSource::admit_root_manifest("pale-workspace.toml".into()).unwrap(),
            documents,
        )
        .unwrap(),
    )
}

fn fixture_documents(
    organization: Option<&OrganizationalTranslationV1>,
    responsibility: Option<&ResponsibilityDerivationV1>,
) -> Vec<(String, Vec<u8>)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let manifest_bytes = std::fs::read_to_string(root.join("pale-workspace.toml")).unwrap();
    let mut manifest: toml::Value = toml::from_str(&manifest_bytes).unwrap();
    let mut documents = Vec::new();
    for (family, label, body) in [
        (
            "organizational_translations",
            "organization.toml",
            organization.map(|v| toml::to_string(v).unwrap()),
        ),
        (
            "responsibility_derivations",
            "responsibility.toml",
            responsibility.map(|v| toml::to_string(v).unwrap()),
        ),
    ] {
        if let Some(body) = body {
            manifest["workspace"][family] =
                toml::Value::Array(vec![toml::Value::String(label.into())]);
            documents.push((label.into(), body.into_bytes()));
        }
    }
    documents.push((
        "pale-workspace.toml".into(),
        toml::to_string(&manifest).unwrap().into_bytes(),
    ));
    // Neither source adds a module release; the existing owning sealed empty
    // release/universe locks remain exact. No publisher evidence is injected.
    for label in ["resolution.lock.toml", "universe.lock.toml"] {
        documents.push((label.into(), std::fs::read(root.join(label)).unwrap()));
    }
    documents
}

/// Explicit development source export, never counted as an ignored behavioral proof.
/// Run this named test with --ignored and `PALE_ORGANIZATION_FIXTURE_DIR` pointing
/// to a fresh empty directory, then compare every emitted byte before SDK adoption.
#[test]
#[ignore = "development-only SDK fixture export; not behavioral coverage"]
fn export_sdk_organization_fixture() {
    let directory = std::path::PathBuf::from(
        std::env::var_os("PALE_ORGANIZATION_FIXTURE_DIR").expect("explicit fixture directory"),
    );
    assert!(directory.is_absolute());
    assert!(
        directory.is_dir()
            && !std::fs::symlink_metadata(&directory)
                .unwrap()
                .file_type()
                .is_symlink()
    );
    assert_eq!(
        std::fs::read_dir(&directory).unwrap().count(),
        0,
        "do not overwrite evidence"
    );
    let organization = org::organizational_translation();
    let responsibility = detail_responsibility();
    let owner = compile(Some(&organization), Some(&responsibility));
    assert!(rows(&owner, 1).iter().any(|entry| matches!(
        entry.record(),
        OrganizationInspectionRecord::DerivedRelation(_)
    )));
    for (label, bytes) in fixture_documents(Some(&organization), Some(&responsibility)) {
        use std::io::Write as _;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(label))
            .unwrap();
        output.write_all(&bytes).unwrap();
    }
}

pub(super) fn rows(
    owner: &PortableWorkspaceUniverse,
    limit: u16,
) -> Vec<OrganizationInspectionEntry<'_>> {
    let mut after = None;
    let mut rows = Vec::new();
    for _ in 0..10_000 {
        let page = owner
            .inspect_organization(after.as_ref(), limit, limit)
            .unwrap();
        assert!(page.inspected() <= limit);
        assert_eq!(usize::from(page.inspected()), page.entries().len());
        assert!(page.entries().len() <= usize::from(limit));
        rows.extend_from_slice(page.entries());
        after = page.continuation().cloned();
        if after.is_none() {
            return rows;
        }
    }
    panic!("bounded fixture enumeration did not terminate")
}

fn value(row: OrganizationInspectionRecord<'_>) -> serde_json::Value {
    use OrganizationInspectionRecord as R;
    match row {
        R::Scope(v) => serde_json::to_value(v),
        R::Organization(v) => serde_json::to_value(v),
        R::Group(v) => serde_json::to_value(v),
        R::DisciplineTree(v) => serde_json::to_value(v),
        R::Role(v) => serde_json::to_value(v),
        R::OccupantBinding(v) => serde_json::to_value(v),
        R::ProviderBinding(v) => serde_json::to_value(v),
        R::CommunicationActor(v) => serde_json::to_value(v),
        R::Leadership(v) => serde_json::to_value(v),
        R::Relation(v) => serde_json::to_value(v),
        R::Object(v) => serde_json::to_value(v),
        R::PopulationRule(v) => serde_json::to_value(v),
        R::GroupLens(v) => serde_json::to_value(v),
        R::Service(v) => serde_json::to_value(v),
        R::NativeProjection(v) => serde_json::to_value(v),
        R::Inspection(v) => serde_json::to_value(v),
        R::DerivedRelation(v) => serde_json::to_value(v),
        R::Responsibility(v) => serde_json::to_value(v),
    }
    .unwrap()
}

#[test]
fn all_eighteen_families_match_canonical_owner_through_tiny_public_compiler_pages() {
    let source = org::organizational_translation();
    let responsibility = local_responsibility(1, true);
    let owner = compile(Some(&source), Some(&responsibility));
    let canonical = crate::composition::lower_organizational_translation(source).unwrap();
    let responsibility =
        crate::composition::lower_responsibility_derivations(vec![responsibility]).unwrap();
    let canonical_json = serde_json::to_value(&canonical.source).unwrap();
    let families = [
        "scopes",
        "organizations",
        "groups",
        "discipline_trees",
        "roles",
        "occupant_bindings",
        "provider_bindings",
        "communication_actor_bindings",
        "leadership",
        "relations",
        "objects",
        "population_rules",
        "group_lenses",
        "services",
        "native_projections",
    ];
    let mut expected = Vec::new();
    for family in families {
        let entries = canonical_json[family].as_array().unwrap();
        assert!(!entries.is_empty(), "owning fixture must exercise {family}");
        expected.extend(entries.iter().cloned());
    }
    assert!(canonical.source.inspection.is_some());
    assert!(!canonical.derived_relations.is_empty());
    assert!(!responsibility.derivations.is_empty());
    expected.push(canonical_json["inspection"].clone());
    expected.extend(
        canonical
            .derived_relations
            .iter()
            .map(|v| serde_json::to_value(v).unwrap()),
    );
    expected.extend(
        responsibility
            .derivations
            .iter()
            .map(|v| serde_json::to_value(v).unwrap()),
    );
    let tiny = rows(&owner, 1);
    assert_eq!(
        tiny.iter().map(|v| value(v.record())).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        tiny.iter()
            .map(|v| v.position.family)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        18
    );
    assert_eq!(
        rows(&owner, 256)
            .iter()
            .map(|v| value(v.record()))
            .collect::<Vec<_>>(),
        expected
    );
    let page = owner.inspect_organization(None, 1, 1).unwrap();
    assert!(page.organization_selected());
    assert!(std::ptr::eq(page.compilation(), &owner));

    let service_refs = tiny
        .iter()
        .filter_map(|entry| match entry.record() {
            OrganizationInspectionRecord::Service(service) => Some(service.contract_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let OrganizationInspectionRecord::Responsibility(last) = tiny.last().unwrap().record() else {
        panic!()
    };
    let AttributableAnswerV1::Supplied {
        value: ResponsibilityPlacementV1::DirectionalService { service_contract },
        ..
    } = &last.placement
    else {
        panic!()
    };
    assert!(service_refs.contains(service_contract));
    assert_ne!(
        service_contract.provider_scope,
        service_contract.consumer_scope
    );
}

#[test]
fn absent_selected_empty_and_limit_terminal_successor_are_distinct() {
    let absent = compile(None, None);
    let page = absent.inspect_organization(None, 1, 1).unwrap();
    assert!(!page.organization_selected());
    assert_eq!(page.inspected(), 0);
    assert!(page.continuation().is_none());
    // Mechanical empty source retains the owning schema/version fields only.
    let mut empty = serde_json::to_value(org::organizational_translation()).unwrap();
    for (key, value) in empty.as_object_mut().unwrap() {
        if value.is_array() {
            *value = serde_json::json!([]);
        }
        if key == "inspection" {
            *value = serde_json::Value::Null;
        }
    }
    let empty: OrganizationalTranslationV1 = serde_json::from_value(empty).unwrap();
    let selected = compile(Some(&empty), None);
    let page = selected.inspect_organization(None, 1, 1).unwrap();
    assert!(page.organization_selected());
    assert!(page.entries().is_empty());
    assert!(page.continuation().is_none());
    let responsibility = local_responsibility(0, false);
    let one = compile(None, Some(&responsibility));
    let page = one.inspect_organization(None, 1, 1).unwrap();
    assert_eq!(page.entries().len(), 1);
    let terminal = one.inspect_organization(page.continuation(), 1, 1).unwrap();
    assert!(terminal.entries().is_empty());
    assert_eq!(terminal.inspected(), 0);
    assert!(terminal.continuation().is_none());
}

#[test]
fn raw_limits_exact_owner_and_private_position_fail_separately() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    let other = compile(Some(&source), None);
    assert_eq!(owner.semantic_ir_digest(), other.semantic_ir_digest());
    let page = owner.inspect_organization(None, 2, 1).unwrap();
    for (inspected, returned) in [(0, 1), (1, 0), (257, 1), (1, 257)] {
        assert!(matches!(
            owner.inspect_organization(page.continuation(), inspected, returned),
            Err(OrganizationInspectionError::InvalidLimit)
        ));
    }
    assert!(matches!(
        other.inspect_organization(page.continuation(), 2, 1),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    assert!(matches!(
        owner.inspect_organization(page.continuation(), 1, 1),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    let mut invalid = page.continuation().unwrap().clone();
    invalid.position.index = usize::MAX;
    assert!(matches!(
        owner.inspect_organization(Some(&invalid), 2, 1),
        Err(OrganizationInspectionError::InvalidPosition)
    ));
    assert_eq!(
        owner
            .inspect_organization(page.continuation(), 2, 1)
            .unwrap()
            .entries()
            .len(),
        1
    );
}

#[test]
fn parallel_and_derived_edges_keep_full_identity_and_negative_native_states() {
    let mut source = org::organizational_translation();
    let mut parallel = source.relations[0].clone();
    parallel.relation_id = "relation.parallel".parse().unwrap();
    source.relations.push(parallel.clone());
    let mut later = source.occupant_bindings[0].clone();
    later.interval = EffectiveIntervalV1 {
        start: 11,
        end_exclusive: Some(20),
    };
    source.occupant_bindings.push(later.clone());
    source.leadership[0].participants = vec![
        org::role("alpha", "accountable-a"),
        org::role("alpha", "accountable-b"),
    ];
    source.leadership[0].affected_scopes =
        vec![org::scope("alpha", "scope"), org::scope("beta", "scope")];
    let owner = compile(Some(&source), None);
    let entries = rows(&owner, 2);
    assert!(entries.iter().any(
        |e| matches!(e.record(), OrganizationInspectionRecord::Relation(v) if v == &parallel)
    ));
    assert!(entries.iter().any(
        |e| matches!(e.record(), OrganizationInspectionRecord::OccupantBinding(v) if v == &later)
    ));
    let edges = entries
        .iter()
        .filter_map(|e| match e.record() {
            OrganizationInspectionRecord::DerivedRelation(v)
                if matches!(
                    v.source,
                    OrganizationalDerivedRelationSourceV1::LeadershipParticipation { .. }
                ) =>
            {
                Some(v)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(edges.len(), 4);
    assert!(edges.iter().all(|v| v.source == edges[0].source));
    assert_eq!(
        edges
            .iter()
            .map(|v| (v.class, &v.from, &v.to, &v.interval))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    assert!(entries.iter().any(|e| matches!(e.record(), OrganizationInspectionRecord::NativeProjection(v) if v.state == EffectiveFactStateV1::Stale && v.native_refs.is_empty())));
    assert!(entries.iter().any(|e| matches!(e.record(), OrganizationInspectionRecord::NativeProjection(v) if v.native_refs.len() == 2)));
    let derived = entries
        .iter()
        .find(|e| matches!(e.record(), OrganizationInspectionRecord::DerivedRelation(_)))
        .unwrap();
    assert!(matches!(
        derived.sources(None, 1, 1).unwrap(),
        OrganizationSources::UnmappedRecord
    ));
}

#[test]
fn sources_reuse_original_indices_and_reject_other_record_continuations() {
    let source = org::organizational_translation();
    let original = compile(Some(&source), None);
    let mut reversed = source.clone();
    reversed.roles.reverse();
    let owner = compile(Some(&reversed), None);
    assert_eq!(original.semantic_ir_digest(), owner.semantic_ir_digest());
    let entries = rows(&owner, 1);
    let role = entries.iter().find(|e| matches!(e.record(), OrganizationInspectionRecord::Role(v) if v.role == source.roles[0].role)).unwrap();
    let mut after = None;
    let mut occurrences = Vec::new();
    for _ in 0..10_000 {
        let OrganizationSources::Named { page, continuation } =
            role.sources(after.as_ref(), 1, 1).unwrap()
        else {
            panic!()
        };
        occurrences.extend(page.occurrences().iter().copied());
        if let Some(cursor) = &continuation {
            let other = entries
                .iter()
                .find(|e| matches!(e.record(), OrganizationInspectionRecord::Scope(_)))
                .unwrap();
            assert!(matches!(
                other.sources(Some(cursor), 1, 1),
                Err(OrganizationInspectionError::ContinuationMismatch)
            ));
            assert!(matches!(
                role.sources(Some(cursor), 2, 1),
                Err(OrganizationInspectionError::ContinuationMismatch)
            ));
        }
        after = continuation;
        if after.is_none() {
            break;
        }
    }
    assert!(after.is_none());
    let definition = occurrences
        .iter()
        .find(|v| v.role() == crate::workspace::AuthoringOccurrenceRole::Definition)
        .unwrap();
    assert_eq!(definition.source_label(), "organization.toml");
    assert_eq!(
        definition.path()[1],
        crate::workspace::AuthoringInstancePathSegment::Index(reversed.roles.len() - 1)
    );
    assert!(occurrences.len() > 1);
    let binding = entries
        .iter()
        .find(|e| matches!(e.record(), OrganizationInspectionRecord::OccupantBinding(_)))
        .unwrap();
    assert!(matches!(
        binding.sources(None, 1, 1).unwrap(),
        OrganizationSources::UnmappedRecord
    ));
}

#[test]
fn invalid_service_reference_is_rejected_by_public_compilation_before_inspection() {
    let mut source = org::organizational_translation();
    let responsibility = local_responsibility(1, true);
    let AttributableAnswerV1::Supplied {
        value: ResponsibilityPlacementV1::DirectionalService { service_contract },
        ..
    } = &responsibility.placement
    else {
        panic!()
    };
    compile(Some(&source), Some(&responsibility));
    source
        .services
        .retain(|service| service.contract_ref() != *service_contract);
    // The unchanged responsibility remains internally valid; its previously
    // admitted exact service is now missing from the selected organizational IR.
    let error = compile_result(Some(&source), Some(&responsibility))
        .err()
        .unwrap();
    assert!(
        matches!(error.workspace_error(), crate::workspace::WorkspaceError::MissingDirectionalService { derivation, contract }
        if derivation == &responsibility.derivation_id && contract.as_ref() == service_contract)
    );
}

#[test]
fn supplied_unadopted_publisher_is_rejected_before_inspection() {
    let source = org::organizational_translation();
    let responsibility = responsibility::responsibility_derivation("alpha", 1, true);
    let error = compile_result(Some(&source), Some(&responsibility))
        .err()
        .unwrap();
    assert!(
        matches!(error.workspace_error(), crate::workspace::WorkspaceError::NamespaceResolution(message)
        if message.contains("responsibility.alpha")
            && message.contains("publisher.alpha")
            && message.contains("release.one")
            && message.contains("responsibility.surface"))
    );
}

fn source_rows(
    entry: &OrganizationInspectionEntry<'_>,
    mut after: Option<OrganizationSourceCursor>,
) -> Vec<(
    String,
    Vec<crate::workspace::AuthoringInstancePathSegment>,
    crate::workspace::AuthoringOccurrenceRole,
)> {
    let mut result = Vec::new();
    for _ in 0..10_000 {
        let OrganizationSources::Named { page, continuation } =
            entry.sources(after.as_ref(), 1, 1).unwrap()
        else {
            panic!("selected role has a named source identity")
        };
        assert!(page.inspected() <= 1);
        result.extend(page.occurrences().iter().map(|row| {
            (
                row.source_label().to_owned(),
                row.path().to_vec(),
                row.role(),
            )
        }));
        after = continuation;
        if after.is_none() {
            return result;
        }
    }
    panic!("bounded source fixture did not terminate")
}

#[test]
fn moved_complete_owner_preserves_record_and_both_continuations() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), Some(&local_responsibility(1, true)));
    let expected = rows(&owner, 1)
        .iter()
        .map(|entry| value(entry.record()))
        .collect::<Vec<_>>();
    let (record, source_after, expected_source) = {
        let entries = rows(&owner, 1);
        let role = entries
            .iter()
            .find(|entry| matches!(entry.record(), OrganizationInspectionRecord::Role(_)))
            .unwrap();
        let OrganizationSources::Named { continuation, .. } = role.sources(None, 1, 1).unwrap()
        else {
            panic!()
        };
        assert!(continuation.is_some());
        let expected_source = source_rows(role, continuation.clone());
        assert!(!expected_source.is_empty());
        (role.retain(), continuation, expected_source)
    };
    let expected_record = value(owner.resolve_organization_record(&record).unwrap().record());
    let mut after = owner
        .inspect_organization(None, 1, 1)
        .unwrap()
        .continuation()
        .cloned();
    assert!(after.is_some());
    // Move the complete object, not just a reference or an Arc clone. All source
    // maps move with it; only the immutable admitted-owner allocation is shared.
    let moved = Box::new(owner);
    assert_eq!(
        value(moved.resolve_organization_record(&record).unwrap().record()),
        expected_record
    );
    assert_eq!(
        source_rows(
            &moved.resolve_organization_record(&record).unwrap(),
            source_after
        ),
        expected_source
    );
    let mut remaining = Vec::new();
    for _ in 0..10_000 {
        let page = moved.inspect_organization(after.as_ref(), 1, 1).unwrap();
        remaining.extend(page.entries().iter().map(|entry| value(entry.record())));
        after = page.continuation().cloned();
        if after.is_none() {
            break;
        }
    }
    assert!(after.is_none());
    assert_eq!(remaining, expected[1..]);
}

#[test]
fn retained_record_and_source_cursors_reject_equal_distinct_owner_and_other_record() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    let other = compile(Some(&source), None);
    assert_eq!(owner.semantic_ir_digest(), other.semantic_ir_digest());
    assert_eq!(
        owner.principal_universe_digest(),
        other.principal_universe_digest()
    );
    let entries = rows(&owner, 1);
    let role = entries
        .iter()
        .find(|entry| matches!(entry.record(), OrganizationInspectionRecord::Role(_)))
        .unwrap();
    let handle = role.retain();
    assert!(matches!(
        other.resolve_organization_record(&handle),
        Err(OrganizationInspectionError::RecordOwnerMismatch)
    ));
    let OrganizationSources::Named { continuation, .. } = role.sources(None, 1, 1).unwrap() else {
        panic!()
    };
    let cursor = continuation.unwrap();
    let mut reordered_source = source.clone();
    reordered_source.roles.reverse();
    let reordered = compile(Some(&reordered_source), None);
    assert_eq!(owner.semantic_ir_digest(), reordered.semantic_ir_digest());
    assert!(matches!(
        reordered.resolve_organization_record(&handle),
        Err(OrganizationInspectionError::RecordOwnerMismatch)
    ));
    let reordered_entries = rows(&reordered, 1);
    let reordered_role = reordered_entries
        .iter()
        .find(|entry| matches!(entry.record(), OrganizationInspectionRecord::Role(_)))
        .unwrap();
    assert_ne!(source_rows(role, None), source_rows(reordered_role, None));
    assert!(matches!(
        reordered_role.sources(Some(&cursor), 1, 1),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    let other_entries = rows(&other, 1);
    let other_role = other_entries
        .iter()
        .find(|entry| matches!(entry.record(), OrganizationInspectionRecord::Role(_)))
        .unwrap();
    assert_eq!(value(role.record()), value(other_role.record()));
    assert!(matches!(
        other_role.sources(Some(&cursor), 1, 1),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    let different_record = entries
        .iter()
        .find(|entry| matches!(entry.record(), OrganizationInspectionRecord::Scope(_)))
        .unwrap();
    assert!(matches!(
        different_record.sources(Some(&cursor), 1, 1),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    let resolved = owner.resolve_organization_record(&handle).unwrap();
    assert!(matches!(
        resolved.sources(Some(&cursor), 1, 2),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    assert_eq!(
        source_rows(&resolved, Some(cursor.clone())),
        source_rows(role, Some(cursor))
    );
}

#[test]
fn each_opaque_token_independently_retains_and_releases_its_semantic_owner() {
    fn retained_until_last_clone<T: Clone>(owner: PortableWorkspaceUniverse, token: T) {
        let weak = Arc::downgrade(owner.organization_owner());
        let last = token.clone();
        drop(owner);
        assert!(weak.upgrade().is_some());
        drop(token);
        assert!(weak.upgrade().is_some());
        drop(last);
        assert!(weak.upgrade().is_none());
    }
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    let cursor = owner
        .inspect_organization(None, 1, 1)
        .unwrap()
        .continuation()
        .unwrap()
        .clone();
    retained_until_last_clone(owner, cursor);

    let owner = compile(Some(&source), None);
    let handle = owner.inspect_organization(None, 1, 1).unwrap().entries()[0].retain();
    retained_until_last_clone(owner, handle);

    let owner = compile(Some(&source), None);
    let cursor = {
        let entries = rows(&owner, 1);
        let role = entries
            .iter()
            .find(|entry| matches!(entry.record(), OrganizationInspectionRecord::Role(_)))
            .unwrap();
        let OrganizationSources::Named { continuation, .. } = role.sources(None, 1, 1).unwrap()
        else {
            panic!()
        };
        continuation.unwrap()
    };
    retained_until_last_clone(owner, cursor);
    // These tokens retain semantic custody, not the dropped outer source maps.
    // Future SDK handles must retain the complete compilation for source access.
}
