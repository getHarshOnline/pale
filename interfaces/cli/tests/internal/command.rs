//! Laws for the shared typed command dispatcher.

use pale_sdk::{
    ApiError, ContractDocument, EffectiveProfilePageRequest, EffectiveProfileSection,
    NativeActionDescriptor, NativeActionPayloadRequest, NativeActionRequest, OneShotPlanKind,
    OneShotPlanningRequest, OneShotPriorMaterialization, PaleApi, PortableWorkspaceDocument,
    PortableWorkspaceExplanationState, PortableWorkspaceImpactState, PortableWorkspaceRequest,
    PortableWorkspaceSubject, PortableWorkspaceSubjectKind, PortableWorkspaceSubjectSelector,
    RepositoryNativeActionCatalogRequest,
};

const PATH_INVENTORY_ACTION_ID: &str = "pale.compute.path-inventory";

use super::{
    CommandDispatcher, CommandOutcome, CommandRequest, EffectiveProfileInspectionRequest,
    OperationHandleLocator, PortableWorkspaceExplanationRequest,
    ResidentOperationObservationRequest,
};

use crate::test_fixtures::{
    absent_baseline, effective_profile_request, empty_workspace_request, one_shot_request,
};

#[path = "command/convergence.rs"]
mod convergence;

#[path = "command/configuration_improvement.rs"]
mod configuration_improvement;

/// Build one exact repository catalog from API-owned action descriptors.
fn repository_catalog_request(
    descriptors: &[NativeActionDescriptor],
) -> RepositoryNativeActionCatalogRequest {
    let profile_digest = "a".repeat(64);
    let instance_digest = "b".repeat(64);
    let repository_id = "pale".to_string();
    let schema = PaleApi::new()
        .unwrap()
        .contract_schema("repository-native-action-catalog.v1")
        .expect("repository action catalog identity is available through the API");
    let contracts = descriptors
        .iter()
        .map(|descriptor| {
            serde_json::from_slice::<serde_json::Value>(descriptor.canonical_contract_bytes())
                .expect("API descriptor contract is JSON")
        })
        .collect::<Vec<_>>();
    let catalog = serde_json::json!({
        "schema": schema
            .wire_schema_id()
            .expect("repository action catalog has a wire identity"),
        "schema_sha256": schema.schema_sha256(),
        "schema_version": 1,
        "profile_digest": profile_digest,
        "instance_digest": instance_digest,
        "repository_id": repository_id,
        "contracts": contracts,
    });
    let bytes = serde_json::to_vec(&catalog).expect("repository action catalog serializes");
    RepositoryNativeActionCatalogRequest::new(bytes, profile_digest, instance_digest, repository_id)
}

/// Assert that a dispatcher failure retains one exact canonical API diagnostic.
fn assert_canonical_failure(error: &ApiError, expected_code: &str) {
    let bytes = error.diagnostic().canonical_bytes();
    let value: serde_json::Value =
        serde_json::from_slice(bytes).expect("dispatcher failure is JSON");
    assert_eq!(value["code"], expected_code);
    assert_eq!(value["effect"], "not_applicable");
    assert_eq!(
        PaleApi::new()
            .unwrap()
            .validate_contract_document(ContractDocument::new(
                "diagnostic.v1".to_string(),
                bytes.to_vec(),
            ))
            .expect("dispatcher failure satisfies the public diagnostic contract")
            .canonical_bytes(),
        bytes
    );
}

#[test]
fn resident_observation_is_unavailable_without_an_attachment() {
    let dispatcher = CommandDispatcher::new().unwrap();
    assert!(!dispatcher.has_resident_attachment());

    let error = dispatcher
        .dispatch(CommandRequest::ResidentOperationObservation(
            ResidentOperationObservationRequest::new(
                OperationHandleLocator::new("operation-handle/test".to_string(), "a".repeat(64)),
                Vec::new(),
            ),
        ))
        .unwrap_err();

    assert_canonical_failure(&error, "resident_observation_unavailable");
}

#[test]
fn contract_queries_and_validation_dispatch_owned_api_results() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let bundle = dispatcher
        .dispatch(CommandRequest::ContractBundleIdentity)
        .unwrap();
    let CommandOutcome::ContractBundleIdentity(bundle) = bundle else {
        panic!("bundle request returned the wrong closed outcome");
    };
    assert!(bundle.member_count() > 0);
    assert_eq!(bundle.schema_manifest_sha256().len(), 64);
    assert_eq!(bundle.schema_set_sha256().len(), 64);

    let schema = dispatcher
        .dispatch(CommandRequest::ContractSchema(
            "action-input.v1".to_string(),
        ))
        .unwrap();
    let CommandOutcome::ContractSchema(schema) = schema else {
        panic!("schema request returned the wrong closed outcome");
    };
    assert_eq!(schema.registry_id(), "action-input.v1");

    let document = br#"{
        "kind":"path_inventory",
        "payload":{"style":"posix_relative","paths":["src/./lib.rs"]}
    }"#;
    let validated = dispatcher
        .dispatch(CommandRequest::ValidateContractDocument(
            ContractDocument::new("action-input.v1".to_string(), document.to_vec()),
        ))
        .unwrap();
    let CommandOutcome::ValidatedContractDocument(validated) = validated else {
        panic!("validation request returned the wrong closed outcome");
    };
    assert_eq!(validated.schema_id(), "action-input.v1");
    assert_eq!(validated.schema_sha256(), schema.schema_sha256());
    let value: serde_json::Value = serde_json::from_slice(document).unwrap();
    let validated_value: serde_json::Value =
        serde_json::from_slice(validated.canonical_bytes()).unwrap();
    assert_eq!(validated_value, value);
}

#[test]
fn consuming_outcome_projections_preserve_unmatched_variants() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let outcome = dispatcher
        .dispatch(CommandRequest::ContractBundleIdentity)
        .unwrap();
    assert!(matches!(
        outcome.into_native_action(),
        Err(outcome) if matches!(*outcome, CommandOutcome::ContractBundleIdentity(_))
    ));

    let outcome = dispatcher
        .dispatch(CommandRequest::NativeActionCatalog)
        .unwrap();
    assert!(matches!(
        outcome.into_admitted_repository_native_actions(),
        Err(outcome) if matches!(*outcome, CommandOutcome::NativeActionCatalog(_))
    ));
}

#[test]
fn portable_dispatch_is_stateless_and_matches_direct_api_projections() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();

    let direct_summary = api
        .compile_portable_workspace(empty_workspace_request(None))
        .unwrap()
        .summary();
    let dispatched = dispatcher
        .dispatch(CommandRequest::PortableWorkspaceCompilation(
            empty_workspace_request(None),
        ))
        .unwrap();
    let CommandOutcome::PortableWorkspaceCompilationSummary(dispatched_summary) = dispatched else {
        panic!("portable compilation returned the wrong closed outcome");
    };
    assert_eq!(
        dispatched_summary.canonical_bytes(),
        direct_summary.canonical_bytes()
    );

    let prior = api
        .compile_portable_workspace(empty_workspace_request(None))
        .unwrap();
    let current = api
        .compile_portable_workspace(empty_workspace_request(Some("dispatcher-phase-two")))
        .unwrap();
    let comparison = api.compare_portable_workspaces(&prior, &current).unwrap();
    let direct_comparison_summary = comparison.summary();
    let dispatched = dispatcher
        .dispatch(CommandRequest::PortableWorkspaceComparison {
            prior: empty_workspace_request(None),
            current: empty_workspace_request(Some("dispatcher-phase-two")),
        })
        .unwrap();
    let CommandOutcome::PortableWorkspaceComparisonSummary(dispatched_summary) = dispatched else {
        panic!("portable comparison returned the wrong closed outcome");
    };
    assert_eq!(
        dispatched_summary.canonical_bytes(),
        direct_comparison_summary.canonical_bytes()
    );

    let direct_roots = comparison.change_roots(0, 2);
    let dispatched = dispatcher
        .dispatch(CommandRequest::PortableWorkspaceChangeRoots {
            prior: empty_workspace_request(None),
            current: empty_workspace_request(Some("dispatcher-phase-two")),
            offset: 0,
            limit: 2,
        })
        .unwrap();
    let CommandOutcome::PortableWorkspaceChangeRoots(dispatched_roots) = dispatched else {
        panic!("root paging returned the wrong closed outcome");
    };
    assert_eq!(dispatched_roots, direct_roots);

    let direct_impacts = comparison.impacts(0, u16::MAX);
    let dispatched = dispatcher
        .dispatch(CommandRequest::PortableWorkspaceImpacts {
            prior: empty_workspace_request(None),
            current: empty_workspace_request(Some("dispatcher-phase-two")),
            offset: 0,
            limit: u16::MAX,
        })
        .unwrap();
    let CommandOutcome::PortableWorkspaceImpacts(dispatched_impacts) = dispatched else {
        panic!("impact paging returned the wrong closed outcome");
    };
    assert_eq!(dispatched_impacts, direct_impacts);

    let selector = direct_impacts
        .entries()
        .iter()
        .find(|impact| {
            impact.state() == PortableWorkspaceImpactState::Affected
                && impact.subject().kind() == PortableWorkspaceSubjectKind::Workspace
        })
        .expect("the changed source reaches the workspace semantic IR")
        .subject()
        .selector();
    let subject = PortableWorkspaceSubject::try_from(selector.clone()).unwrap();
    let direct_explanation = comparison.explain(&subject).unwrap();
    assert_eq!(
        direct_explanation.state(),
        PortableWorkspaceExplanationState::Affected
    );
    let dispatched = dispatcher
        .dispatch(CommandRequest::PortableWorkspaceExplanation(
            PortableWorkspaceExplanationRequest::new(
                empty_workspace_request(None),
                empty_workspace_request(Some("dispatcher-phase-two")),
                selector,
            ),
        ))
        .unwrap();
    let CommandOutcome::PortableWorkspaceExplanation(dispatched_explanation) = dispatched else {
        panic!("portable explanation returned the wrong closed outcome");
    };
    assert_eq!(dispatched_explanation, direct_explanation);

    let invalid = PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: String::new(),
    };
    let direct_error = comparison.explain_selector(invalid.clone()).unwrap_err();
    let request_error = dispatcher
        .dispatch(CommandRequest::PortableWorkspaceExplanation(
            PortableWorkspaceExplanationRequest::new(
                empty_workspace_request(None),
                empty_workspace_request(Some("dispatcher-phase-two")),
                invalid,
            ),
        ))
        .unwrap_err();
    assert_eq!(
        request_error.diagnostic().canonical_bytes(),
        direct_error.diagnostic().canonical_bytes()
    );

    let direct_error = api
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_string(),
            vec![PortableWorkspaceDocument::new(
                "pale-workspace.toml".to_string(),
                b"not TOML".to_vec(),
            )],
        ))
        .unwrap_err();
    let dispatched_error = dispatcher
        .dispatch(CommandRequest::PortableWorkspaceCompilation(
            PortableWorkspaceRequest::new(
                "pale-workspace.toml".to_string(),
                vec![PortableWorkspaceDocument::new(
                    "pale-workspace.toml".to_string(),
                    b"not TOML".to_vec(),
                )],
            ),
        ))
        .unwrap_err();
    assert_eq!(
        dispatched_error.diagnostic().canonical_bytes(),
        direct_error.diagnostic().canonical_bytes()
    );
}

#[test]
fn effective_profile_dispatch_preserves_role_separation_and_api_owned_paging() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();
    let (direct_workspace, direct_profile) = effective_profile_request();
    let page =
        EffectiveProfilePageRequest::new(EffectiveProfileSection::CapabilitiesAndOperations, 0, 7);
    let direct = api
        .compile_portable_workspace(direct_workspace)
        .and_then(|workspace| api.compile_effective_profile(&workspace, direct_profile))
        .map(|profile| profile.inspect(page))
        .unwrap();

    let (workspace, profile) = effective_profile_request();
    let dispatched = dispatcher
        .dispatch(CommandRequest::EffectiveProfileInspection(
            EffectiveProfileInspectionRequest::new(workspace, profile, page),
        ))
        .unwrap();
    let CommandOutcome::EffectiveProfilePage(dispatched) = dispatched else {
        panic!("effective-profile inspection returned the wrong closed outcome");
    };
    assert_eq!(dispatched, direct);
}

#[test]
fn one_shot_dispatch_recompiles_without_retaining_state() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();

    let direct_compilation = api.compile_one_shot(one_shot_request()).unwrap();
    let direct_summary = direct_compilation.summary();
    let dispatched = dispatcher
        .dispatch(CommandRequest::OneShotCompilation(one_shot_request()))
        .unwrap();
    let CommandOutcome::OneShotCompilationSummary(dispatched_summary) = dispatched else {
        panic!("one-shot compilation returned the wrong closed outcome");
    };
    assert_eq!(
        dispatched_summary.canonical_bytes(),
        direct_summary.canonical_bytes()
    );

    let prior = OneShotPriorMaterialization::Unowned;
    let direct_targets = api
        .one_shot_observation_targets(
            &direct_compilation,
            "engine".to_string(),
            OneShotPlanKind::Apply,
            &prior,
        )
        .unwrap();
    let dispatched = dispatcher
        .dispatch(CommandRequest::OneShotObservationTargets {
            compilation: one_shot_request(),
            repository_id: "engine".to_string(),
            kind: OneShotPlanKind::Apply,
            prior: OneShotPriorMaterialization::Unowned,
        })
        .unwrap();
    let CommandOutcome::OneShotObservationTargets(dispatched_targets) = dispatched else {
        panic!("observation-target derivation returned the wrong closed outcome");
    };
    assert_eq!(dispatched_targets, direct_targets);

    let direct_candidate = api
        .plan_one_shot(
            &direct_compilation,
            OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Apply,
                OneShotPriorMaterialization::Unowned,
                absent_baseline(&direct_targets),
            ),
        )
        .unwrap();
    let dispatched = dispatcher
        .dispatch(CommandRequest::OneShotPlanning {
            compilation: one_shot_request(),
            request: OneShotPlanningRequest::new(
                "engine".to_string(),
                OneShotPlanKind::Apply,
                OneShotPriorMaterialization::Unowned,
                absent_baseline(&direct_targets),
            ),
        })
        .unwrap();
    let CommandOutcome::OneShotPlanCandidate(dispatched_candidate) = dispatched else {
        panic!("one-shot planning returned the wrong closed outcome");
    };
    assert_eq!(
        dispatched_candidate.canonical_bytes(),
        direct_candidate.canonical_bytes()
    );
}

#[test]
fn native_catalog_and_repository_admission_share_the_dispatch_path() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let catalog = dispatcher
        .dispatch(CommandRequest::NativeActionCatalog)
        .unwrap();
    let CommandOutcome::NativeActionCatalog(descriptors) = catalog else {
        panic!("catalog request returned the wrong closed outcome");
    };
    assert!(!descriptors.is_empty());

    let admitted = dispatcher
        .dispatch(CommandRequest::AdmitRepositoryNativeActions(
            repository_catalog_request(&descriptors),
        ))
        .unwrap()
        .into_admitted_repository_native_actions()
        .unwrap();
    assert_eq!(admitted.repository_id(), "pale");
    assert_eq!(admitted.descriptors(), descriptors);
}

#[test]
fn complete_and_payload_ingress_converge_on_one_canonical_execution() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let complete = dispatcher
        .dispatch(CommandRequest::NativeAction(NativeActionRequest::new(
            PATH_INVENTORY_ACTION_ID.to_string(),
            br#"{"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs","src/lib.rs"]}}"#.to_vec(),
        )))
        .unwrap();
    let payload = dispatcher
        .dispatch(CommandRequest::NativeActionPayload(
            NativeActionPayloadRequest::new(
                PATH_INVENTORY_ACTION_ID.to_string(),
                br#"{"style":"posix_relative","paths":["src/./lib.rs","src/lib.rs"]}"#.to_vec(),
            ),
        ))
        .unwrap()
        .into_native_action()
        .unwrap();

    let complete = complete.into_native_action().unwrap();

    assert_eq!(complete, payload);
    assert_eq!(complete.canonical_bytes(), payload.canonical_bytes());
}

#[test]
fn complete_and_payload_ingress_converge_on_one_canonical_diagnostic() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let complete = dispatcher
        .dispatch(CommandRequest::NativeAction(NativeActionRequest::new(
            PATH_INVENTORY_ACTION_ID.to_string(),
            br#"{"kind":"path_inventory","payload":{}}"#.to_vec(),
        )))
        .unwrap_err();
    let payload = dispatcher
        .dispatch(CommandRequest::NativeActionPayload(
            NativeActionPayloadRequest::new(PATH_INVENTORY_ACTION_ID.to_string(), b"{}".to_vec()),
        ))
        .unwrap_err();

    assert_canonical_failure(&complete, "malformed_input");
    assert_canonical_failure(&payload, "malformed_input");
    assert_eq!(
        complete.diagnostic().canonical_bytes(),
        payload.diagnostic().canonical_bytes(),
        "the dispatcher must preserve one API-owned failure projection across carrier ingresses"
    );
    let diagnostic: serde_json::Value =
        serde_json::from_slice(complete.diagnostic().canonical_bytes()).unwrap();
    assert_eq!(diagnostic["code"], "malformed_input");
    assert_eq!(diagnostic["disposition"], "invalid_input");
    assert_eq!(diagnostic["effect"], "not_applicable");
}

#[test]
fn repository_catalog_admission_uses_the_same_typed_boundary() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let error = dispatcher
        .dispatch(CommandRequest::AdmitRepositoryNativeActions(
            RepositoryNativeActionCatalogRequest::new(
                b"{}".to_vec(),
                "not-a-profile-digest".to_string(),
                "not-an-instance-digest".to_string(),
                "repository".to_string(),
            ),
        ))
        .unwrap_err();

    assert_canonical_failure(&error, "contract_rejected");
    let diagnostic: serde_json::Value =
        serde_json::from_slice(error.diagnostic().canonical_bytes()).unwrap();
    assert_eq!(diagnostic["code"], "contract_rejected");
    assert_eq!(
        diagnostic["subject"]["id"],
        "repository-native-action-catalog.v1"
    );
}

#[test]
fn contract_dispatch_failures_keep_api_owned_canonical_diagnostics() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let unavailable = dispatcher
        .dispatch(CommandRequest::ContractSchema("unknown.v1".to_string()))
        .unwrap_err();
    assert_canonical_failure(&unavailable, "contract_rejected");

    let malformed = dispatcher
        .dispatch(CommandRequest::ValidateContractDocument(
            ContractDocument::new("action-input.v1".to_string(), b"not JSON".to_vec()),
        ))
        .unwrap_err();
    assert_canonical_failure(&malformed, "invalid_json");
}
