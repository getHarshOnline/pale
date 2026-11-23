//! Physical frontier construction; no consultation policy or signed observation admission.

use super::*;
use crate::{
    BoundedAuthorityEntryBytes, OperationCollectionJournal, OperationCollectionJournalError,
    PreparedAuthorityAppend, SequencedAuthorityEntry,
};
use pale_core::records::JournalSequenceV1;

fn observation() -> OperationCustodyCandidate {
    let successor = record("observation-2", '4', OPERATION_OBSERVATION_V1_SCHEMA);
    OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Exact(record(
                "observation-1",
                '3',
                OPERATION_OBSERVATION_V1_SCHEMA,
            )),
            successor: successor.coordinate().clone(),
        }],
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
    )
    .unwrap()
}

fn cancellation_observation() -> OperationCustodyCandidate {
    let predecessor = record("observation-1", '3', OPERATION_OBSERVATION_V1_SCHEMA);
    let successor = record("observation-2", '4', OPERATION_OBSERVATION_V1_SCHEMA);
    let request = record("control-request", '5', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);
    OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![
            OperationCustodyRecord::OperationObservation(successor.clone()),
            OperationCustodyRecord::CommonRequest(request),
        ],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Exact(predecessor),
            successor: successor.coordinate().clone(),
        }],
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
    )
    .unwrap()
}

fn append(store: AuthorityStoreCoordinate, bytes: &[u8]) -> PreparedAuthorityAppend {
    PreparedAuthorityAppend::new(
        store,
        None,
        SequencedAuthorityEntry::new(
            JournalSequenceV1::new(1).unwrap(),
            BoundedAuthorityEntryBytes::new(bytes.to_vec(), 32).unwrap(),
        ),
    )
    .unwrap()
}

fn pair(candidate: &OperationCustodyCandidate, bytes: &[u8]) -> OperationCollectionJournal {
    OperationCollectionJournal::prepare(
        &coordinate("handle-1", '2'),
        candidate.accumulator().binding().transaction().clone(),
        append(store(), bytes),
        4096,
        32,
    )
    .unwrap()
}

#[test]
fn frontier_is_deterministic_exact_and_bounded_before_attachment() {
    let candidate = observation();
    let exact = pair(&candidate, b"journal one");
    assert_eq!(exact, pair(&candidate, b"journal one"));
    let changed = pair(&candidate, b"journal two");
    assert_ne!(
        exact.frontier().coordinate(),
        changed.frontier().coordinate()
    );
    assert_eq!(
        exact.frontier().schema_sha256(),
        changed.frontier().schema_sha256()
    );
    assert_eq!(
        exact.frontier().coordinate().record_sha256().as_str(),
        pale_core::sha256_hex(exact.frontier().canonical_record_jcs().as_slice())
    );
    assert_eq!(
        exact.frontier().coordinate().record_ref().as_str(),
        format!(
            "pale/private-operation-collection-frontier/v1/{}",
            exact.frontier().coordinate().record_sha256().as_str()
        )
    );
    let alternate_reference = ControlRecord::new(
        ControlRecordCoordinate::new(
            ResidentRecordRefV1::parse("alternate-current-frontier").unwrap(),
            exact.frontier().coordinate().record_sha256().clone(),
        ),
        exact.frontier().schema_id().clone(),
        exact.frontier().schema_sha256().clone(),
        ControlRecordBytes::new(exact.frontier().canonical_record_jcs().as_slice().to_vec())
            .unwrap(),
    );
    assert_eq!(
        OperationCollectionFrontierEvidence::admit(
            &alternate_reference,
            &store(),
            exact.operation_handle(),
            4096,
        ),
        Err(OperationCollectionJournalError::Reference),
        "an internally valid byte-identical frontier cannot select another private head"
    );
    let encoded =
        pale_core::canon::parse_strict(exact.frontier().canonical_record_jcs().as_slice()).unwrap();
    assert_eq!(
        encoded["journal_payload_sha256"],
        ArtifactDigest::hash_bytes(b"journal one").as_str()
    );
    assert_eq!(encoded["operation_handle_ref"], "handle-1");
    let length = exact.frontier().canonical_record_jcs().as_slice().len();
    for (frontier_bound, journal_bound) in [(0, 32), (length - 1, 32), (4096, 0), (4096, 10)] {
        assert_eq!(
            OperationCollectionJournal::prepare(
                &coordinate("handle-1", '2'),
                candidate.accumulator().binding().transaction().clone(),
                append(store(), b"journal one"),
                frontier_bound,
                journal_bound
            ),
            Err(OperationCollectionJournalError::BoundExceeded)
        );
    }
    let foreign = AuthorityStoreCoordinate::new(
        store().store_id().clone(),
        ControlIncarnationIdV1::parse("foreign-incarnation").unwrap(),
    );
    assert_eq!(
        OperationCollectionJournal::prepare(
            &coordinate("handle-1", '2'),
            candidate.accumulator().binding().transaction().clone(),
            append(foreign, b"journal one"),
            4096,
            32
        ),
        Err(OperationCollectionJournalError::Store)
    );
}

#[test]
fn collection_frontier_domain_accepts_exact_uppercase_and_rejects_legacy_mixed_case() {
    let candidate = observation();
    let exact = pair(&candidate, b"journal one");
    assert!(
        OperationCollectionFrontierEvidence::admit(
            exact.frontier(),
            &store(),
            exact.operation_handle(),
            4096,
        )
        .is_ok(),
        "the exact uppercase collection-frontier domain must be admitted"
    );

    let mut legacy =
        pale_core::canon::parse_strict(exact.frontier().canonical_record_jcs().as_slice()).unwrap();
    legacy["domain"] = "Pale_OPERATION_COLLECTION_FRONTIER_V1".into();
    let legacy_bytes = pale_core::canon::to_canonical_string(&legacy)
        .unwrap()
        .into_bytes();
    let legacy_digest = ArtifactDigest::hash_bytes(&legacy_bytes);
    let legacy_reference = ResidentRecordRefV1::parse(format!(
        "pale/private-operation-collection-frontier/v1/{}",
        legacy_digest.as_str()
    ))
    .unwrap();
    let legacy = ControlRecord::new(
        ControlRecordCoordinate::new(
            legacy_reference,
            ControlRecordDigest::parse(legacy_digest.as_str()).unwrap(),
        ),
        exact.frontier().schema_id().clone(),
        exact.frontier().schema_sha256().clone(),
        ControlRecordBytes::new(legacy_bytes).unwrap(),
    );

    assert_eq!(
        OperationCollectionFrontierEvidence::admit(
            &legacy,
            &store(),
            exact.operation_handle(),
            4096,
        ),
        Err(OperationCollectionJournalError::Association),
        "the legacy mixed-case domain must fail after its bytes, digest, and reference are made internally consistent"
    );
}

#[test]
fn snapshot_rejects_a_frontier_from_another_admitted_coordinate() {
    let candidate = observation();
    let exact = pair(&candidate, b"journal one");
    let changed = pair(&candidate, b"journal two");
    let evidence = OperationCollectionFrontierEvidence::admit(
        exact.frontier(),
        &store(),
        exact.operation_handle(),
        4096,
    )
    .expect("exact frontier evidence");
    let head = OperationCollectionHeadObservation::for_handle(
        exact.operation_handle(),
        changed.frontier().coordinate().clone(),
    );
    assert_eq!(
        OperationCollectionSnapshot::new(
            changed.frontier().clone(),
            &evidence,
            head,
            exact.append().entry().bytes().clone(),
        ),
        Err(OperationCollectionJournalError::Association),
        "a valid frontier cannot be paired with evidence admitted from another coordinate"
    );
}

#[test]
fn pair_is_observe_only_and_cannot_be_detached_aliased_or_rebound() {
    let candidate = observation();
    let journal = pair(&candidate, b"journal one");
    let attached = candidate
        .clone()
        .with_collection_journal(journal.clone())
        .unwrap();
    assert_eq!(attached.records().len(), 2);
    assert_eq!(attached.indexes().len(), candidate.indexes().len() + 1);
    assert!(attached.indexes().iter().any(|index| matches!(
        index,
        OperationCustodyIndexMutation::CollectionHead { successor, .. }
            if successor == journal.frontier().coordinate()
    )));
    assert_eq!(attached.collection_journal(), Some(&journal));
    assert_eq!(
        attached.record(OperationCustodyRecordKind::CollectionFrontier),
        Some(journal.frontier())
    );
    let indexes = candidate
        .indexes()
        .iter()
        .filter(|index| {
            matches!(
                index,
                OperationCustodyIndexMutation::LatestObservation { .. }
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        OperationCustodyCandidate::new(
            store(),
            OperationCustodyTransitionKind::Observe,
            attached.records().to_vec(),
            indexes,
            candidate.checkpoint().clone(),
            candidate.accumulator().clone()
        )
        .unwrap_err(),
        OperationCustodyCandidateError::CollectionJournalBinding
    );
    assert_eq!(
        attached
            .with_collection_journal(journal.clone())
            .unwrap_err(),
        OperationCustodyCandidateError::CollectionJournalBinding
    );
    let start = OperationCustodyCandidate::new_start(
        store(),
        OperationCustodyTransitionKind::StartRoot,
        start_records(),
        root_start_indexes(),
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
        start_session_basis(),
    )
    .unwrap();
    assert_eq!(
        start.with_collection_journal(journal).unwrap_err(),
        OperationCustodyCandidateError::CollectionJournalBinding
    );
    let unlike = crate::ControlTransactionPreparation::new(
        store(),
        pale_core::records::ControlTransactionSequenceV1::new(90).unwrap(),
    )
    .unwrap();
    let journal = OperationCollectionJournal::prepare(
        &coordinate("handle-1", '2'),
        unlike,
        append(store(), b"journal one"),
        4096,
        32,
    )
    .unwrap();
    assert_eq!(
        candidate.with_collection_journal(journal).unwrap_err(),
        OperationCustodyCandidateError::CollectionJournalBinding
    );
}

#[test]
fn cancellation_request_and_collection_frontier_form_one_closed_observation_atom() {
    let candidate = cancellation_observation();
    let journal = pair(&candidate, b"cancelled roster");
    let attached = candidate
        .with_collection_journal(journal.clone())
        .expect("one signed request may close with the stopped private roster");

    assert_eq!(attached.records().len(), 3);
    assert_eq!(
        attached.record(OperationCustodyRecordKind::CommonRequest),
        cancellation_observation().record(OperationCustodyRecordKind::CommonRequest)
    );
    assert_eq!(
        attached.record(OperationCustodyRecordKind::CollectionFrontier),
        Some(journal.frontier())
    );
    assert_eq!(attached.collection_journal(), Some(&journal));
}
