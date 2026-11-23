//! Local materialization evidence-settlement law tests.

use std::fs;
use std::{collections::BTreeSet, time::Duration};

use super::*;

#[test]
fn uncertain_receipt_publication_retains_exact_expected_file_bytes_for_read_back() {
    let temporary = tempfile::tempdir().expect("temporary evidence root is created");
    let root = pale_adapter_fs::ConfinedRoot::open(temporary.path())
        .expect("temporary evidence root is confined");
    let relative = "engine.apply.digest.receipt.json";
    let location = root.location(relative).expect("receipt target is confined");
    let expected = b"{\n  \"outcome\": \"succeeded\"\n}\n";
    location
        .write_atomic(expected, RECEIPT_MODE)
        .expect("the visible receipt fixture is published");
    let source = pale_adapter_fs::FsError::Io {
        operation: "synchronize receipt",
        path: temporary.path().join(relative),
        source: std::io::Error::other("injected durability failure"),
    };
    let publication = pale_adapter_fs::FsError::EffectUncertain {
        operation: "publish receipt",
        path: temporary.path().join(relative),
        source: Box::new(source),
    };

    let failure = match reconcile_failed_publication(&location, expected, publication) {
        Err(failure) => failure,
        Ok(()) => unreachable!("non-collision uncertainty remains reconcile-first"),
    };
    assert_eq!(failure.kind, ReceiptPublicationFailureKind::Uncertain);
    assert_eq!(failure.proposed.as_deref(), Some(expected.as_slice()));
    assert_eq!(
        fs::read(temporary.path().join(relative)).expect("visible receipt remains readable"),
        expected
    );
}

#[test]
fn repeated_api_format_receipt_publication_is_exact_byte_idempotent() {
    let temporary = tempfile::tempdir().expect("temporary evidence root is created");
    let root = pale_adapter_fs::ConfinedRoot::open(temporary.path())
        .expect("temporary evidence root is confined");
    let (lock, lock_effect) = pale_adapter_fs::acquire_advisory_lock(
        &root,
        "test-receipts.lock",
        Duration::from_millis(10),
        "test receipts",
    )
    .expect("evidence test lock is held");
    assert_eq!(lock_effect, pale_adapter_fs::AdvisoryLockEffect::Confirmed);
    let evidence = MutationEvidenceRun {
        root,
        _lock: lock,
        kind: PlanKind::Apply,
        receipt_paths: BTreeSet::new(),
    };
    let relative = "engine.apply.digest.receipt.json";
    let expected = pale_core::canon::pretty_json_document(
        br#"{"outcome":"succeeded","repository_id":"engine"}"#,
    )
    .expect("API receipt projection is valid strict JSON");

    evidence
        .publish_immutable(relative, &expected)
        .expect("first API-format receipt publication succeeds");
    evidence
        .publish_immutable(relative, &expected)
        .expect("the exact API-format receipt is idempotent");

    assert!(expected.windows(3).any(|window| window == b"\n  "));
    assert_eq!(
        fs::read(temporary.path().join(relative)).expect("receipt remains readable"),
        expected
    );
}
