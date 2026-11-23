//! Canonical retention and historical re-admission of operation-lease bodies.
//!
//! An operation handle names its child lease by immutable reference and digest.
//! This owner retains those exact bytes before start custody can rely on them,
//! then resolves and re-admits the historical parent session before accepting a
//! recovered operation lease. A retained but unindexed lease body is an inert
//! orphan: it grants no handle, dispatch, continuation, or current authority.

use pale_core::canon;
use pale_core::records::{
    InvalidResidentRecordV1, OperationLeaseV1, OperationLeaseWireV1, ResidentRecordRefV1,
    OPERATION_LEASE_V1_SCHEMA_ID,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordCoordinate, ControlStoreError,
    ControlStoreErrorKind, OperationCustodyCommitOutcome, OperationCustodyPort,
    SessionLeaseCustodyPort,
};

use super::custody_phase::{
    commit_with_exact_readback, DatabaseCommitDisposition, DatabaseReadBackDisposition,
};
use super::lifecycle::{GenerationLifecycleError, GenerationLifecycleRecoveryResolver};
use super::operation_custody::OperationCustodyDatabaseFailureReason;
use super::record_projection;
use super::session_lease_custody::recover_session_lease_body;

/// Retain one already-admitted operation lease before durable start uses it.
///
/// The immutable lease record is retained separately from the later atomic
/// start candidate. A later start failure may therefore leave an unindexed
/// orphan, but no operation handle or index can be recovered from that row.
/// Commit ambiguity receives one bounded exact read-back and is never retried.
///
/// # Errors
///
/// Returns [`OperationCustodyDatabaseFailureReason`] when canonical projection,
/// physical retention, or the sole required read-back cannot prove exact
/// custody.
pub fn retain_operation_lease_body(
    port: &mut dyn OperationCustodyPort,
    store: &AuthorityStoreCoordinate,
    lease_ref: &ResidentRecordRefV1,
    lease: &OperationLeaseV1,
    maximum_body_bytes: usize,
) -> Result<(), OperationCustodyDatabaseFailureReason> {
    let record = record_projection::record(
        record_projection::coordinate(lease_ref, lease.operation_lease_digest().as_str())
            .map_err(retention_projection_failure)?,
        lease,
    )
    .map_err(retention_projection_failure)?;
    if record.canonical_record_jcs().as_slice().len() > maximum_body_bytes {
        return Err(retention_failure(ControlStoreError::new(
            ControlStoreErrorKind::RecoveryRequired,
        )));
    }
    let candidate = OperationLeaseRetentionCandidate {
        store: store.clone(),
        record,
        maximum_body_bytes,
    };
    commit_with_exact_readback(
        port,
        &candidate,
        |port, candidate| {
            port.compare_and_retain_operation_lease(&candidate.store, &candidate.record)
                .map(|outcome| match outcome {
                    OperationCustodyCommitOutcome::Committed => {
                        DatabaseCommitDisposition::Committed
                    }
                    OperationCustodyCommitOutcome::ExactReplay => {
                        DatabaseCommitDisposition::ExactReplay
                    }
                })
        },
        |port, candidate| {
            port.read_operation_record(
                &candidate.store,
                candidate.record.coordinate(),
                candidate.maximum_body_bytes,
            )
            .map(|observed| match observed {
                None => DatabaseReadBackDisposition::Predecessor,
                Some(observed) if observed == candidate.record => {
                    DatabaseReadBackDisposition::ExactSuccessor
                }
                Some(_) => DatabaseReadBackDisposition::ConflictingOrPartial,
            })
        },
    )
    .map(drop)
    .map_err(Into::into)
}

/// Recover one exact lease body and re-admit it through its historical session.
///
/// The parent session is recovered from the same store incarnation through the
/// existing generation/profile/head/fence resolver. This path never calls
/// ambient session admission, refreshes an expiry, or grants observation or
/// continuation permission.
///
/// # Errors
///
/// Returns [`OperationLeaseBodyRecoveryError`] for missing, substituted,
/// oversized, noncanonical, schema-incompatible, or contextually inadmissible
/// historical custody.
pub fn recover_operation_lease_body(
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    lease_coordinate: &ControlRecordCoordinate,
    maximum_operation_lease_body_bytes: usize,
    maximum_session_lease_body_bytes: usize,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<OperationLeaseV1, OperationLeaseBodyRecoveryError> {
    recover_operation_lease_and_parent(
        operation_port,
        session_port,
        store,
        lease_coordinate,
        maximum_operation_lease_body_bytes,
        maximum_session_lease_body_bytes,
        resolver,
    )
    .map(|(lease, _)| lease)
}

/// Reuse the exact re-admitted parent for locator recovery without another read.
pub(super) fn recover_operation_lease_and_parent(
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    lease_coordinate: &ControlRecordCoordinate,
    maximum_operation_lease_body_bytes: usize,
    maximum_session_lease_body_bytes: usize,
    resolver: &dyn GenerationLifecycleRecoveryResolver,
) -> Result<
    (
        OperationLeaseV1,
        pale_core::records::SessionAdmissionLeaseV1,
    ),
    OperationLeaseBodyRecoveryError,
> {
    let record = operation_port
        .read_operation_record(store, lease_coordinate, maximum_operation_lease_body_bytes)?
        .ok_or(OperationLeaseBodyRecoveryError::Missing)?;
    if record.coordinate() != lease_coordinate {
        return Err(OperationLeaseBodyRecoveryError::ContextSubstitution(
            "retained operation lease coordinate",
        ));
    }
    let bytes = record.canonical_record_jcs().as_slice();
    if bytes.len() > maximum_operation_lease_body_bytes {
        return Err(OperationLeaseBodyRecoveryError::BoundExceeded);
    }
    let expected = pale_contracts::record_schema_identity(OPERATION_LEASE_V1_SCHEMA_ID)?;
    if record.schema_id() != &expected.schema || record.schema_sha256() != &expected.schema_sha256 {
        return Err(OperationLeaseBodyRecoveryError::ContextSubstitution(
            "retained operation lease schema",
        ));
    }
    let value = canon::parse_strict(bytes)?;
    let wire: OperationLeaseWireV1 = serde_json::from_value(value)?;
    let canonical = canon::to_canonical_string(&wire)?;
    if canonical.as_bytes() != bytes {
        return Err(OperationLeaseBodyRecoveryError::NonCanonical);
    }
    let parent = recover_session_lease_body(
        session_port,
        store,
        wire.session_admission_lease_digest(),
        maximum_session_lease_body_bytes,
        resolver,
    )?;
    let admitted = OperationLeaseV1::admit_wire(wire, &parent, &expected.schema_sha256)?;
    if admitted.operation_lease_digest().as_str() != lease_coordinate.record_sha256().as_str()
        || admitted.schema() != record.schema_id()
        || admitted.schema_sha256() != record.schema_sha256()
    {
        return Err(OperationLeaseBodyRecoveryError::ContextSubstitution(
            "retained operation lease body",
        ));
    }
    Ok((admitted, parent))
}

/// One private exact-retention candidate used only for ambiguity closure.
struct OperationLeaseRetentionCandidate {
    store: AuthorityStoreCoordinate,
    record: ControlRecord,
    maximum_body_bytes: usize,
}

fn retention_projection_failure(
    source: impl std::error::Error + Send + Sync + 'static,
) -> OperationCustodyDatabaseFailureReason {
    retention_failure(ControlStoreError::with_source(
        ControlStoreErrorKind::RecoveryRequired,
        source,
    ))
}

fn retention_failure(source: ControlStoreError) -> OperationCustodyDatabaseFailureReason {
    OperationCustodyDatabaseFailureReason::Store(source)
}

/// Historical operation-lease body re-admission failure.
#[derive(Debug, thiserror::Error)]
pub enum OperationLeaseBodyRecoveryError {
    /// Physical custody could not return the requested exact record.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// No retained lease body exists at the exact requested coordinate.
    #[error("retained operation lease body is missing")]
    Missing,
    /// The returned physical coordinate, schema, or admitted body was substituted.
    #[error("operation lease recovery context was substituted: {0}")]
    ContextSubstitution(&'static str),
    /// The retained body exceeds the caller's explicit recovery ceiling.
    #[error("retained operation lease body exceeds the recovery byte ceiling")]
    BoundExceeded,
    /// The embedded initial-v1 operation-lease schema cannot be resolved.
    #[error("operation lease schema registry is invalid: {0}")]
    Contract(#[from] pale_contracts::SchemaError),
    /// Retained bytes violate strict canonical JSON grammar.
    #[error("operation lease body violates canonical JSON grammar: {0}")]
    Canonical(#[from] pale_core::canon::CanonError),
    /// Retained bytes do not decode as the exact operation-lease wire.
    #[error("operation lease body does not decode as its typed wire: {0}")]
    Decode(#[from] serde_json::Error),
    /// The retained lease has another valid JSON representation.
    #[error("operation lease body is not canonical JCS")]
    NonCanonical,
    /// The exact historical parent session could not be recovered and re-admitted.
    #[error("operation lease parent session recovery failed: {0}")]
    ParentSession(#[from] GenerationLifecycleError),
    /// The recovered child lease does not narrow its exact historical parent.
    #[error("operation lease body does not admit against its historical parent: {0}")]
    Admission(#[from] InvalidResidentRecordV1),
}

#[cfg(test)]
#[path = "../../tests/internal/resident/operation_lease_custody.rs"]
mod tests;
