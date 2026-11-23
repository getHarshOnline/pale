//! Owner and duty joins resolve within the selected source graph.

use super::*;

fn owner(kind: &str, module: &str, resource: &str) -> ProjectOwnerReferenceV1 {
    let module_id = module_id(module);
    let resource_id = id(resource, WorkspaceResourceId::parse);
    match kind {
        "principal" => ProjectOwnerReferenceV1::Principal {
            module_id,
            resource_id,
        },
        "organization" => ProjectOwnerReferenceV1::Organization {
            module_id,
            resource_id,
        },
        _ => panic!("unknown fixture owner kind"),
    }
}

fn fixture() -> CandidateFixture {
    let mut fixture = module_reference_fixture();
    fixture.module.module.resources = vec![
        WorkspaceResourceSelectionV1::Organization {
            resource_id: id("company", WorkspaceResourceId::parse),
            authority_resolver: id("company-authority", WorkspaceResourceId::parse),
        },
        WorkspaceResourceSelectionV1::Principal {
            resource_id: id("person", WorkspaceResourceId::parse),
            authority_resolver: id("person-authority", WorkspaceResourceId::parse),
        },
    ];
    fixture.module = WorkspaceModuleV1::seal(fixture.module.module.clone()).unwrap();
    let consumer = fixture.consumer.as_mut().unwrap();
    consumer.module.imports[0].module_digest = fixture.module.module_digest.clone();
    consumer.module.resources = vec![WorkspaceResourceSelectionV1::Principal {
        resource_id: id("company", WorkspaceResourceId::parse),
        authority_resolver: id("other-authority", WorkspaceResourceId::parse),
    }];
    *consumer = WorkspaceModuleV1::seal(consumer.module.clone()).unwrap();
    reseal_locks(&mut fixture);
    fixture
}

fn write_project(
    root: &Path,
    fixture: &CandidateFixture,
    owner: Option<ProjectOwnerReferenceV1>,
    duties: &[&str],
) {
    write_fixture(root, fixture);
    let path = root.join("pale-workspace.toml");
    let mut manifest: WorkspaceManifestFile =
        toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let topology = &mut manifest.workspace.desired_topology;
    topology.projects = vec![LogicalProjectV1 {
        id: LogicalProjectIdV1::parse("project").unwrap(),
        title: "Project".into(),
        owner,
        run_policy: RunPolicyIdV1::parse("standard").unwrap(),
        package_seeds: Vec::new(),
        authority_root_ids: Vec::new(),
        capabilities_file: rel("capabilities.toml"),
    }];
    topology.repositories = vec![LogicalRepositoryV1 {
        id: LogicalRepositoryIdV1::parse("source").unwrap(),
        title: "Source".into(),
    }];
    topology.project_repositories = vec![ProjectRepositoryMembershipV1 {
        project_id: topology.projects[0].id.clone(),
        repository_id: topology.repositories[0].id.clone(),
        role: PackageResourceRefV1 {
            package_id: PackageIdV1::parse("engineering").unwrap(),
            resource_id: PackageResourceIdV1::parse("roles.engine").unwrap(),
        },
        stacks: Vec::new(),
        families: Vec::new(),
        responsibility_derivation_ids: duties
            .iter()
            .map(|value| id(value, ResponsibilityDerivationId::parse))
            .collect(),
        materialization_order: 0,
    }];
    fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
}

#[test]
fn explicit_owner_and_duty_preserve_their_independent_records_and_publisher_gate() {
    let fixture = fixture();
    for selected in [
        owner("organization", "candidate", "company"),
        owner("principal", "candidate", "person"),
    ] {
        let root = tempfile::tempdir().unwrap();
        write_project(
            root.path(),
            &fixture,
            Some(selected.clone()),
            &["responsibility.module-reference"],
        );
        let normalized = load_link_normalize(root.path()).unwrap();
        assert_eq!(
            normalized
                .linked
                .source
                .manifest
                .workspace
                .desired_topology
                .projects[0]
                .owner,
            Some(selected)
        );
        assert_eq!(
            normalized.linked.responsibility_derivations.derivations,
            vec![fixture.responsibilities[0].1.clone()]
        );
        assert!(normalized
            .linked
            .source
            .manifest
            .workspace
            .desired_topology
            .execution_workspace_repositories
            .is_empty());
        let verified = universe::seal_and_verify_workspace(normalized).unwrap();
        assert!(matches!(
            verified.admit(&WorkspaceAdmissionProofs::default()),
            Err(WorkspaceError::PublisherAdmissionUnavailable { .. })
        ));
    }
}

#[test]
fn owner_reference_requires_exact_module_resource_and_kind() {
    let fixture = fixture();
    for selected in [
        owner("organization", "absent", "company"),
        owner("organization", "candidate", "absent"),
        owner("principal", "candidate", "company"),
        // The same resource ID exists in this module, but it is a principal.
        owner("organization", "consumer", "company"),
    ] {
        let root = tempfile::tempdir().unwrap();
        write_project(root.path(), &fixture, Some(selected), &[]);
        assert!(
            matches!(load_link_normalize(root.path()), Err(WorkspaceError::NamespaceResolution(message)) if message.contains("project `project` owner"))
        );
    }
    let root = tempfile::tempdir().unwrap();
    write_project(
        root.path(),
        &fixture,
        Some(owner("principal", "consumer", "company")),
        &[],
    );
    load_link_normalize(root.path()).unwrap();
}

#[test]
fn repository_duty_reference_cannot_manufacture_a_derivation() {
    let fixture = fixture();
    let root = tempfile::tempdir().unwrap();
    write_project(root.path(), &fixture, None, &["responsibility.absent"]);
    assert!(
        matches!(load_link_normalize(root.path()), Err(WorkspaceError::NamespaceResolution(message)) if message.contains("repository `source` references absent responsibility `responsibility.absent`"))
    );
}

#[test]
fn captured_but_unadopted_module_cannot_supply_a_project_owner() {
    let mut fixture = candidate_fixture();
    fixture.module.module.resources = vec![WorkspaceResourceSelectionV1::Principal {
        resource_id: id("person", WorkspaceResourceId::parse),
        authority_resolver: id("person-authority", WorkspaceResourceId::parse),
    }];
    reseal_primary(&mut fixture);
    fixture.universe = WorkspaceUniverseLockV1::seal(
        fixture.universe.workspace_id.clone(),
        fixture.resolution.resolution_digest.clone(),
        Vec::new(),
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    write_project(
        root.path(),
        &fixture,
        Some(owner("principal", "candidate", "person")),
        &[],
    );
    assert!(
        matches!(load_link_normalize(root.path()), Err(WorkspaceError::NamespaceResolution(message)) if message.contains("absent from the selected module resources"))
    );
}

#[test]
fn absent_owner_and_duties_stay_unspecified() {
    let fixture = fixture();
    let root = tempfile::tempdir().unwrap();
    write_project(root.path(), &fixture, None, &[]);
    let normalized = load_link_normalize(root.path()).unwrap();
    let topology = &normalized.linked.source.manifest.workspace.desired_topology;
    assert!(topology.projects[0].owner.is_none());
    assert!(topology.project_repositories[0]
        .responsibility_derivation_ids
        .is_empty());
    let encoded = toml::to_string(topology).unwrap();
    assert!(!encoded.contains("owner ="));
    assert!(!encoded.contains("responsibility_derivation_ids"));
}

#[test]
fn authoring_navigation_reaches_the_exact_owner_and_existing_duty() {
    use AuthoringInstancePathSegment::{Field, Index};
    let fixture = fixture();
    let root = tempfile::tempdir().unwrap();
    write_project(
        root.path(),
        &fixture,
        Some(owner("organization", "candidate", "company")),
        &["responsibility.module-reference"],
    );
    let (_, authoring) = load_with_authoring(root.path()).unwrap();
    for (tail, expected_source) in [
        (
            vec![
                Field("projects".into()),
                Index(0),
                Field("owner".into()),
                Field("module_id".into()),
            ],
            "module.toml",
        ),
        (
            vec![
                Field("projects".into()),
                Index(0),
                Field("owner".into()),
                Field("resource_id".into()),
            ],
            "module.toml",
        ),
        (
            vec![
                Field("project_repositories".into()),
                Index(0),
                Field("responsibility_derivation_ids".into()),
                Index(0),
            ],
            "responsibility.toml",
        ),
    ] {
        let mut path = vec![Field("workspace".into()), Field("desired_topology".into())];
        path.extend(tail);
        let occurrence = authoring.occurrence("pale-workspace.toml", &path).unwrap();
        let definition = authoring.definition(occurrence.identity()).unwrap();
        assert_eq!(definition.source_label(), expected_source);
    }
}
