//! Behavioral boundaries for expiring session and evidence records.

#![cfg(unix)]

use pale_adapter_fs::{AdvisoryLockEffect, DurableEvidenceStore, FsError, SessionStore};
use pale_core::records::{
    CompilationInputDigest, ExpiringRecordContract, RecordSchemaIdentity, SchemaDigest, SchemaId,
    SchemaManifestDigest, SchemaRegistryIdentity, SchemaSetDigest,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Stable non-expired lifetime used by ordinary session-store fixtures.
const TTL: Duration = Duration::from_secs(3_600);

fn contract() -> ExpiringRecordContract {
    ExpiringRecordContract {
        envelope: RecordSchemaIdentity {
            schema: SchemaId::parse("test.expiring-record-envelope.v1").unwrap(),
            schema_sha256: SchemaDigest::parse("a".repeat(64)).unwrap(),
        },
        payload: RecordSchemaIdentity {
            schema: SchemaId::parse("test.payload.v1").unwrap(),
            schema_sha256: SchemaDigest::parse("b".repeat(64)).unwrap(),
        },
        schema_registry: SchemaRegistryIdentity {
            schema_manifest_sha256: SchemaManifestDigest::parse("c".repeat(64)).unwrap(),
            schema_set_sha256: SchemaSetDigest::parse("d".repeat(64)).unwrap(),
        },
        compilation_input_digest: CompilationInputDigest::parse("e".repeat(64)).unwrap(),
    }
}

fn session_store(root: &Path, relative: &str, ttl: Duration) -> Result<SessionStore, FsError> {
    SessionStore::new(root, relative, ttl, contract())
}

fn evidence_store(
    root: &Path,
    relative: &str,
    ttl: Duration,
) -> Result<DurableEvidenceStore, FsError> {
    DurableEvidenceStore::new(root, relative, ttl, contract())
}

fn session_store_with_contract(
    root: &Path,
    relative: &str,
    ttl: Duration,
    contract: ExpiringRecordContract,
) -> Result<SessionStore, FsError> {
    SessionStore::new(root, relative, ttl, contract)
}

#[derive(Default, Serialize, Deserialize)]
struct Counter {
    items: Vec<u32>,
}

fn append(store: &SessionStore, item: u32) {
    let published = store
        .update_if(|counter: &mut Counter| {
            counter.items.push(item);
            Some(())
        })
        .unwrap();
    assert_eq!(published, Some(()));
}

#[test]
fn load_absent_is_default() {
    let tmp = tempfile::tempdir().unwrap();
    let store = session_store(tmp.path(), "sessions/none.json", TTL).unwrap();
    let (publication_generation, c): (u64, Counter) = store.load().unwrap();
    assert_eq!(
        publication_generation, 0,
        "absent state starts at publication generation 0"
    );
    assert!(c.items.is_empty());
}

#[test]
fn oversized_state_is_rejected_before_decoding() {
    let tmp = tempfile::tempdir().unwrap();
    let relative = "sessions/oversized.json";
    let path = tmp.path().join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, vec![b'x'; (1 << 20) + 1]).unwrap();

    let store = session_store(tmp.path(), relative, TTL).unwrap();
    assert!(store.load::<Counter>().is_err());
}

#[test]
fn concurrent_writers_never_lose_updates() {
    let tmp = tempfile::tempdir().unwrap();
    let relative = "sessions/s1.json";
    let n: u32 = 24;
    // Each thread opens its OWN handle at the same path and appends one item under the lock.
    std::thread::scope(|scope| {
        for i in 0..n {
            let root = tmp.path().to_path_buf();
            scope.spawn(move || append(&session_store(&root, relative, TTL).unwrap(), i));
        }
    });
    let (publication_generation, c): (u64, Counter) = session_store(tmp.path(), relative, TTL)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(
        c.items.len(),
        n as usize,
        "a lost update: concurrent writers dropped writes (lock/CAS broken)"
    );
    assert_eq!(
        publication_generation, n as u64,
        "publication generation must equal the write count"
    );
    let mut sorted = c.items.clone();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        (0..n).collect::<Vec<_>>(),
        "every write present exactly once"
    );
}

#[test]
fn update_then_load_roundtrips() {
    let tmp = tempfile::tempdir().unwrap();
    let store = session_store(tmp.path(), "sessions/s2.json", TTL).unwrap();
    append(&store, 7);
    append(&store, 9);
    let (publication_generation, c): (u64, Counter) = store.load().unwrap();
    assert_eq!(publication_generation, 2);
    assert_eq!(c.items, vec![7, 9]);
}

#[test]
fn non_v1_unbound_envelope_is_rejected_without_republication() {
    let tmp = tempfile::tempdir().unwrap();
    let relative = "sessions/unbound.json";
    let store = session_store(tmp.path(), relative, TTL).unwrap();
    append(&store, 7);

    let path = tmp.path().join(relative);
    let mut persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    persisted["schema_version"] = serde_json::json!(2);
    persisted.as_object_mut().unwrap().remove("record_class");
    persisted.as_object_mut().unwrap().remove("record_binding");
    std::fs::write(
        &path,
        pale_core::canon::to_canonical_string(&persisted).unwrap(),
    )
    .unwrap();

    let invalid = pale_core::canon::to_canonical_string(&persisted).unwrap();
    let error = store
        .update_if(|state: &mut Counter| {
            state.items.push(9);
            Some(())
        })
        .unwrap_err();
    assert!(error.to_string().contains("decode session state"));
    assert_eq!(std::fs::read(path).unwrap(), invalid.as_bytes());
}

#[test]
fn conditional_update_rejects_stale_state_without_writing() {
    let tmp = tempfile::tempdir().unwrap();
    let store = session_store(tmp.path(), "sessions/cas.json", TTL).unwrap();
    append(&store, 7);

    let changed = store
        .update_if(|c: &mut Counter| (c.items == vec![9]).then(|| c.items.push(11)))
        .unwrap();
    assert!(changed.is_none());

    let (publication_generation, state): (u64, Counter) = store.load().unwrap();
    assert_eq!(
        publication_generation, 1,
        "a rejected comparison is not a publication"
    );
    assert_eq!(state.items, vec![7]);
}

#[cfg(unix)]
#[test]
fn symlinked_session_parent_is_rejected() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), root.path().join("sessions")).unwrap();

    assert!(session_store(root.path(), "sessions/escape.json", TTL).is_err());
    assert!(!outside.path().join("escape.json").exists());
}

#[test]
fn expired_state_reads_as_absent_and_is_pruned() {
    let tmp = tempfile::tempdir().unwrap();
    let relative = "profile/repo/route/expired.json";
    let store = session_store(tmp.path(), relative, Duration::from_secs(1)).unwrap();
    append(&store, 7);

    let path = tmp.path().join(relative);
    let mut persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    persisted["updated_unix_seconds"] = serde_json::json!(0);
    std::fs::write(
        &path,
        pale_core::canon::to_canonical_string(&persisted).unwrap(),
    )
    .unwrap();

    let (publication_generation, counter): (u64, Counter) = store.load().unwrap();
    assert_eq!(publication_generation, 0);
    assert!(
        counter.items.is_empty(),
        "expired state must not affect a new session"
    );
    assert_eq!(store.prune_expired_siblings(16).unwrap(), 1);
    assert!(!path.exists(), "expired state is physically removed");
}

#[test]
fn fractional_ttl_does_not_expire_a_fresh_record_immediately() {
    let tmp = tempfile::tempdir().unwrap();
    let store = session_store(
        tmp.path(),
        "sessions/fractional.json",
        Duration::from_millis(500),
    )
    .unwrap();
    append(&store, 7);

    let (publication_generation, counter): (u64, Counter) = store.load().unwrap();
    assert_eq!(publication_generation, 1);
    assert_eq!(counter.items, [7]);
}

#[test]
fn future_dated_session_state_remains_unknown_and_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let relative = "sessions/future.json";
    let store = session_store(tmp.path(), relative, TTL).unwrap();
    append(&store, 7);

    let path = tmp.path().join(relative);
    let mut persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    persisted["updated_unix_seconds"] = serde_json::json!(4_102_444_800_u64);
    persisted["updated_subsec_nanos"] = serde_json::json!(999_999_999_u32);
    std::fs::write(
        &path,
        pale_core::canon::to_canonical_string(&persisted).unwrap(),
    )
    .unwrap();

    let sealed = std::fs::read(&path).unwrap();
    assert!(matches!(
        store.load::<Counter>(),
        Err(FsError::Invariant { .. })
    ));
    assert!(matches!(
        store.update_if(|counter: &mut Counter| {
            counter.items.push(9);
            Some(())
        }),
        Err(FsError::Invariant { .. })
    ));
    assert!(matches!(
        store.prune_expired_siblings(1),
        Err(FsError::BatchFailed { .. })
    ));
    assert_eq!(
        std::fs::read(path).unwrap(),
        sealed,
        "clock uncertainty must neither reset, overwrite, nor prune session state"
    );
}

#[derive(Default, Serialize, Deserialize)]
struct BoundEvidence {
    identity: String,
    count: u64,
}

#[test]
fn durable_evidence_initializes_identity_once_and_retains_it() {
    let tmp = tempfile::tempdir().unwrap();
    let store = evidence_store(tmp.path(), "evidence/route.json", TTL).unwrap();
    let first = store
        .update_or_initialize(
            || BoundEvidence {
                identity: "route-a".into(),
                count: 0,
            },
            |evidence| {
                evidence.count += 1;
                (evidence.identity.clone(), evidence.count)
            },
        )
        .unwrap();
    let second = store
        .update_or_initialize(
            || panic!("an existing evidence identity must not be reinitialized"),
            |evidence: &mut BoundEvidence| {
                evidence.count += 1;
                (evidence.identity.clone(), evidence.count)
            },
        )
        .unwrap();

    assert_eq!(first, ("route-a".into(), 1));
    assert_eq!(second, ("route-a".into(), 2));
}

#[test]
fn expired_durable_evidence_reinitializes_without_resetting_generation() {
    let tmp = tempfile::tempdir().unwrap();
    let relative = "evidence/expired.json";
    let path = tmp.path().join(relative);
    let store = evidence_store(tmp.path(), relative, Duration::from_secs(1)).unwrap();
    store
        .update_or_initialize(
            || BoundEvidence {
                identity: "route-a".into(),
                count: 0,
            },
            |evidence| evidence.count += 1,
        )
        .unwrap();

    let mut persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    persisted["updated_unix_seconds"] = serde_json::json!(0);
    persisted["updated_subsec_nanos"] = serde_json::json!(0);
    std::fs::write(
        &path,
        pale_core::canon::to_canonical_string(&persisted).unwrap(),
    )
    .unwrap();

    store
        .update_or_initialize(
            || BoundEvidence {
                identity: "route-b".into(),
                count: 0,
            },
            |evidence| evidence.count += 1,
        )
        .unwrap();
    let republished: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(republished["publication_generation"], 2);
    assert_eq!(republished["state"]["identity"], "route-b");
}

#[test]
fn record_class_and_location_are_part_of_the_persisted_binding() {
    let tmp = tempfile::tempdir().unwrap();
    let source = session_store(tmp.path(), "sessions/source.json", TTL).unwrap();
    append(&source, 7);

    std::fs::copy(
        tmp.path().join("sessions/source.json"),
        tmp.path().join("sessions/copied.json"),
    )
    .unwrap();
    let copied = session_store(tmp.path(), "sessions/copied.json", TTL).unwrap();
    assert!(copied.load::<Counter>().is_err());

    let wrong_class = evidence_store(tmp.path(), "sessions/source.json", TTL).unwrap();
    assert!(wrong_class.load_or_default::<Counter>().is_err());

    let source_path = tmp.path().join("sessions/source.json");
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&source_path).unwrap()).unwrap();
    std::fs::write(&source_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    assert!(source.load::<Counter>().is_err());
}

#[test]
fn schema_registry_payload_and_compilation_identity_are_part_of_admission() {
    let tmp = tempfile::tempdir().unwrap();
    let relative = "sessions/contract.json";
    let original = session_store(tmp.path(), relative, TTL).unwrap();
    append(&original, 7);
    let path = tmp.path().join(relative);
    let before = std::fs::read(&path).unwrap();

    let mut variants = Vec::new();
    let mut changed = contract();
    changed.envelope.schema_sha256 = SchemaDigest::parse("1".repeat(64)).unwrap();
    variants.push(changed);
    let mut changed = contract();
    changed.payload.schema_sha256 = SchemaDigest::parse("2".repeat(64)).unwrap();
    variants.push(changed);
    let mut changed = contract();
    changed.schema_registry.schema_manifest_sha256 =
        SchemaManifestDigest::parse("3".repeat(64)).unwrap();
    variants.push(changed);
    let mut changed = contract();
    changed.schema_registry.schema_set_sha256 = SchemaSetDigest::parse("4".repeat(64)).unwrap();
    variants.push(changed);
    let mut changed = contract();
    changed.compilation_input_digest = CompilationInputDigest::parse("5".repeat(64)).unwrap();
    variants.push(changed);

    for changed in variants {
        let store = session_store_with_contract(tmp.path(), relative, TTL, changed).unwrap();
        assert!(store.load::<Counter>().is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn future_dated_durable_evidence_is_not_reinitialized_or_pruned() {
    let tmp = tempfile::tempdir().unwrap();
    let relative = "evidence/future.json";
    let path = tmp.path().join(relative);
    let store = evidence_store(tmp.path(), relative, TTL).unwrap();
    store
        .update_or_initialize(
            || BoundEvidence {
                identity: "route-a".into(),
                count: 0,
            },
            |evidence| evidence.count += 1,
        )
        .unwrap();

    let mut persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    persisted["updated_unix_seconds"] = serde_json::json!(4_102_444_800_u64);
    std::fs::write(
        &path,
        pale_core::canon::to_canonical_string(&persisted).unwrap(),
    )
    .unwrap();

    assert!(matches!(
        store.load_or_default::<BoundEvidence>(),
        Err(FsError::Invariant { .. })
    ));
    assert!(store.prune_expired_siblings(1).is_err());
    assert!(path.exists());
}

#[test]
fn oversized_transition_has_a_publication_error_and_no_record() {
    #[derive(Default, Serialize, Deserialize)]
    struct LargeRecord {
        body: String,
    }

    let tmp = tempfile::tempdir().unwrap();
    let relative = "sessions/large.json";
    let store = session_store(tmp.path(), relative, TTL).unwrap();
    let result = store.update_if(|record: &mut LargeRecord| {
        record.body = "x".repeat((1 << 20) + 1);
        Some(())
    });

    assert!(matches!(
        result,
        Err(FsError::Coordination {
            effect: AdvisoryLockEffect::Confirmed,
            source,
        }) if matches!(*source, FsError::RecordSizeLimit { .. })
    ));
    assert!(!tmp.path().join(relative).exists());
}

#[test]
fn fresh_lexical_prefix_does_not_starve_later_expired_records() {
    let tmp = tempfile::tempdir().unwrap();
    let fresh = session_store(tmp.path(), "sessions/a-fresh.json", TTL).unwrap();
    let expired = session_store(tmp.path(), "sessions/z-expired.json", TTL).unwrap();
    append(&fresh, 1);
    append(&expired, 2);

    let expired_path = tmp.path().join("sessions/z-expired.json");
    let mut persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&expired_path).unwrap()).unwrap();
    persisted["updated_unix_seconds"] = serde_json::json!(0);
    persisted["updated_subsec_nanos"] = serde_json::json!(0);
    std::fs::write(
        &expired_path,
        pale_core::canon::to_canonical_string(&persisted).unwrap(),
    )
    .unwrap();

    assert_eq!(fresh.prune_expired_siblings(1).unwrap(), 1);
    assert!(tmp.path().join("sessions/a-fresh.json").exists());
    assert!(!expired_path.exists());
}

#[test]
fn foreign_session_directory_entries_block_pruning_before_deletion() {
    let tmp = tempfile::tempdir().unwrap();
    let store = session_store(tmp.path(), "sessions/expired.json", TTL).unwrap();
    append(&store, 1);
    let state_path = tmp.path().join("sessions/expired.json");
    let mut persisted: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    persisted["updated_unix_seconds"] = serde_json::json!(0);
    persisted["updated_subsec_nanos"] = serde_json::json!(0);
    std::fs::write(
        &state_path,
        pale_core::canon::to_canonical_string(&persisted).unwrap(),
    )
    .unwrap();
    std::fs::write(tmp.path().join("sessions/foreign.tmp"), b"foreign").unwrap();

    assert!(matches!(
        store.prune_expired_siblings(1),
        Err(FsError::UnexpectedExpiringRecordEntry { .. })
    ));
    assert!(state_path.exists(), "grammar validation precedes pruning");
}
