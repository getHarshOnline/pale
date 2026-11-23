//! Checkpoint values used by protected-recovery tests.

use super::*;

pub(in crate::resident) fn current_checkpoint(
    reference: &ResidentRecordRefV1,
    record: ControlStateCheckpointV1,
    store: &AuthorityStoreCoordinate,
) -> CurrentOperationCheckpoint {
    let projected = crate::resident::record_projection::coordinate(
        reference,
        record.canonical_control_checkpoint_sha256().as_str(),
    )
    .expect("fixture checkpoint coordinate");
    let head = pale_ports::ControlCheckpointHead::new(
        record.checkpoint_domain_id().clone(),
        *record.domain_epoch(),
        *record.domain_sequence(),
        *record.committed_transaction_sequence(),
        projected,
    );
    let coordinate = OperationAccumulatorCheckpoint::new(
        store.clone(),
        record.checkpoint_profile_ref().clone(),
        record.checkpoint_profile_sha256().clone(),
        head.clone(),
        record.state_root_sha256().clone(),
        record.unresolved_custody_root_sha256().clone(),
    );
    let anchor = AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
        store.store_id().clone(),
        store.incarnation_id().clone(),
        head.checkpoint_domain_id().clone(),
        head.domain_epoch(),
        head.domain_sequence(),
        head.coordinate().clone(),
    ));
    CurrentOperationCheckpoint {
        record_bytes: canon::to_canonical_string(&record)
            .expect("fixture checkpoint encoding")
            .len(),
        record,
        coordinate,
        anchor,
    }
}
