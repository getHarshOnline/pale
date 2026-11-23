//! Load a person's workspace declarations into a portable profile and the
//! machine-bound instance of it.
//!
//! This module builds on `captured_input`, `composition`, `identity` and
//! `packages`; `compile`, `doctor`, `improvement`, `observation`, `packages` and
//! `resident` build on it.
//!
//! Inputs reject unknown fields. Machine paths contribute only to the instance
//! digest; changing them must not alter the portable profile digest.

mod authoring;
mod captured;
mod comparison_subjects;
mod dependency;
mod entry;
mod errors;
mod host_capture;
mod impact_roots;
mod inspection_admission;
mod inspection_issuer;
mod instance;
mod link;
mod load;
mod normalize;
mod organization_inspection;
mod portable;
mod profile;
mod proof_dependencies;
mod proof_dependents;
mod session;
mod source;
mod source_admission;
mod source_lowering;
mod subject_inspection;
mod topology;
mod universe;

pub use crate::composition::organization::OrganizationalTranslationError;
pub use crate::composition::responsibility::ResponsibilityDerivationError;
pub use authoring::{
    inspect_authoring_node, AuthoringCorrectionEvidence, AuthoringCorrectionKind,
    AuthoringDocumentKind, AuthoringFieldFact, AuthoringFieldPresence, AuthoringIdentity,
    AuthoringIdentityKind, AuthoringInstancePathSegment, AuthoringLiteral, AuthoringLiteralFact,
    AuthoringNodeFact, AuthoringOccurrence, AuthoringOccurrencePage, AuthoringOccurrencePageCursor,
    AuthoringOccurrenceRole, AuthoringPathSegment, AuthoringSchemaError, AuthoringSemanticIndex,
    AuthoringSourceMap, AuthoringSourceRole, AuthoringTomlGrammar, AuthoringValueKind,
    WorkspaceAuthoringLocation, DEFAULT_AUTHORING_OCCURRENCE_PAGE_LIMIT,
    MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES, MAX_AUTHORING_OCCURRENCE_PAGE_LIMIT,
};
pub(crate) use captured::{
    admit_captured_private_inputs, CapturedPrivateAdmissionError, CapturedPrivateRecords,
};
pub use captured::{
    CapturedLifecycleGuidance, CapturedPrivateDocument, CapturedPrivateWorkspaceInputs,
};
pub use comparison_subjects::{
    PortableWorkspaceComparisonSubjectPage, MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES,
};
pub use entry::{resolve_entry, OneShotHostEffectContext, ResolvedEntry};
pub use errors::{WorkspaceError, WorkspaceSourceCoordinate};
pub use host_capture::{
    CapturedConfigurationImprovementCandidateSourceParts,
    CapturedConfigurationImprovementCandidateSources, CapturedOneShotSourceDocument,
    CapturedOneShotSourceParts, CapturedOneShotSources, OneShotSourceCaptureError,
};
pub use impact_roots::{
    InvalidContributingRootOffset, PortableWorkspaceContributingRootPage,
    PortableWorkspaceContributingRoots, DEFAULT_CONTRIBUTING_ROOT_PAGE_LIMIT,
    MAX_CONTRIBUTING_ROOT_PAGE_LIMIT,
};
pub use inspection_admission::{
    AdmittedComparisonChangeRoots, AdmittedComparisonContributingRoots,
    AdmittedComparisonDirectDependencies, AdmittedComparisonImpacts, AdmittedComparisonSubject,
    AdmittedComparisonSubjectInspection, AdmittedComparisonSubjects, AdmittedDirectDependencies,
    AdmittedOrganizationCorrespondence, AdmittedOrganizationDetail, AdmittedOrganizationSources,
    AdmittedWorkspaceComparisonInspection, AdmittedWorkspaceInspection,
    WorkspaceInspectionAccessError, WorkspaceInspectionAuthorityGeneration,
    WorkspaceInspectionBudget, WorkspaceInspectionBudgetExceeded, WorkspaceInspectionBudgetLimits,
    WorkspaceInspectionComparisonSide, WorkspaceInspectionDenied,
    WorkspaceInspectionOrganizationSection, WorkspaceInspectionRecipient,
};
pub(crate) use inspection_admission::{
    WorkspaceInspectionAdmissionOwner, WorkspaceInspectionAdmissionRequest,
    WorkspaceInspectionRecipientAdmissionRequest,
};
pub use inspection_issuer::{
    WorkspaceInspectionAuthority, WorkspaceInspectionAuthorityError, WorkspaceInspectionGrant,
    WorkspaceInspectionPairIssuance,
};
pub(crate) use instance::{
    resolve_instance, verify_durable_evidence_root, verify_instance_executable_binding,
    verify_native_harnesses, verify_reading_order,
};
pub use instance::{
    verify_materialization_lineage_binding, verify_materialization_lineage_root,
    verify_native_harness, verify_process_executable_binding, verify_session_state_root,
};
pub use load::WorkspaceInputSnapshot;
pub use organization_inspection::{
    OrganizationCommunicationResolutionHandle, OrganizationCorrespondenceCursor,
    OrganizationCorrespondencePage, OrganizationCorrespondenceState, OrganizationEndpointLookup,
    OrganizationGenealogyHandle, OrganizationInspectionCursor, OrganizationInspectionEntry,
    OrganizationInspectionError, OrganizationInspectionPage, OrganizationInspectionRecord,
    OrganizationMemberCursor, OrganizationMemberEntry, OrganizationMemberPage,
    OrganizationRecordHandle, OrganizationResponsibilityProjectionHandle,
    OrganizationServiceAssignmentHandle, OrganizationSourceCursor, OrganizationSources,
};
pub use portable::{
    compile_portable_workspace, validate_standalone_authoring_document, CapturedWorkspaceDocument,
    CapturedWorkspaceSource, PortableWorkspaceDerivation, PortableWorkspaceDerivationExplanation,
    PortableWorkspaceExplanationFrontier, PortableWorkspaceFailure, PortableWorkspaceFailureClass,
    PortableWorkspaceSourceSubject, PortableWorkspaceSubjectComparison,
    PortableWorkspaceSubjectOccurrences, PortableWorkspaceUniverse,
    PortableWorkspaceValueComparison,
};
pub(crate) use profile::{compute_effective_profile, EffectiveProfileInput};
pub use proof_dependencies::{
    InvalidDependencyOffset, PortableWorkspaceDependencyPage, DEFAULT_DEPENDENCY_PAGE_LIMIT,
    MAX_DEPENDENCY_PAGE_COPIED_STRING_BYTES, MAX_DEPENDENCY_PAGE_LIMIT,
};
pub use proof_dependents::{
    InvalidDependentOffset, PortableWorkspaceDependentPage, DEFAULT_DEPENDENT_PAGE_LIMIT,
    MAX_DEPENDENT_PAGE_INPUT_COMPARISONS, MAX_DEPENDENT_PAGE_LIMIT,
};
pub(crate) use session::{
    declared_readonly_authority_paths, declared_readonly_repository_paths,
    minimal_readonly_path_cover, resolve_readonly_authority_paths,
    resolve_readonly_repository_paths,
};
pub use source_lowering::{
    lower_captured_workspace_source, lower_local_workspace_source, WorkspaceSourceLowering,
};
pub use subject_inspection::{
    PortableWorkspaceSubjectInspection, PortableWorkspaceSubjectInspectionCursor,
    PortableWorkspaceSubjectInspectionError,
};
pub(crate) use topology::{
    select_workspace_topology, selected_topology_digest, selected_topology_digest_from_parts,
    SelectedRepositoryMembership, SelectedWorkspaceTopology,
};
pub(crate) use universe::{
    PrincipalAdmittedUniverse, ResidentWorkspaceDefinition, ResidentWorkspaceDependencyKind,
    ResidentWorkspaceDerivation, ResidentWorkspaceParts, WorkspaceAdmissionEvidence,
    WorkspaceSemanticDeclarationEntry,
};

/// Link and normalize once while retaining source-free authoring navigation.
fn normalize_workspace_source(
    source: source::ParsedWorkspaceSource,
) -> Result<(normalize::NormalizedWorkspace, AuthoringSemanticIndex), WorkspaceError> {
    let mut linked = link::link_workspace(source)?;
    let authoring = AuthoringSemanticIndex::from_linked(&mut linked);
    let normalized = normalize::normalize_workspace(linked)?;
    Ok((normalized, authoring))
}

/// Admit one portable source while retaining linker-owned authoring evidence.
pub(crate) fn admit_workspace_source_with_authoring(
    source: source::ParsedWorkspaceSource,
) -> Result<(universe::PrincipalAdmittedUniverse, AuthoringSemanticIndex), WorkspaceError> {
    let (normalized, authoring) = normalize_workspace_source(source)?;
    let verified = universe::seal_and_verify_workspace(normalized)?;
    verified
        .admit_without_external_evidence()
        .map(|universe| (universe, authoring))
}

/// Validate and canonically lower attributable responsibility derivations.
///
/// This pure pre-wiring operation does not add the derivations to the one-shot
/// composition source, admit a universe, or produce native effects.
///
/// # Errors
///
/// Returns [`ResponsibilityDerivationError`] for an incomplete, conflicting,
/// noncanonical, or unresolved derivation set.
pub fn lower_responsibility_derivations_v1(
    derivations: Vec<pale_core::records::ResponsibilityDerivationV1>,
) -> Result<pale_core::records::ResponsibilityDerivationIrV1, ResponsibilityDerivationError> {
    crate::composition::lower_responsibility_derivations(derivations)
}

/// Validate and canonically lower one heterogeneous organizational source.
///
/// This pure pre-wiring operation preserves every explicit negative state and
/// introduces no inferred organization, role, authority, relation, or native
/// projection.
///
/// # Errors
///
/// Returns [`OrganizationalTranslationError`] for an invalid bound, identity,
/// reference, interval, relation, service, lens, or inspection contract.
pub fn lower_organizational_translation_v1(
    source: pale_core::records::OrganizationalTranslationV1,
) -> Result<pale_core::records::OrganizationalTranslationIrV1, OrganizationalTranslationError> {
    crate::composition::lower_organizational_translation(source)
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/main.rs"]
mod tests;
