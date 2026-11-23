//! Physical journal and immutable-member reads in one consistent database view.

use super::*;
use pale_ports::{EventCustodyHistoryObservation, EventCustodyHistoryRead, EventTransitionMember};

/// Return physical evidence only; application recovery decodes the journal basis.
pub(super) fn read(
    connection: &Connection,
    request: &EventCustodyHistoryRead,
) -> Result<EventCustodyHistoryObservation, ControlStoreError> {
    require_store_binding(connection, request.store())?;
    let current_transaction = crate::store::transaction::read(connection, request.store())?;
    let current_journal = read_journal_head(connection)?;
    let expected_frame = frame::encode(
        request.entry().bytes().as_slice(),
        AuthorityByteClass::JournalEntry,
    )
    .map_err(|source| {
        ControlStoreError::with_source(ControlStoreErrorKind::RecoveryRequired, source)
    })?;
    // Compare in SQLite, without copying an unbounded stored frame into Rust.
    let frame_matches: Option<bool> = connection.query_row(
        "SELECT physical_frame = ?2 FROM pale_deployment_authority_journal WHERE journal_sequence = ?1",
        params![sqlite_integer(request.entry().sequence().get())?, expected_frame],
        |row| row.get(0),
    ).optional().map_err(|source| sqlite("read historical Event journal frame", source))?;

    let mut remaining = request.maximum_bytes() - request.entry().bytes().as_slice().len();
    let mut members = Vec::with_capacity(request.members().len());
    let mut any_member_present = false;
    let mut missing_member = false;
    for selected in request.members() {
        any_member_present |=
            record_reference_exists(connection, selected.coordinate().record_ref().as_str())?;
        if crate::deployment::record_body_length(connection, selected.coordinate())?
            .is_some_and(|bytes| bytes > remaining)
        {
            return Ok(EventCustodyHistoryObservation::BoundExceeded);
        }
        let Some(record) =
            crate::deployment::read_record_bounded(connection, selected.coordinate(), remaining)?
        else {
            missing_member = true;
            continue;
        };
        remaining -= record.canonical_record_jcs().as_slice().len();
        if record.schema_id().as_str() != selected.kind().required_schema_id() {
            return Ok(EventCustodyHistoryObservation::ConflictingOrPartial);
        }
        members.push(EventTransitionMember::new(selected.kind(), record));
    }
    if frame_matches.is_none()
        && !any_member_present
        && current_transaction.get() < request.transaction().get()
        && current_journal.is_none_or(|head| head < request.entry().sequence())
    {
        return Ok(EventCustodyHistoryObservation::Absent);
    }
    if frame_matches != Some(true)
        || missing_member
        || current_transaction.get() < request.transaction().get()
        || current_journal.is_none_or(|head| head < request.entry().sequence())
        || !crate::store::transaction::records_were_inserted_in(
            connection,
            crate::ControlStoreKind::DeploymentControl,
            request.store(),
            request.transaction(),
            members.iter().map(EventTransitionMember::record),
        )?
    {
        return Ok(EventCustodyHistoryObservation::ConflictingOrPartial);
    }
    Ok(EventCustodyHistoryObservation::Exact {
        members: members.into_boxed_slice(),
    })
}
