//! Sealing, verification, and publisher-gated universe admission.
//!
//! Publisher evidence is deliberately not synthesized from a readable module,
//! a release label, or a digest. Production code has no bypass: until an exact
//! publisher-verification owner is selected, admission returns
//! [`WorkspaceError::PublisherAdmissionUnavailable`].

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    DeclarationDependencyKindV1, ExperienceCompositionDeclarationsV1,
    ExperienceCompositionSourceDigest, NormativePackAttestorV1, NormativePackIdV1,
    NormativePackReleaseV1, OrganizationalTranslationClosureDigest, OrganizationalTranslationIrV1,
    PackageIdV1, PackageReleaseV1, PrincipalAdmittedUniverseDigest,
    ResponsibilityDerivationClosureDigest, ResponsibilityDerivationIrV1,
    WorkspaceAdmissionEvidenceDigest, WorkspaceAdoptedReleaseV1, WorkspaceDeclarationRefV1,
    WorkspaceDerivationImpactStateV1, WorkspaceDerivationImpactV1, WorkspaceDerivationNodeDigest,
    WorkspaceDerivationNodeV1, WorkspaceDerivationOriginV1, WorkspaceDerivationReceiptPreimageV1,
    WorkspaceDerivationReceiptV1, WorkspaceDerivationRootChangeV1, WorkspaceDerivationRootDigest,
    WorkspaceDerivationRootV1, WorkspaceDerivationRuleV1, WorkspaceDerivationSourceV1,
    WorkspaceDerivationSubjectV1, WorkspaceDerivationValueDigest, WorkspaceId,
    WorkspaceImportSurfaceDigest, WorkspaceModuleDigest, WorkspaceModuleId,
    WorkspaceProtectedDeliveryProofDigest, WorkspacePublisherId, WorkspacePublisherKeyId,
    WorkspacePublisherProofDigest, WorkspaceRecoveryAuthorityId, WorkspaceReleaseId,
    WorkspaceReleaseStateV1, WorkspaceResolutionLockDigest, WorkspaceResourceId,
    WorkspaceSemanticIrDigest, WorkspaceUniverseLockDigest,
};
use serde::Serialize;

use super::link::{
    LinkedAccessEntry, LinkedDeclaration, LinkedExtension, QualifiedDeclarationRef,
    ResolvedImportPath,
};
use super::normalize::{EffectiveDeclarationPolicy, NormalizedWorkspace};
use super::WorkspaceError;

mod ir;
pub(super) use ir::seal_and_verify_workspace;
mod admission;
#[cfg(test)]
#[path = "../../tests/internal/workspace/universe/language_support.rs"]
pub(super) mod language_test_support;
pub(crate) use admission::{
    PrincipalAdmittedUniverse, ResidentWorkspaceDefinition, ResidentWorkspaceDerivation,
    ResidentWorkspaceParts, WorkspaceAdmissionEvidence,
};
pub(crate) use ir::{ResidentWorkspaceDependencyKind, WorkspaceSemanticDeclarationEntry};
mod derivation;
pub(in crate::workspace) use derivation::{
    WorkspaceDerivationExplanation, WorkspaceDerivationExplanationFrontier,
};
