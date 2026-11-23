//! Law tests for derivation paging and causal-explanation projection.

use super::*;
use pale_core::records::{
    ExperienceCompositionSourceDigest, OrganizationalTranslationClosureDigest,
    ResponsibilityDerivationClosureDigest, WorkspaceComponentDigest, WorkspaceDeclarationId,
    WorkspaceDeclarationRefV1, WorkspaceDerivationValueDigest, WorkspaceExportId,
    WorkspaceImportSurfaceDigest, WorkspaceModuleDigest, WorkspaceModuleId, WorkspaceNamespaceId,
    WorkspaceReleaseId, WorkspaceResolutionLockDigest, WorkspaceSemanticIrDigest,
    WorkspaceUniverseLockDigest,
};

#[test]
fn page_window_applies_default_cap_and_terminal_laws() {
    let default = PageWindow::new(1_000, 0, 0);
    assert_eq!(default.range(), 0..usize::from(DEFAULT_PROOF_PAGE_SIZE));
    assert_eq!(
        default.next_offset,
        Some(u32::from(DEFAULT_PROOF_PAGE_SIZE))
    );

    let capped = PageWindow::new(1_000, 0, u16::MAX);
    assert_eq!(capped.range(), 0..usize::from(MAX_PROOF_PAGE_SIZE));
    assert_eq!(capped.next_offset, Some(u32::from(MAX_PROOF_PAGE_SIZE)));

    let exact_end = PageWindow::new(1_000, 1_000, 1);
    assert!(exact_end.range().is_empty());
    assert_eq!(exact_end.next_offset, None);

    let beyond_end = PageWindow::new(1_000, u32::MAX, 1);
    assert!(beyond_end.range().is_empty());
    assert_eq!(beyond_end.next_offset, None);
}

#[test]
fn explanation_projection_preserves_removed_frontier_and_total_absence() {
    let source_id = pale_core::records::WorkspaceDerivationSourceId::parse("removed-source")
        .expect("test source identity is valid");
    let subject = WorkspaceDerivationSubjectV1::Source {
        source_id: source_id.clone(),
    };
    let prior_output = WorkspaceDerivationOutputV1::SourceContent {
        source_path: pale_core::paths::OwnedConfinedRelativePath::parse("removed/source.toml")
            .expect("test source label is confined"),
        source_digest: pale_core::records::WorkspaceSourceContentDigest::hash_bytes(b"prior"),
        byte_len: 5,
    };
    let root = WorkspaceDerivationRootV1::seal(
        subject.clone(),
        WorkspaceDerivationRootChangeV1::Removed {
            prior_output: prior_output.clone(),
        },
    )
    .expect("removed test root is valid");
    let node = WorkspaceDerivationNodeV1::seal(
        WorkspaceDerivationOriginV1::Source { source_id },
        WorkspaceDerivationRuleV1::SourceAdmission,
        subject.clone(),
        prior_output,
        Vec::new(),
    )
    .expect("predecessor source fact is valid");
    let projected_subject = PortableWorkspaceSubject::from_internal(&subject);
    let affected = project_explanation(
        &projected_subject,
        PortableWorkspaceDerivationExplanation::Affected {
            root: Box::new(root),
            frontier: ApplicationExplanationFrontier::Predecessor,
            path: vec![node],
        },
        &[],
    );
    assert_eq!(
        affected.state(),
        PortableWorkspaceExplanationState::Affected
    );
    let causal = affected
        .causal()
        .expect("affected projection carries a witness");
    assert_eq!(causal.root().kind(), PortableWorkspaceChangeKind::Removed);
    assert!(causal.root().prior_fact().is_some());
    assert!(causal.root().current_fact().is_none());
    assert_eq!(
        causal.frontier(),
        PortableWorkspaceExplanationFrontier::Predecessor
    );
    assert!(matches!(
        causal.steps()[0].provenance(),
        PortableWorkspaceProvenance::Source { .. }
    ));
    assert!(matches!(
        causal.steps()[0].fact(),
        PortableWorkspaceFact::SourceContent { .. }
    ));

    let absent = project_explanation(
        &projected_subject,
        PortableWorkspaceDerivationExplanation::NotPresent,
        &[],
    );
    assert_eq!(
        absent.state(),
        PortableWorkspaceExplanationState::NotPresent
    );
    assert!(absent.causal().is_none());
}

#[test]
fn change_root_projection_preserves_the_exact_transition_fact_matrix() {
    let source_id = pale_core::records::WorkspaceDerivationSourceId::parse("added-source")
        .expect("test source identity is valid");
    let subject = WorkspaceDerivationSubjectV1::Source {
        source_id: source_id.clone(),
    };
    let output = WorkspaceDerivationOutputV1::SourceContent {
        source_path: pale_core::paths::OwnedConfinedRelativePath::parse("added/source.toml")
            .expect("test source label is confined"),
        source_digest: pale_core::records::WorkspaceSourceContentDigest::hash_bytes(b"current"),
        byte_len: 7,
    };
    let node = WorkspaceDerivationNodeV1::seal(
        WorkspaceDerivationOriginV1::Source { source_id },
        WorkspaceDerivationRuleV1::SourceAdmission,
        subject.clone(),
        output.clone(),
        Vec::new(),
    )
    .expect("current source fact is valid");
    let root = WorkspaceDerivationRootV1::seal(
        subject.clone(),
        WorkspaceDerivationRootChangeV1::Added {
            current_node: node.node_digest.clone(),
        },
    )
    .expect("added test root is valid");

    let projected = PortableWorkspaceChangeRoot::from_internal(&root, std::slice::from_ref(&node));
    assert_eq!(projected.kind(), PortableWorkspaceChangeKind::Added);
    assert!(projected.prior_fact().is_none());
    assert_eq!(
        projected.current_fact(),
        Some(&PortableWorkspaceFact::from_internal(&output))
    );

    let prior_output = WorkspaceDerivationOutputV1::SourceContent {
        source_path: pale_core::paths::OwnedConfinedRelativePath::parse("added/source.toml")
            .expect("test source label is confined"),
        source_digest: pale_core::records::WorkspaceSourceContentDigest::hash_bytes(b"prior"),
        byte_len: 5,
    };
    let modified = WorkspaceDerivationRootV1::seal(
        subject.clone(),
        WorkspaceDerivationRootChangeV1::Modified {
            prior_output: prior_output.clone(),
            current_node: node.node_digest.clone(),
        },
    )
    .expect("modified test root is valid");
    let projected =
        PortableWorkspaceChangeRoot::from_internal(&modified, std::slice::from_ref(&node));
    assert_eq!(projected.kind(), PortableWorkspaceChangeKind::Modified);
    assert_eq!(
        projected.prior_fact(),
        Some(&PortableWorkspaceFact::from_internal(&prior_output))
    );
    assert_eq!(
        projected.current_fact(),
        Some(&PortableWorkspaceFact::from_internal(&output))
    );

    let removed = WorkspaceDerivationRootV1::seal(
        subject,
        WorkspaceDerivationRootChangeV1::Removed {
            prior_output: prior_output.clone(),
        },
    )
    .expect("removed test root is valid");
    let projected = PortableWorkspaceChangeRoot::from_internal(&removed, &[]);
    assert_eq!(projected.kind(), PortableWorkspaceChangeKind::Removed);
    assert_eq!(
        projected.prior_fact(),
        Some(&PortableWorkspaceFact::from_internal(&prior_output))
    );
    assert!(projected.current_fact().is_none());
}

#[test]
fn provenance_projection_preserves_every_exact_owner_field() {
    let module_digest =
        WorkspaceModuleDigest::hash_canonical(&"module").expect("test module digest is canonical");
    let target = WorkspaceDeclarationRefV1 {
        namespace: WorkspaceNamespaceId::parse("target-ns").unwrap(),
        declaration_id: WorkspaceDeclarationId::parse("target-decl").unwrap(),
    };
    let cases = [
        (
            WorkspaceDerivationOriginV1::Source {
                source_id: pale_core::records::WorkspaceDerivationSourceId::parse("source-a")
                    .unwrap(),
            },
            PortableWorkspaceProvenance::Source {
                source_id: "source-a".to_string(),
            },
        ),
        (
            WorkspaceDerivationOriginV1::ModuleRelease {
                module_id: WorkspaceModuleId::parse("module-a").unwrap(),
                release_id: WorkspaceReleaseId::parse("release-a").unwrap(),
                module_digest: module_digest.clone(),
            },
            PortableWorkspaceProvenance::ModuleRelease {
                module_id: "module-a".to_string(),
                release_id: "release-a".to_string(),
                module_digest: module_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOriginV1::Import {
                importer_module: WorkspaceModuleId::parse("importer").unwrap(),
                upstream_module: WorkspaceModuleId::parse("upstream").unwrap(),
                upstream_release: WorkspaceReleaseId::parse("release-a").unwrap(),
                export_id: WorkspaceExportId::parse("export-a").unwrap(),
                source_namespace: WorkspaceNamespaceId::parse("source-ns").unwrap(),
                local_namespace: WorkspaceNamespaceId::parse("local-ns").unwrap(),
            },
            PortableWorkspaceProvenance::Import {
                importer_module_id: "importer".to_string(),
                upstream_module_id: "upstream".to_string(),
                upstream_release_id: "release-a".to_string(),
                export_id: "export-a".to_string(),
                source_namespace_id: "source-ns".to_string(),
                local_namespace_id: "local-ns".to_string(),
            },
        ),
        (
            WorkspaceDerivationOriginV1::Extension {
                owner_module: WorkspaceModuleId::parse("module-a").unwrap(),
                namespace: WorkspaceNamespaceId::parse("extension-ns").unwrap(),
                target,
            },
            PortableWorkspaceProvenance::Extension {
                owner_module_id: "module-a".to_string(),
                extension_namespace_id: "extension-ns".to_string(),
                target_namespace_id: "target-ns".to_string(),
                target_declaration_id: "target-decl".to_string(),
            },
        ),
        (
            WorkspaceDerivationOriginV1::Compilation,
            PortableWorkspaceProvenance::Compilation,
        ),
    ];

    for (internal, expected) in cases {
        assert_eq!(
            PortableWorkspaceProvenance::from_internal(&internal),
            expected
        );
    }
}

#[test]
fn fact_projection_preserves_every_digest_domain_and_source_coordinate() {
    let source_digest = pale_core::records::WorkspaceSourceContentDigest::hash_bytes(b"source");
    let component_digest = WorkspaceComponentDigest::hash_canonical(&"component").unwrap();
    let module_digest = WorkspaceModuleDigest::hash_canonical(&"module").unwrap();
    let value_digest = WorkspaceDerivationValueDigest::hash_canonical(&"value").unwrap();
    let surface_digest = WorkspaceImportSurfaceDigest::hash_canonical(&"surface").unwrap();
    let responsibility_digest =
        ResponsibilityDerivationClosureDigest::hash_canonical(&"responsibility").unwrap();
    let organizational_digest =
        OrganizationalTranslationClosureDigest::hash_canonical(&"organization").unwrap();
    let composition_digest =
        ExperienceCompositionSourceDigest::hash_canonical(&"composition").unwrap();
    let resolution_digest = WorkspaceResolutionLockDigest::hash_canonical(&"resolution").unwrap();
    let universe_digest = WorkspaceUniverseLockDigest::hash_canonical(&"universe").unwrap();
    let semantic_digest = WorkspaceSemanticIrDigest::hash_canonical(&"semantic").unwrap();
    let cases = [
        (
            WorkspaceDerivationOutputV1::SourceContent {
                source_path: pale_core::paths::OwnedConfinedRelativePath::parse(
                    "sources/exact.toml",
                )
                .unwrap(),
                source_digest: source_digest.clone(),
                byte_len: 6,
            },
            PortableWorkspaceFact::SourceContent {
                source_path: "sources/exact.toml".to_string(),
                source_digest: source_digest.to_string(),
                byte_len: 6,
            },
        ),
        (
            WorkspaceDerivationOutputV1::Component {
                component_digest: component_digest.clone(),
            },
            PortableWorkspaceFact::Component {
                component_digest: component_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::Module {
                module_digest: module_digest.clone(),
            },
            PortableWorkspaceFact::Module {
                module_digest: module_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::Declaration {
                value_digest: value_digest.clone(),
            },
            PortableWorkspaceFact::Declaration {
                value_digest: value_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::ImportSurface {
                surface_digest: surface_digest.clone(),
            },
            PortableWorkspaceFact::ImportSurface {
                surface_digest: surface_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::Extension {
                value_digest: value_digest.clone(),
            },
            PortableWorkspaceFact::Extension {
                value_digest: value_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::EffectivePolicy {
                value_digest: value_digest.clone(),
            },
            PortableWorkspaceFact::EffectivePolicy {
                value_digest: value_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::ResponsibilityClosure {
                closure_digest: responsibility_digest.clone(),
            },
            PortableWorkspaceFact::ResponsibilityClosure {
                closure_digest: responsibility_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::OrganizationalTranslationClosure {
                closure_digest: organizational_digest.clone(),
            },
            PortableWorkspaceFact::OrganizationalTranslationClosure {
                closure_digest: organizational_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::ExperienceComposition {
                source_digest: composition_digest.clone(),
            },
            PortableWorkspaceFact::ExperienceComposition {
                source_digest: composition_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::Resource {
                value_digest: value_digest.clone(),
            },
            PortableWorkspaceFact::Resource {
                value_digest: value_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::ResolutionLock {
                resolution_digest: resolution_digest.clone(),
            },
            PortableWorkspaceFact::ResolutionLock {
                resolution_digest: resolution_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::UniverseLock {
                universe_digest: universe_digest.clone(),
            },
            PortableWorkspaceFact::UniverseLock {
                universe_digest: universe_digest.to_string(),
            },
        ),
        (
            WorkspaceDerivationOutputV1::SemanticIr {
                workspace_semantic_ir_digest: semantic_digest.clone(),
            },
            PortableWorkspaceFact::SemanticIr {
                workspace_semantic_ir_digest: semantic_digest.to_string(),
            },
        ),
    ];

    for (internal, expected) in cases {
        assert_eq!(PortableWorkspaceFact::from_internal(&internal), expected);
    }
}
