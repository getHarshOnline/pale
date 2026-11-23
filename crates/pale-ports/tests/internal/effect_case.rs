//! Behavioral tests for closed effect-case physical transitions.

use super::*;
use crate::ControlRecordBytes;
use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId,
};

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record reference"),
        ControlRecordDigest::parse(digest(byte)).expect("record digest"),
    )
}

fn record(reference: &str, byte: char, schema_id: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse(schema_id).expect("schema identity"),
        SchemaDigest::parse(digest('f')).expect("schema digest"),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes())
            .expect("record bytes"),
    )
}

fn checkpoint(sequence: u64, reference: &str, byte: char) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("effect:resource-1").expect("checkpoint domain"),
        ControlDomainEpochV1::new(4).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        pale_core::records::ControlTransactionSequenceV1::new(sequence + 20).unwrap(),
        coordinate(reference, byte),
    )
}

fn key(byte: char) -> EffectCaseIndexKey {
    EffectCaseIndexKey::parse(digest(byte)).expect("index key")
}

fn store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").expect("store identity"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation identity"),
    )
}

fn candidate(
    kind: EffectCaseTransitionKind,
    members: Vec<EffectCaseTransitionMember>,
    index_mutations: Vec<EffectCaseIndexMutation>,
) -> Result<EffectCaseTransitionCandidate, EffectCaseCandidateError> {
    let checkpoint = EffectCaseCheckpointTransition::new(
        checkpoint(8, "checkpoint-8", '8'),
        record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
        checkpoint(9, "checkpoint-9", '9'),
        record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
    )
    .expect("checkpoint transition");
    EffectCaseTransitionCandidate::new(
        store(),
        kind,
        members.clone(),
        indexes_with_disposition(kind, &members, index_mutations),
        assertions(kind),
        match kind {
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting) => {
                EffectCaseCheckpointMutation::Unchanged
            }
            _ => EffectCaseCheckpointMutation::advance(checkpoint),
        },
    )
}

fn indexes_with_disposition(
    kind: EffectCaseTransitionKind,
    members: &[EffectCaseTransitionMember],
    mut indexes: Vec<EffectCaseIndexMutation>,
) -> Vec<EffectCaseIndexMutation> {
    if indexes
        .iter()
        .any(|index| matches!(index, EffectCaseIndexMutation::ActionDisposition { .. }))
        || matches!(
            kind,
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting)
                | EffectCaseTransitionKind::TransferCustody
        )
    {
        return indexes;
    }

    let successor = |member_kind| {
        members
            .iter()
            .find(|member| member.kind() == member_kind)
            .expect("disposition successor member")
            .record()
            .coordinate()
            .clone()
    };
    let (key, expected, successor) = match kind {
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New) => {
            return indexes;
        }
        EffectCaseTransitionKind::CommitDispatchIntent => (
            indexes
                .iter()
                .find_map(|index| match index {
                    EffectCaseIndexMutation::ActionIntent { key, .. } => Some(key.clone()),
                    _ => None,
                })
                .expect("action-intent key"),
            record("admission-1", '4', EFFECT_ADMISSION_V1_SCHEMA),
            successor(EffectCaseTransitionMemberKind::DispatchIntent),
        ),
        EffectCaseTransitionKind::CancelBeforeDispatch => (
            key('9'),
            record("admission-1", '4', EFFECT_ADMISSION_V1_SCHEMA),
            successor(EffectCaseTransitionMemberKind::ConflictClaim),
        ),
        EffectCaseTransitionKind::SettleWithoutReconciliation => (
            key('9'),
            record("intent-1", '9', EFFECT_DISPATCH_INTENT_V1_SCHEMA),
            successor(EffectCaseTransitionMemberKind::Receipt),
        ),
        EffectCaseTransitionKind::OpenReconciliation => (
            key('9'),
            record("intent-1", '9', EFFECT_DISPATCH_INTENT_V1_SCHEMA),
            successor(EffectCaseTransitionMemberKind::ReconciliationCase),
        ),
        EffectCaseTransitionKind::AdvanceReconciliation(_) => (
            key('9'),
            indexes
                .iter()
                .find_map(|index| match index {
                    EffectCaseIndexMutation::CaseResult {
                        expected: EffectCaseIndexPredecessor::Exact(record),
                        ..
                    } => Some(record.clone()),
                    _ => None,
                })
                .expect("case-result predecessor"),
            successor(EffectCaseTransitionMemberKind::ReconciliationResult),
        ),
        EffectCaseTransitionKind::ReopenReconciliation => (
            key('9'),
            record("result-terminal", '8', RECONCILIATION_RESULT_V1_SCHEMA),
            successor(EffectCaseTransitionMemberKind::ReconciliationCase),
        ),
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting)
        | EffectCaseTransitionKind::TransferCustody => unreachable!("returned above"),
    };
    indexes.push(EffectCaseIndexMutation::ActionDisposition {
        key,
        expected: EffectCaseIndexPredecessor::Exact(expected),
        successor,
    });
    indexes
}

fn assertions(kind: EffectCaseTransitionKind) -> Vec<EffectCaseIndexAssertion> {
    match kind {
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New) => {
            vec![EffectCaseIndexAssertion::ActiveCustody {
                key: key('c'),
                expected: EffectCaseIndexPredecessor::Absent,
            }]
        }
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting) => {
            vec![
                EffectCaseIndexAssertion::LogicalAction {
                    key: key('b'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "admission-1",
                        '4',
                        EFFECT_ADMISSION_V1_SCHEMA,
                    )),
                },
                EffectCaseIndexAssertion::ActionDisposition {
                    key: key('b'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "receipt-1",
                        '5',
                        EFFECT_RECEIPT_V1_SCHEMA,
                    )),
                },
            ]
        }
        EffectCaseTransitionKind::CommitDispatchIntent => {
            vec![EffectCaseIndexAssertion::ActiveClaim {
                key: key('8'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "claim-active",
                    '8',
                    EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                )),
            }]
        }
        EffectCaseTransitionKind::CancelBeforeDispatch => {
            vec![EffectCaseIndexAssertion::ActionIntent {
                key: key('9'),
                expected: EffectCaseIndexPredecessor::Absent,
            }]
        }
        EffectCaseTransitionKind::SettleWithoutReconciliation
        | EffectCaseTransitionKind::OpenReconciliation => {
            vec![EffectCaseIndexAssertion::ActionIntent {
                key: key('9'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "intent-1",
                    '9',
                    EFFECT_DISPATCH_INTENT_V1_SCHEMA,
                )),
            }]
        }
        EffectCaseTransitionKind::AdvanceReconciliation(
            EffectCaseReconciliationAdvanceKind::Open,
        ) => vec![EffectCaseIndexAssertion::ActiveCustody {
            key: key('8'),
            expected: EffectCaseIndexPredecessor::Exact(record(
                "custody-open",
                '8',
                RECONCILIATION_CUSTODY_V1_SCHEMA,
            )),
        }],
        EffectCaseTransitionKind::ReopenReconciliation => {
            vec![
                EffectCaseIndexAssertion::ActiveClaim {
                    key: key('e'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "claim-terminal",
                        '7',
                        EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                    )),
                },
                EffectCaseIndexAssertion::CaseResult {
                    key: key('8'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "result-terminal",
                        '8',
                        RECONCILIATION_RESULT_V1_SCHEMA,
                    )),
                },
            ]
        }
        EffectCaseTransitionKind::AdvanceReconciliation(
            EffectCaseReconciliationAdvanceKind::Terminal,
        ) => Vec::new(),
        EffectCaseTransitionKind::TransferCustody => {
            vec![EffectCaseIndexAssertion::ActionDisposition {
                key: key('9'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "case-current",
                    '9',
                    RECONCILIATION_CASE_V1_SCHEMA,
                )),
            }]
        }
    }
}

fn admit_members() -> Vec<EffectCaseTransitionMember> {
    vec![
        EffectCaseTransitionMember::CommonRequest(record(
            "request-1",
            '1',
            COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::Action(record("action-1", '2', ACTION_ENVELOPE_V1_SCHEMA)),
        EffectCaseTransitionMember::ConflictClaim(record(
            "claim-1",
            '3',
            EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::Admission(record(
            "admission-1",
            '4',
            EFFECT_ADMISSION_V1_SCHEMA,
        )),
    ]
}

fn admit_indexes() -> Vec<EffectCaseIndexMutation> {
    vec![
        EffectCaseIndexMutation::RequestIdentity {
            key: key('a'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate("request-1", '1'),
        },
        EffectCaseIndexMutation::LogicalAction {
            key: key('b'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate("admission-1", '4'),
        },
        EffectCaseIndexMutation::ActionDisposition {
            key: key('b'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate("admission-1", '4'),
        },
        EffectCaseIndexMutation::ActiveClaim {
            key: key('c'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate("claim-1", '3'),
        },
    ]
}

#[test]
fn admit_action_closes_request_action_claim_admission_and_four_indexes() {
    let transition = candidate(
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
        admit_members(),
        admit_indexes(),
    )
    .expect("closed admission transition");

    assert_eq!(
        transition.kind(),
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New)
    );
    assert_eq!(transition.members().len(), 4);
    assert_eq!(transition.index_mutations().len(), 4);
    assert_eq!(transition.index_assertions().len(), 1);
    assert_eq!(
        transition
            .expected_checkpoint()
            .expect("advanced checkpoint")
            .domain_sequence()
            .get(),
        8
    );
    assert_eq!(
        transition
            .successor_checkpoint()
            .expect("advanced checkpoint")
            .domain_sequence()
            .get(),
        9
    );
}

#[test]
fn each_lifecycle_kind_requires_its_exact_record_and_index_shape() {
    let cases = [
        (
            EffectCaseTransitionKind::CommitDispatchIntent,
            vec![EffectCaseTransitionMember::DispatchIntent(record(
                "intent-1",
                '1',
                EFFECT_DISPATCH_INTENT_V1_SCHEMA,
            ))],
            vec![EffectCaseIndexMutation::ActionIntent {
                key: key('a'),
                expected: EffectCaseIndexPredecessor::Absent,
                successor: coordinate("intent-1", '1'),
            }],
        ),
        (
            EffectCaseTransitionKind::CancelBeforeDispatch,
            vec![EffectCaseTransitionMember::ConflictClaim(record(
                "claim-released",
                '2',
                EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
            ))],
            vec![EffectCaseIndexMutation::ActiveClaim {
                key: key('b'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "claim-active",
                    'a',
                    EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                )),
                successor: coordinate("claim-released", '2'),
            }],
        ),
        (
            EffectCaseTransitionKind::SettleWithoutReconciliation,
            vec![
                EffectCaseTransitionMember::Receipt(record(
                    "receipt-1",
                    '5',
                    EFFECT_RECEIPT_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ConflictClaim(record(
                    "claim-released",
                    '6',
                    EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                )),
            ],
            vec![EffectCaseIndexMutation::ActiveClaim {
                key: key('e'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "claim-active",
                    'd',
                    EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                )),
                successor: coordinate("claim-released", '6'),
            }],
        ),
        (
            EffectCaseTransitionKind::OpenReconciliation,
            vec![
                EffectCaseTransitionMember::Receipt(record(
                    "receipt-unknown",
                    '5',
                    EFFECT_RECEIPT_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ConflictClaim(record(
                    "claim-transferred",
                    '6',
                    EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationCase(record(
                    "case-1",
                    '7',
                    RECONCILIATION_CASE_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationCustody(record(
                    "custody-1",
                    'a',
                    RECONCILIATION_CUSTODY_V1_SCHEMA,
                )),
            ],
            vec![
                EffectCaseIndexMutation::ActiveClaim {
                    key: key('f'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "claim-active",
                        'd',
                        EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                    )),
                    successor: coordinate("claim-transferred", '6'),
                },
                EffectCaseIndexMutation::ActiveCustody {
                    key: key('f'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("custody-1", 'a'),
                },
                EffectCaseIndexMutation::CaseResult {
                    key: key('2'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("case-1", '7'),
                },
            ],
        ),
        (
            EffectCaseTransitionKind::AdvanceReconciliation(
                EffectCaseReconciliationAdvanceKind::Open,
            ),
            vec![
                EffectCaseTransitionMember::CommonRequest(record(
                    "reconciliation-common-open",
                    '1',
                    COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationRequest(record(
                    "reconciliation-request-open",
                    '2',
                    RECONCILIATION_REQUEST_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationResult(record(
                    "result-2",
                    '3',
                    RECONCILIATION_RESULT_V1_SCHEMA,
                )),
            ],
            vec![
                EffectCaseIndexMutation::RequestIdentity {
                    key: key('7'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("reconciliation-common-open", '1'),
                },
                EffectCaseIndexMutation::CaseResult {
                    key: key('c'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "result-1",
                        'b',
                        RECONCILIATION_RESULT_V1_SCHEMA,
                    )),
                    successor: coordinate("result-2", '3'),
                },
            ],
        ),
        (
            EffectCaseTransitionKind::AdvanceReconciliation(
                EffectCaseReconciliationAdvanceKind::Terminal,
            ),
            vec![
                EffectCaseTransitionMember::CommonRequest(record(
                    "reconciliation-common-terminal",
                    '1',
                    COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationRequest(record(
                    "reconciliation-request-terminal",
                    '2',
                    RECONCILIATION_REQUEST_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationResult(record(
                    "result-terminal",
                    '3',
                    RECONCILIATION_RESULT_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationCustody(record(
                    "custody-closed",
                    '4',
                    RECONCILIATION_CUSTODY_V1_SCHEMA,
                )),
            ],
            vec![
                EffectCaseIndexMutation::RequestIdentity {
                    key: key('7'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("reconciliation-common-terminal", '1'),
                },
                EffectCaseIndexMutation::CaseResult {
                    key: key('c'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "result-1",
                        'b',
                        RECONCILIATION_RESULT_V1_SCHEMA,
                    )),
                    successor: coordinate("result-terminal", '3'),
                },
                EffectCaseIndexMutation::ActiveCustody {
                    key: key('d'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "custody-open",
                        'c',
                        RECONCILIATION_CUSTODY_V1_SCHEMA,
                    )),
                    successor: coordinate("custody-closed", '4'),
                },
            ],
        ),
        (
            EffectCaseTransitionKind::TransferCustody,
            vec![EffectCaseTransitionMember::ReconciliationCustody(record(
                "custody-2",
                '4',
                RECONCILIATION_CUSTODY_V1_SCHEMA,
            ))],
            vec![EffectCaseIndexMutation::ActiveCustody {
                key: key('d'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "custody-1",
                    'c',
                    RECONCILIATION_CUSTODY_V1_SCHEMA,
                )),
                successor: coordinate("custody-2", '4'),
            }],
        ),
        (
            EffectCaseTransitionKind::ReopenReconciliation,
            vec![
                EffectCaseTransitionMember::ReconciliationCase(record(
                    "case-2",
                    '5',
                    RECONCILIATION_CASE_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationCustody(record(
                    "custody-2",
                    '6',
                    RECONCILIATION_CUSTODY_V1_SCHEMA,
                )),
            ],
            vec![
                EffectCaseIndexMutation::ActiveCustody {
                    key: key('e'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "custody-1",
                        'd',
                        RECONCILIATION_CUSTODY_V1_SCHEMA,
                    )),
                    successor: coordinate("custody-2", '6'),
                },
                EffectCaseIndexMutation::CaseResult {
                    key: key('f'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("case-2", '5'),
                },
            ],
        ),
    ];

    for (kind, members, indexes) in cases {
        assert!(candidate(kind, members, indexes).is_ok(), "{kind:?}");
    }
}

#[test]
fn admission_shape_cannot_omit_request_or_retarget_an_index() {
    let mut members = admit_members();
    members.remove(0);
    assert_eq!(
        candidate(
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
            members,
            admit_indexes()
        ),
        Err(EffectCaseCandidateError::MemberShape)
    );

    let mut indexes = admit_indexes();
    indexes[1] = EffectCaseIndexMutation::LogicalAction {
        key: key('b'),
        expected: EffectCaseIndexPredecessor::Absent,
        successor: coordinate("action-1", '2'),
    };
    assert_eq!(
        candidate(
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
            admit_members(),
            indexes
        ),
        Err(EffectCaseCandidateError::IndexSuccessorClosure)
    );

    let mut indexes = admit_indexes();
    indexes[0] = EffectCaseIndexMutation::RequestIdentity {
        key: key('a'),
        expected: EffectCaseIndexPredecessor::Exact(record(
            "old-request",
            'e',
            COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
        )),
        successor: coordinate("request-1", '1'),
    };
    assert_eq!(
        candidate(
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
            admit_members(),
            indexes
        ),
        Err(EffectCaseCandidateError::IndexPredecessorShape)
    );
}

#[test]
fn claim_and_custody_use_one_resource_key_and_reads_never_duplicate_writes() {
    let mut indexes = admit_indexes();
    indexes[3] = EffectCaseIndexMutation::ActiveClaim {
        key: key('d'),
        expected: EffectCaseIndexPredecessor::Absent,
        successor: coordinate("claim-1", '3'),
    };
    assert_eq!(
        candidate(
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
            admit_members(),
            indexes,
        ),
        Err(EffectCaseCandidateError::ConflictResourceKeyMismatch)
    );

    let kind = EffectCaseTransitionKind::ReopenReconciliation;
    let members = vec![
        EffectCaseTransitionMember::ReconciliationCase(record(
            "case-2",
            '5',
            RECONCILIATION_CASE_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::ReconciliationCustody(record(
            "custody-2",
            '6',
            RECONCILIATION_CUSTODY_V1_SCHEMA,
        )),
    ];
    let mutations = vec![
        EffectCaseIndexMutation::ActiveCustody {
            key: key('e'),
            expected: EffectCaseIndexPredecessor::Exact(record(
                "custody-1",
                'd',
                RECONCILIATION_CUSTODY_V1_SCHEMA,
            )),
            successor: coordinate("custody-2", '6'),
        },
        EffectCaseIndexMutation::CaseResult {
            key: key('8'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate("case-2", '5'),
        },
    ];
    assert_eq!(
        EffectCaseTransitionCandidate::new(
            store(),
            kind,
            members.clone(),
            indexes_with_disposition(kind, &members, mutations),
            assertions(kind),
            EffectCaseCheckpointMutation::advance(
                EffectCaseCheckpointTransition::new(
                    checkpoint(8, "checkpoint-8", '8'),
                    record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                    checkpoint(9, "checkpoint-9", '9'),
                    record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                )
                .expect("checkpoint transition")
            ),
        ),
        Err(EffectCaseCandidateError::AssertionMutationOverlap)
    );

    let kind = EffectCaseTransitionKind::ReopenReconciliation;
    let members = vec![
        EffectCaseTransitionMember::ReconciliationCase(record(
            "case-2",
            '5',
            RECONCILIATION_CASE_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::ReconciliationCustody(record(
            "custody-2",
            '6',
            RECONCILIATION_CUSTODY_V1_SCHEMA,
        )),
    ];
    let mutations = vec![
        EffectCaseIndexMutation::ActiveCustody {
            key: key('e'),
            expected: EffectCaseIndexPredecessor::Exact(record(
                "custody-1",
                'd',
                RECONCILIATION_CUSTODY_V1_SCHEMA,
            )),
            successor: coordinate("custody-2", '6'),
        },
        EffectCaseIndexMutation::CaseResult {
            key: key('f'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate("case-2", '5'),
        },
    ];
    let mut mismatched_assertions = assertions(kind);
    mismatched_assertions[0] = EffectCaseIndexAssertion::ActiveClaim {
        key: key('a'),
        expected: EffectCaseIndexPredecessor::Exact(record(
            "claim-terminal",
            '7',
            EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
        )),
    };
    assert_eq!(
        EffectCaseTransitionCandidate::new(
            store(),
            kind,
            members.clone(),
            indexes_with_disposition(kind, &members, mutations),
            mismatched_assertions,
            EffectCaseCheckpointMutation::advance(
                EffectCaseCheckpointTransition::new(
                    checkpoint(8, "checkpoint-8", '8'),
                    record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                    checkpoint(9, "checkpoint-9", '9'),
                    record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                )
                .expect("checkpoint transition")
            ),
        ),
        Err(EffectCaseCandidateError::ConflictResourceKeyMismatch)
    );
}

#[test]
fn reopen_requires_exact_terminal_claim_and_result_predecessors() {
    let kind = EffectCaseTransitionKind::ReopenReconciliation;
    let members = vec![
        EffectCaseTransitionMember::ReconciliationCase(record(
            "case-2",
            '5',
            RECONCILIATION_CASE_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::ReconciliationCustody(record(
            "custody-2",
            '6',
            RECONCILIATION_CUSTODY_V1_SCHEMA,
        )),
    ];
    let mutations = vec![
        EffectCaseIndexMutation::ActiveCustody {
            key: key('e'),
            expected: EffectCaseIndexPredecessor::Exact(record(
                "custody-closed",
                'd',
                RECONCILIATION_CUSTODY_V1_SCHEMA,
            )),
            successor: coordinate("custody-2", '6'),
        },
        EffectCaseIndexMutation::CaseResult {
            key: key('f'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate("case-2", '5'),
        },
    ];

    let mut missing_claim = assertions(kind);
    missing_claim.remove(0);
    assert_eq!(
        EffectCaseTransitionCandidate::new(
            store(),
            kind,
            members.clone(),
            indexes_with_disposition(kind, &members, mutations.clone()),
            missing_claim,
            EffectCaseCheckpointMutation::advance(
                EffectCaseCheckpointTransition::new(
                    checkpoint(8, "checkpoint-8", '8'),
                    record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                    checkpoint(9, "checkpoint-9", '9'),
                    record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                )
                .expect("checkpoint transition")
            ),
        ),
        Err(EffectCaseCandidateError::AssertionShape)
    );

    let mut nonterminal_case = assertions(kind);
    nonterminal_case[1] = EffectCaseIndexAssertion::CaseResult {
        key: key('8'),
        expected: EffectCaseIndexPredecessor::Exact(record(
            "case-terminal",
            '8',
            RECONCILIATION_CASE_V1_SCHEMA,
        )),
    };
    assert_eq!(
        EffectCaseTransitionCandidate::new(
            store(),
            kind,
            members.clone(),
            indexes_with_disposition(kind, &members, mutations),
            nonterminal_case,
            EffectCaseCheckpointMutation::advance(
                EffectCaseCheckpointTransition::new(
                    checkpoint(8, "checkpoint-8", '8'),
                    record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                    checkpoint(9, "checkpoint-9", '9'),
                    record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                )
                .expect("checkpoint transition")
            ),
        ),
        Err(EffectCaseCandidateError::AssertionPredecessorShape)
    );
}

#[test]
fn reconciliation_advance_cannot_cross_open_and_terminal_shapes() {
    let terminal_members = vec![
        EffectCaseTransitionMember::CommonRequest(record(
            "reconciliation-common-terminal",
            '3',
            COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::ReconciliationRequest(record(
            "reconciliation-request-terminal",
            '4',
            RECONCILIATION_REQUEST_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::ReconciliationResult(record(
            "result-terminal",
            '1',
            RECONCILIATION_RESULT_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::ReconciliationCustody(record(
            "custody-closed",
            '2',
            RECONCILIATION_CUSTODY_V1_SCHEMA,
        )),
    ];
    let terminal_indexes = vec![
        EffectCaseIndexMutation::RequestIdentity {
            key: key('e'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate("reconciliation-common-terminal", '3'),
        },
        EffectCaseIndexMutation::CaseResult {
            key: key('a'),
            expected: EffectCaseIndexPredecessor::Exact(record(
                "result-open",
                'b',
                RECONCILIATION_RESULT_V1_SCHEMA,
            )),
            successor: coordinate("result-terminal", '1'),
        },
        EffectCaseIndexMutation::ActiveCustody {
            key: key('c'),
            expected: EffectCaseIndexPredecessor::Exact(record(
                "custody-open",
                'd',
                RECONCILIATION_CUSTODY_V1_SCHEMA,
            )),
            successor: coordinate("custody-closed", '2'),
        },
    ];

    assert_eq!(
        candidate(
            EffectCaseTransitionKind::AdvanceReconciliation(
                EffectCaseReconciliationAdvanceKind::Open,
            ),
            terminal_members,
            terminal_indexes,
        ),
        Err(EffectCaseCandidateError::MemberShape)
    );
    assert_eq!(
        candidate(
            EffectCaseTransitionKind::AdvanceReconciliation(
                EffectCaseReconciliationAdvanceKind::Terminal,
            ),
            vec![
                EffectCaseTransitionMember::CommonRequest(record(
                    "reconciliation-common-open",
                    '3',
                    COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationRequest(record(
                    "reconciliation-request-open",
                    '4',
                    RECONCILIATION_REQUEST_V1_SCHEMA,
                )),
                EffectCaseTransitionMember::ReconciliationResult(record(
                    "result-open",
                    '1',
                    RECONCILIATION_RESULT_V1_SCHEMA,
                )),
            ],
            vec![
                EffectCaseIndexMutation::RequestIdentity {
                    key: key('e'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("reconciliation-common-open", '3'),
                },
                EffectCaseIndexMutation::CaseResult {
                    key: key('a'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "result-1",
                        'b',
                        RECONCILIATION_RESULT_V1_SCHEMA,
                    )),
                    successor: coordinate("result-open", '1'),
                },
            ],
        ),
        Err(EffectCaseCandidateError::MemberShape)
    );
}

#[test]
fn candidate_rejects_schema_mismatch_coordinate_alias_and_non_successor_checkpoint() {
    let mut members = admit_members();
    members[3] =
        EffectCaseTransitionMember::Admission(record("admission-1", '4', EFFECT_RECEIPT_V1_SCHEMA));
    assert_eq!(
        candidate(
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
            members,
            admit_indexes()
        ),
        Err(EffectCaseCandidateError::MemberSchema)
    );

    let checkpoint_transition = EffectCaseCheckpointTransition::new(
        checkpoint(8, "checkpoint-8", '8'),
        record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
        checkpoint(9, "checkpoint-9", '9'),
        record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
    )
    .expect("checkpoint transition");
    let kind = EffectCaseTransitionKind::CommitDispatchIntent;
    let members = vec![EffectCaseTransitionMember::DispatchIntent(record(
        "checkpoint-8",
        '8',
        EFFECT_DISPATCH_INTENT_V1_SCHEMA,
    ))];
    let result = EffectCaseTransitionCandidate::new(
        store(),
        kind,
        members.clone(),
        indexes_with_disposition(
            kind,
            &members,
            vec![EffectCaseIndexMutation::ActionIntent {
                key: key('a'),
                expected: EffectCaseIndexPredecessor::Absent,
                successor: coordinate("checkpoint-8", '8'),
            }],
        ),
        assertions(kind),
        EffectCaseCheckpointMutation::advance(checkpoint_transition),
    );
    assert_eq!(
        result,
        Err(EffectCaseCandidateError::DuplicateRecordCoordinate)
    );

    let checkpoint_transition = EffectCaseCheckpointTransition::new(
        checkpoint(8, "checkpoint-8", '8'),
        record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
        checkpoint(10, "checkpoint-10", 'a'),
        record("checkpoint-10", 'a', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
    );
    assert_eq!(
        checkpoint_transition,
        Err(EffectCaseCandidateError::CheckpointSuccession)
    );

    let members = vec![EffectCaseTransitionMember::DispatchIntent(record(
        "intent-1",
        '1',
        EFFECT_DISPATCH_INTENT_V1_SCHEMA,
    ))];
    let result = EffectCaseTransitionCandidate::new(
        store(),
        kind,
        members.clone(),
        indexes_with_disposition(
            kind,
            &members,
            vec![EffectCaseIndexMutation::ActionIntent {
                key: key('a'),
                expected: EffectCaseIndexPredecessor::Absent,
                successor: coordinate("intent-1", '1'),
            }],
        ),
        assertions(kind),
        EffectCaseCheckpointMutation::advance(
            EffectCaseCheckpointTransition::new(
                checkpoint(8, "checkpoint-8", '8'),
                record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
                checkpoint(9, "checkpoint-9", '9'),
                record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
            )
            .expect("checkpoint transition"),
        ),
    );
    assert!(result.is_ok());
}

#[test]
fn signed_record_schema_identity_is_namespaced_not_the_registry_resource_id() {
    let mut members = admit_members();
    members[3] =
        EffectCaseTransitionMember::Admission(record("admission-1", '4', "effect-admission.v1"));

    assert_eq!(EFFECT_ADMISSION_V1_SCHEMA, "pale/effect-admission.v1");
    assert_eq!(
        candidate(
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
            members,
            admit_indexes(),
        ),
        Err(EffectCaseCandidateError::MemberSchema)
    );
}

#[test]
fn alias_admission_binds_only_a_new_request_and_preserves_effect_custody() {
    let kind = EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting);
    let request = record("request-alias", '6', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);
    let transition = candidate(
        kind,
        vec![EffectCaseTransitionMember::CommonRequest(request.clone())],
        vec![EffectCaseIndexMutation::RequestIdentity {
            key: key('d'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: request.coordinate().clone(),
        }],
    )
    .expect("alias transaction");

    assert_eq!(transition.members().len(), 1);
    assert_eq!(transition.index_mutations().len(), 1);
    assert_eq!(transition.index_assertions().len(), 2);
    assert_eq!(
        transition.checkpoint_mutation(),
        &EffectCaseCheckpointMutation::Unchanged
    );
    assert!(transition.expected_checkpoint().is_none());
    assert!(transition.successor_checkpoint().is_none());

    let request_observation = EffectCaseIndexObservation::exact_record(
        EffectCaseIndexQuery::RequestIdentity(key('d')),
        request,
    )
    .expect("request-identity observation");
    let logical_observation = EffectCaseIndexObservation::exact_record(
        EffectCaseIndexQuery::LogicalAction(key('b')),
        record("admission-1", '4', EFFECT_ADMISSION_V1_SCHEMA),
    )
    .expect("logical-action observation");
    let disposition_observation = EffectCaseIndexObservation::exact_record(
        EffectCaseIndexQuery::ActionDisposition(key('b')),
        record("receipt-1", '5', EFFECT_RECEIPT_V1_SCHEMA),
    )
    .expect("action-disposition observation");

    assert_eq!(
        request_observation
            .record()
            .expect("request record")
            .coordinate()
            .record_ref()
            .as_str(),
        "request-alias"
    );
    assert_eq!(
        logical_observation
            .record()
            .expect("logical action")
            .schema_id()
            .as_str(),
        EFFECT_ADMISSION_V1_SCHEMA
    );
    assert_eq!(
        disposition_observation
            .record()
            .expect("current disposition")
            .schema_id()
            .as_str(),
        EFFECT_RECEIPT_V1_SCHEMA
    );
}

#[test]
fn alias_rejects_changed_request_reuse_without_reopening_the_effect() {
    let kind = EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting);
    let request = record("request-alias", '6', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);

    assert_eq!(
        candidate(
            kind,
            vec![EffectCaseTransitionMember::CommonRequest(request.clone())],
            vec![EffectCaseIndexMutation::RequestIdentity {
                key: key('d'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "request-original",
                    '7',
                    COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
                )),
                successor: request.coordinate().clone(),
            }],
        ),
        Err(EffectCaseCandidateError::IndexPredecessorShape)
    );
}

#[test]
fn disposition_compare_and_set_uses_the_exact_current_record() {
    let kind = EffectCaseTransitionKind::SettleWithoutReconciliation;
    let members = vec![
        EffectCaseTransitionMember::Receipt(record("receipt-1", '5', EFFECT_RECEIPT_V1_SCHEMA)),
        EffectCaseTransitionMember::ConflictClaim(record(
            "claim-released",
            '6',
            EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
        )),
    ];
    let mut indexes = indexes_with_disposition(
        kind,
        &members,
        vec![EffectCaseIndexMutation::ActiveClaim {
            key: key('e'),
            expected: EffectCaseIndexPredecessor::Exact(record(
                "claim-active",
                'd',
                EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
            )),
            successor: coordinate("claim-released", '6'),
        }],
    );
    let disposition = indexes
        .iter_mut()
        .find(|index| matches!(index, EffectCaseIndexMutation::ActionDisposition { .. }))
        .expect("disposition mutation");
    if let EffectCaseIndexMutation::ActionDisposition { expected, .. } = disposition {
        *expected = EffectCaseIndexPredecessor::Exact(record(
            "another-intent",
            'a',
            EFFECT_DISPATCH_INTENT_V1_SCHEMA,
        ));
    }

    assert_eq!(
        candidate(kind, members, indexes),
        Err(EffectCaseCandidateError::ActionDispositionTransition)
    );
}

#[test]
fn point_observations_reject_wrong_schema_and_record_coordinate() {
    assert_eq!(
        EffectCaseIndexObservation::exact_record(
            EffectCaseIndexQuery::LogicalAction(key('b')),
            record("receipt-1", '5', EFFECT_RECEIPT_V1_SCHEMA),
        ),
        Err(EffectCaseIndexObservationError::RecordSchema)
    );

    let requested = coordinate("receipt-1", '5');
    let exact = EffectCaseRecordObservation::exact(
        requested.clone(),
        record("receipt-1", '5', EFFECT_RECEIPT_V1_SCHEMA),
    )
    .expect("exact record observation");
    assert_eq!(exact.coordinate(), &requested);
    assert_eq!(
        exact.record().expect("present record").coordinate(),
        &requested
    );

    assert_eq!(
        EffectCaseRecordObservation::exact(
            requested,
            record("receipt-2", '6', EFFECT_RECEIPT_V1_SCHEMA),
        ),
        Err(EffectCaseIndexObservationError::RecordCoordinate)
    );
}
