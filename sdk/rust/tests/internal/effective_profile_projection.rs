//! Complete row association and bounded paging over the retained projection.

use std::sync::Arc;

use super::super::{EffectiveAuthoringContinuation, EffectiveAuthoringPage};
use super::*;

fn association_projection() -> EffectiveProfileProjection {
    let mut explanations = BTreeMap::new();
    let effective_configuration = explainable_entries(
        "profile-coordinate",
        EffectiveProfileSection::EffectiveConfiguration,
        (0..130)
            .map(|index| EffectiveProfileEntryDetail::Selection {
                selection: EffectiveProfileSelection::SemanticResource {
                    repository_id: "shared".into(),
                    resource_kind: EffectiveProfileResourceKind::Operation,
                    resource_id: format!("package:operations/selected-{index}"),
                },
            })
            .collect(),
        &mut explanations,
    );
    let capabilities_and_operations = explainable_entries(
        "profile-coordinate",
        EffectiveProfileSection::CapabilitiesAndOperations,
        (0..170)
            .map(|index| EffectiveProfileEntryDetail::Operation {
                repository_id: "shared".into(),
                operation_id: if index == 0 {
                    "shared".into()
                } else {
                    format!("package:operations/available-{index}")
                },
                exposure: EffectiveProfileOperationExposure::Unbound,
                native_realization: EffectiveProfileNativeRealization::Unknown,
            })
            .collect(),
        &mut explanations,
    );
    let mut authoring_entries = BTreeMap::new();
    for (section, entries) in [
        (
            EffectiveProfileSection::EffectiveConfiguration,
            &effective_configuration,
        ),
        (
            EffectiveProfileSection::CapabilitiesAndOperations,
            &capabilities_and_operations,
        ),
    ] {
        index_authoring_entries(section, entries, &mut authoring_entries);
    }
    EffectiveProfileProjection {
        coordinates: EffectiveProfileCoordinates {
            workspace_id: "workspace".into(),
            project_id: "project".into(),
            execution_workspace_id: "execution-workspace".into(),
            machine_id: "machine".into(),
            profile_digest: "profile".into(),
            instance_digest: "instance".into(),
            selected_topology_digest: "topology".into(),
            package_semantic_ir_digest: "package-semantics".into(),
            composition_plan_digest: "composition".into(),
            workspace_semantic_ir_digest: "workspace-semantics".into(),
            workspace_admission_evidence_digest: "admission".into(),
        },
        intended_use_case: Vec::new(),
        effective_configuration,
        capabilities_and_operations,
        machine_realization: Vec::new(),
        gaps_and_explanations: Vec::new(),
        explanations,
        authoring_entries,
    }
}

#[test]
fn retained_primary_provenance_reports_target_layout_and_owned_key_bytes() {
    use pale_core::records::{
        CapabilityIdV1, CapabilityRequirementIdV1, LogicalRepositoryIdV1, PackageIdV1,
        PackageResourceIdV1, PackageResourceRefV1,
    };
    let subject = pale_application::observation::ObservationSubject::CapabilityRequirement {
        repository_id: LogicalRepositoryIdV1::parse("engine").unwrap(),
        operation_id: PackageResourceRefV1 {
            package_id: PackageIdV1::parse("example.engine").unwrap(),
            resource_id: PackageResourceIdV1::parse("inspect").unwrap(),
        },
        requirement_id: CapabilityRequirementIdV1::parse("primary").unwrap(),
        capability_id: CapabilityIdV1::parse("inspection").unwrap(),
    };
    let pale_application::observation::ObservationSubject::CapabilityRequirement {
        repository_id,
        operation_id,
        requirement_id,
        capability_id,
    } = &subject
    else {
        unreachable!()
    };
    let owned_key_utf8_bytes = [
        repository_id.as_str(),
        operation_id.package_id.as_str(),
        operation_id.resource_id.as_str(),
        requirement_id.as_str(),
        capability_id.as_str(),
    ]
    .into_iter()
    .map(str::len)
    .sum::<usize>();
    assert_eq!(owned_key_utf8_bytes, 44);
    let entry_bytes = std::mem::size_of::<EffectiveProfileEntry>();
    let row_bytes = std::mem::size_of::<RetainedProfileEntry>();
    let report = serde_json::json!({
        "architecture": std::env::consts::ARCH,
        "pointer_bytes": std::mem::size_of::<usize>(),
        "public_entry_inline_bytes": entry_bytes,
        "private_retained_row_inline_bytes": row_bytes,
        "row_inline_delta_bytes": row_bytes.checked_sub(entry_bytes).unwrap(),
        "optional_primary_subject_inline_bytes": std::mem::size_of::<Option<pale_application::observation::ObservationSubject>>(),
        "representative_requirement_owned_key_utf8_bytes": owned_key_utf8_bytes,
        "representative_owned_key_strings": 5,
        "limits": "UTF-8 lengths exclude String spare capacity, allocator metadata and row payloads. This is not total retained memory."
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

#[test]
fn primary_provenance_preserves_serialized_rows_and_existing_explanation_handles() {
    let detail = EffectiveProfileEntryDetail::MachineRealization {
        machine_id: "machine".into(),
        realization: EffectiveProfileNativeRealization::Unknown,
    };
    let mut plain = ProfileDetails::default();
    plain.push(detail.clone());
    let mut associated = ProfileDetails::default();
    associated.push_primary(
        detail,
        pale_application::observation::ObservationSubject::Machine("machine".into()),
    );
    let mut plain_explanations = BTreeMap::new();
    let mut associated_explanations = BTreeMap::new();
    let plain = explainable_entries(
        "same-coordinates",
        EffectiveProfileSection::MachineRealization,
        plain,
        &mut plain_explanations,
    );
    let associated = explainable_entries(
        "same-coordinates",
        EffectiveProfileSection::MachineRealization,
        associated,
        &mut associated_explanations,
    );
    assert_eq!(plain[0].entry, associated[0].entry);
    assert_eq!(
        serde_json::to_vec(&plain[0].entry).unwrap(),
        serde_json::to_vec(&associated[0].entry).unwrap()
    );
    assert_eq!(plain_explanations, associated_explanations);
    assert!(plain[0].primary_subject.is_none());
    assert!(associated[0].primary_subject.is_some());
}

#[test]
fn association_pages_preserve_every_row_after_the_first_page() {
    let projection = association_projection();
    let expected = projection
        .effective_configuration
        .iter()
        .chain(&projection.capabilities_and_operations)
        .map(|row| row.entry.clone())
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 300);
    let first = projection.authoring(AuthoringIdentityKind::Repository, "shared");
    assert_eq!(first.entries(), &expected[..64]);
    assert!(first.truncated());
    assert_eq!(first.coordinates(), &projection.coordinates());

    for (limit, maximum) in [(0, 64), (1, 1), (64, 64), (256, 256), (u16::MAX, 256)] {
        let mut offset = 0;
        let mut observed = Vec::new();
        loop {
            let (entries, next) = projection.authoring_page(
                AuthoringIdentityKind::Repository,
                "shared",
                offset,
                limit,
            );
            assert!(!entries.is_empty());
            assert!(entries.len() <= maximum);
            observed.extend(entries);
            if let Some(next) = next {
                assert_eq!(next, observed.len());
                assert!(next > offset);
                offset = next;
            } else {
                break;
            }
        }
        assert_eq!(observed, expected, "requested limit {limit}");
        assert!(observed
            .iter()
            .all(|entry| projection.explain(entry.explanation_handle()).is_some()));
    }
}

#[test]
fn association_lookup_keeps_identity_kind_and_reports_terminal_windows() {
    let projection = association_projection();
    let (operation, next) =
        projection.authoring_page(AuthoringIdentityKind::PackageOperation, "shared", 0, 1);
    assert_eq!(
        operation,
        vec![projection.capabilities_and_operations[0].entry.clone()]
    );
    assert!(next.is_none());

    for (kind, value, offset) in [
        (AuthoringIdentityKind::Repository, "missing", 0),
        (AuthoringIdentityKind::Machine, "shared", 0),
        (AuthoringIdentityKind::Repository, "shared", 300),
        (AuthoringIdentityKind::Repository, "shared", usize::MAX),
    ] {
        let (entries, next) = projection.authoring_page(kind, value, offset, 0);
        assert!(entries.is_empty());
        assert!(next.is_none());
    }
}

#[test]
fn continuation_equality_and_drop_preserve_exact_retained_owner() {
    // Obtain a real compiler identity. The synthetic projection below tests
    // allocation ownership only; it does not claim a SourceDocument association.
    let identity = {
        let documents = [
            (
                "pale-workspace.toml",
                include_bytes!("../fixtures/workspace-language/empty-adopter/pale-workspace.toml")
                    .as_slice(),
            ),
            (
                "resolution.lock.toml",
                include_bytes!("../fixtures/workspace-language/empty-adopter/resolution.lock.toml")
                    .as_slice(),
            ),
            (
                "universe.lock.toml",
                include_bytes!("../fixtures/workspace-language/empty-adopter/universe.lock.toml")
                    .as_slice(),
            ),
        ];
        let compilation = crate::PaleApi::new()
            .unwrap()
            .compile_portable_workspace(crate::PortableWorkspaceRequest::new(
                "pale-workspace.toml".into(),
                documents
                    .into_iter()
                    .map(|(label, bytes)| {
                        crate::PortableWorkspaceDocument::new(label.into(), bytes.to_vec())
                    })
                    .collect(),
            ))
            .unwrap();
        compilation
            .navigate_authoring(crate::AuthoringNavigationRequest::new(
                "pale-workspace.toml".into(),
                Vec::new(),
            ))
            .identity()
            .unwrap()
            .clone()
    };
    let retained;
    let page = {
        let projection = Arc::new(association_projection());
        retained = Arc::downgrade(&projection);
        let token = EffectiveAuthoringContinuation {
            projection: Arc::clone(&projection),
            identity,
            limit: 0,
            offset: 64,
        };
        assert_eq!(token, token.clone());
        let other_limit = EffectiveAuthoringContinuation {
            limit: 64,
            ..token.clone()
        };
        assert_ne!(token, other_limit);
        assert_eq!(format!("{token:?}"), format!("{other_limit:?}"));
        assert!(!format!("{token:?}").contains(token.identity.value()));
        assert_ne!(
            token,
            EffectiveAuthoringContinuation {
                offset: 128,
                ..token.clone()
            }
        );
        assert_ne!(
            token,
            EffectiveAuthoringContinuation {
                projection: Arc::new(association_projection()),
                ..token.clone()
            }
        );
        EffectiveAuthoringPage {
            coordinates: projection.coordinates(),
            entries: Vec::new(),
            continuation: Some(token),
        }
    };
    assert!(retained.upgrade().is_some());
    let clone = page.clone();
    drop(page);
    assert!(retained.upgrade().is_some());
    drop(clone);
    assert!(retained.upgrade().is_none());
}
