//! Exact signed checkpoint and independent-anchor fences shared by protected reads.

use super::{checkpoint_profile::OperationCheckpointSelection, LocalOperationRecoveryError};
use pale_core::{canon, records::*};
use pale_ports::{
    AccumulatorMembershipLeaf, AccumulatorMembershipLimits, AntiRollbackAnchor,
    AntiRollbackAnchorPort, AuthorityStoreCoordinate, ControlDomainAntiRollbackAnchor,
    ControlRecord, OperationAccumulatorCheckpoint, OperationAccumulatorMembership,
    OperationCustodyPort,
};
use std::collections::BTreeMap;

/// Signed current coordinates after the first independent-anchor check.
/// Completion still requires exact native membership and the second anchor read.
pub(super) struct CurrentOperationCheckpoint {
    pub(super) record: ControlStateCheckpointV1,
    record_bytes: usize,
    coordinate: OperationAccumulatorCheckpoint,
    anchor: AntiRollbackAnchor,
}

/// Limits for checking the checkpoint indexed by an observation against the
/// current anchored history. No historical accumulator membership is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationCheckpointAncestryLimits {
    maximum_checkpoints: usize,
    maximum_bytes: usize,
    maximum_admissions: usize,
}

impl OperationCheckpointAncestryLimits {
    /// Bound the next body allocation by the remaining fixed-proof byte budget.
    pub(in crate::resident) fn remaining_bytes(
        self,
        used: usize,
    ) -> Result<usize, LocalOperationRecoveryError> {
        self.maximum_bytes
            .checked_sub(used)
            .filter(|remaining| *remaining > 0)
            .ok_or(LocalOperationRecoveryError::Bounds)
    }

    /// Check a fixed recovery proof before reading or admitting its records.
    pub(in crate::resident) fn require_work(
        self,
        checkpoints: usize,
        bytes: usize,
        admissions: usize,
    ) -> Result<(), LocalOperationRecoveryError> {
        if checkpoints > self.maximum_checkpoints
            || bytes > self.maximum_bytes
            || admissions > self.maximum_admissions
        {
            return Err(LocalOperationRecoveryError::Bounds);
        }
        Ok(())
    }

    /// Bound checkpoint count, canonical bytes and historical admission calls.
    ///
    /// Count includes the current checkpoint. Bytes include the indexed body,
    /// current body and each predecessor read, even when their bytes repeat.
    /// The admission callback must separately bound its internal authority work.
    /// # Errors
    /// Rejects zero limits; no default quota is supplied.
    pub fn new(
        maximum_checkpoints: usize,
        maximum_bytes: usize,
        maximum_admissions: usize,
    ) -> Result<Self, LocalOperationRecoveryError> {
        if [maximum_checkpoints, maximum_bytes, maximum_admissions].contains(&0) {
            return Err(LocalOperationRecoveryError::Bounds);
        }
        Ok(Self {
            maximum_checkpoints,
            maximum_bytes,
            maximum_admissions,
        })
    }
}

/// Existing locator/profile and owner-resolved checkpoint authority, not a new grant.
pub(super) struct CheckpointRecoveryInput<'a> {
    pub(super) store: &'a AuthorityStoreCoordinate,
    pub(super) selection: &'a OperationCheckpointSelection,
    pub(super) handle: &'a OperationHandleV1,
    pub(super) authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(super) maximum_record_bytes: usize,
}

/// Share the exact locator-recovery signed-head checks with observation recovery.
pub(super) fn read_current_checkpoint<Signature>(
    input: CheckpointRecoveryInput<'_>,
    operation_port: &mut dyn OperationCustodyPort,
    anchor_port: &mut dyn AntiRollbackAnchorPort,
    verify_checkpoint_signature: Signature,
) -> Result<CurrentOperationCheckpoint, LocalOperationRecoveryError>
where
    Signature: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let CheckpointRecoveryInput {
        store,
        selection,
        handle,
        authority,
        maximum_record_bytes,
    } = input;
    if maximum_record_bytes == 0 {
        return Err(LocalOperationRecoveryError::Bounds);
    }
    if !selection.matches_authority(authority)
        || authority.principal_id != handle.issuer_principal_id()
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    let domain = selection.initial_basis()?.domain().clone();
    let (head, checkpoint_record) = operation_port
        .read_checkpoint_head(store, &domain, maximum_record_bytes)?
        .ok_or(LocalOperationRecoveryError::Missing)?;
    if head.checkpoint_domain_id() != &domain || head.coordinate() != checkpoint_record.coordinate()
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    let schema = pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID)?;
    check_record(
        &checkpoint_record,
        &schema.schema,
        &schema.schema_sha256,
        maximum_record_bytes,
    )?;
    let wire: ControlStateCheckpointWireV1 = serde_json::from_value(canon::parse_strict(
        checkpoint_record.canonical_record_jcs().as_slice(),
    )?)?;
    let checkpoint = ControlStateCheckpointV1::admit_signed_wire(
        wire,
        &schema.schema_sha256,
        authority,
        verify_checkpoint_signature,
    )?;
    selection.verify(store, handle, &checkpoint)?;
    if head.domain_epoch() != *checkpoint.domain_epoch()
        || head.domain_sequence() != *checkpoint.domain_sequence()
        || head.committed_transaction_sequence() != *checkpoint.committed_transaction_sequence()
        || head.coordinate().record_sha256().as_str()
            != checkpoint.canonical_control_checkpoint_sha256().as_str()
        || canon::to_canonical_string(&checkpoint)?.as_bytes()
            != checkpoint_record.canonical_record_jcs().as_slice()
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    let expected_anchor = AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
        store.store_id().clone(),
        store.incarnation_id().clone(),
        domain,
        head.domain_epoch(),
        head.domain_sequence(),
        head.coordinate().clone(),
    ));
    exact_anchor(anchor_port, &expected_anchor)?;
    let coordinate = OperationAccumulatorCheckpoint::new(
        store.clone(),
        checkpoint.checkpoint_profile_ref().clone(),
        checkpoint.checkpoint_profile_sha256().clone(),
        head,
        checkpoint.state_root_sha256().clone(),
        checkpoint.unresolved_custody_root_sha256().clone(),
    );
    Ok(CurrentOperationCheckpoint {
        record: checkpoint,
        record_bytes: checkpoint_record.canonical_record_jcs().as_slice().len(),
        coordinate,
        anchor: expected_anchor,
    })
}

impl CurrentOperationCheckpoint {
    /// Exact immutable reference paired with this admitted signed checkpoint.
    pub(super) fn reference(&self) -> &ResidentRecordRefV1 {
        self.coordinate.checkpoint().coordinate().record_ref()
    }
    /// Retain the exact signed head already checked by this read owner.
    pub(super) fn into_record(self) -> (ResidentRecordRefV1, ControlStateCheckpointV1) {
        (
            self.coordinate
                .checkpoint()
                .coordinate()
                .record_ref()
                .clone(),
            self.record,
        )
    }

    /// Authenticate exact ancestry, not the index's claimed co-commit relation.
    /// The later membership check must still recheck the head and anchor.
    pub(super) fn verify_ancestry<Admission>(
        &self,
        indexed: &ControlRecord,
        maximum_record_bytes: usize,
        limits: OperationCheckpointAncestryLimits,
        port: &mut dyn OperationCustodyPort,
        admit: Admission,
    ) -> Result<ControlStateCheckpointV1, LocalOperationRecoveryError>
    where
        Admission: FnMut(
            &ResidentRecordRefV1,
            ControlStateCheckpointWireV1,
        ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    {
        self.verify_ancestry_set(&[indexed], maximum_record_bytes, limits, port, admit)
            .map(|mut admitted| admitted.remove(0))
    }

    /// Authenticate several indexed checkpoints in one shared bounded walk.
    ///
    /// Each exact body is charged once to the byte budget, including indexed
    /// inputs, the current head, and records read during the backward walk. The
    /// traversal shares one count, byte, and admission budget, so requesting
    /// two ancestors cannot silently multiply the configured limits.
    pub(super) fn verify_ancestry_set<Admission>(
        &self,
        indexed: &[&ControlRecord],
        maximum_record_bytes: usize,
        limits: OperationCheckpointAncestryLimits,
        port: &mut dyn OperationCustodyPort,
        mut admit: Admission,
    ) -> Result<Vec<ControlStateCheckpointV1>, LocalOperationRecoveryError>
    where
        Admission: FnMut(
            &ResidentRecordRefV1,
            ControlStateCheckpointWireV1,
        ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    {
        if indexed.is_empty() {
            return Err(LocalOperationRecoveryError::Association);
        }
        let mut remaining = limits.maximum_bytes;
        let mut charged = BTreeMap::new();
        for record in indexed {
            check_record(
                record,
                self.record.schema_id(),
                self.record.schema_sha256(),
                maximum_record_bytes,
            )?;
            match charged.insert(
                record.coordinate().record_ref().clone(),
                record.coordinate().record_sha256().clone(),
            ) {
                Some(digest) if &digest != record.coordinate().record_sha256() => {
                    return Err(LocalOperationRecoveryError::Association);
                }
                Some(_) => {}
                None => {
                    remaining = remaining
                        .checked_sub(record.canonical_record_jcs().as_slice().len())
                        .ok_or(LocalOperationRecoveryError::Bounds)?;
                }
            }
        }
        match charged.insert(
            self.coordinate
                .checkpoint()
                .coordinate()
                .record_ref()
                .clone(),
            self.coordinate
                .checkpoint()
                .coordinate()
                .record_sha256()
                .clone(),
        ) {
            Some(digest)
                if &digest != self.coordinate.checkpoint().coordinate().record_sha256() =>
            {
                return Err(LocalOperationRecoveryError::Association);
            }
            Some(_) => {}
            None => {
                remaining = remaining
                    .checked_sub(self.record_bytes)
                    .ok_or(LocalOperationRecoveryError::Bounds)?;
            }
        }
        let mut found = vec![None; indexed.len()];
        let mut checkpoint = self.record.clone();
        let mut reference = self
            .coordinate
            .checkpoint()
            .coordinate()
            .record_ref()
            .clone();
        let mut count = 1;
        let mut admissions = 0;
        loop {
            for (position, expected) in indexed.iter().enumerate() {
                if found[position].is_none()
                    && &reference == expected.coordinate().record_ref()
                    && checkpoint.canonical_control_checkpoint_sha256().as_str()
                        == expected.coordinate().record_sha256().as_str()
                {
                    if canon::to_canonical_string(&checkpoint)?.as_bytes()
                        != expected.canonical_record_jcs().as_slice()
                    {
                        return Err(LocalOperationRecoveryError::Association);
                    }
                    found[position] = Some(checkpoint.clone());
                }
            }
            if found.iter().all(Option::is_some) {
                return found
                    .into_iter()
                    .map(|value| value.ok_or(LocalOperationRecoveryError::Association))
                    .collect();
            }
            let (Some(previous_ref), Some(previous_digest)) = (
                checkpoint.previous_control_checkpoint_ref(),
                checkpoint.previous_control_checkpoint_sha256(),
            ) else {
                return Err(LocalOperationRecoveryError::Association);
            };
            if count == limits.maximum_checkpoints
                || admissions == limits.maximum_admissions
                || remaining == 0
            {
                return Err(LocalOperationRecoveryError::Bounds);
            }
            let coordinate =
                super::record_projection::coordinate(previous_ref, previous_digest.as_str())
                    .map_err(|_| LocalOperationRecoveryError::Association)?;
            let record = port
                .read_operation_record(
                    self.coordinate.store(),
                    &coordinate,
                    maximum_record_bytes.min(remaining),
                )?
                .ok_or(LocalOperationRecoveryError::Missing)?;
            if record.coordinate() != &coordinate {
                return Err(LocalOperationRecoveryError::Association);
            }
            check_record(
                &record,
                self.record.schema_id(),
                self.record.schema_sha256(),
                maximum_record_bytes.min(remaining),
            )?;
            match charged.insert(
                record.coordinate().record_ref().clone(),
                record.coordinate().record_sha256().clone(),
            ) {
                Some(digest) if &digest != record.coordinate().record_sha256() => {
                    return Err(LocalOperationRecoveryError::Association);
                }
                Some(_) => {}
                None => {
                    remaining = remaining
                        .checked_sub(record.canonical_record_jcs().as_slice().len())
                        .ok_or(LocalOperationRecoveryError::Bounds)?;
                }
            }
            let wire = serde_json::from_value(canon::parse_strict(
                record.canonical_record_jcs().as_slice(),
            )?)?;
            admissions += 1;
            let predecessor = admit(previous_ref, wire)?;
            if predecessor.canonical_control_checkpoint_sha256().as_str()
                != coordinate.record_sha256().as_str()
                || canon::to_canonical_string(&predecessor)?.as_bytes()
                    != record.canonical_record_jcs().as_slice()
            {
                return Err(LocalOperationRecoveryError::Association);
            }
            checkpoint.verify_successor_of(previous_ref, &predecessor)?;
            reference = previous_ref.clone();
            checkpoint = predecessor;
            count += 1;
        }
    }

    pub(super) fn verify_membership(
        &self,
        selection: &OperationCheckpointSelection,
        leaves: Vec<AccumulatorMembershipLeaf>,
        limits: AccumulatorMembershipLimits,
        operation_port: &mut dyn OperationCustodyPort,
        anchor_port: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<(), LocalOperationRecoveryError> {
        let membership = OperationAccumulatorMembership::new(
            self.coordinate.clone(),
            leaves,
            limits,
            selection.implementation(),
        )?;
        let result = operation_port.verify_accumulator_membership(&membership)?;
        if result.request() != &membership {
            return Err(LocalOperationRecoveryError::Association);
        }
        // The independent anchor is not in the native snapshot. A race rejects, never retries.
        exact_anchor(anchor_port, &self.anchor)
    }
}

fn exact_anchor(
    port: &mut dyn AntiRollbackAnchorPort,
    expected: &AntiRollbackAnchor,
) -> Result<(), LocalOperationRecoveryError> {
    let key = expected.key();
    let actual = port
        .read(&key)?
        .ok_or(LocalOperationRecoveryError::Missing)?;
    if actual.key() != key || &actual != expected {
        return Err(LocalOperationRecoveryError::Association);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/support/resident/checkpoint_recovery.rs"]
pub(in crate::resident) mod checkpoint_recovery_fixture;

pub(super) fn check_record(
    record: &ControlRecord,
    schema: &SchemaId,
    digest: &SchemaDigest,
    maximum: usize,
) -> Result<(), LocalOperationRecoveryError> {
    if record.canonical_record_jcs().as_slice().len() > maximum {
        return Err(LocalOperationRecoveryError::Bounds);
    }
    if record.schema_id() != schema || record.schema_sha256() != digest {
        return Err(LocalOperationRecoveryError::Association);
    }
    Ok(())
}
