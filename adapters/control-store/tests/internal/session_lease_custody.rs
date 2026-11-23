//! Physical session-lease body retention tests.

use super::*;
use crate::test_support as support;
use crate::ControlStoreKind;
use pale_core::records::{SchemaDigest, SchemaId, SessionAdmissionLeaseDigest};
use pale_ports::{AuthorityStoreCoordinate, ControlRecordBytes};

fn body(
    coordinate: AuthorityStoreCoordinate,
    digest: SessionAdmissionLeaseDigest,
    bytes: &[u8],
) -> SessionLeaseBody {
    SessionLeaseBody::new(
        coordinate,
        digest,
        SchemaId::parse("pale/session-admission-lease.v1").expect("schema id"),
        SchemaDigest::parse("a".repeat(64)).expect("schema digest"),
        ControlRecordBytes::new(bytes.to_vec()).expect("nonempty body"),
    )
}

#[test]
fn sqlite_session_body_retention_is_immutable_bounded_and_store_bound() {
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        ControlStoreKind::DeploymentControl,
        "session-lease-body",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .expect("open deployment candidate");
    let digest = SessionAdmissionLeaseDigest::parse("b".repeat(64)).expect("lease digest");
    let retained = body(coordinate.clone(), digest.clone(), br#"{"lease":"one"}"#);

    assert_eq!(
        store.compare_and_retain(&retained).unwrap(),
        SessionLeaseBodyRetention::Retained
    );
    assert_eq!(
        store.compare_and_retain(&retained).unwrap(),
        SessionLeaseBodyRetention::ExactReplay
    );
    let read = store
        .read(&SessionLeaseBodyRead::new(
            coordinate.clone(),
            digest.clone(),
            retained.canonical_wire_jcs().as_slice().len(),
        ))
        .unwrap();
    assert_eq!(read.as_ref(), Some(&retained));

    let conflicting = body(coordinate.clone(), digest.clone(), br#"{"lease":"two"}"#);
    assert_eq!(
        store.compare_and_retain(&conflicting).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(
        store
            .read(&SessionLeaseBodyRead::new(
                coordinate.clone(),
                digest.clone(),
                retained.canonical_wire_jcs().as_slice().len() - 1,
            ))
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let non_ascii_digest =
        SessionAdmissionLeaseDigest::parse("c".repeat(64)).expect("non-ASCII row digest");
    store
        .record_custody_connection()
        .expect("session body connection")
        .execute(
            "INSERT INTO pale_deployment_session_lease_body \
             (session_admission_lease_digest, schema_id, schema_sha256, canonical_wire_jcs) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                non_ascii_digest.as_str(),
                "é".repeat(MAX_SESSION_LEASE_SCHEMA_ID_BYTES),
                "a".repeat(SESSION_LEASE_SCHEMA_DIGEST_BYTES),
                br#"{"lease":"non-ascii-schema"}"#,
            ],
        )
        .expect("physical character limit admits the oversized byte sequence");
    assert_eq!(
        store
            .read(&SessionLeaseBodyRead::new(
                coordinate.clone(),
                non_ascii_digest,
                1 << 20,
            ))
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired,
        "metadata byte bounds are checked independently of SQLite character length"
    );

    let other = AuthorityStoreCoordinate::new(
        coordinate.store_id().clone(),
        pale_core::records::ControlIncarnationIdV1::parse("other-incarnation")
            .expect("other incarnation"),
    );
    assert_eq!(
        store
            .read(&SessionLeaseBodyRead::new(other, digest, 1 << 20))
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::Storage
    );
}

#[test]
fn session_retention_advances_once_and_exhaustion_rolls_back_the_new_body() {
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        ControlStoreKind::DeploymentControl,
        "session-clock",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .unwrap();
    let head = || {
        crate::store::transaction::read(&store.record_custody_connection().unwrap(), &coordinate)
            .unwrap()
            .get()
    };
    let retained = body(
        coordinate.clone(),
        SessionAdmissionLeaseDigest::parse("b".repeat(64)).unwrap(),
        br#"{"lease":"one"}"#,
    );
    assert_eq!(head(), 1);
    assert_eq!(
        store.compare_and_retain(&retained).unwrap(),
        SessionLeaseBodyRetention::Retained
    );
    assert_eq!(head(), 2);
    assert_eq!(
        store.compare_and_retain(&retained).unwrap(),
        SessionLeaseBodyRetention::ExactReplay
    );
    assert_eq!(head(), 2);
    assert_eq!(store.record_custody_connection().unwrap().execute("UPDATE pale_control_store_header SET logical_transaction_sequence=9007199254740991 WHERE singleton=1", []).unwrap(), 1);
    assert_eq!(
        store.compare_and_retain(&retained).unwrap(),
        SessionLeaseBodyRetention::ExactReplay
    );
    let new_body = body(
        coordinate.clone(),
        SessionAdmissionLeaseDigest::parse("c".repeat(64)).unwrap(),
        br#"{"lease":"new"}"#,
    );
    assert_eq!(
        store.compare_and_retain(&new_body).unwrap_err().kind(),
        ControlStoreErrorKind::TransactionExhausted
    );
    assert_eq!(
        store
            .record_custody_connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM pale_deployment_session_lease_body",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(head(), 9_007_199_254_740_991);
    assert_eq!(
        store
            .read(&SessionLeaseBodyRead::new(
                coordinate.clone(),
                new_body.digest().clone(),
                1024
            ))
            .unwrap(),
        None
    );
}

#[test]
fn private_admission_basis_is_atomic_immutable_and_shares_the_read_bound() {
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        ControlStoreKind::DeploymentControl,
        "session-private-basis",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .expect("open deployment candidate");
    let digest = SessionAdmissionLeaseDigest::parse("d".repeat(64)).expect("lease digest");
    let retained = body(coordinate.clone(), digest.clone(), br#"{"lease":"one"}"#)
        .with_private_admission_basis(
            ControlRecordBytes::new(br#"{"basis":"one"}"#.to_vec()).expect("basis bytes"),
        );
    let total_bytes = retained.canonical_wire_jcs().as_slice().len()
        + retained
            .canonical_private_admission_basis_jcs()
            .expect("private basis")
            .as_slice()
            .len();

    assert_eq!(
        store.compare_and_retain(&retained).unwrap(),
        SessionLeaseBodyRetention::Retained
    );
    assert_eq!(
        store.compare_and_retain(&retained).unwrap(),
        SessionLeaseBodyRetention::ExactReplay
    );
    assert_eq!(
        store
            .read(&SessionLeaseBodyRead::new(
                coordinate.clone(),
                digest.clone(),
                total_bytes,
            ))
            .unwrap(),
        Some(retained.clone())
    );
    assert_eq!(
        store
            .read(&SessionLeaseBodyRead::new(
                coordinate.clone(),
                digest.clone(),
                total_bytes - 1,
            ))
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let substituted = body(coordinate, digest, br#"{"lease":"one"}"#).with_private_admission_basis(
        ControlRecordBytes::new(br#"{"basis":"two"}"#.to_vec()).expect("basis bytes"),
    );
    assert_eq!(
        store.compare_and_retain(&substituted).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}
