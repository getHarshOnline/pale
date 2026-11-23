//! Private store-wide logical transaction bookkeeping.
//!
//! This is neither an authority journal nor an anti-rollback witness. The
//! mutation owner must retain its existing transaction and commit classifier;
//! these helpers never commit, sign, retry, or acquire an independent lock.

use pale_core::records::ControlTransactionSequenceV1;
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordCoordinate, ControlStoreError,
    ControlStoreErrorKind, ControlTransactionPreparation,
};
use rusqlite::{
    params, types::ValueRef, Connection, OptionalExtension, Transaction, TransactionBehavior,
};

use crate::error::{sqlite, sqlite_integer};
use crate::ControlStoreKind;

/// Release a bounded read snapshot before returning an optimistic proposal.
///
/// This checks only the existing physical store class and exact incarnation;
/// principal/deployment authority admission remains with each semantic owner.
/// No write transaction survives into checkpoint construction or signing.
pub(crate) fn prepare_snapshot(
    connection: &mut Connection,
    store: &AuthorityStoreCoordinate,
    kind: ControlStoreKind,
) -> Result<ControlTransactionPreparation, ControlStoreError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|source| sqlite("begin control transaction preparation", source))?;
    let matches = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pale_control_store_header \
         WHERE singleton = 1 AND store_kind = ?1 AND logical_store_id = ?2 \
         AND control_incarnation_id = ?3)",
            params![
                kind.as_str(),
                store.store_id().as_str(),
                store.incarnation_id().as_str()
            ],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|source| sqlite("compare control transaction store class", source))?;
    if !matches {
        return Err(recovery());
    }
    let prepared = prepare(&transaction, store)?;
    transaction
        .rollback()
        .map_err(|source| sqlite("release control transaction preparation", source))?;
    Ok(prepared)
}

/// Read the exact positive head without demanding an available successor.
///
/// The SQL result is one integer, so malformed text/BLOB values are never
/// allocated into owned buffers. A missing or invalid head is recovery, not
/// permission to initialize a counter. Semantic store-kind/scope checks remain
/// with the existing caller in addition to this exact store/incarnation join.
pub(crate) fn read(
    connection: &Connection,
    store: &AuthorityStoreCoordinate,
) -> Result<ControlTransactionSequenceV1, ControlStoreError> {
    let raw = connection
        .query_row(
            "SELECT logical_transaction_sequence FROM pale_control_store_header \
             WHERE singleton = 1 AND logical_store_id = ?1 AND control_incarnation_id = ?2",
            params![store.store_id().as_str(), store.incarnation_id().as_str()],
            |row| {
                Ok(match row.get_ref(0)? {
                    ValueRef::Integer(value) => Some(value),
                    _ => None,
                })
            },
        )
        .optional()
        .map_err(|source| sqlite("read control transaction head", source))?
        .flatten()
        .ok_or_else(recovery)?;
    let raw = u64::try_from(raw).map_err(|_| recovery())?;
    ControlTransactionSequenceV1::new(raw).map_err(|_| recovery())
}

/// Prepare a new mutation only after the caller has excluded exact replay.
pub(crate) fn prepare(
    connection: &Connection,
    store: &AuthorityStoreCoordinate,
) -> Result<ControlTransactionPreparation, ControlStoreError> {
    ControlTransactionPreparation::new(store.clone(), read(connection, store)?).map_err(|source| {
        ControlStoreError::with_source(ControlStoreErrorKind::TransactionExhausted, source)
    })
}

/// Compare and advance the retained fence inside the caller's transaction.
///
/// Later failure rolls this change back with the records, indexes and nodes.
/// No independent commit or external work occurs here. Exact replay callers
/// never invoke this helper, including when the current head is saturated.
pub(crate) fn advance(
    transaction: &Transaction<'_>,
    prepared: &ControlTransactionPreparation,
) -> Result<(), ControlStoreError> {
    if read(transaction, prepared.store())? != prepared.expected() {
        return Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ));
    }
    let changed = transaction
        .execute(
            "UPDATE pale_control_store_header SET logical_transaction_sequence = ?1 \
             WHERE singleton = 1 AND logical_store_id = ?2 AND control_incarnation_id = ?3 \
             AND logical_transaction_sequence = ?4",
            params![
                sqlite_integer(prepared.successor().get())?,
                prepared.store().store_id().as_str(),
                prepared.store().incarnation_id().as_str(),
                sqlite_integer(prepared.expected().get())?,
            ],
        )
        .map_err(|source| sqlite("advance control transaction head", source))?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))
    }
}

/// Read a retained immutable record's actual insertion stamp, not a new number.
///
/// Table selection is the existing closed physical store class. Missing records
/// return `None`; present malformed metadata fails recovery. Byte and semantic
/// record admission remain the callers' existing owners.
pub(crate) fn record_stamp(
    connection: &Connection,
    kind: ControlStoreKind,
    coordinate: &ControlRecordCoordinate,
) -> Result<Option<ControlTransactionSequenceV1>, ControlStoreError> {
    let sql = match kind {
        ControlStoreKind::PrincipalTrust => {
            "SELECT inserted_transaction_sequence FROM pale_principal_trust_record \
             WHERE record_ref = ?1 AND record_sha256 = ?2"
        }
        ControlStoreKind::DeploymentControl => {
            "SELECT inserted_transaction_sequence FROM pale_deployment_control_record \
             WHERE record_ref = ?1 AND record_sha256 = ?2"
        }
    };
    let raw = connection
        .query_row(
            sql,
            params![
                coordinate.record_ref().as_str(),
                coordinate.record_sha256().as_str()
            ],
            |row| {
                Ok(match row.get_ref(0)? {
                    ValueRef::Integer(value) => Some(value),
                    _ => None,
                })
            },
        )
        .optional()
        .map_err(|source| sqlite("read control record insertion stamp", source))?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let raw = u64::try_from(raw.ok_or_else(recovery)?).map_err(|_| recovery())?;
    ControlTransactionSequenceV1::new(raw)
        .map(Some)
        .map_err(|_| recovery())
}

/// Compare an actual append with its atom without computing a successor.
///
/// Previously retained dependencies must not be passed as new atom members.
/// The current head may be later, including at the maximum; this comparison
/// neither rewrites the insertion stamp nor upgrades historical classification.
pub(crate) fn records_were_inserted_in<'a>(
    connection: &Connection,
    kind: ControlStoreKind,
    store: &AuthorityStoreCoordinate,
    expected: ControlTransactionSequenceV1,
    records: impl IntoIterator<Item = &'a ControlRecord>,
) -> Result<bool, ControlStoreError> {
    let current = read(connection, store)?;
    if expected.get() > current.get() {
        // An uncommitted candidate normally proposes the next transaction.
        // Its successor is absent, not corrupted; each semantic classifier
        // distinguishes this predecessor from partial retained successor data.
        return Ok(false);
    }
    for record in records {
        if record_stamp(connection, kind, record.coordinate())? != Some(expected) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn recovery() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}
