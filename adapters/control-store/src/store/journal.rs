//! Shared physical authority-journal custody inside an existing transaction.

use crate::{
    error::{sqlite, sqlite_integer},
    frame,
};
use pale_core::records::JournalSequenceV1;
use pale_ports::{
    AuthorityByteClass, ControlStoreError, ControlStoreErrorKind, PreparedAuthorityAppend,
};
use rusqlite::{params, Connection, OptionalExtension};

/// Append the transition's framed entry and advance the single journal head.
pub(crate) fn append_authority_entry(
    connection: &Connection,
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
            "INSERT INTO pale_deployment_authority_journal \
             (journal_sequence, physical_frame) VALUES (?1, ?2)",
            params![sequence, frame],
        )
        .map_err(|source| sqlite("append custody authority entry", source))?;
    let expected = append.expected_head().map(JournalSequenceV1::get);
    let expected = expected.map(sqlite_integer).transpose()?;
    let changed = connection
        .execute(
            "UPDATE pale_deployment_authority_head SET journal_sequence = ?1 \
             WHERE singleton = 1 AND journal_sequence IS ?2",
            params![sequence, expected],
        )
        .map_err(|source| sqlite("advance custody authority head", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Compare the complete framed journal successor byte-for-byte.
pub(crate) fn journal_successor_matches(
    connection: &Connection,
    append: &PreparedAuthorityAppend,
    maximum_journal_payload_bytes: usize,
) -> Result<bool, ControlStoreError> {
    if read_journal_head(connection)? != Some(append.entry().sequence()) {
        return Ok(false);
    }
    journal_entry_matches(connection, append, maximum_journal_payload_bytes)
}

/// Reject an unusable limit before reading storage or beginning a write.
pub(crate) fn require_journal_bound(
    append: &PreparedAuthorityAppend,
    maximum_journal_payload_bytes: usize,
) -> Result<(), ControlStoreError> {
    if maximum_journal_payload_bytes == 0
        || append.entry().bytes().as_slice().len() > maximum_journal_payload_bytes
    {
        return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
    }
    Ok(())
}

/// Compare validated payload bytes, not just a shared journal sequence.
/// Read frame length first; borrow the bounded `SQLite` blob without copying it.
pub(crate) fn journal_entry_matches(
    connection: &Connection,
    append: &PreparedAuthorityAppend,
    maximum_journal_payload_bytes: usize,
) -> Result<bool, ControlStoreError> {
    inspect_journal_entry(
        connection,
        append.entry().sequence(),
        maximum_journal_payload_bytes,
        |payload| payload == append.entry().bytes().as_slice(),
    )
}

/// Apply a bounded inspection to one validated, borrowed journal payload.
/// Authority and collection custody share the same bounded physical reader.
fn inspect_journal_entry(
    connection: &Connection,
    sequence: JournalSequenceV1,
    maximum_journal_payload_bytes: usize,
    inspect: impl FnOnce(&[u8]) -> bool,
) -> Result<bool, ControlStoreError> {
    inspect_frame(
        connection,
        "FROM pale_deployment_authority_journal WHERE journal_sequence = ?1",
        [sqlite_integer(sequence.get())?],
        maximum_journal_payload_bytes,
        inspect,
    )
}

/// Inspect a bounded journal frame selected by adapter-owned SQL and bound keys.
/// The caller holds the read or write transaction across both point reads.
pub(crate) fn inspect_frame(
    connection: &Connection,
    selection: &str,
    parameters: impl rusqlite::Params + Clone,
    maximum_journal_payload_bytes: usize,
    inspect: impl FnOnce(&[u8]) -> bool,
) -> Result<bool, ControlStoreError> {
    let maximum_frame_bytes = maximum_journal_payload_bytes
        .checked_add(frame::FRAME_HEADER_BYTES)
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::BoundExceeded))?;
    let length = connection
        .query_row(
            &format!("SELECT length(physical_frame) {selection}"),
            parameters.clone(),
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|source| sqlite("read custody journal frame length", source))?;
    let Some(length) = length else {
        return Ok(false);
    };
    let length = usize::try_from(length)
        .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    if length > maximum_frame_bytes {
        return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
    }
    let mut statement = connection
        .prepare(&format!("SELECT physical_frame {selection}"))
        .map_err(|source| sqlite("prepare custody journal frame read", source))?;
    let mut rows = statement
        .query(parameters)
        .map_err(|source| sqlite("read custody journal frame", source))?;
    let row = rows
        .next()
        .map_err(|source| sqlite("read custody journal frame row", source))?
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    let value = row.get_ref(0).map_err(|source| {
        ControlStoreError::with_source(ControlStoreErrorKind::RecoveryRequired, source)
    })?;
    let observed = value.as_blob().map_err(|source| {
        ControlStoreError::with_source(ControlStoreErrorKind::RecoveryRequired, source)
    })?;
    if observed.len() != length {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    let payload = frame::decode(
        observed,
        maximum_journal_payload_bytes,
        AuthorityByteClass::JournalEntry,
    )
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
    })?;
    if payload.is_empty() {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        ));
    }
    Ok(inspect(payload))
}

/// Compare the exact journal predecessor and absence of the successor row.
pub(crate) fn journal_predecessor_matches(
    connection: &Connection,
    append: &PreparedAuthorityAppend,
) -> Result<bool, ControlStoreError> {
    Ok(read_journal_head(connection)? == append.expected_head()
        && !journal_entry_exists(connection, append.entry().sequence())?)
}

/// Read the nullable singleton journal head.
pub(crate) fn read_journal_head(
    connection: &Connection,
) -> Result<Option<JournalSequenceV1>, ControlStoreError> {
    let value = connection
        .query_row(
            "SELECT journal_sequence FROM pale_deployment_authority_head WHERE singleton = 1",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|source| sqlite("read custody authority head", source))?
        .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
    value
        .map(|value| {
            u64::try_from(value)
                .ok()
                .and_then(|value| JournalSequenceV1::new(value).ok())
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
        })
        .transpose()
}

/// Test whether the shared journal already contains one sequence coordinate.
pub(crate) fn journal_entry_exists(
    connection: &Connection,
    sequence: JournalSequenceV1,
) -> Result<bool, ControlStoreError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_deployment_authority_journal \
             WHERE journal_sequence = ?1)",
            [sqlite_integer(sequence.get())?],
            |row| row.get(0),
        )
        .map_err(|source| sqlite("test custody authority entry", source))
}

/// Prove that the observed journal head closes one contiguous physical prefix.
pub(crate) fn journal_frontier_is_complete(
    connection: &Connection,
) -> Result<bool, ControlStoreError> {
    let head = read_journal_head(connection)?;
    let (count, minimum, maximum) = connection
        .query_row(
            "SELECT COUNT(*), MIN(journal_sequence), MAX(journal_sequence) \
             FROM pale_deployment_authority_journal",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .map_err(|source| sqlite("verify custody authority frontier", source))?;
    match head {
        None => Ok(count == 0 && minimum.is_none() && maximum.is_none()),
        Some(head) => Ok(count == sqlite_integer(head.get())?
            && minimum == Some(1)
            && maximum == Some(sqlite_integer(head.get())?)),
    }
}
