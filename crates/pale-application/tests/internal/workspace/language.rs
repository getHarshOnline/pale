//! Workspace-language trust pipeline, namespace, policy, and admission tests.

use std::fs;
use std::path::Path;

use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::*;

use crate::workspace::universe::language_test_support::WorkspaceAdmissionProofs;
use crate::workspace::{
    link, normalize, source, universe, AuthoringCorrectionKind, AuthoringInstancePathSegment,
    WorkspaceError,
};

use crate::{organization_fixtures, responsibility_fixtures};

#[path = "language/local_source.rs"]
mod local_source;
#[path = "language/project_context.rs"]
mod project_context;

fn id<T>(value: &str, parse: impl FnOnce(String) -> Result<T, InvalidWorkspaceLanguageId>) -> T {
    parse(value.to_string()).unwrap()
}

fn module_id(value: &str) -> WorkspaceModuleId {
    id(value, WorkspaceModuleId::parse)
}

fn rel(value: impl Into<String>) -> OwnedConfinedRelativePath {
    OwnedConfinedRelativePath::parse(value.into()).expect("fixture path is confined")
}

fn qualified(module: &str, namespace: &str, declaration_id: &str) -> link::QualifiedDeclarationRef {
    link::QualifiedDeclarationRef {
        owner_module: module_id(module),
        local_ref: WorkspaceDeclarationRefV1 {
            namespace: id(namespace, WorkspaceNamespaceId::parse),
            declaration_id: id(declaration_id, WorkspaceDeclarationId::parse),
        },
    }
}

fn declaration(namespace: &str, kind: DeclarationDependencyKindV1) -> WorkspaceDeclarationV1 {
    let other = if namespace == "alpha" {
        "beta"
    } else {
        "alpha"
    };
    WorkspaceDeclarationV1::Trait {
        id: id("common", WorkspaceDeclarationId::parse),
        namespace: id(namespace, WorkspaceNamespaceId::parse),
        dependencies: vec![WorkspaceDeclarationDependencyV1 {
            target: WorkspaceDeclarationRefV1 {
                namespace: id(other, WorkspaceNamespaceId::parse),
                declaration_id: id("common", WorkspaceDeclarationId::parse),
            },
            kind,
        }],
    }
}

fn publisher(name: &str) -> WorkspacePublisherRefV1 {
    WorkspacePublisherRefV1 {
        publisher_id: id(name, WorkspacePublisherId::parse),
        root_key_id: id(&format!("{name}-root"), WorkspacePublisherKeyId::parse),
        recovery_authority_id: id(
            &format!("{name}-recovery"),
            WorkspaceRecoveryAuthorityId::parse,
        ),
        delegated_release_key_id: id(&format!("{name}-release"), WorkspacePublisherKeyId::parse),
    }
}

fn ceiling() -> WorkspacePolicyCeilingV1 {
    WorkspacePolicyCeilingV1 {
        disclosure: DisclosureCeilingV1::Public,
        effect: WorkspaceEffectCeilingV1::Read,
        model_use_permitted: false,
        retention_seconds: 3_600,
        rights: WorkspaceRightsCeilingV1 {
            permitted: vec![id("read", WorkspaceRightId::parse)],
        },
        controllers: WorkspaceControllerCeilingV1 {
            permitted: vec![id("principal", WorkspaceControllerScopeId::parse)],
        },
        authorities: WorkspaceAuthorityCeilingV1 {
            permitted: vec![id("principal", WorkspaceAuthorityScopeId::parse)],
        },
    }
}

fn projection(
    owner_module: &WorkspaceModuleId,
    local_namespace: &str,
    declaration: &WorkspaceDeclarationV1,
) -> WorkspaceExportProjectionV1 {
    WorkspaceExportProjectionV1 {
        owner_module: owner_module.clone(),
        local_ref: WorkspaceDeclarationRefV1 {
            namespace: id(local_namespace, WorkspaceNamespaceId::parse),
            declaration_id: declaration.declaration_ref().declaration_id,
        },
        declaration: declaration.clone(),
    }
}

struct CandidateFixture {
    module: WorkspaceModuleV1,
    consumer: Option<WorkspaceModuleV1>,
    additional_modules: Vec<WorkspaceModuleV1>,
    components: Vec<(String, WorkspaceLanguagePrimitivesV1)>,
    responsibilities: Vec<(String, ResponsibilityDerivationV1)>,
    resolution: WorkspaceResolutionLockV1,
    universe: WorkspaceUniverseLockV1,
}

type ExportMutation = (&'static str, fn(&mut WorkspaceExportV1));
type FixtureMutation = (&'static str, fn(&mut CandidateFixture));

fn candidate_fixture() -> CandidateFixture {
    let module_id = module_id("candidate");
    let alpha = WorkspaceLanguagePrimitivesV1 {
        schema: "pale/workspace-language-primitives.v1".into(),
        schema_version: 1,
        component_id: id("alpha-source", WorkspaceComponentId::parse),
        module_id: module_id.clone(),
        declarations: vec![declaration("alpha", DeclarationDependencyKindV1::Positive)],
    };
    let beta = WorkspaceLanguagePrimitivesV1 {
        schema: "pale/workspace-language-primitives.v1".into(),
        schema_version: 1,
        component_id: id("beta-source", WorkspaceComponentId::parse),
        module_id: module_id.clone(),
        declarations: vec![declaration("beta", DeclarationDependencyKindV1::Positive)],
    };
    let components = vec![("alpha.toml".into(), alpha), ("beta.toml".into(), beta)];
    let refs = components
        .iter()
        .map(|(_, component)| WorkspaceSourceComponentRefV1 {
            component_id: component.component_id.clone(),
            component_digest: component.component_digest().unwrap(),
        })
        .collect();
    let module = WorkspaceModuleV1::seal(WorkspaceModulePreimageV1 {
        module_id: module_id.clone(),
        release_id: id("release-1", WorkspaceReleaseId::parse),
        publisher: publisher("candidate-publisher"),
        source_components: refs,
        imports: Vec::new(),
        exports: vec![WorkspaceExportV1 {
            export_id: id("public-api", WorkspaceExportId::parse),
            declarations: components
                .iter()
                .map(|(_, component)| {
                    projection(
                        &module_id,
                        component.declarations[0]
                            .declaration_ref()
                            .namespace
                            .as_ref(),
                        &component.declarations[0],
                    )
                })
                .collect(),
            visibility: WorkspaceVisibilityV1::Public,
            delivery: WorkspaceDeliveryV1::Interface,
            propagation: WorkspacePropagationV1::Transitive,
            ceiling: ceiling(),
            obligations: vec![id("retain-attribution", WorkspaceObligationId::parse)],
            exclusive_authority: Vec::new(),
        }],
        extensions: Vec::new(),
        resources: Vec::new(),
    })
    .unwrap();
    let resolution = WorkspaceResolutionLockV1::seal(vec![WorkspaceResolvedReleaseV1 {
        module_id: module_id.clone(),
        release_id: module.module.release_id.clone(),
        module_digest: module.module_digest.clone(),
        publisher: module.module.publisher.clone(),
        release_state: WorkspaceReleaseStateV1::Published,
    }])
    .unwrap();
    let universe = WorkspaceUniverseLockV1::seal(
        id("synthetic", WorkspaceId::parse),
        resolution.resolution_digest.clone(),
        vec![WorkspaceAdoptedReleaseV1 {
            module_id,
            release_id: module.module.release_id.clone(),
            module_digest: module.module_digest.clone(),
        }],
    )
    .unwrap();
    CandidateFixture {
        module,
        consumer: None,
        additional_modules: Vec::new(),
        components,
        responsibilities: Vec::new(),
        resolution,
        universe,
    }
}

fn reseal_primary(fixture: &mut CandidateFixture) {
    fixture.module.module.source_components = fixture
        .components
        .iter()
        .filter(|(_, component)| component.module_id == fixture.module.module.module_id)
        .map(|(_, component)| WorkspaceSourceComponentRefV1 {
            component_id: component.component_id.clone(),
            component_digest: component.component_digest().unwrap(),
        })
        .collect();
    fixture.module = WorkspaceModuleV1::seal(fixture.module.module.clone()).unwrap();
    reseal_locks(fixture);
}

fn reseal_locks(fixture: &mut CandidateFixture) {
    let modules = std::iter::once(&fixture.module)
        .chain(fixture.consumer.iter())
        .chain(fixture.additional_modules.iter());
    fixture.resolution = WorkspaceResolutionLockV1::seal(
        modules
            .clone()
            .map(|module| WorkspaceResolvedReleaseV1 {
                module_id: module.module.module_id.clone(),
                release_id: module.module.release_id.clone(),
                module_digest: module.module_digest.clone(),
                publisher: module.module.publisher.clone(),
                release_state: WorkspaceReleaseStateV1::Published,
            })
            .collect(),
    )
    .unwrap();
    fixture.universe = WorkspaceUniverseLockV1::seal(
        id("synthetic", WorkspaceId::parse),
        fixture.resolution.resolution_digest.clone(),
        std::iter::once(&fixture.module)
            .chain(fixture.consumer.iter())
            .chain(fixture.additional_modules.iter())
            .map(|module| WorkspaceAdoptedReleaseV1 {
                module_id: module.module.module_id.clone(),
                release_id: module.module.release_id.clone(),
                module_digest: module.module_digest.clone(),
            })
            .collect(),
    )
    .unwrap();
}

fn set_primary_release_state(fixture: &mut CandidateFixture, state: WorkspaceReleaseStateV1) {
    let mut releases = fixture.resolution.releases.clone();
    releases
        .iter_mut()
        .find(|release| release.module_id == fixture.module.module.module_id)
        .unwrap()
        .release_state = state;
    fixture.resolution = WorkspaceResolutionLockV1::seal(releases).unwrap();
    fixture.universe = WorkspaceUniverseLockV1::seal(
        id("synthetic", WorkspaceId::parse),
        fixture.resolution.resolution_digest.clone(),
        fixture.universe.adopted_releases.clone(),
    )
    .unwrap();
}

fn add_consumer(fixture: &mut CandidateFixture, propagation: WorkspacePropagationV1) {
    let consumer_id = module_id("consumer");
    let component = WorkspaceLanguagePrimitivesV1 {
        schema: "pale/workspace-language-primitives.v1".into(),
        schema_version: 1,
        component_id: id("consumer-source", WorkspaceComponentId::parse),
        module_id: consumer_id.clone(),
        declarations: Vec::new(),
    };
    let consumer = WorkspaceModuleV1::seal(WorkspaceModulePreimageV1 {
        module_id: consumer_id,
        release_id: id("release-1", WorkspaceReleaseId::parse),
        publisher: publisher("consumer-publisher"),
        source_components: vec![WorkspaceSourceComponentRefV1 {
            component_id: component.component_id.clone(),
            component_digest: component.component_digest().unwrap(),
        }],
        imports: vec![WorkspaceImportV1 {
            module_id: fixture.module.module.module_id.clone(),
            release_id: fixture.module.module.release_id.clone(),
            module_digest: fixture.module.module_digest.clone(),
            publisher: fixture.module.module.publisher.clone(),
            paths: vec![WorkspaceImportPathV1 {
                export_id: id("public-api", WorkspaceExportId::parse),
                source_namespace: id("alpha", WorkspaceNamespaceId::parse),
                local_namespace: id("upstream", WorkspaceNamespaceId::parse),
            }],
            protected_delivery_authority: None,
            visibility_ceiling: fixture.module.module.exports[0].visibility,
            edge_policy: fixture.module.module.exports[0].ceiling.clone(),
            propagation,
            adoption_policy: WorkspaceAdoptionPolicyV1::Manual,
        }],
        exports: Vec::new(),
        extensions: Vec::new(),
        resources: Vec::new(),
    })
    .unwrap();
    fixture.components.push(("consumer.toml".into(), component));
    fixture.consumer = Some(consumer);
    reseal_locks(fixture);
}

fn record_anchor_fixture(two_extensions: bool) -> CandidateFixture {
    let mut fixture = candidate_fixture();
    add_consumer(&mut fixture, WorkspacePropagationV1::Prevent);
    let consumer = fixture.consumer.as_mut().unwrap();
    consumer.module.imports[0].paths = vec![
        WorkspaceImportPathV1 {
            export_id: id("public-api", WorkspaceExportId::parse),
            source_namespace: id("beta", WorkspaceNamespaceId::parse),
            local_namespace: id("a-alias", WorkspaceNamespaceId::parse),
        },
        WorkspaceImportPathV1 {
            export_id: id("public-api", WorkspaceExportId::parse),
            source_namespace: id("alpha", WorkspaceNamespaceId::parse),
            local_namespace: id("z-alias", WorkspaceNamespaceId::parse),
        },
    ];
    for namespace in if two_extensions {
        vec!["a-alias", "z-alias"]
    } else {
        vec!["a-alias"]
    } {
        consumer.module.extensions.push(WorkspaceExtensionV1 {
            target: WorkspaceDeclarationRefV1 {
                namespace: id(namespace, WorkspaceNamespaceId::parse),
                declaration_id: id("common", WorkspaceDeclarationId::parse),
            },
            namespace: id("augmentation", WorkspaceNamespaceId::parse),
            added_traits: Vec::new(),
            narrowed_ceiling: None,
            added_obligations: Vec::new(),
        });
    }
    reseal_consumer(&mut fixture);
    fixture
}

fn reseal_consumer(fixture: &mut CandidateFixture) {
    let consumer = fixture.consumer.as_mut().unwrap();
    consumer.module.source_components = fixture
        .components
        .iter()
        .filter(|(_, component)| component.module_id == consumer.module.module_id)
        .map(|(_, component)| WorkspaceSourceComponentRefV1 {
            component_id: component.component_id.clone(),
            component_digest: component.component_digest().unwrap(),
        })
        .collect();
    *consumer = WorkspaceModuleV1::seal(consumer.module.clone()).unwrap();
    reseal_locks(fixture);
}

fn add_consumer_dependency(fixture: &mut CandidateFixture) {
    let consumer_component = fixture
        .components
        .iter_mut()
        .find(|(_, component)| component.module_id == module_id("consumer"))
        .expect("consumer source component");
    consumer_component
        .1
        .declarations
        .push(WorkspaceDeclarationV1::Trait {
            id: id("use-upstream", WorkspaceDeclarationId::parse),
            namespace: id("consumer", WorkspaceNamespaceId::parse),
            dependencies: vec![WorkspaceDeclarationDependencyV1 {
                target: WorkspaceDeclarationRefV1 {
                    namespace: id("upstream", WorkspaceNamespaceId::parse),
                    declaration_id: id("common", WorkspaceDeclarationId::parse),
                },
                kind: DeclarationDependencyKindV1::Positive,
            }],
        });
    reseal_consumer(fixture);
}

fn add_policy_carrying_reexport(fixture: &mut CandidateFixture) {
    fixture.module.module.exports[0].visibility = WorkspaceVisibilityV1::Protected;
    fixture.module.module.exports[0].exclusive_authority =
        vec![WorkspaceExclusiveAuthorityClaimV1 {
            scope: id("shared-scope", WorkspaceAuthorityScopeId::parse),
            authority_resolver: id("authority-a", WorkspaceResourceId::parse),
        }];
    reseal_primary(fixture);
    add_consumer(fixture, WorkspacePropagationV1::Transitive);
    let narrowed_projection = projection(
        &fixture.module.module.module_id,
        "upstream",
        &fixture.components[0].1.declarations[0],
    );
    let consumer = fixture.consumer.as_mut().unwrap();
    consumer.module.imports[0].protected_delivery_authority =
        Some(id("delivery-authority", WorkspaceResourceId::parse));
    consumer.module.exports.push(WorkspaceExportV1 {
        export_id: id("narrowed", WorkspaceExportId::parse),
        declarations: vec![narrowed_projection],
        visibility: WorkspaceVisibilityV1::Protected,
        delivery: WorkspaceDeliveryV1::Interface,
        propagation: WorkspacePropagationV1::Transitive,
        ceiling: WorkspacePolicyCeilingV1 {
            disclosure: DisclosureCeilingV1::Protected,
            effect: WorkspaceEffectCeilingV1::None,
            model_use_permitted: false,
            retention_seconds: 1_800,
            rights: ceiling().rights,
            controllers: ceiling().controllers,
            authorities: ceiling().authorities,
        },
        obligations: vec![
            id("retain-attribution", WorkspaceObligationId::parse),
            id("local-audit", WorkspaceObligationId::parse),
        ],
        exclusive_authority: fixture.module.module.exports[0].exclusive_authority.clone(),
    });
    reseal_consumer(fixture);
}

fn add_transitive_consumer(fixture: &mut CandidateFixture) {
    let upstream = fixture
        .consumer
        .as_ref()
        .expect("consumer re-export exists");
    let mut edge_policy = upstream.module.exports[0].ceiling.clone();
    edge_policy.rights.permitted.clear();
    edge_policy.controllers.permitted.clear();
    edge_policy.authorities.permitted.clear();
    let downstream = WorkspaceModuleV1::seal(WorkspaceModulePreimageV1 {
        module_id: module_id("downstream"),
        release_id: id("release-1", WorkspaceReleaseId::parse),
        publisher: publisher("downstream-publisher"),
        source_components: Vec::new(),
        imports: vec![WorkspaceImportV1 {
            module_id: upstream.module.module_id.clone(),
            release_id: upstream.module.release_id.clone(),
            module_digest: upstream.module_digest.clone(),
            publisher: upstream.module.publisher.clone(),
            paths: vec![WorkspaceImportPathV1 {
                export_id: id("narrowed", WorkspaceExportId::parse),
                source_namespace: id("upstream", WorkspaceNamespaceId::parse),
                local_namespace: id("transitive", WorkspaceNamespaceId::parse),
            }],
            protected_delivery_authority: Some(id(
                "downstream-delivery",
                WorkspaceResourceId::parse,
            )),
            visibility_ceiling: WorkspaceVisibilityV1::Private,
            propagation: WorkspacePropagationV1::Prevent,
            edge_policy,
            adoption_policy: WorkspaceAdoptionPolicyV1::Manual,
        }],
        exports: Vec::new(),
        extensions: Vec::new(),
        resources: Vec::new(),
    })
    .unwrap();
    fixture.additional_modules.push(downstream);
    reseal_locks(fixture);
}

fn write_fixture(root: &Path, fixture: &CandidateFixture) {
    fs::create_dir_all(root).unwrap();
    for (name, component) in &fixture.components {
        fs::write(root.join(name), toml::to_string(component).unwrap()).unwrap();
    }
    for (name, responsibility) in &fixture.responsibilities {
        fs::write(root.join(name), toml::to_string(responsibility).unwrap()).unwrap();
    }
    fs::write(
        root.join("module.toml"),
        toml::to_string(&fixture.module).unwrap(),
    )
    .unwrap();
    if let Some(consumer) = &fixture.consumer {
        fs::write(
            root.join("consumer-module.toml"),
            toml::to_string(consumer).unwrap(),
        )
        .unwrap();
    }
    for module in &fixture.additional_modules {
        fs::write(
            root.join(format!("{}-module.toml", module.module.module_id)),
            toml::to_string(module).unwrap(),
        )
        .unwrap();
    }
    fs::write(
        root.join("resolution.lock.toml"),
        toml::to_string(&fixture.resolution).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("universe.lock.toml"),
        toml::to_string(&fixture.universe).unwrap(),
    )
    .unwrap();
    let manifest = WorkspaceManifestFile {
        workspace: WorkspaceMeta {
            schema_version: 1,
            workspace_id: id("synthetic", WorkspaceId::parse),
            distribution_lock: rel("distribution.lock.toml"),
            desired_topology: WorkspaceDesiredTopologyV1 {
                projects: Vec::new(),
                execution_workspaces: Vec::new(),
                repositories: Vec::new(),
                execution_paths: Vec::new(),
                project_repositories: Vec::new(),
                project_execution_workspaces: Vec::new(),
                execution_workspace_repositories: Vec::new(),
            },
            machine_binding_root: rel("machines"),
            module_manifests: std::iter::once(rel("module.toml"))
                .chain(
                    fixture
                        .consumer
                        .as_ref()
                        .map(|_| rel("consumer-module.toml")),
                )
                .chain(
                    fixture
                        .additional_modules
                        .iter()
                        .map(|module| rel(format!("{}-module.toml", module.module.module_id))),
                )
                .collect(),
            source_components: fixture
                .components
                .iter()
                .map(|(name, _)| rel(name))
                .collect(),
            module_sources: Vec::new(),
            responsibility_derivations: fixture
                .responsibilities
                .iter()
                .map(|(name, _)| rel(name))
                .collect(),
            organizational_translations: Vec::new(),
            experience_compositions: Vec::new(),
            resolution_lock: rel("resolution.lock.toml"),
            universe_lock: rel("universe.lock.toml"),
        },
    };
    fs::write(
        root.join("pale-workspace.toml"),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();
}

fn load_link_normalize(root: &Path) -> Result<normalize::NormalizedWorkspace, WorkspaceError> {
    let parsed = source::parse_workspace_source(
        &pale_adapter_fs::LocalSourceFilesystem,
        &root.join("pale-workspace.toml"),
    )?;
    normalize::normalize_workspace(link::link_workspace(parsed)?)
}

fn load_with_authoring(
    root: &Path,
) -> Result<
    (
        normalize::NormalizedWorkspace,
        crate::workspace::AuthoringSemanticIndex,
    ),
    WorkspaceError,
> {
    let parsed = source::parse_workspace_source(
        &pale_adapter_fs::LocalSourceFilesystem,
        &root.join("pale-workspace.toml"),
    )?;
    crate::workspace::normalize_workspace_source(parsed)
}

fn capture_portable_fixture(root: &Path) -> crate::workspace::CapturedWorkspaceSource {
    use crate::workspace::{CapturedWorkspaceDocument, CapturedWorkspaceSource};
    let documents = fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            CapturedWorkspaceDocument::new(
                entry.file_name().to_str().unwrap().to_owned(),
                fs::read(entry.path()).unwrap(),
            )
            .unwrap()
        })
        .collect();
    CapturedWorkspaceSource::new(
        CapturedWorkspaceSource::admit_root_manifest("pale-workspace.toml".into()).unwrap(),
        documents,
    )
    .unwrap()
}

fn load_portable_fixture(root: &Path) -> crate::workspace::PortableWorkspaceUniverse {
    crate::workspace::compile_portable_workspace(capture_portable_fixture(root)).unwrap()
}

#[test]
fn record_anchors_retain_original_indices_without_retagging_named_references() {
    use crate::workspace::{
        AuthoringIdentityKind, AuthoringInstancePathSegment::*, AuthoringOccurrenceRole,
    };
    let fixture = record_anchor_fixture(true);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let captured = capture_portable_fixture(temp.path());
    let parsed =
        source::parse_workspace_source(&captured, Path::new("pale-workspace.toml")).unwrap();
    let mut linked = link::link_workspace(parsed).unwrap();
    assert_eq!(linked.record_anchors.len(), 4);
    let retained_subjects = linked
        .resolved_imports
        .iter()
        .map(link::ResolvedImportPath::subject)
        .chain(linked.extensions.iter().map(link::LinkedExtension::subject))
        .collect::<Vec<_>>();
    let authoring = crate::workspace::AuthoringSemanticIndex::from_linked(&mut linked);
    assert!(
        linked.record_anchors.is_empty(),
        "intake moves the one sidecar map"
    );
    let (normalized, filesystem_authoring) = load_with_authoring(temp.path()).unwrap();
    for subject in &retained_subjects {
        assert_eq!(
            authoring.subject_record_anchor(subject),
            filesystem_authoring.subject_record_anchor(subject)
        );
    }
    // Source aliases sort a→beta, z→alpha; resolved extensions sort alpha→beta.
    assert_eq!(
        linked.extensions[0].target.local_ref.namespace.as_str(),
        "alpha"
    );
    for (extension, source_index) in linked.extensions.iter().zip([1, 0]) {
        let subject = extension.subject();
        let anchor = authoring.subject_record_anchor(&subject).unwrap();
        assert_eq!(anchor.source_label(), "consumer-module.toml");
        assert_eq!(
            anchor.path(),
            &[
                Field("module".into()),
                Field("extensions".into()),
                Index(source_index)
            ]
        );
        assert!(authoring
            .occurrence(anchor.source_label(), anchor.path())
            .is_none());
        let mut target_path = anchor.path().to_vec();
        target_path.push(Field("target".into()));
        target_path.push(Field("declaration_id".into()));
        let occurrence = authoring
            .occurrence(anchor.source_label(), &target_path)
            .unwrap();
        assert_eq!(
            occurrence.identity().kind(),
            AuthoringIdentityKind::Declaration
        );
        assert_eq!(occurrence.role(), AuthoringOccurrenceRole::Reference);
        assert_ne!(
            authoring.proof_subject(occurrence.identity()),
            Some(&subject)
        );
    }
    for (source_index, import) in linked.resolved_imports.iter().enumerate() {
        let subject = import.subject();
        let anchor = authoring.subject_record_anchor(&subject).unwrap();
        assert_eq!(anchor.source_label(), "consumer-module.toml");
        assert!(authoring
            .occurrence(anchor.source_label(), anchor.path())
            .is_none());
        assert_eq!(
            anchor.path(),
            &[
                Field("module".into()),
                Field("imports".into()),
                Index(0),
                Field("paths".into()),
                Index(source_index)
            ]
        );
        let mut namespace_path = anchor.path().to_vec();
        namespace_path.push(Field("local_namespace".into()));
        let occurrence = authoring
            .occurrence(anchor.source_label(), &namespace_path)
            .unwrap();
        assert_eq!(
            occurrence.identity().kind(),
            AuthoringIdentityKind::Namespace
        );
        assert_eq!(occurrence.role(), AuthoringOccurrenceRole::Definition);
        assert!(authoring.proof_subject(occurrence.identity()).is_none());
        let WorkspaceDerivationSubjectV1::Import {
            local_namespace: _,
            importer_module,
            upstream_module,
            upstream_release,
            export_id,
            source_namespace,
        } = subject
        else {
            unreachable!()
        };
        let local_namespace = id("absent", WorkspaceNamespaceId::parse);
        assert!(authoring
            .subject_record_anchor(&WorkspaceDerivationSubjectV1::Import {
                importer_module,
                upstream_module,
                upstream_release,
                export_id,
                source_namespace,
                local_namespace,
            })
            .is_none());
    }
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let proof = WorkspaceAdmissionProofs::synthetic_for(&verified);
    let admitted = verified.admit(&proof).unwrap();
    for subject in &retained_subjects {
        assert!(admitted
            .derivation_nodes()
            .iter()
            .any(|node| &node.subject == subject));
    }
    let failure = crate::workspace::compile_portable_workspace(captured).unwrap_err();
    assert_eq!(
        failure.class(),
        crate::workspace::PortableWorkspaceFailureClass::OwnerProofUnavailable
    );
    drop(admitted);
    drop(linked);
    drop(filesystem_authoring);
    drop(fixture);
    drop(temp);
    for subject in &retained_subjects {
        assert_eq!(
            authoring
                .subject_record_anchor(subject)
                .unwrap()
                .source_label(),
            "consumer-module.toml"
        );
    }
    let unsupported = WorkspaceDerivationSubjectV1::Declaration {
        owner_module: module_id("candidate"),
        declaration: qualified("candidate", "alpha", "common").local_ref,
    };
    assert!(authoring.subject_record_anchor(&unsupported).is_none());
}

#[test]
fn record_anchor_collision_keeps_existing_linked_subject_rejection() {
    let mut fixture = record_anchor_fixture(true);
    fixture.consumer.as_mut().unwrap().module.imports[0].paths[0].source_namespace =
        id("alpha", WorkspaceNamespaceId::parse);
    reseal_consumer(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let (normalized, authoring) = load_with_authoring(temp.path()).unwrap();
    assert_eq!(normalized.linked.extensions.len(), 2);
    let subject = normalized.linked.extensions[0].subject();
    assert_eq!(subject, normalized.linked.extensions[1].subject());
    assert_eq!(
        authoring
            .subject_record_anchor(&subject)
            .unwrap()
            .path()
            .last(),
        Some(&AuthoringInstancePathSegment::Index(0))
    );
    let error = universe::seal_and_verify_workspace(normalized).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::DuplicateLanguageIdentity {
            kind: "derivation subject",
            ..
        }
    ));
}

#[test]
fn record_anchor_absence_on_public_empty_workspace_grants_no_subject_presence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let portable = load_portable_fixture(&root);
    let subject = WorkspaceDerivationSubjectV1::Extension {
        owner_module: module_id("not-selected"),
        namespace: id("augmentation", WorkspaceNamespaceId::parse),
        target: qualified("not-selected", "names", "target").local_ref,
    };
    assert!(portable.subject_record_anchor(&subject).is_none());
}

#[test]
fn source_denotations_join_actual_producers_without_merging_proof_subjects() {
    use crate::workspace::{AuthoringInstancePathSegment::*, AuthoringOccurrenceRole};
    let mut fixture = candidate_fixture();
    fixture
        .module
        .module
        .resources
        .push(WorkspaceResourceSelectionV1::Package {
            resource_id: id("selected-package", WorkspaceResourceId::parse),
            release: PackageReleaseV1 {
                id: PackageIdV1::parse("selected").unwrap(),
                version: PackageVersionV1::parse("1.0.0").unwrap(),
                content_digest: PackageDigest::hash_bytes(b"selected"),
            },
        });
    reseal_primary(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let (normalized, authoring) = load_with_authoring(temp.path()).unwrap();
    let subject_at = |label: &str, path: &[AuthoringInstancePathSegment]| {
        let occurrence = authoring.occurrence(label, path).unwrap();
        authoring
            .proof_subject(occurrence.identity())
            .unwrap()
            .clone()
    };
    let parsed = &normalized.linked.source;
    assert_eq!(
        subject_at("pale-workspace.toml", &[]),
        WorkspaceDerivationSubjectV1::Source {
            source_id: parsed.manifest_source.source_id.clone()
        }
    );
    assert_eq!(
        subject_at(
            "pale-workspace.toml",
            &[Field("workspace".into()), Field("workspace_id".into())]
        ),
        WorkspaceDerivationSubjectV1::Workspace {
            workspace_id: parsed.manifest.workspace.workspace_id.clone()
        }
    );
    for (index, label) in parsed
        .manifest
        .workspace
        .module_manifests
        .iter()
        .enumerate()
    {
        assert_eq!(
            subject_at(label.as_str(), &[]),
            subject_at(
                "pale-workspace.toml",
                &[
                    Field("workspace".into()),
                    Field("module_manifests".into()),
                    Index(index)
                ]
            )
        );
    }
    for (index, (label, component)) in fixture.components.iter().enumerate() {
        assert_eq!(
            subject_at(label, &[]),
            subject_at(
                "pale-workspace.toml",
                &[
                    Field("workspace".into()),
                    Field("source_components".into()),
                    Index(index)
                ]
            )
        );
        assert_eq!(
            subject_at(label, &[Field("component_id".into())]),
            WorkspaceDerivationSubjectV1::Component {
                component_id: component.component_id.clone()
            }
        );
        assert_eq!(
            subject_at(label, &[Field("module_id".into())]),
            WorkspaceDerivationSubjectV1::Module {
                module_id: component.module_id.clone()
            }
        );
    }
    for (label, field_name) in [
        ("resolution.lock.toml", "resolution_lock"),
        ("universe.lock.toml", "universe_lock"),
    ] {
        assert_eq!(
            subject_at(label, &[]),
            subject_at(
                "pale-workspace.toml",
                &[Field("workspace".into()), Field(field_name.into())]
            )
        );
    }
    let resource = WorkspaceDerivationSubjectV1::Resource {
        owner_module: fixture.module.module.module_id.clone(),
        resource_id: fixture.module.module.resources[0].resource_id().clone(),
    };
    assert_eq!(
        subject_at(
            "module.toml",
            &[
                Field("module".into()),
                Field("resources".into()),
                Index(0),
                Field("resource_id".into())
            ]
        ),
        resource
    );
    let declaration = qualified("candidate", "alpha", "common");
    let subject = WorkspaceDerivationSubjectV1::Declaration {
        owner_module: declaration.owner_module.clone(),
        declaration: declaration.local_ref.clone(),
    };
    let definition = authoring
        .occurrence(
            "alpha.toml",
            &[Field("declarations".into()), Index(0), Field("id".into())],
        )
        .unwrap();
    assert_eq!(
        authoring.proof_subject(definition.identity()),
        Some(&subject)
    );
    let page = authoring.subject_occurrences_page(&subject, None, 256, 256);
    assert!(page.is_complete());
    assert_eq!(
        page.occurrences(),
        authoring
            .occurrences_page(definition.identity(), None, 256, 256)
            .occurrences()
    );
    for role in [
        AuthoringOccurrenceRole::Definition,
        AuthoringOccurrenceRole::Reference,
        AuthoringOccurrenceRole::Projection,
    ] {
        assert!(page
            .occurrences()
            .iter()
            .any(|occurrence| occurrence.role() == role));
    }
    assert_eq!(
        authoring.definition(definition.identity()),
        Some(definition)
    );
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let proofs = WorkspaceAdmissionProofs::synthetic_for(&verified);
    let admitted = verified.admit(&proofs).unwrap();
    assert!(admitted.contains_derivation_subject(&subject));
    assert!(admitted.contains_derivation_subject(&resource));
    let policy = WorkspaceDerivationSubjectV1::Policy {
        owner_module: declaration.owner_module,
        declaration: declaration.local_ref,
    };
    assert!(admitted.contains_derivation_subject(&policy));
    let policy_page = authoring.subject_occurrences_page(&policy, None, 256, 256);
    assert!(policy_page.is_complete());
    assert!(policy_page.occurrences().is_empty());
    assert!(admitted
        .direct_dependency_page(&policy, 0, 64)
        .unwrap()
        .is_some());
}

#[test]
fn canonical_language_fixtures_match_the_record_builders() {
    let update = std::env::var_os("PALE_UPDATE_CONFORMANCE").as_deref() == Some("1".as_ref());
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR")).join(if update {
        "../../contracts/conformance/workspace-language.v1"
    } else {
        "tests/fixtures/workspace-language"
    });
    let compare = |case: &str, member: &str, bytes: &[u8]| {
        let path = canonical.join(case).join(member);
        if update {
            fs::write(path, bytes).unwrap();
        } else {
            assert_eq!(fs::read(path).unwrap(), bytes, "{case}/{member}");
        }
    };
    let mut fixture = candidate_fixture();
    let temporary = tempfile::tempdir().unwrap();
    write_fixture(temporary.path(), &fixture);
    for member in [
        "alpha.toml",
        "beta.toml",
        "module.toml",
        "resolution.lock.toml",
        "universe.lock.toml",
        "pale-workspace.toml",
    ] {
        compare(
            "populated",
            member,
            &fs::read(temporary.path().join(member)).unwrap(),
        );
    }
    let candidate =
        pale_core::canon::parse_strict(fixture.module.canonical_candidate().unwrap().as_bytes())
            .unwrap();
    let mut candidate_bytes = serde_json::to_vec_pretty(&candidate).unwrap();
    candidate_bytes.push(b'\n');
    compare("populated", "module-candidate.json", &candidate_bytes);
    // Keep the handwritten parser alternative untouched, including its different formatting.
    let handwritten: WorkspaceLanguagePrimitivesV1 = toml::from_str(
        &fs::read_to_string(canonical.join("populated/alpha-handwritten.toml")).unwrap(),
    )
    .unwrap();
    assert_eq!(handwritten, fixture.components[0].1);

    set_primary_release_state(&mut fixture, WorkspaceReleaseStateV1::Candidate);
    compare(
        "release-candidate",
        "resolution.lock.toml",
        toml::to_string(&fixture.resolution).unwrap().as_bytes(),
    );
    compare(
        "release-candidate",
        "universe.lock.toml",
        toml::to_string(&fixture.universe).unwrap().as_bytes(),
    );

    let mut incomplete = candidate_fixture();
    incomplete.module.module.exports[0].delivery = WorkspaceDeliveryV1::Source;
    reseal_primary(&mut incomplete);
    // Source delivery still lacks component documents: regenerate identities, not the missing proof.
    compare(
        "source-incomplete",
        "module.toml",
        toml::to_string(&incomplete.module).unwrap().as_bytes(),
    );
    compare(
        "source-incomplete",
        "resolution.lock.toml",
        toml::to_string(&incomplete.resolution).unwrap().as_bytes(),
    );
    compare(
        "source-incomplete",
        "universe.lock.toml",
        toml::to_string(&incomplete.universe).unwrap().as_bytes(),
    );
}

#[test]
fn candidate_builder_and_direct_toml_are_jcs_identical_and_namespaces_are_independent() {
    let fixture = candidate_fixture();
    let direct: WorkspaceModuleV1 = toml::from_str(include_str!(
        "../../fixtures/workspace-language/populated/module.toml"
    ))
    .unwrap();
    assert_eq!(
        direct.canonical_candidate().unwrap(),
        fixture.module.canonical_candidate().unwrap()
    );
    let handwritten_component: WorkspaceLanguagePrimitivesV1 = toml::from_str(include_str!(
        "../../fixtures/workspace-language/populated/alpha-handwritten.toml"
    ))
    .unwrap();
    assert_eq!(handwritten_component, fixture.components[0].1);
    assert_eq!(
        handwritten_component.component_digest().unwrap(),
        fixture.components[0].1.component_digest().unwrap()
    );
    assert_ne!(
        &include_bytes!("../../fixtures/workspace-language/populated/alpha-handwritten.toml")[..],
        &include_bytes!("../../fixtures/workspace-language/populated/alpha.toml")[..]
    );
    let candidate_fixture = pale_core::canon::parse_strict(include_bytes!(
        "../../fixtures/workspace-language/populated/module-candidate.json"
    ))
    .unwrap();
    assert_eq!(
        fixture.module.canonical_candidate().unwrap(),
        pale_core::canon::to_canonical_string(&candidate_fixture).unwrap()
    );

    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let parsed = source::parse_workspace_source(
        &pale_adapter_fs::LocalSourceFilesystem,
        &temp.path().join("pale-workspace.toml"),
    )
    .unwrap();
    let normalized = normalize::normalize_workspace(link::link_workspace(parsed).unwrap()).unwrap();
    assert_eq!(normalized.linked.declarations.len(), 2);
    assert!(normalized.linked.declarations.contains_key(&qualified(
        "candidate",
        "alpha",
        "common"
    )));
    assert!(normalized
        .linked
        .declarations
        .contains_key(&qualified("candidate", "beta", "common")));
}

#[test]
fn malformed_projection_fails_schema_and_typed_decode() {
    let fixture = candidate_fixture();
    let mut malformed = serde_json::to_value(&fixture.module).unwrap();
    malformed["module"]["exports"][0]["declarations"][0]
        .as_object_mut()
        .unwrap()
        .remove("declaration");
    assert!(pale_contracts::validate_value(&malformed, "workspace-module.v1").is_err());
    assert!(serde_json::from_value::<WorkspaceModuleV1>(malformed).is_err());
}

#[test]
fn equal_local_names_remain_module_scoped_and_cannot_transpose() {
    let mut fixture = candidate_fixture();
    add_consumer(&mut fixture, WorkspacePropagationV1::Transitive);
    fixture.consumer.as_mut().unwrap().module.imports.clear();
    fixture
        .components
        .iter_mut()
        .find(|(_, component)| component.module_id == module_id("consumer"))
        .unwrap()
        .1
        .declarations = vec![WorkspaceDeclarationV1::Trait {
        id: id("common", WorkspaceDeclarationId::parse),
        namespace: id("alpha", WorkspaceNamespaceId::parse),
        dependencies: Vec::new(),
    }];
    reseal_consumer(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let normalized = load_link_normalize(temp.path()).unwrap();
    assert!(normalized.linked.declarations.contains_key(&qualified(
        "candidate",
        "alpha",
        "common"
    )));
    assert!(normalized
        .linked
        .declarations
        .contains_key(&qualified("consumer", "alpha", "common")));

    fixture
        .components
        .iter_mut()
        .find(|(_, component)| component.module_id == module_id("consumer"))
        .unwrap()
        .1
        .declarations = vec![WorkspaceDeclarationV1::Trait {
        id: id("local-use", WorkspaceDeclarationId::parse),
        namespace: id("consumer", WorkspaceNamespaceId::parse),
        dependencies: vec![WorkspaceDeclarationDependencyV1 {
            target: WorkspaceDeclarationRefV1 {
                namespace: id("alpha", WorkspaceNamespaceId::parse),
                declaration_id: id("common", WorkspaceDeclarationId::parse),
            },
            kind: DeclarationDependencyKindV1::Positive,
        }],
    }];
    reseal_consumer(&mut fixture);
    write_fixture(temp.path(), &fixture);
    assert!(matches!(
        load_link_normalize(temp.path()),
        Err(WorkspaceError::NamespaceResolution(_))
    ));
}

#[test]
fn interface_projection_compiles_without_upstream_source_and_drops_private_interiors() {
    let mut fixture = candidate_fixture();
    fixture.components[0]
        .1
        .declarations
        .push(WorkspaceDeclarationV1::Trait {
            id: id("private-detail", WorkspaceDeclarationId::parse),
            namespace: id("alpha", WorkspaceNamespaceId::parse),
            dependencies: Vec::new(),
        });
    reseal_primary(&mut fixture);
    add_consumer(&mut fixture, WorkspacePropagationV1::Transitive);
    add_consumer_dependency(&mut fixture);
    fixture
        .components
        .retain(|(_, component)| component.module_id == module_id("consumer"));

    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let normalized = load_link_normalize(temp.path()).unwrap();
    assert_eq!(normalized.linked.declarations.len(), 3);
    assert!(!normalized.linked.declarations.contains_key(&qualified(
        "candidate",
        "alpha",
        "private-detail"
    )));
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let proofs = WorkspaceAdmissionProofs::synthetic_for(&verified);
    let admitted = verified.admit(&proofs).unwrap();
    assert_eq!(admitted.semantic_declaration_count(), 3);
    assert_eq!(admitted.admission_evidence_count(), 2);
}

#[test]
fn authoring_retains_unselected_interface_source_without_resolved_references() {
    use crate::workspace::{AuthoringInstancePathSegment::*, AuthoringOccurrenceRole};

    let mut fixture = candidate_fixture();
    fixture.components[0]
        .1
        .declarations
        .push(WorkspaceDeclarationV1::Trait {
            id: id("private-detail", WorkspaceDeclarationId::parse),
            namespace: id("alpha", WorkspaceNamespaceId::parse),
            dependencies: vec![WorkspaceDeclarationDependencyV1 {
                target: qualified("candidate", "alpha", "common").local_ref,
                kind: DeclarationDependencyKindV1::Positive,
            }],
        });
    reseal_primary(&mut fixture);
    add_consumer(&mut fixture, WorkspacePropagationV1::Transitive);
    add_consumer_dependency(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let (normalized, authoring) = load_with_authoring(temp.path()).unwrap();

    let definition = authoring
        .occurrence(
            "alpha.toml",
            &[Field("declarations".into()), Index(1), Field("id".into())],
        )
        .expect("captured private source remains authorable");
    assert_eq!(definition.role(), AuthoringOccurrenceRole::Definition);
    let private_subject = WorkspaceDerivationSubjectV1::Declaration {
        owner_module: module_id("candidate"),
        declaration: qualified("candidate", "alpha", "private-detail").local_ref,
    };
    assert_eq!(
        authoring.proof_subject(definition.identity()),
        Some(&private_subject)
    );
    let namespace = authoring
        .occurrence(
            "alpha.toml",
            &[
                Field("declarations".into()),
                Index(1),
                Field("namespace".into()),
            ],
        )
        .unwrap();
    assert!(authoring.proof_subject(namespace.identity()).is_none());
    assert_eq!(
        authoring
            .occurrence(
                "alpha.toml",
                &[
                    Field("declarations".into()),
                    Index(1),
                    Field("namespace".into()),
                ]
            )
            .unwrap()
            .role(),
        AuthoringOccurrenceRole::Definition,
    );
    assert_eq!(
        authoring.definition(definition.identity()),
        Some(definition)
    );
    assert!(authoring
        .occurrence(
            "alpha.toml",
            &[
                Field("declarations".into()),
                Index(1),
                Field("dependencies".into()),
                Index(0),
                Field("target".into()),
                Field("declaration_id".into()),
            ]
        )
        .is_none());
    let exported_reference = authoring
        .occurrence(
            "alpha.toml",
            &[
                Field("declarations".into()),
                Index(0),
                Field("dependencies".into()),
                Index(0),
                Field("target".into()),
                Field("declaration_id".into()),
            ],
        )
        .expect("selected export still has a resolved dependency");
    assert_eq!(
        exported_reference.role(),
        AuthoringOccurrenceRole::Reference
    );
    assert!(authoring
        .definition(exported_reference.identity())
        .is_some());
    assert_eq!(normalized.linked.declarations.len(), 3);
    assert!(!normalized.linked.declarations.contains_key(&qualified(
        "candidate",
        "alpha",
        "private-detail",
    )));
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let proofs = WorkspaceAdmissionProofs::synthetic_for(&verified);
    let admitted = verified.admit(&proofs).unwrap();
    assert_eq!(admitted.semantic_declaration_count(), 3);
    assert!(!admitted.contains_derivation_subject(&private_subject));
    let private_page = authoring.subject_occurrences_page(&private_subject, None, 256, 256);
    assert!(private_page.is_complete());
    assert_eq!(private_page.occurrences(), &[definition]);
}

#[test]
fn present_owner_source_must_equal_its_export_projection() {
    let mut fixture = candidate_fixture();
    if let WorkspaceDeclarationV1::Trait { dependencies, .. } =
        &mut fixture.module.module.exports[0].declarations[0].declaration
    {
        dependencies.clear();
    }
    fixture.module = WorkspaceModuleV1::seal(fixture.module.module.clone()).unwrap();
    reseal_locks(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    assert!(matches!(
        load_link_normalize(temp.path()),
        Err(WorkspaceError::ExportProjectionMismatch(_))
    ));
}

#[test]
fn source_delivery_requires_the_upstream_source_components() {
    let mut fixture = candidate_fixture();
    fixture.module.module.exports[0].delivery = WorkspaceDeliveryV1::Source;
    reseal_primary(&mut fixture);
    add_consumer(&mut fixture, WorkspacePropagationV1::Transitive);
    fixture
        .components
        .retain(|(_, component)| component.module_id == module_id("consumer"));
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    assert!(matches!(
        load_link_normalize(temp.path()),
        Err(WorkspaceError::SourceDeliveryUnavailable { .. })
    ));
}

#[test]
fn publisher_gate_fails_closed_and_test_proof_admits_without_runtime_implication() {
    let fixture = candidate_fixture();
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let normalized = load_link_normalize(temp.path()).unwrap();
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    assert!(matches!(
        verified.admit(&WorkspaceAdmissionProofs::default()),
        Err(WorkspaceError::PublisherAdmissionUnavailable { .. })
    ));

    let normalized = load_link_normalize(temp.path()).unwrap();
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let proof = WorkspaceAdmissionProofs::synthetic_for(&verified);
    let admitted = verified.admit(&proof).unwrap();
    assert!(!admitted.digest().as_str().is_empty());
    assert!(!admitted.workspace_semantic_ir_digest().as_str().is_empty());
    assert_eq!(admitted.admission_evidence_count(), 1);
    assert!(admitted.admission_evidence_custody_is_complete());

    let normalized = load_link_normalize(temp.path()).unwrap();
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let mut reissued = WorkspaceAdmissionProofs::synthetic_for(&verified);
    reissued.reissue_publisher_proofs();
    let reissued_admission = verified.admit(&reissued).unwrap();
    assert_eq!(
        admitted.workspace_semantic_ir_digest(),
        reissued_admission.workspace_semantic_ir_digest()
    );
    assert_ne!(
        admitted.admission_evidence_digest(),
        reissued_admission.admission_evidence_digest()
    );
    assert_ne!(admitted.digest(), reissued_admission.digest());

    let mut corrupted = admitted;
    corrupted.corrupt_admission_evidence_digest();
    assert!(!corrupted.admission_evidence_custody_is_complete());
}

#[test]
fn lifecycle_state_is_resolution_owned_and_new_admission_requires_published() {
    let mut fixture = candidate_fixture();
    let module_digest = fixture.module.module_digest.clone();
    let published_resolution = fixture.resolution.resolution_digest.clone();
    for state in [
        WorkspaceReleaseStateV1::Candidate,
        WorkspaceReleaseStateV1::Revoked,
    ] {
        set_primary_release_state(&mut fixture, state);
        assert_eq!(fixture.module.module_digest, module_digest);
        assert_ne!(fixture.resolution.resolution_digest, published_resolution);
        let temp = tempfile::tempdir().unwrap();
        write_fixture(temp.path(), &fixture);
        let normalized = load_link_normalize(temp.path()).unwrap();
        let verified = universe::seal_and_verify_workspace(normalized).unwrap();
        let proof = WorkspaceAdmissionProofs::synthetic_for(&verified);
        assert!(matches!(
            verified.admit(&proof),
            Err(WorkspaceError::ReleaseStateNotAdmissible { .. })
        ));
    }
}

#[test]
fn protected_paths_require_distinct_exact_proofs() {
    let mut fixture = candidate_fixture();
    fixture.module.module.exports[0].visibility = WorkspaceVisibilityV1::Protected;
    reseal_primary(&mut fixture);
    add_consumer(&mut fixture, WorkspacePropagationV1::Transitive);
    fixture.consumer.as_mut().unwrap().module.imports[0].protected_delivery_authority =
        Some(id("delivery-authority", WorkspaceResourceId::parse));
    reseal_consumer(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);

    let normalized = load_link_normalize(temp.path()).unwrap();
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let mut missing_protected = WorkspaceAdmissionProofs::synthetic_for(&verified);
    missing_protected.clear_protected_delivery();
    assert!(matches!(
        verified.admit(&missing_protected),
        Err(WorkspaceError::ProtectedDeliveryAdmissionUnavailable { .. })
    ));

    let normalized = load_link_normalize(temp.path()).unwrap();
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let mut transposed_protected = WorkspaceAdmissionProofs::synthetic_for(&verified);
    transposed_protected
        .transpose_protected_authority(id("wrong-authority", WorkspaceResourceId::parse));
    assert!(matches!(
        verified.admit(&transposed_protected),
        Err(WorkspaceError::ProtectedDeliveryAdmissionUnavailable { .. })
    ));

    let normalized = load_link_normalize(temp.path()).unwrap();
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let proofs = WorkspaceAdmissionProofs::synthetic_for(&verified);
    verified.admit(&proofs).unwrap();
}

#[test]
fn derivation_receipt_is_closed_complete_and_retains_exact_source_order_evidence() {
    let mut current_fixture = candidate_fixture();
    current_fixture.components.push((
        "gamma.toml".into(),
        WorkspaceLanguagePrimitivesV1 {
            schema: "pale/workspace-language-primitives.v1".into(),
            schema_version: 1,
            component_id: id("gamma-source", WorkspaceComponentId::parse),
            module_id: current_fixture.module.module.module_id.clone(),
            declarations: Vec::new(),
        },
    ));
    reseal_primary(&mut current_fixture);

    let mut prior_fixture = candidate_fixture();
    prior_fixture.components[1]
        .1
        .declarations
        .push(WorkspaceDeclarationV1::Trait {
            id: id("prior-only", WorkspaceDeclarationId::parse),
            namespace: id("beta", WorkspaceNamespaceId::parse),
            dependencies: Vec::new(),
        });
    prior_fixture.components.push((
        "removed.toml".into(),
        WorkspaceLanguagePrimitivesV1 {
            schema: "pale/workspace-language-primitives.v1".into(),
            schema_version: 1,
            component_id: id("removed-source", WorkspaceComponentId::parse),
            module_id: prior_fixture.module.module.module_id.clone(),
            declarations: Vec::new(),
        },
    ));
    reseal_primary(&mut prior_fixture);
    let prior_dir = tempfile::tempdir().unwrap();
    write_fixture(prior_dir.path(), &prior_fixture);
    let prior_verified =
        universe::seal_and_verify_workspace(load_link_normalize(prior_dir.path()).unwrap())
            .unwrap();
    let prior_proof = WorkspaceAdmissionProofs::synthetic_for(&prior_verified);
    let prior_admitted = prior_verified.admit(&prior_proof).unwrap();
    let prior_derivation = prior_admitted.into_resident_parts().derivation;
    let current_dir = tempfile::tempdir().unwrap();
    write_fixture(current_dir.path(), &current_fixture);
    let current_verified =
        universe::seal_and_verify_workspace(load_link_normalize(current_dir.path()).unwrap())
            .unwrap();
    let current_proof = WorkspaceAdmissionProofs::synthetic_for(&current_verified);
    let current_admitted = current_verified.admit(&current_proof).unwrap();
    let receipt = current_admitted
        .derivation_receipt(&prior_derivation)
        .unwrap();
    receipt.verify().unwrap();
    current_admitted
        .verify_derivation_receipt(&prior_derivation, &receipt)
        .unwrap();
    let root_sources = receipt
        .roots
        .iter()
        .filter_map(|root| match &root.subject {
            WorkspaceDerivationSubjectV1::Source { source_id } => Some(source_id.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(root_sources.contains(&"component:beta-source"));
    assert!(root_sources.contains(&"component:gamma-source"));
    assert!(root_sources.contains(&"component:removed-source"));
    assert_eq!(receipt.impacts.len(), receipt.nodes.len());
    assert!(receipt.impacts.iter().any(|impact| {
        matches!(
            (&impact.subject, &impact.state),
            (
                WorkspaceDerivationSubjectV1::Workspace { .. },
                WorkspaceDerivationImpactStateV1::Affected { .. }
            )
        )
    }));

    prior_fixture.components.reverse();
    current_fixture.components.reverse();
    reseal_primary(&mut prior_fixture);
    reseal_primary(&mut current_fixture);
    let permuted_prior_dir = tempfile::tempdir().unwrap();
    write_fixture(permuted_prior_dir.path(), &prior_fixture);
    let permuted_prior = universe::seal_and_verify_workspace(
        load_link_normalize(permuted_prior_dir.path()).unwrap(),
    )
    .unwrap();
    let permuted_prior_proof = WorkspaceAdmissionProofs::synthetic_for(&permuted_prior);
    let permuted_prior = permuted_prior.admit(&permuted_prior_proof).unwrap();
    let permuted_prior = permuted_prior.into_resident_parts().derivation;
    let permuted_current_dir = tempfile::tempdir().unwrap();
    write_fixture(permuted_current_dir.path(), &current_fixture);
    let permuted_current = universe::seal_and_verify_workspace(
        load_link_normalize(permuted_current_dir.path()).unwrap(),
    )
    .unwrap();
    let permuted_current_proof = WorkspaceAdmissionProofs::synthetic_for(&permuted_current);
    let permuted_current = permuted_current.admit(&permuted_current_proof).unwrap();
    let second = permuted_current
        .derivation_receipt(&permuted_prior)
        .unwrap();
    assert_ne!(receipt, second);
    assert_eq!(
        receipt.workspace_semantic_ir_digest,
        second.workspace_semantic_ir_digest
    );
    let mut tampered = receipt.clone();
    tampered.current_universe_digest =
        WorkspaceUniverseLockDigest::hash_canonical(&"tampered").unwrap();
    assert!(matches!(
        tampered.verify(),
        Err(InvalidWorkspaceDerivationReceipt::CoordinateMismatch)
    ));

    let semantically_false =
        WorkspaceDerivationReceiptV1::seal(WorkspaceDerivationReceiptPreimageV1 {
            prior_resolution_digest: receipt.prior_resolution_digest.clone(),
            current_resolution_digest: receipt.current_resolution_digest.clone(),
            prior_universe_digest: WorkspaceUniverseLockDigest::hash_canonical(
                &"different prior frontier",
            )
            .unwrap(),
            current_universe_digest: receipt.current_universe_digest.clone(),
            workspace_semantic_ir_digest: receipt.workspace_semantic_ir_digest.clone(),
            sources: receipt.sources.clone(),
            nodes: receipt.nodes.clone(),
            roots: receipt.roots.clone(),
            impacts: receipt.impacts.clone(),
        })
        .unwrap();
    semantically_false.verify().unwrap();
    assert!(matches!(
        current_admitted.verify_derivation_receipt(&prior_derivation, &semantically_false),
        Err(WorkspaceError::DerivationSemanticMismatch)
    ));

    let mut unused_sources = receipt.sources.clone();
    unused_sources.push(WorkspaceDerivationSourceV1 {
        source_id: id("unused", WorkspaceDerivationSourceId::parse),
        source_path: OwnedConfinedRelativePath::parse("unused.toml").unwrap(),
        source_digest: WorkspaceSourceContentDigest::hash_bytes(b"unused"),
        byte_len: 6,
    });
    unused_sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    assert!(matches!(
        WorkspaceDerivationReceiptV1::seal(WorkspaceDerivationReceiptPreimageV1 {
            prior_resolution_digest: receipt.prior_resolution_digest.clone(),
            current_resolution_digest: receipt.current_resolution_digest.clone(),
            prior_universe_digest: receipt.prior_universe_digest.clone(),
            current_universe_digest: receipt.current_universe_digest.clone(),
            workspace_semantic_ir_digest: receipt.workspace_semantic_ir_digest.clone(),
            sources: unused_sources,
            nodes: receipt.nodes.clone(),
            roots: receipt.roots.clone(),
            impacts: receipt.impacts.clone(),
        }),
        Err(InvalidWorkspaceDerivationReceipt::SourceAdmissionBijection)
    ));
    let mut path_substitution = receipt.sources.clone();
    path_substitution[0].source_path =
        OwnedConfinedRelativePath::parse("substituted.toml").unwrap();
    let mut digest_substitution = receipt.sources.clone();
    digest_substitution[0].source_digest = WorkspaceSourceContentDigest::hash_bytes(b"substituted");
    let mut length_substitution = receipt.sources.clone();
    length_substitution[0].byte_len += 1;
    for substituted_sources in [path_substitution, digest_substitution, length_substitution] {
        assert!(matches!(
            WorkspaceDerivationReceiptV1::seal(WorkspaceDerivationReceiptPreimageV1 {
                prior_resolution_digest: receipt.prior_resolution_digest.clone(),
                current_resolution_digest: receipt.current_resolution_digest.clone(),
                prior_universe_digest: receipt.prior_universe_digest.clone(),
                current_universe_digest: receipt.current_universe_digest.clone(),
                workspace_semantic_ir_digest: receipt.workspace_semantic_ir_digest.clone(),
                sources: substituted_sources,
                nodes: receipt.nodes.clone(),
                roots: receipt.roots.clone(),
                impacts: receipt.impacts.clone(),
            }),
            Err(InvalidWorkspaceDerivationReceipt::FactKind)
        ));
    }

    let declaration = receipt
        .nodes
        .iter()
        .find(|node| node.rule == WorkspaceDerivationRuleV1::DeclarationLinking)
        .unwrap();
    assert!(matches!(
        WorkspaceDerivationNodeV1::seal(
            WorkspaceDerivationOriginV1::Compilation,
            declaration.rule,
            declaration.subject.clone(),
            declaration.output.clone(),
            declaration.inputs.clone(),
        ),
        Err(InvalidWorkspaceDerivationReceipt::FactKind)
    ));
}

#[test]
fn removed_source_explanation_traverses_the_predecessor_proof() {
    let current_fixture = candidate_fixture();
    let mut prior_fixture = candidate_fixture();
    prior_fixture.components.push((
        "removed.toml".into(),
        WorkspaceLanguagePrimitivesV1 {
            schema: "pale/workspace-language-primitives.v1".into(),
            schema_version: 1,
            component_id: id("removed-source", WorkspaceComponentId::parse),
            module_id: prior_fixture.module.module.module_id.clone(),
            declarations: Vec::new(),
        },
    ));
    reseal_primary(&mut prior_fixture);

    let prior_dir = tempfile::tempdir().unwrap();
    write_fixture(prior_dir.path(), &prior_fixture);
    let prior_verified =
        universe::seal_and_verify_workspace(load_link_normalize(prior_dir.path()).unwrap())
            .unwrap();
    let prior_proof = WorkspaceAdmissionProofs::synthetic_for(&prior_verified);
    let prior = prior_verified.admit(&prior_proof).unwrap();

    let current_dir = tempfile::tempdir().unwrap();
    write_fixture(current_dir.path(), &current_fixture);
    let current_verified =
        universe::seal_and_verify_workspace(load_link_normalize(current_dir.path()).unwrap())
            .unwrap();
    let current_proof = WorkspaceAdmissionProofs::synthetic_for(&current_verified);
    let current = current_verified.admit(&current_proof).unwrap();
    let receipt = current.portable_derivation_receipt(&prior).unwrap();
    let root = receipt
        .roots
        .iter()
        .find(|root| {
            matches!(
                &root.subject,
                WorkspaceDerivationSubjectV1::Source { source_id }
                    if source_id.as_str() == "component:removed-source"
            )
        })
        .unwrap();
    assert!(matches!(
        &root.change,
        WorkspaceDerivationRootChangeV1::Removed { .. }
    ));
    let workspace_subject = receipt
        .impacts
        .iter()
        .find_map(|impact| {
            matches!(
                &impact.subject,
                WorkspaceDerivationSubjectV1::Workspace { .. }
            )
            .then_some(&impact.subject)
        })
        .unwrap();

    let explanation = current
        .explain_portable_derivation(&prior, &receipt, workspace_subject)
        .unwrap();
    let universe::WorkspaceDerivationExplanation::Affected {
        root: selected,
        frontier,
        path,
    } = explanation
    else {
        panic!("the workspace remains reachable from the removed source")
    };
    assert_eq!(selected.root_digest, root.root_digest);
    assert_eq!(
        frontier,
        universe::WorkspaceDerivationExplanationFrontier::Predecessor
    );
    assert_eq!(path.first().unwrap().subject, root.subject);
    assert_eq!(&path.last().unwrap().subject, workspace_subject);
}

#[test]
fn semantic_derivation_fan_in_admits_its_exact_bound_and_rejects_the_next() {
    assert_eq!(
        MAX_WORKSPACE_DERIVATION_INPUTS,
        4 + MAX_WORKSPACE_MODULES
            + MAX_WORKSPACE_DECLARATIONS
            + MAX_WORKSPACE_IMPORT_PATHS
            + MAX_WORKSPACE_EXTENSIONS
            + MAX_WORKSPACE_RESOURCES
    );
    assert_eq!(
        MAX_WORKSPACE_DERIVATION_ROOTS,
        2 * MAX_WORKSPACE_DERIVATION_SOURCES
    );
    let workspace_id = id("bounded", WorkspaceId::parse);
    let inputs = (0..MAX_WORKSPACE_DERIVATION_INPUTS)
        .map(|index| WorkspaceDerivationNodeDigest::hash_canonical(&index).unwrap())
        .collect::<Vec<_>>();
    let seal = |inputs| {
        WorkspaceDerivationNodeV1::seal(
            WorkspaceDerivationOriginV1::Compilation,
            WorkspaceDerivationRuleV1::SemanticIrSealing,
            WorkspaceDerivationSubjectV1::Workspace {
                workspace_id: workspace_id.clone(),
            },
            WorkspaceDerivationOutputV1::SemanticIr {
                workspace_semantic_ir_digest: WorkspaceSemanticIrDigest::hash_canonical(&"bounded")
                    .unwrap(),
            },
            inputs,
        )
    };
    seal(inputs.clone()).unwrap();
    let mut overflow = inputs;
    overflow.push(WorkspaceDerivationNodeDigest::hash_canonical(&"overflow").unwrap());
    assert!(matches!(
        seal(overflow),
        Err(InvalidWorkspaceDerivationReceipt::Limit {
            kind: "node input",
            observed,
            limit: MAX_WORKSPACE_DERIVATION_INPUTS,
        }) if observed == MAX_WORKSPACE_DERIVATION_INPUTS + 1
    ));
}

#[test]
fn export_projection_reuses_source_declaration_integrity() {
    let fixture = candidate_fixture();
    let mut preimage = fixture.module.module.clone();
    let projection = &mut preimage.exports[0].declarations[0].declaration;
    let duplicate = projection.dependencies()[0].clone();
    match projection {
        WorkspaceDeclarationV1::Trait { dependencies, .. }
        | WorkspaceDeclarationV1::Attribute { dependencies, .. }
        | WorkspaceDeclarationV1::Resource { dependencies, .. }
        | WorkspaceDeclarationV1::Relationship { dependencies, .. }
        | WorkspaceDeclarationV1::Policy { dependencies, .. } => {
            dependencies.push(duplicate);
        }
    }
    assert!(matches!(
        WorkspaceModuleV1::seal(preimage),
        Err(InvalidWorkspaceModule::Duplicate {
            kind: "declaration dependency",
            ..
        })
    ));
}

#[test]
fn derivation_distinguishes_module_policy_change_from_lock_only_change() {
    let mut fixture = candidate_fixture();
    let prior_dir = tempfile::tempdir().unwrap();
    write_fixture(prior_dir.path(), &fixture);
    let prior_verified =
        universe::seal_and_verify_workspace(load_link_normalize(prior_dir.path()).unwrap())
            .unwrap();
    let prior_proof = WorkspaceAdmissionProofs::synthetic_for(&prior_verified);
    let prior_admitted = prior_verified.admit(&prior_proof).unwrap();
    let prior_digest = prior_admitted.workspace_semantic_ir_digest().clone();
    let prior_derivation = prior_admitted.into_resident_parts().derivation;
    fixture.module.module.exports[0].ceiling.retention_seconds = 1_800;
    reseal_primary(&mut fixture);
    let policy_dir = tempfile::tempdir().unwrap();
    write_fixture(policy_dir.path(), &fixture);
    let policy_verified =
        universe::seal_and_verify_workspace(load_link_normalize(policy_dir.path()).unwrap())
            .unwrap();
    let policy_proof = WorkspaceAdmissionProofs::synthetic_for(&policy_verified);
    let policy_admitted = policy_verified.admit(&policy_proof).unwrap();
    let policy_receipt = policy_admitted
        .derivation_receipt(&prior_derivation)
        .unwrap();
    assert!(policy_receipt.roots.iter().any(|root| matches!(
        &root.subject,
        WorkspaceDerivationSubjectV1::Source { source_id }
            if source_id.as_str() == "module:candidate"
    )));
    assert!(!policy_receipt.roots.iter().any(|root| matches!(
        &root.subject,
        WorkspaceDerivationSubjectV1::Source { source_id }
            if source_id.as_str().starts_with("component:")
    )));

    let mut lock_only = candidate_fixture();
    let available = WorkspaceModuleV1::seal(WorkspaceModulePreimageV1 {
        module_id: module_id("available-unadopted"),
        release_id: id("release-1", WorkspaceReleaseId::parse),
        publisher: publisher("available-unadopted-publisher"),
        source_components: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        extensions: Vec::new(),
        resources: Vec::new(),
    })
    .unwrap();
    lock_only.additional_modules.push(available);
    reseal_locks(&mut lock_only);
    lock_only.universe = WorkspaceUniverseLockV1::seal(
        id("synthetic", WorkspaceId::parse),
        lock_only.resolution.resolution_digest.clone(),
        vec![WorkspaceAdoptedReleaseV1 {
            module_id: lock_only.module.module.module_id.clone(),
            release_id: lock_only.module.module.release_id.clone(),
            module_digest: lock_only.module.module_digest.clone(),
        }],
    )
    .unwrap();
    let lock_dir = tempfile::tempdir().unwrap();
    write_fixture(lock_dir.path(), &lock_only);
    let lock_verified =
        universe::seal_and_verify_workspace(load_link_normalize(lock_dir.path()).unwrap()).unwrap();
    let lock_proof = WorkspaceAdmissionProofs::synthetic_for(&lock_verified);
    let lock_admitted = lock_verified.admit(&lock_proof).unwrap();
    let lock_receipt = lock_admitted.derivation_receipt(&prior_derivation).unwrap();
    assert!(lock_receipt.roots.iter().any(|root| matches!(
        &root.subject,
        WorkspaceDerivationSubjectV1::Source { source_id }
            if source_id.as_str() == "resolution-lock"
    )));
    assert!(lock_receipt.roots.iter().any(|root| matches!(
        &root.subject,
        WorkspaceDerivationSubjectV1::Source { source_id }
            if source_id.as_str() == "universe-lock"
    )));
    assert!(!lock_receipt.roots.iter().any(|root| matches!(
        &root.subject,
        WorkspaceDerivationSubjectV1::Source { source_id }
            if source_id.as_str().starts_with("component:")
                || source_id.as_str().starts_with("module:")
    )));
    assert_ne!(
        lock_receipt.prior_resolution_digest,
        lock_receipt.current_resolution_digest
    );
    assert_ne!(
        lock_receipt.prior_universe_digest,
        lock_receipt.current_universe_digest
    );
    assert!(lock_receipt.impacts.iter().any(|impact| matches!(
        (&impact.subject, &impact.state),
        (
            WorkspaceDerivationSubjectV1::Workspace { .. },
            WorkspaceDerivationImpactStateV1::Affected { root_digests }
        ) if !root_digests.is_empty()
    )));
    assert_eq!(
        lock_receipt.workspace_semantic_ir_digest, prior_digest,
        "unadopted lock availability is evidence, not selected semantic identity"
    );
}

#[test]
fn empty_sovereign_adopter_has_no_module_or_declaration_leakage() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let normalized = load_link_normalize(&root).unwrap();
    let workspace = &normalized.linked.source.manifest.workspace;
    let topology = &workspace.desired_topology;

    assert!(topology.projects.is_empty());
    assert!(topology.execution_workspaces.is_empty());
    assert!(topology.repositories.is_empty());
    assert!(topology.execution_paths.is_empty());
    assert!(topology.project_repositories.is_empty());
    assert!(topology.project_execution_workspaces.is_empty());
    assert!(topology.execution_workspace_repositories.is_empty());
    assert!(workspace.module_manifests.is_empty());
    assert!(workspace.source_components.is_empty());
    assert!(workspace.responsibility_derivations.is_empty());
    assert!(workspace.organizational_translations.is_empty());
    assert!(workspace.experience_compositions.is_empty());

    assert!(normalized.linked.source.modules.is_empty());
    assert!(normalized.linked.source.components.is_empty());
    assert!(normalized
        .linked
        .source
        .responsibility_derivations
        .is_empty());
    assert!(normalized
        .linked
        .source
        .organizational_translation
        .is_none());
    assert!(normalized.linked.source.experience_composition.is_none());
    assert!(normalized
        .linked
        .source
        .resolution_lock
        .value
        .releases
        .is_empty());
    assert!(normalized
        .linked
        .source
        .universe_lock
        .value
        .adopted_releases
        .is_empty());
    assert!(normalized.linked.adopted_modules.is_empty());
    assert!(normalized.linked.declarations.is_empty());
    assert!(normalized.linked.dependency_analysis.components.is_empty());
    let convergence = &normalized.linked.dependency_analysis.witness;
    assert_eq!(convergence.declaration_count, 0);
    assert_eq!(convergence.direct_edge_count, 0);
    assert_eq!(convergence.forward_completed, 0);
    assert_eq!(convergence.reverse_assigned, 0);
    assert_eq!(convergence.component_count, 0);
    assert_eq!(convergence.condensation_edge_count, 0);
    assert!(normalized.linked.extensions.is_empty());
    assert!(normalized.linked.resolved_imports.is_empty());
    assert!(normalized.linked.access_entries.is_empty());
    assert!(normalized.linked.resources.is_empty());
    assert!(normalized.linked.package_releases.is_empty());
    assert!(normalized.linked.normative_pack_releases.is_empty());
    assert!(normalized
        .linked
        .responsibility_derivations
        .derivations
        .is_empty());
    assert!(normalized.linked.organizational_translation.is_none());
    assert_eq!(
        normalized.linked.experience_composition,
        ExperienceCompositionDeclarationsV1::empty()
    );
    assert!(normalized.policies.is_empty());
    assert!(normalized.exclusive_authority.is_empty());

    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let admitted = verified.admit_without_external_evidence().unwrap();
    assert!(!admitted.digest().as_str().is_empty());
    assert_eq!(admitted.admission_evidence_count(), 0);
}

#[test]
fn workspace_admission_denies_candidate_only_private_selection_identifiers() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("pale-workspace.toml"),
        r#"[workspace]
schema_version = 1
workspace_id = "candidate-deny"
distribution_lock = "distribution.lock.toml"
machine_binding_root = "machines"
desired_topology = { projects = [], execution_workspaces = [], repositories = [], execution_paths = [], project_repositories = [], project_execution_workspaces = [], execution_workspace_repositories = [] }
module_manifests = []
source_components = []
responsibility_derivations = ["responsibility-concept.toml"]
organizational_translations = []
experience_compositions = []
resolution_lock = "resolution.lock.toml"
universe_lock = "universe.lock.toml"
"#,
    )
    .unwrap();

    let error = source::parse_workspace_source(
        &pale_adapter_fs::LocalSourceFilesystem,
        &temp.path().join("pale-workspace.toml"),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::ResponsibilityDerivation(source)
            if matches!(
                source.as_ref(),
                crate::composition::ResponsibilityDerivationError::CandidateOnlySurface {
                    token: "concept",
                    ..
                }
            )
    ));
}

#[test]
fn safe_responsibility_filename_cannot_hide_a_candidate_only_admitted_identity() {
    fn rejected(
        responsibility: Option<&ResponsibilityDerivationV1>,
        organization: Option<&OrganizationalTranslationV1>,
    ) {
        let temp = tempfile::tempdir().unwrap();
        if let Some(value) = responsibility {
            fs::write(
                temp.path().join("responsibility.toml"),
                toml::to_string(value).unwrap(),
            )
            .unwrap();
        }
        if let Some(value) = organization {
            fs::write(
                temp.path().join("organization.toml"),
                toml::to_string(value).unwrap(),
            )
            .unwrap();
        }
        let responsibility_sources = if responsibility.is_some() {
            "[\"responsibility.toml\"]"
        } else {
            "[]"
        };
        let organization_sources = if organization.is_some() {
            "[\"organization.toml\"]"
        } else {
            "[]"
        };
        let manifest = format!(
            r#"[workspace]
schema_version = 1
workspace_id = "candidate-content-deny"
distribution_lock = "distribution.lock.toml"
machine_binding_root = "machines"
desired_topology = {{ projects = [], execution_workspaces = [], repositories = [], execution_paths = [], project_repositories = [], project_execution_workspaces = [], execution_workspace_repositories = [] }}
module_manifests = []
source_components = []
responsibility_derivations = {responsibility_sources}
organizational_translations = {organization_sources}
experience_compositions = []
resolution_lock = "resolution.lock.toml"
universe_lock = "universe.lock.toml"
"#
        );
        fs::write(temp.path().join("pale-workspace.toml"), manifest).unwrap();
        let error = source::parse_workspace_source(
            &pale_adapter_fs::LocalSourceFilesystem,
            &temp.path().join("pale-workspace.toml"),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            WorkspaceError::ResponsibilityDerivation(source)
                if matches!(
                    source.as_ref(),
                    crate::composition::ResponsibilityDerivationError::CandidateOnlySurface {
                        token: "concept",
                        ..
                    }
                )
        ));
    }

    type B3Mutation = fn(&mut ResponsibilityDerivationV1);
    let b3_cases: &[(&str, B3Mutation)] = &[
        ("derivation", |v| {
            v.derivation_id = ResponsibilityDerivationId::parse("concept").unwrap()
        }),
        ("provenance", |v| {
            v.provenance.source_id = WorkspaceDerivationSourceId::parse("concept").unwrap()
        }),
        ("responsibility", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } = &mut v.responsibility {
                value.verb = ResponsibilityVerbId::parse("concept").unwrap()
            }
        }),
        ("native-owner", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } = &mut v.native_owner {
                value.owner = WorkspaceResourceId::parse("concept").unwrap()
            }
        }),
        ("trigger", |v| {
            if let AttributableAnswerV1::Supplied {
                value: ResponsibilityTriggerV1::BoundedRequest { request_contract },
                ..
            } = &mut v.trigger
            {
                *request_contract = WorkspaceResourceId::parse("concept").unwrap()
            }
        }),
        ("placement", |v| {
            if let AttributableAnswerV1::Supplied {
                value: ResponsibilityPlacementV1::VerticalLocal { scope },
                ..
            } = &mut v.placement
            {
                *scope = WorkspaceResourceId::parse("concept").unwrap()
            }
        }),
        ("structural", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } =
                &mut v.structural_objects[1].selection
            {
                *value = WorkspaceResourceId::parse("concept").unwrap()
            }
        }),
        ("framer", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } = &mut v.request_framer {
                value.actor = WorkspaceResourceId::parse("concept").unwrap()
            }
        }),
        ("evidence", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } = &mut v.provider_completion {
                value.requirement.evidence_ref = WorkspaceResourceId::parse("concept").unwrap()
            }
        }),
        ("module", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } = &mut v.publisher_module {
                value.module_id = WorkspaceModuleId::parse("concept").unwrap()
            }
        }),
        ("ceiling", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } = &mut v.ceilings {
                value.policy.rights.permitted[0] = WorkspaceRightId::parse("concept").unwrap()
            }
        }),
        ("incumbent", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } = &mut v.incumbent {
                value.mechanism = WorkspaceResourceId::parse("concept").unwrap()
            }
        }),
        ("readiness", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } = &mut v.readiness {
                value.operation_contract = WorkspaceResourceId::parse("concept").unwrap()
            }
        }),
        ("direct-path", |v| {
            if let AttributableAnswerV1::Supplied { value, .. } = &mut v.direct_path {
                value.path = WorkspaceResourceId::parse("concept").unwrap()
            }
        }),
        ("residence", |v| {
            v.residence[0].resource = WorkspaceResourceId::parse("concept").unwrap()
        }),
        ("projection", |v| {
            v.native_projections[0].projection_id =
                ResponsibilityProjectionId::parse("concept").unwrap()
        }),
        ("genealogy", |v| {
            v.genealogy_links.push(ResponsibilityGenealogyLinkV1 {
                link_id: ResponsibilityGenealogyLinkId::parse("concept").unwrap(),
                responsibilities: vec![v.derivation_id.clone()],
                projections: Vec::new(),
                occurrences: vec![ResponsibilityOccurrenceId::parse("occurrence").unwrap()],
            })
        }),
    ];
    for (family, mutate) in b3_cases {
        let mut value = responsibility_fixtures::responsibility_derivation("safe-file", 1, false);
        mutate(&mut value);
        rejected(Some(&value), None);
        assert!(!family.is_empty());
    }

    type B4Mutation = fn(&mut OrganizationalTranslationV1);
    let b4_cases: &[(&str, B4Mutation)] = &[
        ("scope", |v| {
            v.scopes[0].scope.scope_id = LogicalScopeId::parse("concept").unwrap()
        }),
        ("authority-axes", |v| {
            v.scopes[0].axes.owner = WorkspaceResourceId::parse("concept").unwrap()
        }),
        ("organization", |v| {
            v.organizations[0].organization.organization_id =
                LogicalOrganizationId::parse("concept").unwrap()
        }),
        ("group", |v| {
            v.groups[0].group.group_id = OrganizationalGroupId::parse("concept").unwrap()
        }),
        ("tree", |v| {
            v.discipline_trees[0].tree.tree_id = DisciplineTreeId::parse("concept").unwrap()
        }),
        ("role", |v| {
            v.roles[0].role.role_id = OrganizationalRoleId::parse("concept").unwrap()
        }),
        ("occupant-binding", |v| {
            v.occupant_bindings[0].occupant = OrganizationalOccupantId::parse("concept").unwrap()
        }),
        ("provider-binding", |v| {
            v.provider_bindings[0].provider = OrganizationalProviderId::parse("concept").unwrap()
        }),
        ("communication-binding", |v| {
            v.communication_actor_bindings[0].handle =
                CommunicationActorHandleId::parse("concept").unwrap()
        }),
        ("leadership", |v| {
            v.leadership[0].topic = LeadershipTopicId::parse("concept").unwrap()
        }),
        ("relation", |v| {
            v.relations[0].relation_id = OrganizationalRelationId::parse("concept").unwrap()
        }),
        ("object", |v| {
            v.objects[0].object_id = OrganizationalObjectId::parse("concept").unwrap()
        }),
        ("population-rule", |v| {
            v.population_rules[0].object_class =
                OrganizationalObjectClassId::parse("concept").unwrap()
        }),
        ("group-lens", |v| {
            v.group_lenses[0].profile_id = GroupLensProfileId::parse("concept").unwrap()
        }),
        ("service", |v| {
            v.services[0].contract_id = DirectionalServiceContractId::parse("concept").unwrap()
        }),
        ("service-assignment", |v| {
            v.services[0].assignments[0].account = ServiceAccountId::parse("concept").unwrap()
        }),
        ("service-policy", |v| {
            v.services[0].ceiling.authorities.permitted[0] =
                WorkspaceAuthorityScopeId::parse("concept").unwrap()
        }),
        ("native-projection", |v| {
            v.native_projections[0].projection_id =
                OrganizationalProjectionId::parse("concept").unwrap()
        }),
        ("inspection", |v| {
            v.inspection.as_mut().unwrap().inspection_id =
                OrganizationalInspectionId::parse("concept").unwrap()
        }),
    ];
    for (family, mutate) in b4_cases {
        let mut value = organization_fixtures::organizational_translation();
        mutate(&mut value);
        rejected(None, Some(&value));
        assert!(!family.is_empty());
    }
}

#[test]
fn directional_responsibility_resolves_exactly_one_selected_service_contract() {
    let responsibilities = crate::composition::lower_responsibility_derivations(vec![
        responsibility_fixtures::responsibility_derivation("alpha", 1, true),
    ])
    .unwrap();
    let organization = crate::composition::lower_organizational_translation(
        organization_fixtures::organizational_translation(),
    )
    .unwrap();
    assert!(
        link::validate_directional_service_closure(&responsibilities, Some(&organization)).is_ok()
    );

    let mutate_contract =
        |mut responsibilities: ResponsibilityDerivationIrV1,
         mutate: fn(&mut DirectionalServiceContractRefV1)| {
            let AttributableAnswerV1::Supplied {
                value: ResponsibilityPlacementV1::DirectionalService { service_contract },
                ..
            } = &mut responsibilities.derivations[0].placement
            else {
                panic!("directional fixture carries a supplied service contract")
            };
            mutate(service_contract);
            responsibilities
        };
    let provider_substitution = mutate_contract(responsibilities.clone(), |contract| {
        contract.provider_scope = organization_fixtures::scope("gamma", "scope");
    });
    assert!(matches!(
        link::validate_directional_service_closure(&provider_substitution, Some(&organization)),
        Err(WorkspaceError::MissingDirectionalService { .. })
    ));
    let consumer_substitution = mutate_contract(responsibilities.clone(), |contract| {
        contract.consumer_scope = organization_fixtures::scope("gamma", "scope");
    });
    assert!(matches!(
        link::validate_directional_service_closure(&consumer_substitution, Some(&organization)),
        Err(WorkspaceError::MissingDirectionalService { .. })
    ));
    let reversed = mutate_contract(responsibilities.clone(), |contract| {
        std::mem::swap(&mut contract.provider_scope, &mut contract.consumer_scope);
    });
    assert!(matches!(
        link::validate_directional_service_closure(&reversed, Some(&organization)),
        Err(WorkspaceError::MissingDirectionalService { .. })
    ));
    assert!(matches!(
        link::validate_directional_service_closure(&responsibilities, None),
        Err(WorkspaceError::MissingDirectionalService { .. })
    ));
    let unavailable = crate::composition::lower_responsibility_derivations(vec![
        responsibility_fixtures::responsibility_derivation("gamma", 1, true),
    ])
    .unwrap();
    assert!(matches!(
        link::validate_directional_service_closure(&unavailable, Some(&organization)),
        Err(WorkspaceError::MissingDirectionalService { contract, .. })
            if contract.as_ref() == &responsibility_fixtures::directional_service_contract("gamma")
    ));

    let local = crate::composition::lower_responsibility_derivations(vec![
        responsibility_fixtures::responsibility_derivation("local", 0, false),
    ])
    .unwrap();
    assert!(link::validate_directional_service_closure(&local, None).is_ok());
}

fn module_reference_fixture() -> CandidateFixture {
    let mut fixture = candidate_fixture();
    add_consumer(&mut fixture, WorkspacePropagationV1::Transitive);
    let mut responsibility =
        responsibility_fixtures::responsibility_derivation("module-reference", 0, false);
    let AttributableAnswerV1::Supplied {
        value: publisher, ..
    } = &mut responsibility.publisher_module
    else {
        panic!("responsibility fixture supplies a publisher module")
    };
    publisher.module_id = fixture.module.module.module_id.clone();
    publisher.release_id = fixture.module.module.release_id.clone();
    publisher.export_id = fixture.module.module.exports[0].export_id.clone();

    let AttributableAnswerV1::Supplied { value: adopter, .. } = &mut responsibility.adopter_module
    else {
        panic!("responsibility fixture supplies an adopter module")
    };
    let import = &fixture
        .consumer
        .as_ref()
        .expect("consumer module")
        .module
        .imports[0];
    adopter.module_id = module_id("consumer");
    adopter.imported_module_id = import.module_id.clone();
    adopter.imported_release_id = import.release_id.clone();
    adopter.imported_export_id = import.paths[0].export_id.clone();
    adopter.local_namespace = import.paths[0].local_namespace.clone();
    fixture
        .responsibilities
        .push(("responsibility.toml".into(), responsibility));
    fixture
}

fn assert_responsibility_reference_rejected(
    mutate: impl FnOnce(&mut ResponsibilityDerivationV1),
    expected_fragment: &str,
) {
    let mut fixture = module_reference_fixture();
    mutate(&mut fixture.responsibilities[0].1);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    assert!(matches!(
        load_link_normalize(temp.path()),
        Err(WorkspaceError::NamespaceResolution(message))
            if message.contains("responsibility `responsibility.module-reference`")
                && message.contains(expected_fragment)
    ));
}

#[test]
fn responsibility_module_references_resolve_against_the_selected_closure() {
    let fixture = module_reference_fixture();
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let normalized = load_link_normalize(temp.path()).unwrap();
    assert_eq!(
        normalized
            .linked
            .responsibility_derivations
            .derivations
            .iter()
            .map(|derivation| derivation.derivation_id.as_str())
            .collect::<Vec<_>>(),
        ["responsibility.module-reference"]
    );
}

#[test]
fn responsibility_publisher_rejects_an_absent_adopted_module() {
    assert_responsibility_reference_rejected(
        |responsibility| {
            let AttributableAnswerV1::Supplied { value, .. } = &mut responsibility.publisher_module
            else {
                unreachable!()
            };
            value.module_id = module_id("absent");
        },
        "publisher module `absent`",
    );
}

#[test]
fn responsibility_publisher_rejects_a_wrong_release() {
    assert_responsibility_reference_rejected(
        |responsibility| {
            let AttributableAnswerV1::Supplied { value, .. } = &mut responsibility.publisher_module
            else {
                unreachable!()
            };
            value.release_id = id("release-2", WorkspaceReleaseId::parse);
        },
        "release `release-2`",
    );
}

#[test]
fn responsibility_publisher_rejects_an_absent_export() {
    assert_responsibility_reference_rejected(
        |responsibility| {
            let AttributableAnswerV1::Supplied { value, .. } = &mut responsibility.publisher_module
            else {
                unreachable!()
            };
            value.export_id = id("absent", WorkspaceExportId::parse);
        },
        "export `absent`",
    );
}

#[test]
fn responsibility_adopter_requires_an_exact_resolved_import() {
    type Mutation = fn(&mut ResponsibilityAdopterModuleV1);
    let cases: &[(Mutation, &str)] = &[
        (
            |value| value.module_id = module_id("candidate"),
            "adopter module `candidate`",
        ),
        (
            |value| value.imported_module_id = module_id("consumer"),
            "import of `consumer`",
        ),
        (
            |value| value.imported_release_id = id("release-2", WorkspaceReleaseId::parse),
            "release `release-2`",
        ),
        (
            |value| value.imported_export_id = id("absent", WorkspaceExportId::parse),
            "export `absent`",
        ),
    ];
    for (mutate, expected_fragment) in cases {
        assert_responsibility_reference_rejected(
            |responsibility| {
                let AttributableAnswerV1::Supplied { value, .. } =
                    &mut responsibility.adopter_module
                else {
                    unreachable!()
                };
                mutate(value);
            },
            expected_fragment,
        );
    }
}

#[test]
fn responsibility_adopter_requires_the_exact_local_namespace() {
    assert_responsibility_reference_rejected(
        |responsibility| {
            let AttributableAnswerV1::Supplied { value, .. } = &mut responsibility.adopter_module
            else {
                unreachable!()
            };
            value.local_namespace = id("absent", WorkspaceNamespaceId::parse);
        },
        "namespace `absent`",
    );
}

#[test]
fn resolved_but_unadopted_module_does_not_enter_the_semantic_universe() {
    use crate::workspace::{
        AuthoringInstancePathSegment::*, AuthoringOccurrenceRole, PortableWorkspaceSourceSubject,
        PortableWorkspaceSubjectOccurrences,
    };

    let mut fixture = candidate_fixture();
    fixture
        .module
        .module
        .resources
        .push(WorkspaceResourceSelectionV1::Package {
            resource_id: id("unadopted-package", WorkspaceResourceId::parse),
            release: PackageReleaseV1 {
                id: PackageIdV1::parse("unadopted").unwrap(),
                version: PackageVersionV1::parse("1.0.0").unwrap(),
                content_digest: PackageDigest::hash_bytes(b"first"),
            },
        });
    reseal_primary(&mut fixture);
    fixture.universe = WorkspaceUniverseLockV1::seal(
        id("synthetic", WorkspaceId::parse),
        fixture.resolution.resolution_digest.clone(),
        Vec::new(),
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let portable = load_portable_fixture(temp.path());
    let source_path = [Field("declarations".into()), Index(0), Field("id".into())];
    let PortableWorkspaceSourceSubject::Unselected {
        occurrence,
        subject,
    } = portable.source_subject("alpha.toml", &source_path)
    else {
        panic!("unadopted definition must remain mapped but unselected");
    };
    assert_eq!(occurrence.role(), AuthoringOccurrenceRole::Definition);
    let PortableWorkspaceSubjectOccurrences::Unselected(page) =
        portable.subject_occurrences(subject, None, 256, 256)
    else {
        panic!("source capture must not establish selected membership");
    };
    assert!(page.is_complete());
    assert!(page.occurrences().contains(&occurrence));
    assert!(matches!(
        portable.source_subject(
            "alpha.toml",
            &[
                Field("declarations".into()),
                Index(0),
                Field("namespace".into()),
            ]
        ),
        PortableWorkspaceSourceSubject::NoDenotation { .. }
    ));
    assert!(matches!(
        portable.source_subject(
            "alpha.toml",
            &[
                Field("declarations".into()),
                Index(0),
                Field("dependencies".into()),
                Index(0),
                Field("target".into()),
                Field("declaration_id".into()),
            ]
        ),
        PortableWorkspaceSourceSubject::NoOccurrence
    ));
    let (normalized, authoring) = load_with_authoring(temp.path()).unwrap();
    let definition = authoring
        .occurrence(
            "alpha.toml",
            &[Field("declarations".into()), Index(0), Field("id".into())],
        )
        .expect("captured unadopted source remains authorable");
    assert_eq!(definition.role(), AuthoringOccurrenceRole::Definition);
    assert_eq!(
        authoring
            .occurrence(
                "alpha.toml",
                &[
                    Field("declarations".into()),
                    Index(0),
                    Field("namespace".into()),
                ]
            )
            .unwrap()
            .role(),
        AuthoringOccurrenceRole::Definition,
    );
    assert_eq!(
        authoring.definition(definition.identity()),
        Some(definition)
    );
    assert!(authoring
        .occurrence(
            "alpha.toml",
            &[
                Field("declarations".into()),
                Index(0),
                Field("dependencies".into()),
                Index(0),
                Field("target".into()),
                Field("declaration_id".into()),
            ]
        )
        .is_none());
    assert!(normalized.linked.adopted_modules.is_empty());
    assert!(normalized.linked.declarations.is_empty());
    assert!(normalized.linked.package_releases.is_empty());
    let verified = universe::seal_and_verify_workspace(normalized).unwrap();
    let original_digest = verified.digest().clone();
    let proof = WorkspaceAdmissionProofs::synthetic_for(&verified);
    verified.admit(&proof).unwrap();

    if let WorkspaceResourceSelectionV1::Package { release, .. } =
        &mut fixture.module.module.resources[0]
    {
        release.content_digest = PackageDigest::hash_bytes(b"second");
    }
    reseal_primary(&mut fixture);
    fixture.universe = WorkspaceUniverseLockV1::seal(
        id("synthetic", WorkspaceId::parse),
        fixture.resolution.resolution_digest.clone(),
        Vec::new(),
    )
    .unwrap();
    let changed = tempfile::tempdir().unwrap();
    write_fixture(changed.path(), &fixture);
    let changed =
        universe::seal_and_verify_workspace(load_with_authoring(changed.path()).unwrap().0)
            .unwrap();
    assert_eq!(original_digest, *changed.digest());
}

#[test]
fn workspace_graph_rejects_the_first_declaration_beyond_its_owned_bound() {
    let mut fixture = candidate_fixture();
    for (_, component) in &mut fixture.components {
        component.declarations.clear();
    }
    fixture.components[0].1.declarations = (0..513)
        .map(|index| WorkspaceDeclarationV1::Trait {
            id: id(&format!("d{index}"), WorkspaceDeclarationId::parse),
            namespace: id("bounded", WorkspaceNamespaceId::parse),
            dependencies: Vec::new(),
        })
        .collect();
    fixture.components[1].1.declarations = (0..512)
        .map(|index| WorkspaceDeclarationV1::Trait {
            id: id(&format!("e{index}"), WorkspaceDeclarationId::parse),
            namespace: id("bounded", WorkspaceNamespaceId::parse),
            dependencies: Vec::new(),
        })
        .collect();
    for (_, component) in &mut fixture.components {
        component
            .declarations
            .sort_by_key(WorkspaceDeclarationV1::declaration_ref);
    }
    fixture.module.module.exports.clear();
    reseal_primary(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    assert!(matches!(
        load_link_normalize(temp.path()),
        Err(WorkspaceError::WorkspaceGraphLimit {
            observed,
            limit: MAX_WORKSPACE_DECLARATIONS,
            ..
        }) if observed == MAX_WORKSPACE_DECLARATIONS + 1
    ));
}

#[test]
fn dense_positive_scc_has_counted_finite_passes_and_is_permutation_invariant() {
    let mut fixture = candidate_fixture();
    let refs = (0..64)
        .map(|index| WorkspaceDeclarationRefV1 {
            namespace: id("dense", WorkspaceNamespaceId::parse),
            declaration_id: id(&format!("d{index:02}"), WorkspaceDeclarationId::parse),
        })
        .collect::<Vec<_>>();
    fixture.components[0].1.declarations = refs
        .iter()
        .map(|declaration_ref| WorkspaceDeclarationV1::Trait {
            id: declaration_ref.declaration_id.clone(),
            namespace: declaration_ref.namespace.clone(),
            dependencies: refs
                .iter()
                .cloned()
                .map(|target| WorkspaceDeclarationDependencyV1 {
                    target,
                    kind: DeclarationDependencyKindV1::Positive,
                })
                .collect(),
        })
        .collect();
    fixture.components[1].1.declarations.clear();
    fixture.module.module.exports.clear();
    reseal_primary(&mut fixture);
    let first = tempfile::tempdir().unwrap();
    write_fixture(first.path(), &fixture);
    let first_normalized = load_link_normalize(first.path()).unwrap();
    let witness = &first_normalized.linked.dependency_analysis.witness;
    assert_eq!(witness.declaration_count, 64);
    assert_eq!(witness.direct_edge_count, 64 * 64);
    assert_eq!(witness.forward_completed, 64);
    assert_eq!(witness.reverse_assigned, 64);
    assert_eq!(witness.component_count, 1);
    assert_eq!(witness.condensation_edge_count, 0);
    let first_digest = universe::seal_and_verify_workspace(first_normalized)
        .unwrap()
        .digest()
        .clone();

    fixture.components.reverse();
    reseal_primary(&mut fixture);
    let second = tempfile::tempdir().unwrap();
    write_fixture(second.path(), &fixture);
    let second_digest =
        universe::seal_and_verify_workspace(load_link_normalize(second.path()).unwrap())
            .unwrap()
            .digest()
            .clone();
    assert_eq!(first_digest, second_digest);
}

#[test]
fn workspace_graph_rejects_the_first_dense_scc_beyond_its_edge_bound() {
    let mut fixture = candidate_fixture();
    let refs = (0..65)
        .map(|index| WorkspaceDeclarationRefV1 {
            namespace: id("dense", WorkspaceNamespaceId::parse),
            declaration_id: id(&format!("d{index:02}"), WorkspaceDeclarationId::parse),
        })
        .collect::<Vec<_>>();
    fixture.components[0].1.declarations = refs
        .iter()
        .map(|declaration_ref| WorkspaceDeclarationV1::Trait {
            id: declaration_ref.declaration_id.clone(),
            namespace: declaration_ref.namespace.clone(),
            dependencies: refs
                .iter()
                .cloned()
                .map(|target| WorkspaceDeclarationDependencyV1 {
                    target,
                    kind: DeclarationDependencyKindV1::Positive,
                })
                .collect(),
        })
        .collect();
    fixture.components[1].1.declarations.clear();
    fixture.module.module.exports.clear();
    reseal_primary(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    assert!(matches!(
        load_link_normalize(temp.path()),
        Err(WorkspaceError::WorkspaceGraphLimit {
            observed: 4_225,
            limit: MAX_WORKSPACE_DECLARATION_EDGES,
            ..
        })
    ));
}

#[test]
fn logical_resources_are_preserved_without_resolving_native_truth() {
    let mut fixture = candidate_fixture();
    fixture.module.module.resources.extend([
        WorkspaceResourceSelectionV1::NativeApplication {
            resource_id: id("logical-calendar", WorkspaceResourceId::parse),
            authority_resolver: id("calendar-owner", WorkspaceResourceId::parse),
        },
        WorkspaceResourceSelectionV1::Package {
            resource_id: id("package-engineering", WorkspaceResourceId::parse),
            release: PackageReleaseV1 {
                id: PackageIdV1::parse("engineering").unwrap(),
                version: PackageVersionV1::parse("1.0.0").unwrap(),
                content_digest: PackageDigest::hash_bytes(b"package release"),
            },
        },
        WorkspaceResourceSelectionV1::Package {
            resource_id: id("package-engineering-duplicate", WorkspaceResourceId::parse),
            release: PackageReleaseV1 {
                id: PackageIdV1::parse("engineering").unwrap(),
                version: PackageVersionV1::parse("1.0.0").unwrap(),
                content_digest: PackageDigest::hash_bytes(b"package release"),
            },
        },
        WorkspaceResourceSelectionV1::NormativePack {
            resource_id: id("pack-legal", WorkspaceResourceId::parse),
            release: NormativePackReleaseV1 {
                id: NormativePackIdV1::parse("engineering").unwrap(),
                version: NormativePackVersionV1::parse("2026-08-28").unwrap(),
                attestor: NormativePackAttestorV1::parse("pack-owner").unwrap(),
                content_digest: NormativePackDigest::hash_bytes(b"normative pack release"),
            },
        },
        WorkspaceResourceSelectionV1::NormativePack {
            resource_id: id("pack-legal-second-attestor", WorkspaceResourceId::parse),
            release: NormativePackReleaseV1 {
                id: NormativePackIdV1::parse("engineering").unwrap(),
                version: NormativePackVersionV1::parse("2026-08-28").unwrap(),
                attestor: NormativePackAttestorV1::parse("pack-owner-b").unwrap(),
                content_digest: NormativePackDigest::hash_bytes(b"other attested bytes"),
            },
        },
    ]);
    reseal_primary(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let normalized = load_link_normalize(temp.path()).unwrap();
    assert_eq!(normalized.linked.resources.len(), 5);
    assert_eq!(normalized.linked.package_releases.len(), 1);
    assert_eq!(normalized.linked.normative_pack_releases.len(), 2);
    assert_eq!(
        normalized.linked.resources[0].1.resource_id().as_str(),
        "logical-calendar"
    );
    assert!(normalized.linked.resources.iter().any(|(_, selection)| matches!(
        selection,
        WorkspaceResourceSelectionV1::Package { release, .. }
            if release.version.as_str() == "1.0.0"
                && release.content_digest.as_str() == PackageDigest::hash_bytes(b"package release").as_str()
    )));
    assert!(
        serde_json::from_value::<WorkspaceResourceSelectionV1>(serde_json::json!({
            "kind": "package",
            "resource_id": "missing-pin",
            "release": {
                "id": "engineering",
                "version": "1.0.0"
            }
        }))
        .is_err()
    );
    fixture
        .module
        .module
        .resources
        .push(WorkspaceResourceSelectionV1::NormativePack {
            resource_id: id("pack-legal-conflict", WorkspaceResourceId::parse),
            release: NormativePackReleaseV1 {
                id: NormativePackIdV1::parse("engineering").unwrap(),
                version: NormativePackVersionV1::parse("2026-08-29").unwrap(),
                attestor: NormativePackAttestorV1::parse("pack-owner").unwrap(),
                content_digest: NormativePackDigest::hash_bytes(b"conflicting pack"),
            },
        });
    reseal_primary(&mut fixture);
    let pack_conflict = tempfile::tempdir().unwrap();
    write_fixture(pack_conflict.path(), &fixture);
    assert!(matches!(
        load_link_normalize(pack_conflict.path()),
        Err(WorkspaceError::ReleaseClosure(
            ReleaseClosureErrorV1::ConflictingNormativePackRelease { .. }
        ))
    ));
    fixture.module.module.resources.pop();
    fixture
        .module
        .module
        .resources
        .push(WorkspaceResourceSelectionV1::Package {
            resource_id: id("package-engineering-conflict", WorkspaceResourceId::parse),
            release: PackageReleaseV1 {
                id: PackageIdV1::parse("engineering").unwrap(),
                version: PackageVersionV1::parse("2.0.0").unwrap(),
                content_digest: PackageDigest::hash_bytes(b"conflicting release"),
            },
        });
    reseal_primary(&mut fixture);
    let conflict = tempfile::tempdir().unwrap();
    write_fixture(conflict.path(), &fixture);
    assert!(matches!(
        load_link_normalize(conflict.path()),
        Err(WorkspaceError::ReleaseClosure(
            ReleaseClosureErrorV1::ConflictingPackageRelease { .. }
        ))
    ));
}

#[test]
fn package_and_attestor_scoped_pack_release_closures_do_not_collapse() {
    let package = PackageReleaseV1 {
        id: PackageIdV1::parse("shared").unwrap(),
        version: PackageVersionV1::parse("1.0.0").unwrap(),
        content_digest: PackageDigest::hash_bytes(b"package-a"),
    };
    let pack = |attestor: &str, bytes: &[u8]| NormativePackReleaseV1 {
        id: NormativePackIdV1::parse("shared").unwrap(),
        attestor: NormativePackAttestorV1::parse(attestor).unwrap(),
        version: NormativePackVersionV1::parse("1.0.0").unwrap(),
        content_digest: NormativePackDigest::hash_bytes(bytes),
    };
    let canonical = canonicalize_release_closures(
        vec![package.clone(), package.clone()],
        vec![pack("owner-a", b"pack-a"), pack("owner-b", b"pack-b")],
    )
    .unwrap();
    assert_eq!(canonical.packages, vec![package.clone()]);
    assert_eq!(canonical.normative_packs.len(), 2);

    let mut conflicting_package = package.clone();
    conflicting_package.version = PackageVersionV1::parse("2.0.0").unwrap();
    assert!(matches!(
        canonicalize_release_closures(vec![package, conflicting_package], Vec::new()),
        Err(ReleaseClosureErrorV1::ConflictingPackageRelease { .. })
    ));
    let package_a = PackageReleaseV1 {
        id: PackageIdV1::parse("digest-conflict").unwrap(),
        version: PackageVersionV1::parse("1.0.0").unwrap(),
        content_digest: PackageDigest::hash_bytes(b"a"),
    };
    let mut package_b = package_a.clone();
    package_b.content_digest = PackageDigest::hash_bytes(b"b");
    assert!(matches!(
        canonicalize_release_closures(vec![package_a, package_b], Vec::new()),
        Err(ReleaseClosureErrorV1::ConflictingPackageRelease { .. })
    ));
    assert!(matches!(
        canonicalize_release_closures(
            Vec::new(),
            vec![pack("owner-a", b"pack-a"), pack("owner-a", b"different")],
        ),
        Err(ReleaseClosureErrorV1::ConflictingNormativePackRelease { .. })
    ));
    let mut pack_version_conflict = pack("owner-a", b"pack-a");
    pack_version_conflict.version = NormativePackVersionV1::parse("2.0.0").unwrap();
    assert!(matches!(
        canonicalize_release_closures(
            Vec::new(),
            vec![pack("owner-a", b"pack-a"), pack_version_conflict],
        ),
        Err(ReleaseClosureErrorV1::ConflictingNormativePackRelease { .. })
    ));
}

#[test]
fn selected_package_and_pack_releases_move_only_their_typed_semantic_closure() {
    let mut fixture = candidate_fixture();
    fixture.module.module.resources = vec![
        WorkspaceResourceSelectionV1::Package {
            resource_id: id("package", WorkspaceResourceId::parse),
            release: PackageReleaseV1 {
                id: PackageIdV1::parse("shared").unwrap(),
                version: PackageVersionV1::parse("1.0.0").unwrap(),
                content_digest: PackageDigest::hash_bytes(b"package-a"),
            },
        },
        WorkspaceResourceSelectionV1::NormativePack {
            resource_id: id("pack", WorkspaceResourceId::parse),
            release: NormativePackReleaseV1 {
                id: NormativePackIdV1::parse("shared").unwrap(),
                attestor: NormativePackAttestorV1::parse("owner").unwrap(),
                version: NormativePackVersionV1::parse("1.0.0").unwrap(),
                content_digest: NormativePackDigest::hash_bytes(b"pack-a"),
            },
        },
    ];
    reseal_primary(&mut fixture);
    let compile = |fixture: &CandidateFixture| {
        let dir = tempfile::tempdir().unwrap();
        write_fixture(dir.path(), fixture);
        let normalized = load_link_normalize(dir.path()).unwrap();
        let packages = normalized.linked.package_releases.clone();
        let packs = normalized.linked.normative_pack_releases.clone();
        let digest = universe::seal_and_verify_workspace(normalized)
            .unwrap()
            .digest()
            .clone();
        (digest, packages, packs)
    };
    let (base, base_packages, base_packs) = compile(&fixture);
    let package_release = fixture
        .module
        .module
        .resources
        .iter_mut()
        .find_map(|resource| match resource {
            WorkspaceResourceSelectionV1::Package { release, .. } => Some(release),
            _ => None,
        })
        .expect("fixture retains its Package release selection");
    package_release.content_digest = PackageDigest::hash_bytes(b"package-b");
    reseal_primary(&mut fixture);
    let (package_changed, changed_packages, unchanged_packs) = compile(&fixture);
    assert_ne!(base, package_changed);
    assert_ne!(base_packages, changed_packages);
    assert_eq!(base_packs, unchanged_packs);

    let pack_release = fixture
        .module
        .module
        .resources
        .iter_mut()
        .find_map(|resource| match resource {
            WorkspaceResourceSelectionV1::NormativePack { release, .. } => Some(release),
            _ => None,
        })
        .expect("fixture retains its normative Pack release selection");
    pack_release.content_digest = NormativePackDigest::hash_bytes(b"pack-b");
    reseal_primary(&mut fixture);
    let (pack_changed, packages_after_pack, packs_after_pack) = compile(&fixture);
    assert_ne!(package_changed, pack_changed);
    assert_eq!(changed_packages, packages_after_pack);
    assert_ne!(unchanged_packs, packs_after_pack);
}

#[test]
fn prevent_keeps_current_local_use_lawful_but_denies_onward_reexport() {
    let mut local_use = candidate_fixture();
    local_use.module.module.exports[0].propagation = WorkspacePropagationV1::Prevent;
    reseal_primary(&mut local_use);
    add_consumer(&mut local_use, WorkspacePropagationV1::Prevent);
    add_consumer_dependency(&mut local_use);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &local_use);
    assert!(load_link_normalize(temp.path()).is_ok());

    let onward_projection = projection(
        &local_use.module.module.module_id,
        "upstream",
        &local_use.components[0].1.declarations[0],
    );
    local_use
        .consumer
        .as_mut()
        .unwrap()
        .module
        .exports
        .push(WorkspaceExportV1 {
            export_id: id("onward", WorkspaceExportId::parse),
            declarations: vec![onward_projection],
            visibility: WorkspaceVisibilityV1::Public,
            delivery: WorkspaceDeliveryV1::Interface,
            propagation: WorkspacePropagationV1::Prevent,
            ceiling: ceiling(),
            obligations: Vec::new(),
            exclusive_authority: Vec::new(),
        });
    reseal_consumer(&mut local_use);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &local_use);
    assert!(matches!(
        load_link_normalize(temp.path()),
        Err(WorkspaceError::OnwardPropagationDenied(_))
    ));
}

#[test]
fn transitive_reexport_must_carry_or_narrow_every_inherited_policy_axis() {
    let mut accepted = candidate_fixture();
    add_policy_carrying_reexport(&mut accepted);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &accepted);
    assert!(load_link_normalize(temp.path()).is_ok());

    let cases: [ExportMutation; 3] = [
        ("visibility-and-delivery-widening", |export| {
            export.visibility = WorkspaceVisibilityV1::Public;
            export.delivery = WorkspaceDeliveryV1::Source;
        }),
        ("dropped-obligation", |export| {
            export
                .obligations
                .retain(|obligation| obligation.as_str() != "retain-attribution");
        }),
        ("dropped-exclusive-claim", |export| {
            export.exclusive_authority.clear();
        }),
    ];
    for (name, mutate) in cases {
        let mut fixture = candidate_fixture();
        add_policy_carrying_reexport(&mut fixture);
        mutate(&mut fixture.consumer.as_mut().unwrap().module.exports[0]);
        reseal_consumer(&mut fixture);
        let temp = tempfile::tempdir().unwrap();
        write_fixture(temp.path(), &fixture);
        assert!(
            matches!(
                load_link_normalize(temp.path()),
                Err(WorkspaceError::PolicyWidening(_))
            ),
            "{name} must fail at the shared access-policy resolver"
        );
    }
}

#[test]
fn visibility_widening_retains_one_exact_precondition_fenced_correction() {
    let mut fixture = candidate_fixture();
    fixture.module.module.exports[0].visibility = WorkspaceVisibilityV1::Protected;
    reseal_primary(&mut fixture);
    add_consumer(&mut fixture, WorkspacePropagationV1::Transitive);
    let import = &mut fixture.consumer.as_mut().unwrap().module.imports[0];
    import.protected_delivery_authority =
        Some(id("delivery-authority", WorkspaceResourceId::parse));
    import.visibility_ceiling = WorkspaceVisibilityV1::Public;
    reseal_consumer(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    for name in [
        "pale-workspace.toml",
        "module.toml",
        "consumer-module.toml",
        "alpha.toml",
        "beta.toml",
        "consumer.toml",
        "resolution.lock.toml",
        "universe.lock.toml",
    ] {
        eprintln!(
            "\n--- {name} ---\n{}",
            fs::read_to_string(temp.path().join(name)).unwrap()
        );
    }

    let error = load_link_normalize(temp.path()).expect_err("widening must be rejected");
    let correction = error
        .authoring_correction()
        .expect("the linker proved one deterministic narrowing");
    assert_eq!(
        correction.kind(),
        AuthoringCorrectionKind::NarrowImportVisibility
    );
    assert_eq!(correction.source_label(), "consumer-module.toml");
    assert_eq!(correction.expected_source(), "\"public\"");
    assert_eq!(correction.replacement_source(), "\"protected\"");
    assert_eq!(correction.proof_subject().value(), "candidate::public-api");
    assert_eq!(
        correction.path(),
        &[
            AuthoringInstancePathSegment::Field("module".to_owned()),
            AuthoringInstancePathSegment::Field("imports".to_owned()),
            AuthoringInstancePathSegment::Index(0),
            AuthoringInstancePathSegment::Field("visibility_ceiling".to_owned()),
        ]
    );
}

#[test]
fn every_import_policy_axis_rejects_widening_with_typed_coordinates() {
    for axis in [
        WorkspacePolicyAxisV1::Disclosure,
        WorkspacePolicyAxisV1::Effect,
        WorkspacePolicyAxisV1::ModelUse,
        WorkspacePolicyAxisV1::Retention,
        WorkspacePolicyAxisV1::Rights,
        WorkspacePolicyAxisV1::Controller,
        WorkspacePolicyAxisV1::Authority,
    ] {
        let mut fixture = candidate_fixture();
        let export = &mut fixture.module.module.exports[0];
        match axis {
            WorkspacePolicyAxisV1::Disclosure => {
                export.ceiling.disclosure = DisclosureCeilingV1::Protected;
            }
            WorkspacePolicyAxisV1::Effect => export.ceiling.effect = WorkspaceEffectCeilingV1::None,
            WorkspacePolicyAxisV1::ModelUse => export.ceiling.model_use_permitted = false,
            WorkspacePolicyAxisV1::Retention => export.ceiling.retention_seconds = 100,
            WorkspacePolicyAxisV1::Rights
            | WorkspacePolicyAxisV1::Controller
            | WorkspacePolicyAxisV1::Authority => {}
        }
        reseal_primary(&mut fixture);
        add_consumer(&mut fixture, WorkspacePropagationV1::Transitive);
        let import = &mut fixture.consumer.as_mut().unwrap().module.imports[0];
        match axis {
            WorkspacePolicyAxisV1::Disclosure => {
                import.edge_policy.disclosure = DisclosureCeilingV1::Public;
            }
            WorkspacePolicyAxisV1::Effect => {
                import.edge_policy.effect = WorkspaceEffectCeilingV1::Read;
            }
            WorkspacePolicyAxisV1::ModelUse => import.edge_policy.model_use_permitted = true,
            WorkspacePolicyAxisV1::Retention => import.edge_policy.retention_seconds = 101,
            WorkspacePolicyAxisV1::Rights => import
                .edge_policy
                .rights
                .permitted
                .push(id("derive", WorkspaceRightId::parse)),
            WorkspacePolicyAxisV1::Controller => import
                .edge_policy
                .controllers
                .permitted
                .push(id("other-controller", WorkspaceControllerScopeId::parse)),
            WorkspacePolicyAxisV1::Authority => import
                .edge_policy
                .authorities
                .permitted
                .push(id("other-authority", WorkspaceAuthorityScopeId::parse)),
        }
        reseal_consumer(&mut fixture);
        let temp = tempfile::tempdir().unwrap();
        write_fixture(temp.path(), &fixture);
        assert!(matches!(
            load_link_normalize(temp.path()),
            Err(WorkspaceError::ImportPolicyWidening { axis: observed, .. }) if observed == axis
        ));
    }
}

#[test]
fn import_policy_provenance_retains_export_edge_and_effective_coordinates() {
    let mut fixture = candidate_fixture();
    add_consumer(&mut fixture, WorkspacePropagationV1::Transitive);
    let import = &mut fixture.consumer.as_mut().unwrap().module.imports[0];
    import.edge_policy.effect = WorkspaceEffectCeilingV1::None;
    import.edge_policy.rights.permitted.clear();
    import.edge_policy.controllers.permitted.clear();
    import.edge_policy.authorities.permitted.clear();
    let declared_edge = import.edge_policy.clone();
    reseal_consumer(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let normalized = load_link_normalize(temp.path()).unwrap();
    let path = &normalized.linked.resolved_imports[0];
    assert_eq!(path.export_ceiling, ceiling());
    assert_eq!(path.import_ceiling, declared_edge);
    assert_eq!(
        path.effective_ceiling,
        path.export_ceiling.meet(&path.import_ceiling)
    );
    assert!(path.effective_ceiling.rights.permitted.is_empty());
    assert!(path.effective_ceiling.controllers.permitted.is_empty());
    assert!(path.effective_ceiling.authorities.permitted.is_empty());
}

#[test]
fn transitive_imports_retain_each_edge_policy_coordinate() {
    let mut fixture = candidate_fixture();
    add_policy_carrying_reexport(&mut fixture);
    add_transitive_consumer(&mut fixture);
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let normalized = load_link_normalize(temp.path()).unwrap();
    assert_eq!(normalized.linked.resolved_imports.len(), 2);
    let downstream = normalized
        .linked
        .resolved_imports
        .iter()
        .find(|path| path.importer_module == module_id("downstream"))
        .unwrap();
    assert_eq!(
        downstream.effective_ceiling,
        downstream.export_ceiling.meet(&downstream.import_ceiling)
    );
    assert!(downstream.import_ceiling.rights.permitted.is_empty());
    assert!(downstream.import_ceiling.controllers.permitted.is_empty());
    assert!(downstream.import_ceiling.authorities.permitted.is_empty());
}

#[test]
fn checked_in_populated_fixture_crosses_the_parser_linker_and_normalizer() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace-language/populated");
    assert_eq!(
        load_link_normalize(&root)
            .unwrap()
            .linked
            .declarations
            .len(),
        2
    );
}

#[test]
fn negatives_fail_at_the_owning_namespace_cycle_lock_and_policy_boundaries() {
    let cases: [FixtureMutation; 17] = [
        ("resolution-digest", |fixture| {
            fixture.resolution.resolution_digest =
                WorkspaceResolutionLockDigest::hash_canonical(&"wrong").unwrap();
        }),
        ("universe-transposition", |fixture| {
            fixture.universe.universe_digest =
                WorkspaceUniverseLockDigest::hash_canonical(&"wrong").unwrap();
        }),
        ("namespace", |fixture| {
            if let WorkspaceDeclarationV1::Trait { dependencies, .. } =
                &mut fixture.components[0].1.declarations[0]
            {
                dependencies[0].target.namespace = id("absent", WorkspaceNamespaceId::parse);
            }
            reseal_primary(fixture);
        }),
        ("cycle", |fixture| {
            if let WorkspaceDeclarationV1::Trait { dependencies, .. } =
                &mut fixture.components[0].1.declarations[0]
            {
                dependencies[0].kind = DeclarationDependencyKindV1::Negation;
            }
            reseal_primary(fixture);
        }),
        ("policy", |fixture| {
            fixture.module.module.exports[0].visibility = WorkspaceVisibilityV1::Private;
            reseal_primary(fixture);
            add_consumer(fixture, WorkspacePropagationV1::Prevent);
            add_consumer_dependency(fixture);
        }),
        ("prevent", |fixture| {
            fixture.module.module.exports[0].propagation = WorkspacePropagationV1::Prevent;
            reseal_primary(fixture);
            add_consumer(fixture, WorkspacePropagationV1::Transitive);
        }),
        ("narrowing", |fixture| {
            fixture.module.module.extensions.push(WorkspaceExtensionV1 {
                target: WorkspaceDeclarationRefV1 {
                    namespace: id("alpha", WorkspaceNamespaceId::parse),
                    declaration_id: id("common", WorkspaceDeclarationId::parse),
                },
                namespace: id("candidate", WorkspaceNamespaceId::parse),
                added_traits: Vec::new(),
                narrowed_ceiling: Some(WorkspacePolicyCeilingV1 {
                    disclosure: DisclosureCeilingV1::Public,
                    effect: WorkspaceEffectCeilingV1::Mutate,
                    model_use_permitted: true,
                    retention_seconds: 7_200,
                    rights: ceiling().rights,
                    controllers: ceiling().controllers,
                    authorities: ceiling().authorities,
                }),
                added_obligations: Vec::new(),
            });
            reseal_primary(fixture);
        }),
        ("exclusive-authority", |fixture| {
            for (index, authority) in ["authority-a", "authority-b"].into_iter().enumerate() {
                fixture.components[index]
                    .1
                    .declarations
                    .push(WorkspaceDeclarationV1::Policy {
                        id: id("authority-policy", WorkspaceDeclarationId::parse),
                        namespace: id(
                            if index == 0 { "alpha" } else { "beta" },
                            WorkspaceNamespaceId::parse,
                        ),
                        ceiling: ceiling(),
                        obligations: Vec::new(),
                        exclusive_authority: vec![WorkspaceExclusiveAuthorityClaimV1 {
                            scope: id("shared-scope", WorkspaceAuthorityScopeId::parse),
                            authority_resolver: id(authority, WorkspaceResourceId::parse),
                        }],
                        dependencies: Vec::new(),
                    });
            }
            reseal_primary(fixture);
        }),
        ("prevented-onward-propagation", |fixture| {
            add_consumer(fixture, WorkspacePropagationV1::Prevent);
            let copied_projection = projection(
                &fixture.module.module.module_id,
                "alpha",
                &fixture.components[0].1.declarations[0],
            );
            fixture
                .consumer
                .as_mut()
                .unwrap()
                .module
                .exports
                .push(WorkspaceExportV1 {
                    export_id: id("copied", WorkspaceExportId::parse),
                    declarations: vec![copied_projection],
                    visibility: WorkspaceVisibilityV1::Public,
                    delivery: WorkspaceDeliveryV1::Interface,
                    propagation: WorkspacePropagationV1::Prevent,
                    ceiling: ceiling(),
                    obligations: Vec::new(),
                    exclusive_authority: Vec::new(),
                });
            reseal_consumer(fixture);
        }),
        ("added-trait-is-not-trait", |fixture| {
            fixture.components[1].1.declarations[0] = WorkspaceDeclarationV1::Attribute {
                id: id("common", WorkspaceDeclarationId::parse),
                namespace: id("beta", WorkspaceNamespaceId::parse),
                value_kind: WorkspaceValueKindV1::Text,
                dependencies: vec![WorkspaceDeclarationDependencyV1 {
                    target: WorkspaceDeclarationRefV1 {
                        namespace: id("alpha", WorkspaceNamespaceId::parse),
                        declaration_id: id("common", WorkspaceDeclarationId::parse),
                    },
                    kind: DeclarationDependencyKindV1::Positive,
                }],
            };
            fixture.module.module.extensions.push(WorkspaceExtensionV1 {
                target: WorkspaceDeclarationRefV1 {
                    namespace: id("alpha", WorkspaceNamespaceId::parse),
                    declaration_id: id("common", WorkspaceDeclarationId::parse),
                },
                namespace: id("candidate", WorkspaceNamespaceId::parse),
                added_traits: vec![WorkspaceDeclarationRefV1 {
                    namespace: id("beta", WorkspaceNamespaceId::parse),
                    declaration_id: id("common", WorkspaceDeclarationId::parse),
                }],
                narrowed_ceiling: None,
                added_obligations: Vec::new(),
            });
            reseal_primary(fixture);
        }),
        ("noncanonical-component-set", |fixture| {
            fixture.components[0]
                .1
                .declarations
                .push(WorkspaceDeclarationV1::Trait {
                    id: id("aardvark", WorkspaceDeclarationId::parse),
                    namespace: id("alpha", WorkspaceNamespaceId::parse),
                    dependencies: Vec::new(),
                });
            reseal_primary(fixture);
        }),
        ("cross-module-reference-without-import", |fixture| {
            add_consumer(fixture, WorkspacePropagationV1::Prevent);
            fixture.consumer.as_mut().unwrap().module.imports.clear();
            add_consumer_dependency(fixture);
        }),
        ("unselected-export", |fixture| {
            add_consumer(fixture, WorkspacePropagationV1::Prevent);
            fixture.consumer.as_mut().unwrap().module.imports[0].paths[0].export_id =
                id("absent-export", WorkspaceExportId::parse);
            add_consumer_dependency(fixture);
        }),
        ("ambiguous-import-alias", |fixture| {
            add_consumer(fixture, WorkspacePropagationV1::Prevent);
            fixture.consumer.as_mut().unwrap().module.imports[0]
                .paths
                .push(WorkspaceImportPathV1 {
                    export_id: id("public-api", WorkspaceExportId::parse),
                    source_namespace: id("beta", WorkspaceNamespaceId::parse),
                    local_namespace: id("upstream", WorkspaceNamespaceId::parse),
                });
            add_consumer_dependency(fixture);
        }),
        ("adopted-importer-with-unadopted-upstream", |fixture| {
            add_consumer(fixture, WorkspacePropagationV1::Prevent);
            let consumer = fixture.consumer.as_ref().unwrap();
            fixture.universe = WorkspaceUniverseLockV1::seal(
                id("synthetic", WorkspaceId::parse),
                fixture.resolution.resolution_digest.clone(),
                vec![WorkspaceAdoptedReleaseV1 {
                    module_id: consumer.module.module_id.clone(),
                    release_id: consumer.module.release_id.clone(),
                    module_digest: consumer.module_digest.clone(),
                }],
            )
            .unwrap();
        }),
        ("adopted-importer-with-missing-upstream", |fixture| {
            add_consumer(fixture, WorkspacePropagationV1::Prevent);
            let consumer = fixture.consumer.as_ref().unwrap();
            fixture.resolution =
                WorkspaceResolutionLockV1::seal(vec![WorkspaceResolvedReleaseV1 {
                    module_id: consumer.module.module_id.clone(),
                    release_id: consumer.module.release_id.clone(),
                    module_digest: consumer.module_digest.clone(),
                    publisher: consumer.module.publisher.clone(),
                    release_state: WorkspaceReleaseStateV1::Published,
                }])
                .unwrap();
            fixture.universe = WorkspaceUniverseLockV1::seal(
                id("synthetic", WorkspaceId::parse),
                fixture.resolution.resolution_digest.clone(),
                vec![WorkspaceAdoptedReleaseV1 {
                    module_id: consumer.module.module_id.clone(),
                    release_id: consumer.module.release_id.clone(),
                    module_digest: consumer.module_digest.clone(),
                }],
            )
            .unwrap();
        }),
        ("adoption-coordinate-transposition", |fixture| {
            add_consumer(fixture, WorkspacePropagationV1::Prevent);
            let wrong_digest = fixture.consumer.as_ref().unwrap().module_digest.clone();
            let mut adopted = fixture.universe.adopted_releases.clone();
            adopted
                .iter_mut()
                .find(|release| release.module_id == fixture.module.module.module_id)
                .unwrap()
                .module_digest = wrong_digest;
            fixture.universe = WorkspaceUniverseLockV1::seal(
                id("synthetic", WorkspaceId::parse),
                fixture.resolution.resolution_digest.clone(),
                adopted,
            )
            .unwrap();
        }),
    ];
    for (name, mutate) in cases {
        let mut fixture = candidate_fixture();
        mutate(&mut fixture);
        let temp = tempfile::tempdir().unwrap();
        write_fixture(temp.path(), &fixture);
        assert!(
            load_link_normalize(temp.path()).is_err(),
            "{name} must fail closed"
        );
    }
}

#[test]
fn public_sealers_canonicalize_sets_and_reject_duplicate_identities() {
    let fixture = candidate_fixture();
    let mut permuted = fixture.module.module.clone();
    permuted.source_components.reverse();
    assert_eq!(
        WorkspaceModuleV1::seal(permuted).unwrap().module_digest,
        fixture.module.module_digest
    );

    let mut duplicate_module = fixture.module.module.clone();
    duplicate_module
        .exports
        .push(duplicate_module.exports[0].clone());
    assert!(matches!(
        WorkspaceModuleV1::seal(duplicate_module),
        Err(InvalidWorkspaceModule::Duplicate { .. })
    ));

    let extension = WorkspaceExtensionV1 {
        namespace: id("alpha", WorkspaceNamespaceId::parse),
        target: WorkspaceDeclarationRefV1 {
            namespace: id("alpha", WorkspaceNamespaceId::parse),
            declaration_id: id("common", WorkspaceDeclarationId::parse),
        },
        added_traits: Vec::new(),
        narrowed_ceiling: None,
        added_obligations: Vec::new(),
    };
    let mut duplicate_extension = fixture.module.module.clone();
    duplicate_extension.extensions = vec![extension.clone(), extension];
    assert!(matches!(
        WorkspaceModuleV1::seal(duplicate_extension),
        Err(InvalidWorkspaceModule::Duplicate {
            kind: "extension",
            ..
        })
    ));

    let mut too_many_exports = fixture.module.module.clone();
    too_many_exports.exports =
        vec![fixture.module.module.exports[0].clone(); MAX_WORKSPACE_EXPORTS + 1];
    assert!(matches!(
        WorkspaceModuleV1::seal(too_many_exports),
        Err(InvalidWorkspaceModule::Limit {
            kind: "export",
            observed,
            limit: MAX_WORKSPACE_EXPORTS,
        }) if observed == MAX_WORKSPACE_EXPORTS + 1
    ));

    let oversized_policy_declarations = [
        (
            "policy obligation",
            MAX_WORKSPACE_OBLIGATIONS,
            WorkspaceDeclarationV1::Policy {
                id: id("common", WorkspaceDeclarationId::parse),
                namespace: id("alpha", WorkspaceNamespaceId::parse),
                ceiling: ceiling(),
                obligations: vec![
                    id("bounded-obligation", WorkspaceObligationId::parse);
                    MAX_WORKSPACE_OBLIGATIONS + 1
                ],
                exclusive_authority: Vec::new(),
                dependencies: Vec::new(),
            },
        ),
        (
            "policy authority claim",
            MAX_WORKSPACE_AUTHORITY_CLAIMS,
            WorkspaceDeclarationV1::Policy {
                id: id("common", WorkspaceDeclarationId::parse),
                namespace: id("alpha", WorkspaceNamespaceId::parse),
                ceiling: ceiling(),
                obligations: Vec::new(),
                exclusive_authority: vec![
                    WorkspaceExclusiveAuthorityClaimV1 {
                        scope: id("bounded-scope", WorkspaceAuthorityScopeId::parse),
                        authority_resolver: id("bounded-authority", WorkspaceResourceId::parse,),
                    };
                    MAX_WORKSPACE_AUTHORITY_CLAIMS + 1
                ],
                dependencies: Vec::new(),
            },
        ),
    ];
    for (kind, limit, declaration) in oversized_policy_declarations {
        let mut module = fixture.module.module.clone();
        module.exports[0].declarations[0].declaration = declaration;
        let retained = WorkspaceModuleV1 {
            schema: "pale/workspace-module.v1".into(),
            schema_version: 1,
            module: module.clone(),
            module_digest: fixture.module.module_digest.clone(),
        };
        let is_expected_limit = |error| match (kind, error) {
            (
                "policy obligation",
                InvalidWorkspaceModule::PolicyCombination(
                    WorkspacePolicyCombinationError::ObligationLimit {
                        observed,
                        limit: observed_limit,
                    },
                ),
            ) => observed == limit + 1 && observed_limit == limit,
            (
                "policy authority claim",
                InvalidWorkspaceModule::PolicyCombination(
                    WorkspacePolicyCombinationError::AuthorityLimit {
                        observed,
                        limit: observed_limit,
                    },
                ),
            ) => observed == limit + 1 && observed_limit == limit,
            _ => false,
        };
        assert!(is_expected_limit(
            WorkspaceModuleV1::seal(module).unwrap_err()
        ));
        assert!(is_expected_limit(retained.verify().unwrap_err()));
    }

    let release = fixture.resolution.releases[0].clone();
    assert!(matches!(
        WorkspaceResolutionLockV1::seal(vec![release.clone(), release]),
        Err(InvalidWorkspaceLock::DuplicateModule(_))
    ));
    assert!(matches!(
        WorkspaceResolutionLockV1::seal(vec![
            fixture.resolution.releases[0].clone();
            MAX_WORKSPACE_MODULES + 1
        ]),
        Err(InvalidWorkspaceLock::Limit { .. })
    ));
    let adoption = fixture.universe.adopted_releases[0].clone();
    assert!(matches!(
        WorkspaceUniverseLockV1::seal(
            id("synthetic", WorkspaceId::parse),
            fixture.resolution.resolution_digest.clone(),
            vec![adoption.clone(), adoption],
        ),
        Err(InvalidWorkspaceLock::DuplicateModule(_))
    ));

    let source = WorkspaceDerivationSourceV1 {
        source_id: id("duplicate", WorkspaceDerivationSourceId::parse),
        source_path: pale_core::paths::OwnedConfinedRelativePath::parse("source.toml").unwrap(),
        source_digest: WorkspaceSourceContentDigest::hash_bytes(b"source"),
        byte_len: 6,
    };
    let receipt_preimage = |sources| WorkspaceDerivationReceiptPreimageV1 {
        prior_resolution_digest: fixture.resolution.resolution_digest.clone(),
        current_resolution_digest: fixture.resolution.resolution_digest.clone(),
        prior_universe_digest: fixture.universe.universe_digest.clone(),
        current_universe_digest: fixture.universe.universe_digest.clone(),
        workspace_semantic_ir_digest: WorkspaceSemanticIrDigest::hash_canonical(&"semantic")
            .unwrap(),
        sources,
        nodes: Vec::new(),
        roots: Vec::new(),
        impacts: Vec::new(),
    };
    assert!(matches!(
        WorkspaceDerivationReceiptV1::seal(receipt_preimage(vec![source.clone(), source.clone()])),
        Err(InvalidWorkspaceDerivationReceipt::AggregateOrder)
    ));
    assert!(matches!(
        WorkspaceDerivationReceiptV1::seal(receipt_preimage(vec![
            source;
            MAX_WORKSPACE_DERIVATION_SOURCES
                + 1
        ])),
        Err(InvalidWorkspaceDerivationReceipt::Limit { .. })
    ));

    let uri_source = WorkspaceDerivationSourceV1 {
        source_id: id("uri-source", WorkspaceDerivationSourceId::parse),
        source_path: pale_core::paths::OwnedConfinedRelativePath::parse("https:artifact").unwrap(),
        source_digest: WorkspaceSourceContentDigest::hash_bytes(b"source"),
        byte_len: 6,
    };
    assert!(matches!(
        WorkspaceDerivationReceiptV1::seal(receipt_preimage(vec![uri_source])),
        Err(InvalidWorkspaceDerivationReceipt::SourcePath(_))
    ));
}

#[test]
fn portable_source_rejects_absolute_identity_and_unknown_secret_field() {
    let fixture = candidate_fixture();
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let module_path = temp.path().join("module.toml");
    let leaked = fs::read_to_string(&module_path).unwrap().replace(
        "module_id = \"candidate\"",
        "module_id = \"/private/candidate\"\nsecret = \"not-portable\"",
    );
    fs::write(module_path, leaked).unwrap();
    assert!(matches!(
        source::parse_workspace_source(
            &pale_adapter_fs::LocalSourceFilesystem,
            &temp.path().join("pale-workspace.toml")
        ),
        Err(WorkspaceError::SchemaCheck { .. }) | Err(WorkspaceError::Decode { .. })
    ));
}

#[test]
fn supplied_module_digest_is_rejected_during_typed_source_admission() {
    use crate::workspace::{
        compile_portable_workspace, CapturedWorkspaceDocument, CapturedWorkspaceSource,
        PortableWorkspaceFailureClass,
    };

    let fixture = candidate_fixture();
    let temp = tempfile::tempdir().unwrap();
    write_fixture(temp.path(), &fixture);
    let module_path = temp.path().join("module.toml");
    let mut module: toml::Value =
        toml::from_str(&fs::read_to_string(&module_path).unwrap()).unwrap();
    assert_ne!(
        module["module_digest"].as_str(),
        Some("0".repeat(64).as_str())
    );
    module["module_digest"] = toml::Value::String("0".repeat(64));
    pale_contracts::validate_value(
        &serde_json::to_value(&module).unwrap(),
        "workspace-module.v1",
    )
    .expect("a valid-format digest passes structural schema admission");
    fs::write(&module_path, toml::to_string(&module).unwrap()).unwrap();

    let error = source::parse_workspace_source(
        &pale_adapter_fs::LocalSourceFilesystem,
        &temp.path().join("pale-workspace.toml"),
    )
    .expect_err("the supplied digest does not verify against its record");
    let WorkspaceError::Decode {
        coordinate, record, ..
    } = &error
    else {
        panic!("expected typed source rejection, got {error:?}");
    };
    assert_eq!(coordinate.path(), module_path);
    assert_eq!(*record, "workspace module");
    assert_eq!(coordinate.line(), None);
    assert_eq!(coordinate.column(), None);
    assert_eq!(
        error.failure_disposition(),
        crate::ApplicationFailureDisposition::InvalidInput
    );

    let documents = fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            CapturedWorkspaceDocument::new(
                entry.file_name().to_str().unwrap().to_owned(),
                fs::read(entry.path()).unwrap(),
            )
            .unwrap()
        })
        .collect();
    let source = CapturedWorkspaceSource::new(
        CapturedWorkspaceSource::admit_root_manifest("pale-workspace.toml".into()).unwrap(),
        documents,
    )
    .unwrap();
    let failure = compile_portable_workspace(source)
        .expect_err("captured records use the same typed admission boundary");
    assert_eq!(
        failure.class(),
        PortableWorkspaceFailureClass::ContractRejected
    );
    assert!(matches!(
        failure.workspace_error(),
        WorkspaceError::Decode { .. }
    ));
}

mod comparison_subjects;
mod impact_roots;
mod proof_dependents;
mod source_identity;
