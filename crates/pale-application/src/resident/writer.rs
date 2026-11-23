//! Application-owned preparation of clean resident-writer transitions.
//!
//! Contextual admission remains in the signed record families. This module
//! closes exact admitted record references and one admitted control-state
//! checkpoint transition into a physical candidate. The candidate carries no
//! commit authority: a custody adapter must compare the exact predecessor and
//! advance the writer and checkpoint heads in one transaction.
//!
//! Checkpoint-profile evaluation, accumulator-root construction, and proof
//! that the changed-leaf set represents the writer mutation remain upstream
//! of this projection. This module reuses
//! [`ControlStateCheckpointV1::verify_successor_of`] for the complete pure
//! checkpoint-succession law. It then closes writer fences against the physical
//! trust store and checkpoints against the distinct logical principal-control
//! store selected for the writer domain.

use pale_core::canon;
use pale_core::records::{
    ControlStateCheckpointV1, InvalidResidentRecordV1, PrincipalResidentWriterFenceV1,
    PrincipalResidentWriterTerminalReceiptV1, ResidentRecordRefV1,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord, ControlRecordError,
    PreparedPrincipalResidentWriterTransition, PrincipalResidentWriterCandidateError,
    PrincipalResidentWriterHead, PrincipalResidentWriterTransitionKind, PrincipalTrustStoreBinding,
};

use super::record_projection::{self, ControlRecordProjectionError};

/// Failure to project admitted clean writer records into a closed candidate.
#[derive(Debug, thiserror::Error)]
pub enum PrincipalResidentWriterProjectionError {
    /// An admitted record no longer reproduces its canonical unsigned digest.
    #[error(transparent)]
    InvalidRecord(#[from] InvalidResidentRecordV1),
    /// Canonical signed-record encoding failed.
    #[error("canonicalize admitted principal resident-writer record")]
    Canonicalize(#[from] canon::CanonError),
    /// A shared physical record coordinate or byte string is malformed.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// Exact admitted references do not close over one clean transition.
    #[error("admitted resident-writer records do not close over the selected predecessor")]
    ReferenceClosure,
    /// The writer fence does not match the selected physical store and principal.
    #[error("resident-writer fence does not match the selected physical trust-store binding")]
    TrustStoreClosure,
    /// The admitted checkpoint does not advance the selected logical hard frontier.
    #[error("resident-writer checkpoint does not close over the selected principal control store")]
    CheckpointClosure,
    /// The projected physical transition contradicts its finite head algebra.
    #[error(transparent)]
    Candidate(#[from] PrincipalResidentWriterCandidateError),
}

impl From<ControlRecordProjectionError> for PrincipalResidentWriterProjectionError {
    fn from(error: ControlRecordProjectionError) -> Self {
        match error {
            ControlRecordProjectionError::Canonicalize(source) => Self::Canonicalize(source),
            ControlRecordProjectionError::Record(source) => Self::Record(source),
        }
    }
}

/// Reference-bearing preimage shared by both clean writer transitions.
///
/// The tuple preserves five independently typed inputs: predecessor writer,
/// terminal receipt, successor writer, predecessor checkpoint, and successor
/// checkpoint. [`prepare_clean_principal_resident_writer_retirement`] and
/// [`prepare_clean_principal_resident_writer_successor`] interpret the writer
/// states according to their distinct transition laws; this preimage merely
/// prevents either projection from accepting an unlabelled argument sequence.
///
/// Constructing the preimage verifies no digest, reference, writer state,
/// checkpoint succession, or commit authority. Its fields remain private so
/// the five record pairs cannot be relabelled after construction.
pub struct CleanPrincipalResidentWriterTransitionPreimage<'record> {
    predecessor_writer_ref: &'record ResidentRecordRefV1,
    predecessor_writer: &'record PrincipalResidentWriterFenceV1,
    terminal_receipt_ref: &'record ResidentRecordRefV1,
    terminal_receipt: &'record PrincipalResidentWriterTerminalReceiptV1,
    successor_writer_ref: &'record ResidentRecordRefV1,
    successor_writer: &'record PrincipalResidentWriterFenceV1,
    predecessor_checkpoint_ref: &'record ResidentRecordRefV1,
    predecessor_checkpoint: &'record ControlStateCheckpointV1,
    successor_checkpoint_ref: &'record ResidentRecordRefV1,
    successor_checkpoint: &'record ControlStateCheckpointV1,
}

impl<'record> CleanPrincipalResidentWriterTransitionPreimage<'record> {
    /// Pair every admitted record with the exact reference asserted for it.
    ///
    /// Writer states and cross-record closure remain deliberately unchecked
    /// until one of the two transition-specific preparation functions consumes
    /// the preimage.
    #[must_use]
    pub const fn new(
        predecessor_writer: (
            &'record ResidentRecordRefV1,
            &'record PrincipalResidentWriterFenceV1,
        ),
        terminal_receipt: (
            &'record ResidentRecordRefV1,
            &'record PrincipalResidentWriterTerminalReceiptV1,
        ),
        successor_writer: (
            &'record ResidentRecordRefV1,
            &'record PrincipalResidentWriterFenceV1,
        ),
        predecessor_checkpoint: (
            &'record ResidentRecordRefV1,
            &'record ControlStateCheckpointV1,
        ),
        successor_checkpoint: (
            &'record ResidentRecordRefV1,
            &'record ControlStateCheckpointV1,
        ),
    ) -> Self {
        Self {
            predecessor_writer_ref: predecessor_writer.0,
            predecessor_writer: predecessor_writer.1,
            terminal_receipt_ref: terminal_receipt.0,
            terminal_receipt: terminal_receipt.1,
            successor_writer_ref: successor_writer.0,
            successor_writer: successor_writer.1,
            predecessor_checkpoint_ref: predecessor_checkpoint.0,
            predecessor_checkpoint: predecessor_checkpoint.1,
            successor_checkpoint_ref: successor_checkpoint.0,
            successor_checkpoint: successor_checkpoint.1,
        }
    }
}

/// Project one contextually admitted generation-one active fence.
///
/// The result is already paired with the exact admitted checkpoint transition,
/// but carries no commit authority.
///
/// # Errors
///
/// Returns [`PrincipalResidentWriterProjectionError`] when the record is not
/// self-consistent or cannot form the sole initial active transition.
pub fn prepare_initial_principal_resident_writer_transition(
    physical_trust_store: &PrincipalTrustStoreBinding,
    principal_control_store: &AuthorityStoreCoordinate,
    writer_fence: (&ResidentRecordRefV1, &PrincipalResidentWriterFenceV1),
    predecessor_checkpoint_ref: &ResidentRecordRefV1,
    predecessor_checkpoint: &ControlStateCheckpointV1,
    checkpoint_ref: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<PreparedPrincipalResidentWriterTransition, PrincipalResidentWriterProjectionError> {
    let (writer_fence_ref, writer_fence) = writer_fence;
    writer_fence.verify()?;
    admit_physical_trust_store(physical_trust_store, writer_fence)?;
    admit_checkpoint_pair(
        principal_control_store,
        writer_fence,
        predecessor_checkpoint_ref,
        predecessor_checkpoint,
        checkpoint,
    )?;
    let coordinate = fence_coordinate(writer_fence_ref, writer_fence)?;
    let expected_checkpoint =
        checkpoint_projection(predecessor_checkpoint_ref, predecessor_checkpoint)?;
    let next_checkpoint = checkpoint_projection(checkpoint_ref, checkpoint)?;
    PreparedPrincipalResidentWriterTransition::new(
        physical_trust_store.clone(),
        PrincipalResidentWriterTransitionKind::InitialActive,
        expected_checkpoint,
        None,
        vec![
            record_projection::record(coordinate.clone(), writer_fence)?,
            next_checkpoint.1,
        ],
        head(coordinate, writer_fence, next_checkpoint.0),
    )
    .map_err(Into::into)
}

/// Project an exact terminal receipt and same-generation retired fence.
///
/// Only immutable reference/digest closure is rechecked here. The concrete
/// records can be constructed publicly only through their contextual admission
/// APIs, which remain the semantic owner of signature, chronology, and complete
/// lifetime invariants.
///
/// The result is already paired with the exact admitted checkpoint transition,
/// but carries no commit authority.
///
/// # Errors
///
/// Returns [`PrincipalResidentWriterProjectionError`] when any record fails its
/// self-check, the references do not close exactly, or physical projection fails.
pub fn prepare_clean_principal_resident_writer_retirement(
    physical_trust_store: &PrincipalTrustStoreBinding,
    principal_control_store: &AuthorityStoreCoordinate,
    preimage: CleanPrincipalResidentWriterTransitionPreimage<'_>,
) -> Result<PreparedPrincipalResidentWriterTransition, PrincipalResidentWriterProjectionError> {
    let CleanPrincipalResidentWriterTransitionPreimage {
        predecessor_writer_ref: active_ref,
        predecessor_writer: active,
        terminal_receipt_ref: terminal_ref,
        terminal_receipt: terminal,
        successor_writer_ref: retired_ref,
        successor_writer: retired,
        predecessor_checkpoint_ref,
        predecessor_checkpoint,
        successor_checkpoint_ref: checkpoint_ref,
        successor_checkpoint: checkpoint,
    } = preimage;
    active.verify()?;
    terminal.verify()?;
    retired.verify()?;
    admit_physical_trust_store(physical_trust_store, active)?;
    admit_physical_trust_store(physical_trust_store, retired)?;
    admit_checkpoint_pair(
        principal_control_store,
        retired,
        predecessor_checkpoint_ref,
        predecessor_checkpoint,
        checkpoint,
    )?;
    if !terminal_closes(terminal, active_ref, active)
        || retired.previous_writer_fence_ref().as_ref() != Some(active_ref)
        || retired.previous_writer_fence_sha256().as_ref()
            != Some(active.canonical_principal_resident_writer_fence_sha256())
        || retired.prior_terminal_receipt_ref().as_ref() != Some(terminal_ref)
        || retired.prior_terminal_receipt_sha256().as_ref()
            != Some(terminal.canonical_principal_resident_writer_terminal_receipt_sha256())
    {
        return Err(PrincipalResidentWriterProjectionError::ReferenceClosure);
    }

    let active_coordinate = fence_coordinate(active_ref, active)?;
    let terminal_coordinate = terminal_coordinate(terminal_ref, terminal)?;
    let retired_coordinate = fence_coordinate(retired_ref, retired)?;
    let active_record = record_projection::record(active_coordinate.clone(), active)?;
    let expected_checkpoint =
        checkpoint_projection(predecessor_checkpoint_ref, predecessor_checkpoint)?;
    let next_checkpoint = checkpoint_projection(checkpoint_ref, checkpoint)?;
    PreparedPrincipalResidentWriterTransition::new(
        physical_trust_store.clone(),
        PrincipalResidentWriterTransitionKind::CleanRetirement,
        expected_checkpoint.clone(),
        Some((
            head(active_coordinate, active, expected_checkpoint.0),
            active_record,
        )),
        vec![
            record_projection::record(terminal_coordinate, terminal)?,
            record_projection::record(retired_coordinate.clone(), retired)?,
            next_checkpoint.1,
        ],
        head(retired_coordinate, retired, next_checkpoint.0),
    )
    .map_err(Into::into)
}

/// Project an active successor from an exact cleanly retired head.
///
/// The result is already paired with the exact admitted checkpoint transition,
/// but carries no commit authority.
///
/// # Errors
///
/// Returns [`PrincipalResidentWriterProjectionError`] when the contextual
/// records do not share the exact retired/terminal lineage or projection fails.
pub fn prepare_clean_principal_resident_writer_successor(
    physical_trust_store: &PrincipalTrustStoreBinding,
    principal_control_store: &AuthorityStoreCoordinate,
    preimage: CleanPrincipalResidentWriterTransitionPreimage<'_>,
) -> Result<PreparedPrincipalResidentWriterTransition, PrincipalResidentWriterProjectionError> {
    let CleanPrincipalResidentWriterTransitionPreimage {
        predecessor_writer_ref: retired_ref,
        predecessor_writer: retired,
        terminal_receipt_ref: terminal_ref,
        terminal_receipt: terminal,
        successor_writer_ref: successor_ref,
        successor_writer: successor,
        predecessor_checkpoint_ref,
        predecessor_checkpoint,
        successor_checkpoint_ref: checkpoint_ref,
        successor_checkpoint: checkpoint,
    } = preimage;
    retired.verify()?;
    terminal.verify()?;
    successor.verify()?;
    admit_physical_trust_store(physical_trust_store, retired)?;
    admit_physical_trust_store(physical_trust_store, successor)?;
    admit_checkpoint_pair(
        principal_control_store,
        successor,
        predecessor_checkpoint_ref,
        predecessor_checkpoint,
        checkpoint,
    )?;
    if retired.prior_terminal_receipt_ref().as_ref() != Some(terminal_ref)
        || retired.prior_terminal_receipt_sha256().as_ref()
            != Some(terminal.canonical_principal_resident_writer_terminal_receipt_sha256())
        || successor.previous_writer_fence_ref().as_ref() != Some(retired_ref)
        || successor.previous_writer_fence_sha256().as_ref()
            != Some(retired.canonical_principal_resident_writer_fence_sha256())
        || successor.prior_terminal_receipt_ref().as_ref() != Some(terminal_ref)
        || successor.prior_terminal_receipt_sha256().as_ref()
            != Some(terminal.canonical_principal_resident_writer_terminal_receipt_sha256())
    {
        return Err(PrincipalResidentWriterProjectionError::ReferenceClosure);
    }

    let retired_coordinate = fence_coordinate(retired_ref, retired)?;
    let successor_coordinate = fence_coordinate(successor_ref, successor)?;
    let retired_record = record_projection::record(retired_coordinate.clone(), retired)?;
    let expected_checkpoint =
        checkpoint_projection(predecessor_checkpoint_ref, predecessor_checkpoint)?;
    let next_checkpoint = checkpoint_projection(checkpoint_ref, checkpoint)?;
    PreparedPrincipalResidentWriterTransition::new(
        physical_trust_store.clone(),
        PrincipalResidentWriterTransitionKind::CleanSuccessor,
        expected_checkpoint.clone(),
        Some((
            head(retired_coordinate, retired, expected_checkpoint.0),
            retired_record,
        )),
        vec![
            record_projection::record(successor_coordinate.clone(), successor)?,
            next_checkpoint.1,
        ],
        head(successor_coordinate, successor, next_checkpoint.0),
    )
    .map_err(Into::into)
}

/// Require the writer fence to name the selected physical trust store.
fn admit_physical_trust_store(
    physical_trust_store: &PrincipalTrustStoreBinding,
    writer: &PrincipalResidentWriterFenceV1,
) -> Result<(), PrincipalResidentWriterProjectionError> {
    let exact = writer.trust_store_id() == physical_trust_store.trust_store_id()
        && writer.trust_store_incarnation_id() == physical_trust_store.trust_store_incarnation_id()
        && writer.principal_id() == physical_trust_store.principal_id();
    if exact {
        Ok(())
    } else {
        Err(PrincipalResidentWriterProjectionError::TrustStoreClosure)
    }
}

/// Project the typed semantic writer coordinates into the physical head.
fn head(
    coordinate: pale_ports::ControlRecordCoordinate,
    writer: &PrincipalResidentWriterFenceV1,
    checkpoint: ControlCheckpointHead,
) -> PrincipalResidentWriterHead {
    PrincipalResidentWriterHead::new(
        writer.writer_fence_id().clone(),
        writer.principal_id().clone(),
        *writer.writer_generation(),
        *writer.state(),
        coordinate,
        checkpoint,
    )
}

/// Project one admitted control checkpoint without flattening its domain axes.
fn checkpoint_projection(
    record_ref: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<(ControlCheckpointHead, ControlRecord), PrincipalResidentWriterProjectionError> {
    let coordinate = record_projection::coordinate(
        record_ref,
        checkpoint.canonical_control_checkpoint_sha256().as_str(),
    )?;
    Ok((
        ControlCheckpointHead::new(
            checkpoint.checkpoint_domain_id().clone(),
            *checkpoint.domain_epoch(),
            *checkpoint.domain_sequence(),
            *checkpoint.committed_transaction_sequence(),
            coordinate.clone(),
        ),
        record_projection::record(coordinate, checkpoint)?,
    ))
}

/// Recheck the successor law and both selected principal-control coordinates.
///
/// Accumulator-root and changed-leaf semantics remain owned by the upstream
/// checkpoint constructor; this function does not infer them from a writer
/// record.
fn admit_checkpoint_pair(
    principal_control_store: &AuthorityStoreCoordinate,
    writer: &PrincipalResidentWriterFenceV1,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &ControlStateCheckpointV1,
    successor: &ControlStateCheckpointV1,
) -> Result<(), PrincipalResidentWriterProjectionError> {
    successor
        .verify_successor_of(predecessor_ref, predecessor)
        .map_err(|_| PrincipalResidentWriterProjectionError::CheckpointClosure)?;
    let exact_successor = predecessor.control_store_id() == principal_control_store.store_id()
        && predecessor.control_incarnation_id() == principal_control_store.incarnation_id()
        && successor.control_store_id() == principal_control_store.store_id()
        && successor.control_incarnation_id() == principal_control_store.incarnation_id()
        && predecessor.checkpoint_domain_id().as_str() == "principal-resident-writer"
        && successor.checkpoint_domain_id().as_str() == "principal-resident-writer"
        && predecessor.issuer_principal_id() == writer.principal_id()
        && successor.issuer_principal_id() == writer.principal_id();
    if exact_successor {
        Ok(())
    } else {
        Err(PrincipalResidentWriterProjectionError::CheckpointClosure)
    }
}

/// Erase a fence digest only at the heterogeneous physical boundary.
fn fence_coordinate(
    record_ref: &ResidentRecordRefV1,
    writer: &PrincipalResidentWriterFenceV1,
) -> Result<pale_ports::ControlRecordCoordinate, ControlRecordProjectionError> {
    record_projection::coordinate(
        record_ref,
        writer
            .canonical_principal_resident_writer_fence_sha256()
            .as_str(),
    )
}

/// Erase a terminal-receipt digest only at the heterogeneous physical boundary.
fn terminal_coordinate(
    record_ref: &ResidentRecordRefV1,
    terminal: &PrincipalResidentWriterTerminalReceiptV1,
) -> Result<pale_ports::ControlRecordCoordinate, ControlRecordProjectionError> {
    record_projection::coordinate(
        record_ref,
        terminal
            .canonical_principal_resident_writer_terminal_receipt_sha256()
            .as_str(),
    )
}

/// Check the immutable reference and digest closing one active fence.
fn terminal_closes(
    terminal: &PrincipalResidentWriterTerminalReceiptV1,
    active_ref: &ResidentRecordRefV1,
    active: &PrincipalResidentWriterFenceV1,
) -> bool {
    terminal.writer_fence_ref() == active_ref
        && terminal.writer_fence_sha256()
            == active.canonical_principal_resident_writer_fence_sha256()
}

#[cfg(test)]
#[path = "../../tests/internal/resident/writer.rs"]
mod tests;
