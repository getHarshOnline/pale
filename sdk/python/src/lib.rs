//! Native Python projection of [`pale_sdk`].
//!
//! This crate is the `_native` module of the Python package;
//! `pale-python-stubgen` renders its type declarations.
//!
//! Python values are converted to owned Rust batches before Pale application
//! work starts.
//! Every bounded Rust-only call detaches from the interpreter, and all admitted
//! failures cross one [`PaleDiagnosticError`] boundary. Its message is the
//! canonical summary and its `diagnostic` attribute carries the exact record.

mod authoring;
mod control_storage;
mod distribution;
mod effective_profile;
mod effective_profile_entries;
mod effective_profile_entry_values;
mod effective_profile_explanation;
mod effective_profile_values;
mod error;
mod improvement;
mod local;
mod observation;
mod one_shot;
mod portable;
mod profile_dispositions;
mod repository_actions;
mod resident_hook;
mod workspace_subject_value;

use std::sync::Arc;

use pale_sdk::{capability_inventory, ApiLeaf, PaleApi};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

pub use authoring::{
    PyAuthoringCorrection, PyAuthoringCorrectionKind, PyAuthoringFieldFact,
    PyAuthoringFieldPresence, PyAuthoringIdentity, PyAuthoringIdentityKind,
    PyAuthoringIngressClass, PyAuthoringIngressLimits, PyAuthoringInspection,
    PyAuthoringInspectionState, PyAuthoringInstancePathSegment, PyAuthoringLiteral,
    PyAuthoringLiteralFact, PyAuthoringLiteralKind, PyAuthoringNavigation, PyAuthoringNodeFact,
    PyAuthoringOccurrence, PyAuthoringOccurrencePage, PyAuthoringOccurrencePageCursor,
    PyAuthoringOccurrenceRole, PyAuthoringPathSegment, PyAuthoringSourceLocation,
    PyAuthoringValueKind, PyStandaloneAuthoringRole,
};
pub use control_storage::{
    PyLocalControlStorageInspection, PyLocalControlStorageInspectionRejection,
    PyLocalControlStorageInspectionRejectionKind, PyLocalControlStorageInspectionRequest,
    PyLocalControlStorageInspectionState, PyLocalControlStorageInspectionStateKind,
    PyLocalControlStorageProfileRequirement, PyLocalControlStoreKind,
};
pub use distribution::{PyPackageDistributionInspection, PyPackageReleaseIdentity};
pub use effective_profile::{
    PyEffectiveAuthoringContinuation, PyEffectiveAuthoringInspection, PyEffectiveAuthoringPage,
    PyEffectiveProfileCompilation, PyEffectiveProfileCompilationRequest,
    PyEffectiveProfileCoordinates, PyEffectiveProfilePage, PyEffectiveProfileSection,
};
pub use effective_profile_entries::{
    PyEffectiveProfileCapabilityDetail, PyEffectiveProfileCapabilityNarrowingDetail,
    PyEffectiveProfileDeclaredSelectionDetail, PyEffectiveProfileEntry,
    PyEffectiveProfileExecutionPathDetail, PyEffectiveProfileFindingDetail,
    PyEffectiveProfileGoverningCeilingDetail, PyEffectiveProfileMachineRealizationDetail,
    PyEffectiveProfileOperationDetail, PyEffectiveProfileProviderRepositoryDetail,
    PyEffectiveProfileRepositoryDetail, PyEffectiveProfileResponsibilityDetail,
    PyEffectiveProfileSelectionDetail,
};
pub use effective_profile_entry_values::{
    PyEffectiveProfileApplicationParticipant, PyEffectiveProfileCapabilityParticipant,
    PyEffectiveProfileCommunicationResponsibility, PyEffectiveProfileContextMemoryResponsibility,
    PyEffectiveProfileCrmBusinessStateResponsibility, PyEffectiveProfileEffectClass,
    PyEffectiveProfileEngineParticipant, PyEffectiveProfileEngineeringExecutionResponsibility,
    PyEffectiveProfileExtensionResponsibility, PyEffectiveProfileInterfaceParticipant,
    PyEffectiveProfileNativeEffectReadBackResponsibility,
    PyEffectiveProfileNativeEventPublicationResponsibility,
    PyEffectiveProfileOrganizationalWorkResponsibility, PyEffectiveProfilePluginParticipant,
    PyEffectiveProfileProviderBindingRef, PyEffectiveProfileRepositoryAccess,
    PyEffectiveProfileSkillParticipant, PyEffectiveProfileTerminalHarnessExecutionResponsibility,
    PyEffectiveProfileTerminalLeafParticipant,
};
pub use effective_profile_explanation::{
    PyEffectiveProfileCapabilityClassificationEvidence, PyEffectiveProfileConfigurationEvidence,
    PyEffectiveProfileDeclaredSelectionEvidence, PyEffectiveProfileEffectiveSelectionEvidence,
    PyEffectiveProfileExecutionPathEvidence, PyEffectiveProfileExplanation,
    PyEffectiveProfileFindingClassificationEvidence, PyEffectiveProfileMachineRealizationEvidence,
    PyEffectiveProfileOperationClassificationEvidence, PyEffectiveProfileResponsibilityEvidence,
};
pub use effective_profile_values::{
    PyEffectiveProfileAuthorityRootSelection, PyEffectiveProfileBoundProviderDisposition,
    PyEffectiveProfileContinuityContractReason, PyEffectiveProfileDeclaredAllFromPackageSelector,
    PyEffectiveProfileDeclaredExactSelector, PyEffectiveProfileDeclaredModeDefaultSelection,
    PyEffectiveProfileDeclaredResourceSelection, PyEffectiveProfileDelegationEdgeReason,
    PyEffectiveProfileDisclosureCeilingSelection,
    PyEffectiveProfileEffectMismatchProviderDisposition, PyEffectiveProfileEntryContractReason,
    PyEffectiveProfileExcludedByNarrowingProviderDisposition, PyEffectiveProfileLifecycleSelection,
    PyEffectiveProfileMissingBindingProviderDisposition, PyEffectiveProfileModeAxis,
    PyEffectiveProfileNativeRealization, PyEffectiveProfileNativeRealizationUnknownFinding,
    PyEffectiveProfileNormativePackSelection, PyEffectiveProfileOperationUnboundFinding,
    PyEffectiveProfileOriginReceiptReason, PyEffectiveProfilePackageSelection,
    PyEffectiveProfileProviderBindingReason, PyEffectiveProfileProviderKind,
    PyEffectiveProfileRecipeContractMismatchProviderDisposition,
    PyEffectiveProfileRequirementFinding, PyEffectiveProfileResourceKind,
    PyEffectiveProfileResponsibilityContractReason, PyEffectiveProfileRunPolicySelection,
    PyEffectiveProfileSemanticResourceSelection, PyEffectiveProfileSessionSelection,
    PyEffectiveProfileSettlementContractReason, PyEffectiveProfileTerminationContractReason,
};
pub(crate) use effective_profile_values::{
    PyEffectiveProfileDeclaredSelection, PyEffectiveProfileFinding, PyEffectiveProfilePathReason,
    PyEffectiveProfileProviderDisposition, PyEffectiveProfileSelection,
};
pub use error::PaleDiagnosticError;
pub use improvement::{
    PyConfigurationImprovementCandidateRequest, PyConfigurationImprovementComparison,
    PyConfigurationImprovementCoordinateChange, PyConfigurationImprovementCoordinateChangeKind,
    PyConfigurationImprovementDiagnosis, PyConfigurationImprovementDisposition,
    PyConfigurationImprovementEvaluation, PyConfigurationImprovementExplanation,
    PyConfigurationImprovementInspection, PyConfigurationImprovementPhase,
    PyConfigurationImprovementPreview, PyConfigurationImprovementPreviewSummary,
    PyConfigurationImprovementProposal, PyConfigurationImprovementProposalRequest,
    PyConfigurationImprovementTerminal, PyPreparedConfigurationImprovementPreview,
};
pub use local::{
    PyCapturedNativeActionRequest, PyCompiledInterfaceIdentity, PyDurablePublicationMechanism,
    PyLocalConfigurationImprovementRequest, PyLocalMaterializationAction,
    PyLocalMaterializationAdmissionRequest, PyLocalMaterializationAdmittedRun,
    PyLocalMaterializationDrift, PyLocalMaterializationDriftKind,
    PyLocalMaterializationMachineEffectKind, PyLocalMaterializationOwnershipChange,
    PyLocalMaterializationOwnershipObservation, PyLocalMaterializationReceiptObservation,
    PyLocalMaterializationReconciliation, PyLocalMaterializationRunOutcome,
    PyLocalMaterializationRunResult, PyLocalMaterializationTargetAdmission,
    PyLocalMaterializationTargetOutcome, PyLocalMaterializationTargetResult,
    PyLocalNativeActionInputRequest, PyLocalOneShotCompilation, PyLocalOneShotCompilationRequest,
    PyLocalOneShotPlan, PyLocalOneShotPlanCounts, PyLocalOneShotPlanPublication,
    PyLocalOneShotPlanRequest, PyLocalPackageDistributionRequest, PyLocalRepositoryStatus,
};
pub use observation::{
    PyObservationBackpressureRecovery, PyObservationCancellation, PyObservationCategory,
    PyObservationCausalPath, PyObservationCausalPathResult, PyObservationCausalResultKind,
    PyObservationCausalStep, PyObservationCausalStepKind, PyObservationChange,
    PyObservationChangeKind, PyObservationCursor, PyObservationDeltaOutcome,
    PyObservationDeltaOutcomeKind, PyObservationFact, PyObservationFilter, PyObservationFreshness,
    PyObservationFreshnessClass, PyObservationFreshnessKind, PyObservationGeneration,
    PyObservationGenerationKind, PyObservationHealthState, PyObservationLens,
    PyObservationLensCursor, PyObservationLensDisposition, PyObservationLensDispositionKind,
    PyObservationLensLimits, PyObservationLensNode, PyObservationNativeState,
    PyObservationNativeStateKind, PyObservationPage, PyObservationPageCursor,
    PyObservationPageLimits, PyObservationPageResult, PyObservationPageResultKind,
    PyObservationProcessIntent, PyObservationProfileContinuation, PyObservationProfileError,
    PyObservationProfileMapping, PyObservationProfilePage, PyObservationProfileResult,
    PyObservationProfileResultKind, PyObservationReadinessState, PyObservationReconciliationState,
    PyObservationReconciliationStateKind, PyObservationResynchronization,
    PyObservationResynchronizationReason, PyObservationRevision, PyObservationRevisionKind,
    PyObservationSnapshot, PyObservationStepCompletionKind, PyObservationStepLimitKind,
    PyObservationSubject, PyObservationSubjectKind,
};
pub use one_shot::{
    PyBaselineEntry, PyCompilationDocument, PyMachineLocalEffectCandidate, PyOneShotAdmittedPlan,
    PyOneShotBoundPlan, PyOneShotCompilation, PyOneShotCompilationRequest,
    PyOneShotObservationRequest, PyOneShotPlanCandidate, PyOneShotPlanKind,
    PyOneShotPlanningRequest, PyOneShotPriorMaterialization, PyOneShotPrivateInputs,
    PyOneShotSummary, PyOneShotWorkspaceInputs,
};
pub use portable::{
    PyAbsentWorkspaceSubjectValues, PyContractBundleIdentity, PyContractSchema,
    PyCurrentOnlyWorkspaceSubjectValues, PyDiagnostic, PyDiagnosticDisposition,
    PyDiagnosticEffectCertainty, PyDiagnosticProjection, PyDiagnosticRetry, PyDiagnosticStage,
    PyNativeActionDescriptor, PyPortableWorkspaceCompilation, PyPortableWorkspaceDependencyCursor,
    PyPortableWorkspaceDependencyPage, PyPriorOnlyWorkspaceSubjectValues,
    PySharedWorkspaceSubjectValues, PyValidatedContractDocument, PyWorkspaceCausalStep,
    PyWorkspaceChangeKind, PyWorkspaceChangeRoot, PyWorkspaceChangeRootPage, PyWorkspaceComparison,
    PyWorkspaceComparisonSubjectCursor, PyWorkspaceComparisonSubjectPage,
    PyWorkspaceComparisonSummary, PyWorkspaceDerivationRule, PyWorkspaceExplanation,
    PyWorkspaceExplanationFrontier, PyWorkspaceExplanationState, PyWorkspaceFact,
    PyWorkspaceFactKind, PyWorkspaceImpact, PyWorkspaceImpactPage, PyWorkspaceImpactState,
    PyWorkspaceProvenance, PyWorkspaceProvenanceKind, PyWorkspaceSubject,
    PyWorkspaceSubjectComparison, PyWorkspaceSubjectKind, PyWorkspaceValueComparison,
};
pub(crate) use profile_dispositions::PyRequirementDisposition;
pub use profile_dispositions::{
    PyBlockedRequirementDisposition, PyNarrowedRequirementDisposition, PyNarrowingReason,
    PyOperationExposure, PyPolicyBlock, PyProviderBindingAbsentRequirementDisposition,
    PySatisfiedRequirementDisposition,
};
pub use repository_actions::PyAdmittedRepositoryActions;
pub use resident_hook::{PyResidentHookAdmission, PyResidentHookRequest};
pub use workspace_subject_value::{
    PyWholeWorkspaceSubjectValue, PyWorkspaceComponentSubjectValue,
    PyWorkspaceDeclarationSubjectValue, PyWorkspaceExperienceCompositionSubjectValue,
    PyWorkspaceExtensionSubjectValue, PyWorkspaceImportSubjectValue, PyWorkspaceModuleSubjectValue,
    PyWorkspaceOrganizationalTranslationClosureSubjectValue, PyWorkspacePolicySubjectValue,
    PyWorkspaceResolutionLockSubjectValue, PyWorkspaceResourceSubjectValue,
    PyWorkspaceResponsibilityClosureSubjectValue, PyWorkspaceSourceSubjectValue,
    PyWorkspaceUniverseLockSubjectValue,
};

/// Native Python handle over the complete eligible Pale façade.
#[pyclass(name = "Pale", frozen)]
pub struct PalePython {
    pub(crate) api: Arc<PaleApi>,
}

/// Copy one Rust byte slice into the Python-native `bytes` representation.
///
/// Keeping this conversion in one owner prevents individual projection
/// getters from accidentally exposing `Vec<u8>` as Python `list[int]`.
fn python_bytes(py: Python<'_>, bytes: &[u8]) -> Py<PyBytes> {
    PyBytes::new(py, bytes).unbind()
}

#[pymethods]
impl PalePython {
    /// Verify embedded contracts and construct the native façade.
    ///
    /// # Errors
    ///
    /// Returns the stable initialization code and message when the embedded
    /// contract bundle contradicts the compiled façade.
    #[new]
    pub fn new(py: Python<'_>) -> PyResult<Self> {
        py.detach(PaleApi::new)
            .map(|api| Self { api: Arc::new(api) })
            .map_err(error::initialization_error)
    }

    /// Return the API-owned Python eligibility set in stable order.
    #[getter]
    pub fn capability_ids(&self) -> Vec<String> {
        eligible_capability_ids()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }
}

/// Derive the Python surface from the one API-owned inventory.
fn eligible_capability_ids() -> Vec<&'static str> {
    capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Python))
        .map(|capability| capability.id().as_str())
        .collect()
}

/// Compile-time proof for every value allowed to cross [`Python::detach`].
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PaleApi>();
    assert_send_sync::<PalePython>();
    assert_send_sync::<PyEffectiveProfileCompilation>();
    assert_send_sync::<PyObservationSnapshot>();
};

/// Private native module embedded in the public `pypale` Python package.
///
/// Declarative exports let `PyO3` derive the authoritative native type stub from
/// the same classes and signatures that form the `CPython` ABI.
#[pymodule(name = "_native")]
mod python_module {
    #[pymodule_export]
    use super::{
        PaleDiagnosticError, PalePython, PyAbsentWorkspaceSubjectValues,
        PyAdmittedRepositoryActions, PyAuthoringCorrection, PyAuthoringCorrectionKind,
        PyAuthoringFieldFact, PyAuthoringFieldPresence, PyAuthoringIdentity,
        PyAuthoringIdentityKind, PyAuthoringIngressClass, PyAuthoringIngressLimits,
        PyAuthoringInspection, PyAuthoringInspectionState, PyAuthoringInstancePathSegment,
        PyAuthoringLiteral, PyAuthoringLiteralFact, PyAuthoringLiteralKind, PyAuthoringNavigation,
        PyAuthoringNodeFact, PyAuthoringOccurrence, PyAuthoringOccurrencePage,
        PyAuthoringOccurrencePageCursor, PyAuthoringOccurrenceRole, PyAuthoringPathSegment,
        PyAuthoringSourceLocation, PyAuthoringValueKind, PyBaselineEntry,
        PyBlockedRequirementDisposition, PyCapturedNativeActionRequest, PyCompilationDocument,
        PyCompiledInterfaceIdentity, PyConfigurationImprovementCandidateRequest,
        PyConfigurationImprovementComparison, PyConfigurationImprovementCoordinateChange,
        PyConfigurationImprovementCoordinateChangeKind, PyConfigurationImprovementDiagnosis,
        PyConfigurationImprovementDisposition, PyConfigurationImprovementEvaluation,
        PyConfigurationImprovementExplanation, PyConfigurationImprovementInspection,
        PyConfigurationImprovementPhase, PyConfigurationImprovementPreview,
        PyConfigurationImprovementPreviewSummary, PyConfigurationImprovementProposal,
        PyConfigurationImprovementProposalRequest, PyConfigurationImprovementTerminal,
        PyContractBundleIdentity, PyContractSchema, PyCurrentOnlyWorkspaceSubjectValues,
        PyDiagnostic, PyDiagnosticDisposition, PyDiagnosticEffectCertainty, PyDiagnosticProjection,
        PyDiagnosticRetry, PyDiagnosticStage, PyDurablePublicationMechanism,
        PyEffectiveAuthoringContinuation, PyEffectiveAuthoringInspection, PyEffectiveAuthoringPage,
        PyEffectiveProfileApplicationParticipant, PyEffectiveProfileAuthorityRootSelection,
        PyEffectiveProfileBoundProviderDisposition,
        PyEffectiveProfileCapabilityClassificationEvidence, PyEffectiveProfileCapabilityDetail,
        PyEffectiveProfileCapabilityNarrowingDetail, PyEffectiveProfileCapabilityParticipant,
        PyEffectiveProfileCommunicationResponsibility, PyEffectiveProfileCompilation,
        PyEffectiveProfileCompilationRequest, PyEffectiveProfileConfigurationEvidence,
        PyEffectiveProfileContextMemoryResponsibility, PyEffectiveProfileContinuityContractReason,
        PyEffectiveProfileCoordinates, PyEffectiveProfileCrmBusinessStateResponsibility,
        PyEffectiveProfileDeclaredAllFromPackageSelector, PyEffectiveProfileDeclaredExactSelector,
        PyEffectiveProfileDeclaredModeDefaultSelection,
        PyEffectiveProfileDeclaredResourceSelection, PyEffectiveProfileDeclaredSelectionDetail,
        PyEffectiveProfileDeclaredSelectionEvidence, PyEffectiveProfileDelegationEdgeReason,
        PyEffectiveProfileDisclosureCeilingSelection, PyEffectiveProfileEffectClass,
        PyEffectiveProfileEffectMismatchProviderDisposition,
        PyEffectiveProfileEffectiveSelectionEvidence, PyEffectiveProfileEngineParticipant,
        PyEffectiveProfileEngineeringExecutionResponsibility, PyEffectiveProfileEntry,
        PyEffectiveProfileEntryContractReason,
        PyEffectiveProfileExcludedByNarrowingProviderDisposition,
        PyEffectiveProfileExecutionPathDetail, PyEffectiveProfileExecutionPathEvidence,
        PyEffectiveProfileExplanation, PyEffectiveProfileExtensionResponsibility,
        PyEffectiveProfileFindingClassificationEvidence, PyEffectiveProfileFindingDetail,
        PyEffectiveProfileGoverningCeilingDetail, PyEffectiveProfileInterfaceParticipant,
        PyEffectiveProfileLifecycleSelection, PyEffectiveProfileMachineRealizationDetail,
        PyEffectiveProfileMachineRealizationEvidence,
        PyEffectiveProfileMissingBindingProviderDisposition, PyEffectiveProfileModeAxis,
        PyEffectiveProfileNativeEffectReadBackResponsibility,
        PyEffectiveProfileNativeEventPublicationResponsibility,
        PyEffectiveProfileNativeRealization, PyEffectiveProfileNativeRealizationUnknownFinding,
        PyEffectiveProfileNormativePackSelection,
        PyEffectiveProfileOperationClassificationEvidence, PyEffectiveProfileOperationDetail,
        PyEffectiveProfileOperationUnboundFinding,
        PyEffectiveProfileOrganizationalWorkResponsibility, PyEffectiveProfileOriginReceiptReason,
        PyEffectiveProfilePackageSelection, PyEffectiveProfilePage,
        PyEffectiveProfilePluginParticipant, PyEffectiveProfileProviderBindingReason,
        PyEffectiveProfileProviderBindingRef, PyEffectiveProfileProviderKind,
        PyEffectiveProfileProviderRepositoryDetail,
        PyEffectiveProfileRecipeContractMismatchProviderDisposition,
        PyEffectiveProfileRepositoryAccess, PyEffectiveProfileRepositoryDetail,
        PyEffectiveProfileRequirementFinding, PyEffectiveProfileResourceKind,
        PyEffectiveProfileResponsibilityContractReason, PyEffectiveProfileResponsibilityDetail,
        PyEffectiveProfileResponsibilityEvidence, PyEffectiveProfileRunPolicySelection,
        PyEffectiveProfileSection, PyEffectiveProfileSelectionDetail,
        PyEffectiveProfileSemanticResourceSelection, PyEffectiveProfileSessionSelection,
        PyEffectiveProfileSettlementContractReason, PyEffectiveProfileSkillParticipant,
        PyEffectiveProfileTerminalHarnessExecutionResponsibility,
        PyEffectiveProfileTerminalLeafParticipant, PyEffectiveProfileTerminationContractReason,
        PyLocalConfigurationImprovementRequest, PyLocalControlStorageInspection,
        PyLocalControlStorageInspectionRejection, PyLocalControlStorageInspectionRejectionKind,
        PyLocalControlStorageInspectionRequest, PyLocalControlStorageInspectionState,
        PyLocalControlStorageInspectionStateKind, PyLocalControlStorageProfileRequirement,
        PyLocalControlStoreKind, PyLocalMaterializationAction,
        PyLocalMaterializationAdmissionRequest, PyLocalMaterializationAdmittedRun,
        PyLocalMaterializationDrift, PyLocalMaterializationDriftKind,
        PyLocalMaterializationMachineEffectKind, PyLocalMaterializationOwnershipChange,
        PyLocalMaterializationOwnershipObservation, PyLocalMaterializationReceiptObservation,
        PyLocalMaterializationReconciliation, PyLocalMaterializationRunOutcome,
        PyLocalMaterializationRunResult, PyLocalMaterializationTargetAdmission,
        PyLocalMaterializationTargetOutcome, PyLocalMaterializationTargetResult,
        PyLocalNativeActionInputRequest, PyLocalOneShotCompilation,
        PyLocalOneShotCompilationRequest, PyLocalOneShotPlan, PyLocalOneShotPlanCounts,
        PyLocalOneShotPlanPublication, PyLocalOneShotPlanRequest,
        PyLocalPackageDistributionRequest, PyLocalRepositoryStatus, PyMachineLocalEffectCandidate,
        PyNarrowedRequirementDisposition, PyNarrowingReason, PyNativeActionDescriptor,
        PyObservationBackpressureRecovery, PyObservationCancellation, PyObservationCategory,
        PyObservationCausalPath, PyObservationCausalPathResult, PyObservationCausalResultKind,
        PyObservationCausalStep, PyObservationCausalStepKind, PyObservationChange,
        PyObservationChangeKind, PyObservationCursor, PyObservationDeltaOutcome,
        PyObservationDeltaOutcomeKind, PyObservationFact, PyObservationFilter,
        PyObservationFreshness, PyObservationFreshnessClass, PyObservationFreshnessKind,
        PyObservationGeneration, PyObservationGenerationKind, PyObservationHealthState,
        PyObservationLens, PyObservationLensCursor, PyObservationLensDisposition,
        PyObservationLensDispositionKind, PyObservationLensLimits, PyObservationLensNode,
        PyObservationNativeState, PyObservationNativeStateKind, PyObservationPage,
        PyObservationPageCursor, PyObservationPageLimits, PyObservationPageResult,
        PyObservationPageResultKind, PyObservationProcessIntent, PyObservationProfileContinuation,
        PyObservationProfileError, PyObservationProfileMapping, PyObservationProfilePage,
        PyObservationProfileResult, PyObservationProfileResultKind, PyObservationReadinessState,
        PyObservationReconciliationState, PyObservationReconciliationStateKind,
        PyObservationResynchronization, PyObservationResynchronizationReason,
        PyObservationRevision, PyObservationRevisionKind, PyObservationSnapshot,
        PyObservationStepCompletionKind, PyObservationStepLimitKind, PyObservationSubject,
        PyObservationSubjectKind, PyOneShotAdmittedPlan, PyOneShotBoundPlan, PyOneShotCompilation,
        PyOneShotCompilationRequest, PyOneShotObservationRequest, PyOneShotPlanCandidate,
        PyOneShotPlanKind, PyOneShotPlanningRequest, PyOneShotPriorMaterialization,
        PyOneShotPrivateInputs, PyOneShotSummary, PyOneShotWorkspaceInputs, PyOperationExposure,
        PyPackageDistributionInspection, PyPackageReleaseIdentity, PyPolicyBlock,
        PyPortableWorkspaceCompilation, PyPortableWorkspaceDependencyCursor,
        PyPortableWorkspaceDependencyPage, PyPreparedConfigurationImprovementPreview,
        PyPriorOnlyWorkspaceSubjectValues, PyProviderBindingAbsentRequirementDisposition,
        PyResidentHookAdmission, PyResidentHookRequest, PySatisfiedRequirementDisposition,
        PySharedWorkspaceSubjectValues, PyStandaloneAuthoringRole, PyValidatedContractDocument,
        PyWholeWorkspaceSubjectValue, PyWorkspaceCausalStep, PyWorkspaceChangeKind,
        PyWorkspaceChangeRoot, PyWorkspaceChangeRootPage, PyWorkspaceComparison,
        PyWorkspaceComparisonSubjectCursor, PyWorkspaceComparisonSubjectPage,
        PyWorkspaceComparisonSummary, PyWorkspaceComponentSubjectValue,
        PyWorkspaceDeclarationSubjectValue, PyWorkspaceDerivationRule,
        PyWorkspaceExperienceCompositionSubjectValue, PyWorkspaceExplanation,
        PyWorkspaceExplanationFrontier, PyWorkspaceExplanationState,
        PyWorkspaceExtensionSubjectValue, PyWorkspaceFact, PyWorkspaceFactKind, PyWorkspaceImpact,
        PyWorkspaceImpactPage, PyWorkspaceImpactState, PyWorkspaceImportSubjectValue,
        PyWorkspaceModuleSubjectValue, PyWorkspaceOrganizationalTranslationClosureSubjectValue,
        PyWorkspacePolicySubjectValue, PyWorkspaceProvenance, PyWorkspaceProvenanceKind,
        PyWorkspaceResolutionLockSubjectValue, PyWorkspaceResourceSubjectValue,
        PyWorkspaceResponsibilityClosureSubjectValue, PyWorkspaceSourceSubjectValue,
        PyWorkspaceSubject, PyWorkspaceSubjectComparison, PyWorkspaceSubjectKind,
        PyWorkspaceUniverseLockSubjectValue, PyWorkspaceValueComparison,
    };
}

#[cfg(test)]
#[path = "../tests/internal/mod.rs"]
mod tests;
