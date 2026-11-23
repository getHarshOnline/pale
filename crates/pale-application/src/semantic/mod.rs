//! Work out what a sealed Package graph means for one repository: the roles,
//! procedures and policies that apply, without reading Package source again.
//!
//! This module builds on `host_command` and `packages`; `compile`, `failure`,
//! `render`, `resident` and `source_intelligence` build on it.
//!
//! Package admission owns authored links, source paths, schema validation, and
//! occurrence closure. This module begins only after those concerns have been
//! consumed. It composes selected meaning for one repository and then joins the
//! private operation/capability state without manufacturing provider selection
//! or provider execution availability.

mod diagnostic;
mod graph_error;
mod model;
mod obligation;
pub(crate) use obligation::obligation_disposition;
mod package_lineage;
mod page_reading;
mod projection;

pub(crate) use diagnostic::DiagnosticAccumulator;
pub use diagnostic::{
    InvalidCapabilityBindingReason, InvalidModeDefaultReason, SemanticCause, SemanticComposition,
    SemanticCoordinate, SemanticDiagnostic, SemanticDiagnosticCode, SemanticDiagnosticSource,
    SemanticDiagnostics, SemanticResourceExpectation, SemanticStage, MAX_SEMANTIC_DIAGNOSTICS,
};
pub use graph_error::SemanticError;
pub(crate) use model::resource_label;
pub(crate) use model::ResolvedPageReading;
pub use model::{
    BodyLineage, ProviderUnavailability, RepositorySemanticPlan, ResolvedCapability,
    ResolvedDoctrine, ResolvedDoctrineRecord, ResolvedOperation, ResolvedOperationMechanics,
    ResolvedOperationRecord, ResolvedPolicyFamily, ResolvedProviderSelection, ResolvedRecipe,
    ResolvedRole, ResolvedRoleRecord, ResolvedRunMode, ResolvedRunModeRecord, ResolvedStack,
};
pub use obligation::{
    compile_package_obligations, settle_obligation, validate_evidence_coordinates,
    AdmittedEvidenceCoordinates, CapabilityObligation, CapabilityObligationDisposition,
    CoverageCoordinate, CoverageGap, EvidenceCoordinate, EvidenceCoordinateError,
    EvidenceCoordinateState, EvidenceCoordinates, EvidenceSettlementInput, EvidenceSettlementState,
    FreshnessCoordinate, FreshnessLayer, FreshnessState, GroundingAssertion, GroundingCoordinate,
    GroundingKind, IndependenceCoordinate, IndependenceCoupling, IndependenceRelation,
    MechanicalObligationClosure, MechanicalObligationError, NativeVerification, ObligationSubject,
    OperationObligation, OperationObligationState, OwnerPolicyRequirement,
    ProviderEvidenceCoordinate, ProviderProcedureObligation, QualitativeFrontier,
    RepeatabilityCoordinate, RepeatabilityMethod, RepositoryObligationClosure,
    RepositoryVerificationDisposition, ResidualAction, SettlementDecision, SettlementTarget,
    MAX_EVIDENCE_AXIS_MEMBERS,
};
pub use package_lineage::{
    admit_normative_pack_reference, admit_normative_pack_use, admit_package_activation,
    admit_package_materialization, admit_package_native_effect, admit_package_settlement,
    admit_package_use, admit_root_occurrence, derive_child_occurrence,
    propagate_package_invalidation, AdmittedNormativePackReference, AdmittedNormativePackUse,
    AdmittedPackageActivation, AdmittedPackageMaterialization, AdmittedPackageNativeEffect,
    AdmittedPackageOccurrence, AdmittedPackageSettlement, AdmittedPackageUse, ChildOccurrenceError,
    NormativePackReferenceAdmission, NormativePackReferenceError, NormativePackUse,
    NormativePackUseError, NormativePackUsePurpose, NormativePackUseRights, OccurrenceCeilings,
    OccurrenceContextDerivation, PackPrecedence, PackageActivation, PackageActivationError,
    PackageGeneratedDependencySource, PackageInvalidation, PackageInvalidationClosure,
    PackageInvalidationKind, PackageLineageEdge, PackageLineageError, PackageLineageGraph,
    PackageLineageNode, PackageMaterialization, PackageMaterializationError, PackageNativeEffect,
    PackageNativeEffectError, PackageOccurrence, PackageOccurrenceContext, PackageSettlement,
    PackageSettlementError, PackageUse, PackageUseDisposition, PackageUseError,
    RootOccurrenceError, MAX_PACKAGE_LINEAGE_MEMBERS,
};
pub(crate) use projection::project_repository_semantic_plans;
