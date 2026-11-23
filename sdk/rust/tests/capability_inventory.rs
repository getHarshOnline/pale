//! Conformance gates for the closed public capability inventory.

use std::collections::BTreeSet;

use pale_sdk::{
    capability_inventory, ApiCancellationSemantics, ApiCapabilityClass, ApiCapabilityContract,
    ApiCapabilityEffect, ApiCapabilityId, ApiInvocationMode, ApiLeaf,
};

/// Match every public identity variant so additions require an explicit test decision.
const fn stable_id(id: ApiCapabilityId) -> &'static str {
    match id {
        ApiCapabilityId::A2aDelegateHandleInspection => "a2a.delegate-handle-inspection",
        ApiCapabilityId::ContractBundleIdentity => "contract.bundle-identity",
        ApiCapabilityId::ContractSchemaLookup => "contract.schema-lookup",
        ApiCapabilityId::ContractDocumentValidation => "contract.document-validation",
        ApiCapabilityId::DiagnosticProjection => "diagnostic.projection",
        ApiCapabilityId::DiagnosticAdmission => "diagnostic.admission",
        ApiCapabilityId::PackageDistributionInspection => "package-distribution.inspection",
        ApiCapabilityId::PackageCatalogInspection => "package-catalog.inspection",
        ApiCapabilityId::LocalPackageCatalogInspection => "local.package-catalog-inspection",
        ApiCapabilityId::LocalPackageIndexUpdate => "local.package-index-update",
        ApiCapabilityId::AuthoringIngressLimits => "authoring.ingress-limits",
        ApiCapabilityId::StandaloneAuthoringValidation => "authoring.standalone-validation",
        ApiCapabilityId::StandaloneAuthoringInspection => "authoring.standalone-inspection",
        ApiCapabilityId::PortableWorkspaceCompilation => "workspace.portable-compilation",
        ApiCapabilityId::PortableWorkspaceAuthoringInspection => {
            "workspace.portable-authoring-inspection"
        }
        ApiCapabilityId::PortableWorkspaceAuthoringNavigation => {
            "workspace.portable-authoring-navigation"
        }
        ApiCapabilityId::PortableWorkspaceAuthoringOccurrences => {
            "workspace.portable-authoring-occurrences"
        }
        ApiCapabilityId::PortableWorkspaceSourceSubjects => "workspace.portable-source-subjects",
        ApiCapabilityId::PortableWorkspaceComparison => "workspace.portable-comparison",
        ApiCapabilityId::PortableWorkspaceComparisonSubjects => {
            "workspace.portable-comparison-subjects"
        }
        ApiCapabilityId::PortableWorkspaceSubjectComparison => {
            "workspace.portable-subject-comparison"
        }
        ApiCapabilityId::PortableWorkspaceDirectDependencies => {
            "workspace.portable-direct-dependencies"
        }
        ApiCapabilityId::PortableWorkspaceSubjectInspection => {
            "workspace.portable-subject-inspection"
        }
        ApiCapabilityId::PortableWorkspaceOrganizationInspection => {
            "workspace.portable-organization-inspection"
        }
        ApiCapabilityId::PortableWorkspaceChangeRoots => "workspace.portable-change-roots",
        ApiCapabilityId::PortableWorkspaceImpacts => "workspace.portable-impacts",
        ApiCapabilityId::PortableWorkspaceDirectDependents => {
            "workspace.portable-direct-dependents"
        }
        ApiCapabilityId::PortableWorkspaceContributingRoots => {
            "workspace.portable-contributing-roots"
        }
        ApiCapabilityId::PortableWorkspaceExplanation => "workspace.portable-explanation",
        ApiCapabilityId::EffectiveProfileCompilation => "workspace.effective-profile-compilation",
        ApiCapabilityId::EffectiveProfileInspection => "workspace.effective-profile-inspection",
        ApiCapabilityId::EffectiveProfileExplanation => "workspace.effective-profile-explanation",
        ApiCapabilityId::EffectiveProfileAuthoringInspection => {
            "workspace.effective-profile-authoring-inspection"
        }
        ApiCapabilityId::EffectiveProfileAuthoringNavigation => {
            "workspace.effective-profile-authoring-navigation"
        }
        ApiCapabilityId::EffectiveProfileAuthoringOccurrences => {
            "workspace.effective-profile-authoring-occurrences"
        }
        ApiCapabilityId::EffectiveProfileAuthoringJoin => {
            "workspace.effective-profile-authoring-join"
        }
        ApiCapabilityId::EffectiveProfileAuthoringPages => {
            "workspace.effective-profile-authoring-pages"
        }
        ApiCapabilityId::ConfigurationImprovement => "workspace.configuration-improvement",
        ApiCapabilityId::ObservationSnapshot => "observation.snapshot",
        ApiCapabilityId::ObservationPage => "observation.page",
        ApiCapabilityId::ObservationProfileInspection => "observation.profile-inspection",
        ApiCapabilityId::ObservationDelta => "observation.delta",
        ApiCapabilityId::ObservationCausalPath => "observation.causal-path",
        ApiCapabilityId::ObservationVisualLens => "observation.visual-lens",
        ApiCapabilityId::NativeActionCatalog => "native-action.catalog",
        ApiCapabilityId::NativeActionInvocation => "native-action.invocation",
        ApiCapabilityId::NativeActionPayloadInvocation => "native-action.payload-invocation",
        ApiCapabilityId::RepositoryNativeActionAdmission => "native-action.repository-admission",
        ApiCapabilityId::ResidentHookAdmission => "resident-hook.admission",
        ApiCapabilityId::CodexCommandObservation => "resident-hook.codex-command-observation",
        ApiCapabilityId::ResidentOperationObservation => "resident.operation-observation",
        ApiCapabilityId::ResidentInvokeStart => "resident.invoke-start",
        ApiCapabilityId::ResidentInvokeAdvance => "resident.invoke-advance",
        ApiCapabilityId::ResidentConsultStart => "resident.consult-start",
        ApiCapabilityId::ResidentConsultAdvance => "resident.consult-advance",
        ApiCapabilityId::ResidentDelegateStart => "resident.delegate-start",
        ApiCapabilityId::ResidentDelegateAdvance => "resident.delegate-advance",
        ApiCapabilityId::ResidentOperationCancellation => "resident.operation-cancellation",
        ApiCapabilityId::OneShotCompilation => "one-shot.compilation",
        ApiCapabilityId::OneShotCompilationSummary => "one-shot.compilation-summary",
        ApiCapabilityId::OneShotObservationTargets => "one-shot.observation-targets",
        ApiCapabilityId::OneShotPlanning => "one-shot.planning",
        ApiCapabilityId::OneShotMachineEffectBinding => "one-shot.machine-effect-binding",
        ApiCapabilityId::OneShotSavedPlanAdmission => "one-shot.saved-plan-admission",
        ApiCapabilityId::LocalWorkspaceSourceInspection => "local.workspace-source-inspection",
        ApiCapabilityId::LocalDoctorInspection => "local.doctor-inspection",
        ApiCapabilityId::LocalOneShotCompilation => "local.one-shot-compilation",
        ApiCapabilityId::LocalPackageDistributionInspection => {
            "local.package-distribution-inspection"
        }
        ApiCapabilityId::LocalNativeActionInputCapture => "local.native-action-input-capture",
        ApiCapabilityId::LocalRepositoryStatus => "local.repository-status",
        ApiCapabilityId::LocalOneShotPlanning => "local.one-shot-planning",
        ApiCapabilityId::LocalOneShotPlanPublication => "local.one-shot-plan-publication",
        ApiCapabilityId::LocalMaterializationRunAdmission => "local.materialization-run-admission",
        ApiCapabilityId::LocalMaterializationRunSettlement => {
            "local.materialization-run-settlement"
        }
        ApiCapabilityId::LocalMaterializationReconciliationInspection => {
            "local.materialization-reconciliation-inspection"
        }
        ApiCapabilityId::CompiledInterfaceIdentity => "local.compiled-interface-identity",
        ApiCapabilityId::CurrentExecutableBindingVerification => {
            "local.current-executable-binding-verification"
        }
        ApiCapabilityId::LocalControlStorageInspection => "local.control-storage-inspection",
        ApiCapabilityId::LocalConfigurationImprovementCapture => {
            "local.configuration-improvement-capture"
        }
        ApiCapabilityId::LocalConfigurationImprovementPreview => {
            "local.configuration-improvement-preview"
        }
        ApiCapabilityId::LocalSourceGuidancePlanning => "local.source-guidance-planning",
        ApiCapabilityId::LocalSourceGuidanceWrite => "local.source-guidance-write",
        ApiCapabilityId::LocalSourceGuidanceRecoveryPlanning => {
            "local.source-guidance-recovery-planning"
        }
        ApiCapabilityId::LocalSourceGuidanceRecoveryExecution => {
            "local.source-guidance-recovery-execution"
        }
    }
}

/// Keep the journey-specific leaf matrix independent from its implementation.
const fn expected_leaves(id: ApiCapabilityId) -> &'static [ApiLeaf] {
    use ApiCapabilityId::{
        A2aDelegateHandleInspection, AuthoringIngressLimits, CompiledInterfaceIdentity,
        ConfigurationImprovement, ContractBundleIdentity, ContractDocumentValidation,
        ContractSchemaLookup, CurrentExecutableBindingVerification, DiagnosticAdmission,
        DiagnosticProjection, EffectiveProfileAuthoringInspection, EffectiveProfileAuthoringJoin,
        EffectiveProfileAuthoringNavigation, EffectiveProfileCompilation,
        EffectiveProfileExplanation, EffectiveProfileInspection,
        LocalConfigurationImprovementCapture, LocalConfigurationImprovementPreview,
        LocalControlStorageInspection, LocalMaterializationReconciliationInspection,
        LocalMaterializationRunAdmission, LocalMaterializationRunSettlement,
        LocalNativeActionInputCapture, LocalOneShotCompilation, LocalOneShotPlanPublication,
        LocalOneShotPlanning, LocalPackageDistributionInspection, LocalRepositoryStatus,
        NativeActionCatalog, NativeActionInvocation, NativeActionPayloadInvocation,
        ObservationCausalPath, ObservationDelta, ObservationPage, ObservationSnapshot,
        ObservationVisualLens, OneShotCompilation, OneShotCompilationSummary,
        OneShotMachineEffectBinding, OneShotObservationTargets, OneShotPlanning,
        OneShotSavedPlanAdmission, PackageDistributionInspection,
        PortableWorkspaceAuthoringInspection, PortableWorkspaceAuthoringNavigation,
        PortableWorkspaceChangeRoots, PortableWorkspaceComparison, PortableWorkspaceCompilation,
        PortableWorkspaceExplanation, PortableWorkspaceImpacts, RepositoryNativeActionAdmission,
        ResidentConsultAdvance, ResidentConsultStart, ResidentDelegateAdvance,
        ResidentDelegateStart, ResidentHookAdmission, ResidentInvokeAdvance, ResidentInvokeStart,
        ResidentOperationCancellation, ResidentOperationObservation, StandaloneAuthoringInspection,
        StandaloneAuthoringValidation,
    };
    use ApiLeaf::{A2a, Acp, Cli, Go, Lsp, Mcp, Node, Python, Rust, TypeScript, Wasm};

    match id {
        A2aDelegateHandleInspection => &[Rust, A2a],
        ApiCapabilityId::ObservationProfileInspection => &[Rust, Node, Go, Python],
        ApiCapabilityId::PortableWorkspaceAuthoringOccurrences => &[Rust, Node, Go, Python, Wasm],
        ApiCapabilityId::EffectiveProfileAuthoringOccurrences
        | ApiCapabilityId::EffectiveProfileAuthoringPages => &[Rust, Node, Go, Python],
        ApiCapabilityId::PortableWorkspaceSubjectComparison => &[Rust, Node, Go, Python, Wasm],
        ApiCapabilityId::PortableWorkspaceDirectDependencies
        | ApiCapabilityId::PortableWorkspaceComparisonSubjects => &[Rust, Node, Python],
        ApiCapabilityId::PortableWorkspaceSourceSubjects
        | ApiCapabilityId::PortableWorkspaceDirectDependents
        | ApiCapabilityId::PortableWorkspaceContributingRoots
        | ApiCapabilityId::PortableWorkspaceSubjectInspection
        | ApiCapabilityId::PortableWorkspaceOrganizationInspection => &[Rust, Node],
        ContractBundleIdentity | ContractSchemaLookup | ContractDocumentValidation => {
            &[Rust, Cli, Mcp, TypeScript, Node, Go, Python, Wasm]
        }
        DiagnosticProjection => &[Rust, Cli, Acp, Node, Go, Python, Wasm],
        DiagnosticAdmission => &[Rust, Cli, Node, Go, Python, Wasm],
        PackageDistributionInspection => &[Rust, Cli, Lsp, Node, Go, Python],
        ApiCapabilityId::LocalWorkspaceSourceInspection
        | ApiCapabilityId::LocalDoctorInspection
        | ApiCapabilityId::PackageCatalogInspection
        | ApiCapabilityId::LocalPackageCatalogInspection
        | ApiCapabilityId::LocalPackageIndexUpdate => &[Rust, Cli],
        AuthoringIngressLimits => &[Rust, Cli, Acp, Node, Go, Python, Wasm],
        StandaloneAuthoringValidation => &[Rust, Cli, Lsp, Acp, Node, Go, Python, Wasm],
        StandaloneAuthoringInspection
        | PortableWorkspaceAuthoringInspection
        | PortableWorkspaceAuthoringNavigation => &[Rust, Cli, Lsp, Acp, Node, Go, Python, Wasm],
        PortableWorkspaceCompilation
        | PortableWorkspaceComparison
        | PortableWorkspaceChangeRoots
        | PortableWorkspaceImpacts
        | PortableWorkspaceExplanation => &[Rust, Cli, Mcp, Lsp, Acp, Node, Go, Python, Wasm],
        EffectiveProfileCompilation | EffectiveProfileInspection | EffectiveProfileExplanation => {
            &[Rust, Cli, Mcp, Lsp, Acp, Node, Go, Python]
        }
        EffectiveProfileAuthoringInspection
        | EffectiveProfileAuthoringNavigation
        | EffectiveProfileAuthoringJoin => &[Rust, Cli, Lsp, Acp, Node, Go, Python],
        ConfigurationImprovement => &[Rust, Cli, Mcp, Lsp, Acp, Node, Go, Python],
        LocalConfigurationImprovementCapture => &[Rust, Cli],
        ApiCapabilityId::LocalSourceGuidancePlanning => &[Rust, Cli],
        ApiCapabilityId::LocalSourceGuidanceWrite
        | ApiCapabilityId::LocalSourceGuidanceRecoveryPlanning
        | ApiCapabilityId::LocalSourceGuidanceRecoveryExecution => &[Rust, Cli],
        LocalConfigurationImprovementPreview => &[Rust, Node, Go, Python],
        ObservationSnapshot
        | ObservationPage
        | ObservationDelta
        | ObservationCausalPath
        | ObservationVisualLens => &[Rust, Cli, Node, Go, Python],
        NativeActionCatalog | NativeActionPayloadInvocation => {
            &[Rust, Cli, Mcp, Node, Go, Python, Wasm]
        }
        NativeActionInvocation => &[Rust, Cli, Node, Go, Python, Wasm],
        RepositoryNativeActionAdmission => &[Rust, Cli, Node, Go, Python],
        ResidentHookAdmission => &[Rust, Cli, Node, Go, Python],
        ApiCapabilityId::CodexCommandObservation => &[Rust],
        ResidentOperationObservation
        | ResidentDelegateStart
        | ResidentDelegateAdvance
        | ResidentOperationCancellation => &[Rust, Cli, Mcp, A2a],
        ResidentInvokeStart
        | ResidentInvokeAdvance
        | ResidentConsultStart
        | ResidentConsultAdvance => &[Rust, Cli, Mcp],
        LocalPackageDistributionInspection
        | LocalNativeActionInputCapture
        | LocalRepositoryStatus
        | LocalOneShotPlanning
        | LocalOneShotPlanPublication
        | LocalMaterializationRunAdmission
        | LocalMaterializationRunSettlement
        | LocalMaterializationReconciliationInspection
        | LocalOneShotCompilation
        | CompiledInterfaceIdentity
        | CurrentExecutableBindingVerification
        | LocalControlStorageInspection => &[Rust, Cli, Node, Go, Python],
        OneShotCompilation
        | OneShotCompilationSummary
        | OneShotObservationTargets
        | OneShotPlanning => &[Rust, Cli, Mcp, Node, Go, Python],
        OneShotMachineEffectBinding | OneShotSavedPlanAdmission => &[Rust, Cli, Node, Go, Python],
    }
}

#[test]
fn inventory_is_complete_unique_and_stably_ordered() {
    let inventory = capability_inventory();
    let expected = vec![
        "a2a.delegate-handle-inspection",
        "contract.bundle-identity",
        "contract.schema-lookup",
        "contract.document-validation",
        "diagnostic.projection",
        "diagnostic.admission",
        "package-distribution.inspection",
        "package-catalog.inspection",
        "authoring.ingress-limits",
        "authoring.standalone-validation",
        "authoring.standalone-inspection",
        "workspace.portable-compilation",
        "workspace.portable-authoring-inspection",
        "workspace.portable-authoring-navigation",
        "workspace.portable-authoring-occurrences",
        "workspace.portable-source-subjects",
        "workspace.portable-comparison",
        "workspace.portable-comparison-subjects",
        "workspace.portable-subject-comparison",
        "workspace.portable-direct-dependencies",
        "workspace.portable-direct-dependents",
        "workspace.portable-subject-inspection",
        "workspace.portable-organization-inspection",
        "workspace.portable-change-roots",
        "workspace.portable-impacts",
        "workspace.portable-contributing-roots",
        "workspace.portable-explanation",
        "workspace.effective-profile-compilation",
        "workspace.effective-profile-inspection",
        "workspace.effective-profile-explanation",
        "workspace.effective-profile-authoring-inspection",
        "workspace.effective-profile-authoring-navigation",
        "workspace.effective-profile-authoring-occurrences",
        "workspace.effective-profile-authoring-join",
        "workspace.effective-profile-authoring-pages",
        "workspace.configuration-improvement",
        "observation.snapshot",
        "observation.page",
        "observation.profile-inspection",
        "observation.delta",
        "observation.causal-path",
        "observation.visual-lens",
        "native-action.catalog",
        "native-action.invocation",
        "native-action.payload-invocation",
        "native-action.repository-admission",
        "resident-hook.admission",
        "resident-hook.codex-command-observation",
        "resident.operation-observation",
        "resident.invoke-start",
        "resident.invoke-advance",
        "resident.consult-start",
        "resident.consult-advance",
        "resident.delegate-start",
        "resident.delegate-advance",
        "resident.operation-cancellation",
        "one-shot.compilation",
        "one-shot.compilation-summary",
        "one-shot.observation-targets",
        "one-shot.planning",
        "one-shot.machine-effect-binding",
        "one-shot.saved-plan-admission",
    ];
    #[cfg(feature = "native-local")]
    let expected = expected
        .into_iter()
        .chain([
            "local.workspace-source-inspection",
            "local.doctor-inspection",
            "local.one-shot-compilation",
            "local.package-distribution-inspection",
            "local.package-catalog-inspection",
            "local.package-index-update",
            "local.native-action-input-capture",
            "local.repository-status",
            "local.one-shot-planning",
            "local.one-shot-plan-publication",
            "local.materialization-run-admission",
            "local.materialization-run-settlement",
            "local.materialization-reconciliation-inspection",
            "local.compiled-interface-identity",
            "local.current-executable-binding-verification",
            "local.control-storage-inspection",
            "local.configuration-improvement-capture",
            "local.configuration-improvement-preview",
            "local.source-guidance-planning",
            "local.source-guidance-write",
            "local.source-guidance-recovery-planning",
            "local.source-guidance-recovery-execution",
        ])
        .collect::<Vec<_>>();
    let actual = inventory
        .iter()
        .map(|capability| {
            assert_eq!(capability.id().as_str(), stable_id(capability.id()));
            capability.id().as_str()
        })
        .collect::<Vec<_>>();
    let unique = actual.iter().copied().collect::<BTreeSet<_>>();

    assert_eq!(actual, expected);
    assert_eq!(unique.len(), inventory.len());
}

#[test]
fn every_row_has_truthful_contract_and_current_execution_semantics() {
    for capability in capability_inventory() {
        assert_eq!(capability.invocation(), ApiInvocationMode::Immediate);
        let expected_cancellation = match capability.id() {
            ApiCapabilityId::ObservationDelta | ApiCapabilityId::ObservationVisualLens => {
                ApiCancellationSemantics::CooperativePreflight
            }
            _ => ApiCancellationSemantics::Unavailable,
        };
        assert_eq!(capability.cancellation(), expected_cancellation);
        assert!(capability.output().is_some());
        for contract in [capability.input(), capability.output()]
            .into_iter()
            .flatten()
        {
            assert!(!contract.identity().is_empty());
            if let ApiCapabilityContract::PublicSchema(schema_id) = contract {
                assert!(matches!(
                    schema_id,
                    "action-execution.v1"
                        | "action-input.v1"
                        | "diagnostic.v1"
                        | "repository-native-action-catalog.v1"
                ));
            }
        }
    }
}

#[test]
fn leaf_eligibility_is_unique_bounded_and_transport_coherent() {
    let inventory = capability_inventory();
    for capability in inventory {
        let leaves = capability.eligible_leaves();
        assert!(!leaves.is_empty());
        assert_eq!(leaves, expected_leaves(capability.id()));
        assert_eq!(
            leaves.iter().copied().collect::<BTreeSet<_>>().len(),
            leaves.len()
        );
        assert!(leaves.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(capability.eligible_for(ApiLeaf::Rust));

        if capability.eligible_for(ApiLeaf::Mcp) {
            assert!(capability.eligible_for(ApiLeaf::Cli));
        }
        if capability.eligible_for(ApiLeaf::TypeScript) {
            assert!(matches!(
                capability.id(),
                ApiCapabilityId::ContractBundleIdentity
                    | ApiCapabilityId::ContractSchemaLookup
                    | ApiCapabilityId::ContractDocumentValidation
            ));
        }
        if capability.eligible_for(ApiLeaf::Lsp) {
            assert!(matches!(
                capability.class(),
                ApiCapabilityClass::Portable | ApiCapabilityClass::Native
            ));
            assert_eq!(capability.effect(), ApiCapabilityEffect::EffectFree);
        }
        if capability.eligible_for(ApiLeaf::Wasm) {
            assert_eq!(capability.class(), ApiCapabilityClass::Portable);
            assert_eq!(capability.effect(), ApiCapabilityEffect::EffectFree);
        }
        if capability.class() == ApiCapabilityClass::Native {
            assert!(!capability.eligible_for(ApiLeaf::TypeScript));
            if capability.eligible_for(ApiLeaf::Lsp) {
                assert!(matches!(
                    capability.id(),
                    ApiCapabilityId::EffectiveProfileCompilation
                        | ApiCapabilityId::EffectiveProfileInspection
                        | ApiCapabilityId::EffectiveProfileExplanation
                        | ApiCapabilityId::EffectiveProfileAuthoringInspection
                        | ApiCapabilityId::EffectiveProfileAuthoringNavigation
                        | ApiCapabilityId::EffectiveProfileAuthoringJoin
                        | ApiCapabilityId::ConfigurationImprovement
                ));
            }
            assert!(!capability.eligible_for(ApiLeaf::Wasm));
        }
    }

    assert_eq!(
        inventory
            .iter()
            .filter(|capability| capability.class() == ApiCapabilityClass::Resident)
            .map(|capability| capability.id())
            .collect::<Vec<_>>(),
        [
            ApiCapabilityId::ResidentHookAdmission,
            ApiCapabilityId::CodexCommandObservation,
            ApiCapabilityId::ResidentOperationObservation,
            ApiCapabilityId::ResidentInvokeStart,
            ApiCapabilityId::ResidentInvokeAdvance,
            ApiCapabilityId::ResidentConsultStart,
            ApiCapabilityId::ResidentConsultAdvance,
            ApiCapabilityId::ResidentDelegateStart,
            ApiCapabilityId::ResidentDelegateAdvance,
            ApiCapabilityId::ResidentOperationCancellation,
        ]
    );
    assert_eq!(
        inventory
            .iter()
            .filter(|capability| capability.eligible_for(ApiLeaf::TypeScript))
            .count(),
        3
    );
    assert_eq!(
        inventory
            .iter()
            .filter(|capability| capability.eligible_for(ApiLeaf::A2a))
            .map(|capability| capability.id())
            .collect::<Vec<_>>(),
        [
            ApiCapabilityId::A2aDelegateHandleInspection,
            ApiCapabilityId::ResidentOperationObservation,
            ApiCapabilityId::ResidentDelegateStart,
            ApiCapabilityId::ResidentDelegateAdvance,
            ApiCapabilityId::ResidentOperationCancellation,
        ],
        "A2A exposes structural inspection and the resident-owned Delegate journey only"
    );
    assert_eq!(
        inventory
            .iter()
            .filter(|capability| capability.eligible_for(ApiLeaf::Acp))
            .map(|capability| capability.id())
            .collect::<Vec<_>>(),
        [
            ApiCapabilityId::DiagnosticProjection,
            ApiCapabilityId::AuthoringIngressLimits,
            ApiCapabilityId::StandaloneAuthoringValidation,
            ApiCapabilityId::StandaloneAuthoringInspection,
            ApiCapabilityId::PortableWorkspaceCompilation,
            ApiCapabilityId::PortableWorkspaceAuthoringInspection,
            ApiCapabilityId::PortableWorkspaceAuthoringNavigation,
            ApiCapabilityId::PortableWorkspaceComparison,
            ApiCapabilityId::PortableWorkspaceChangeRoots,
            ApiCapabilityId::PortableWorkspaceImpacts,
            ApiCapabilityId::PortableWorkspaceExplanation,
            ApiCapabilityId::EffectiveProfileCompilation,
            ApiCapabilityId::EffectiveProfileInspection,
            ApiCapabilityId::EffectiveProfileExplanation,
            ApiCapabilityId::EffectiveProfileAuthoringInspection,
            ApiCapabilityId::EffectiveProfileAuthoringNavigation,
            ApiCapabilityId::EffectiveProfileAuthoringJoin,
            ApiCapabilityId::ConfigurationImprovement,
        ],
        "ACP eligibility must equal its implemented effect-free façade subset"
    );
    let mcp_capabilities = inventory
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Mcp))
        .map(|capability| capability.id())
        .collect::<Vec<_>>();
    assert_eq!(
        mcp_capabilities,
        [
            ApiCapabilityId::ContractBundleIdentity,
            ApiCapabilityId::ContractSchemaLookup,
            ApiCapabilityId::ContractDocumentValidation,
            ApiCapabilityId::PortableWorkspaceCompilation,
            ApiCapabilityId::PortableWorkspaceComparison,
            ApiCapabilityId::PortableWorkspaceChangeRoots,
            ApiCapabilityId::PortableWorkspaceImpacts,
            ApiCapabilityId::PortableWorkspaceExplanation,
            ApiCapabilityId::EffectiveProfileCompilation,
            ApiCapabilityId::EffectiveProfileInspection,
            ApiCapabilityId::EffectiveProfileExplanation,
            ApiCapabilityId::ConfigurationImprovement,
            ApiCapabilityId::NativeActionCatalog,
            ApiCapabilityId::NativeActionPayloadInvocation,
            ApiCapabilityId::ResidentOperationObservation,
            ApiCapabilityId::ResidentInvokeStart,
            ApiCapabilityId::ResidentInvokeAdvance,
            ApiCapabilityId::ResidentConsultStart,
            ApiCapabilityId::ResidentConsultAdvance,
            ApiCapabilityId::ResidentDelegateStart,
            ApiCapabilityId::ResidentDelegateAdvance,
            ApiCapabilityId::ResidentOperationCancellation,
            ApiCapabilityId::OneShotCompilation,
            ApiCapabilityId::OneShotCompilationSummary,
            ApiCapabilityId::OneShotObservationTargets,
            ApiCapabilityId::OneShotPlanning,
        ]
    );
    assert_eq!(
        inventory
            .iter()
            .filter(|capability| capability.eligible_for(ApiLeaf::Mcp))
            .count(),
        26
    );

    let lsp_capabilities = inventory
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Lsp))
        .map(|capability| capability.id())
        .collect::<Vec<_>>();
    assert_eq!(
        lsp_capabilities,
        [
            ApiCapabilityId::PackageDistributionInspection,
            ApiCapabilityId::StandaloneAuthoringValidation,
            ApiCapabilityId::StandaloneAuthoringInspection,
            ApiCapabilityId::PortableWorkspaceCompilation,
            ApiCapabilityId::PortableWorkspaceAuthoringInspection,
            ApiCapabilityId::PortableWorkspaceAuthoringNavigation,
            ApiCapabilityId::PortableWorkspaceComparison,
            ApiCapabilityId::PortableWorkspaceChangeRoots,
            ApiCapabilityId::PortableWorkspaceImpacts,
            ApiCapabilityId::PortableWorkspaceExplanation,
            ApiCapabilityId::EffectiveProfileCompilation,
            ApiCapabilityId::EffectiveProfileInspection,
            ApiCapabilityId::EffectiveProfileExplanation,
            ApiCapabilityId::EffectiveProfileAuthoringInspection,
            ApiCapabilityId::EffectiveProfileAuthoringNavigation,
            ApiCapabilityId::EffectiveProfileAuthoringJoin,
            ApiCapabilityId::ConfigurationImprovement,
        ]
    );

    assert!(
        inventory
            .iter()
            .all(|capability| capability.eligible_for(ApiLeaf::Rust)),
        "every currently registered capability must be callable through the Rust façade"
    );
    for leaf in [ApiLeaf::Cli, ApiLeaf::Node, ApiLeaf::Go, ApiLeaf::Python] {
        let unavailable: &[ApiCapabilityId] = match leaf {
            ApiLeaf::Node => &[
                ApiCapabilityId::A2aDelegateHandleInspection,
                ApiCapabilityId::PackageCatalogInspection,
                ApiCapabilityId::CodexCommandObservation,
                ApiCapabilityId::ResidentOperationObservation,
                ApiCapabilityId::ResidentInvokeStart,
                ApiCapabilityId::ResidentInvokeAdvance,
                ApiCapabilityId::ResidentConsultStart,
                ApiCapabilityId::ResidentConsultAdvance,
                ApiCapabilityId::ResidentDelegateStart,
                ApiCapabilityId::ResidentDelegateAdvance,
                ApiCapabilityId::ResidentOperationCancellation,
            ],
            ApiLeaf::Python => &[
                ApiCapabilityId::A2aDelegateHandleInspection,
                ApiCapabilityId::PackageCatalogInspection,
                ApiCapabilityId::PortableWorkspaceSourceSubjects,
                ApiCapabilityId::PortableWorkspaceDirectDependents,
                ApiCapabilityId::PortableWorkspaceSubjectInspection,
                ApiCapabilityId::PortableWorkspaceOrganizationInspection,
                ApiCapabilityId::PortableWorkspaceContributingRoots,
                ApiCapabilityId::CodexCommandObservation,
                ApiCapabilityId::ResidentOperationObservation,
                ApiCapabilityId::ResidentInvokeStart,
                ApiCapabilityId::ResidentInvokeAdvance,
                ApiCapabilityId::ResidentConsultStart,
                ApiCapabilityId::ResidentConsultAdvance,
                ApiCapabilityId::ResidentDelegateStart,
                ApiCapabilityId::ResidentDelegateAdvance,
                ApiCapabilityId::ResidentOperationCancellation,
            ],
            ApiLeaf::Go => &[
                ApiCapabilityId::A2aDelegateHandleInspection,
                ApiCapabilityId::PackageCatalogInspection,
                ApiCapabilityId::PortableWorkspaceSourceSubjects,
                ApiCapabilityId::PortableWorkspaceComparisonSubjects,
                ApiCapabilityId::PortableWorkspaceDirectDependencies,
                ApiCapabilityId::PortableWorkspaceDirectDependents,
                ApiCapabilityId::PortableWorkspaceSubjectInspection,
                ApiCapabilityId::PortableWorkspaceOrganizationInspection,
                ApiCapabilityId::PortableWorkspaceContributingRoots,
                ApiCapabilityId::CodexCommandObservation,
                ApiCapabilityId::ResidentOperationObservation,
                ApiCapabilityId::ResidentInvokeStart,
                ApiCapabilityId::ResidentInvokeAdvance,
                ApiCapabilityId::ResidentConsultStart,
                ApiCapabilityId::ResidentConsultAdvance,
                ApiCapabilityId::ResidentDelegateStart,
                ApiCapabilityId::ResidentDelegateAdvance,
                ApiCapabilityId::ResidentOperationCancellation,
            ],
            _ => &[
                ApiCapabilityId::A2aDelegateHandleInspection,
                ApiCapabilityId::PortableWorkspaceAuthoringOccurrences,
                ApiCapabilityId::PortableWorkspaceSourceSubjects,
                ApiCapabilityId::PortableWorkspaceComparisonSubjects,
                ApiCapabilityId::PortableWorkspaceSubjectComparison,
                ApiCapabilityId::PortableWorkspaceDirectDependencies,
                ApiCapabilityId::PortableWorkspaceDirectDependents,
                ApiCapabilityId::PortableWorkspaceSubjectInspection,
                ApiCapabilityId::PortableWorkspaceOrganizationInspection,
                ApiCapabilityId::PortableWorkspaceContributingRoots,
                ApiCapabilityId::EffectiveProfileAuthoringOccurrences,
                ApiCapabilityId::EffectiveProfileAuthoringPages,
                ApiCapabilityId::ObservationProfileInspection,
                ApiCapabilityId::CodexCommandObservation,
            ],
        };
        let native_unavailable: &[ApiCapabilityId] = match leaf {
            #[cfg(feature = "native-local")]
            ApiLeaf::Node | ApiLeaf::Go | ApiLeaf::Python => &[
                ApiCapabilityId::LocalWorkspaceSourceInspection,
                ApiCapabilityId::LocalDoctorInspection,
                ApiCapabilityId::LocalPackageCatalogInspection,
                ApiCapabilityId::LocalPackageIndexUpdate,
                ApiCapabilityId::LocalConfigurationImprovementCapture,
                ApiCapabilityId::LocalSourceGuidancePlanning,
                ApiCapabilityId::LocalSourceGuidanceWrite,
                ApiCapabilityId::LocalSourceGuidanceRecoveryPlanning,
                ApiCapabilityId::LocalSourceGuidanceRecoveryExecution,
            ],
            #[cfg(feature = "native-local")]
            ApiLeaf::Cli => &[ApiCapabilityId::LocalConfigurationImprovementPreview],
            _ => &[],
        };
        assert_eq!(
            inventory
                .iter()
                .filter(|capability| !capability.eligible_for(leaf))
                .map(|capability| capability.id())
                .collect::<Vec<_>>(),
            unavailable
                .iter()
                .copied()
                .chain(native_unavailable.iter().copied())
                .collect::<Vec<_>>(),
            "{leaf:?} must not claim an operation before a complete thin journey exists"
        );
    }
}

#[test]
fn mcp_actions_preserve_payload_and_repository_admission_boundaries() {
    for id in [
        ApiCapabilityId::NativeActionCatalog,
        ApiCapabilityId::NativeActionPayloadInvocation,
    ] {
        let capability = capability_inventory()
            .iter()
            .find(|capability| capability.id() == id)
            .expect("native-action capability is inventoried");
        assert!(capability.eligible_for(ApiLeaf::Cli));
        assert!(capability.eligible_for(ApiLeaf::Mcp));
    }
    for id in [
        ApiCapabilityId::NativeActionInvocation,
        ApiCapabilityId::RepositoryNativeActionAdmission,
    ] {
        let capability = capability_inventory()
            .iter()
            .find(|capability| capability.id() == id)
            .expect("native-action capability is inventoried");
        assert!(capability.eligible_for(ApiLeaf::Cli));
        assert!(!capability.eligible_for(ApiLeaf::Mcp));
    }
}

#[test]
fn planning_and_saved_admission_preserve_distinct_consequence_classes() {
    let planning = capability_inventory()
        .iter()
        .filter(|capability| capability.effect() == ApiCapabilityEffect::EffectFreePlanning)
        .map(|capability| capability.id())
        .collect::<Vec<_>>();

    let expected = vec![
        ApiCapabilityId::OneShotPlanning,
        ApiCapabilityId::OneShotMachineEffectBinding,
    ];
    #[cfg(feature = "native-local")]
    let expected = expected
        .into_iter()
        .chain([
            ApiCapabilityId::LocalOneShotPlanning,
            ApiCapabilityId::LocalSourceGuidancePlanning,
            ApiCapabilityId::LocalSourceGuidanceRecoveryPlanning,
        ])
        .collect::<Vec<_>>();
    assert_eq!(planning, expected);
    assert_eq!(
        capability_inventory()
            .iter()
            .filter(|capability| {
                capability.effect() == ApiCapabilityEffect::EffectFreeAdmission
            })
            .map(|capability| capability.id())
            .collect::<Vec<_>>(),
        {
            let expected = vec![ApiCapabilityId::OneShotSavedPlanAdmission];
            #[cfg(feature = "native-local")]
            let expected = expected
                .into_iter()
                .chain([ApiCapabilityId::LocalMaterializationRunAdmission])
                .collect::<Vec<_>>();
            expected
        }
    );

    #[cfg(feature = "native-local")]
    assert_eq!(
        capability_inventory()
            .iter()
            .filter(|capability| capability.effect() == ApiCapabilityEffect::NativeEffect)
            .map(|capability| capability.id())
            .collect::<Vec<_>>(),
        [
            ApiCapabilityId::LocalPackageIndexUpdate,
            ApiCapabilityId::LocalOneShotPlanPublication,
            ApiCapabilityId::LocalMaterializationRunSettlement,
            ApiCapabilityId::LocalSourceGuidanceWrite,
            ApiCapabilityId::LocalSourceGuidanceRecoveryExecution,
        ]
    );
}
