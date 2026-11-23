//! Public-boundary tests for captured one-shot requests and diagnostics.

use pale_application::compile::{
    EffectiveProfileNarrowingReason as ApplicationNarrowingReason,
    EffectiveProfileOperationExposure as ApplicationOperationExposure,
    EffectiveProfilePolicyBlock as ApplicationPolicyBlock,
    EffectiveProfileProviderDisposition as ApplicationProviderDisposition,
    EffectiveProfileRequirementDisposition as ApplicationRequirementDisposition,
};
use pale_application::semantic::ProviderUnavailability;
use pale_core::records::{
    AgentSurfacePlanV1, ArtifactDigest, MaterializationStateTransitionV1, MaterializationStateV1,
    SurfaceOwnership,
};
use pale_sdk::{
    AuthoringIdentityKind, AuthoringInspectionRequest, AuthoringInspectionState,
    AuthoringInstancePathSegment, AuthoringLiteral, AuthoringNavigationRequest,
    AuthoringOccurrencePageRequest, AuthoringOccurrenceRole, AuthoringPathSegment,
    CompilationDocument, DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticRetry,
    DiagnosticStage, EffectiveProfileCompilationRequest, EffectiveProfileDeclaredSelection,
    EffectiveProfileDeclaredSelector, EffectiveProfileEntryDetail,
    EffectiveProfileExplanationEvidence, EffectiveProfileFinding, EffectiveProfileNarrowingReason,
    EffectiveProfileNativeRealization, EffectiveProfileOperationExposure,
    EffectiveProfilePageRequest, EffectiveProfilePolicyBlock, EffectiveProfileProviderDisposition,
    EffectiveProfileRequirementDisposition, EffectiveProfileResourceKind, EffectiveProfileSection,
    EffectiveProfileSelection, ObservationCancellation, ObservationCategory,
    ObservationCausalSelector, ObservationDeltaOutcome, ObservationDeltaRequest, ObservationFact,
    ObservationFilter, ObservationGeneration, ObservationLensDisposition, ObservationLensRequest,
    ObservationPage, ObservationPageDisposition, ObservationPageRequest,
    ObservationResynchronizationReason, ObservationSnapshot, ObservationSubjectKind,
    OneShotBaselineEntry, OneShotBaselineObservation, OneShotCompilation,
    OneShotCompilationRequest, OneShotObservationTarget, OneShotPlanCandidate, OneShotPlanKind,
    OneShotPlanningRequest, OneShotPriorMaterialization, OneShotPrivateInputs, PaleApi,
    PortableWorkspaceDocument, PortableWorkspaceRequest,
};

macro_rules! captured_fixture {
    ($member:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cross-leaf.v1/one-shot/",
            $member
        ))
    };
}

#[path = "internal/observation_profile_compiled.rs"]
mod observation_profile_compiled;

#[path = "one_shot/missing_execution_paths.rs"]
mod missing_execution_paths;

#[path = "one_shot/session_diagnostics.rs"]
mod session_diagnostics;

fn document(label: &str, bytes: &[u8]) -> CompilationDocument {
    CompilationDocument::new(label.to_string(), bytes.to_vec())
}

fn ready_observation_page(
    snapshot: &ObservationSnapshot,
    request: ObservationPageRequest,
) -> ObservationPage {
    match snapshot
        .page(request)
        .expect("the observation cursor remains compatible")
    {
        ObservationPageDisposition::Ready { page } => page,
        ObservationPageDisposition::Cancelled { .. } => {
            panic!("an active observation request cannot be cancelled")
        }
    }
}

fn captured_workspace(reverse_source_order: bool) -> PortableWorkspaceRequest {
    captured_workspace_with_access(reverse_source_order, "mutate")
}

fn captured_workspace_with_access(
    reverse_source_order: bool,
    access: &str,
) -> PortableWorkspaceRequest {
    assert!(matches!(access, "read" | "mutate"));
    let manifest = String::from_utf8(captured_fixture!("workspace/pale-workspace.toml").to_vec())
        .expect("workspace fixture is UTF-8");
    assert!(manifest.contains("access = \"mutate\""));
    let manifest = manifest.replace("access = \"mutate\"", &format!("access = \"{access}\""));
    let mut documents = vec![
        PortableWorkspaceDocument::new(
            "composition.toml".to_string(),
            captured_fixture!("workspace/composition.toml").to_vec(),
        ),
        PortableWorkspaceDocument::new("pale-workspace.toml".to_string(), manifest.into_bytes()),
        PortableWorkspaceDocument::new(
            "workspace-resolution.lock.toml".to_string(),
            captured_fixture!("workspace/workspace-resolution.lock.toml").to_vec(),
        ),
        PortableWorkspaceDocument::new(
            "workspace-universe.lock.toml".to_string(),
            captured_fixture!("workspace/workspace-universe.lock.toml").to_vec(),
        ),
    ];
    if reverse_source_order {
        documents.reverse();
    }
    PortableWorkspaceRequest::new("pale-workspace.toml".to_string(), documents)
}

fn placeholder_private() -> OneShotPrivateInputs {
    OneShotPrivateInputs::new(
        document("capabilities.toml", b"capabilities-secret"),
        document("machines/machine.toml", b"machine-secret"),
        document("distribution-lock.toml", b"lock-secret"),
        Vec::new(),
    )
}

fn captured_private() -> OneShotPrivateInputs {
    OneShotPrivateInputs::new(
        document(
            "capabilities.toml",
            captured_fixture!("private/capabilities.toml"),
        ),
        document(
            "machines/machine.toml",
            captured_fixture!("private/machine-binding.toml"),
        ),
        document(
            "distribution-lock.toml",
            captured_fixture!("private/distribution-lock.toml"),
        ),
        Vec::new(),
    )
}

fn captured_private_with_provider_repositories() -> OneShotPrivateInputs {
    let machine = String::from_utf8(captured_fixture!("private/machine-binding.toml").to_vec())
        .expect("machine fixture is UTF-8");
    let projections = r#"[[provider_repositories]]
repository_id = "engine"
provider_binding_id = "provider-binding-claude-code"
native_repository_coordinate = "owner://rig/repository"

[[provider_repositories]]
repository_id = "engine"
provider_binding_id = "provider-binding-unselected"
native_repository_coordinate = "owner://unselected/repository"

"#;
    let machine = machine.replace(
        "[authority_roots]",
        &format!("{projections}[authority_roots]"),
    );
    OneShotPrivateInputs::new(
        document(
            "capabilities.toml",
            captured_fixture!("private/capabilities.toml"),
        ),
        CompilationDocument::new("machines/machine.toml".into(), machine.into_bytes()),
        document(
            "distribution-lock.toml",
            captured_fixture!("private/distribution-lock.toml"),
        ),
        Vec::new(),
    )
}

fn captured_private_with_declared_role_wildcard() -> OneShotPrivateInputs {
    let capabilities = String::from_utf8(captured_fixture!("private/capabilities.toml").to_vec())
        .expect("capabilities fixture is UTF-8");
    let capabilities = format!(
        "[semantic]\nroles = [{{ selection = \"all_from_package\", package_id = \"example.engine\" }}]\n\n{capabilities}"
    );
    OneShotPrivateInputs::new(
        CompilationDocument::new("capabilities.toml".into(), capabilities.into_bytes()),
        document(
            "machines/machine.toml",
            captured_fixture!("private/machine-binding.toml"),
        ),
        document(
            "distribution-lock.toml",
            captured_fixture!("private/distribution-lock.toml"),
        ),
        Vec::new(),
    )
}

fn captured_private_with_capability_and_machine_bodies_swapped() -> OneShotPrivateInputs {
    OneShotPrivateInputs::new(
        document(
            "capabilities.toml",
            captured_fixture!("private/machine-binding.toml"),
        ),
        document(
            "machines/machine.toml",
            captured_fixture!("private/capabilities.toml"),
        ),
        document(
            "distribution-lock.toml",
            captured_fixture!("private/distribution-lock.toml"),
        ),
        Vec::new(),
    )
}

fn captured_request(reverse_source_order: bool) -> OneShotCompilationRequest {
    captured_request_with_workspace(
        reverse_source_order,
        captured_workspace(reverse_source_order),
    )
}

fn captured_request_with_workspace(
    reverse_source_order: bool,
    workspace: PortableWorkspaceRequest,
) -> OneShotCompilationRequest {
    let package_documents = captured_package_documents(reverse_source_order);
    OneShotCompilationRequest::new(
        "demo".to_string(),
        "default".to_string(),
        "machine".to_string(),
        captured_private(),
        package_documents,
        workspace,
    )
}

fn captured_package_documents(reverse_source_order: bool) -> Vec<CompilationDocument> {
    let mut package_documents = vec![
        document(
            "engine/package.toml",
            captured_fixture!("packages/engine/package.toml"),
        ),
        document(
            "engine/roles.toml",
            captured_fixture!("packages/engine/roles.toml"),
        ),
        document(
            "engine/role.md",
            captured_fixture!("packages/engine/role.md"),
        ),
    ];
    if reverse_source_order {
        package_documents.reverse();
    }
    package_documents
}

fn captured_effective_profile_request(
    reverse_source_order: bool,
) -> EffectiveProfileCompilationRequest {
    EffectiveProfileCompilationRequest::new(
        "demo".to_string(),
        "default".to_string(),
        "machine".to_string(),
        captured_private(),
        captured_package_documents(reverse_source_order),
    )
}

fn compile_fixture(api: &PaleApi, reverse_source_order: bool) -> OneShotCompilation {
    api.compile_one_shot(captured_request(reverse_source_order))
        .expect("the shared captured one-shot fixture compiles")
}

fn authoring_field(value: &str) -> AuthoringInstancePathSegment {
    AuthoringInstancePathSegment::Field(value.to_owned())
}

fn absent_baseline(targets: &[OneShotObservationTarget]) -> Vec<OneShotBaselineEntry> {
    targets
        .iter()
        .map(|target| {
            OneShotBaselineEntry::new(
                target.relative_path().to_string(),
                OneShotBaselineObservation::Absent,
            )
        })
        .collect()
}

fn decode_candidate(candidate: &OneShotPlanCandidate) -> AgentSurfacePlanV1 {
    let value = pale_core::canon::parse_strict(candidate.canonical_bytes())
        .expect("candidate bytes are strict canonical JSON");
    pale_contracts::validate_value(&value, "agent-surface-plan.v1")
        .expect("candidate bytes satisfy the existing plan contract");
    serde_json::from_value(value).expect("candidate bytes decode into the typed plan")
}

fn prior_bytes(plan: &AgentSurfacePlanV1) -> Vec<u8> {
    pale_core::canon::to_canonical_string(
        plan.transition
            .next_state()
            .expect("an initial apply publishes next ownership state"),
    )
    .expect("next ownership state has canonical bytes")
    .into_bytes()
}

fn present_baseline_from_claimed_files(
    targets: &[OneShotObservationTarget],
    state: &MaterializationStateV1,
) -> Vec<OneShotBaselineEntry> {
    targets
        .iter()
        .map(|target| {
            let surface = state
                .surfaces
                .iter()
                .find(|surface| surface.rel_path == target.relative_path())
                .expect("every required target occurs in the prior state");
            let complete_file_sha256 = match surface.ownership {
                SurfaceOwnership::WholeFile => surface.sha256.clone(),
                SurfaceOwnership::ManagedBlock => ArtifactDigest::hash_bytes(
                    format!("claimed complete file for {}", target.relative_path()).as_bytes(),
                ),
            };
            OneShotBaselineEntry::new(
                target.relative_path().to_string(),
                OneShotBaselineObservation::Present {
                    sha256: complete_file_sha256.as_str().to_string(),
                    unix_mode: 0o644,
                },
            )
        })
        .collect()
}

fn assert_invalid_planning_claim(error: &pale_sdk::ApiError) {
    assert!(matches!(
        error.diagnostic().code(),
        "one_shot_planning_input_rejected" | "one_shot_plan_rejected"
    ));
    assert_eq!(error.diagnostic().stage(), DiagnosticStage::Planning);
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert_eq!(error.diagnostic().retry(), DiagnosticRetry::AfterCorrection);
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
    assert!(std::error::Error::source(error).is_none());
}

#[test]
fn captured_success_is_deterministic_order_independent_and_not_live_ready() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = compile_fixture(&api, false);
    let permuted = compile_fixture(&api, true);
    let cloned = compilation.clone();
    let summary = compilation.summary();

    assert_eq!(summary, permuted.summary());
    assert_eq!(summary, cloned.summary());
    assert_eq!(
        summary.canonical_bytes(),
        permuted.summary().canonical_bytes()
    );
    let canonical_summary = pale_core::canon::parse_strict(summary.canonical_bytes())
        .expect("the API summary must remain strict canonical JSON");
    assert_eq!(canonical_summary["project_id"], "demo");
    assert_eq!(canonical_summary["execution_workspace_id"], "default");
    assert_eq!(canonical_summary["repository_count"], 1);
    assert_eq!(
        canonical_summary["repositories"][0]["repository_id"],
        "engine"
    );
    assert_eq!(summary.project_id(), "demo");
    assert_eq!(summary.execution_workspace_id(), "default");
    assert_eq!(summary.repository_count(), 1);
    assert_eq!(summary.repositories().len(), 1);
    assert_eq!(summary.package_release_count(), 1);
    assert_eq!(summary.normative_pack_release_count(), 0);
    assert!(summary.package_semantic_record_count() > 0);
    assert_eq!(summary.workspace_admission_evidence_count(), 0);
    assert!(summary.executable_identity_digest().is_none());

    assert_ne!(
        summary.workspace_resolution_lock_digest(),
        summary.workspace_universe_lock_digest()
    );
    assert_ne!(
        summary.responsibility_derivation_closure_digest(),
        summary.organizational_translation_closure_digest()
    );
    assert_ne!(
        summary.experience_composition_source_digest(),
        summary.experience_composition_plan_digest()
    );

    let repository = &summary.repositories()[0];
    assert_eq!(repository.repository_id(), "engine");
    assert_eq!(repository.role_count(), 1);
    assert_eq!(repository.operation_count(), 0);
    assert!(repository.surface_count() > 0);
    assert_eq!(repository.deferred_surface_count(), 0);
    assert_eq!(repository.execution_path_count(), 1);
    assert_eq!(repository.execution_paths().len(), 1);
    let path = &repository.execution_paths()[0];
    assert_eq!(path.selection_id(), "direct-claude");
    assert_eq!(path.experience_path_id(), "direct-claude-code");
    let terminal_provider = path
        .terminal_provider_binding()
        .expect("terminal path must retain its exact provider binding");
    assert_eq!(
        terminal_provider.provider_binding_id,
        "provider-binding-claude-code"
    );
    assert_eq!(terminal_provider.provider_binding_identity.len(), 64);
    assert_eq!(
        path.composition_plan_digest(),
        summary.experience_composition_plan_digest()
    );
    assert!(!path.responsibility_path_identity().is_empty());
    assert_eq!(path.work_delegation_edge_ids(), ["delegation-entry-worker"]);
    assert!(!path.is_direct());
    assert!(!path.activation_selected());
    assert!(path.hook_events().is_empty());
    assert!(path.gated_tools().is_empty());
    assert!(repository
        .activation_selected_execution_path_ids()
        .is_empty());
    assert!(repository.activation_selected_hook_events().is_empty());
    assert_eq!(
        repository.surfaces().len(),
        repository.surface_count() as usize
    );

    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<pale_sdk::OneShotCompilationSummary>();
    assert_send_sync::<pale_sdk::RepositorySummary>();
    assert_send_sync::<pale_sdk::ExecutionPathSummary>();

    assert_ne!(summary.profile_digest(), summary.instance_digest());
    assert_ne!(
        summary.package_semantic_ir_digest(),
        summary.workspace_semantic_ir_digest()
    );
    assert_ne!(
        summary.workspace_semantic_ir_digest(),
        summary.principal_admitted_universe_digest()
    );
    assert_ne!(
        summary.compilation_input_digest(),
        summary.one_shot_compilation_digest()
    );
    assert_ne!(
        summary.schema_manifest_sha256(),
        summary.schema_set_sha256()
    );
}

#[test]
fn effective_profile_explanations_do_not_cross_private_instances() {
    let api = PaleApi::new().unwrap();
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .unwrap();
    let baseline = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .unwrap();
    let machine = std::str::from_utf8(captured_fixture!("private/machine-binding.toml"))
        .unwrap()
        .replace(
            "/captured/repository",
            "/captured/repository-other-instance",
        );
    let changed_private = OneShotPrivateInputs::new(
        document(
            "capabilities.toml",
            captured_fixture!("private/capabilities.toml"),
        ),
        document("machines/machine.toml", machine.as_bytes()),
        document(
            "distribution-lock.toml",
            captured_fixture!("private/distribution-lock.toml"),
        ),
        Vec::new(),
    );
    let changed = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".into(),
                "default".into(),
                "machine".into(),
                changed_private,
                captured_package_documents(false),
            ),
        )
        .expect("the alternate private instance is independently valid");
    let mut expected_coordinates = baseline.coordinates();
    assert_ne!(
        expected_coordinates.instance_digest,
        changed.instance_digest()
    );
    expected_coordinates.instance_digest = changed.instance_digest().to_owned();
    assert_eq!(
        expected_coordinates,
        changed.coordinates(),
        "only private instance differs"
    );
    let request =
        || EffectiveProfilePageRequest::new(EffectiveProfileSection::MachineRealization, 0, 1);
    let prior_page = baseline.inspect(request());
    let changed_page = changed.inspect(request());
    let prior = &prior_page.entries()[0];
    let current = &changed_page.entries()[0];
    assert_eq!(
        prior.detail(),
        current.detail(),
        "native realization stays unobserved"
    );
    assert!(baseline.explain(prior.explanation_handle()).is_some());
    assert!(changed.explain(current.explanation_handle()).is_some());
    assert!(
        changed.explain(prior.explanation_handle()).is_none(),
        "a prior private-instance handle must not resolve"
    );
    assert!(baseline.explain(current.explanation_handle()).is_none());

    let snapshot = api.observe_effective_profile(&baseline);
    let subject = machine_observation_subject(&snapshot);
    assert_eq!(
        snapshot
            .inspect_profile(&changed, &workspace, &subject, None, 1, 1)
            .unwrap_err(),
        pale_sdk::ObservationProfileError::CoordinateMismatch,
    );
    assert_eq!(
        snapshot
            .associate_profile(&changed, &workspace)
            .unwrap_err(),
        pale_sdk::ObservationProfileError::CoordinateMismatch,
    );
}

fn machine_observation_subject(
    snapshot: &pale_sdk::ObservationSnapshot,
) -> pale_sdk::ObservationSubject {
    ready_observation_page(
        snapshot,
        ObservationPageRequest::new(
            ObservationFilter::all().with_category(ObservationCategory::Health),
            256,
        ),
    )
    .facts
    .into_iter()
    .find_map(|fact| match fact {
        ObservationFact::Health { subject, .. }
            if subject.kind() == ObservationSubjectKind::Machine =>
        {
            Some(subject)
        }
        _ => None,
    })
    .expect("fixture has one configured machine health subject")
}

#[test]
fn observation_profile_association_keeps_handles_bounds_and_shared_source_not_profile_allocation() {
    use pale_sdk::ObservationProfileError;
    let api = PaleApi::new().unwrap();
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .unwrap();
    let profile = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .unwrap();
    let rebuilt = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .unwrap();
    assert_eq!(profile.coordinates(), rebuilt.coordinates());
    let snapshot = api.observe_effective_profile(&profile);
    let subject = machine_observation_subject(&snapshot);
    let clone = snapshot.clone();
    let foreign_snapshot = api.observe_effective_profile(&profile);
    let foreign_workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .unwrap();
    let foreign_profile = api
        .compile_effective_profile(
            &foreign_workspace,
            captured_effective_profile_request(false),
        )
        .unwrap();
    assert_eq!(profile.coordinates(), foreign_profile.coordinates());
    let association = snapshot.associate_profile(&profile, &workspace).unwrap();
    assert_eq!(association.coordinates(), &profile.coordinates());
    assert_eq!(
        association.cursor().generation(),
        snapshot.cursor().generation()
    );
    assert_eq!(
        association.cursor().revision(),
        snapshot.cursor().revision()
    );
    assert!(association.belongs_to(&snapshot));
    assert!(association.belongs_to(&clone));
    assert!(!association.belongs_to(&foreign_snapshot));
    assert_eq!(
        association.cursor().generation(),
        foreign_snapshot.cursor().generation()
    );
    assert_eq!(
        association.cursor().revision(),
        foreign_snapshot.cursor().revision()
    );
    assert_eq!(
        snapshot
            .associate_profile(&foreign_profile, &foreign_workspace)
            .unwrap_err(),
        ObservationProfileError::SourceMismatch
    );
    assert_eq!(
        snapshot
            .associate_profile(&profile, &foreign_workspace)
            .unwrap_err(),
        ObservationProfileError::SourceMismatch
    );
    assert_eq!(
        snapshot
            .inspect_profile(&foreign_profile, &foreign_workspace, &subject, None, 1, 1)
            .unwrap_err(),
        ObservationProfileError::SourceMismatch
    );
    assert_eq!(
        snapshot
            .inspect_profile(&profile, &foreign_workspace, &subject, None, 1, 1)
            .unwrap_err(),
        ObservationProfileError::SourceMismatch
    );
    for (inspected, returned) in [(0, 1), (1, 0), (257, 1), (1, 257)] {
        assert_eq!(
            snapshot
                .inspect_profile(&profile, &workspace, &subject, None, inspected, returned)
                .unwrap_err(),
            ObservationProfileError::InvalidLimit
        );
    }
    let first = snapshot
        .inspect_profile(&profile, &workspace, &subject, None, 1, 1)
        .unwrap();
    assert_eq!(first.inspected, 1);
    assert!(
        first.entries.is_empty(),
        "the first unrelated row consumes the entire raw-work budget"
    );
    let retained_cursor = first.continuation.clone().unwrap();
    let other_subject = ready_observation_page(
        &snapshot,
        ObservationPageRequest::new(ObservationFilter::all(), 256),
    )
    .facts
    .into_iter()
    .find_map(|fact| match fact {
        ObservationFact::Readiness { subject, .. }
            if subject.kind() == ObservationSubjectKind::EffectiveProfile =>
        {
            Some(subject)
        }
        _ => None,
    })
    .unwrap();
    assert_eq!(
        snapshot
            .inspect_profile(
                &profile,
                &workspace,
                &other_subject,
                Some(&retained_cursor),
                1,
                1
            )
            .unwrap_err(),
        ObservationProfileError::ContinuationMismatch
    );
    assert_eq!(
        foreign_snapshot
            .inspect_profile(&profile, &workspace, &subject, Some(&retained_cursor), 1, 1)
            .unwrap_err(),
        ObservationProfileError::ContinuationMismatch
    );
    assert_eq!(
        snapshot
            .inspect_profile(&profile, &workspace, &subject, Some(&retained_cursor), 2, 1)
            .unwrap_err(),
        ObservationProfileError::ContinuationMismatch
    );
    let mut continuation = first.continuation;
    let mut actual = first.entries;
    while let Some(cursor) = continuation {
        let page = clone
            .inspect_profile(&rebuilt, &workspace, &subject, Some(&cursor), 1, 1)
            .unwrap();
        assert_eq!(page.inspected, 1);
        assert!(page.entries.len() <= 1);
        actual.extend(page.entries);
        continuation = page.continuation;
    }
    let expected = [
        EffectiveProfileSection::MachineRealization,
        EffectiveProfileSection::GapsAndExplanations,
    ]
    .into_iter()
    .flat_map(|section| {
        profile
            .inspect(EffectiveProfilePageRequest::new(section, 0, 256))
            .entries()
            .to_vec()
    })
    .filter(|entry| {
        matches!(
            entry.detail(),
            EffectiveProfileEntryDetail::MachineRealization { .. }
                | EffectiveProfileEntryDetail::Finding {
                    finding: EffectiveProfileFinding::NativeRealizationUnknown { .. }
                }
        )
    })
    .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 2);
    for entry in &actual {
        assert!(profile.explain(entry.explanation_handle()).is_some());
        assert_eq!(
            profile.explain(entry.explanation_handle()),
            rebuilt.explain(entry.explanation_handle())
        );
    }
    let facts = serde_json::to_string(
        &ready_observation_page(
            &snapshot,
            ObservationPageRequest::new(ObservationFilter::all(), 256),
        )
        .facts,
    )
    .unwrap();
    drop(profile);
    drop(rebuilt);
    drop(workspace);
    assert_eq!(association.coordinates(), &foreign_profile.coordinates());
    assert!(association.belongs_to(&snapshot));
    assert_eq!(
        association.cursor().generation(),
        snapshot.cursor().generation()
    );
    assert_eq!(
        association.cursor().revision(),
        snapshot.cursor().revision()
    );
    assert_eq!(
        snapshot
            .associate_profile(&foreign_profile, &foreign_workspace)
            .unwrap_err(),
        ObservationProfileError::ExpiredSource
    );
    assert_eq!(
        snapshot
            .inspect_profile(
                &foreign_profile,
                &foreign_workspace,
                &subject,
                Some(&retained_cursor),
                0,
                1
            )
            .unwrap_err(),
        ObservationProfileError::InvalidLimit,
        "invalid limits are rejected before an expired source is inspected"
    );
    assert_eq!(
        snapshot
            .inspect_profile(
                &foreign_profile,
                &foreign_workspace,
                &subject,
                Some(&retained_cursor),
                1,
                1
            )
            .unwrap_err(),
        ObservationProfileError::ExpiredSource
    );
    assert_eq!(
        serde_json::to_string(
            &ready_observation_page(
                &snapshot,
                ObservationPageRequest::new(ObservationFilter::all(), 256),
            )
            .facts
        )
        .unwrap(),
        facts
    );
}

#[test]
fn effective_profile_projection_is_bounded_deterministic_and_native_unknown() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");
    let profile = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .expect("effective profile fixture is admitted");
    let permuted_workspace = api
        .compile_portable_workspace(captured_workspace(true))
        .expect("permuted workspace fixture is admitted");
    let permuted = api
        .compile_effective_profile(
            &permuted_workspace,
            captured_effective_profile_request(true),
        )
        .expect("permuted effective profile fixture is admitted");

    assert_eq!(profile.coordinates(), permuted.coordinates());
    assert_eq!(profile.coordinates().workspace_id, "act-ws");
    assert_eq!(profile.coordinates().project_id, "demo");
    assert_eq!(profile.coordinates().execution_workspace_id, "default");
    assert_eq!(profile.coordinates().machine_id, "machine");
    assert_eq!(
        profile.coordinates().instance_digest,
        profile.instance_digest()
    );
    assert_ne!(profile.instance_digest(), profile.profile_digest());
    assert!(!profile
        .coordinates()
        .workspace_semantic_ir_digest
        .is_empty());
    assert!(!profile
        .coordinates()
        .workspace_admission_evidence_digest
        .is_empty());

    for section in [
        EffectiveProfileSection::IntendedUseCase,
        EffectiveProfileSection::EffectiveConfiguration,
        EffectiveProfileSection::CapabilitiesAndOperations,
        EffectiveProfileSection::MachineRealization,
        EffectiveProfileSection::GapsAndExplanations,
    ] {
        assert_eq!(
            profile.inspect(EffectiveProfilePageRequest::new(section, 0, 0)),
            permuted.inspect(EffectiveProfilePageRequest::new(section, 0, 0)),
            "source ordering must not change {section:?}"
        );
    }

    let machine = profile.inspect(EffectiveProfilePageRequest::new(
        EffectiveProfileSection::MachineRealization,
        0,
        1,
    ));
    assert_eq!(machine.next_offset(), None);
    assert!(matches!(
        machine.entries(),
        [entry] if matches!(entry.detail(), EffectiveProfileEntryDetail::MachineRealization {
            realization: EffectiveProfileNativeRealization::Unknown,
            ..
        })
    ));
    let machine_entry = &machine.entries()[0];
    let explanation = profile
        .explain(machine_entry.explanation_handle())
        .expect("every projected row retains one bounded explanation");
    assert_eq!(explanation.handle, *machine_entry.explanation_handle());
    assert!(matches!(
        explanation.evidence,
        EffectiveProfileExplanationEvidence::MachineRealization {
            realization: EffectiveProfileNativeRealization::Unknown
        }
    ));

    let first = profile.inspect(EffectiveProfilePageRequest::new(
        EffectiveProfileSection::EffectiveConfiguration,
        0,
        1,
    ));
    assert_eq!(first.offset(), 0);
    assert_eq!(first.entries().len(), 1);
    assert_eq!(first.next_offset(), Some(1));
    let terminal = profile.inspect(EffectiveProfilePageRequest::new(
        EffectiveProfileSection::EffectiveConfiguration,
        u32::MAX,
        1,
    ));
    assert_eq!(terminal.offset(), u32::MAX);
    assert!(terminal.entries().is_empty());
    assert_eq!(terminal.next_offset(), None);
    assert_eq!(
        profile.inspect(EffectiveProfilePageRequest::new(
            EffectiveProfileSection::EffectiveConfiguration,
            0,
            257,
        )),
        profile.inspect(EffectiveProfilePageRequest::new(
            EffectiveProfileSection::EffectiveConfiguration,
            0,
            256,
        ))
    );

    let page = profile.inspect(EffectiveProfilePageRequest::new(
        EffectiveProfileSection::EffectiveConfiguration,
        0,
        1,
    ));
    let value = serde_json::to_value(&page).expect("public profile page serializes");
    assert_eq!(
        pale_core::canon::parse_strict(page.canonical_bytes()).expect("page is canonical JSON"),
        value
    );
    assert_eq!(
        pale_core::canon::to_canonical_string(&value).expect("value canonicalizes"),
        std::str::from_utf8(page.canonical_bytes()).expect("canonical page is UTF-8")
    );
    assert_eq!(value["section"], "effective-configuration");
    assert!(value["entries"][0]["kind"].is_string());
    assert!(value["entries"][0].get("nativeRealization").is_some());
    assert!(value["entries"][0].get("label").is_none());
    assert!(value["entries"][0].get("detail").is_none());
}

#[test]
fn effective_profile_projects_only_path_selected_provider_repositories_and_source_joins() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");
    let profile = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".into(),
                "default".into(),
                "machine".into(),
                captured_private_with_provider_repositories(),
                captured_package_documents(false),
            ),
        )
        .expect("selected provider repository is admitted");

    let page = profile.inspect(EffectiveProfilePageRequest::new(
        EffectiveProfileSection::EffectiveConfiguration,
        0,
        256,
    ));
    let projections = page
        .entries()
        .iter()
        .filter_map(|entry| match entry.detail() {
            EffectiveProfileEntryDetail::ProviderRepository {
                repository_id,
                provider_binding_id,
                native_repository_coordinate,
                native_realization,
            } => Some((
                repository_id.as_str(),
                provider_binding_id.as_str(),
                native_repository_coordinate.as_str(),
                *native_realization,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        projections,
        [(
            "engine",
            "provider-binding-claude-code",
            "owner://rig/repository",
            EffectiveProfileNativeRealization::Unknown,
        )]
    );

    let provider = profile.navigate_authoring(AuthoringNavigationRequest::new(
        "composition.toml".to_owned(),
        vec![
            authoring_field("declarations"),
            authoring_field("selected_experience_paths"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("provider_binding_ids"),
            AuthoringInstancePathSegment::Index(0),
        ],
    ));
    let provider_identity = provider.identity().expect("provider binding resolves");
    assert_eq!(
        provider_identity.kind(),
        AuthoringIdentityKind::CompositionProviderBinding
    );
    assert!(provider.references().iter().any(|reference| {
        reference.source_label() == "machines/machine.toml"
            && reference.path()
                == [
                    authoring_field("provider_repositories"),
                    AuthoringInstancePathSegment::Index(0),
                    authoring_field("provider_binding_id"),
                ]
    }));
    let provider_profile = profile.inspect_effective_authoring(provider_identity);
    assert!(provider_profile.entries().iter().any(|entry| matches!(
        entry.detail(),
        EffectiveProfileEntryDetail::ExecutionPath { provider_binding_ids, .. }
            if provider_binding_ids == &["provider-binding-claude-code"]
    )));
    assert!(provider_profile.entries().iter().any(|entry| matches!(
        entry.detail(),
        EffectiveProfileEntryDetail::ProviderRepository {
            provider_binding_id,
            native_repository_coordinate,
            ..
        } if provider_binding_id == "provider-binding-claude-code"
            && native_repository_coordinate == "owner://rig/repository"
    )));

    let delegation = profile.navigate_authoring(AuthoringNavigationRequest::new(
        "composition.toml".to_owned(),
        vec![
            authoring_field("declarations"),
            authoring_field("selected_experience_paths"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("work_delegation_edge_ids"),
            AuthoringInstancePathSegment::Index(0),
        ],
    ));
    let delegation_identity = delegation.identity().expect("delegation edge resolves");
    assert_eq!(
        delegation_identity.kind(),
        AuthoringIdentityKind::CompositionEdge
    );
    let delegation_profile = profile.inspect_effective_authoring(delegation_identity);
    assert!(delegation_profile.entries().iter().any(|entry| matches!(
        entry.detail(),
        EffectiveProfileEntryDetail::ExecutionPath { work_delegation_edge_ids, .. }
            if work_delegation_edge_ids == &["delegation-entry-worker"]
    )));
}

#[test]
fn observation_facade_preserves_bounds_coordinates_and_unknown_native_truth() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");
    let profile = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .expect("effective profile fixture is admitted");
    let snapshot = api.observe_effective_profile(&profile);

    let summary = snapshot.summary();
    assert!(summary.fact_count > 0);
    assert_eq!(summary.omitted_fact_count, 0);
    let ObservationGeneration::EffectiveProfile {
        profile_digest,
        instance_digest,
        composition_plan_digest,
    } = summary.cursor.generation()
    else {
        panic!("effective-profile observation used a resident generation");
    };
    assert_eq!(profile_digest.len(), 64);
    assert_eq!(instance_digest.len(), 64);
    assert_ne!(profile_digest, instance_digest);
    assert_eq!(
        composition_plan_digest,
        &profile.coordinates().composition_plan_digest,
    );

    let first = ready_observation_page(
        &snapshot,
        ObservationPageRequest::new(ObservationFilter::all(), 1),
    );
    assert_eq!(first.facts.len(), 1);
    assert!(first.continuation.is_some());
    let continued = ready_observation_page(
        &snapshot,
        ObservationPageRequest::new(ObservationFilter::all(), 1)
            .after(first.continuation.expect("more facts remain")),
    );
    assert_eq!(continued.facts.len(), 1);

    let foreign_filter_cursor = ready_observation_page(
        &snapshot,
        ObservationPageRequest::new(ObservationFilter::all(), 1),
    )
    .continuation
    .expect("more facts remain");
    let mismatched_page = snapshot
        .page(
            ObservationPageRequest::new(
                ObservationFilter::all().with_category(ObservationCategory::Health),
                1,
            )
            .after(foreign_filter_cursor),
        )
        .expect_err("a continuation cannot cross filters");
    assert!(matches!(
        mismatched_page.reason,
        ObservationResynchronizationReason::ContinuationMismatch
    ));

    let native = ready_observation_page(
        &snapshot,
        ObservationPageRequest::new(
            ObservationFilter::all().with_category(ObservationCategory::NativeOwner),
            0,
        ),
    );
    assert!(native.facts.iter().all(|fact| matches!(
        fact,
        ObservationFact::NativeOwner {
            state: pale_sdk::ObservationNativeState::Unknown
                | pale_sdk::ObservationNativeState::Unavailable,
            ..
        }
    )));
    let native_subject = native
        .facts
        .iter()
        .find_map(|fact| match fact {
            ObservationFact::NativeOwner { subject, .. } => Some(subject),
            _ => None,
        })
        .expect("the native page carries an admitted opaque subject");
    assert!(matches!(
        native_subject.kind(),
        ObservationSubjectKind::Machine | ObservationSubjectKind::CapabilityRequirement
    ));
    let subject_page = ready_observation_page(
        &snapshot,
        ObservationPageRequest::new(ObservationFilter::all().with_subject(native_subject), 0),
    );
    assert!(!subject_page.facts.is_empty());

    assert!(matches!(
        snapshot.delta_from(
            &snapshot,
            ObservationDeltaRequest::new(
                snapshot.resume_coordinate(),
                8,
                ObservationCancellation::Active,
            ),
        ),
        ObservationDeltaOutcome::CaughtUp { .. }
    ));
    assert!(matches!(
        snapshot.delta_from(
            &snapshot,
            ObservationDeltaRequest::new(
                snapshot.resume_coordinate(),
                8,
                ObservationCancellation::Cancelled,
            ),
        ),
        ObservationDeltaOutcome::Cancelled { .. }
    ));

    let all = ready_observation_page(
        &snapshot,
        ObservationPageRequest::new(ObservationFilter::all(), 0),
    );
    let path_subject = all
        .facts
        .iter()
        .find_map(|fact| match fact {
            ObservationFact::Readiness { subject, .. }
                if subject.kind() == ObservationSubjectKind::ExecutionPath =>
            {
                Some(subject)
            }
            _ => None,
        })
        .expect("the fixture retains one execution-path subject");
    let selector = ObservationCausalSelector::new(&snapshot, path_subject);
    assert!(snapshot
        .causal_path(&selector)
        .expect("same-generation selector is valid")
        .is_some());

    let read_workspace = api
        .compile_portable_workspace(captured_workspace_with_access(false, "read"))
        .expect("read-only workspace fixture is admitted");
    let read_profile = api
        .compile_effective_profile(&read_workspace, captured_effective_profile_request(false))
        .expect("read-only effective profile fixture is admitted");
    let read_snapshot = api.observe_effective_profile(&read_profile);
    let cross_generation = ObservationCausalSelector::new(&read_snapshot, path_subject);
    let mismatch = snapshot
        .causal_path(&cross_generation)
        .expect_err("a causal selector cannot cross admitted generations");
    assert!(matches!(
        mismatch.reason,
        ObservationResynchronizationReason::GenerationMismatch { .. }
    ));
    assert!(matches!(
        snapshot.visual_lens(ObservationLensRequest::new(
            ObservationFilter::all(),
            1,
            false,
            ObservationCancellation::Active,
        )),
        ObservationLensDisposition::Ready { ref lens }
            if lens.nodes.len() == 1
                && !lens.nodes[0].facts.is_empty()
                && lens.continuation.is_some()
                && lens.snapshot_omitted_count == 0
    ));
    assert!(matches!(
        snapshot.visual_lens(ObservationLensRequest::new(
            ObservationFilter::all(),
            1,
            true,
            ObservationCancellation::Cancelled,
        )),
        ObservationLensDisposition::Cancelled { .. }
    ));

    let value = serde_json::to_value(&native).expect("owned observation page serializes");
    assert!(value["facts"]
        .as_array()
        .is_some_and(|facts| !facts.is_empty()));
    assert!(value["facts"][0]["subject"]["kind"].is_string());
}

#[test]
fn authoring_projection_uses_exact_admitted_labels_and_generated_variant_docs() {
    let api = PaleApi::new().expect("API initializes");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("portable workspace compiles");
    let profile = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .expect("effective profile compiles");

    let visibility = profile
        .inspect_authoring(AuthoringInspectionRequest::new(
            "pale-workspace.toml".into(),
            vec![
                AuthoringPathSegment::Field("workspace".into()),
                AuthoringPathSegment::Field("desired_topology".into()),
                AuthoringPathSegment::Field("execution_workspace_repositories".into()),
                AuthoringPathSegment::ArrayItem,
                AuthoringPathSegment::Field("access".into()),
            ],
        ))
        .expect("generated metadata is coherent");
    assert_eq!(visibility.state(), AuthoringInspectionState::Available);
    assert_eq!(visibility.schema_id(), Some("pale-workspace.v1"));
    let values = visibility
        .node()
        .expect("available node")
        .allowed_values()
        .iter()
        .map(|value| value.value())
        .collect::<Vec<_>>();
    assert!(values.contains(&&AuthoringLiteral::String("read".into())));
    assert!(values.contains(&&AuthoringLiteral::String("mutate".into())));

    let machine = profile
        .inspect_authoring(AuthoringInspectionRequest::new(
            "machines/machine.toml".into(),
            Vec::new(),
        ))
        .expect("machine grammar is retained");
    assert_eq!(machine.state(), AuthoringInspectionState::Available);
    assert_eq!(machine.schema_id(), Some("machine-binding.v1"));

    let markdown = profile
        .inspect_authoring(AuthoringInspectionRequest::new(
            "engine/role.md".into(),
            Vec::new(),
        ))
        .expect("text custody is retained");
    assert_eq!(markdown.state(), AuthoringInspectionState::TextDocument);
}

#[test]
fn effective_profile_axes_project_exhaustively_without_flattening() {
    let operation_cases = [
        (
            ApplicationOperationExposure::Unbound,
            EffectiveProfileOperationExposure::Unbound,
        ),
        (
            ApplicationOperationExposure::SemanticOnly,
            EffectiveProfileOperationExposure::SemanticOnly,
        ),
        (
            ApplicationOperationExposure::CapabilityBearing,
            EffectiveProfileOperationExposure::CapabilityBearing,
        ),
    ];
    for (application, projected) in operation_cases {
        assert_eq!(
            EffectiveProfileOperationExposure::from(application),
            projected
        );
    }

    let requirement_cases = [
        (
            ApplicationRequirementDisposition::Satisfied,
            EffectiveProfileRequirementDisposition::Satisfied,
        ),
        (
            ApplicationRequirementDisposition::ProviderBindingAbsent,
            EffectiveProfileRequirementDisposition::ProviderBindingAbsent,
        ),
        (
            ApplicationRequirementDisposition::Narrowed(
                ApplicationNarrowingReason::CapabilityNotPermitted,
            ),
            EffectiveProfileRequirementDisposition::Narrowed {
                reason: EffectiveProfileNarrowingReason::CapabilityNotPermitted,
            },
        ),
        (
            ApplicationRequirementDisposition::PolicyBlocked(
                ApplicationPolicyBlock::CapabilityForbidden,
            ),
            EffectiveProfileRequirementDisposition::Blocked {
                reason: EffectiveProfilePolicyBlock::CapabilityForbidden,
            },
        ),
    ];
    for (application, projected) in requirement_cases {
        assert_eq!(
            EffectiveProfileRequirementDisposition::from(application),
            projected
        );
    }

    assert_eq!(
        EffectiveProfileProviderDisposition::from(ApplicationProviderDisposition::Unbound(
            ProviderUnavailability::MissingBinding,
        )),
        EffectiveProfileProviderDisposition::MissingBinding
    );
    assert_eq!(
        EffectiveProfileProviderDisposition::from(ApplicationProviderDisposition::Unbound(
            ProviderUnavailability::ExcludedByNarrowing,
        )),
        EffectiveProfileProviderDisposition::ExcludedByNarrowing
    );

    let finding = EffectiveProfileEntryDetail::Finding {
        finding: EffectiveProfileFinding::Requirement {
            repository_id: "engine".into(),
            operation_id: "engine:operations/inspect".into(),
            requirement_id: "inspect".into(),
            capability_id: "inspection".into(),
            disposition: EffectiveProfileRequirementDisposition::ProviderBindingAbsent,
        },
    };
    let value = serde_json::to_value(finding).expect("structured finding serializes");
    assert_eq!(value["kind"], "finding");
    assert_eq!(value["finding"]["kind"], "requirement");
    assert_eq!(
        value["finding"]["disposition"]["kind"],
        "provider-binding-absent"
    );
    assert!(value.get("label").is_none());
    assert!(value.get("detail").is_none());
}

#[test]
fn admitted_topology_navigation_resolves_definition_and_references() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");
    let navigation = workspace.navigate_authoring(AuthoringNavigationRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            authoring_field("workspace"),
            authoring_field("desired_topology"),
            authoring_field("project_execution_workspaces"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("project_id"),
        ],
    ));

    assert_eq!(
        navigation.identity().map(|identity| identity.value()),
        Some("demo")
    );
    let definition = navigation.definition().expect("project definition exists");
    assert_eq!(definition.source_label(), "pale-workspace.toml");
    assert_eq!(
        definition.path(),
        &[
            authoring_field("workspace"),
            authoring_field("desired_topology"),
            authoring_field("projects"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("id"),
        ]
    );
    assert!(navigation.references().iter().any(|location| {
        location.path()
            == [
                authoring_field("workspace"),
                authoring_field("desired_topology"),
                authoring_field("project_repositories"),
                AuthoringInstancePathSegment::Index(0),
                authoring_field("project_id"),
            ]
    }));
    assert!(!navigation.references_truncated());
    assert!(!navigation.alternatives_truncated());
}

#[test]
fn admitted_source_and_composition_coordinates_resolve_without_retaining_source_bytes() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");

    let source = workspace.navigate_authoring(AuthoringNavigationRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            authoring_field("workspace"),
            authoring_field("experience_compositions"),
            AuthoringInstancePathSegment::Index(0),
        ],
    ));
    assert_eq!(
        source.identity().map(|identity| identity.kind()),
        Some(AuthoringIdentityKind::SourceDocument)
    );
    let definition = source.definition().expect("source document exists");
    assert_eq!(definition.source_label(), "composition.toml");
    assert!(definition.path().is_empty());

    let path = workspace.navigate_authoring(AuthoringNavigationRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            authoring_field("workspace"),
            authoring_field("desired_topology"),
            authoring_field("execution_paths"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("experience_path_id"),
        ],
    ));
    assert_eq!(
        path.identity().map(|identity| identity.kind()),
        Some(AuthoringIdentityKind::ExperiencePath)
    );
    let definition = path.definition().expect("experience path exists");
    assert_eq!(definition.source_label(), "composition.toml");
    assert_eq!(
        definition.path(),
        &[
            authoring_field("declarations"),
            authoring_field("selected_experience_paths"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("id"),
        ]
    );
}

#[test]
fn admitted_composition_navigation_preserves_participant_and_capability_namespaces() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");

    let participant = workspace.navigate_authoring(AuthoringNavigationRequest::new(
        "composition.toml".to_owned(),
        vec![
            authoring_field("declarations"),
            authoring_field("responsibility_contracts"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("participant"),
            authoring_field("capability_id"),
        ],
    ));
    let participant_identity = participant.identity().expect("participant resolves");
    assert_eq!(
        participant_identity.kind(),
        AuthoringIdentityKind::CompositionParticipant
    );
    assert_eq!(participant_identity.value(), "capability:entry");
    assert_eq!(participant_identity.authored_value(), "entry");
    assert_eq!(
        participant
            .definition()
            .expect("manifest declares the participant")
            .path(),
        &[
            authoring_field("declarations"),
            authoring_field("participant_provider_manifests"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("participants"),
            AuthoringInstancePathSegment::Index(2),
            authoring_field("capability_id"),
        ]
    );
    assert!(participant.references().iter().any(|reference| {
        reference.path()
            == [
                authoring_field("declarations"),
                authoring_field("edge_contracts"),
                AuthoringInstancePathSegment::Index(0),
                authoring_field("from"),
                authoring_field("capability_id"),
            ]
    }));

    let capability = workspace.navigate_authoring(AuthoringNavigationRequest::new(
        "composition.toml".to_owned(),
        vec![
            authoring_field("declarations"),
            authoring_field("selected_experience_paths"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("capability_ceiling"),
            AuthoringInstancePathSegment::Index(0),
        ],
    ));
    assert_eq!(
        capability.identity().map(|identity| identity.kind()),
        Some(AuthoringIdentityKind::Capability)
    );
    assert_eq!(
        capability.identity().map(|identity| identity.value()),
        Some("neutral-capability")
    );
}

#[test]
fn occurrence_pages_preserve_compiler_roles_and_exact_owner() {
    let api = PaleApi::new().unwrap();
    let workspace = std::sync::Arc::new(
        api.compile_portable_workspace(captured_workspace(false))
            .unwrap(),
    );
    let navigation = workspace.navigate_authoring(AuthoringNavigationRequest::new(
        "pale-workspace.toml".into(),
        vec![
            authoring_field("workspace"),
            authoring_field("desired_topology"),
            authoring_field("projects"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("id"),
        ],
    ));
    let identity = navigation.identity().unwrap().clone();
    let request = AuthoringOccurrencePageRequest::new(identity.clone(), 1, 1);
    let first = workspace.authoring_occurrences(request.clone()).unwrap();
    assert_eq!(first.inspected(), 1);
    let cursor = first.continuation().unwrap().clone();
    drop(first);
    let alias = workspace.clone();
    assert!(alias
        .authoring_occurrences(request.clone().after(cursor.clone()))
        .is_ok());

    let rebuilt = api
        .compile_portable_workspace(captured_workspace(true))
        .unwrap();
    assert_eq!(workspace.semantic_ir_digest(), rebuilt.semantic_ir_digest());
    assert!(rebuilt.authoring_occurrences(request.clone()).is_ok());
    let other = workspace
        .navigate_authoring(AuthoringNavigationRequest::new(
            "composition.toml".into(),
            Vec::new(),
        ))
        .identity()
        .unwrap()
        .clone();
    let rejected = [
        rebuilt.authoring_occurrences(request.clone().after(cursor.clone())),
        workspace.authoring_occurrences(
            AuthoringOccurrencePageRequest::new(other, 1, 1).after(cursor.clone()),
        ),
        workspace.authoring_occurrences(
            AuthoringOccurrencePageRequest::new(identity.clone(), 2, 1).after(cursor.clone()),
        ),
        workspace.authoring_occurrences(
            AuthoringOccurrencePageRequest::new(identity.clone(), 1, 2).after(cursor),
        ),
    ];
    for result in rejected {
        let error = result.expect_err("a continuation cannot change owner or query");
        assert_eq!(error.diagnostic().code(), "authoring_continuation_mismatch");
        assert!(error.diagnostic().source().is_none());
        assert!(!error.to_string().contains("pale-workspace.toml"));
    }

    let mut reference = None;
    for (inspected_limit, returned_limit) in [(1, 1), (0, 0), (u16::MAX, u16::MAX)] {
        let request =
            AuthoringOccurrencePageRequest::new(identity.clone(), inspected_limit, returned_limit);
        let mut page = workspace.authoring_occurrences(request.clone()).unwrap();
        let mut occurrences = Vec::new();
        let mut inspected = 0;
        let mut saw_sparse_page = false;
        loop {
            assert!(page.inspected() <= if inspected_limit == 1 { 1 } else { 256 });
            assert!(page.occurrences().len() <= if returned_limit == 1 { 1 } else { 256 });
            inspected += usize::from(page.inspected());
            assert!(inspected < 10_000, "the finite fixture must reach its end");
            saw_sparse_page |= page.occurrences().is_empty() && !page.is_complete();
            for occurrence in page.occurrences() {
                let resolved = workspace.navigate_authoring(AuthoringNavigationRequest::new(
                    occurrence.location().source_label().to_owned(),
                    occurrence.location().path().to_vec(),
                ));
                assert_eq!(resolved.identity(), Some(&identity));
                assert_eq!(resolved.occurrence_role(), Some(occurrence.role()));
            }
            occurrences.extend_from_slice(page.occurrences());
            let Some(cursor) = page.continuation().cloned() else {
                break;
            };
            page = workspace
                .authoring_occurrences(request.clone().after(cursor))
                .unwrap();
        }
        if inspected_limit == 1 {
            assert!(saw_sparse_page);
            assert_eq!(
                page.inspected(),
                0,
                "exact-bound exhaustion needs an empty tail"
            );
        }
        assert_eq!(
            occurrences
                .iter()
                .filter(|item| item.role() == AuthoringOccurrenceRole::Definition)
                .map(|item| item.location())
                .collect::<Vec<_>>(),
            vec![navigation.definition().unwrap()],
        );
        assert_eq!(
            occurrences
                .iter()
                .filter(|item| item.role() == AuthoringOccurrenceRole::Reference)
                .map(|item| item.location().clone())
                .collect::<Vec<_>>(),
            navigation.references(),
        );
        match &reference {
            None => reference = Some((occurrences, inspected)),
            Some(expected) => assert_eq!(&(occurrences, inspected), expected),
        }
    }
    drop(workspace);
    drop(alias);
    drop(rebuilt);
    assert!(
        !reference.unwrap().0.is_empty(),
        "owned locations survive producer drops"
    );
}

#[test]
fn effective_occurrence_cursors_reject_recompilation_cross_kind_and_raw_limit_changes() {
    let api = PaleApi::new().unwrap();
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .unwrap();
    let profile = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .unwrap();
    let rebuilt = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .unwrap();
    assert_eq!(profile.coordinates(), rebuilt.coordinates());
    let identity = profile
        .navigate_authoring(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            Vec::new(),
        ))
        .identity()
        .unwrap()
        .clone();
    let request = AuthoringOccurrencePageRequest::new(identity.clone(), 1, 0);
    let first = profile.authoring_occurrences(request.clone()).unwrap();
    let cursor = first.continuation().unwrap().clone();
    let alias = profile.clone();
    drop(profile);
    assert!(alias
        .authoring_occurrences(request.clone().after(cursor.clone()))
        .is_ok());
    for rejected in [
        rebuilt.authoring_occurrences(request.clone().after(cursor.clone())),
        workspace.authoring_occurrences(request.clone().after(cursor.clone())),
        alias.authoring_occurrences(
            AuthoringOccurrencePageRequest::new(identity, 1, 64).after(cursor.clone()),
        ),
    ] {
        assert_eq!(
            rejected.unwrap_err().diagnostic().code(),
            "authoring_continuation_mismatch"
        );
    }
    assert!(!format!("{cursor:?} {request:?}").contains("pale-workspace.toml"));
    assert!(rebuilt.authoring_occurrences(request).is_ok());
}

#[test]
fn authoring_identity_retains_scope_without_disclosing_it_or_binding_generation() {
    let (machine_identity, repeated_machine_identity, lock_identity) = {
        let api = PaleApi::new().expect("embedded API dependencies are coherent");
        let workspace = api
            .compile_portable_workspace(captured_workspace(false))
            .expect("workspace fixture is admitted");
        let compile_identity = |machine_label: &str, lock_label: &str| {
            let private = OneShotPrivateInputs::new(
                document(
                    "capabilities.toml",
                    captured_fixture!("private/capabilities.toml"),
                ),
                document(
                    machine_label,
                    captured_fixture!("private/machine-binding.toml"),
                ),
                document(
                    lock_label,
                    captured_fixture!("private/distribution-lock.toml"),
                ),
                Vec::new(),
            );
            let profile = api
                .compile_effective_profile(
                    &workspace,
                    EffectiveProfileCompilationRequest::new(
                        "demo".into(),
                        "default".into(),
                        "machine".into(),
                        private,
                        captured_package_documents(false),
                    ),
                )
                .expect("role-labelled private fixture is admitted");
            let identity = profile
                .navigate_authoring(AuthoringNavigationRequest::new(
                    "identity-source.toml".into(),
                    Vec::new(),
                ))
                .identity()
                .expect("the compiler indexes the private source document")
                .clone();
            (profile, identity)
        };
        let (machine_profile, machine_identity) =
            compile_identity("identity-source.toml", "distribution-lock.toml");
        let (_, repeated_machine_identity) =
            compile_identity("identity-source.toml", "distribution-lock.toml");
        let (_, lock_identity) = compile_identity("machines/machine.toml", "identity-source.toml");
        let page = machine_profile
            .authoring_occurrences(AuthoringOccurrencePageRequest::new(
                machine_identity.clone(),
                1,
                1,
            ))
            .unwrap();
        let error = machine_profile
            .authoring_occurrences(
                AuthoringOccurrencePageRequest::new(lock_identity.clone(), 1, 1)
                    .after(page.continuation().unwrap().clone()),
            )
            .expect_err("equal descriptions cannot substitute a different compiler scope");
        assert_eq!(error.diagnostic().code(), "authoring_continuation_mismatch");
        (machine_identity, repeated_machine_identity, lock_identity)
    };

    // Each identity outlives its result and compilation. The same label denotes
    // different grammar-scoped subjects, despite identical readable fields.
    assert_eq!(machine_identity, repeated_machine_identity);
    assert_ne!(machine_identity, lock_identity);
    for identity in [
        &machine_identity,
        &repeated_machine_identity,
        &lock_identity,
    ] {
        assert_eq!(identity.kind(), AuthoringIdentityKind::SourceDocument);
        assert_eq!(identity.value(), "identity-source.toml");
        assert_eq!(identity.authored_value(), "identity-source.toml");
        assert_eq!(
            serde_json::to_string(identity).expect("identity description serializes"),
            r#"{"kind":"source-document","value":"identity-source.toml","authored_value":"identity-source.toml"}"#,
        );
        assert_eq!(
            format!("{identity:?}"),
            "AuthoringIdentity { kind: SourceDocument, value: \"identity-source.toml\", authored_value: \"identity-source.toml\" }",
        );
    }
}

#[test]
fn effective_authoring_inspection_joins_linker_identity_to_retained_profile_rows() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");
    let profile = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".into(),
                "default".into(),
                "machine".into(),
                captured_private(),
                captured_package_documents(false),
            ),
        )
        .expect("effective profile is admitted");
    let navigation = profile.navigate_authoring(AuthoringNavigationRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            authoring_field("workspace"),
            authoring_field("desired_topology"),
            authoring_field("projects"),
            AuthoringInstancePathSegment::Index(0),
            authoring_field("run_policy"),
        ],
    ));
    let identity = navigation.identity().expect("run policy resolves");
    assert_eq!(identity.kind(), AuthoringIdentityKind::RunPolicy);

    let inspection = profile.inspect_effective_authoring(identity);
    assert_eq!(inspection.coordinates(), &profile.coordinates());
    assert!(!inspection.truncated());
    assert!(inspection.entries().iter().any(|entry| matches!(
        entry.detail(),
        EffectiveProfileEntryDetail::Selection {
            selection: EffectiveProfileSelection::RunPolicy { run_policy_id },
        } if run_policy_id == "standard"
    )));
}

#[test]
fn effective_authoring_pages_preserve_rows_and_reject_unlike_continuations() {
    let api = PaleApi::new().unwrap();
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .unwrap();
    let compile = || {
        api.compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".into(),
                "default".into(),
                "machine".into(),
                captured_private(),
                captured_package_documents(false),
            ),
        )
        .unwrap()
    };
    let profile = compile();
    let rebuilt = compile();
    assert_eq!(profile.coordinates(), rebuilt.coordinates());
    let identity = profile
        .navigate_authoring(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            vec![
                authoring_field("workspace"),
                authoring_field("desired_topology"),
                authoring_field("repositories"),
                AuthoringInstancePathSegment::Index(0),
                authoring_field("id"),
            ],
        ))
        .identity()
        .unwrap()
        .clone();
    let other_identity = profile
        .navigate_authoring(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            vec![
                authoring_field("workspace"),
                authoring_field("desired_topology"),
                authoring_field("projects"),
                AuthoringInstancePathSegment::Index(0),
                authoring_field("run_policy"),
            ],
        ))
        .identity()
        .unwrap()
        .clone();
    let reference = profile.inspect_effective_authoring(&identity);
    assert!(!reference.truncated());
    assert!(reference.entries().len() > 1);
    let page = profile
        .inspect_effective_authoring_page(&identity, None, 1)
        .unwrap();
    let continuation = page.continuation().unwrap().clone();
    drop(page);
    for (owner, subject, limit) in [
        (&rebuilt, &identity, 1),
        (&profile, &other_identity, 1),
        (&profile, &identity, 2),
    ] {
        let error = owner
            .inspect_effective_authoring_page(subject, Some(&continuation), limit)
            .expect_err("a continuation belongs to its original owner, identity and request");
        assert_eq!(error.diagnostic().code(), "authoring_continuation_mismatch");
        assert!(error.diagnostic().source().is_none());
    }
    let alias = profile.clone();
    drop(profile);
    assert!(!alias
        .inspect_effective_authoring_page(&identity, Some(&continuation), 1)
        .unwrap()
        .entries()
        .is_empty());
    for limit in [1, 0, u16::MAX] {
        let mut continuation = None;
        let mut entries = Vec::new();
        loop {
            let page = alias
                .inspect_effective_authoring_page(&identity, continuation.as_ref(), limit)
                .unwrap();
            assert_eq!(page.coordinates(), reference.coordinates());
            assert!(
                page.entries().len()
                    <= if limit == 0 {
                        64
                    } else {
                        usize::from(limit.min(256))
                    }
            );
            entries.extend_from_slice(page.entries());
            continuation = page.continuation().cloned();
            if continuation.is_none() {
                break;
            }
            assert!(entries.len() < reference.entries().len());
        }
        assert_eq!(entries, reference.entries());
        for entry in entries {
            assert!(alias.explain(entry.explanation_handle()).is_some());
        }
    }
    let owned_page = alias
        .inspect_effective_authoring_page(&identity, None, 0)
        .unwrap();
    drop(alias);
    drop(rebuilt);
    drop(workspace);
    assert_eq!(owned_page.entries(), reference.entries());
}

#[test]
fn structural_identity_completion_survives_a_new_unadmitted_array_member() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");
    let navigation = workspace.navigate_authoring(AuthoringNavigationRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            authoring_field("workspace"),
            authoring_field("desired_topology"),
            authoring_field("project_repositories"),
            AuthoringInstancePathSegment::Index(1),
            authoring_field("project_id"),
        ],
    ));

    assert!(navigation.identity().is_none());
    assert!(navigation.definition().is_none());
    assert!(navigation.references().is_empty());
    assert_eq!(
        navigation
            .alternatives()
            .iter()
            .map(|identity| identity.authored_value())
            .collect::<Vec<_>>(),
        vec!["demo"]
    );
    assert!(!navigation.alternatives_truncated());
}

#[test]
fn declared_wildcard_remains_distinct_from_effective_resource_closure() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");
    let profile = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".into(),
                "default".into(),
                "machine".into(),
                captured_private_with_declared_role_wildcard(),
                captured_package_documents(false),
            ),
        )
        .expect("declared wildcard fixture is admitted");

    let intended = profile.inspect(EffectiveProfilePageRequest::new(
        EffectiveProfileSection::IntendedUseCase,
        0,
        256,
    ));
    assert!(intended.entries().iter().any(|entry| matches!(
        entry.detail(),
        EffectiveProfileEntryDetail::DeclaredSelection {
            selection: EffectiveProfileDeclaredSelection::Resource {
                resource_kind: EffectiveProfileResourceKind::Role,
                selector: EffectiveProfileDeclaredSelector::AllFromPackage { package_id },
            },
        } if package_id == "example.engine"
    )));

    let effective = profile.inspect(EffectiveProfilePageRequest::new(
        EffectiveProfileSection::EffectiveConfiguration,
        0,
        256,
    ));
    assert!(effective.entries().iter().any(|entry| matches!(
        entry.detail(),
        EffectiveProfileEntryDetail::Selection {
            selection: EffectiveProfileSelection::SemanticResource {
                resource_kind: EffectiveProfileResourceKind::Role,
                resource_id,
                ..
            },
        } if resource_id == "example.engine:engine"
    )));
}

#[test]
fn authored_binding_mismatch_is_invalid_input_in_both_compilation_journeys() {
    use std::error::Error as _;

    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let mut composition: toml::Value = toml::from_str(
        std::str::from_utf8(captured_fixture!("workspace/composition.toml")).unwrap(),
    )
    .unwrap();
    composition["declarations"]["provider_bindings"][0]["binding_identity"]
        ["provider_binding_sha256"] = toml::Value::String("0".repeat(64));
    let composition = toml::to_string(&composition).unwrap().into_bytes();
    let workspace_request = || {
        PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_owned(),
            vec![
                PortableWorkspaceDocument::new("composition.toml".to_owned(), composition.clone()),
                PortableWorkspaceDocument::new(
                    "pale-workspace.toml".to_owned(),
                    captured_fixture!("workspace/pale-workspace.toml").to_vec(),
                ),
                PortableWorkspaceDocument::new(
                    "workspace-resolution.lock.toml".to_owned(),
                    captured_fixture!("workspace/workspace-resolution.lock.toml").to_vec(),
                ),
                PortableWorkspaceDocument::new(
                    "workspace-universe.lock.toml".to_owned(),
                    captured_fixture!("workspace/workspace-universe.lock.toml").to_vec(),
                ),
            ],
        )
    };
    let one_shot = api
        .compile_one_shot(captured_request_with_workspace(false, workspace_request()))
        .expect_err("a changed declaration digest is not a compiler invariant failure");
    let workspace = api.compile_portable_workspace(workspace_request()).unwrap();
    let profile = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .expect_err("profile inspection applies the same composition admission law");

    for (error, code, stage) in [
        (
            one_shot,
            "one_shot_semantics_rejected",
            DiagnosticStage::SemanticAdmission,
        ),
        (
            profile,
            "effective_profile_compilation_rejected",
            DiagnosticStage::Compilation,
        ),
    ] {
        let diagnostic = error.diagnostic();
        assert_eq!(diagnostic.code(), code);
        assert_eq!(diagnostic.stage(), stage);
        assert_eq!(
            diagnostic.disposition(),
            DiagnosticDisposition::InvalidInput
        );
        assert_eq!(diagnostic.retry(), DiagnosticRetry::AfterCorrection);
        assert_eq!(
            diagnostic.effect(),
            DiagnosticEffectCertainty::NotApplicable
        );
        assert_eq!(
            diagnostic.subject(),
            Some(("provider_binding", "provider-binding-claude-code"))
        );
        assert!(diagnostic.facts().iter().any(|fact| {
            fact.key() == "violation"
                && fact.value()
                    == "claimed binding identity does not match the complete canonical binding body"
        }));
        assert!(
            error.source().is_none(),
            "private compiler causes are not exposed"
        );
        assert!(!diagnostic.canonical_json().contains("owner://"));
    }
}

#[test]
fn effective_profile_failure_uses_its_own_bounded_request_context() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .expect("workspace fixture is admitted");
    let error = api
        .compile_effective_profile(
            &workspace,
            EffectiveProfileCompilationRequest::new(
                "demo".into(),
                "default".into(),
                "machine".into(),
                placeholder_private(),
                captured_package_documents(false),
            ),
        )
        .expect_err("placeholder private bodies are not valid compiler input");
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code(), "effective_profile_compilation_rejected");
    assert_eq!(diagnostic.stage(), DiagnosticStage::Compilation);
    assert_eq!(
        diagnostic.disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert_eq!(
        diagnostic.effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
    assert!(!diagnostic.notes().is_empty());
    assert_eq!(
        diagnostic
            .facts()
            .into_iter()
            .map(|fact| (fact.key().to_owned(), fact.value().to_owned()))
            .collect::<Vec<_>>(),
        [
            ("project_id".into(), "demo".into()),
            ("execution_workspace_id".into(), "default".into()),
            ("machine_id".into(), "machine".into()),
        ]
    );
}

#[test]
fn source_guidance_is_owned_and_refusal_retains_structured_no_effect_diagnostics() {
    use pale_sdk::SourceGuidanceHost;
    use sha2::{Digest, Sha256};
    use std::error::Error as _;

    let api = PaleApi::new().unwrap();
    let workspace = api
        .compile_portable_workspace(captured_workspace(false))
        .unwrap();
    let profile = api
        .compile_effective_profile(&workspace, captured_effective_profile_request(false))
        .unwrap();
    let codex = profile
        .source_guidance("engine", SourceGuidanceHost::Codex)
        .unwrap();
    let claude = profile
        .source_guidance("engine", SourceGuidanceHost::ClaudeCode)
        .unwrap();
    assert_eq!(codex.repository_id(), "engine");
    assert_eq!(codex.profile_digest(), profile.profile_digest());
    assert_eq!(codex.entrypoint_path(), "AGENTS.md");
    assert_eq!(claude.entrypoint_path(), "AGENTS.md");
    assert_ne!(codex.digest(), claude.digest());
    for repository in ["../engine", "unselected"] {
        let error = profile
            .source_guidance(repository, SourceGuidanceHost::Codex)
            .unwrap_err();
        let diagnostic = error.diagnostic();
        assert_eq!(diagnostic.code(), "source_guidance_projection_rejected");
        assert_eq!(
            diagnostic.disposition(),
            DiagnosticDisposition::InvalidInput
        );
        assert_eq!(
            diagnostic.effect(),
            DiagnosticEffectCertainty::NotApplicable
        );
        assert!(
            error.source().is_none(),
            "opaque retained sources are not public diagnostic chains"
        );
    }
    drop(profile);
    drop(workspace);
    drop(api);
    for projection in [codex, claude] {
        assert!(projection
            .entrypoint_contribution()
            .contains(".pale/guidance/"));
        assert!(projection
            .files()
            .iter()
            .any(|file| file.content().contains("Exact retained role body.")));
        for file in projection.files() {
            assert_eq!(
                file.sha256(),
                format!("{:x}", Sha256::digest(file.content().as_bytes()))
            );
        }
    }
}

#[test]
fn fixed_private_roles_reject_swapped_byte_bodies() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let request = OneShotCompilationRequest::new(
        "demo".to_string(),
        "default".to_string(),
        "machine".to_string(),
        captured_private_with_capability_and_machine_bodies_swapped(),
        vec![
            document(
                "engine/package.toml",
                captured_fixture!("packages/engine/package.toml"),
            ),
            document(
                "engine/roles.toml",
                captured_fixture!("packages/engine/roles.toml"),
            ),
            document(
                "engine/role.md",
                captured_fixture!("packages/engine/role.md"),
            ),
        ],
        captured_workspace(false),
    );
    let error = api
        .compile_one_shot(request)
        .expect_err("fixed private roles are bound by their constructor arguments");

    assert_eq!(error.diagnostic().code(), "one_shot_private_input_rejected");
    assert_eq!(
        error.diagnostic().stage(),
        DiagnosticStage::WorkspaceLoading
    );
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
    let diagnostic = String::from_utf8_lossy(error.diagnostic().canonical_bytes());
    let debug = format!("{error:?}");
    for private_body in [
        "Synthetic Engine source",
        "Portable compilation without an action gate",
    ] {
        assert!(!diagnostic.contains(private_body));
        assert!(!debug.contains(private_body));
    }
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn planning_reuses_exact_target_and_transition_laws_for_apply_adopt_and_remove() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = compile_fixture(&api, false);
    let unowned = OneShotPriorMaterialization::Unowned;
    let apply_targets = api
        .one_shot_observation_targets(
            &compilation,
            "engine".to_string(),
            OneShotPlanKind::Apply,
            &unowned,
        )
        .expect("the application owner derives initial apply targets");
    assert!(!apply_targets.is_empty());
    assert!(apply_targets
        .windows(2)
        .all(|window| window[0].relative_path() < window[1].relative_path()));
    let target_projection: serde_json::Value =
        serde_json::from_slice(apply_targets.canonical_bytes()).unwrap();
    assert_eq!(
        target_projection["targets"],
        serde_json::Value::Array(
            apply_targets
                .iter()
                .map(|target| serde_json::Value::String(target.relative_path().to_string()))
                .collect(),
        )
    );
    assert_eq!(
        apply_targets.canonical_bytes(),
        pale_core::canon::to_canonical_string(&target_projection)
            .unwrap()
            .as_bytes()
    );

    let apply = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Apply,
                OneShotPriorMaterialization::Unowned,
                absent_baseline(&apply_targets),
            ),
        )
        .expect("absent exact baseline produces an initial apply candidate");
    let repeated_apply = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Apply,
                OneShotPriorMaterialization::Unowned,
                absent_baseline(&apply_targets),
            ),
        )
        .expect("the same admitted claims reproduce the same candidate");
    assert_eq!(apply.kind(), OneShotPlanKind::Apply);
    assert_eq!(apply.repository_id(), "engine");
    assert_eq!(
        apply.one_shot_compilation_digest(),
        compilation.summary().one_shot_compilation_digest()
    );
    assert_eq!(
        apply.operation_count(),
        u32::try_from(apply_targets.len()).expect("fixture target count fits the public width")
    );
    assert_eq!(apply.create_count(), apply.operation_count());
    assert_eq!(apply.adopt_count(), 0);
    assert_eq!(apply.update_count(), 0);
    assert_eq!(apply.unchanged_count(), 0);
    assert_eq!(apply.delete_count(), 0);
    assert_eq!(apply.splice_count(), 0);
    assert_eq!(apply.plan_digest(), repeated_apply.plan_digest());
    assert_eq!(apply.canonical_bytes(), repeated_apply.canonical_bytes());
    let bound = api
        .bind_one_shot_machine_effects(repeated_apply, Vec::new())
        .expect("an empty host-effect set binds through the application owner");
    assert_eq!(bound.kind(), OneShotPlanKind::Apply);
    assert_eq!(bound.repository_id(), "engine");
    let saved = bound.canonical_bytes().to_vec();
    let admitted = api
        .admit_one_shot_saved_plan(bound, &saved)
        .expect("exact saved bytes mint one opaque effect capability");
    assert_eq!(admitted.kind(), OneShotPlanKind::Apply);
    assert_eq!(admitted.repository_id(), "engine");
    assert!(!admitted.plan_digest().is_empty());
    let apply_record = decode_candidate(&apply);
    let prior_canonical = prior_bytes(&apply_record);
    let prior_state = apply_record
        .transition
        .next_state()
        .expect("initial apply publishes ownership state")
        .clone();

    let adopt_targets = api
        .one_shot_observation_targets(
            &compilation,
            "engine".to_string(),
            OneShotPlanKind::Adopt,
            &OneShotPriorMaterialization::Unowned,
        )
        .expect("adoption shares the apply target union");
    assert_eq!(apply_targets, adopt_targets);
    let adopt = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Adopt,
                OneShotPriorMaterialization::Unowned,
                present_baseline_from_claimed_files(&adopt_targets, &prior_state),
            ),
        )
        .expect("present-file claims produce explicit adoption");
    assert_eq!(adopt.kind(), OneShotPlanKind::Adopt);
    assert_eq!(adopt.adopt_count(), adopt.operation_count());
    assert_eq!(adopt.create_count(), 0);
    assert_eq!(adopt.operation_count(), apply.operation_count());
    decode_candidate(&adopt);

    let owned = OneShotPriorMaterialization::Owned {
        canonical_state: prior_canonical.clone(),
    };
    let owned_apply_targets = api
        .one_shot_observation_targets(
            &compilation,
            "engine".to_string(),
            OneShotPlanKind::Apply,
            &owned,
        )
        .expect("apply target union retains prior and desired targets");
    let remove_targets = api
        .one_shot_observation_targets(
            &compilation,
            "engine".to_string(),
            OneShotPlanKind::Remove,
            &owned,
        )
        .expect("remove targets come only from prior ownership");
    assert_eq!(owned_apply_targets, apply_targets);
    assert_eq!(remove_targets, apply_targets);

    let remove = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Remove,
                OneShotPriorMaterialization::Owned {
                    canonical_state: prior_canonical,
                },
                present_baseline_from_claimed_files(&remove_targets, &prior_state),
            ),
        )
        .expect("prior ownership produces a bounded removal candidate");
    assert_eq!(remove.kind(), OneShotPlanKind::Remove);
    assert_eq!(
        remove.delete_count() + remove.splice_count(),
        remove.operation_count()
    );
    assert_eq!(remove.operation_count(), apply.operation_count());
    assert!(matches!(
        decode_candidate(&remove).transition,
        MaterializationStateTransitionV1::Remove { .. }
    ));

    let remove_unowned_targets = api
        .one_shot_observation_targets(
            &compilation,
            "engine".to_string(),
            OneShotPlanKind::Remove,
            &OneShotPriorMaterialization::Unowned,
        )
        .expect("unowned removal observes no targets");
    assert!(remove_unowned_targets.is_empty());
    let remove_unowned = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Remove,
                OneShotPriorMaterialization::Unowned,
                Vec::new(),
            ),
        )
        .expect("unowned removal is an empty candidate, not an inferred effect");
    assert_eq!(remove_unowned.operation_count(), 0);
    assert!(matches!(
        decode_candidate(&remove_unowned).transition,
        MaterializationStateTransitionV1::RemoveUnowned {}
    ));
}

#[test]
fn planning_rejects_malformed_repository_selectors_at_the_api_boundary() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = compile_fixture(&api, false);
    let prior = OneShotPriorMaterialization::Unowned;

    let observation_error = api
        .one_shot_observation_targets(
            &compilation,
            "Engine".to_string(),
            OneShotPlanKind::Apply,
            &prior,
        )
        .expect_err("a malformed repository selector must fail before target derivation");
    let planning_error = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "Engine".to_string(),
                OneShotPlanKind::Apply,
                prior,
                Vec::new(),
            ),
        )
        .expect_err("a malformed repository selector must fail before plan construction");

    for error in [&observation_error, &planning_error] {
        assert_eq!(
            error.diagnostic().code(),
            "one_shot_planning_input_rejected"
        );
        assert_eq!(
            error.diagnostic().disposition(),
            DiagnosticDisposition::InvalidInput
        );
        assert_eq!(error.diagnostic().retry(), DiagnosticRetry::AfterCorrection);
        assert_eq!(
            error.diagnostic().effect(),
            DiagnosticEffectCertainty::NotApplicable
        );
    }
}

#[test]
fn read_repository_access_preserves_observation_but_blocks_plan_projection() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_one_shot(captured_request_with_workspace(
            false,
            captured_workspace_with_access(false, "read"),
        ))
        .expect("read-only repository membership compiles");
    let targets = api
        .one_shot_observation_targets(
            &compilation,
            "engine".to_string(),
            OneShotPlanKind::Apply,
            &OneShotPriorMaterialization::Unowned,
        )
        .expect("read-only repository membership remains observable");

    let error = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Apply,
                OneShotPriorMaterialization::Unowned,
                absent_baseline(&targets),
            ),
        )
        .expect_err("read-only membership cannot project a mutation plan");

    assert_eq!(
        error.diagnostic().code(),
        "repository_mutation_not_permitted"
    );
    assert_eq!(error.diagnostic().stage(), DiagnosticStage::Planning);
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::Blocked
    );
    assert_eq!(
        error.diagnostic().retry(),
        DiagnosticRetry::AfterStateChange
    );
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn planning_rejects_duplicate_incomplete_noncanonical_and_digest_wrong_claims() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = compile_fixture(&api, false);
    let targets = api
        .one_shot_observation_targets(
            &compilation,
            "engine".to_string(),
            OneShotPlanKind::Apply,
            &OneShotPriorMaterialization::Unowned,
        )
        .expect("the application owner derives initial apply targets");
    assert!(!targets.is_empty());

    let mut duplicate = absent_baseline(&targets);
    duplicate.push(OneShotBaselineEntry::new(
        targets[0].relative_path().to_string(),
        OneShotBaselineObservation::Absent,
    ));
    let error = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Apply,
                OneShotPriorMaterialization::Unowned,
                duplicate,
            ),
        )
        .expect_err("a duplicate observation target must be rejected");
    assert_invalid_planning_claim(&error);

    let mut incomplete = absent_baseline(&targets);
    incomplete.pop();
    let error = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Apply,
                OneShotPriorMaterialization::Unowned,
                incomplete,
            ),
        )
        .expect_err("an incomplete observation target set must be rejected");
    assert_invalid_planning_claim(&error);

    let apply = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Apply,
                OneShotPriorMaterialization::Unowned,
                absent_baseline(&targets),
            ),
        )
        .expect("exact absent claims produce a state-bearing plan");
    let canonical = prior_bytes(&decode_candidate(&apply));
    let mut noncanonical = vec![b' '];
    noncanonical.extend_from_slice(&canonical);
    let error = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Remove,
                OneShotPriorMaterialization::Owned {
                    canonical_state: noncanonical,
                },
                Vec::new(),
            ),
        )
        .expect_err("non-canonical prior bytes must be rejected before target planning");
    assert_invalid_planning_claim(&error);
    assert_eq!(
        error.diagnostic().code(),
        "one_shot_planning_input_rejected"
    );

    let mut digest_wrong: serde_json::Value =
        serde_json::from_slice(&canonical).expect("canonical state is JSON");
    digest_wrong["state_digest"] = serde_json::Value::String("0".repeat(64));
    let digest_wrong = pale_core::canon::to_canonical_string(&digest_wrong)
        .expect("the digest-wrong value remains canonically encodable")
        .into_bytes();
    let error = api
        .plan_one_shot(
            &compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Remove,
                OneShotPriorMaterialization::Owned {
                    canonical_state: digest_wrong,
                },
                Vec::new(),
            ),
        )
        .expect_err("a self-digest mismatch must be rejected before target planning");
    assert_invalid_planning_claim(&error);
    assert_eq!(
        error.diagnostic().code(),
        "one_shot_planning_input_rejected"
    );
}

#[test]
fn captured_package_rejection_is_typed_redacted_and_effect_free() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let secret = "private-source-body";
    let request = OneShotCompilationRequest::new(
        "demo".to_string(),
        "default".to_string(),
        "machine".to_string(),
        placeholder_private(),
        vec![document("../escaped.toml", secret.as_bytes())],
        captured_workspace(false),
    );
    let error = api
        .compile_one_shot(request)
        .expect_err("an escaped Package label must fail before Package parsing");

    assert_eq!(error.diagnostic().code(), "one_shot_source_rejected");
    assert_eq!(error.diagnostic().stage(), DiagnosticStage::InputAdmission);
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
    assert_eq!(error.diagnostic().retry(), DiagnosticRetry::AfterCorrection);
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
    assert!(!String::from_utf8_lossy(error.diagnostic().canonical_bytes()).contains(secret));
    assert!(!format!("{error:?}").contains(secret));
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn raw_request_and_prior_state_debug_never_render_labels_or_bytes() {
    let secret = "do-not-render-this-value";
    let raw = document(secret, secret.as_bytes());
    let raw_debug = format!("{raw:?}");
    assert!(!raw_debug.contains(secret));
    assert!(raw_debug.contains("label_byte_len"));
    assert!(raw_debug.contains("byte_len"));

    let private = OneShotPrivateInputs::new(
        document(secret, secret.as_bytes()),
        document(secret, secret.as_bytes()),
        document(secret, secret.as_bytes()),
        vec![document(secret, secret.as_bytes())],
    );
    let private_debug = format!("{private:?}");
    assert!(!private_debug.contains(secret));
    assert!(private_debug.contains("lifecycle_guidance_count"));

    let prior = OneShotPriorMaterialization::Owned {
        canonical_state: secret.as_bytes().to_vec(),
    };
    let prior_debug = format!("{prior:?}");
    assert!(!prior_debug.contains(secret));
    assert!(prior_debug.contains("canonical_state_bytes"));
}
