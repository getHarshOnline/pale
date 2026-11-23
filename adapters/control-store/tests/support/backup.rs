//! Bounded qualification of logical `SQLite` copies into fresh inactive stores.

use std::time::{Duration, Instant};

use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};

use crate::candidate::CandidateReservation;
use crate::location::{ControlStoreLocation, StoreFileObservation};
use crate::store::{
    classify_connection_with_sources, classify_store_with_sources, ControlStoreIdentity,
    StoreOpenClassification, StoreOpenRejection,
};
use crate::{verify_embedded_storage_sources, ControlSqliteError, VerifiedStorageSources};

/// Positive page, step, and elapsed-time ceilings for one copy qualification.
///
/// The page and step ceilings bound calls into the native backup API. The
/// elapsed ceiling is an acceptance condition over the complete qualification,
/// including close, synchronization, classification, and parent sealing. It
/// cannot interrupt one blocking native call; a result returned after the
/// deadline is discarded instead of being accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BackupQualificationBudget {
    pages_per_step: i32,
    maximum_steps: u32,
    acceptance_deadline: Duration,
}

impl BackupQualificationBudget {
    /// Construct one strictly positive, native-representable budget.
    #[must_use]
    pub(crate) fn new(
        pages_per_step: u32,
        maximum_steps: u32,
        acceptance_deadline: Duration,
    ) -> Option<Self> {
        let pages_per_step = i32::try_from(pages_per_step).ok()?;
        (pages_per_step > 0 && maximum_steps > 0 && !acceptance_deadline.is_zero()).then_some(
            Self {
                pages_per_step,
                maximum_steps,
                acceptance_deadline,
            },
        )
    }
}

/// Native result observed immediately before a copy needed another step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackupStepObservation {
    /// Pages were copied but the logical copy remains incomplete.
    More,
    /// The native destination could not acquire its required lock.
    Busy,
    /// The native source connection was writing.
    Locked,
}

/// Exact native-step progress retained through destination finalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BackupProgress {
    completed_steps: u32,
    last_step: Option<BackupStepObservation>,
}

/// Failure to qualify one fresh inactive logical copy.
#[derive(Debug, thiserror::Error)]
pub(crate) enum BackupQualificationError {
    /// Existing control-store custody or physical storage failed.
    #[error(transparent)]
    Storage(#[from] ControlSqliteError),
    /// The source did not classify as a structurally copyable store.
    #[error("inactive-copy source is not structurally copyable: {0:?}")]
    SourceClassification(StoreOpenClassification),
    /// The destination did not reproduce the source's exact classification.
    #[error("inactive-copy destination differs from its source classification: {0:?}")]
    DestinationClassification(StoreOpenClassification),
    /// Source and destination resolve to the same canonical pathname.
    #[error("inactive-copy destination aliases its source")]
    DestinationAliasesSource,
    /// The native page-step ceiling was reached before completion.
    #[error("inactive-copy step budget exhausted after {completed_steps} steps")]
    StepBudgetExhausted {
        /// Native backup steps that returned before the ceiling.
        completed_steps: u32,
        /// Last retryable native outcome, if a step returned.
        last_step: Option<BackupStepObservation>,
    },
    /// The acceptance deadline elapsed before qualification completed.
    #[error("inactive-copy acceptance deadline elapsed after {completed_steps} steps")]
    AcceptanceDeadlineExceeded {
        /// Native backup steps that returned before the deadline was observed.
        completed_steps: u32,
        /// Last retryable native outcome, if a step returned.
        last_step: Option<BackupStepObservation>,
    },
    /// A future non-exhaustive native result was not interpreted as success.
    #[error("inactive-copy backup returned an unsupported native step result")]
    UnsupportedStepResult,
    /// One or both native connections did not close cleanly.
    #[error(
        "inactive-copy connections did not close cleanly: destination={destination:?}, source={source:?}"
    )]
    ConnectionCloseFailed {
        /// Destination close failure, when present.
        destination: Option<Box<ControlSqliteError>>,
        /// Source close failure, when present.
        source: Option<Box<ControlSqliteError>>,
    },
    /// Backup work and explicit connection closure both failed.
    #[error("inactive-copy backup failed before its connections closed: {failure}; {close}")]
    CopyAndCloseFailed {
        /// Original backup failure.
        failure: Box<BackupQualificationError>,
        /// Exact connection-close failures observed afterward.
        close: Box<BackupQualificationError>,
    },
    /// Candidate cleanup failed after an earlier qualification failure.
    #[error("inactive-copy qualification failed and its candidate could not be discarded")]
    CleanupFailed {
        /// Original qualification failure.
        failure: Box<BackupQualificationError>,
        /// Exact storage failure encountered while discarding the candidate.
        #[source]
        cleanup: ControlSqliteError,
    },
}

/// Copy one verified source snapshot into a distinct, freshly reserved inactive store.
///
/// The source connection establishes a read transaction before the native
/// backup begins. Success therefore identifies a `SQLite`-consistent logical
/// snapshot at that transaction, not physical byte equality and not equality
/// with writes committed after the snapshot. The destination is always a fresh
/// canonical location under a caller-retained private root. This function never
/// replaces or activates a live store and never selects the unresolved
/// backup/restore policy.
///
/// The same mechanism can qualify a backup and then copy that backup into a
/// second, distinct inactive recovery root. Each invocation independently
/// verifies the source identity, destination classification, and reservation
/// custody.
///
/// These checks inherit [`ControlStoreLocation`]'s and
/// [`CandidateReservation`]'s documented same-UID pathname-substitution
/// limitation. They are bounded path revalidation, not descriptor-bound
/// `SQLite` admission or an atomic exclusion of every same-principal actor.
///
/// # Errors
///
/// Returns [`BackupQualificationError`] when either location is inconsistent
/// with `expected`, the source is not structurally copyable, the destination is
/// occupied or aliases the source, native backup work exceeds a bound, a copied
/// store does not reproduce the source classification, or safe cleanup fails.
pub(crate) fn qualify_inactive_store_copy(
    source: &ControlStoreLocation,
    destination: &ControlStoreLocation,
    expected: &ControlStoreIdentity,
    budget: BackupQualificationBudget,
) -> Result<(), BackupQualificationError> {
    let started = Instant::now();
    if source.scope() != expected.scope() || destination.scope() != expected.scope() {
        return Err(ControlSqliteError::StoreLocationScopeMismatch.into());
    }
    if source.path() == destination.path() {
        return Err(BackupQualificationError::DestinationAliasesSource);
    }

    let sources = verify_embedded_storage_sources()?;
    let (source_connection, source_classification) =
        open_snapshot_source(source, expected, &sources)?;
    ensure_deadline(started, budget, 0, None)?;

    let reservation = CandidateReservation::reserve_store(destination)?;
    let mut destination_connection = match reservation.open_sqlite("open inactive-copy candidate") {
        Ok(connection) => connection,
        Err(failure) => {
            drop(source_connection);
            return Err(reject_candidate(
                reservation,
                BackupQualificationError::Storage(failure),
            ));
        }
    };

    let copy_result = match Backup::new(&source_connection, &mut destination_connection) {
        Ok(backup) => run_backup_steps(&backup, started, budget),
        Err(source) => Err(BackupQualificationError::Storage(
            ControlSqliteError::sqlite("begin inactive SQLite copy", source),
        )),
    };
    let close_result = close_copy_connections(destination_connection, source_connection);
    let progress = match (copy_result, close_result) {
        (Err(failure), Err(close)) => {
            return Err(reject_candidate(
                reservation,
                BackupQualificationError::CopyAndCloseFailed {
                    failure: Box::new(failure),
                    close: Box::new(close),
                },
            ));
        }
        (Err(failure), Ok(())) | (Ok(_), Err(failure)) => {
            return Err(reject_candidate(reservation, failure));
        }
        (Ok(progress), Ok(())) => progress,
    };

    let finalization = (|| {
        ensure_deadline(
            started,
            budget,
            progress.completed_steps,
            progress.last_step,
        )?;
        reservation.verify_no_sidecars("verify inactive-copy sidecars")?;
        reservation.sync_closed_main("synchronize inactive-copy main file")?;
        ensure_deadline(
            started,
            budget,
            progress.completed_steps,
            progress.last_step,
        )?;
        let destination_classification =
            classify_store_with_sources(destination, expected, &sources)?;
        if destination_classification != source_classification {
            return Err(BackupQualificationError::DestinationClassification(
                destination_classification,
            ));
        }
        ensure_deadline(
            started,
            budget,
            progress.completed_steps,
            progress.last_step,
        )?;
        reservation.seal_identity_and_parent()?;
        ensure_deadline(
            started,
            budget,
            progress.completed_steps,
            progress.last_step,
        )
    })();
    if let Err(failure) = finalization {
        return Err(reject_candidate(reservation, failure));
    }

    drop(reservation);
    Ok(())
}

/// Open the exact source connection, pin its snapshot, then classify that snapshot.
fn open_snapshot_source(
    source: &ControlStoreLocation,
    expected: &ControlStoreIdentity,
    sources: &VerifiedStorageSources,
) -> Result<(Connection, StoreOpenClassification), BackupQualificationError> {
    let file_identity = match source.entry()? {
        StoreFileObservation::Missing => {
            return Err(BackupQualificationError::SourceClassification(
                StoreOpenClassification::Missing,
            ));
        }
        StoreFileObservation::Unsafe => {
            return Err(BackupQualificationError::SourceClassification(
                StoreOpenClassification::Rejected(StoreOpenRejection::UnsafeEntryKind),
            ));
        }
        StoreFileObservation::Regular { empty: true, .. } => {
            return Err(BackupQualificationError::SourceClassification(
                StoreOpenClassification::Empty,
            ));
        }
        StoreFileObservation::Regular {
            identity,
            empty: false,
        } => identity,
    };
    let connection = source.open_existing_sqlite(
        OpenFlags::SQLITE_OPEN_READ_ONLY,
        file_identity,
        "open inactive-copy source",
    )?;
    connection
        .execute_batch("BEGIN DEFERRED")
        .map_err(|source| ControlSqliteError::sqlite("begin inactive-copy snapshot", source))?;
    connection
        .query_row("SELECT COUNT(*) FROM sqlite_schema", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|source| ControlSqliteError::sqlite("pin inactive-copy snapshot", source))?;
    let classification = classify_connection_with_sources(&connection, expected, sources)?;
    if !matches!(
        classification,
        StoreOpenClassification::ProfileUnselected { .. } | StoreOpenClassification::Ready
    ) {
        return Err(BackupQualificationError::SourceClassification(
            classification,
        ));
    }
    Ok((connection, classification))
}

/// Explicitly close both native connections and retain every close failure.
fn close_copy_connections(
    destination: Connection,
    source: Connection,
) -> Result<(), BackupQualificationError> {
    let destination = close_connection(destination, "close inactive-copy candidate").err();
    let source = close_connection(source, "close inactive-copy source").err();
    if destination.is_none() && source.is_none() {
        Ok(())
    } else {
        Err(BackupQualificationError::ConnectionCloseFailed {
            destination: destination.map(Box::new),
            source: source.map(Box::new),
        })
    }
}

/// Close one connection without losing the native error or leaking its owner.
fn close_connection(
    connection: Connection,
    operation: &'static str,
) -> Result<(), ControlSqliteError> {
    connection.close().map_err(|(connection, source)| {
        drop(connection);
        ControlSqliteError::sqlite(operation, source)
    })
}

/// Execute native backup steps under exact call-count and acceptance ceilings.
fn run_backup_steps(
    backup: &Backup<'_, '_>,
    started: Instant,
    budget: BackupQualificationBudget,
) -> Result<BackupProgress, BackupQualificationError> {
    let mut completed_steps = 0;
    let mut last_step = None;
    loop {
        ensure_deadline(started, budget, completed_steps, last_step)?;
        if completed_steps == budget.maximum_steps {
            return Err(BackupQualificationError::StepBudgetExhausted {
                completed_steps,
                last_step,
            });
        }
        let result = backup.step(budget.pages_per_step).map_err(|source| {
            BackupQualificationError::Storage(ControlSqliteError::sqlite(
                "copy inactive SQLite pages",
                source,
            ))
        })?;
        completed_steps += 1;
        match observe_native_step(result)? {
            None => {
                ensure_deadline(started, budget, completed_steps, last_step)?;
                return Ok(BackupProgress {
                    completed_steps,
                    last_step,
                });
            }
            Some(observation) => last_step = Some(observation),
        }
        ensure_deadline(started, budget, completed_steps, last_step)?;
    }
}

/// Reduce the pinned native result vocabulary without treating unknowns as success.
fn observe_native_step(
    result: StepResult,
) -> Result<Option<BackupStepObservation>, BackupQualificationError> {
    match result {
        StepResult::Done => Ok(None),
        StepResult::More => Ok(Some(BackupStepObservation::More)),
        StepResult::Busy => Ok(Some(BackupStepObservation::Busy)),
        StepResult::Locked => Ok(Some(BackupStepObservation::Locked)),
        _ => Err(BackupQualificationError::UnsupportedStepResult),
    }
}

/// Reject an elapsed acceptance ceiling without implying native interruption.
fn ensure_deadline(
    started: Instant,
    budget: BackupQualificationBudget,
    completed_steps: u32,
    last_step: Option<BackupStepObservation>,
) -> Result<(), BackupQualificationError> {
    if started.elapsed() < budget.acceptance_deadline {
        Ok(())
    } else {
        Err(BackupQualificationError::AcceptanceDeadlineExceeded {
            completed_steps,
            last_step,
        })
    }
}

/// Discard only the reserved main file while retaining both failure causes.
fn reject_candidate(
    reservation: CandidateReservation,
    failure: BackupQualificationError,
) -> BackupQualificationError {
    match reservation.discard() {
        Ok(()) => failure,
        Err(cleanup) => BackupQualificationError::CleanupFailed {
            failure: Box::new(failure),
            cleanup,
        },
    }
}

#[cfg(test)]
#[path = "../internal/backup.rs"]
mod tests;
