//! Handle-scoped collection history, atomically retained with operation custody.

use crate::{
    error::{sqlite, sqlite_integer},
    frame,
};
use pale_core::records::JournalSequenceV1;
use pale_ports::{
    AuthorityByteClass, ControlRecordCoordinate, ControlStoreError, ControlStoreErrorKind,
    PreparedAuthorityAppend,
};
use rusqlite::{params, Connection, OptionalExtension};

/// Append the transition's framed entry and advance the handle’s journal head.
pub(super) fn append_collection_entry(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
    append: &PreparedAuthorityAppend,
) -> Result<(), ControlStoreError> {
    let frame = frame::encode(
        append.entry().bytes().as_slice(),
        AuthorityByteClass::JournalEntry,
    )
    .map_err(|source| {
        ControlStoreError::with_source(ControlStoreErrorKind::RecoveryRequired, source)
    })?;
    let sequence = sqlite_integer(append.entry().sequence().get())?;
    connection
        .execute(
            "INSERT INTO pale_deployment_collection_journal \
             (handle_ref, handle_sha256, journal_sequence, physical_frame) VALUES (?3, ?4, ?1, ?2)",
            params![
                sequence,
                frame,
                handle.record_ref().as_str(),
                handle.record_sha256().as_str()
            ],
        )
        .map_err(|source| sqlite("append custody collection entry", source))?;
    let expected = append.expected_head().map(JournalSequenceV1::get);
    let expected = expected.map(sqlite_integer).transpose()?;
    connection.execute(
        "INSERT OR IGNORE INTO pale_deployment_collection_head (handle_ref, handle_sha256, journal_sequence) VALUES (?1, ?2, NULL)",
        params![handle.record_ref().as_str(), handle.record_sha256().as_str()],
    ).map_err(|source| sqlite("initialize collection head", source))?;
    let changed = connection
        .execute(
            "UPDATE pale_deployment_collection_head SET journal_sequence = ?1 \
             WHERE handle_ref = ?3 AND handle_sha256 = ?4 AND journal_sequence IS ?2",
            params![
                sequence,
                expected,
                handle.record_ref().as_str(),
                handle.record_sha256().as_str()
            ],
        )
        .map_err(|source| sqlite("advance custody collection head", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Compare the complete framed journal successor byte-for-byte.
pub(super) fn journal_successor_matches(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
    append: &PreparedAuthorityAppend,
    maximum_journal_payload_bytes: usize,
) -> Result<bool, ControlStoreError> {
    if read_journal_head(connection, handle)? != Some(append.entry().sequence()) {
        return Ok(false);
    }
    journal_entry_matches(connection, handle, append, maximum_journal_payload_bytes)
}

/// Compare validated payload bytes, not just a handle’s journal sequence.
/// Read frame length first; borrow the bounded `SQLite` blob without copying it.
pub(super) fn journal_entry_matches(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
    append: &PreparedAuthorityAppend,
    maximum_journal_payload_bytes: usize,
) -> Result<bool, ControlStoreError> {
    inspect_journal_entry(
        connection,
        handle,
        append.entry().sequence(),
        maximum_journal_payload_bytes,
        |payload| payload == append.entry().bytes().as_slice(),
    )
}

/// Read one exact unframed journal payload under an explicit byte ceiling.
///
/// This point read copies only the selected bounded payload. It does not scan
/// the journal or interpret application-owned bytes.
pub(super) fn read_journal_entry(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
    sequence: JournalSequenceV1,
    maximum_journal_payload_bytes: usize,
) -> Result<Option<pale_ports::BoundedAuthorityEntryBytes>, ControlStoreError> {
    if maximum_journal_payload_bytes == 0 {
        return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
    }
    let mut bytes = None;
    let exists = inspect_journal_entry(
        connection,
        handle,
        sequence,
        maximum_journal_payload_bytes,
        |payload| {
            bytes = Some(payload.to_vec());
            true
        },
    )?;
    if !exists {
        return Ok(None);
    }
    bytes
        .map(|bytes| {
            pale_ports::BoundedAuthorityEntryBytes::new(bytes, maximum_journal_payload_bytes)
        })
        .transpose()
        .map_err(|source| {
            let kind = if matches!(
                source.kind(),
                pale_ports::DurableAuthorityStoreErrorKind::BoundExceeded { .. }
            ) {
                ControlStoreErrorKind::BoundExceeded
            } else {
                ControlStoreErrorKind::RecoveryRequired
            };
            ControlStoreError::with_source(kind, source)
        })
}

/// Check only the current physical head and absence of rows beyond it.
/// This bounded point/range check does not attest the historical journal prefix;
/// recovery owns that validation through its bounded journal pages.
pub(super) fn require_journal_head_consistency(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
    maximum_journal_payload_bytes: usize,
) -> Result<(), ControlStoreError> {
    let head = read_journal_head(connection, handle)?;
    let beyond_head: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_collection_journal \
             WHERE handle_ref = ?2 AND handle_sha256 = ?3 AND journal_sequence > ?1 ORDER BY journal_sequence LIMIT 1)",
            params![head.map(|head| sqlite_integer(head.get())).transpose()?.unwrap_or(0), handle.record_ref().as_str(), handle.record_sha256().as_str()],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("compare custody collection head boundary", source))?;
    if beyond_head {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    if let Some(head) = head {
        if !inspect_journal_entry(
            connection,
            handle,
            head,
            maximum_journal_payload_bytes,
            |_| true,
        )? {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
    }
    Ok(())
}

/// Compare the exact journal predecessor and absence of the successor row.
pub(super) fn journal_predecessor_matches(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
    append: &PreparedAuthorityAppend,
) -> Result<bool, ControlStoreError> {
    Ok(
        read_journal_head(connection, handle)? == append.expected_head()
            && !journal_entry_exists(connection, handle, append.entry().sequence())?,
    )
}

/// Read the nullable head for this handle.
pub(super) fn read_journal_head(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
) -> Result<Option<JournalSequenceV1>, ControlStoreError> {
    let value = connection
        .query_row(
            "SELECT journal_sequence FROM pale_deployment_collection_head WHERE handle_ref = ?1 AND handle_sha256 = ?2",
            params![handle.record_ref().as_str(), handle.record_sha256().as_str()],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|source| sqlite("read custody collection head", source))?
        .flatten();
    value
        .map(|value| {
            u64::try_from(value)
                .ok()
                .and_then(|value| JournalSequenceV1::new(value).ok())
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
        })
        .transpose()
}

/// Test whether the handle’s journal already contains one sequence coordinate.
pub(super) fn journal_entry_exists(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
    sequence: JournalSequenceV1,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_collection_journal \
             WHERE handle_ref = ?2 AND handle_sha256 = ?3 AND journal_sequence = ?1)",
            params![
                sqlite_integer(sequence.get())?,
                handle.record_ref().as_str(),
                handle.record_sha256().as_str()
            ],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("test custody collection entry", source))
}

/// Prove that the observed journal head closes one contiguous physical prefix.
pub(super) fn journal_frontier_is_complete(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
) -> Result<bool, ControlStoreError> {
    let head = read_journal_head(connection, handle)?;
    let (count, minimum, maximum) = connection
        .query_row(
            "SELECT COUNT(*), MIN(journal_sequence), MAX(journal_sequence) \
             FROM pale_deployment_collection_journal WHERE handle_ref = ?1 AND handle_sha256 = ?2",
            params![
                handle.record_ref().as_str(),
                handle.record_sha256().as_str()
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .map_err(|source| sqlite("verify custody collection frontier", source))?;
    match head {
        None => Ok(count == 0 && minimum.is_none() && maximum.is_none()),
        Some(head) => Ok(count == sqlite_integer(head.get())?
            && minimum == Some(1)
            && maximum == Some(sqlite_integer(head.get())?)),
    }
}

/// Share frame bounds and integrity checks with authority custody, using a separate key.
fn inspect_journal_entry(
    connection: &Connection,
    handle: &ControlRecordCoordinate,
    sequence: JournalSequenceV1,
    maximum_journal_payload_bytes: usize,
    inspect: impl FnOnce(&[u8]) -> bool,
) -> Result<bool, ControlStoreError> {
    crate::store::journal::inspect_frame(
        connection,
        "FROM pale_deployment_collection_journal WHERE handle_ref = ?1 AND handle_sha256 = ?2 AND journal_sequence = ?3",
        params![handle.record_ref().as_str(), handle.record_sha256().as_str(), sqlite_integer(sequence.get())?],
        maximum_journal_payload_bytes,
        inspect,
    )
}
