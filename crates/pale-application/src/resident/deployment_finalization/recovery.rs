//! Readmit one retained deployment finalization at its original commit time.
//!
//! The caller supplies already-admitted finalization records and the exact
//! historical trust and control checkpoints. This module verifies the retained
//! command and receipt against that evidence and keeps the physical snapshot.
//! It does not select current authority, run the command again, write a store,
//! compare anchors, or make a deployment ready.

use pale_core::{canon, records::*};
use pale_ports::{
    ControlCheckpointHead, ControlRecord, DeploymentFinalizationSnapshot,
    PrincipalTrustCheckpointHead,
};
use serde::{de::DeserializeOwned, Deserialize};

use super::{projection, AdmittedDeploymentFinalization};
use crate::resident::record_projection;

/// Exact semantic evidence expected to match one physical finalization closure.
pub(in crate::resident) struct HistoricalDeploymentFinalizationRecoveryBasis<'a, 'records> {
    /// Already-admitted binding, signer authorities, and trust transition.
    pub finalization: &'a AdmittedDeploymentFinalization<'records>,
    /// Exact predecessor trust checkpoint and immutable reference.
    pub predecessor_trust: (&'a ResidentRecordRefV1, &'a PrincipalTrustCheckpointV1),
    /// Exact successor trust checkpoint and immutable reference.
    pub successor_trust: (&'a ResidentRecordRefV1, &'a PrincipalTrustCheckpointV1),
    /// Exact predecessor principal-control checkpoint and immutable reference.
    pub predecessor_control: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Exact successor principal-control checkpoint and immutable reference.
    pub successor_control: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    /// Selected command schema digest.
    pub command_schema_sha256: &'a SchemaDigest,
    /// Selected receipt schema digest.
    pub receipt_schema_sha256: &'a SchemaDigest,
}

/// Result of resolving the administrator authority selected by historical trust.
pub(in crate::resident) enum HistoricalLocalAdminAuthorityResolution {
    /// The exact authority was admitted under its own historical owner.
    Resolved(Box<LocalAdminAuthorityV1>),
    /// No evidence establishes the selected authority.
    Unavailable,
    /// Available evidence assigns unlike records to the selected coordinate.
    Conflicting,
}

/// Resolve one historical administrator authority without selecting it as current.
pub(in crate::resident) trait HistoricalLocalAdminAuthorityResolver {
    /// Resolve the exact authority record selected by the supplied trust checkpoint.
    fn resolve_historical_local_admin_authority(
        &mut self,
        authority_record: &ControlRecord,
        trust_checkpoint: &PrincipalTrustCheckpointV1,
    ) -> HistoricalLocalAdminAuthorityResolution;
}

impl<F> HistoricalLocalAdminAuthorityResolver for F
where
    F: FnMut(
        &ControlRecord,
        &PrincipalTrustCheckpointV1,
    ) -> HistoricalLocalAdminAuthorityResolution,
{
    fn resolve_historical_local_admin_authority(
        &mut self,
        authority_record: &ControlRecord,
        trust_checkpoint: &PrincipalTrustCheckpointV1,
    ) -> HistoricalLocalAdminAuthorityResolution {
        self(authority_record, trust_checkpoint)
    }
}

/// Verify one retained command with its resolved administrator subject key.
pub(in crate::resident) trait HistoricalLocalAdminCommandSignatureVerifier {
    /// Verify the canonical command signature.
    fn verify_historical_local_admin_command_signature(
        &mut self,
        public_key: &Ed25519PublicKeyV1,
        domain: &[u8],
        preimage: &LocalAdminCommandPreimageV1,
        digest: &LocalAdminCommandDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;
}

impl<F> HistoricalLocalAdminCommandSignatureVerifier for F
where
    F: FnMut(
        &Ed25519PublicKeyV1,
        &[u8],
        &LocalAdminCommandPreimageV1,
        &LocalAdminCommandDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    fn verify_historical_local_admin_command_signature(
        &mut self,
        public_key: &Ed25519PublicKeyV1,
        domain: &[u8],
        preimage: &LocalAdminCommandPreimageV1,
        digest: &LocalAdminCommandDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self(public_key, domain, preimage, digest, signature)
    }
}

/// Verify one retained receipt with its historical deployment runtime key.
pub(in crate::resident) trait HistoricalLocalAdminReceiptSignatureVerifier {
    /// Verify the canonical receipt signature.
    fn verify_historical_local_admin_receipt_signature(
        &mut self,
        public_key: &Ed25519PublicKeyV1,
        domain: &[u8],
        preimage: &LocalAdminReceiptPreimageV1,
        digest: &LocalAdminReceiptDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>;
}

impl<F> HistoricalLocalAdminReceiptSignatureVerifier for F
where
    F: FnMut(
        &Ed25519PublicKeyV1,
        &[u8],
        &LocalAdminReceiptPreimageV1,
        &LocalAdminReceiptDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    fn verify_historical_local_admin_receipt_signature(
        &mut self,
        public_key: &Ed25519PublicKeyV1,
        domain: &[u8],
        preimage: &LocalAdminReceiptPreimageV1,
        digest: &LocalAdminReceiptDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self(public_key, domain, preimage, digest, signature)
    }
}

/// Physical closure with its historically admitted command and receipt.
///
/// This value records a past committed outcome. It does not restore current
/// administration or runtime authority and cannot be used to repeat the write.
pub(in crate::resident) struct RecoveredHistoricalDeploymentFinalization {
    evidence: DeploymentFinalizationSnapshot,
    command: LocalAdminCommandV1,
    receipt: LocalAdminReceiptV1,
}

impl RecoveredHistoricalDeploymentFinalization {
    /// Borrow the exact physical closure used during recovery.
    #[must_use]
    pub(in crate::resident) const fn evidence(&self) -> &DeploymentFinalizationSnapshot {
        &self.evidence
    }

    /// Borrow the command admitted at its recorded commit time.
    #[must_use]
    pub(in crate::resident) const fn command(&self) -> &LocalAdminCommandV1 {
        &self.command
    }

    /// Borrow the authenticated retained receipt.
    #[must_use]
    pub(in crate::resident) const fn receipt(&self) -> &LocalAdminReceiptV1 {
        &self.receipt
    }
}

/// Why a physical finalization cannot be readmitted as its retained outcome.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum HistoricalDeploymentFinalizationRecoveryError {
    /// Stored bytes are not strict JSON.
    #[error("stored deployment finalization record is not strict JSON")]
    StoredRecordSyntax(#[source] canon::CanonError),
    /// Stored JSON bytes are valid but not canonical.
    #[error("stored deployment finalization record is not canonical JSON")]
    NonCanonicalStoredRecord,
    /// Stored JSON does not have the selected record's wire shape.
    #[error("stored deployment finalization record has the wrong wire shape")]
    StoredRecordDecode(#[source] serde_json::Error),
    /// Exact historical authority evidence is absent.
    #[error("historical local administrator authority is unavailable")]
    HistoricalAuthorityUnavailable,
    /// Historical authority evidence conflicts for the selected coordinate.
    #[error("historical local administrator authority evidence conflicts")]
    HistoricalAuthorityConflict,
    /// Existing record admission rejected authority, association, time, or signature.
    #[error(transparent)]
    Record(#[from] InvalidResidentRecordV1),
    /// Physical and already-admitted semantic evidence disagree.
    #[error("deployment finalization physical and semantic evidence differ")]
    Association,
    /// An admitted record does not reproduce the exact stored row.
    #[error("deployment finalization admitted record does not reproduce stored bytes")]
    StoredRecordMismatch,
}

/// Readmit one retained finalization without consulting current heads or time.
///
/// The historical authority resolver must apply that authority record's own
/// admission law. Command validity is checked at the receipt's authenticated
/// commit time. Every semantic record is projected back to its exact stored row.
///
/// # Errors
///
/// Rejects unavailable or conflicting authority, non-canonical bytes, failed
/// signatures, unlike candidate order or affected heads, substituted checkpoints,
/// a changed replay tuple, receipt association, or any byte-level mismatch.
pub(in crate::resident) fn recover_historical_deployment_finalization(
    evidence: DeploymentFinalizationSnapshot,
    basis: HistoricalDeploymentFinalizationRecoveryBasis<'_, '_>,
    authority_resolver: &mut dyn HistoricalLocalAdminAuthorityResolver,
    command_signature_verifier: &mut dyn HistoricalLocalAdminCommandSignatureVerifier,
    receipt_signature_verifier: &mut dyn HistoricalLocalAdminReceiptSignatureVerifier,
) -> Result<RecoveredHistoricalDeploymentFinalization, HistoricalDeploymentFinalizationRecoveryError>
{
    verify_basis(&evidence, &basis)?;

    let authority = match authority_resolver.resolve_historical_local_admin_authority(
        evidence.admin_authority_record(),
        basis.predecessor_trust.1,
    ) {
        HistoricalLocalAdminAuthorityResolution::Resolved(authority) => *authority,
        HistoricalLocalAdminAuthorityResolution::Unavailable => {
            return Err(
                HistoricalDeploymentFinalizationRecoveryError::HistoricalAuthorityUnavailable,
            );
        }
        HistoricalLocalAdminAuthorityResolution::Conflicting => {
            return Err(HistoricalDeploymentFinalizationRecoveryError::HistoricalAuthorityConflict);
        }
    };
    require_exact_record(evidence.admin_authority_record(), &authority)?;

    let receipt_value = decode_canonical_value(evidence.records().receipt())?;
    let receipt_context: StoredReceiptContext = serde_json::from_value(receipt_value.clone())
        .map_err(HistoricalDeploymentFinalizationRecoveryError::StoredRecordDecode)?;
    let receipt_wire: LocalAdminReceiptWireV1 = serde_json::from_value(receipt_value)
        .map_err(HistoricalDeploymentFinalizationRecoveryError::StoredRecordDecode)?;
    receipt_wire.clone().verify_structure()?;
    let committed_at = receipt_context
        .committed_at
        .ok_or(HistoricalDeploymentFinalizationRecoveryError::Association)?;
    let recorded_at = receipt_context.recorded_at;

    let command_wire: LocalAdminCommandWireV1 = decode_canonical(evidence.records().command())?;
    let command = LocalAdminCommandV1::admit_historical_authenticated_wire(
        command_wire,
        &HistoricalLocalAdminCommandAdmissionContextV1 {
            expected_schema_sha256: basis.command_schema_sha256,
            trust_checkpoint: basis.predecessor_trust.1,
            admin_authority_ref: basis.predecessor_trust.1.local_admin_authority_ref(),
            admin_authority: &authority,
            committed_at,
        },
        |key, domain, preimage, digest, signature| {
            projection::verify_command_selection(
                basis.finalization,
                basis.successor_control,
                preimage,
            )
            .map_err(|mismatch| InvalidResidentRecordV1::ContextualAdmission {
                record: "historical deployment finalization",
                invariant: mismatch.invariant(),
            })?;
            command_signature_verifier.verify_historical_local_admin_command_signature(
                key, domain, preimage, digest, signature,
            )
        },
    )?;
    require_exact_record(evidence.records().command(), &command)?;
    verify_replay(
        evidence.replay(),
        &authority,
        &command,
        &receipt_context.admin_receipt_id,
    )?;

    let receipt_projection = projection::FinalizationReceiptProjection::new(
        basis.finalization,
        basis.successor_control,
        evidence.records().command().coordinate().record_ref(),
        &command,
    )?;
    let receipt = LocalAdminReceiptV1::admit_contextual_wire(
        receipt_wire,
        &receipt_projection.admission_context(
            basis.finalization,
            evidence.records().command().coordinate().record_ref(),
            &command,
            evidence.replay().receipt_id(),
            basis.receipt_schema_sha256,
            &committed_at,
            &recorded_at,
        ),
        |key, domain, preimage, digest, signature| {
            receipt_signature_verifier.verify_historical_local_admin_receipt_signature(
                key, domain, preimage, digest, signature,
            )
        },
    )?;
    require_exact_receipt(evidence.records().receipt(), &receipt)?;

    Ok(RecoveredHistoricalDeploymentFinalization {
        evidence,
        command,
        receipt,
    })
}

fn verify_basis(
    evidence: &DeploymentFinalizationSnapshot,
    basis: &HistoricalDeploymentFinalizationRecoveryBasis<'_, '_>,
) -> Result<(), HistoricalDeploymentFinalizationRecoveryError> {
    let selected = basis.finalization.records();
    for checkpoint in [basis.predecessor_trust.1, basis.successor_trust.1] {
        checkpoint.verify()?;
    }
    for checkpoint in [basis.predecessor_control.1, basis.successor_control.1] {
        checkpoint.verify()?;
    }
    if basis.predecessor_trust.0 != selected.trust.predecessor_ref
        || basis.predecessor_trust.1 != selected.trust.predecessor
        || basis.successor_trust.0 != selected.trust.successor_ref
        || basis.successor_trust.1 != selected.trust.successor
        || evidence.binding().principal_id() != basis.predecessor_trust.1.principal_id()
        || evidence.predecessor_trust()
            != &trust_head(basis.predecessor_trust.0, basis.predecessor_trust.1)?
        || evidence.successor_trust()
            != &trust_head(basis.successor_trust.0, basis.successor_trust.1)?
        || evidence.predecessor_control()
            != &control_head(basis.predecessor_control.0, basis.predecessor_control.1)?
        || evidence.successor_control()
            != &control_head(basis.successor_control.0, basis.successor_control.1)?
        || basis.successor_control.1.previous_control_checkpoint_ref()
            != &Some(basis.predecessor_control.0.clone())
        || basis
            .successor_control
            .1
            .previous_control_checkpoint_sha256()
            != &Some(
                basis
                    .predecessor_control
                    .1
                    .canonical_control_checkpoint_sha256()
                    .clone(),
            )
    {
        return Err(HistoricalDeploymentFinalizationRecoveryError::Association);
    }
    require_exact_record(
        evidence.predecessor_trust_record(),
        basis.predecessor_trust.1,
    )?;
    require_exact_record(
        evidence.records().trust_checkpoint(),
        basis.successor_trust.1,
    )?;
    require_exact_record(
        evidence.predecessor_control_record(),
        basis.predecessor_control.1,
    )?;
    require_exact_record(
        evidence.records().control_checkpoint(),
        basis.successor_control.1,
    )?;
    require_exact_record(evidence.records().binding(), selected.binding.1)?;
    require_exact_record(
        evidence.records().checkpoint_signing_authority(),
        selected.checkpoint_signer.1,
    )?;
    require_exact_record(
        evidence.records().runtime_authority(),
        selected.runtime_authority.1,
    )?;
    Ok(())
}

fn verify_replay(
    replay: &pale_ports::DeploymentFinalizationReplayIdentity,
    authority: &LocalAdminAuthorityV1,
    command: &LocalAdminCommandV1,
    receipt_id: &LocalAdminReceiptIdV1,
) -> Result<(), HistoricalDeploymentFinalizationRecoveryError> {
    if replay.admin_authority().record_ref() != command.admin_authority_ref()
        || replay.admin_authority().record_sha256().as_str()
            != authority.canonical_admin_authority_sha256().as_str()
        || replay.admin_command_id() != command.admin_command_id()
        || replay.idempotency_key() != command.idempotency_key()
        || replay.command().record_sha256().as_str()
            != command.canonical_admin_command_sha256().as_str()
        || replay.receipt_id() != receipt_id
    {
        return Err(HistoricalDeploymentFinalizationRecoveryError::Association);
    }
    Ok(())
}

fn decode_canonical<T>(
    record: &ControlRecord,
) -> Result<T, HistoricalDeploymentFinalizationRecoveryError>
where
    T: DeserializeOwned,
{
    let value = decode_canonical_value(record)?;
    serde_json::from_value(value)
        .map_err(HistoricalDeploymentFinalizationRecoveryError::StoredRecordDecode)
}

fn decode_canonical_value(
    record: &ControlRecord,
) -> Result<serde_json::Value, HistoricalDeploymentFinalizationRecoveryError> {
    let bytes = record.canonical_record_jcs().as_slice();
    let value = canon::parse_strict(bytes)
        .map_err(HistoricalDeploymentFinalizationRecoveryError::StoredRecordSyntax)?;
    if canon::to_canonical_string(&value)
        .map_err(HistoricalDeploymentFinalizationRecoveryError::StoredRecordSyntax)?
        .as_bytes()
        != bytes
    {
        return Err(HistoricalDeploymentFinalizationRecoveryError::NonCanonicalStoredRecord);
    }
    Ok(value)
}

#[derive(Deserialize)]
struct StoredReceiptContext {
    admin_receipt_id: LocalAdminReceiptIdV1,
    committed_at: Option<UnixTimeV1>,
    recorded_at: UnixTimeV1,
}

fn require_exact_record<R>(
    physical: &ControlRecord,
    semantic: &R,
) -> Result<(), HistoricalDeploymentFinalizationRecoveryError>
where
    R: serde::Serialize + record_projection::ResidentRecordSchema,
{
    let projected = record_projection::record(physical.coordinate().clone(), semantic)
        .map_err(|_| HistoricalDeploymentFinalizationRecoveryError::StoredRecordMismatch)?;
    if projected == *physical {
        Ok(())
    } else {
        Err(HistoricalDeploymentFinalizationRecoveryError::StoredRecordMismatch)
    }
}

fn require_exact_receipt(
    physical: &ControlRecord,
    receipt: &LocalAdminReceiptV1,
) -> Result<(), HistoricalDeploymentFinalizationRecoveryError> {
    let projected = record_projection::record_with_schema(
        physical.coordinate().clone(),
        receipt.schema_id().clone(),
        receipt.schema_sha256().clone(),
        receipt,
    )
    .map_err(|_| HistoricalDeploymentFinalizationRecoveryError::StoredRecordMismatch)?;
    if projected == *physical {
        Ok(())
    } else {
        Err(HistoricalDeploymentFinalizationRecoveryError::StoredRecordMismatch)
    }
}

fn trust_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &PrincipalTrustCheckpointV1,
) -> Result<PrincipalTrustCheckpointHead, HistoricalDeploymentFinalizationRecoveryError> {
    Ok(PrincipalTrustCheckpointHead::new(
        checkpoint.principal_id().clone(),
        *checkpoint.checkpoint_epoch(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_trust_checkpoint_sha256().as_str(),
        )
        .map_err(|_| HistoricalDeploymentFinalizationRecoveryError::Association)?,
    ))
}

fn control_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlCheckpointHead, HistoricalDeploymentFinalizationRecoveryError> {
    Ok(ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )
        .map_err(|_| HistoricalDeploymentFinalizationRecoveryError::Association)?,
    ))
}
