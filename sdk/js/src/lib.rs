//! Native `PaleJS` binding for Node.js, projected from [`pale_sdk`].
//!
//! The `PaleJS` package (`@rtepl/pale`) loads this crate as its native addon.
//!
//! The leaf owns only Node-API translation and opaque host handles. Every
//! compiler, validator, diagnostic, planning, and native-action decision is
//! delegated to [`pale_sdk::PaleApi`]. One-shot compilation runs on the
//! Node-API worker pool with owned input and promise resolution, as does fresh
//! local configuration preview; other calls remain synchronous. Scheduling
//! introduces no second cancellation model.

mod authoring;
mod control_storage;
mod distribution;
mod effective_profile;
mod effective_profile_entries;
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

use napi::Result;
use napi_derive::napi;
use pale_sdk::{capability_inventory, ApiLeaf, PaleApi};

pub use authoring::{
    AuthoringCorrection, AuthoringCorrectionKind, AuthoringFieldFact, AuthoringFieldPresence,
    AuthoringIdentity, AuthoringIdentityKind, AuthoringIngressClass, AuthoringIngressLimits,
    AuthoringInspection, AuthoringInspectionState, AuthoringInstancePathSegment,
    AuthoringLiteralFact, AuthoringLiteralKind, AuthoringLocationSegment, AuthoringNavigation,
    AuthoringNodeFact, AuthoringOccurrence, AuthoringOccurrencePage, AuthoringOccurrencePageCursor,
    AuthoringOccurrenceRole, AuthoringPathSegment, AuthoringSourceLocation, AuthoringValueKind,
    PortableWorkspaceSourceInspection, PortableWorkspaceSourceState,
    PortableWorkspaceSubjectMembership, PortableWorkspaceSubjectOccurrences,
    StandaloneAuthoringRole, MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES,
};
pub use control_storage::{
    LocalControlStorageInspection, LocalControlStorageInspectionRequest,
    LocalControlStorageProfileRequirement, LocalControlStorageRejection,
    LocalControlStorageRejectionKind, LocalControlStorageStateKind, LocalControlStoreKind,
};
pub use distribution::{PackageDistributionInspection, PackageReleaseIdentity};
pub use effective_profile::{
    EffectiveAuthoringContinuation, EffectiveAuthoringInspection, EffectiveAuthoringPage,
    EffectiveProfileCompilation, EffectiveProfileCompilationRequest, EffectiveProfileCoordinates,
    EffectiveProfilePage, EffectiveProfileSection,
};
pub use effective_profile_entries::{
    EffectiveProfileEffectClass, EffectiveProfileEntry, EffectiveProfileEntryDetail,
    EffectiveProfileParticipant, EffectiveProfileProviderBindingRef,
    EffectiveProfileRepositoryAccess, EffectiveProfileResponsibility,
};
pub use effective_profile_explanation::{
    EffectiveProfileExplanation, EffectiveProfileExplanationEvidence,
};
pub use effective_profile_values::{
    EffectiveProfileDeclaredSelection, EffectiveProfileDeclaredSelector, EffectiveProfileFinding,
    EffectiveProfileModeAxis, EffectiveProfileNativeRealization, EffectiveProfilePathReason,
    EffectiveProfileProviderDisposition, EffectiveProfileProviderKind,
    EffectiveProfileResourceKind, EffectiveProfileSelection,
};
pub use improvement::{
    ConfigurationImprovementCandidateRequest, ConfigurationImprovementCoordinateChange,
    ConfigurationImprovementCoordinateChangeKind, ConfigurationImprovementDiagnosis,
    ConfigurationImprovementDisposition, ConfigurationImprovementEvaluation,
    ConfigurationImprovementExplanation, ConfigurationImprovementInspection,
    ConfigurationImprovementPhase, ConfigurationImprovementPreview,
    ConfigurationImprovementPreviewSummary, ConfigurationImprovementProposal,
    ConfigurationImprovementProposalRequest, ConfigurationImprovementTerminal,
    LocalConfigurationImprovementRequest, LocalConfigurationImprovementTask,
    PreparedConfigurationImprovementPreview,
};

pub use local::{
    CapturedNativeActionRequest, CompiledInterfaceIdentity, DurablePublicationMechanism,
    LocalMaterializationAction, LocalMaterializationAdmissionRequest,
    LocalMaterializationAdmittedRun, LocalMaterializationDrift, LocalMaterializationDriftKind,
    LocalMaterializationMachineEffectKind, LocalMaterializationOwnershipChange,
    LocalMaterializationOwnershipObservation, LocalMaterializationReceiptObservation,
    LocalMaterializationReconciliation, LocalMaterializationRunOutcome,
    LocalMaterializationRunResult, LocalMaterializationTargetAdmission,
    LocalMaterializationTargetOutcome, LocalMaterializationTargetResult,
    LocalNativeActionInputRequest, LocalOneShotCompilation, LocalOneShotCompilationRequest,
    LocalOneShotPlan, LocalOneShotPlanCounts, LocalOneShotPlanPublication, LocalOneShotPlanRequest,
    LocalPackageDistributionRequest, LocalRepositoryStatus,
};
pub use observation::{
    ObservationBackpressureRecovery, ObservationCancellation, ObservationCategory,
    ObservationCausalPath, ObservationCausalPathResult, ObservationCausalResultKind,
    ObservationCausalStep, ObservationCausalStepKind, ObservationChange, ObservationChangeKind,
    ObservationCursor, ObservationDeltaOutcome, ObservationDeltaOutcomeKind, ObservationFact,
    ObservationFilter, ObservationFreshness, ObservationFreshnessClass, ObservationFreshnessKind,
    ObservationGeneration, ObservationGenerationKind, ObservationHealthState, ObservationLens,
    ObservationLensCursor, ObservationLensDisposition, ObservationLensDispositionKind,
    ObservationLensLimits, ObservationLensNode, ObservationNativeState, ObservationNativeStateKind,
    ObservationPage, ObservationPageCursor, ObservationPageLimits, ObservationPageResult,
    ObservationPageResultKind, ObservationProcessIntent, ObservationProfileAssociation,
    ObservationProfileAssociationResult, ObservationProfileAssociationResultKind,
    ObservationProfileContinuation, ObservationProfileError, ObservationProfileMapping,
    ObservationProfilePage, ObservationProfileResult, ObservationProfileResultKind,
    ObservationReadinessState, ObservationReconciliationState, ObservationReconciliationStateKind,
    ObservationResynchronization, ObservationResynchronizationReason, ObservationRevision,
    ObservationRevisionKind, ObservationSnapshot, ObservationStepCompletionKind,
    ObservationStepLimitKind, ObservationSubject, ObservationSubjectKind,
};
pub use one_shot::{
    BaselineEntry, CompilationDocument, OneShotAdmittedPlan, OneShotBoundPlan, OneShotCompilation,
    OneShotCompilationRequest, OneShotCompilationTask, OneShotObservationRequest,
    OneShotPlanCandidate, OneShotPlanKind, OneShotPlanningRequest, OneShotPriorMaterialization,
    OneShotPrivateInputs, OneShotSummary,
};
pub use portable::{
    ContractBundleIdentity, ContractSchema, Diagnostic, DiagnosticDisposition,
    DiagnosticEffectCertainty, DiagnosticFact, DiagnosticProjection, DiagnosticRetry,
    DiagnosticStage, NativeActionDescriptor, OrganizationAttribution, OrganizationAxes,
    OrganizationCommunicationResolution, OrganizationCommunicationResolutionHandle,
    OrganizationComparisonSide, OrganizationConsumerAcceptanceAnswer, OrganizationCorrespondence,
    OrganizationCorrespondenceCursor, OrganizationCorrespondenceState, OrganizationCursor,
    OrganizationDerivedClass, OrganizationDerivedSource, OrganizationDetailCursor,
    OrganizationDetailPage, OrganizationDetailParent, OrganizationDetailRows,
    OrganizationDisclosure, OrganizationEffectCeiling, OrganizationEndpointLookup,
    OrganizationEntry, OrganizationEvidenceRequirement, OrganizationFactState,
    OrganizationGenealogyHandle, OrganizationGenealogyLink, OrganizationGroupRef,
    OrganizationInspectionCoordinate, OrganizationInterval, OrganizationLensRight,
    OrganizationNativeReadBackAnswer, OrganizationNode, OrganizationPage,
    OrganizationPlacementAnswer, OrganizationProjectionClass, OrganizationProviderAcceptanceAnswer,
    OrganizationProviderCompletionAnswer, OrganizationRecordHandle, OrganizationRef,
    OrganizationRelationClass, OrganizationResidence, OrganizationResidenceClass,
    OrganizationResourceAnswer, OrganizationResponsibility, OrganizationResponsibilityLifecycle,
    OrganizationResponsibilityPlacement, OrganizationResponsibilityProjection,
    OrganizationResponsibilityProjectionHandle, OrganizationRoleRef, OrganizationRow,
    OrganizationScopeRef, OrganizationServiceAssignment, OrganizationServiceAssignmentHandle,
    OrganizationServiceRef, OrganizationSettlementAnswer, OrganizationSourceCursor,
    OrganizationSourceMapping, OrganizationSourcePage, OrganizationStructuralKind,
    OrganizationStructuralSelection, OrganizationTreeRef, PortableWorkspaceCompilation,
    PortableWorkspaceDependencyCursor, PortableWorkspaceDependencyPage,
    PortableWorkspaceDependentCursor, PortableWorkspaceDependentPage, PortableWorkspaceDocument,
    PortableWorkspaceRequest, PortableWorkspaceSubjectInspection,
    PortableWorkspaceSubjectInspectionCursor, RecipientQualifiedComparisonSubjectCursor,
    RecipientQualifiedComparisonSubjectPage, RecipientQualifiedContributingRootsCursor,
    RecipientQualifiedContributingRootsPage, RecipientQualifiedDependencyCursor,
    RecipientQualifiedDependencyPage, RecipientQualifiedOrganizationCorrespondence,
    RecipientQualifiedOrganizationCorrespondenceCursor,
    RecipientQualifiedOrganizationCorrespondenceState, RecipientQualifiedOrganizationCursor,
    RecipientQualifiedOrganizationEntry, RecipientQualifiedOrganizationPage,
    RecipientQualifiedOrganizationRecordHandle, RecipientQualifiedOrganizationSourceCursor,
    RecipientQualifiedOrganizationSourceMapping, RecipientQualifiedOrganizationSourcePage,
    RecipientQualifiedSubjectInspection, RecipientQualifiedSubjectInspectionCursor,
    RecipientQualifiedWorkspaceComparisonInspection, RecipientQualifiedWorkspaceInspection,
    RecipientQualifiedWorkspacePairSummary, RecipientQualifiedWorkspaceSummary,
    ValidatedContractDocument, WorkspaceCausalStep, WorkspaceChangeKind, WorkspaceChangeRoot,
    WorkspaceChangeRootPage, WorkspaceComparison, WorkspaceComparisonSubjectCursor,
    WorkspaceComparisonSubjectPage, WorkspaceComparisonSummary, WorkspaceContributingRootsCursor,
    WorkspaceContributingRootsPage, WorkspaceContributingRootsState, WorkspaceDerivationRule,
    WorkspaceExplanation, WorkspaceExplanationFrontier, WorkspaceExplanationState, WorkspaceFact,
    WorkspaceFactKind, WorkspaceImpact, WorkspaceImpactPage, WorkspaceImpactState,
    WorkspaceProvenance, WorkspaceProvenanceKind, WorkspaceSubject, WorkspaceSubjectComparison,
    WorkspaceSubjectKind, WorkspaceSubjectValues, WorkspaceValueComparison,
};
pub use profile_dispositions::{
    NarrowingReason, OperationExposure, PolicyBlock, RequirementDisposition,
};
pub use repository_actions::AdmittedRepositoryActions;
pub use resident_hook::{ResidentHookAdmission, ResidentHookRequest};
pub use workspace_subject_value::WorkspaceSubjectValue;

/// Native PaleJS handle over the complete eligible Pale façade.
#[napi(js_name = "Pale")]
pub struct Pale {
    pub(crate) api: Arc<PaleApi>,
}

#[napi]
impl Pale {
    /// Verify embedded contracts and construct the native façade.
    ///
    /// # Errors
    ///
    /// Returns the stable initialization code and message when the embedded
    /// contract bundle contradicts the compiled façade.
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        PaleApi::new()
            .map(|api| Self { api: Arc::new(api) })
            .map_err(error::initialization_error)
    }

    /// Return the API-owned Node eligibility set in stable order.
    #[napi(getter)]
    pub fn capability_ids(&self) -> Vec<String> {
        eligible_capability_ids()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }
}

/// Derive the leaf surface from the one API-owned capability inventory.
fn eligible_capability_ids() -> Vec<&'static str> {
    capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Node))
        .map(|capability| capability.id().as_str())
        .collect()
}

/// Compile-time proof that no unsendable escape hatch is needed by the host handle.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PaleApi>();
    assert_send_sync::<Pale>();
    assert_send_sync::<EffectiveProfileCompilation>();
    assert_send_sync::<ObservationSnapshot>();
};

#[cfg(test)]
#[path = "../tests/internal/mod.rs"]
mod tests;
