//! Raw A2A custody bounds do not recanonicalize protocol bytes or erase SSE identity.

use super::*;
use pale_core::records::{
    ImmutableRecordReferenceV1, A2A_ROUTE_ADMISSION_V1_SCHEMA, A2A_TASK_BINDING_V1_SCHEMA,
    A2A_TASK_OBSERVATION_V1_SCHEMA,
};

#[test]
fn raw_evidence_keeps_exact_bytes_and_counts_sse_metadata_in_its_bound() {
    let bytes = b"{ \"jsonrpc\" : \"2.0\", \"id\" : \"rpc-1\" }\n".to_vec();
    let reference = ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse("a2a/raw").unwrap(),
        record_sha256: ArtifactDigest::hash_bytes(&bytes),
    };
    let metadata = Some(("event-1".to_owned(), "message".to_owned()));
    let bound = bytes.len() + 14;
    let exact =
        OperationA2aEvidence::new(reference.clone(), bytes.clone(), metadata.clone(), bound)
            .unwrap();
    assert_eq!(exact.bytes(), bytes);
    assert_eq!(exact.sse(), Some(("event-1", "message")));
    assert_eq!(exact.byte_len(), bound);
    assert!(matches!(
        OperationA2aEvidence::new(reference.clone(), bytes.clone(), metadata, bound - 1),
        Err(OperationCustodyCandidateError::A2aEvidence)
    ));
    let changed_metadata = OperationA2aEvidence::new(
        reference.clone(),
        bytes.clone(),
        Some(("event-2".into(), "message".into())),
        bound,
    )
    .unwrap();
    assert_eq!(exact.coordinate(), changed_metadata.coordinate());
    assert_ne!(
        exact, changed_metadata,
        "metadata has its own exact custody identity"
    );
    let mut changed_bytes = bytes;
    changed_bytes.push(b' ');
    assert!(matches!(
        OperationA2aEvidence::new(reference, changed_bytes, None, bound),
        Err(OperationCustodyCandidateError::A2aEvidence)
    ));
}

fn response_evidence(reference: &str) -> OperationA2aEvidence {
    OperationA2aEvidence::new(
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse(reference).unwrap(),
            record_sha256: ArtifactDigest::hash_bytes(b"response"),
        },
        b"response".to_vec(),
        None,
        8,
    )
    .unwrap()
}

fn ordinary_observation() -> OperationCustodyCandidate {
    let predecessor = record("observation-1", '3', OPERATION_OBSERVATION_V1_SCHEMA);
    let successor = record("observation-2", '4', OPERATION_OBSERVATION_V1_SCHEMA);
    OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
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

#[test]
fn ordinary_observation_accepts_at_most_two_disjoint_a2a_items() {
    let candidate = ordinary_observation()
        .with_a2a_evidence_items(vec![
            response_evidence("a2a-response-1"),
            response_evidence("a2a-response-2"),
        ])
        .unwrap();
    assert_eq!(candidate.kind(), OperationCustodyTransitionKind::Observe);
    assert_eq!(candidate.a2a_evidence().len(), 2);
    assert!(candidate.a2a_retained_evidence().is_empty());
    assert!(candidate.a2a_retained_records().is_empty());
    assert!(candidate.a2a_associated_route().is_none());

    assert_eq!(
        candidate
            .with_a2a_evidence_items(vec![response_evidence("a2a-response-3")])
            .unwrap_err(),
        OperationCustodyCandidateError::A2aEvidence
    );
    for invalid in [
        Vec::new(),
        vec![
            response_evidence("a2a-one"),
            response_evidence("a2a-two"),
            response_evidence("a2a-three"),
        ],
    ] {
        assert_eq!(
            ordinary_observation()
                .with_a2a_evidence_items(invalid)
                .unwrap_err(),
            OperationCustodyCandidateError::A2aEvidence
        );
    }
    assert_eq!(
        ordinary_observation()
            .with_a2a_evidence_items(vec![
                response_evidence("a2a-duplicate"),
                response_evidence("a2a-duplicate"),
            ])
            .unwrap_err(),
        OperationCustodyCandidateError::DuplicateRecordReference
    );
    for alias in [
        "observation-1",
        "observation-2",
        "checkpoint-8",
        "checkpoint-9",
    ] {
        assert_eq!(
            ordinary_observation()
                .with_a2a_evidence_items(vec![response_evidence(alias)])
                .unwrap_err(),
            OperationCustodyCandidateError::DuplicateRecordReference
        );
    }
}

#[test]
fn send_preparation_index_is_derived_from_the_exact_retained_dispatch() {
    let preparation = record(
        "send-preparation",
        '4',
        PRIVATE_OPERATION_A2A_SEND_PREPARATION_V1_SCHEMA,
    );
    let dispatch = record("dispatch", '3', OPERATION_OBSERVATION_V1_SCHEMA);
    let records = vec![
        OperationCustodyRecord::A2aSendPreparation(preparation.clone()),
        OperationCustodyRecord::A2aRouteAdmission(record(
            "route",
            '5',
            A2A_ROUTE_ADMISSION_V1_SCHEMA,
        )),
    ];
    let evidence = OperationA2aEvidence::new(
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse("send-request").unwrap(),
            record_sha256: ArtifactDigest::hash_bytes(b"request"),
        },
        b"request".to_vec(),
        None,
        7,
    )
    .unwrap();
    let dependencies = || OperationA2aDependencies {
        new_evidence: vec![evidence.clone()],
        retained_evidence: Vec::new(),
        retained_records: vec![
            record("handle", '2', OPERATION_HANDLE_V1_SCHEMA),
            dispatch.clone(),
        ],
        associated_route: None,
    };
    let make = |index_key| {
        let checkpoint = checkpoint_transition();
        OperationCustodyCandidate::new_a2a_custody(
            store(),
            OperationCustodyTransitionKind::PrepareA2aSend,
            records.clone(),
            vec![OperationCustodyIndexMutation::A2aSendPreparation {
                key: index_key,
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: preparation.coordinate().clone(),
            }],
            checkpoint.clone(),
            accumulator(&checkpoint),
            dependencies(),
        )
    };
    let exact_key = OperationCustodyIndexKey::for_a2a_send_preparation(dispatch.coordinate());
    let exact = make(exact_key).unwrap();
    assert_eq!(exact.indexes().len(), 2);
    assert_eq!(
        make(key('a')).unwrap_err(),
        OperationCustodyCandidateError::IndexPredecessor
    );
}

fn get_task_parents(remote: ControlRecord) -> Vec<ControlRecord> {
    vec![
        record("handle-get-task", '2', OPERATION_HANDLE_V1_SCHEMA),
        record("dispatch-get-task", '3', OPERATION_OBSERVATION_V1_SCHEMA),
        record("route-get-task", '5', A2A_ROUTE_ADMISSION_V1_SCHEMA),
        record("binding-get-task", '6', A2A_TASK_BINDING_V1_SCHEMA),
        remote,
    ]
}

fn get_task_request_evidence() -> OperationA2aEvidence {
    OperationA2aEvidence::new(
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse("get-task-request").unwrap(),
            record_sha256: ArtifactDigest::hash_bytes(b"get-task"),
        },
        b"get-task".to_vec(),
        None,
        8,
    )
    .unwrap()
}

fn get_task_preparation_candidate(
    predecessor: OperationCustodyIndexPredecessor,
) -> Result<OperationCustodyCandidate, OperationCustodyCandidateError> {
    let preparation = record(
        "get-task-preparation",
        '8',
        PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
    );
    let checkpoint = checkpoint_transition();
    OperationCustodyCandidate::new_a2a_custody(
        store(),
        OperationCustodyTransitionKind::PrepareA2aGetTask,
        vec![OperationCustodyRecord::A2aGetTaskPreparation(
            preparation.clone(),
        )],
        vec![OperationCustodyIndexMutation::CurrentA2aGetTask {
            key: key('a'),
            expected: predecessor,
            successor: preparation.coordinate().clone(),
        }],
        checkpoint.clone(),
        accumulator(&checkpoint),
        OperationA2aDependencies {
            new_evidence: vec![get_task_request_evidence()],
            retained_evidence: Vec::new(),
            retained_records: get_task_parents(record(
                "remote-get-task",
                '7',
                A2A_TASK_OBSERVATION_V1_SCHEMA,
            )),
            associated_route: None,
        },
    )
}

#[test]
fn get_task_preparation_accepts_only_absence_or_an_exact_terminal_predecessor() {
    let absent = get_task_preparation_candidate(OperationCustodyIndexPredecessor::Absent)
        .expect("first read preparation");
    assert!(absent.indexes().iter().any(|index| matches!(
        index,
        OperationCustodyIndexMutation::A2aGetTaskCheckpoint { key, successor }
        if key == &OperationCustodyIndexKey::for_a2a_get_task_checkpoint(
            absent.records()[0].record().coordinate()
        ) && successor == absent.checkpoint().successor_record().coordinate()
    )));

    let disposition = record(
        "get-task-disposition",
        '9',
        PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA,
    );
    assert!(
        get_task_preparation_candidate(OperationCustodyIndexPredecessor::Exact(disposition))
            .is_ok()
    );

    let preparation = record(
        "older-get-task-preparation",
        'a',
        PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
    );
    assert_eq!(
        get_task_preparation_candidate(OperationCustodyIndexPredecessor::Exact(preparation))
            .unwrap_err(),
        OperationCustodyCandidateError::IndexPredecessor
    );
}

#[test]
fn terminal_remote_observation_releases_the_get_task_index_for_a_fresh_read() {
    let remote = record("remote-get-task", '7', A2A_TASK_OBSERVATION_V1_SCHEMA);
    let preparation = record(
        "get-task-preparation",
        '8',
        PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
    );
    let checkpoint = checkpoint_transition();
    let candidate = OperationCustodyCandidate::new_a2a_custody(
        store(),
        OperationCustodyTransitionKind::PrepareA2aGetTask,
        vec![OperationCustodyRecord::A2aGetTaskPreparation(
            preparation.clone(),
        )],
        vec![OperationCustodyIndexMutation::CurrentA2aGetTask {
            key: key('a'),
            expected: OperationCustodyIndexPredecessor::Exact(remote.clone()),
            successor: preparation.coordinate().clone(),
        }],
        checkpoint.clone(),
        accumulator(&checkpoint),
        OperationA2aDependencies {
            new_evidence: vec![get_task_request_evidence()],
            retained_evidence: Vec::new(),
            retained_records: get_task_parents(remote),
            associated_route: None,
        },
    )
    .expect("fresh read after a completed public observation");
    assert_eq!(candidate.indexes().len(), 2);
}

#[test]
fn local_disposition_closes_only_the_exact_preparation() {
    let preparation = record(
        "get-task-preparation",
        '8',
        PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
    );
    let disposition = record(
        "get-task-disposition",
        '9',
        PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA,
    );
    let make = |expected| {
        let checkpoint = checkpoint_transition();
        OperationCustodyCandidate::new_a2a_custody(
            store(),
            OperationCustodyTransitionKind::DisposeA2aGetTask,
            vec![OperationCustodyRecord::A2aGetTaskLocalDisposition(
                disposition.clone(),
            )],
            vec![OperationCustodyIndexMutation::CurrentA2aGetTask {
                key: key('a'),
                expected,
                successor: disposition.coordinate().clone(),
            }],
            checkpoint.clone(),
            accumulator(&checkpoint),
            OperationA2aDependencies {
                new_evidence: Vec::new(),
                retained_evidence: Vec::new(),
                retained_records: Vec::new(),
                associated_route: None,
            },
        )
    };
    let closed = make(OperationCustodyIndexPredecessor::Exact(preparation)).unwrap();
    assert!(closed.indexes().iter().any(|index| matches!(
        index,
        OperationCustodyIndexMutation::A2aGetTaskCheckpoint { .. }
    )));
    assert_eq!(
        make(OperationCustodyIndexPredecessor::Absent).unwrap_err(),
        OperationCustodyCandidateError::IndexPredecessor
    );
}

#[test]
fn binding_atom_requires_first_observation_and_disjoint_exact_evidence() {
    // Opaque port-shape fixture only; contextual issuers and real SQLite are
    // exercised by the application/adapter journey, not asserted here.
    let route = record("route", '5', A2A_ROUTE_ADMISSION_V1_SCHEMA);
    let binding = record("binding", '6', A2A_TASK_BINDING_V1_SCHEMA);
    let observation = record("remote-observation", '7', A2A_TASK_OBSERVATION_V1_SCHEMA);
    let records = vec![
        OperationCustodyRecord::A2aRouteAdmission(route),
        OperationCustodyRecord::A2aTaskBinding(binding.clone()),
        OperationCustodyRecord::A2aTaskObservation(observation.clone()),
    ];
    let indexes = vec![
        OperationCustodyIndexMutation::A2aTaskBinding {
            operation_key: key('2'),
            key: key('a'),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: binding.coordinate().clone(),
        },
        OperationCustodyIndexMutation::LatestA2aTaskObservation {
            key: key('a'),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: observation.coordinate().clone(),
        },
    ];
    let raw = |name: &str| {
        OperationA2aEvidence::new(
            ImmutableRecordReferenceV1 {
                record_ref: ResidentRecordRefV1::parse(name).unwrap(),
                record_sha256: ArtifactDigest::hash_bytes(b"raw"),
            },
            b"raw".to_vec(),
            None,
            3,
        )
        .unwrap()
    };
    let make = |records, appended, retained| {
        OperationCustodyCandidate::new_a2a_custody(
            store(),
            OperationCustodyTransitionKind::BindA2aTask,
            records,
            indexes.clone(),
            checkpoint_transition(),
            accumulator(&checkpoint_transition()),
            crate::OperationA2aDependencies {
                associated_route: None,
                new_evidence: appended,
                retained_evidence: retained,
                retained_records: vec![
                    record("handle", '2', OPERATION_HANDLE_V1_SCHEMA),
                    record("dispatch", '3', OPERATION_OBSERVATION_V1_SCHEMA),
                ],
            },
        )
    };
    let exact = make(
        records.clone(),
        vec![raw("request"), raw("response")],
        vec![],
    )
    .unwrap();
    assert_eq!(exact.indexes().len(), 4);
    assert!(exact.indexes().iter().any(|index| matches!(index,
        OperationCustodyIndexMutation::FirstA2aTaskObservation { successor, .. }
        if successor == observation.coordinate())));
    assert!(exact.indexes().iter().any(|index| matches!(index,
        OperationCustodyIndexMutation::A2aTaskObservationCheckpoint { successor, .. }
        if successor == exact.checkpoint().successor_record().coordinate())));
    let retained = make(records.clone(), vec![raw("response")], vec![raw("request")]).unwrap();
    assert_eq!(retained.a2a_evidence().len(), 1);
    assert_eq!(retained.a2a_retained_evidence().len(), 1);
    for alias in ["binding", "dispatch", "handle", "remote-observation"] {
        assert_eq!(
            make(records.clone(), vec![raw(alias)], vec![]).unwrap_err(),
            OperationCustodyCandidateError::DuplicateRecordReference
        );
    }
    let prior = checkpoint_transition();
    let prior_ref = prior
        .predecessor_record()
        .unwrap()
        .coordinate()
        .record_ref()
        .as_str();
    assert_eq!(
        make(records.clone(), vec![raw(prior_ref)], vec![]).unwrap_err(),
        OperationCustodyCandidateError::DuplicateRecordReference
    );
    assert_eq!(
        make(records.clone(), vec![], vec![raw(prior_ref)]).unwrap_err(),
        OperationCustodyCandidateError::DuplicateRecordReference
    );
    assert!(make(records[..2].to_vec(), vec![raw("request")], vec![]).is_err());
    assert!(make(records.clone(), vec![], vec![]).is_err());
    assert_eq!(
        make(records.clone(), vec![raw("request")], vec![raw("request")]).unwrap_err(),
        OperationCustodyCandidateError::DuplicateRecordReference
    );
    let next = record("remote-observation-2", '8', A2A_TASK_OBSERVATION_V1_SCHEMA);
    let make_successor = |new_evidence, retained_evidence| {
        OperationCustodyCandidate::new_a2a_custody(
            store(),
            OperationCustodyTransitionKind::ObserveA2aTask,
            vec![OperationCustodyRecord::A2aTaskObservation(next.clone())],
            vec![
                OperationCustodyIndexMutation::A2aTaskBinding {
                    operation_key: key('2'),
                    key: key('a'),
                    expected: OperationCustodyIndexPredecessor::Exact(binding.clone()),
                    successor: binding.coordinate().clone(),
                },
                OperationCustodyIndexMutation::LatestA2aTaskObservation {
                    key: key('a'),
                    expected: OperationCustodyIndexPredecessor::Exact(observation.clone()),
                    successor: next.coordinate().clone(),
                },
            ],
            checkpoint_transition(),
            accumulator(&checkpoint_transition()),
            crate::OperationA2aDependencies {
                associated_route: None,
                new_evidence,
                retained_evidence,
                retained_records: vec![
                    binding.clone(),
                    records[0].record().clone(),
                    record("handle", '2', OPERATION_HANDLE_V1_SCHEMA),
                    record("dispatch", '3', OPERATION_OBSERVATION_V1_SCHEMA),
                ],
            },
        )
    };
    assert!(
        make_successor(vec![raw("valid-response")], vec![]).is_ok(),
        "exact retained binding is a required dependency, not an alias"
    );
    assert_eq!(
        make_successor(vec![raw("remote-observation")], vec![]).unwrap_err(),
        OperationCustodyCandidateError::DuplicateRecordReference
    );
    assert_eq!(
        make_successor(vec![], vec![raw("remote-observation")]).unwrap_err(),
        OperationCustodyCandidateError::DuplicateRecordReference
    );
    assert!(OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::BindA2aTask,
        records,
        indexes,
        checkpoint_transition(),
        accumulator(&checkpoint_transition())
    )
    .is_err());
}
