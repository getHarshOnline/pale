//! Behavioral parity for API capabilities projected through the command façade.

use pale_sdk::{AuthoringIdentityKind, AuthoringInspectionState, AuthoringInstancePathSegment};

use super::*;

use crate::command::{
    AuthoringIngressClass, AuthoringInspectionRequest, AuthoringNavigationRequest,
    AuthoringPathSegment, EffectiveProfileAuthoringInspectionRequest,
    EffectiveProfileAuthoringJoinRequest, EffectiveProfileAuthoringNavigationRequest,
    EffectiveProfileExplanationRequest, EffectiveProfileObservationRequest,
    ObservationCancellation, ObservationCategory, ObservationCausalSelector,
    ObservationDeltaOutcome, ObservationDeltaRequest, ObservationFact, ObservationFilter,
    ObservationLensDisposition, ObservationLensLimits, ObservationLensRequest,
    ObservationPageDisposition, ObservationPageLimits, ObservationPageRequest,
    ObservationSubjectKind, PortableWorkspaceAuthoringInspectionRequest,
    PortableWorkspaceAuthoringNavigationRequest, StandaloneAuthoringDocument,
    StandaloneAuthoringInspectionRequest, StandaloneAuthoringRole,
};
#[cfg(feature = "native-local")]
use crate::command::{
    LocalMaterializationCommand, LocalMaterializationOutcome,
    LocalMaterializationSettlementDisposition, LocalSavedPlanPublicationDisposition,
    LocalSavedPlanPublicationRequest,
};
#[cfg(feature = "native-local")]
use crate::test_fixtures::local_executable_binding_request;
use crate::test_fixtures::{
    absent_baseline, effective_profile_request, empty_workspace_request, one_shot_request,
};

fn field(value: &str) -> AuthoringInstancePathSegment {
    AuthoringInstancePathSegment::Field(value.to_owned())
}

fn structural_field(value: &str) -> AuthoringPathSegment {
    AuthoringPathSegment::Field(value.to_owned())
}

#[test]
fn diagnostic_and_standalone_authoring_journeys_preserve_api_results() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();
    let diagnostic = api
        .contract_schema("not-registered.v1")
        .unwrap_err()
        .diagnostic()
        .canonical_bytes()
        .to_vec();
    let outcome = dispatcher
        .dispatch(CommandRequest::DiagnosticAdmission(diagnostic.clone()))
        .unwrap();
    let CommandOutcome::DiagnosticAdmission(admitted) = outcome else {
        panic!("diagnostic admission returned the wrong closed outcome");
    };
    assert_eq!(admitted.canonical_bytes(), diagnostic);

    let direct_limits = PaleApi::authoring_ingress_limits(AuthoringIngressClass::Workspace);
    let outcome = dispatcher
        .dispatch(CommandRequest::AuthoringIngressLimits(
            AuthoringIngressClass::Workspace,
        ))
        .unwrap();
    let CommandOutcome::AuthoringIngressLimits(dispatched_limits) = outcome else {
        panic!("authoring limits returned the wrong closed outcome");
    };
    assert_eq!(
        (
            dispatched_limits.max_documents(),
            dispatched_limits.max_document_bytes(),
            dispatched_limits.max_total_bytes(),
        ),
        (
            direct_limits.max_documents(),
            direct_limits.max_document_bytes(),
            direct_limits.max_total_bytes(),
        )
    );

    let source = include_bytes!("../../fixtures/cross-leaf.v1/portable/pale-workspace.toml");
    let document = || {
        StandaloneAuthoringDocument::new(
            StandaloneAuthoringRole::WorkspaceManifest,
            "pale-workspace.toml".to_owned(),
            source.to_vec(),
        )
    };
    api.validate_authoring_document(
        StandaloneAuthoringRole::WorkspaceManifest,
        "pale-workspace.toml".to_owned(),
        source.to_vec(),
    )
    .unwrap();
    assert!(matches!(
        dispatcher
            .dispatch(CommandRequest::StandaloneAuthoringValidation(document()))
            .unwrap(),
        CommandOutcome::StandaloneAuthoringValidation
    ));

    let direct = api
        .inspect_standalone_authoring(
            StandaloneAuthoringRole::WorkspaceManifest,
            vec![structural_field("workspace")],
        )
        .unwrap();
    let outcome = dispatcher
        .dispatch(CommandRequest::StandaloneAuthoringInspection(
            StandaloneAuthoringInspectionRequest::new(
                StandaloneAuthoringRole::WorkspaceManifest,
                vec![structural_field("workspace")],
            ),
        ))
        .unwrap();
    let CommandOutcome::StandaloneAuthoringInspection(dispatched) = outcome else {
        panic!("standalone authoring inspection returned the wrong closed outcome");
    };
    assert_eq!(dispatched.state(), direct.state());
    assert_eq!(dispatched.schema_id(), direct.schema_id());
    assert_eq!(dispatched.definition(), direct.definition());

    let oversized_path = || {
        (0..65)
            .map(|index| structural_field(&format!("over-bound-{index}")))
            .collect::<Vec<_>>()
    };
    let direct = api
        .inspect_standalone_authoring(StandaloneAuthoringRole::WorkspaceManifest, oversized_path())
        .unwrap_err();
    let dispatched = dispatcher
        .dispatch(CommandRequest::StandaloneAuthoringInspection(
            StandaloneAuthoringInspectionRequest::new(
                StandaloneAuthoringRole::WorkspaceManifest,
                oversized_path(),
            ),
        ))
        .unwrap_err();
    assert_eq!(
        dispatched.diagnostic().canonical_bytes(),
        direct.diagnostic().canonical_bytes()
    );
}

#[test]
fn portable_authoring_journeys_use_the_admitted_workspace_owner() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();
    let direct_workspace = api
        .compile_portable_workspace(empty_workspace_request(None))
        .unwrap();
    let direct_inspection = direct_workspace
        .inspect_authoring(AuthoringInspectionRequest::new(
            "pale-workspace.toml".to_owned(),
            vec![
                structural_field("workspace"),
                structural_field("desired_topology"),
            ],
        ))
        .unwrap();
    let outcome = dispatcher
        .dispatch(CommandRequest::PortableWorkspaceAuthoringInspection(
            PortableWorkspaceAuthoringInspectionRequest::new(
                empty_workspace_request(None),
                AuthoringInspectionRequest::new(
                    "pale-workspace.toml".to_owned(),
                    vec![
                        structural_field("workspace"),
                        structural_field("desired_topology"),
                    ],
                ),
            ),
        ))
        .unwrap();
    let CommandOutcome::PortableWorkspaceAuthoringInspection(dispatched) = outcome else {
        panic!("portable authoring inspection returned the wrong closed outcome");
    };
    assert_eq!(dispatched.state(), AuthoringInspectionState::Available);
    assert_eq!(dispatched.state(), direct_inspection.state());
    assert_eq!(dispatched.schema_id(), direct_inspection.schema_id());

    let navigation = || {
        AuthoringNavigationRequest::new(
            "pale-workspace.toml".to_owned(),
            vec![field("workspace"), field("workspace_id")],
        )
    };
    let direct_navigation = direct_workspace.navigate_authoring(navigation());
    let outcome = dispatcher
        .dispatch(CommandRequest::PortableWorkspaceAuthoringNavigation(
            PortableWorkspaceAuthoringNavigationRequest::new(
                empty_workspace_request(None),
                navigation(),
            ),
        ))
        .unwrap();
    let CommandOutcome::PortableWorkspaceAuthoringNavigation(dispatched_navigation) = outcome
    else {
        panic!("portable authoring navigation returned the wrong closed outcome");
    };
    assert_eq!(dispatched_navigation, direct_navigation);
    assert_eq!(
        dispatched_navigation
            .identity()
            .map(|identity| (identity.kind(), identity.value())),
        Some((AuthoringIdentityKind::Workspace, "empty-adopter"))
    );
}

#[test]
fn effective_profile_proof_and_authoring_journeys_share_one_compiler_path() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();
    let (workspace, profile) = effective_profile_request();
    let direct_profile = api
        .compile_portable_workspace(workspace)
        .and_then(|workspace| api.compile_effective_profile(&workspace, profile))
        .unwrap();
    let page = direct_profile.inspect(EffectiveProfilePageRequest::new(
        EffectiveProfileSection::EffectiveConfiguration,
        0,
        1,
    ));
    let handle = page
        .entries()
        .first()
        .expect("effective profile contains one configuration entry")
        .explanation_handle()
        .clone();
    let direct_explanation = direct_profile.explain(&handle);
    let (workspace, profile) = effective_profile_request();
    let outcome = dispatcher
        .dispatch(CommandRequest::EffectiveProfileExplanation(
            EffectiveProfileExplanationRequest::new(workspace, profile, handle),
        ))
        .unwrap();
    let CommandOutcome::EffectiveProfileExplanation(dispatched_explanation) = outcome else {
        panic!("effective profile explanation returned the wrong closed outcome");
    };
    assert_eq!(dispatched_explanation, direct_explanation);

    let inspection =
        || AuthoringInspectionRequest::new("machines/machine.toml".to_owned(), Vec::new());
    let direct_inspection = direct_profile.inspect_authoring(inspection()).unwrap();
    let (workspace, profile) = effective_profile_request();
    let outcome = dispatcher
        .dispatch(CommandRequest::EffectiveProfileAuthoringInspection(
            EffectiveProfileAuthoringInspectionRequest::new(workspace, profile, inspection()),
        ))
        .unwrap();
    let CommandOutcome::EffectiveProfileAuthoringInspection(dispatched_inspection) = outcome else {
        panic!("effective authoring inspection returned the wrong closed outcome");
    };
    assert_eq!(dispatched_inspection.state(), direct_inspection.state());
    assert_eq!(
        dispatched_inspection.schema_id(),
        direct_inspection.schema_id()
    );

    let navigation = || {
        AuthoringNavigationRequest::new(
            "pale-workspace.toml".to_owned(),
            vec![
                field("workspace"),
                field("desired_topology"),
                field("projects"),
                AuthoringInstancePathSegment::Index(0),
                field("run_policy"),
            ],
        )
    };
    let direct_navigation = direct_profile.navigate_authoring(navigation());
    let identity = direct_navigation
        .identity()
        .expect("run policy identity resolves")
        .clone();
    let (workspace, profile) = effective_profile_request();
    let outcome = dispatcher
        .dispatch(CommandRequest::EffectiveProfileAuthoringNavigation(
            EffectiveProfileAuthoringNavigationRequest::new(workspace, profile, navigation()),
        ))
        .unwrap();
    let CommandOutcome::EffectiveProfileAuthoringNavigation(dispatched_navigation) = outcome else {
        panic!("effective authoring navigation returned the wrong closed outcome");
    };
    assert_eq!(dispatched_navigation, direct_navigation);

    let direct_join = direct_profile.inspect_effective_authoring(&identity);
    let (workspace, profile) = effective_profile_request();
    let outcome = dispatcher
        .dispatch(CommandRequest::EffectiveProfileAuthoringJoin(
            EffectiveProfileAuthoringJoinRequest::new(workspace, profile, identity),
        ))
        .unwrap();
    let CommandOutcome::EffectiveProfileAuthoringJoin(dispatched_join) = outcome else {
        panic!("effective authoring join returned the wrong closed outcome");
    };
    assert_eq!(dispatched_join, direct_join);
}

#[test]
fn observation_journeys_reuse_one_opaque_api_snapshot() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let (workspace, profile) = effective_profile_request();
    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationSnapshot(
            EffectiveProfileObservationRequest::new(workspace, profile),
        ))
        .unwrap();
    let CommandOutcome::ObservationSnapshot(snapshot) = outcome else {
        panic!("observation snapshot returned the wrong closed outcome");
    };
    let summary = snapshot.summary();
    assert!(summary.fact_count > 0);
    assert_eq!(summary.omitted_fact_count, 0);

    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationPage {
            snapshot: snapshot.clone(),
            request: ObservationPageRequest::new(ObservationFilter::all(), 1),
        })
        .unwrap();
    let CommandOutcome::ObservationPage(page) = outcome else {
        panic!("observation page returned the wrong closed outcome");
    };
    let ObservationPageDisposition::Ready { page: first_page } =
        page.expect("snapshot owns its first-page cursor")
    else {
        panic!("an active observation request cannot be cancelled");
    };
    assert_eq!(first_page.facts.len(), 1);
    assert!(first_page.continuation.is_some());

    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationDelta {
            current: snapshot.clone(),
            prior: snapshot.clone(),
            request: ObservationDeltaRequest::new(
                snapshot.resume_coordinate(),
                1,
                ObservationCancellation::Active,
            ),
        })
        .unwrap();
    assert!(matches!(
        outcome,
        CommandOutcome::ObservationDelta(ObservationDeltaOutcome::CaughtUp { .. })
    ));

    let ObservationPageDisposition::Ready { page: all } = snapshot
        .page(ObservationPageRequest::new(ObservationFilter::all(), 0))
        .unwrap()
    else {
        panic!("an active observation request cannot be cancelled");
    };
    let path_subject = all
        .facts
        .into_iter()
        .find_map(|fact| match fact {
            ObservationFact::Readiness { subject, .. }
                if subject.kind() == ObservationSubjectKind::ExecutionPath =>
            {
                Some(subject)
            }
            _ => None,
        })
        .expect("effective profile exposes one execution-path subject");
    let selector = ObservationCausalSelector::new(&snapshot, &path_subject);
    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationCausalPath {
            snapshot: snapshot.clone(),
            selector,
        })
        .unwrap();
    let CommandOutcome::ObservationCausalPath(path) = outcome else {
        panic!("observation causal path returned the wrong closed outcome");
    };
    assert!(path.unwrap().is_some());

    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationVisualLens {
            snapshot,
            request: ObservationLensRequest::new(
                ObservationFilter::all(),
                1,
                true,
                ObservationCancellation::Active,
            ),
        })
        .unwrap();
    assert!(matches!(
        outcome,
        CommandOutcome::ObservationVisualLens(ObservationLensDisposition::Ready { .. })
    ));
}

#[test]
fn observation_journey_matches_the_cross_leaf_vector() {
    let expected: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../fixtures/cross-leaf.v1/observation-journey.json"
    ))
    .expect("observation vector is JSON");
    let dispatcher = CommandDispatcher::new().unwrap();
    let (workspace, profile) = effective_profile_request();
    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationSnapshot(
            EffectiveProfileObservationRequest::new(workspace, profile),
        ))
        .unwrap();
    let CommandOutcome::ObservationSnapshot(snapshot) = outcome else {
        panic!("observation snapshot returned the wrong closed outcome");
    };

    let page_filter = ObservationFilter::all().with_category(ObservationCategory::Readiness);
    let page_limits = ObservationPageLimits::new(16, 1, 4_096);
    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationPage {
            snapshot: snapshot.clone(),
            request: ObservationPageRequest::bounded(
                page_filter.clone(),
                page_limits,
                ObservationCancellation::Active,
            ),
        })
        .unwrap();
    let CommandOutcome::ObservationPage(page) = outcome else {
        panic!("observation page returned the wrong closed outcome");
    };
    let page = page.expect("first page belongs to this snapshot");
    let ObservationPageDisposition::Ready { page: ready_page } = &page else {
        panic!("an active page request cannot be cancelled");
    };
    let continuation = ready_page
        .continuation
        .clone()
        .expect("the bounded page retains a continuation");
    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationPage {
            snapshot: snapshot.clone(),
            request: ObservationPageRequest::bounded(
                page_filter,
                ObservationPageLimits::new(16, 2, 4_096),
                ObservationCancellation::Active,
            )
            .after(continuation),
        })
        .unwrap();
    let CommandOutcome::ObservationPage(Err(continuation_mismatch)) = outcome else {
        panic!("changed limits must require a fresh page sequence");
    };

    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationPage {
            snapshot: snapshot.clone(),
            request: ObservationPageRequest::bounded(
                ObservationFilter::all(),
                ObservationPageLimits::new(256, 256, u32::MAX),
                ObservationCancellation::Active,
            ),
        })
        .unwrap();
    let CommandOutcome::ObservationPage(Ok(ObservationPageDisposition::Ready { page: all })) =
        outcome
    else {
        panic!("the discovery page must be ready");
    };
    let execution_path = all
        .facts
        .iter()
        .find_map(|fact| match fact {
            ObservationFact::Readiness { subject, .. }
                if subject.kind() == ObservationSubjectKind::ExecutionPath =>
            {
                Some(subject.clone())
            }
            _ => None,
        })
        .expect("the fixture retains one execution-path readiness fact");
    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationCausalPath {
            snapshot: snapshot.clone(),
            selector: ObservationCausalSelector::new(&snapshot, &execution_path),
        })
        .unwrap();
    let CommandOutcome::ObservationCausalPath(causal_path) = outcome else {
        panic!("causal path returned the wrong closed outcome");
    };
    let causal_path = causal_path.expect("the selector belongs to this snapshot");
    let causal_kind = if causal_path.is_some() {
        "ready"
    } else {
        "absent"
    };

    let lens_limits = ObservationLensLimits::new(256, 1, 4_096, 1, 256);
    let outcome = dispatcher
        .dispatch(CommandRequest::ObservationVisualLens {
            snapshot: snapshot.clone(),
            request: ObservationLensRequest::bounded(
                ObservationFilter::all().with_subject(&execution_path),
                lens_limits,
                true,
                ObservationCancellation::Active,
            ),
        })
        .unwrap();
    let CommandOutcome::ObservationVisualLens(lens) = outcome else {
        panic!("observation lens returned the wrong closed outcome");
    };
    let page_projection = match &page {
        ObservationPageDisposition::Ready { page } => serde_json::json!({
            "kind": "ready",
            "page": {
                "cursor": page.cursor,
                "facts": page.facts,
                "continuation": page.continuation.as_ref().map(|cursor| serde_json::json!({
                    "snapshot": cursor.snapshot(),
                    "inspectedPosition": cursor.inspected_position(),
                })),
                "completion": page.completion,
                "inspected": page.inspected,
                "copiedStringBytes": page.copied_string_bytes,
                "snapshotOmittedCount": page.snapshot_omitted_count,
            }
        }),
        ObservationPageDisposition::Cancelled { .. } => {
            panic!("the active page request cannot be cancelled")
        }
    };
    let lens_projection = match &lens {
        ObservationLensDisposition::Ready { lens } => serde_json::json!({
            "kind": "ready",
            "lens": {
                "cursor": lens.cursor,
                "nodes": lens.nodes,
                "causalPaths": lens.causal_paths,
                "continuation": lens.continuation.as_ref().map(|cursor| serde_json::json!({
                    "snapshot": cursor.snapshot(),
                    "inspectedPosition": cursor.inspected_position(),
                })),
                "completion": lens.completion,
                "inspectedSubjects": lens.inspected_subjects,
                "copiedStringBytes": lens.copied_string_bytes,
                "causalPathLookups": lens.causal_path_lookups,
                "causalSteps": lens.causal_steps,
                "snapshotOmittedCount": lens.snapshot_omitted_count,
            }
        }),
        ObservationLensDisposition::Resynchronize { .. }
        | ObservationLensDisposition::Cancelled { .. } => {
            panic!("the active first lens step must be ready")
        }
    };

    let actual = serde_json::json!({
        "snapshot": snapshot.summary(),
        "pageLimits": page_limits,
        "page": page_projection,
        "continuationMismatch": {
            "kind": "resynchronize",
            "resynchronization": continuation_mismatch,
        },
        "causalPath": {
            "kind": causal_kind,
            "path": causal_path,
        },
        "lensLimits": lens_limits,
        "lens": lens_projection,
    });
    assert_eq!(actual, expected);
}

#[test]
fn plan_binding_and_saved_admission_remain_effect_free_and_composable() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let targets = dispatcher
        .dispatch(CommandRequest::OneShotObservationTargets {
            compilation: one_shot_request(),
            repository_id: "engine".to_owned(),
            kind: OneShotPlanKind::Apply,
            prior: OneShotPriorMaterialization::Unowned,
        })
        .unwrap();
    let CommandOutcome::OneShotObservationTargets(targets) = targets else {
        panic!("observation target request returned the wrong closed outcome");
    };
    let candidate = dispatcher
        .dispatch(CommandRequest::OneShotPlanning {
            compilation: one_shot_request(),
            request: OneShotPlanningRequest::new(
                "engine".to_owned(),
                OneShotPlanKind::Apply,
                OneShotPriorMaterialization::Unowned,
                absent_baseline(&targets),
            ),
        })
        .unwrap()
        .into_one_shot_plan_candidate()
        .unwrap();
    assert_eq!(candidate.deferred_effect_count(), 0);
    let bound = dispatcher
        .dispatch(CommandRequest::OneShotMachineEffectBinding {
            candidate,
            effects: Vec::new(),
        })
        .unwrap()
        .into_one_shot_bound_plan()
        .unwrap();
    let saved_bytes = bound.canonical_bytes().to_vec();
    let admitted = dispatcher
        .dispatch(CommandRequest::OneShotSavedPlanAdmission {
            rebuilt: bound,
            saved_bytes,
        })
        .unwrap()
        .into_one_shot_admitted_plan()
        .unwrap();
    assert_eq!(admitted.kind(), OneShotPlanKind::Apply);
    assert_eq!(admitted.repository_id(), "engine");
    assert_eq!(admitted.plan_digest().len(), 64);
}

#[test]
#[cfg(feature = "native-local")]
fn current_executable_verification_recompiles_then_uses_the_sdk_check() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();
    let (_direct_root, direct_request) = local_executable_binding_request();
    let direct = api
        .compile_local_one_shot(direct_request)
        .and_then(|compilation| api.verify_current_executable_binding(&compilation))
        .unwrap();
    let (_dispatched_root, dispatched_request) = local_executable_binding_request();
    let outcome = dispatcher
        .dispatch(CommandRequest::CurrentExecutableBindingVerification(
            dispatched_request,
        ))
        .unwrap();
    let CommandOutcome::CurrentExecutableBindingVerification(dispatched) = outcome else {
        panic!("executable binding verification returned the wrong closed outcome");
    };
    assert_eq!(dispatched.canonical_bytes(), direct.canonical_bytes());
}

#[test]
#[cfg(feature = "native-local")]
fn local_materialization_commands_retain_one_compilation_through_settlement() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let (temporary, request) = local_executable_binding_request();
    let outcome = dispatcher
        .dispatch(CommandRequest::LocalOneShotCompilation(request))
        .unwrap();
    let CommandOutcome::LocalOneShotCompilation(compilation) = outcome else {
        panic!("local compilation returned the wrong closed outcome");
    };
    let summary = compilation.summary();
    let summary_bytes = summary.canonical_bytes().to_vec();
    let repository_id = summary.repositories()[0].repository_id().to_owned();
    let direct_status = PaleApi::new()
        .unwrap()
        .inspect_local_workspace_status(&compilation)
        .unwrap();

    let outcome = dispatcher
        .dispatch(CommandRequest::LocalMaterialization(
            LocalMaterializationCommand::InspectWorkspace { compilation },
        ))
        .unwrap();
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::WorkspaceStatus {
        compilation,
        status,
    }) = outcome
    else {
        panic!("local status returned the wrong closed outcome");
    };
    assert_eq!(status.canonical_bytes(), direct_status.canonical_bytes());
    assert_eq!(status.repositories()[0].repository_id(), repository_id);
    assert_eq!(compilation.summary().canonical_bytes(), summary_bytes);

    let plans = temporary.path().join("plans");
    let outcome = dispatcher
        .dispatch(CommandRequest::LocalMaterialization(
            LocalMaterializationCommand::PublishPlans(LocalSavedPlanPublicationRequest::new(
                compilation,
                OneShotPlanKind::Apply,
                plans.display().to_string(),
            )),
        ))
        .unwrap();
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::PlanPublication(
        LocalSavedPlanPublicationDisposition::Complete {
            compilation,
            publications,
        },
    )) = outcome
    else {
        panic!("local plan publication returned the wrong closed outcome");
    };
    assert_eq!(publications.len(), 1);
    assert!(publications[0].counts().total_operations() > 0);
    assert_eq!(publications[0].publication().repository_id(), repository_id);
    assert!(plans
        .join(publications[0].publication().relative_target())
        .is_file());
    assert_eq!(compilation.summary().canonical_bytes(), summary_bytes);

    let evidence = temporary.path().join("receipts");
    let outcome = dispatcher
        .dispatch(CommandRequest::LocalMaterialization(
            LocalMaterializationCommand::Admit {
                compilation,
                request: pale_sdk::LocalMaterializationAdmissionRequest::new(
                    OneShotPlanKind::Apply,
                    plans.display().to_string(),
                    evidence.display().to_string(),
                ),
            },
        ))
        .unwrap();
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::Admitted(run)) = outcome
    else {
        panic!("local materialization admission returned the wrong closed outcome");
    };
    assert!(!evidence.exists());
    assert_eq!(run.targets().len(), 1);
    assert_eq!(run.targets()[0].repository_id(), repository_id);

    let outcome = dispatcher
        .dispatch(CommandRequest::LocalMaterialization(
            LocalMaterializationCommand::Settle(run),
        ))
        .unwrap();
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::Settlement(
        LocalMaterializationSettlementDisposition::Settled(result),
    )) = outcome
    else {
        panic!("local settlement returned the wrong closed outcome");
    };
    assert_eq!(
        result.outcome(),
        pale_sdk::LocalMaterializationRunOutcome::Succeeded
    );
    assert_eq!(result.targets().len(), 1);
}

#[test]
#[cfg(feature = "native-local")]
fn rejected_local_settlement_returns_the_same_run_for_permitted_readback_only() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();
    let admit_without_executable_binding = |api: &PaleApi| {
        let (temporary, request) = local_executable_binding_request();
        let machine_path = temporary.path().join("workspace/bindings/machines/m.toml");
        let mut machine = toml::from_str::<toml::Value>(
            &std::fs::read_to_string(&machine_path).expect("machine binding is readable"),
        )
        .expect("machine binding is TOML");
        assert!(
            machine
                .as_table_mut()
                .expect("machine binding is a table")
                .remove("surface_executable")
                .is_some(),
            "fixture removes the executable binding before compilation"
        );
        std::fs::write(
            &machine_path,
            toml::to_string(&machine).expect("machine binding serializes"),
        )
        .expect("unbound machine fixture is written");

        let compilation = api
            .compile_local_one_shot(request)
            .expect("an unbound local workspace still compiles");
        let repository_id = compilation.summary().repositories()[0]
            .repository_id()
            .to_owned();
        let plan = api
            .plan_local_one_shot(
                &compilation,
                pale_sdk::LocalOneShotPlanRequest::new(repository_id, OneShotPlanKind::Apply),
            )
            .expect("the unbound workspace can still produce an effect-free plan");
        let plans = temporary.path().join("plans");
        api.publish_local_one_shot_plan(&plan, plans.display().to_string())
            .expect("the exact saved plan is published before admission");
        let evidence = temporary.path().join("receipts");
        let run = api
            .admit_local_materialization_run(
                &compilation,
                pale_sdk::LocalMaterializationAdmissionRequest::new(
                    OneShotPlanKind::Apply,
                    plans.display().to_string(),
                    evidence.display().to_string(),
                ),
            )
            .expect("missing executable binding is rejected only at settlement");
        (temporary, run, evidence)
    };

    let (_direct_fixture, mut direct_run, direct_evidence) = admit_without_executable_binding(&api);
    let direct_error = api
        .settle_local_materialization_run(&mut direct_run)
        .expect_err("the direct SDK rejects the missing executable binding");
    assert_eq!(
        direct_error.diagnostic().effect(),
        pale_sdk::DiagnosticEffectCertainty::NotStarted
    );
    assert!(!direct_evidence.exists());

    let (_dispatched_fixture, dispatched_run, dispatched_evidence) =
        admit_without_executable_binding(&api);
    let outcome = dispatcher
        .dispatch(CommandRequest::LocalMaterialization(
            LocalMaterializationCommand::Settle(Box::new(dispatched_run)),
        ))
        .expect("the dispatcher returns rejected settlement as an owned outcome");
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::Settlement(
        LocalMaterializationSettlementDisposition::Rejected { error, run },
    )) = outcome
    else {
        panic!("rejected local settlement returned the wrong closed outcome");
    };
    assert_eq!(
        error.diagnostic().canonical_bytes(),
        direct_error.diagnostic().canonical_bytes()
    );
    assert!(!dispatched_evidence.exists());

    let direct_readback_error = api
        .inspect_local_materialization_reconciliation(&run)
        .expect_err("a pre-effect rejection is not an outcome-unknown readback");
    let dispatched_readback_error = dispatcher
        .dispatch(CommandRequest::LocalMaterialization(
            LocalMaterializationCommand::Reconcile(run),
        ))
        .expect_err("the same retained run rejects an unpermitted readback");
    assert_eq!(
        dispatched_readback_error.diagnostic().canonical_bytes(),
        direct_readback_error.diagnostic().canonical_bytes()
    );
}
