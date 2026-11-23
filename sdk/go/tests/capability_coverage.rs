//! Exact correspondence between API-owned Go eligibility and the native ABI.

use pale_go_ffi::*;
use pale_sdk::{capability_inventory, ApiCapabilityId, ApiLeaf};

type Operation = extern "C" fn(u64, u64) -> u64;

#[test]
fn every_eligible_capability_has_one_native_entry() {
    use ApiCapabilityId::*;

    let bindings: &[(ApiCapabilityId, Operation)] = &[
        (ContractBundleIdentity, pale_go_contract_bundle_identity),
        (ContractSchemaLookup, pale_go_contract_schema),
        (
            ContractDocumentValidation,
            pale_go_validate_contract_document,
        ),
        (DiagnosticProjection, pale_go_project_diagnostic),
        (DiagnosticAdmission, pale_go_admit_diagnostic),
        (
            PackageDistributionInspection,
            pale_go_inspect_package_distribution,
        ),
        (AuthoringIngressLimits, pale_go_authoring_ingress_limits),
        (
            StandaloneAuthoringValidation,
            pale_go_validate_authoring_document,
        ),
        (
            StandaloneAuthoringInspection,
            pale_go_inspect_standalone_authoring,
        ),
        (
            PortableWorkspaceCompilation,
            pale_go_compile_portable_workspace,
        ),
        (
            PortableWorkspaceAuthoringInspection,
            pale_go_inspect_portable_workspace_authoring,
        ),
        (
            PortableWorkspaceAuthoringNavigation,
            pale_go_navigate_portable_workspace_authoring,
        ),
        (
            PortableWorkspaceComparison,
            pale_go_compare_portable_workspaces,
        ),
        (
            PortableWorkspaceSubjectComparison,
            pale_go_compare_portable_workspace_subject,
        ),
        (
            PortableWorkspaceAuthoringOccurrences,
            pale_go_portable_authoring_occurrences,
        ),
        (
            EffectiveProfileAuthoringOccurrences,
            pale_go_effective_authoring_occurrences,
        ),
        (
            EffectiveProfileAuthoringPages,
            pale_go_effective_authoring_page,
        ),
        (
            PortableWorkspaceChangeRoots,
            pale_go_portable_workspace_change_roots,
        ),
        (PortableWorkspaceImpacts, pale_go_portable_workspace_impacts),
        (
            PortableWorkspaceExplanation,
            pale_go_explain_portable_workspace,
        ),
        (
            EffectiveProfileCompilation,
            pale_go_compile_effective_profile,
        ),
        (
            EffectiveProfileInspection,
            pale_go_inspect_effective_profile,
        ),
        (
            EffectiveProfileExplanation,
            pale_go_explain_effective_profile,
        ),
        (
            EffectiveProfileAuthoringInspection,
            pale_go_inspect_effective_profile_authoring,
        ),
        (
            EffectiveProfileAuthoringNavigation,
            pale_go_navigate_effective_profile_authoring,
        ),
        (
            EffectiveProfileAuthoringJoin,
            pale_go_join_effective_profile_authoring,
        ),
        (ConfigurationImprovement, pale_go_improvement_inspect),
        (ObservationSnapshot, pale_go_observe_effective_profile),
        (ObservationPage, pale_go_observation_page),
        (
            ObservationProfileInspection,
            pale_go_observation_inspect_profile,
        ),
        (ObservationDelta, pale_go_observation_delta),
        (ObservationCausalPath, pale_go_observation_causal_path),
        (ObservationVisualLens, pale_go_observation_visual_lens),
        (NativeActionCatalog, pale_go_native_action_catalog),
        (NativeActionInvocation, pale_go_invoke_native_action),
        (
            NativeActionPayloadInvocation,
            pale_go_invoke_native_action_payload,
        ),
        (
            RepositoryNativeActionAdmission,
            pale_go_admit_repository_native_actions,
        ),
        (ResidentHookAdmission, pale_go_admit_resident_hook),
        (OneShotCompilation, pale_go_compile_one_shot),
        (OneShotCompilationSummary, pale_go_one_shot_summary),
        (
            OneShotObservationTargets,
            pale_go_one_shot_observation_targets,
        ),
        (OneShotPlanning, pale_go_plan_one_shot),
        (
            OneShotMachineEffectBinding,
            pale_go_bind_one_shot_machine_effects,
        ),
        (OneShotSavedPlanAdmission, pale_go_admit_one_shot_saved_plan),
        (LocalOneShotCompilation, pale_go_compile_local_one_shot),
        (
            LocalPackageDistributionInspection,
            pale_go_inspect_local_package_distribution,
        ),
        (
            LocalNativeActionInputCapture,
            pale_go_capture_local_native_action_request,
        ),
        (
            LocalRepositoryStatus,
            pale_go_inspect_local_repository_status,
        ),
        (LocalOneShotPlanning, pale_go_plan_local_one_shot),
        (
            LocalOneShotPlanPublication,
            pale_go_publish_local_one_shot_plan,
        ),
        (
            LocalMaterializationRunAdmission,
            pale_go_admit_local_materialization_run,
        ),
        (
            LocalMaterializationRunSettlement,
            pale_go_settle_local_materialization_run,
        ),
        (
            LocalMaterializationReconciliationInspection,
            pale_go_inspect_local_materialization_reconciliation,
        ),
        (
            CompiledInterfaceIdentity,
            pale_go_compiled_interface_identity,
        ),
        (
            CurrentExecutableBindingVerification,
            pale_go_verify_current_executable_binding,
        ),
        (
            LocalControlStorageInspection,
            pale_go_inspect_local_control_store,
        ),
        (
            LocalConfigurationImprovementPreview,
            pale_go_preview_local_configuration_improvement,
        ),
    ];
    let mut eligible = capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Go))
        .map(|capability| capability.id().as_str())
        .collect::<Vec<_>>();
    let mut callable = bindings
        .iter()
        .map(|(id, _)| id.as_str())
        .collect::<Vec<_>>();
    eligible.sort_unstable();
    callable.sort_unstable();
    assert_eq!(
        callable, eligible,
        "Go ABI callable witnesses must match eligibility, including multiplicity",
    );
}
