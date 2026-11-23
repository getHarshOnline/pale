//! Admit one historical principal control-checkpoint chain.
//!
//! Physical custody supplies bounded exact bytes from the admitted bootstrap
//! checkpoint through one selected store head. Independent resolvers provide
//! the source-supported checkpoint profile and each record-time signer. A
//! source-specific verifier proves accumulator construction because the store
//! does not retain a generic historical mutation batch for every transition.
//! This module does not infer that proof from the current accumulator state.
//! It does not select a current signer, admit deployment genesis, compare an
//! independent anchor, open a store for writes, or make a resident ready.

use pale_core::canon;
use pale_core::records::{
    CanonicalSignatureV1, CheckpointProfileDigest, CheckpointSigningAuthorityDigest,
    CheckpointSigningAuthorityV1, ControlCheckpointDomainIdV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, ControlIncarnationIdV1,
    ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1, ControlStateCheckpointV1,
    ControlStateCheckpointWireV1, ControlStoreIdV1, Ed25519PublicKeyV1, InvalidResidentRecordV1,
    PrincipalAuthorityStateV1, ResidentKeyIdV1, ResidentPrincipalIdV1, ResidentRecordRefV1,
    SchemaDigest, SigningKeyEpochV1, UnixTimeV1,
};
use pale_ports::{
    ControlRecord, ControlRecordCoordinate, PrincipalControlCheckpointChainReadLimits,
    PrincipalControlCheckpointChainSnapshot,
};
use serde::Deserialize;

use super::{principal_checkpoint::SupportedPrincipalTrustCheckpointProfile, record_projection};

/// Result of resolving one immutable checkpoint-profile coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) enum HistoricalControlCheckpointProfileResolution {
    /// Exact source bytes for the requested reference and digest.
    Resolved(Box<[u8]>),
    /// No source evidence exists for the exact coordinate.
    Unavailable,
    /// Available source evidence assigns unlike bytes to the coordinate.
    Conflicting,
}

/// Resolve source-owned checkpoint-profile bytes without using checkpoint data as authority.
pub(in crate::resident) trait HistoricalControlCheckpointProfileResolver {
    /// Resolve one exact immutable profile coordinate.
    fn resolve_historical_control_checkpoint_profile(
        &mut self,
        reference: &ResidentRecordRefV1,
        digest: &CheckpointProfileDigest,
    ) -> HistoricalControlCheckpointProfileResolution;
}

impl<F> HistoricalControlCheckpointProfileResolver for F
where
    F: FnMut(
        &ResidentRecordRefV1,
        &CheckpointProfileDigest,
    ) -> HistoricalControlCheckpointProfileResolution,
{
    fn resolve_historical_control_checkpoint_profile(
        &mut self,
        reference: &ResidentRecordRefV1,
        digest: &CheckpointProfileDigest,
    ) -> HistoricalControlCheckpointProfileResolution {
        self(reference, digest)
    }
}

/// One independently resolved historical checkpoint signer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct ResolvedHistoricalCheckpointSigner {
    reference: ResidentRecordRefV1,
    authority: CheckpointSigningAuthorityV1,
}

impl ResolvedHistoricalCheckpointSigner {
    /// Bind an immutable authority reference to its admitted candidate record.
    #[must_use]
    pub(in crate::resident) const fn new(
        reference: ResidentRecordRefV1,
        authority: CheckpointSigningAuthorityV1,
    ) -> Self {
        Self {
            reference,
            authority,
        }
    }
}

/// Result of resolving the exact signer named by a historical checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) enum HistoricalCheckpointSignerResolution {
    /// One authority record resolves the requested coordinate.
    Resolved(Box<ResolvedHistoricalCheckpointSigner>),
    /// No authority evidence exists for the requested coordinate.
    Unavailable,
    /// Available evidence assigns unlike authorities to the coordinate.
    Conflicting,
}

/// Resolve and scope record-time checkpoint signers from admitted trust history.
pub(in crate::resident) trait HistoricalCheckpointSignerResolver {
    /// Resolve the exact authority coordinate named by the checkpoint.
    fn resolve_historical_checkpoint_signer(
        &mut self,
        reference: &ResidentRecordRefV1,
        digest: &CheckpointSigningAuthorityDigest,
    ) -> HistoricalCheckpointSignerResolution;

    /// Establish that the authority's delegated prefix set covers this domain.
    fn signer_covers_domain(
        &mut self,
        authority: &CheckpointSigningAuthorityV1,
        domain: &ControlCheckpointDomainIdV1,
    ) -> bool;
}

/// Verify one control-checkpoint signature with a resolved historical key.
pub(in crate::resident) trait HistoricalControlCheckpointSignatureVerifier {
    /// Authenticate the exact canonical preimage and digest.
    fn verify_historical_control_checkpoint_signature(
        &mut self,
        domain: &[u8],
        public_key: &Ed25519PublicKeyV1,
        preimage: &ControlStateCheckpointPreimageV1,
        digest: &ControlStateCheckpointDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;
}

impl<F> HistoricalControlCheckpointSignatureVerifier for F
where
    F: FnMut(
        &[u8],
        &Ed25519PublicKeyV1,
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    fn verify_historical_control_checkpoint_signature(
        &mut self,
        domain: &[u8],
        public_key: &Ed25519PublicKeyV1,
        preimage: &ControlStateCheckpointPreimageV1,
        digest: &ControlStateCheckpointDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self(domain, public_key, preimage, digest, signature)
    }
}

/// Outcome from the source owner that constructed an accumulator checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::resident) enum HistoricalAccumulatorConstructionResolution {
    /// Source evidence proves the exact checkpoint construction.
    Verified,
    /// Required source evidence is absent.
    Unavailable,
    /// Available source evidence contradicts this checkpoint construction.
    Conflicting,
}

/// Source-specific proof boundary for historical accumulator construction.
///
/// The physical store has exact checkpoint bytes but no generic mutation-batch
/// history. Implementations must verify the actual bootstrap or producer
/// transition evidence. They must not substitute the current accumulator root.
pub(in crate::resident) trait HistoricalAccumulatorConstructionVerifier {
    /// Verify one initial or successor construction against its exact profile.
    fn verify_historical_accumulator_construction(
        &mut self,
        profile: &SupportedPrincipalTrustCheckpointProfile,
        predecessor: Option<(&ResidentRecordRefV1, &ControlStateCheckpointV1)>,
        reference: &ResidentRecordRefV1,
        checkpoint: &ControlStateCheckpointV1,
    ) -> HistoricalAccumulatorConstructionResolution;
}

impl<F> HistoricalAccumulatorConstructionVerifier for F
where
    F: FnMut(
        &SupportedPrincipalTrustCheckpointProfile,
        Option<(&ResidentRecordRefV1, &ControlStateCheckpointV1)>,
        &ResidentRecordRefV1,
        &ControlStateCheckpointV1,
    ) -> HistoricalAccumulatorConstructionResolution,
{
    fn verify_historical_accumulator_construction(
        &mut self,
        profile: &SupportedPrincipalTrustCheckpointProfile,
        predecessor: Option<(&ResidentRecordRefV1, &ControlStateCheckpointV1)>,
        reference: &ResidentRecordRefV1,
        checkpoint: &ControlStateCheckpointV1,
    ) -> HistoricalAccumulatorConstructionResolution {
        self(profile, predecessor, reference, checkpoint)
    }
}

/// One fully admitted historical checkpoint and its stored coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct RecoveredPrincipalControlCheckpoint {
    coordinate: ControlRecordCoordinate,
    checkpoint: ControlStateCheckpointV1,
}

impl RecoveredPrincipalControlCheckpoint {
    /// Borrow the immutable coordinate reproduced from the stored checkpoint.
    #[must_use]
    pub(in crate::resident) const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Borrow the admitted checkpoint at this coordinate.
    #[must_use]
    pub(in crate::resident) const fn checkpoint(&self) -> &ControlStateCheckpointV1 {
        &self.checkpoint
    }
}

/// Historical control-checkpoint chain closed against one physical current head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct RecoveredPrincipalControlCheckpointChain {
    evidence: PrincipalControlCheckpointChainSnapshot,
    profile: SupportedPrincipalTrustCheckpointProfile,
    checkpoints: Box<[RecoveredPrincipalControlCheckpoint]>,
}

impl RecoveredPrincipalControlCheckpointChain {
    /// Borrow the exact physical evidence that was admitted.
    #[must_use]
    pub(in crate::resident) const fn evidence(&self) -> &PrincipalControlCheckpointChainSnapshot {
        &self.evidence
    }

    /// Borrow the independently resolved source-supported profile.
    #[must_use]
    pub(in crate::resident) const fn profile(&self) -> &SupportedPrincipalTrustCheckpointProfile {
        &self.profile
    }

    /// Borrow checkpoints in semantic domain-sequence order.
    #[must_use]
    pub(in crate::resident) const fn checkpoints(&self) -> &[RecoveredPrincipalControlCheckpoint] {
        &self.checkpoints
    }
}

/// Why physical control-checkpoint evidence cannot be admitted as one chain.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum PrincipalControlCheckpointChainRecoveryError {
    /// The selected evidence exceeds its count or aggregate-byte limit.
    #[error("principal control-checkpoint chain exceeds its recovery bound")]
    BoundExceeded,
    /// Stored bytes are not strict canonical JSON.
    #[error("stored principal control checkpoint is not canonical JSON")]
    StoredRecordSyntax(#[source] canon::CanonError),
    /// Stored JSON does not decode as a control-checkpoint wire.
    #[error("stored principal control checkpoint has the wrong wire shape")]
    StoredRecordDecode(#[source] serde_json::Error),
    /// Checkpoint shape, schema, lineage, context, or signature was rejected.
    #[error(transparent)]
    Record(#[from] InvalidResidentRecordV1),
    /// Source profile evidence is absent.
    #[error("historical checkpoint profile is unavailable")]
    ProfileUnavailable,
    /// Source profile evidence conflicts at one coordinate.
    #[error("historical checkpoint profile evidence conflicts")]
    ProfileConflict,
    /// The source bytes do not admit the selected supported profile.
    #[error("historical checkpoint profile is unsupported")]
    UnsupportedProfile,
    /// Signer evidence is absent for one checkpoint.
    #[error("historical checkpoint signer is unavailable")]
    SignerUnavailable,
    /// Signer evidence conflicts at one coordinate.
    #[error("historical checkpoint signer evidence conflicts")]
    SignerConflict,
    /// The resolved signer does not cover the checkpoint context and time.
    #[error("historical checkpoint signer does not admit this checkpoint")]
    SignerContext,
    /// Accumulator construction evidence is absent.
    #[error("historical accumulator construction evidence is unavailable")]
    ConstructionUnavailable,
    /// Accumulator construction evidence contradicts this checkpoint.
    #[error("historical accumulator construction evidence conflicts")]
    ConstructionConflict,
    /// A checkpoint does not reproduce its exact stored record.
    #[error("admitted control checkpoint does not reproduce stored bytes")]
    StoredRecordMismatch,
    /// Domain sequences or physical transaction stamps do not form one chain.
    #[error("principal control checkpoints do not form one complete physical chain")]
    Chain,
    /// The recovered final checkpoint differs from the selected physical head.
    #[error("principal control-checkpoint chain does not end at the selected current head")]
    CurrentHead,
}

/// Admit bounded physical evidence from bootstrap through the current head.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn recover_principal_control_checkpoint_chain(
    evidence: PrincipalControlCheckpointChainSnapshot,
    admitted_initial_ref: &ResidentRecordRefV1,
    admitted_initial: &ControlStateCheckpointV1,
    expected_schema_sha256: &SchemaDigest,
    limits: PrincipalControlCheckpointChainReadLimits,
    profile_resolver: &mut dyn HistoricalControlCheckpointProfileResolver,
    signer_resolver: &mut dyn HistoricalCheckpointSignerResolver,
    signature_verifier: &mut dyn HistoricalControlCheckpointSignatureVerifier,
    construction_verifier: &mut dyn HistoricalAccumulatorConstructionVerifier,
) -> Result<RecoveredPrincipalControlCheckpointChain, PrincipalControlCheckpointChainRecoveryError>
{
    if evidence.records().len() > limits.maximum_checkpoints()
        || evidence
            .records()
            .iter()
            .try_fold(0usize, |total, record| {
                total.checked_add(record.record().canonical_record_jcs().as_slice().len())
            })
            .is_none_or(|total| total > limits.maximum_record_bytes())
    {
        return Err(PrincipalControlCheckpointChainRecoveryError::BoundExceeded);
    }
    admitted_initial.verify()?;
    let profile_bytes = match profile_resolver.resolve_historical_control_checkpoint_profile(
        admitted_initial.checkpoint_profile_ref(),
        admitted_initial.checkpoint_profile_sha256(),
    ) {
        HistoricalControlCheckpointProfileResolution::Resolved(bytes) => bytes,
        HistoricalControlCheckpointProfileResolution::Unavailable => {
            return Err(PrincipalControlCheckpointChainRecoveryError::ProfileUnavailable);
        }
        HistoricalControlCheckpointProfileResolution::Conflicting => {
            return Err(PrincipalControlCheckpointChainRecoveryError::ProfileConflict);
        }
    };
    let profile = SupportedPrincipalTrustCheckpointProfile::admit(
        admitted_initial.checkpoint_profile_ref().clone(),
        admitted_initial.checkpoint_profile_sha256(),
        &profile_bytes,
    )
    .map_err(|_| PrincipalControlCheckpointChainRecoveryError::UnsupportedProfile)?;
    if !profile.supports_checkpoint(admitted_initial) {
        return Err(PrincipalControlCheckpointChainRecoveryError::UnsupportedProfile);
    }

    let mut candidates = Vec::with_capacity(evidence.records().len());
    for (index, physical) in evidence.records().iter().enumerate() {
        let bytes = physical.record().canonical_record_jcs().as_slice();
        let value = canon::parse_strict(bytes)
            .map_err(PrincipalControlCheckpointChainRecoveryError::StoredRecordSyntax)?;
        if canon::to_canonical_string(&value)
            .map_err(PrincipalControlCheckpointChainRecoveryError::StoredRecordSyntax)?
            .as_bytes()
            != bytes
        {
            return Err(PrincipalControlCheckpointChainRecoveryError::StoredRecordMismatch);
        }
        let order: StoredControlCheckpointOrder = serde_json::from_value(value.clone())
            .map_err(PrincipalControlCheckpointChainRecoveryError::StoredRecordDecode)?;
        let authority: StoredControlCheckpointAuthority = serde_json::from_value(value.clone())
            .map_err(PrincipalControlCheckpointChainRecoveryError::StoredRecordDecode)?;
        let wire = serde_json::from_value(value)
            .map_err(PrincipalControlCheckpointChainRecoveryError::StoredRecordDecode)?;
        candidates.push(StoredControlCheckpointCandidate {
            index,
            order,
            authority,
            wire,
        });
    }
    candidates.sort_by_key(|candidate| candidate.order.domain_sequence);
    if candidates.windows(2).any(|pair| {
        pair[0].order.domain_sequence.checked_successor() != Some(pair[1].order.domain_sequence)
    }) {
        return Err(PrincipalControlCheckpointChainRecoveryError::Chain);
    }

    let mut recovered = Vec::with_capacity(candidates.len());
    let mut previous_inserted = None;
    for candidate in candidates {
        let physical = &evidence.records()[candidate.index];
        let predecessor = recovered.last();
        let checkpoint = admit_checkpoint(
            candidate.wire,
            &candidate.authority,
            predecessor,
            expected_schema_sha256,
            signer_resolver,
            signature_verifier,
        )?;
        if checkpoint.issuer_principal_id() != evidence.binding().principal_id()
            || !profile.supports_checkpoint(&checkpoint)
        {
            return Err(PrincipalControlCheckpointChainRecoveryError::UnsupportedProfile);
        }
        if physical.inserted_in() != *checkpoint.committed_transaction_sequence()
            || previous_inserted.is_some_and(|previous| physical.inserted_in() <= previous)
        {
            return Err(PrincipalControlCheckpointChainRecoveryError::Chain);
        }
        require_exact_stored_record(physical.record(), &checkpoint)?;
        if recovered.is_empty()
            && (physical.record().coordinate() != evidence.initial().coordinate()
                || physical.record().coordinate().record_ref() != admitted_initial_ref
                || checkpoint.checkpoint_domain_id() != evidence.initial().checkpoint_domain_id()
                || checkpoint.domain_epoch() != &evidence.initial().domain_epoch()
                || checkpoint.domain_sequence() != &evidence.initial().domain_sequence()
                || checkpoint.committed_transaction_sequence()
                    != &evidence.initial().committed_transaction_sequence()
                || checkpoint != *admitted_initial)
        {
            return Err(PrincipalControlCheckpointChainRecoveryError::Chain);
        }
        let construction = construction_verifier.verify_historical_accumulator_construction(
            &profile,
            recovered
                .last()
                .map(|prior| (prior.coordinate.record_ref(), &prior.checkpoint)),
            physical.record().coordinate().record_ref(),
            &checkpoint,
        );
        match construction {
            HistoricalAccumulatorConstructionResolution::Verified => {}
            HistoricalAccumulatorConstructionResolution::Unavailable => {
                return Err(PrincipalControlCheckpointChainRecoveryError::ConstructionUnavailable);
            }
            HistoricalAccumulatorConstructionResolution::Conflicting => {
                return Err(PrincipalControlCheckpointChainRecoveryError::ConstructionConflict);
            }
        }
        recovered.push(RecoveredPrincipalControlCheckpoint {
            coordinate: physical.record().coordinate().clone(),
            checkpoint,
        });
        previous_inserted = Some(physical.inserted_in());
    }
    let current = recovered
        .last()
        .ok_or(PrincipalControlCheckpointChainRecoveryError::Chain)?;
    if current.coordinate != *evidence.through().coordinate()
        || current.checkpoint.checkpoint_domain_id() != evidence.through().checkpoint_domain_id()
        || *current.checkpoint.domain_epoch() != evidence.through().domain_epoch()
        || *current.checkpoint.domain_sequence() != evidence.through().domain_sequence()
        || *current.checkpoint.committed_transaction_sequence()
            != evidence.through().committed_transaction_sequence()
    {
        return Err(PrincipalControlCheckpointChainRecoveryError::CurrentHead);
    }
    Ok(RecoveredPrincipalControlCheckpointChain {
        evidence,
        profile,
        checkpoints: recovered.into_boxed_slice(),
    })
}

fn admit_checkpoint(
    wire: ControlStateCheckpointWireV1,
    preimage: &StoredControlCheckpointAuthority,
    predecessor: Option<&RecoveredPrincipalControlCheckpoint>,
    expected_schema_sha256: &SchemaDigest,
    signer_resolver: &mut dyn HistoricalCheckpointSignerResolver,
    signature_verifier: &mut dyn HistoricalControlCheckpointSignatureVerifier,
) -> Result<ControlStateCheckpointV1, PrincipalControlCheckpointChainRecoveryError> {
    let resolved = match signer_resolver.resolve_historical_checkpoint_signer(
        &preimage.checkpoint_signing_authority_ref,
        &preimage.checkpoint_signing_authority_sha256,
    ) {
        HistoricalCheckpointSignerResolution::Resolved(resolved) => resolved,
        HistoricalCheckpointSignerResolution::Unavailable => {
            return Err(PrincipalControlCheckpointChainRecoveryError::SignerUnavailable);
        }
        HistoricalCheckpointSignerResolution::Conflicting => {
            return Err(PrincipalControlCheckpointChainRecoveryError::SignerConflict);
        }
    };
    let authority = &resolved.authority;
    authority.verify()?;
    let exact_context = resolved.reference == preimage.checkpoint_signing_authority_ref
        && authority.canonical_checkpoint_signing_authority_sha256()
            == &preimage.checkpoint_signing_authority_sha256
        && authority.principal_id() == &preimage.issuer_principal_id
        && authority.control_store_id() == &preimage.control_store_id
        && authority.control_incarnation_id() == &preimage.control_incarnation_id
        && authority.checkpoint_profile_ref() == &preimage.checkpoint_profile_ref
        && authority.checkpoint_profile_sha256() == &preimage.checkpoint_profile_sha256
        && authority.subject_key_id() == &preimage.issuer_key_id
        && authority.subject_key_epoch() == &preimage.issuer_key_epoch
        && authority.state() == &PrincipalAuthorityStateV1::Active
        && authority.issued_at() <= &preimage.issued_at
        && &preimage.issued_at < authority.expires_at()
        && matches!(
            authority.scope(),
            pale_core::records::CheckpointSigningScopeV1::PrincipalTrustStore { .. }
        )
        && signer_resolver.signer_covers_domain(authority, &preimage.checkpoint_domain_id);
    if !exact_context {
        return Err(PrincipalControlCheckpointChainRecoveryError::SignerContext);
    }
    let context = ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: &resolved.reference,
        authority_sha256: authority.canonical_checkpoint_signing_authority_sha256(),
        principal_id: authority.principal_id(),
        control_store_id: authority.control_store_id(),
        control_incarnation_id: authority.control_incarnation_id(),
        checkpoint_domain_id: &preimage.checkpoint_domain_id,
        checkpoint_profile_ref: authority.checkpoint_profile_ref(),
        checkpoint_profile_sha256: authority.checkpoint_profile_sha256(),
        subject_key_id: authority.subject_key_id(),
        subject_key_epoch: *authority.subject_key_epoch(),
    };
    let verify = |domain: &[u8],
                  preimage: &ControlStateCheckpointPreimageV1,
                  digest: &ControlStateCheckpointDigest,
                  signature: &CanonicalSignatureV1| {
        signature_verifier.verify_historical_control_checkpoint_signature(
            domain,
            authority.subject_public_key(),
            preimage,
            digest,
            signature,
        )
    };
    match predecessor {
        None => ControlStateCheckpointV1::admit_initial_wire(
            wire,
            expected_schema_sha256,
            &context,
            verify,
        ),
        Some(predecessor) => ControlStateCheckpointV1::admit_successor_wire(
            wire,
            predecessor.coordinate.record_ref(),
            &predecessor.checkpoint,
            expected_schema_sha256,
            &context,
            verify,
        ),
    }
    .map_err(Into::into)
}

fn require_exact_stored_record(
    physical: &ControlRecord,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<(), PrincipalControlCheckpointChainRecoveryError> {
    if physical.coordinate().record_sha256().as_str()
        != checkpoint.canonical_control_checkpoint_sha256().as_str()
    {
        return Err(PrincipalControlCheckpointChainRecoveryError::StoredRecordMismatch);
    }
    let projected = record_projection::record(physical.coordinate().clone(), checkpoint)
        .map_err(|_| PrincipalControlCheckpointChainRecoveryError::StoredRecordMismatch)?;
    if projected == *physical {
        Ok(())
    } else {
        Err(PrincipalControlCheckpointChainRecoveryError::StoredRecordMismatch)
    }
}

#[derive(Deserialize)]
struct StoredControlCheckpointOrder {
    domain_sequence: pale_core::records::ControlDomainSequenceV1,
}

struct StoredControlCheckpointCandidate {
    index: usize,
    order: StoredControlCheckpointOrder,
    authority: StoredControlCheckpointAuthority,
    wire: ControlStateCheckpointWireV1,
}

#[derive(Deserialize)]
struct StoredControlCheckpointAuthority {
    checkpoint_signing_authority_ref: ResidentRecordRefV1,
    checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest,
    issuer_principal_id: ResidentPrincipalIdV1,
    issuer_key_id: ResidentKeyIdV1,
    issuer_key_epoch: SigningKeyEpochV1,
    control_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
    checkpoint_domain_id: ControlCheckpointDomainIdV1,
    checkpoint_profile_ref: ResidentRecordRefV1,
    checkpoint_profile_sha256: CheckpointProfileDigest,
    issued_at: UnixTimeV1,
}

#[cfg(test)]
#[path = "../../tests/internal/resident/principal_control_checkpoint_recovery.rs"]
pub(in crate::resident) mod tests;
