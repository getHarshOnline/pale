//! Deployment-bound `SQLite` custody for opaque authority bytes.
//!
//! This module implements [`DurableAuthorityStorePort`] without interpreting
//! application-owned journal or checkpoint records. Its physical frame is a
//! tested candidate only: construction remains crate-private while the complete
//! durability profile is unresolved.

use std::sync::{Mutex, MutexGuard};

mod event_custody;
mod operation_custody;

use pale_core::records::JournalSequenceV1;
use pale_ports::{
    AuthorityByteClass, AuthorityStoreCoordinate, BoundedAuthorityEntryBytes,
    BoundedRecoveryCheckpointBytes, ControlStoreError, ControlStoreErrorKind,
    DurableAuthorityStoreError, DurableAuthorityStoreErrorKind, DurableAuthorityStorePort,
    JournalTailPage, JournalTailPageBoundary, JournalTailPageRead, PreparedAuthorityAppend,
    PreparedRecoveryCheckpoint, RecoveryCheckpointCandidate, RecoveryCheckpointPhysicalRejection,
    RecoveryCheckpointRead, SequencedAuthorityEntry, StoredRecoveryCheckpoint,
};
use rusqlite::{params, Connection, OpenFlags, Transaction, TransactionBehavior};

use crate::frame::{self, FRAME_HEADER_BYTES};
use crate::location::{ControlStoreLocation, StoreFileObservation};
use crate::store::{
    classify_connection_with_sources, ControlStoreIdentity, StoreOpenClassification,
};
use crate::{verify_embedded_storage_sources, ControlSqliteError, ControlStoreKind};

/// Deployment-only physical adapter for the neutral durable-authority port.
///
/// No public constructor exists while the deployment path, synchronization
/// profile, hard bounds, and checkpoint policy remain unselected. The type and
/// its port implementation are executable structural evidence, not resident
/// admission or authority.
pub(crate) struct DeploymentControlSqliteStore {
    coordinate: AuthorityStoreCoordinate,
    connection: Mutex<Connection>,
}

impl DeploymentControlSqliteStore {
    /// Open an initialized deployment store solely as an unadmitted structural candidate.
    ///
    /// This crate-private seam exists for confined adapter and differential tests.
    /// Future runtime assembly must introduce a distinct admitted constructor
    /// after it can prove the selected storage profile.
    pub(crate) fn open_unadmitted_candidate(
        location: &ControlStoreLocation,
        coordinate: AuthorityStoreCoordinate,
    ) -> Result<Self, ControlSqliteError> {
        if location.scope().kind() != ControlStoreKind::DeploymentControl {
            return Err(ControlSqliteError::SourceInvariant(
                "deployment authority candidate scope",
            ));
        }
        let sources = verify_embedded_storage_sources()?;
        let identity = ControlStoreIdentity::new(
            location.scope().clone(),
            coordinate.store_id().clone(),
            coordinate.incarnation_id().clone(),
        );
        let file_identity = match location.entry()? {
            StoreFileObservation::Regular {
                identity,
                empty: false,
            } => identity,
            _ => {
                return Err(ControlSqliteError::SourceInvariant(
                    "deployment authority candidate entry",
                ));
            }
        };
        let connection = location.open_existing_sqlite(
            OpenFlags::SQLITE_OPEN_READ_WRITE,
            file_identity,
            "open deployment authority candidate",
        )?;
        match classify_connection_with_sources(&connection, &identity, &sources)? {
            StoreOpenClassification::ProfileUnselected { .. } => Ok(Self {
                coordinate,
                connection: Mutex::new(connection),
            }),
            _ => Err(ControlSqliteError::SourceInvariant(
                "deployment authority candidate classification",
            )),
        }
    }

    /// Reject cross-store or cross-incarnation substitution before physical I/O.
    fn check_coordinate(
        &self,
        supplied: &AuthorityStoreCoordinate,
    ) -> Result<(), DurableAuthorityStoreError> {
        if supplied == &self.coordinate {
            Ok(())
        } else {
            Err(DurableAuthorityStoreErrorKind::Unavailable.into())
        }
    }

    /// Acquire this adapter's sole process-local connection owner.
    fn connection(&self) -> Result<MutexGuard<'_, Connection>, DurableAuthorityStoreError> {
        self.connection
            .lock()
            .map_err(|_| DurableAuthorityStoreErrorKind::Unavailable.into())
    }

    /// Reject session-body access through another physical store incarnation.
    pub(crate) fn check_session_lease_coordinate(
        &self,
        supplied: &AuthorityStoreCoordinate,
    ) -> Result<(), ControlStoreError> {
        if supplied == &self.coordinate {
            Ok(())
        } else {
            Err(ControlStoreError::new(ControlStoreErrorKind::Storage))
        }
    }

    /// Acquire the same connection owner for session and Event record custody.
    pub(crate) fn record_custody_connection(
        &self,
    ) -> Result<MutexGuard<'_, Connection>, ControlStoreError> {
        self.connection
            .lock()
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::Storage))
    }
}

impl DurableAuthorityStorePort for DeploymentControlSqliteStore {
    fn compare_and_append(
        &self,
        append: &PreparedAuthorityAppend,
    ) -> Result<(), DurableAuthorityStoreError> {
        self.check_coordinate(append.store())?;
        let frame = frame::encode(
            append.entry().bytes().as_slice(),
            AuthorityByteClass::JournalEntry,
        )?;
        let expected = append.expected_head();
        let sequence = sequence_to_sql(append.entry().sequence());
        let mut connection = self.connection()?;
        let transaction = begin_immediate(&mut connection, AuthorityByteClass::JournalEntry)?;
        if read_head(&transaction)? != expected {
            return Err(DurableAuthorityStoreErrorKind::Conflict.into());
        }
        let mutation = crate::store::transaction::prepare(&transaction, append.store())
            .map_err(|error| map_transaction_error(error, AuthorityByteClass::JournalEntry))?;
        crate::store::transaction::advance(&transaction, &mutation)
            .map_err(|error| map_transaction_error(error, AuthorityByteClass::JournalEntry))?;
        transaction
            .execute(
                "INSERT INTO pale_deployment_authority_journal \
                 (journal_sequence, physical_frame) VALUES (?1, ?2)",
                params![sequence, frame],
            )
            .map_err(|error| map_uncommitted(error, AuthorityByteClass::JournalEntry))?;
        let changed = transaction
            .execute(
                "UPDATE pale_deployment_authority_head SET journal_sequence = ?1 \
                 WHERE singleton = 1 AND journal_sequence IS ?2",
                params![sequence, expected.map(sequence_to_sql)],
            )
            .map_err(|error| map_uncommitted(error, AuthorityByteClass::JournalEntry))?;
        if changed != 1 {
            return Err(DurableAuthorityStoreErrorKind::Conflict.into());
        }
        commit_mutation(transaction)
    }

    fn compare_and_store_checkpoint(
        &self,
        prepared: &PreparedRecoveryCheckpoint,
    ) -> Result<(), DurableAuthorityStoreError> {
        self.check_coordinate(prepared.store())?;
        let sequence = sequence_to_sql(prepared.checkpoint().through_sequence());
        let frame = frame::encode(
            prepared.checkpoint().bytes().as_slice(),
            AuthorityByteClass::RecoveryCheckpoint,
        )?;
        let mut connection = self.connection()?;
        let transaction = begin_immediate(&mut connection, AuthorityByteClass::RecoveryCheckpoint)?;
        if read_head(&transaction)? != Some(prepared.expected_head()) {
            return Err(DurableAuthorityStoreErrorKind::Conflict.into());
        }
        crate::store::transaction::read(&transaction, prepared.store()).map_err(|error| {
            map_transaction_error(error, AuthorityByteClass::RecoveryCheckpoint)
        })?;
        let inserted = transaction
            .execute(
                "INSERT INTO pale_deployment_recovery_checkpoint \
                 (through_journal_sequence, physical_frame) VALUES (?1, ?2) \
                 ON CONFLICT(through_journal_sequence) DO NOTHING",
                params![sequence, frame],
            )
            .map_err(|error| map_uncommitted(error, AuthorityByteClass::RecoveryCheckpoint))?;
        if inserted == 1 {
            let mutation = crate::store::transaction::prepare(&transaction, prepared.store())
                .map_err(|error| {
                    map_transaction_error(error, AuthorityByteClass::RecoveryCheckpoint)
                })?;
            crate::store::transaction::advance(&transaction, &mutation).map_err(|error| {
                map_transaction_error(error, AuthorityByteClass::RecoveryCheckpoint)
            })?;
        } else {
            let stored: Vec<u8> = transaction
                .query_row(
                    "SELECT physical_frame FROM pale_deployment_recovery_checkpoint \
                     WHERE through_journal_sequence = ?1",
                    [sequence],
                    |row| row.get(0),
                )
                .map_err(|error| map_uncommitted(error, AuthorityByteClass::RecoveryCheckpoint))?;
            if stored != frame {
                return Err(DurableAuthorityStoreErrorKind::Conflict.into());
            }
        }
        commit_mutation(transaction)
    }

    fn read_recovery_checkpoints(
        &self,
        request: &RecoveryCheckpointRead,
    ) -> Result<Vec<RecoveryCheckpointCandidate>, DurableAuthorityStoreError> {
        self.check_coordinate(request.store())?;
        let limits = request.limits();
        let limit = sql_limit(
            limits.maximum_checkpoint_candidates(),
            AuthorityByteClass::RecoveryCheckpoint,
        )?;
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT through_journal_sequence, length(physical_frame), physical_frame \
                 FROM pale_deployment_recovery_checkpoint \
                 ORDER BY through_journal_sequence DESC LIMIT ?1",
            )
            .map_err(|error| map_uncommitted(error, AuthorityByteClass::RecoveryCheckpoint))?;
        let mut rows = statement
            .query([limit])
            .map_err(|error| map_uncommitted(error, AuthorityByteClass::RecoveryCheckpoint))?;
        let mut checkpoints = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|error| map_uncommitted(error, AuthorityByteClass::RecoveryCheckpoint))?
        {
            let sequence = decode_sequence(
                row.get(0).map_err(|error| {
                    map_uncommitted(error, AuthorityByteClass::RecoveryCheckpoint)
                })?,
                AuthorityByteClass::RecoveryCheckpoint,
            )?;
            let physical_length: i64 = row
                .get(1)
                .map_err(|error| map_uncommitted(error, AuthorityByteClass::RecoveryCheckpoint))?;
            if let Err(error) = check_physical_length(
                physical_length,
                limits.maximum_checkpoint_bytes(),
                AuthorityByteClass::RecoveryCheckpoint,
            ) {
                checkpoints.push(RecoveryCheckpointCandidate::Rejected {
                    through_sequence: sequence,
                    reason: checkpoint_rejection(error)?,
                });
                continue;
            }
            let physical_frame: Vec<u8> = row
                .get(2)
                .map_err(|error| map_uncommitted(error, AuthorityByteClass::RecoveryCheckpoint))?;
            let payload = match frame::decode(
                &physical_frame,
                limits.maximum_checkpoint_bytes(),
                AuthorityByteClass::RecoveryCheckpoint,
            ) {
                Ok(payload) => payload,
                Err(error) => {
                    checkpoints.push(RecoveryCheckpointCandidate::Rejected {
                        through_sequence: sequence,
                        reason: checkpoint_rejection(error)?,
                    });
                    continue;
                }
            };
            let bytes = match BoundedRecoveryCheckpointBytes::new(
                payload.to_vec(),
                limits.maximum_checkpoint_bytes(),
            ) {
                Ok(bytes) => bytes,
                Err(error) => {
                    checkpoints.push(RecoveryCheckpointCandidate::Rejected {
                        through_sequence: sequence,
                        reason: checkpoint_rejection(error)?,
                    });
                    continue;
                }
            };
            checkpoints.push(RecoveryCheckpointCandidate::Trusted(
                StoredRecoveryCheckpoint::new(sequence, bytes),
            ));
        }
        Ok(checkpoints)
    }

    fn read_journal_head(
        &self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<Option<JournalSequenceV1>, DurableAuthorityStoreError> {
        self.check_coordinate(store)?;
        let connection = self.connection()?;
        read_head(&connection)
    }

    fn read_journal_page(
        &self,
        request: &JournalTailPageRead,
    ) -> Result<JournalTailPage, DurableAuthorityStoreError> {
        self.check_coordinate(request.store())?;
        let connection = self.connection()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| map_uncommitted(error, AuthorityByteClass::JournalEntry))?;
        if read_head(&transaction)?.is_none_or(|head| head < request.through()) {
            return Err(
                DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry).into(),
            );
        }

        let maximum = request.maximum_page_entries();
        if maximum == 0 {
            return Err(DurableAuthorityStoreErrorKind::BoundExceeded {
                class: AuthorityByteClass::JournalEntry,
            }
            .into());
        }
        let after = request.after().map_or(0, JournalSequenceV1::get);
        let mut entries = Vec::with_capacity(maximum);
        let mut quarantine = None;
        {
            let mut statement = transaction
                .prepare(
                    "SELECT journal_sequence, length(physical_frame), physical_frame \
                     FROM pale_deployment_authority_journal \
                     WHERE journal_sequence > ?1 AND journal_sequence <= ?2 \
                     ORDER BY journal_sequence ASC LIMIT ?3",
                )
                .map_err(|error| map_uncommitted(error, AuthorityByteClass::JournalEntry))?;
            let mut rows = statement
                .query(params![
                    u64_to_sql(after)?,
                    sequence_to_sql(request.through()),
                    sql_limit(maximum, AuthorityByteClass::JournalEntry)?,
                ])
                .map_err(|error| map_uncommitted(error, AuthorityByteClass::JournalEntry))?;
            let mut expected = request.after().map_or_else(
                || JournalSequenceV1::new(1).ok(),
                JournalSequenceV1::checked_successor,
            );
            while let Some(row) = rows
                .next()
                .map_err(|error| map_uncommitted(error, AuthorityByteClass::JournalEntry))?
            {
                let observed = decode_sequence(
                    row.get(0).map_err(|error| {
                        map_uncommitted(error, AuthorityByteClass::JournalEntry)
                    })?,
                    AuthorityByteClass::JournalEntry,
                )?;
                if Some(observed) != expected {
                    quarantine = expected.or(Some(observed));
                    break;
                }
                let physical_length: i64 = row
                    .get(1)
                    .map_err(|error| map_uncommitted(error, AuthorityByteClass::JournalEntry))?;
                match check_physical_length(
                    physical_length,
                    request.limits().maximum_entry_bytes(),
                    AuthorityByteClass::JournalEntry,
                ) {
                    Ok(()) => {}
                    Err(error)
                        if matches!(
                            error.kind(),
                            DurableAuthorityStoreErrorKind::Corrupt(
                                AuthorityByteClass::JournalEntry
                            )
                        ) =>
                    {
                        quarantine = Some(observed);
                        break;
                    }
                    Err(error) => return Err(error),
                }
                let physical_frame: Vec<u8> = row
                    .get(2)
                    .map_err(|error| map_uncommitted(error, AuthorityByteClass::JournalEntry))?;
                let payload = match frame::decode(
                    &physical_frame,
                    request.limits().maximum_entry_bytes(),
                    AuthorityByteClass::JournalEntry,
                ) {
                    Ok(payload) => payload,
                    Err(error)
                        if matches!(
                            error.kind(),
                            DurableAuthorityStoreErrorKind::Corrupt(
                                AuthorityByteClass::JournalEntry
                            )
                        ) =>
                    {
                        quarantine = Some(observed);
                        break;
                    }
                    Err(error) => return Err(error),
                };
                entries.push(SequencedAuthorityEntry::new(
                    observed,
                    BoundedAuthorityEntryBytes::new(
                        payload.to_vec(),
                        request.limits().maximum_entry_bytes(),
                    )?,
                ));
                expected = observed.checked_successor();
            }
            if quarantine.is_none()
                && entries.last().map(SequencedAuthorityEntry::sequence) != Some(request.through())
                && entries.len() < maximum
            {
                quarantine = expected;
            }
        }
        transaction
            .commit()
            .map_err(|error| map_uncommitted(error, AuthorityByteClass::JournalEntry))?;

        let boundary = match quarantine {
            Some(first_untrusted_sequence) => JournalTailPageBoundary::QuarantinedFrom {
                first_untrusted_sequence,
            },
            None if entries.last().map(SequencedAuthorityEntry::sequence)
                == Some(request.through()) =>
            {
                JournalTailPageBoundary::Complete
            }
            None if entries.len() < request.remaining_entries() => JournalTailPageBoundary::More {
                resume_after: entries
                    .last()
                    .map(SequencedAuthorityEntry::sequence)
                    .ok_or(DurableAuthorityStoreErrorKind::Corrupt(
                        AuthorityByteClass::JournalEntry,
                    ))?,
            },
            None => {
                return Err(DurableAuthorityStoreErrorKind::BoundExceeded {
                    class: AuthorityByteClass::JournalEntry,
                }
                .into())
            }
        };
        JournalTailPage::new(request, entries, boundary)
    }
}

/// Convert one candidate-local physical rejection without hiding store failure.
fn checkpoint_rejection(
    error: DurableAuthorityStoreError,
) -> Result<RecoveryCheckpointPhysicalRejection, DurableAuthorityStoreError> {
    match error.kind() {
        DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::RecoveryCheckpoint) => {
            Ok(RecoveryCheckpointPhysicalRejection::Integrity)
        }
        DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::RecoveryCheckpoint,
        } => Ok(RecoveryCheckpointPhysicalRejection::BoundExceeded),
        _ => Err(error),
    }
}

/// Begin one exact-head mutation transaction without hidden retry.
fn begin_immediate(
    connection: &mut Connection,
    class: AuthorityByteClass,
) -> Result<Transaction<'_>, DurableAuthorityStoreError> {
    connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| map_uncommitted(error, class))
}

/// Finalize either mutation without turning a native error into proof of rollback.
///
/// Journal appends and checkpoint projections share this physical boundary;
/// their callers apply the distinct append and checkpoint recovery laws.
fn commit_mutation(transaction: Transaction<'_>) -> Result<(), DurableAuthorityStoreError> {
    transaction.commit().map_err(|source| {
        DurableAuthorityStoreError::with_source(
            DurableAuthorityStoreErrorKind::AmbiguousCommit,
            source,
        )
    })
}

/// Read the nullable singleton journal head inside the caller's transaction.
fn read_head(
    connection: &Connection,
) -> Result<Option<JournalSequenceV1>, DurableAuthorityStoreError> {
    let value: Option<i64> = match connection.query_row(
        "SELECT journal_sequence FROM pale_deployment_authority_head WHERE singleton = 1",
        [],
        |row| row.get(0),
    ) {
        Ok(value) => value,
        Err(source @ rusqlite::Error::QueryReturnedNoRows) => {
            return Err(DurableAuthorityStoreError::with_source(
                DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry),
                source,
            ));
        }
        Err(error) => return Err(map_uncommitted(error, AuthorityByteClass::JournalEntry)),
    };
    value
        .map(|value| decode_sequence(value, AuthorityByteClass::JournalEntry))
        .transpose()
}

/// Convert one portable sequence to `SQLite`'s signed integer domain.
fn sequence_to_sql(sequence: JournalSequenceV1) -> i64 {
    i64::try_from(sequence.get()).expect("portable JCS integer always fits in i64")
}

/// Convert an arbitrary read frontier to `SQLite`'s signed integer domain.
fn u64_to_sql(value: u64) -> Result<i64, DurableAuthorityStoreError> {
    i64::try_from(value).map_err(|_| {
        DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::JournalEntry,
        }
        .into()
    })
}

/// Admit an untrusted `SQLite` sequence into the closed portable coordinate.
fn decode_sequence(
    value: i64,
    class: AuthorityByteClass,
) -> Result<JournalSequenceV1, DurableAuthorityStoreError> {
    let value = u64::try_from(value).map_err(|_| DurableAuthorityStoreErrorKind::Corrupt(class))?;
    JournalSequenceV1::new(value).map_err(|_| DurableAuthorityStoreErrorKind::Corrupt(class).into())
}

/// Convert a bounded record count into a non-overflowing `SQLite` limit.
fn sql_limit(value: usize, class: AuthorityByteClass) -> Result<i64, DurableAuthorityStoreError> {
    i64::try_from(value).map_err(|_| DurableAuthorityStoreErrorKind::BoundExceeded { class }.into())
}

/// Reject a physically oversized frame before allocating its BLOB payload.
fn check_physical_length(
    physical_length: i64,
    maximum_payload_bytes: usize,
    class: AuthorityByteClass,
) -> Result<(), DurableAuthorityStoreError> {
    let maximum = FRAME_HEADER_BYTES
        .checked_add(maximum_payload_bytes)
        .ok_or(DurableAuthorityStoreErrorKind::BoundExceeded { class })?;
    let observed = usize::try_from(physical_length)
        .map_err(|_| DurableAuthorityStoreErrorKind::Corrupt(class))?;
    if observed > maximum {
        Err(DurableAuthorityStoreErrorKind::BoundExceeded { class }.into())
    } else {
        Ok(())
    }
}

/// Classify a read, transaction-start, or pre-commit failure with its exact cause.
fn map_uncommitted(
    error: rusqlite::Error,
    class: AuthorityByteClass,
) -> DurableAuthorityStoreError {
    let kind = if crate::error::sqlite_error_is_corruption(&error)
        || error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation)
    {
        DurableAuthorityStoreErrorKind::Corrupt(class)
    } else {
        DurableAuthorityStoreErrorKind::Unavailable
    };
    DurableAuthorityStoreError::with_source(kind, error)
}

/// Preserve the authority port's existing failure classes and native clock cause.
/// A saturated valid head is unavailable for new mutations, not corrupt bytes.
fn map_transaction_error(
    error: ControlStoreError,
    class: AuthorityByteClass,
) -> DurableAuthorityStoreError {
    let kind = match error.kind() {
        ControlStoreErrorKind::BoundExceeded => {
            DurableAuthorityStoreErrorKind::BoundExceeded { class }
        }
        ControlStoreErrorKind::CandidateConflict => DurableAuthorityStoreErrorKind::Conflict,
        ControlStoreErrorKind::RecoveryRequired => DurableAuthorityStoreErrorKind::Corrupt(class),
        ControlStoreErrorKind::OutcomeUnknown => DurableAuthorityStoreErrorKind::AmbiguousCommit,
        ControlStoreErrorKind::TransactionExhausted | ControlStoreErrorKind::Storage => {
            DurableAuthorityStoreErrorKind::Unavailable
        }
    };
    DurableAuthorityStoreError::with_source(kind, error)
}

#[cfg(test)]
#[path = "../tests/internal/authority.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/internal/resident_application.rs"]
mod resident_application_tests;
