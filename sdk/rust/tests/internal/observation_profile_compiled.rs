//! Compiled primary-row wiring across repositories and Package operations.

use super::*;
use pale_sdk::{EffectiveProfileEntry, ObservationProfileMapping, ObservationSubject};

fn parse(bytes: &[u8]) -> toml::Value {
    toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap()
}

fn association_packages() -> Vec<CompilationDocument> {
    let mut documents = Vec::new();
    for (directory, package_id) in [("engine", "example.engine"), ("other", "example.other")] {
        let mut manifest = parse(captured_fixture!("packages/engine/package.toml"));
        let package = &mut manifest["package"];
        package["id"] = toml::Value::String(package_id.into());
        package["sources"]
            .as_array_mut()
            .unwrap()
            .push(parse(b"path = 'operations.toml'\nrole = 'declaration'"));
        package["sources"]
            .as_array_mut()
            .unwrap()
            .push(parse(b"path = 'policy.toml'\nrole = 'declaration'"));
        package["resources"].as_array_mut().unwrap().push(parse(b"id = 'inspection-policy'\nkind = 'capability_policy'\ndefinition_source = 'policy.toml'\nexposure = 'exported'"));
        let mut operations = String::from("[operations]\nschema_version = 1\n");
        for operation in ["inspect", "review", "unbound"] {
            package["sources"].as_array_mut().unwrap().push(parse(
                format!("path = '{operation}.md'\nrole = 'body'").as_bytes(),
            ));
            documents.push(CompilationDocument::new(
                format!("{directory}/{operation}.md"),
                format!("# {operation}\n\nInspect the selected evidence.\n").into_bytes(),
            ));
            package["resources"].as_array_mut().unwrap().push(parse(
                format!("id = '{operation}'\nkind = 'operation'\ndefinition_source = 'operations.toml'\nexposure = 'exported'").as_bytes(),
            ));
            operations.push_str(&format!(
                "\n[[operations.operation]]\nid = '{operation}'\ndescription = 'Inspect source without native effects.'\ndoctrine = '{operation}.md'\n"
            ));
            if operation != "unbound" {
                operations.push_str(
                    "capability_policy = { scope = 'local', resource_id = 'inspection-policy' }\n",
                );
                for (requirement, capability) in
                    [("primary", "inspection"), ("secondary", "comparison")]
                {
                    operations.push_str(&format!(
                        "[[operations.operation.capability_requirements]]\nid = '{requirement}'\ndescription = 'Read selected evidence.'\ncapability = '{capability}'\neffect_class = 'read'\npolicy = {{ kind = 'required_block' }}\nfailure_posture = 'fail_closed'\n"
                    ));
                }
            }
        }
        for (name, bytes) in [
            (
                "package.toml",
                toml::to_string(&manifest).unwrap().into_bytes(),
            ),
            ("operations.toml", operations.into_bytes()),
            ("policy.toml", b"permitted_capabilities = ['inspection', 'comparison']\npermitted_effects = ['read']\nforbidden_capabilities = []\nforbidden_effects = []\n".to_vec()),
            (
                "roles.toml",
                captured_fixture!("packages/engine/roles.toml").to_vec(),
            ),
            (
                "role.md",
                captured_fixture!("packages/engine/role.md").to_vec(),
            ),
        ] {
            documents.push(CompilationDocument::new(
                format!("{directory}/{name}"),
                bytes,
            ));
        }
    }
    documents
}

fn compile_associations(
    api: &PaleApi,
) -> (
    pale_sdk::PortableWorkspaceCompilation,
    pale_sdk::EffectiveProfileCompilation,
) {
    let mut manifest = parse(captured_fixture!("workspace/pale-workspace.toml"));
    let topology = &mut manifest["workspace"]["desired_topology"];
    topology["projects"][0]["package_seeds"]
        .as_array_mut()
        .unwrap()
        .push(toml::Value::String("example.other".into()));
    for (table, identity_field) in [
        ("repositories", "id"),
        ("project_repositories", "repository_id"),
        ("execution_workspace_repositories", "repository_id"),
    ] {
        let mut second = topology[table][0].clone();
        second[identity_field] = toml::Value::String("notes".into());
        if table == "project_repositories" {
            second["materialization_order"] = toml::Value::Integer(1);
        }
        topology[table].as_array_mut().unwrap().push(second);
    }
    let workspace = api
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            [
                (
                    "pale-workspace.toml",
                    toml::to_string(&manifest).unwrap().into_bytes(),
                ),
                (
                    "composition.toml",
                    captured_fixture!("workspace/composition.toml").to_vec(),
                ),
                (
                    "workspace-resolution.lock.toml",
                    captured_fixture!("workspace/workspace-resolution.lock.toml").to_vec(),
                ),
                (
                    "workspace-universe.lock.toml",
                    captured_fixture!("workspace/workspace-universe.lock.toml").to_vec(),
                ),
            ]
            .into_iter()
            .map(|(name, bytes)| PortableWorkspaceDocument::new(name.into(), bytes))
            .collect(),
        ))
        .unwrap();

    let inspection = api
        .inspect_package_distribution(pale_sdk::PackageDistributionRequest::new(
            association_packages(),
        ))
        .unwrap();
    let lock = pale_core::records::DistributionLockFile {
        distribution: pale_core::records::DistributionMeta {
            compiler: inspection.compiler().into(),
            schema_manifest_sha256: pale_core::records::SchemaManifestDigest::parse(
                inspection.schema_manifest_sha256(),
            )
            .unwrap(),
            schema_set_sha256: pale_core::records::SchemaSetDigest::parse(
                inspection.schema_set_sha256(),
            )
            .unwrap(),
        },
        packages: inspection
            .packages()
            .iter()
            .map(|package| {
                parse(
                    format!(
                        "id = '{}'\nversion = '{}'\ncontent_digest = '{}'",
                        package.id(),
                        package.version(),
                        package.content_digest()
                    )
                    .as_bytes(),
                )
                .try_into()
                .unwrap()
            })
            .collect(),
    };
    let mut capabilities =
        String::from_utf8(captured_fixture!("private/capabilities.toml").to_vec()).unwrap();
    capabilities.push_str("\n[semantic]\noperations = [{ selection = 'all_from_package', package_id = 'example.engine' }, { selection = 'all_from_package', package_id = 'example.other' }]\n");
    for package in ["example.engine", "example.other"] {
        for operation in ["inspect", "review"] {
            capabilities.push_str(&format!("\n[[operation_bindings]]\noperation = {{ package_id = '{package}', resource_id = '{operation}' }}\nbinding = {{ selection = {{ kind = 'internal' }} }}\n"));
        }
    }
    let mut machine = parse(captured_fixture!("private/machine-binding.toml"));
    machine["repository_checkouts"]
        .as_array_mut()
        .unwrap()
        .push(parse(
            b"repository_id = 'notes'\ncheckout_root = '/captured/notes'",
        ));
    let profile = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".into(),
                "default".into(),
                "machine".into(),
                OneShotPrivateInputs::new(
                    document("capabilities.toml", capabilities.as_bytes()),
                    document(
                        "machines/machine.toml",
                        toml::to_string(&machine).unwrap().as_bytes(),
                    ),
                    document(
                        "distribution-lock.toml",
                        toml::to_string(&lock).unwrap().as_bytes(),
                    ),
                    Vec::new(),
                ),
                association_packages(),
            ),
        )
        .unwrap();
    (workspace, profile)
}

fn all_profile_rows(profile: &pale_sdk::EffectiveProfileCompilation) -> Vec<EffectiveProfileEntry> {
    [
        EffectiveProfileSection::IntendedUseCase,
        EffectiveProfileSection::EffectiveConfiguration,
        EffectiveProfileSection::CapabilitiesAndOperations,
        EffectiveProfileSection::MachineRealization,
        EffectiveProfileSection::GapsAndExplanations,
    ]
    .into_iter()
    .flat_map(|section| {
        let page = profile.inspect(EffectiveProfilePageRequest::new(section, 0, 256));
        assert!(
            page.next_offset().is_none(),
            "fixture must fit one reference page per section"
        );
        page.entries().to_vec()
    })
    .collect()
}

fn expected_primary(detail: &EffectiveProfileEntryDetail, subject: &ObservationSubject) -> bool {
    match (subject.kind(), detail) {
        (
            ObservationSubjectKind::Repository,
            EffectiveProfileEntryDetail::Repository { repository_id, .. },
        ) => Some(repository_id.as_str()) == subject.repository_id(),
        (
            ObservationSubjectKind::Operation,
            EffectiveProfileEntryDetail::Operation {
                repository_id,
                operation_id,
                ..
            }
            | EffectiveProfileEntryDetail::Finding {
                finding:
                    EffectiveProfileFinding::OperationUnbound {
                        repository_id,
                        operation_id,
                    },
            },
        ) => {
            Some(repository_id.as_str()) == subject.repository_id()
                && Some(operation_id.as_str()) == subject.operation_id()
        }
        (
            ObservationSubjectKind::CapabilityRequirement,
            EffectiveProfileEntryDetail::Capability {
                repository_id,
                operation_id,
                requirement_id,
                capability_id,
                ..
            }
            | EffectiveProfileEntryDetail::Finding {
                finding:
                    EffectiveProfileFinding::Requirement {
                        repository_id,
                        operation_id,
                        requirement_id,
                        capability_id,
                        ..
                    },
            },
        ) => {
            Some(repository_id.as_str()) == subject.repository_id()
                && Some(operation_id.as_str()) == subject.operation_id()
                && Some(requirement_id.as_str()) == subject.requirement_id()
                && Some(capability_id.as_str()) == subject.capability_id()
        }
        (
            ObservationSubjectKind::ExecutionPath,
            EffectiveProfileEntryDetail::ExecutionPath { selection_id, .. },
        ) => Some(selection_id.as_str()) == subject.execution_path_id(),
        _ => false,
    }
}

#[test]
fn compiled_associations_keep_primary_keys_rows_and_handles() {
    let api = PaleApi::new().unwrap();
    let (workspace, profile) = compile_associations(&api);
    let snapshot = api.observe_effective_profile(&profile);
    let facts = ready_observation_page(
        &snapshot,
        ObservationPageRequest::bounded(
            ObservationFilter::all(),
            pale_sdk::ObservationPageLimits::new(u16::MAX, 256, u32::MAX),
            ObservationCancellation::Active,
        ),
    );
    assert!(facts.continuation.is_none());
    let mut subjects = Vec::new();
    for fact in facts.facts {
        if let ObservationFact::Readiness { subject, .. } = fact {
            if subject.kind() != ObservationSubjectKind::Machine && !subjects.contains(&subject) {
                subjects.push(subject);
            }
        }
    }
    subjects.push(ObservationSubject::from(
        &pale_application::observation::ObservationSubject::CompiledGeneration(
            pale_core::records::CompiledGenerationDigest::hash_canonical(
                &"unrelated resident generation",
            )
            .unwrap(),
        ),
    ));
    for (kind, count) in [
        (ObservationSubjectKind::Repository, 2),
        (ObservationSubjectKind::Operation, 12),
        (ObservationSubjectKind::CapabilityRequirement, 16),
        (ObservationSubjectKind::ExecutionPath, 1),
        (ObservationSubjectKind::EffectiveProfile, 1),
        (ObservationSubjectKind::CompiledGeneration, 1),
    ] {
        assert_eq!(
            subjects
                .iter()
                .filter(|subject| subject.kind() == kind)
                .count(),
            count,
            "{kind:?}"
        );
    }
    let original = all_profile_rows(&profile);
    let mut bound_operations = 0;
    let mut unbound_operations = 0;
    for subject in subjects {
        let expected = original
            .iter()
            .filter(|entry| expected_primary(entry.detail(), &subject))
            .cloned()
            .collect::<Vec<_>>();
        let supported = !matches!(
            subject.kind(),
            ObservationSubjectKind::EffectiveProfile | ObservationSubjectKind::CompiledGeneration
        );
        if supported {
            assert!(!expected.is_empty(), "{subject:?}");
        }
        match subject.kind() {
            ObservationSubjectKind::Repository | ObservationSubjectKind::ExecutionPath => {
                assert_eq!(expected.len(), 1)
            }
            ObservationSubjectKind::CapabilityRequirement => assert_eq!(expected.len(), 2),
            ObservationSubjectKind::Operation => match expected.len() {
                1 => bound_operations += 1,
                2 => unbound_operations += 1,
                _ => {
                    panic!("operation must retain its primary row and only its own unbound finding")
                }
            },
            _ => assert!(expected.is_empty()),
        }
        let mut actual = Vec::new();
        let mut after = None;
        let mut visited = 0;
        loop {
            let page = snapshot
                .inspect_profile(&profile, &workspace, &subject, after.as_ref(), 3, 1)
                .unwrap();
            assert_eq!(
                page.mapping,
                if supported {
                    ObservationProfileMapping::PrimaryRows
                } else {
                    ObservationProfileMapping::NoPrimaryRows
                }
            );
            assert!(page.inspected <= 3 && page.entries.len() <= 1);
            visited += usize::from(page.inspected);
            actual.extend(page.entries);
            after = page.continuation;
            if after.is_none() {
                break;
            }
        }
        assert_eq!(visited, original.len());
        assert_eq!(actual, expected, "{subject:?}");
        assert_eq!(
            serde_json::to_vec(&actual).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
        for entry in actual {
            let explanation = profile.explain(entry.explanation_handle()).unwrap();
            assert_eq!(explanation.handle, *entry.explanation_handle());
        }
    }
    assert_eq!((bound_operations, unbound_operations), (8, 4));
}
