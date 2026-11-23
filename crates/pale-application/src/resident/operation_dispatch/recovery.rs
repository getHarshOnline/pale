//! Shared read-back proof for a recovered operation head.

use super::*;
use crate::resident::{
    record_projection, OperationDispatchCustodyError, RecoveredOperationObservation,
};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorPort, ControlCheckpointHead,
    ControlDomainAntiRollbackAnchor, OperationCustodyPort,
};

/// Re-read the recovered operation head, its checkpoint and the independent anchor.
///
/// Recovery already admitted the complete history. This check proves that the
/// authenticated head is still the physical operation head and that its admitted
/// current checkpoint still matches both storage and the independent anchor. The
/// recovery may have selected an older history member for immutable attempt
/// evidence; that selection never substitutes for the actual head checked here.
pub(in crate::resident) fn verify_recovered_operation_head_custody(
    operation: &IssuedLocalOperation,
    recovered: &RecoveredOperationObservation,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_record_bytes: usize,
) -> Result<(), OperationAttemptError> {
    let (handle_ref, handle) = operation.handle();
    verify_operation_head_custody(
        recovered.store(),
        handle_ref,
        handle,
        recovered.selected_head(),
        recovered.selected_head_checkpoint_record(),
        recovered.current_checkpoint(),
        custody,
        anchor,
        maximum_record_bytes,
    )
}

/// Re-read one already-admitted head basis before its next signature.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn verify_operation_head_custody(
    store: &pale_ports::AuthorityStoreCoordinate,
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    head: (&ResidentRecordRefV1, &OperationObservationV1),
    head_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    current_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_record_bytes: usize,
) -> Result<(), OperationAttemptError> {
    let selected = crate::resident::read_operation_observation_for_handle(
        custody,
        store,
        handle_ref,
        handle,
        maximum_record_bytes,
    )
    .map_err(OperationDispatchCustodyError::from)?
    .ok_or(OperationDispatchCustodyError::HeadChanged)?;
    let (head_ref, head_observation) = head;
    let expected_head = record_projection::record(
        record_projection::coordinate(
            head_ref,
            head_observation
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)?,
        head_observation,
    )
    .map_err(OperationObservationCustodyProjectionError::from)?;
    let (head_checkpoint_ref, head_checkpoint) = head_checkpoint;
    let expected_head_checkpoint = record_projection::record(
        record_projection::coordinate(
            head_checkpoint_ref,
            head_checkpoint
                .canonical_control_checkpoint_sha256()
                .as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)?,
        head_checkpoint,
    )
    .map_err(OperationObservationCustodyProjectionError::from)?;
    if selected.observation() != &expected_head
        || selected.checkpoint() != &expected_head_checkpoint
    {
        return Err(OperationDispatchCustodyError::HeadChanged.into());
    }

    let (current_ref, current) = current_checkpoint;
    let current_coordinate = record_projection::coordinate(
        current_ref,
        current.canonical_control_checkpoint_sha256().as_str(),
    )
    .map_err(OperationObservationCustodyProjectionError::from)?;
    let current_record = record_projection::record(current_coordinate.clone(), current)
        .map_err(OperationObservationCustodyProjectionError::from)?;
    let current_head = ControlCheckpointHead::new(
        current.checkpoint_domain_id().clone(),
        *current.domain_epoch(),
        *current.domain_sequence(),
        *current.committed_transaction_sequence(),
        current_coordinate.clone(),
    );
    let stored = custody
        .read_checkpoint_head(store, current.checkpoint_domain_id(), maximum_record_bytes)
        .map_err(OperationDispatchCustodyError::from)?;
    if !stored.as_ref().is_some_and(|(head, record)| {
        head == &current_head
            && record == &current_record
            && record.canonical_record_jcs().as_slice().len() <= maximum_record_bytes
    }) {
        return Err(OperationDispatchCustodyError::HeadChanged.into());
    }
    let expected_anchor = AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
        store.store_id().clone(),
        store.incarnation_id().clone(),
        current.checkpoint_domain_id().clone(),
        *current.domain_epoch(),
        *current.domain_sequence(),
        current_coordinate,
    ));
    if anchor
        .read(&expected_anchor.key())
        .map_err(OperationDispatchCustodyError::from)?
        .as_ref()
        != Some(&expected_anchor)
    {
        return Err(OperationDispatchCustodyError::AnchorChanged.into());
    }
    Ok(())
}
