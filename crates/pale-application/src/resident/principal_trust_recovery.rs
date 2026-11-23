//! Recover one signed historical trust-checkpoint chain.
//!
//! Physical custody supplies exact bounded bytes through one selected head.
//! The epoch-one checkpoint arrives from the existing bootstrap admission.
//! This module matches that checkpoint to storage, verifies each successor with
//! its record-time principal key, and proves an unbroken predecessor chain. It
//! does not admit the checkpoint's referenced head sets or trust-frontier roots.
//! It also does not select a current signer, compare independent anchors, open
//! a store for mutation, or make a resident ready.

use pale_core::canon;
#[cfg(feature = "interface-test-support")]
use pale_core::records::ControlTransactionSequenceV1;
use pale_core::records::{
    CanonicalSignatureV1, Ed25519PublicKeyV1, InvalidResidentRecordV1, PrincipalKeyEpochV1,
    PrincipalTrustCheckpointDigest, PrincipalTrustCheckpointEpochV1,
    PrincipalTrustCheckpointPreimageV1, PrincipalTrustCheckpointV1, PrincipalTrustCheckpointWireV1,
    ResidentKeyIdV1, ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest,
};
use pale_ports::{
    ControlRecord, ControlRecordCoordinate, PrincipalTrustCheckpointHead,
    PrincipalTrustCheckpointSetReadLimits, PrincipalTrustCheckpointSetSnapshot,
};
use serde::Deserialize;

use super::record_projection;

/// Result of resolving the principal key named by one historical record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) enum HistoricalPrincipalKeyResolution {
    /// The exact public key for the requested principal, key ID, and epoch.
    Resolved(Ed25519PublicKeyV1),
    /// The resolver has no evidence for this exact key coordinate.
    Unavailable,
    /// Available evidence assigns more than one key to the same coordinate.
    Conflicting,
}

/// Narrow key-history boundary used only for record-time signature verification.
pub(in crate::resident) trait HistoricalPrincipalKeyResolver {
    /// Resolve one exact principal-key coordinate without selecting it as current.
    fn resolve_historical_principal_key(
        &mut self,
        principal_id: &ResidentPrincipalIdV1,
        key_id: &ResidentKeyIdV1,
        key_epoch: PrincipalKeyEpochV1,
    ) -> HistoricalPrincipalKeyResolution;
}

impl<F> HistoricalPrincipalKeyResolver for F
where
    F: FnMut(
        &ResidentPrincipalIdV1,
        &ResidentKeyIdV1,
        PrincipalKeyEpochV1,
    ) -> HistoricalPrincipalKeyResolution,
{
    fn resolve_historical_principal_key(
        &mut self,
        principal_id: &ResidentPrincipalIdV1,
        key_id: &ResidentKeyIdV1,
        key_epoch: PrincipalKeyEpochV1,
    ) -> HistoricalPrincipalKeyResolution {
        self(principal_id, key_id, key_epoch)
    }
}

/// Cryptographic check over one resolved historical principal key.
pub(in crate::resident) trait HistoricalPrincipalCheckpointSignatureVerifier {
    /// Verify the exact canonical checkpoint signature.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the signature is not valid for
    /// the supplied domain, preimage, digest, and resolved public key.
    fn verify_historical_checkpoint_signature(
        &mut self,
        domain: &[u8],
        public_key: &Ed25519PublicKeyV1,
        preimage: &PrincipalTrustCheckpointPreimageV1,
        digest: &PrincipalTrustCheckpointDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;
}

impl<F> HistoricalPrincipalCheckpointSignatureVerifier for F
where
    F: FnMut(
        &[u8],
        &Ed25519PublicKeyV1,
        &PrincipalTrustCheckpointPreimageV1,
        &PrincipalTrustCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    fn verify_historical_checkpoint_signature(
        &mut self,
        domain: &[u8],
        public_key: &Ed25519PublicKeyV1,
        preimage: &PrincipalTrustCheckpointPreimageV1,
        digest: &PrincipalTrustCheckpointDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self(domain, public_key, preimage, digest, signature)
    }
}

/// One signature-verified checkpoint and its exact immutable physical coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct RecoveredSignedPrincipalTrustCheckpoint {
    coordinate: ControlRecordCoordinate,
    checkpoint: PrincipalTrustCheckpointV1,
}

impl RecoveredSignedPrincipalTrustCheckpoint {
    /// Borrow the immutable stored coordinate reproduced by the checkpoint.
    #[must_use]
    pub(in crate::resident) const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Borrow the signature-verified checkpoint.
    #[must_use]
    pub(in crate::resident) const fn checkpoint(&self) -> &PrincipalTrustCheckpointV1 {
        &self.checkpoint
    }
}

/// Signed historical checkpoint chain ending at one physical current head.
///
/// Every entry has its record-time signature and generic predecessor relation
/// verified. The retained physical evidence preserves exact bytes and store
/// transaction sequences; those sequences increase with the signed epochs.
/// This value does not establish referenced-head or trust-root closure,
/// current-key status, anchor equality, writer ownership, deployment readiness,
/// or process health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::resident) struct RecoveredSignedPrincipalTrustCheckpointChain {
    evidence: PrincipalTrustCheckpointSetSnapshot,
    checkpoints: Box<[RecoveredSignedPrincipalTrustCheckpoint]>,
}

impl RecoveredSignedPrincipalTrustCheckpointChain {
    #[cfg(feature = "interface-test-support")]
    pub(in crate::resident) fn for_interface_conformance(
        checkpoint_ref: ResidentRecordRefV1,
        checkpoint: PrincipalTrustCheckpointV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        use pale_ports::{
            PrincipalTrustCheckpointHead, PrincipalTrustCheckpointRecord,
            PrincipalTrustCheckpointSetSnapshot, PrincipalTrustStoreBinding,
        };

        checkpoint.verify()?;
        let coordinate = record_projection::coordinate(
            &checkpoint_ref,
            checkpoint.canonical_trust_checkpoint_sha256().as_str(),
        )
        .map_err(|_| InvalidResidentRecordV1::ContextualAdmission {
            record: "interface conformance trust checkpoint",
            invariant: "projectable physical coordinate",
        })?;
        let record = record_projection::record(coordinate.clone(), &checkpoint).map_err(|_| {
            InvalidResidentRecordV1::ContextualAdmission {
                record: "interface conformance trust checkpoint",
                invariant: "projectable canonical record",
            }
        })?;
        let head = PrincipalTrustCheckpointHead::new(
            checkpoint.principal_id().clone(),
            *checkpoint.checkpoint_epoch(),
            coordinate.clone(),
        );
        let sequence = ControlTransactionSequenceV1::new(1)?;
        let evidence = PrincipalTrustCheckpointSetSnapshot::new(
            PrincipalTrustStoreBinding::new(
                checkpoint.trust_store_id().clone(),
                checkpoint.trust_store_incarnation_id().clone(),
                checkpoint.principal_id().clone(),
            ),
            head,
            sequence,
            vec![PrincipalTrustCheckpointRecord::new(sequence, record)].into_boxed_slice(),
        )
        .map_err(|_| InvalidResidentRecordV1::ContextualAdmission {
            record: "interface conformance trust checkpoint",
            invariant: "one exact physical snapshot",
        })?;
        Ok(Self {
            evidence,
            checkpoints: vec![RecoveredSignedPrincipalTrustCheckpoint {
                coordinate,
                checkpoint,
            }]
            .into_boxed_slice(),
        })
    }

    /// Borrow the exact physical evidence used for verification.
    #[must_use]
    pub(in crate::resident) const fn evidence(&self) -> &PrincipalTrustCheckpointSetSnapshot {
        &self.evidence
    }

    /// Borrow signed checkpoints from epoch one through the selected head.
    #[must_use]
    pub(in crate::resident) const fn checkpoints(
        &self,
    ) -> &[RecoveredSignedPrincipalTrustCheckpoint] {
        &self.checkpoints
    }

    /// Borrow the final signed checkpoint.
    #[must_use]
    pub(in crate::resident) fn current(&self) -> &RecoveredSignedPrincipalTrustCheckpoint {
        self.checkpoints
            .last()
            .expect("recovered signed checkpoint chain is non-empty by construction")
    }
}

/// Why stored evidence cannot be verified as one signed checkpoint chain.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum PrincipalTrustCheckpointChainRecoveryError {
    /// Stored bytes are not strict JSON.
    #[error("stored principal trust checkpoint is not strict JSON")]
    StoredRecordSyntax(#[source] canon::CanonError),
    /// Stored JSON is valid but not in canonical form.
    #[error("stored principal trust checkpoint is not canonical JSON")]
    NonCanonicalStoredRecord,
    /// Stored JSON does not decode as a trust-checkpoint wire.
    #[error("stored principal trust checkpoint has the wrong wire shape")]
    StoredRecordDecode(#[source] serde_json::Error),
    /// Generic record structure, digest, signature, or succession was rejected.
    #[error(transparent)]
    Record(#[from] InvalidResidentRecordV1),
    /// The supplied bootstrap checkpoint does not match epoch one in this store.
    #[error("admitted initial trust checkpoint does not match stored epoch one")]
    InitialCheckpoint,
    /// Historical key evidence is absent for one signed checkpoint.
    #[error("historical principal key is unavailable")]
    HistoricalKeyUnavailable,
    /// Historical evidence conflicts for one principal-key coordinate.
    #[error("historical principal key evidence conflicts")]
    HistoricalKeyConflict,
    /// A verified record did not reproduce its exact stored row.
    #[error("verified principal trust checkpoint does not reproduce stored bytes")]
    StoredRecordMismatch,
    /// Checkpoint epochs or predecessor coordinates do not form one chain.
    #[error("principal trust checkpoints do not form one complete chain")]
    Chain,
    /// Signed epoch order contradicts physical insertion order.
    #[error("principal trust checkpoint insertion order contradicts signed epoch order")]
    PhysicalTransactionOrder,
    /// The evidence exceeds the selected item or aggregate-byte ceiling.
    #[error("principal trust checkpoint chain exceeds its recovery bound")]
    BoundExceeded,
    /// The signed final checkpoint differs from the selected physical head.
    #[error("principal trust checkpoint chain does not end at the selected current head")]
    CurrentHead,
}

/// Verify bounded physical evidence as one signed historical checkpoint chain.
///
/// Epoch one arrives from bootstrap admission. For each successor, the key
/// resolver is asked for the exact record-time key coordinate. A resolved key
/// proves only that signature after the verifier accepts it. Current signer
/// selection and independent minima remain later cold-start checks.
///
/// # Errors
///
/// Rejects non-canonical or substituted bytes, unknown or conflicting key
/// history, failed signatures, missing or forked epochs, non-increasing
/// insertion transactions, a broken predecessor chain, and disagreement with
/// the selected physical current head.
pub(in crate::resident) fn recover_signed_principal_trust_checkpoint_chain(
    evidence: PrincipalTrustCheckpointSetSnapshot,
    initial_checkpoint_ref: &ResidentRecordRefV1,
    initial_checkpoint: &PrincipalTrustCheckpointV1,
    expected_schema_sha256: &SchemaDigest,
    limits: PrincipalTrustCheckpointSetReadLimits,
    key_resolver: &mut dyn HistoricalPrincipalKeyResolver,
    signature_verifier: &mut dyn HistoricalPrincipalCheckpointSignatureVerifier,
) -> Result<RecoveredSignedPrincipalTrustCheckpointChain, PrincipalTrustCheckpointChainRecoveryError>
{
    if evidence.checkpoint_records().len() > limits.maximum_checkpoints()
        || evidence
            .checkpoint_records()
            .iter()
            .try_fold(0usize, |total, record| {
                total.checked_add(record.record().canonical_record_jcs().as_slice().len())
            })
            .is_none_or(|total| total > limits.maximum_record_bytes())
    {
        return Err(PrincipalTrustCheckpointChainRecoveryError::BoundExceeded);
    }
    let binding = evidence.binding();
    initial_checkpoint.verify()?;
    if initial_checkpoint.checkpoint_epoch().get() != 1
        || initial_checkpoint.schema_sha256() != expected_schema_sha256
        || initial_checkpoint.principal_id() != binding.principal_id()
        || initial_checkpoint.trust_store_id() != binding.trust_store_id()
        || initial_checkpoint.trust_store_incarnation_id() != binding.trust_store_incarnation_id()
    {
        return Err(PrincipalTrustCheckpointChainRecoveryError::InitialCheckpoint);
    }
    let initial_coordinate = record_projection::coordinate(
        initial_checkpoint_ref,
        initial_checkpoint
            .canonical_trust_checkpoint_sha256()
            .as_str(),
    )
    .map_err(|_| PrincipalTrustCheckpointChainRecoveryError::InitialCheckpoint)?;
    let mut candidates = Vec::with_capacity(evidence.checkpoint_records().len());
    for (index, physical) in evidence.checkpoint_records().iter().enumerate() {
        let value = decode_canonical_value(physical.record())?;
        let order: StoredCheckpointOrder = serde_json::from_value(value.clone())
            .map_err(PrincipalTrustCheckpointChainRecoveryError::StoredRecordDecode)?;
        let wire = serde_json::from_value(value)
            .map_err(PrincipalTrustCheckpointChainRecoveryError::StoredRecordDecode)?;
        candidates.push(StoredCheckpointCandidate { index, order, wire });
    }
    candidates.sort_by_key(|candidate| candidate.order.checkpoint_epoch);
    if candidates.windows(2).any(|pair| {
        pair[0].order.checkpoint_epoch.checked_successor() != Some(pair[1].order.checkpoint_epoch)
    }) {
        return Err(PrincipalTrustCheckpointChainRecoveryError::Chain);
    }
    if candidates.windows(2).any(|pair| {
        evidence.checkpoint_records()[pair[0].index].inserted_in()
            >= evidence.checkpoint_records()[pair[1].index].inserted_in()
    }) {
        return Err(PrincipalTrustCheckpointChainRecoveryError::PhysicalTransactionOrder);
    }
    let Some(initial) = candidates.first() else {
        return Err(PrincipalTrustCheckpointChainRecoveryError::Chain);
    };
    let initial_physical = &evidence.checkpoint_records()[initial.index];
    if initial.order.checkpoint_epoch.get() != 1
        || initial_physical.record().coordinate() != &initial_coordinate
    {
        return Err(PrincipalTrustCheckpointChainRecoveryError::InitialCheckpoint);
    }
    require_exact_stored_record(initial_physical.record(), initial_checkpoint)?;
    let mut checkpoints = Vec::with_capacity(candidates.len());
    checkpoints.push(RecoveredSignedPrincipalTrustCheckpoint {
        coordinate: initial_coordinate,
        checkpoint: initial_checkpoint.clone(),
    });
    for candidate in candidates.into_iter().skip(1) {
        let predecessor = checkpoints
            .last()
            .ok_or(PrincipalTrustCheckpointChainRecoveryError::Chain)?;
        let mut resolution_failure = None;
        let verified_checkpoint = PrincipalTrustCheckpointV1::admit_successor_wire(
            candidate.wire,
            predecessor.coordinate.record_ref(),
            &predecessor.checkpoint,
            expected_schema_sha256,
            |_, _| Ok(()),
            |domain, preimage, digest, signature| {
                let public_key = match key_resolver.resolve_historical_principal_key(
                    &preimage.issuer_principal_id,
                    &preimage.issuer_key_id,
                    preimage.issuer_key_epoch,
                ) {
                    HistoricalPrincipalKeyResolution::Resolved(key) => key,
                    HistoricalPrincipalKeyResolution::Unavailable => {
                        resolution_failure = Some(
                            PrincipalTrustCheckpointChainRecoveryError::HistoricalKeyUnavailable,
                        );
                        return Err(key_resolution_rejected());
                    }
                    HistoricalPrincipalKeyResolution::Conflicting => {
                        resolution_failure =
                            Some(PrincipalTrustCheckpointChainRecoveryError::HistoricalKeyConflict);
                        return Err(key_resolution_rejected());
                    }
                };
                signature_verifier.verify_historical_checkpoint_signature(
                    domain,
                    &public_key,
                    preimage,
                    digest,
                    signature,
                )
            },
        );
        let verified_checkpoint = match verified_checkpoint {
            Ok(checkpoint) => checkpoint,
            Err(_) if resolution_failure.is_some() => {
                return Err(resolution_failure.expect("checked as present"));
            }
            Err(error) => return Err(error.into()),
        };
        let physical = &evidence.checkpoint_records()[candidate.index];
        require_exact_stored_record(physical.record(), &verified_checkpoint)?;
        checkpoints.push(RecoveredSignedPrincipalTrustCheckpoint {
            coordinate: physical.record().coordinate().clone(),
            checkpoint: verified_checkpoint,
        });
    }
    require_complete_checkpoint_chain(&checkpoints, evidence.through())?;
    Ok(RecoveredSignedPrincipalTrustCheckpointChain {
        evidence,
        checkpoints: checkpoints.into_boxed_slice(),
    })
}

#[derive(Deserialize)]
struct StoredCheckpointOrder {
    checkpoint_epoch: PrincipalTrustCheckpointEpochV1,
}

struct StoredCheckpointCandidate {
    index: usize,
    order: StoredCheckpointOrder,
    wire: PrincipalTrustCheckpointWireV1,
}

fn decode_canonical_value(
    record: &ControlRecord,
) -> Result<serde_json::Value, PrincipalTrustCheckpointChainRecoveryError> {
    let bytes = record.canonical_record_jcs().as_slice();
    let value = canon::parse_strict(bytes)
        .map_err(PrincipalTrustCheckpointChainRecoveryError::StoredRecordSyntax)?;
    if canon::to_canonical_string(&value)
        .map_err(PrincipalTrustCheckpointChainRecoveryError::StoredRecordSyntax)?
        .as_bytes()
        != bytes
    {
        return Err(PrincipalTrustCheckpointChainRecoveryError::NonCanonicalStoredRecord);
    }
    Ok(value)
}

fn require_exact_stored_record(
    physical: &ControlRecord,
    semantic: &PrincipalTrustCheckpointV1,
) -> Result<(), PrincipalTrustCheckpointChainRecoveryError> {
    let projected = record_projection::record(physical.coordinate().clone(), semantic)
        .map_err(|_| PrincipalTrustCheckpointChainRecoveryError::StoredRecordMismatch)?;
    if projected == *physical {
        Ok(())
    } else {
        Err(PrincipalTrustCheckpointChainRecoveryError::StoredRecordMismatch)
    }
}

fn require_complete_checkpoint_chain(
    checkpoints: &[RecoveredSignedPrincipalTrustCheckpoint],
    current: &PrincipalTrustCheckpointHead,
) -> Result<(), PrincipalTrustCheckpointChainRecoveryError> {
    let last = checkpoints
        .last()
        .ok_or(PrincipalTrustCheckpointChainRecoveryError::Chain)?;
    if last.coordinate != *current.coordinate()
        || last.checkpoint.principal_id() != current.principal_id()
        || *last.checkpoint.checkpoint_epoch() != current.checkpoint_epoch()
    {
        return Err(PrincipalTrustCheckpointChainRecoveryError::CurrentHead);
    }
    Ok(())
}

fn key_resolution_rejected() -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "principal trust checkpoint",
        invariant: "resolved historical principal key",
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/principal_trust_recovery.rs"]
mod tests;
