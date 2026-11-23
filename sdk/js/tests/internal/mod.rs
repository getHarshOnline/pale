//! Node binding eligibility and callable-witness conformance.

use super::eligible_capability_ids;
use pale_sdk::{capability_inventory, ApiCapabilityId, ApiLeaf};

mod authoring_errors;
mod effective_profile_entries;
mod effective_profile_explanation;
mod profile_dispositions;
mod subject_inspection;
mod workspace_subject_value;

fn callable_capability_witnesses() -> Vec<ApiCapabilityId> {
    macro_rules! witness {
        ($id:expr, $call:path) => {{
            let _callable = $call;
            $id
        }};
    }
    vec![
        witness!(
            ApiCapabilityId::ContractBundleIdentity,
            super::Pale::contract_bundle_identity
        ),
        witness!(
            ApiCapabilityId::ContractSchemaLookup,
            super::Pale::contract_schema
        ),
        witness!(
            ApiCapabilityId::ContractDocumentValidation,
            super::Pale::validate_contract_document
        ),
        witness!(
            ApiCapabilityId::DiagnosticProjection,
            super::Pale::project_diagnostic
        ),
        witness!(
            ApiCapabilityId::DiagnosticAdmission,
            super::Pale::admit_diagnostic
        ),
        witness!(
            ApiCapabilityId::PackageDistributionInspection,
            super::Pale::inspect_package_distribution
        ),
        witness!(
            ApiCapabilityId::AuthoringIngressLimits,
            super::Pale::authoring_ingress_limits
        ),
        witness!(
            ApiCapabilityId::StandaloneAuthoringValidation,
            super::Pale::validate_standalone_authoring
        ),
        witness!(
            ApiCapabilityId::StandaloneAuthoringInspection,
            super::Pale::inspect_standalone_authoring
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceCompilation,
            super::Pale::compile_portable_workspace
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceAuthoringInspection,
            super::Pale::inspect_portable_workspace_authoring
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceAuthoringNavigation,
            super::Pale::navigate_portable_workspace_authoring
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceAuthoringOccurrences,
            super::Pale::portable_workspace_authoring_occurrences
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceSourceSubjects,
            super::Pale::portable_workspace_source_subject
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceSubjectInspection,
            super::PortableWorkspaceCompilation::inspect_subject
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceOrganizationInspection,
            super::PortableWorkspaceCompilation::inspect_organization
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceComparison,
            super::Pale::compare_portable_workspaces
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceDirectDependents,
            super::PortableWorkspaceCompilation::direct_dependents
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceDirectDependencies,
            super::PortableWorkspaceCompilation::direct_dependencies
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceComparisonSubjects,
            super::WorkspaceComparison::subjects
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceContributingRoots,
            super::WorkspaceComparison::contributing_roots
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceChangeRoots,
            super::WorkspaceComparison::change_roots
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceImpacts,
            super::WorkspaceComparison::impacts
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceExplanation,
            super::WorkspaceComparison::explain
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceSubjectComparison,
            super::WorkspaceComparison::compare_subject
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileCompilation,
            super::Pale::compile_effective_profile
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileInspection,
            super::EffectiveProfileCompilation::inspect
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileExplanation,
            super::EffectiveProfileCompilation::explain
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringInspection,
            super::EffectiveProfileCompilation::inspect_authoring
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringNavigation,
            super::EffectiveProfileCompilation::navigate_authoring
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringOccurrences,
            super::EffectiveProfileCompilation::authoring_occurrences
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringJoin,
            super::EffectiveProfileCompilation::inspect_effective_authoring
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringPages,
            super::EffectiveProfileCompilation::inspect_effective_authoring_page
        ),
        witness!(
            ApiCapabilityId::ConfigurationImprovement,
            super::Pale::inspect_configuration_improvement
        ),
        witness!(
            ApiCapabilityId::ObservationSnapshot,
            super::Pale::observe_effective_profile
        ),
        witness!(
            ApiCapabilityId::ObservationPage,
            super::ObservationSnapshot::page
        ),
        witness!(
            ApiCapabilityId::ObservationProfileInspection,
            super::ObservationSnapshot::inspect_profile
        ),
        witness!(
            ApiCapabilityId::ObservationDelta,
            super::ObservationSnapshot::delta_from
        ),
        witness!(
            ApiCapabilityId::ObservationCausalPath,
            super::ObservationSnapshot::causal_path
        ),
        witness!(
            ApiCapabilityId::ObservationVisualLens,
            super::ObservationSnapshot::visual_lens
        ),
        witness!(
            ApiCapabilityId::NativeActionCatalog,
            super::Pale::native_action_catalog
        ),
        witness!(
            ApiCapabilityId::NativeActionInvocation,
            super::Pale::invoke_native_action
        ),
        witness!(
            ApiCapabilityId::NativeActionPayloadInvocation,
            super::Pale::invoke_native_action_payload
        ),
        witness!(
            ApiCapabilityId::RepositoryNativeActionAdmission,
            super::Pale::admit_repository_native_actions
        ),
        witness!(
            ApiCapabilityId::ResidentHookAdmission,
            super::Pale::admit_resident_hook
        ),
        witness!(
            ApiCapabilityId::OneShotCompilation,
            super::Pale::compile_one_shot
        ),
        witness!(
            ApiCapabilityId::OneShotCompilationSummary,
            super::Pale::one_shot_summary
        ),
        witness!(
            ApiCapabilityId::OneShotObservationTargets,
            super::Pale::one_shot_observation_targets
        ),
        witness!(ApiCapabilityId::OneShotPlanning, super::Pale::plan_one_shot),
        witness!(
            ApiCapabilityId::OneShotMachineEffectBinding,
            super::Pale::bind_one_shot_machine_effects
        ),
        witness!(
            ApiCapabilityId::OneShotSavedPlanAdmission,
            super::Pale::admit_one_shot_saved_plan
        ),
        witness!(
            ApiCapabilityId::LocalOneShotCompilation,
            super::Pale::compile_local_one_shot
        ),
        witness!(
            ApiCapabilityId::LocalPackageDistributionInspection,
            super::Pale::inspect_local_package_distribution
        ),
        witness!(
            ApiCapabilityId::LocalNativeActionInputCapture,
            super::Pale::capture_local_native_action_request
        ),
        witness!(
            ApiCapabilityId::LocalRepositoryStatus,
            super::Pale::inspect_local_repository_status
        ),
        witness!(
            ApiCapabilityId::LocalOneShotPlanning,
            super::Pale::plan_local_one_shot
        ),
        witness!(
            ApiCapabilityId::LocalOneShotPlanPublication,
            super::Pale::publish_local_one_shot_plan
        ),
        witness!(
            ApiCapabilityId::LocalMaterializationRunAdmission,
            super::Pale::admit_local_materialization_run
        ),
        witness!(
            ApiCapabilityId::LocalMaterializationRunSettlement,
            super::Pale::settle_local_materialization_run
        ),
        witness!(
            ApiCapabilityId::LocalMaterializationReconciliationInspection,
            super::Pale::inspect_local_materialization_reconciliation
        ),
        witness!(
            ApiCapabilityId::CompiledInterfaceIdentity,
            super::Pale::compiled_interface_identity
        ),
        witness!(
            ApiCapabilityId::CurrentExecutableBindingVerification,
            super::Pale::verify_current_executable_binding
        ),
        witness!(
            ApiCapabilityId::LocalControlStorageInspection,
            super::Pale::inspect_local_control_store
        ),
        witness!(
            ApiCapabilityId::LocalConfigurationImprovementPreview,
            super::Pale::preview_local_configuration_improvement
        ),
    ]
}

#[test]
fn exported_capabilities_equal_the_api_owned_node_allowlist() {
    let expected = capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Node))
        .map(|capability| capability.id())
        .collect::<Vec<_>>();

    let mut witnessed_ids = callable_capability_witnesses()
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>();
    let mut expected_ids = expected
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>();
    witnessed_ids.sort_unstable();
    expected_ids.sort_unstable();
    assert_eq!(witnessed_ids, expected_ids);
    assert_eq!(
        eligible_capability_ids(),
        expected
            .iter()
            .map(|capability| capability.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn bundle_identity_matches_the_shared_api_semantics() {
    let api = pale_sdk::PaleApi::new().expect("API initializes");
    let node = super::Pale::new().expect("Node leaf initializes");
    let expected = api.contract_bundle_identity();
    let actual = node.contract_bundle_identity();
    assert_eq!(
        actual.schema_manifest_sha256,
        expected.schema_manifest_sha256()
    );
    assert_eq!(actual.schema_set_sha256, expected.schema_set_sha256());
    assert_eq!(actual.member_count, expected.member_count());
}

#[test]
fn authoring_limits_and_structural_facts_are_exact_api_projections() {
    let node = super::Pale::new().expect("Node leaf initializes");
    let expected_limits =
        pale_sdk::PaleApi::authoring_ingress_limits(pale_sdk::AuthoringIngressClass::Workspace);
    let actual_limits = node
        .authoring_ingress_limits(super::AuthoringIngressClass::Workspace)
        .expect("workspace limits fit the Node ABI");
    assert_eq!(
        actual_limits.max_documents,
        expected_limits.max_documents() as u32
    );
    assert_eq!(
        actual_limits.max_document_bytes.get_u64(),
        (false, expected_limits.max_document_bytes(), true)
    );
    assert_eq!(
        actual_limits.max_total_bytes.get_u64(),
        (false, expected_limits.max_total_bytes(), true)
    );

    let expected = node
        .api
        .inspect_standalone_authoring(
            pale_sdk::StandaloneAuthoringRole::WorkspaceManifest,
            Vec::new(),
        )
        .expect("embedded workspace grammar is coherent");
    let expected_schema = expected.schema_id().map(str::to_owned);
    let expected_fields = expected
        .node()
        .expect("workspace grammar has a root node")
        .fields()
        .iter()
        .map(|field| field.name().to_owned())
        .collect::<Vec<_>>();
    let actual = super::AuthoringInspection::from_api(expected);
    assert_eq!(actual.schema_id, expected_schema);
    assert_eq!(
        actual
            .node
            .expect("Node projection retains the root node")
            .fields
            .into_iter()
            .map(|field| field.name)
            .collect::<Vec<_>>(),
        expected_fields
    );
}

#[test]
fn node_host_and_api_handles_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<pale_sdk::PaleApi>();
    assert_send_sync::<super::Pale>();
}
