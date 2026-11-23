//! Structural joins for Package materialization, activation, use, effect, and settlement claims.

use pale_core::records::{
    ActivationFenceDigest, CompiledGenerationDigest, EffectClass, ImmutableRecordReferenceV1,
    PackageResourceRefV1, UnixTimeV1, WorkspaceResourceId,
};

use super::super::{
    settle_obligation, EvidenceSettlementInput, NativeVerification, ObligationSubject,
    SettlementDecision, SettlementTarget, MAX_EVIDENCE_AXIS_MEMBERS,
};
use super::AdmittedPackageOccurrence;

/// Claimed materialization for one occurrence; it proves no owner-native state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageMaterialization {
    /// Materialization record.
    pub materialization: ImmutableRecordReferenceV1,
    /// Exact occurrence materialized.
    pub occurrence: ImmutableRecordReferenceV1,
    /// Opaque artifact coordinate.
    ///
    /// This record cannot distinguish a generated projection from an installed
    /// artifact, so neither lifecycle may be inferred from this field.
    pub artifact: ImmutableRecordReferenceV1,
}

/// Materialization claim structurally joined to its exact occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPackageMaterialization {
    record: PackageMaterialization,
    occurrence: ImmutableRecordReferenceV1,
}

impl AdmittedPackageMaterialization {
    /// Borrow the structurally admitted materialization claim.
    #[must_use]
    pub fn record(&self) -> &PackageMaterialization {
        &self.record
    }

    /// Borrow the exact occurrence carried through admission.
    #[must_use]
    pub fn occurrence(&self) -> &ImmutableRecordReferenceV1 {
        &self.occurrence
    }
}

/// Materialization does not name the supplied occurrence.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("Package materialization does not name the supplied occurrence")]
pub struct PackageMaterializationError;

/// Admit a materialization without implying activation.
///
/// # Errors
///
/// Returns [`PackageMaterializationError`] when `candidate` names another
/// occurrence.
pub fn admit_package_materialization(
    occurrence: &AdmittedPackageOccurrence,
    candidate: PackageMaterialization,
) -> Result<AdmittedPackageMaterialization, PackageMaterializationError> {
    if candidate.occurrence != occurrence.record().occurrence {
        return Err(PackageMaterializationError);
    }
    Ok(AdmittedPackageMaterialization {
        occurrence: occurrence.record().occurrence.clone(),
        record: candidate,
    })
}

/// Claimed activation for one materialized occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageActivation {
    /// Activation record.
    pub activation: ImmutableRecordReferenceV1,
    /// Exact materialization activated.
    pub materialization: ImmutableRecordReferenceV1,
    /// Exact activation fence.
    pub fence: ActivationFenceDigest,
}

/// Activation claim structurally joined to its exact materialization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPackageActivation {
    record: PackageActivation,
    occurrence: ImmutableRecordReferenceV1,
}

impl AdmittedPackageActivation {
    /// Borrow the structurally admitted activation claim.
    #[must_use]
    pub fn record(&self) -> &PackageActivation {
        &self.record
    }

    /// Borrow the exact occurrence carried through materialization admission.
    #[must_use]
    pub fn occurrence(&self) -> &ImmutableRecordReferenceV1 {
        &self.occurrence
    }
}

/// Activation does not name the supplied materialization.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("Package activation does not name the supplied materialization")]
pub struct PackageActivationError;

/// Admit an activation without implying Package use.
///
/// # Errors
///
/// Returns [`PackageActivationError`] when `candidate` names another
/// materialization.
pub fn admit_package_activation(
    materialization: &AdmittedPackageMaterialization,
    candidate: PackageActivation,
) -> Result<AdmittedPackageActivation, PackageActivationError> {
    if candidate.materialization != materialization.record().materialization {
        return Err(PackageActivationError);
    }
    Ok(AdmittedPackageActivation {
        occurrence: materialization.occurrence().clone(),
        record: candidate,
    })
}

/// Claimed-use dispositions that must not be collapsed into one success Boolean.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageUseDisposition {
    /// Invocation began but has no terminal disposition.
    Attempted,
    /// Admission denied the attempted use.
    Denied,
    /// The declared reduced-outcome path was used.
    Degraded,
    /// Attempted use terminated with failure.
    Failed,
    /// The exact selected procedure completed; native effect remains separate.
    Completed,
}

/// Immutable Package-use claim for one exact occurrence and invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageUse {
    /// Immutable use-record identity.
    pub use_record: ImmutableRecordReferenceV1,
    /// Exact Package occurrence consumed.
    pub occurrence: ImmutableRecordReferenceV1,
    /// Exact operation selected within that occurrence.
    pub operation: PackageResourceRefV1,
    /// Invocation identity.
    pub invocation: ImmutableRecordReferenceV1,
    /// Optional exact activation consumed by this use.
    pub activation: Option<ImmutableRecordReferenceV1>,
    /// Optional provider coordinate; semantic-only use has none.
    pub provider: Option<ImmutableRecordReferenceV1>,
    /// Exact generation used by the invocation.
    pub generation: CompiledGenerationDigest,
    /// Evidence available at use time; later evidence cannot replace it.
    pub evidence_at_use: Vec<ImmutableRecordReferenceV1>,
    /// Exact use disposition.
    pub disposition: PackageUseDisposition,
}

/// Package-use claim structurally joined to its exact occurrence and generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPackageUse(PackageUse);

impl AdmittedPackageUse {
    /// Borrow the structurally admitted Package-use claim.
    #[must_use]
    pub fn record(&self) -> &PackageUse {
        &self.0
    }
}

/// A Package-use claim disagrees with its exact occurrence.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageUseError {
    /// Use names another occurrence.
    #[error("Package use does not name the supplied occurrence")]
    OccurrenceMismatch,
    /// Use names another compiled generation.
    #[error("Package use does not name the occurrence's compiled generation")]
    GenerationMismatch,
    /// Use names an operation outside the occurrence's narrowed ceiling.
    #[error("Package use names an operation outside the occurrence ceiling")]
    OperationNotSelected,
    /// Use activation differs from the exact supplied activation.
    #[error("Package use activation does not match the supplied activation")]
    ActivationMismatch,
    /// Evidence-at-use exceeded its finite bound.
    #[error("Package use evidence contains {size} members; maximum is 4096")]
    EvidenceTooLarge {
        /// Observed member count.
        size: usize,
    },
    /// Evidence-at-use must be strictly ordered and duplicate-free.
    #[error("Package use evidence is not in canonical strict order")]
    NonCanonicalEvidence,
}

/// Structurally admit an immutable use claim against one exact occurrence.
///
/// The function reads no ambient role, provider, evidence, or session state, so
/// simultaneous uses of one release cannot borrow authority from one another.
///
/// # Errors
///
/// Returns [`PackageUseError`] when the occurrence, generation, operation, or
/// optional activation disagrees with the admitted inputs, or when evidence is
/// not within its canonical finite bound.
pub fn admit_package_use(
    occurrence: &AdmittedPackageOccurrence,
    activation: Option<&AdmittedPackageActivation>,
    evidence: PackageUse,
) -> Result<AdmittedPackageUse, PackageUseError> {
    let occurrence = occurrence.record();
    if evidence.occurrence != occurrence.occurrence {
        return Err(PackageUseError::OccurrenceMismatch);
    }
    if evidence.generation != occurrence.generation {
        return Err(PackageUseError::GenerationMismatch);
    }
    if !occurrence.ceilings.operations.contains(&evidence.operation) {
        return Err(PackageUseError::OperationNotSelected);
    }
    let admitted_activation = activation.map(|activation| &activation.record().activation);
    if evidence.activation.as_ref() != admitted_activation
        || activation.is_some_and(|activation| activation.occurrence() != &occurrence.occurrence)
    {
        return Err(PackageUseError::ActivationMismatch);
    }
    if evidence.evidence_at_use.len() > MAX_EVIDENCE_AXIS_MEMBERS {
        return Err(PackageUseError::EvidenceTooLarge {
            size: evidence.evidence_at_use.len(),
        });
    }
    if !evidence
        .evidence_at_use
        .windows(2)
        .all(|pair| pair[0] < pair[1])
    {
        return Err(PackageUseError::NonCanonicalEvidence);
    }
    Ok(AdmittedPackageUse(evidence))
}

/// Claimed native effect kept separate from completed Package use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageNativeEffect {
    /// Immutable native-effect record identity.
    pub effect_record: ImmutableRecordReferenceV1,
    /// Exact use whose invocation requested the effect.
    pub use_record: ImmutableRecordReferenceV1,
    /// Reference claimed to contain owner-native effect/read-back evidence.
    pub native_effect: ImmutableRecordReferenceV1,
    /// Native owner claimed to have produced the read-back evidence.
    pub owner: WorkspaceResourceId,
    /// Effect class claimed by the referenced owner evidence.
    pub effect: EffectClass,
}

/// Native-effect claim structurally joined to one exact use and occurrence ceiling.
///
/// Admission checks identity and ceiling membership only; it does not
/// authenticate the owner or referenced read-back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPackageNativeEffect(PackageNativeEffect);

impl AdmittedPackageNativeEffect {
    /// Borrow the structurally admitted native-effect claim.
    #[must_use]
    pub fn record(&self) -> &PackageNativeEffect {
        &self.0
    }
}

/// Native-effect evidence violates the exact use or occurrence ceiling.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageNativeEffectError {
    /// Supplied use was admitted for another occurrence.
    #[error("Package use was admitted for another occurrence")]
    UseOccurrenceMismatch,
    /// Effect evidence names another Package use.
    #[error("native effect does not name the supplied Package use")]
    UseMismatch,
    /// Admission denied the use, so no native effect may descend from it.
    #[error("a denied Package use cannot produce an admitted native effect")]
    DeniedUse,
    /// Owner-native evidence establishes an effect outside the occurrence ceiling.
    #[error("native effect lies outside the occurrence effect ceiling")]
    EffectOutsideCeiling,
}

/// Structurally admit a claimed owner-native effect without converting it into settlement.
///
/// # Errors
///
/// Returns [`PackageNativeEffectError`] when use identity or effect ceiling does
/// not match the supplied admitted records.
pub fn admit_package_native_effect(
    occurrence: &AdmittedPackageOccurrence,
    package_use: &AdmittedPackageUse,
    candidate: PackageNativeEffect,
) -> Result<AdmittedPackageNativeEffect, PackageNativeEffectError> {
    if package_use.record().occurrence != occurrence.record().occurrence
        || package_use.record().generation != occurrence.record().generation
    {
        return Err(PackageNativeEffectError::UseOccurrenceMismatch);
    }
    if candidate.use_record != package_use.record().use_record {
        return Err(PackageNativeEffectError::UseMismatch);
    }
    if package_use.record().disposition == PackageUseDisposition::Denied {
        return Err(PackageNativeEffectError::DeniedUse);
    }
    if !occurrence
        .record()
        .ceilings
        .effects
        .contains(&candidate.effect)
    {
        return Err(PackageNativeEffectError::EffectOutsideCeiling);
    }
    Ok(AdmittedPackageNativeEffect(candidate))
}

/// Typed settlement claim kept separate from use and native effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageSettlement {
    /// Exact occurrence whose obligation was classified.
    pub occurrence: ImmutableRecordReferenceV1,
    /// Immutable settlement record.
    pub settlement: ImmutableRecordReferenceV1,
    /// Exact compiled generation whose obligation was classified.
    pub generation: CompiledGenerationDigest,
    /// Exact Package obligation classified.
    pub obligation: ObligationSubject,
    /// Optional exact use participating in settlement.
    pub use_record: Option<ImmutableRecordReferenceV1>,
    /// Immutable evidence-settlement input record.
    pub evidence: ImmutableRecordReferenceV1,
    /// State and residual action established at settlement time.
    pub decision: SettlementDecision,
}

/// Settlement claim structurally joined to one exact occurrence and optional use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPackageSettlement {
    record: PackageSettlement,
    native_effect: Option<ImmutableRecordReferenceV1>,
}

impl AdmittedPackageSettlement {
    /// Borrow the structurally admitted settlement claim.
    #[must_use]
    pub fn record(&self) -> &PackageSettlement {
        &self.record
    }

    /// Borrow the exact native-effect identity retained by settlement admission.
    #[must_use]
    pub(super) fn native_effect(&self) -> Option<&ImmutableRecordReferenceV1> {
        self.native_effect.as_ref()
    }
}

/// Settlement violates occurrence, generation, operation, or use identity.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageSettlementError {
    /// Settlement names another occurrence.
    #[error("Package settlement does not name the supplied occurrence")]
    OccurrenceMismatch,
    /// Settlement names another generation.
    #[error("Package settlement does not name the occurrence generation")]
    GenerationMismatch,
    /// Settlement obligation names an operation outside the occurrence ceiling.
    #[error("Package settlement obligation lies outside the occurrence ceiling")]
    OperationOutsideCeiling,
    /// Settlement's optional use coordinate disagrees with the supplied use.
    #[error("Package settlement use coordinate does not match the supplied use")]
    UseMismatch,
    /// Settlement does not name the supplied evidence-input record.
    #[error("Package settlement does not name the supplied evidence input")]
    EvidenceMismatch,
    /// Settlement input does not describe the same obligation and generation.
    #[error("Package settlement input disagrees with its occurrence or obligation")]
    SettlementInputMismatch,
    /// Stored state differs from the canonical settlement decision.
    #[error("Package settlement decision differs from the canonical state-machine result")]
    DecisionMismatch,
    /// Native-effect settlement lacks the exact admitted native effect.
    #[error("Package settlement native-effect evidence does not form an exact join")]
    NativeEffectMismatch,
}

/// Structurally admit settlement without conflating it with use or native effect.
///
/// # Errors
///
/// Returns [`PackageSettlementError`] when any exact occurrence, generation,
/// operation, or use join disagrees.
pub fn admit_package_settlement(
    occurrence: &AdmittedPackageOccurrence,
    package_use: Option<&AdmittedPackageUse>,
    native_effect: Option<&AdmittedPackageNativeEffect>,
    evidence_ref: &ImmutableRecordReferenceV1,
    input: &EvidenceSettlementInput,
    checked_at: UnixTimeV1,
    candidate: PackageSettlement,
) -> Result<AdmittedPackageSettlement, PackageSettlementError> {
    let occurrence = occurrence.record();
    if candidate.occurrence != occurrence.occurrence {
        return Err(PackageSettlementError::OccurrenceMismatch);
    }
    if candidate.generation != occurrence.generation {
        return Err(PackageSettlementError::GenerationMismatch);
    }
    if !occurrence
        .ceilings
        .operations
        .contains(&candidate.obligation.operation)
    {
        return Err(PackageSettlementError::OperationOutsideCeiling);
    }
    if candidate.evidence != *evidence_ref {
        return Err(PackageSettlementError::EvidenceMismatch);
    }
    if input.generation != candidate.generation || input.obligation != candidate.obligation {
        return Err(PackageSettlementError::SettlementInputMismatch);
    }
    let supplied_use = package_use.map(|package_use| &package_use.record().use_record);
    if candidate.use_record.as_ref() != supplied_use {
        return Err(PackageSettlementError::UseMismatch);
    }
    if let Some(package_use) = package_use {
        let package_use = package_use.record();
        if package_use.occurrence != occurrence.occurrence
            || package_use.generation != occurrence.generation
            || package_use.operation != candidate.obligation.operation
        {
            return Err(PackageSettlementError::UseMismatch);
        }
    }
    match (&input.target, &input.native_verification, native_effect) {
        (
            SettlementTarget::NativeEffect { effect, owner },
            NativeVerification::Verified {
                owner: verifying_owner,
                evidence,
            }
            | NativeVerification::Denied {
                owner: verifying_owner,
                evidence,
            }
            | NativeVerification::Failed {
                owner: verifying_owner,
                evidence,
            }
            | NativeVerification::EffectUnknown {
                owner: verifying_owner,
                evidence,
            },
            Some(native_effect),
        ) if owner == verifying_owner
            && native_effect.record().owner == *owner
            && native_effect.record().effect == *effect
            && native_effect.record().native_effect == *evidence => {}
        (
            SettlementTarget::NativeEffect { owner, .. },
            NativeVerification::Pending {
                owner: pending_owner,
            },
            None,
        ) if owner == pending_owner => {}
        (SettlementTarget::NativeEffect { .. }, _, _) => {
            return Err(PackageSettlementError::NativeEffectMismatch);
        }
        (_, NativeVerification::NotApplicable, None) => {}
        _ => return Err(PackageSettlementError::NativeEffectMismatch),
    }
    if settle_obligation(input, checked_at) != candidate.decision {
        return Err(PackageSettlementError::DecisionMismatch);
    }
    Ok(AdmittedPackageSettlement {
        record: candidate,
        native_effect: native_effect.map(|effect| effect.record().effect_record.clone()),
    })
}
