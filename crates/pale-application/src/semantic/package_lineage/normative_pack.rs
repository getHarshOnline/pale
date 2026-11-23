//! Body-free normative-Pack reference and use-claim structural admission.

use pale_core::records::{ImmutableRecordReferenceV1, NormativePackReleaseV1, UnixTimeV1};

use super::AdmittedPackageOccurrence;

/// Supplied references for independently distinguished normative-Pack rights.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormativePackUseRights {
    /// Context-use right reference.
    pub context: ImmutableRecordReferenceV1,
    /// Evaluation-use right reference.
    pub evaluation: ImmutableRecordReferenceV1,
    /// Optional training-use right reference.
    pub training: Option<ImmutableRecordReferenceV1>,
    /// Optional derivative-use right reference.
    pub derivative: Option<ImmutableRecordReferenceV1>,
    /// Optional genericization right reference.
    pub genericization: Option<ImmutableRecordReferenceV1>,
    /// Optional publication right reference.
    pub publication: Option<ImmutableRecordReferenceV1>,
}

impl NormativePackUseRights {
    fn evidence_for(
        &self,
        purpose: NormativePackUsePurpose,
    ) -> Option<&ImmutableRecordReferenceV1> {
        match purpose {
            NormativePackUsePurpose::Context => Some(&self.context),
            NormativePackUsePurpose::Evaluation => Some(&self.evaluation),
            NormativePackUsePurpose::Training => self.training.as_ref(),
            NormativePackUsePurpose::Derivative => self.derivative.as_ref(),
            NormativePackUsePurpose::Genericization => self.genericization.as_ref(),
            NormativePackUsePurpose::Publication => self.publication.as_ref(),
        }
    }
}

/// Supplied precedence disposition for otherwise conflicting Pack releases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackPrecedence {
    /// No conflicting applicable Pack is known.
    Uncontested,
    /// An exact owner-rule reference is supplied for the conflict.
    OwnerRuled {
        /// Immutable owner precedence decision.
        rule: ImmutableRecordReferenceV1,
    },
    /// Conflict remains explicit and blocks admission.
    Unresolved {
        /// Exact conflicting release.
        conflicting_release: NormativePackReleaseV1,
    },
}

/// Exact normative-Pack reference admission; no Pack body is stored here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormativePackReferenceAdmission {
    /// Immutable identity of this admission.
    pub admission: ImmutableRecordReferenceV1,
    /// Exact attestor-owned release coordinate.
    pub release: NormativePackReleaseV1,
    /// Supplied authority reference; authentication remains owner-native.
    pub authority: ImmutableRecordReferenceV1,
    /// Independently typed, supplied use-right references.
    pub rights: NormativePackUseRights,
    /// Semantic and jurisdictional scope.
    pub scope: ImmutableRecordReferenceV1,
    /// Inclusive effective start.
    pub effective_from: UnixTimeV1,
    /// Exclusive effective end, when one exists.
    pub effective_until: Option<UnixTimeV1>,
    /// Explicit precedence/conflict disposition.
    pub precedence: PackPrecedence,
    /// Optional exact predecessor release.
    pub supersedes: Option<NormativePackReleaseV1>,
    /// Supplied owner revocation-policy reference.
    pub revocation: ImmutableRecordReferenceV1,
    /// Supplied evidence-contract reference for a claimed Pack use.
    pub use_evidence: ImmutableRecordReferenceV1,
    /// Supplied owner decommission-contract reference.
    pub decommission: ImmutableRecordReferenceV1,
}

/// Normative-Pack reference structurally admitted against supplied exact
/// owner-contract references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedNormativePackReference(NormativePackReferenceAdmission);

impl AdmittedNormativePackReference {
    /// Borrow the exact body-free admission.
    #[must_use]
    pub fn record(&self) -> &NormativePackReferenceAdmission {
        &self.0
    }
}

/// Claimed normative-Pack use kept separate from reference admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormativePackUse {
    /// Immutable Pack-use record.
    pub use_record: ImmutableRecordReferenceV1,
    /// Exact admitted reference consumed.
    pub admission: ImmutableRecordReferenceV1,
    /// Exact Package occurrence consuming the Pack.
    pub occurrence: ImmutableRecordReferenceV1,
    /// Independently typed use class claimed by this occurrence.
    pub purpose: NormativePackUsePurpose,
    /// Exact right authorizing [`Self::purpose`].
    pub right: ImmutableRecordReferenceV1,
    /// Exact evidence contract admitted for use.
    pub evidence_contract: ImmutableRecordReferenceV1,
    /// Exact immutable evidence available at use time.
    pub evidence_at_use: ImmutableRecordReferenceV1,
    /// Use time checked against the admitted effective interval.
    pub used_at: UnixTimeV1,
}

/// Non-substitutable normative-Pack use classes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NormativePackUsePurpose {
    /// Supply bounded context.
    Context,
    /// Evaluate an admitted subject.
    Evaluation,
    /// Train or fine-tune a model.
    Training,
    /// Prepare derivative material.
    Derivative,
    /// Generalize source-specific material.
    Genericization,
    /// Publish material outside the admitted private boundary.
    Publication,
}

/// Normative-Pack use claim structurally admitted against exact selection,
/// rights, and interval.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedNormativePackUse(NormativePackUse);

impl AdmittedNormativePackUse {
    /// Borrow the admitted use occurrence.
    #[must_use]
    pub fn record(&self) -> &NormativePackUse {
        &self.0
    }
}

/// Pack use violates its exact reference admission or occurrence.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NormativePackUseError {
    /// Use names another admitted Pack reference.
    #[error("normative Pack use does not name the supplied admission")]
    AdmissionMismatch,
    /// Occurrence did not select the exact admitted Pack reference.
    #[error("Package occurrence did not select the supplied normative Pack admission")]
    OccurrenceMismatch,
    /// Use evidence differs from the owner-required evidence coordinate.
    #[error("normative Pack use evidence differs from the admitted owner requirement")]
    EvidenceMismatch,
    /// Use cites no right, or cites another use class's right.
    #[error("normative Pack use does not carry the exact right for its purpose")]
    RightMismatch,
    /// Use lies outside the admitted effective interval.
    #[error("normative Pack use lies outside its admitted effective interval")]
    OutsideEffectiveInterval,
}

/// Structurally admit one Pack-use claim without copying Pack substance.
///
/// This function structurally matches the occurrence, admission, right,
/// evidence-contract, and effective-interval coordinates. It neither
/// authenticates [`NormativePackUse::evidence_at_use`] nor resolves an owner
/// lifecycle head, so success proves neither actual use nor authority to begin
/// a new use.
///
/// # Errors
///
/// Returns [`NormativePackUseError`] when admission, occurrence, evidence, or
/// effective-period coordinates disagree.
pub fn admit_normative_pack_use(
    occurrence: &AdmittedPackageOccurrence,
    admission: &AdmittedNormativePackReference,
    candidate: NormativePackUse,
) -> Result<AdmittedNormativePackUse, NormativePackUseError> {
    let admission = admission.record();
    if candidate.admission != admission.admission {
        return Err(NormativePackUseError::AdmissionMismatch);
    }
    if candidate.occurrence != occurrence.record().occurrence
        || !occurrence
            .record()
            .normative_packs
            .iter()
            .any(|selected| selected.record() == admission)
    {
        return Err(NormativePackUseError::OccurrenceMismatch);
    }
    if candidate.evidence_contract != admission.use_evidence {
        return Err(NormativePackUseError::EvidenceMismatch);
    }
    if admission.rights.evidence_for(candidate.purpose) != Some(&candidate.right) {
        return Err(NormativePackUseError::RightMismatch);
    }
    if candidate.used_at < admission.effective_from
        || admission
            .effective_until
            .is_some_and(|end| candidate.used_at >= end)
    {
        return Err(NormativePackUseError::OutsideEffectiveInterval);
    }
    Ok(AdmittedNormativePackUse(candidate))
}

/// Normative-Pack reference cannot be admitted under its exact owner contract.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NormativePackReferenceError {
    /// Effective interval is empty or reversed.
    #[error("normative Pack effective interval must end after it begins")]
    InvalidEffectiveInterval,
    /// Supplied precedence leaves an applicable conflict unresolved.
    #[error("supplied normative Pack precedence leaves the conflict unresolved")]
    UnresolvedConflict,
    /// A supersession must retain the same Pack identity and attestor.
    #[error(
        "normative Pack supersession must name a different release of the same Pack and attestor"
    )]
    InvalidSupersession,
}

/// Admit an exact Pack reference without loading or copying Pack substance.
///
/// # Errors
///
/// Returns [`NormativePackReferenceError`] for a reversed/empty effective
/// interval, unresolved precedence conflict, or a supersession that changes
/// Pack identity or attestor. Selection, possession, or prior use supplies
/// neither authority nor rights. This structural admission records the exact
/// revocation and decommission contracts but does not authenticate an owner
/// lifecycle head. New-use fencing and decommission therefore remain closed
/// until an owner-published lifecycle interface can prove them completely.
pub fn admit_normative_pack_reference(
    candidate: NormativePackReferenceAdmission,
) -> Result<AdmittedNormativePackReference, NormativePackReferenceError> {
    if candidate
        .effective_until
        .is_some_and(|end| end <= candidate.effective_from)
    {
        return Err(NormativePackReferenceError::InvalidEffectiveInterval);
    }
    if matches!(candidate.precedence, PackPrecedence::Unresolved { .. }) {
        return Err(NormativePackReferenceError::UnresolvedConflict);
    }
    if candidate.supersedes.as_ref().is_some_and(|predecessor| {
        predecessor.id != candidate.release.id
            || predecessor.attestor != candidate.release.attestor
            || predecessor == &candidate.release
    }) {
        return Err(NormativePackReferenceError::InvalidSupersession);
    }
    Ok(AdmittedNormativePackReference(candidate))
}
