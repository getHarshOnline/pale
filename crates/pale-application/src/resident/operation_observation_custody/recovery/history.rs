//! Bounded raw history traversal shared by protected reads and pending-expiry repair.

use super::*;

pub(in crate::resident) fn read_history(
    port: &mut dyn OperationCustodyPort,
    store: &AuthorityStoreCoordinate,
    mut record: ControlRecord,
    limits: OperationObservationRecoveryLimits,
) -> Result<Vec<(ControlRecord, OperationObservationWireV1)>, OperationObservationReadmissionError>
{
    let schema = pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID)?;
    let mut remaining_bytes = limits.maximum_chain_bytes;
    let mut expected_revision = None;
    let mut history: Vec<(ControlRecord, OperationObservationWireV1)> = Vec::new();
    loop {
        if history.len() == limits.maximum_revisions {
            return Err(OperationObservationReadmissionError::Bounds);
        }
        let bytes = record.canonical_record_jcs().as_slice();
        if bytes.len() > limits.maximum_record_bytes {
            return Err(OperationObservationReadmissionError::Bounds);
        }
        remaining_bytes = remaining_bytes
            .checked_sub(bytes.len())
            .ok_or(OperationObservationReadmissionError::Bounds)?;
        if record.schema_id() != &schema.schema || record.schema_sha256() != &schema.schema_sha256 {
            return Err(OperationObservationReadmissionError::Association);
        }
        let wire: OperationObservationWireV1 = serde_json::from_value(canon::parse_strict(bytes)?)?;
        let revision = wire.revision().get();
        if expected_revision.is_some_and(|expected| expected != revision) {
            return Err(OperationObservationReadmissionError::Lineage);
        }
        let predecessor = match (revision, wire.predecessor_ref(), wire.predecessor_sha256()) {
            (1, None, None) => None,
            (revision, Some(reference), Some(digest)) if revision > 1 => Some(
                crate::resident::record_projection::coordinate(reference, digest.as_str())
                    .map_err(|_| OperationObservationReadmissionError::Association)?,
            ),
            _ => return Err(OperationObservationReadmissionError::Lineage),
        };
        history.push((record, wire));
        let Some(coordinate) = predecessor else {
            break;
        };
        // Strictly decreasing claimed revisions reject a cycle/gap before a
        // further lookup. Actual digest and predecessor authority follow below.
        expected_revision = Some(revision - 1);
        if history.len() == limits.maximum_revisions || remaining_bytes == 0 {
            return Err(OperationObservationReadmissionError::Bounds);
        }
        record = port
            .read_operation_record(
                store,
                &coordinate,
                limits.maximum_record_bytes.min(remaining_bytes),
            )?
            .ok_or(OperationObservationReadmissionError::Missing)?;
        if record.coordinate() != &coordinate {
            return Err(OperationObservationReadmissionError::Association);
        }
    }
    Ok(history)
}
