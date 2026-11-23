//! Recovered-observation values used by resident behavior tests.

use super::*;

impl RecoveredOperationObservation {
    pub(in crate::resident) fn fixture(
        operation: &IssuedLocalOperation,
        selected: (ResidentRecordRefV1, OperationObservationV1),
        head: (ResidentRecordRefV1, OperationObservationV1),
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
        store: AuthorityStoreCoordinate,
    ) -> Self {
        let (handle_ref, handle) = operation.handle();
        let handle = crate::resident::record_projection::coordinate(
            handle_ref,
            handle.canonical_operation_handle_sha256().as_str(),
        )
        .expect("fixture handle coordinate");
        Self::fixture_for_handle(handle, selected, head, checkpoint, store)
    }

    pub(in crate::resident) fn fixture_for_handle(
        handle: pale_ports::ControlRecordCoordinate,
        selected: (ResidentRecordRefV1, OperationObservationV1),
        head: (ResidentRecordRefV1, OperationObservationV1),
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
        store: AuthorityStoreCoordinate,
    ) -> Self {
        Self::history_fixture_for_handle(handle, selected, None, None, head, checkpoint, store)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn history_fixture_for_handle(
        handle: pale_ports::ControlRecordCoordinate,
        selected: (ResidentRecordRefV1, OperationObservationV1),
        selected_predecessor: Option<(ResidentRecordRefV1, OperationObservationV1)>,
        context_binding: Option<(ResidentRecordRefV1, OperationObservationV1)>,
        head: (ResidentRecordRefV1, OperationObservationV1),
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
        store: AuthorityStoreCoordinate,
    ) -> Self {
        let current = checkpoint_recovery::checkpoint_recovery_fixture::current_checkpoint(
            &checkpoint.0,
            checkpoint.1.clone(),
            &store,
        );
        let head_checkpoint_ref = checkpoint.0.clone();
        let head_checkpoint = checkpoint.1.clone();
        Self {
            handle,
            selected,
            selected_predecessor,
            selected_predecessor_predecessor: None,
            context_binding,
            current,
            selected_checkpoint_ref: checkpoint.0,
            selected_checkpoint: checkpoint.1,
            head_checkpoint_ref,
            head_checkpoint,
            store,
            head: Some(head),
            immediate_predecessor: None,
        }
    }
}
