//! Behavioral laws for application-owned effect-case custody phases.

use std::collections::{BTreeSet, VecDeque};

use pale_core::records::{
    ChangedControlLeafKeyV1, CheckpointProfileDigest, CheckpointSigningAuthorityDigest,
    ControlCheckpointDomainIdV1, ControlCheckpointIdV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlIncarnationIdV1, ControlLeafCountV1, ControlLogicalKeyV1,
    ControlRecordClassV1, ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1,
    ControlStateCheckpointV1, ControlStoreIdV1, ControlTransactionSequenceV1, ResidentKeyIdV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId, SigningKeyEpochV1,
    UnixTimeV1, ACTION_ENVELOPE_V1_SCHEMA, COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, EFFECT_ADMISSION_V1_SCHEMA,
    EFFECT_CONFLICT_CLAIM_V1_SCHEMA, EFFECT_DISPATCH_INTENT_V1_SCHEMA, EFFECT_RECEIPT_V1_SCHEMA,
    RECONCILIATION_CASE_V1_SCHEMA, RECONCILIATION_CUSTODY_V1_SCHEMA,
    RECONCILIATION_REQUEST_V1_SCHEMA, RECONCILIATION_RESULT_V1_SCHEMA,
};
use pale_ports::{
    AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError, AntiRollbackAnchorErrorKind,
    AntiRollbackAnchorObservation, AntiRollbackAnchorPort, AuthorityStoreCoordinate,
    ControlCheckpointHead, ControlDomainAntiRollbackAnchor, ControlRecord, ControlRecordBytes,
    ControlRecordCoordinate, ControlRecordDigest, ControlStoreError, ControlStoreErrorKind,
    EffectCaseActionAdmissionKind, EffectCaseCheckpointMutation, EffectCaseCheckpointTransition,
    EffectCaseCommitOutcome, EffectCaseCustodyPort, EffectCaseIndexAssertion, EffectCaseIndexKey,
    EffectCaseIndexMutation, EffectCaseIndexObservation, EffectCaseIndexPredecessor,
    EffectCaseIndexQuery, EffectCaseReadBackOutcome, EffectCaseReconciliationAdvanceKind,
    EffectCaseRecordObservation, EffectCaseTransitionCandidate, EffectCaseTransitionKind,
    EffectCaseTransitionMember, EffectCaseTransitionMemberKind,
    PreparedAntiRollbackAnchorTransition,
};

use super::phase::{EffectCaseCommitFailureReason, EffectCaseCommitSuccess};
use super::record::{
    derive_key, project_intent, project_receipt, project_request, project_result, EffectCaseKeys,
};
use super::replay::{classify_effect_case_replay, EffectCaseReplayDisposition};
use super::*;
use crate::resident::{
    read_operation_start, read_operation_start_for_request, OperationRecoveryError,
};
use pale_ports::{
    OperationCustodyCandidate, OperationCustodyCommitOutcome, OperationCustodyIndexKey,
    OperationCustodyIndexObservation, OperationCustodyIndexQuery, OperationCustodyPort,
    OperationCustodyReadBackOutcome,
};

use crate::effect_case_lineage_fixture as lineage;

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

fn key(byte: char) -> EffectCaseIndexKey {
    EffectCaseIndexKey::parse(digest(byte)).expect("index key")
}

fn store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").expect("store identity"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation identity"),
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

fn checkpoint_transition() -> EffectCaseCheckpointTransition {
    EffectCaseCheckpointTransition::new(
        checkpoint(8, "checkpoint-8", '8'),
        record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
        checkpoint(9, "checkpoint-9", '9'),
        record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
    )
    .expect("checkpoint transition")
}

struct AdmittedCheckpointPair {
    predecessor_ref: ResidentRecordRefV1,
    predecessor: ControlStateCheckpointV1,
    successor_ref: ResidentRecordRefV1,
    successor: ControlStateCheckpointV1,
}

fn checkpoint_preimage(sequence: u64) -> ControlStateCheckpointPreimageV1 {
    ControlStateCheckpointPreimageV1 {
        schema_id: SchemaId::parse(CONTROL_STATE_CHECKPOINT_V1_SCHEMA).expect("checkpoint schema"),
        schema_sha256: SchemaDigest::parse(digest('a')).expect("checkpoint schema digest"),
        control_checkpoint_id: ControlCheckpointIdV1::parse(format!("checkpoint-{sequence}"))
            .expect("checkpoint identity"),
        control_store_id: ControlStoreIdV1::parse("control-store").expect("store identity"),
        control_incarnation_id: ControlIncarnationIdV1::parse("incarnation-1")
            .expect("incarnation identity"),
        checkpoint_domain_id: ControlCheckpointDomainIdV1::parse("effect:resource-1")
            .expect("checkpoint domain"),
        domain_epoch: ControlDomainEpochV1::new(1).expect("domain epoch"),
        domain_sequence: ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        previous_control_checkpoint_ref: None,
        previous_control_checkpoint_sha256: None,
        checkpoint_profile_ref: ResidentRecordRefV1::parse("checkpoint-profile")
            .expect("profile reference"),
        checkpoint_profile_sha256: CheckpointProfileDigest::parse(digest('b'))
            .expect("profile digest"),
        control_schema_version: 1,
        committed_transaction_sequence: ControlTransactionSequenceV1::new(sequence)
            .expect("transaction sequence"),
        state_root_sha256: pale_core::records::ArtifactDigest::parse(digest('c'))
            .expect("state root"),
        leaf_count: ControlLeafCountV1::new(1).expect("leaf count"),
        changed_leaf_keys: vec![ChangedControlLeafKeyV1 {
            record_class: ControlRecordClassV1::parse("effect-case").expect("record class"),
            scope_key_sha256: pale_core::records::ArtifactDigest::parse(digest('d'))
                .expect("scope digest"),
            logical_key: ControlLogicalKeyV1::parse("action-1").expect("logical key"),
        }],
        unresolved_custody_root_sha256: pale_core::records::ArtifactDigest::parse(digest('e'))
            .expect("custody root"),
        issued_at: UnixTimeV1::new(sequence).expect("checkpoint time"),
        checkpoint_signing_authority_ref: ResidentRecordRefV1::parse("checkpoint-authority")
            .expect("authority reference"),
        checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest::parse(digest('f'))
            .expect("authority digest"),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
    }
}

fn checkpoint_authority(
    value: &ControlStateCheckpointPreimageV1,
) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
    ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: &value.checkpoint_signing_authority_ref,
        authority_sha256: &value.checkpoint_signing_authority_sha256,
        principal_id: &value.issuer_principal_id,
        control_store_id: &value.control_store_id,
        control_incarnation_id: &value.control_incarnation_id,
        checkpoint_domain_id: &value.checkpoint_domain_id,
        checkpoint_profile_ref: &value.checkpoint_profile_ref,
        checkpoint_profile_sha256: &value.checkpoint_profile_sha256,
        subject_key_id: &value.issuer_key_id,
        subject_key_epoch: value.issuer_key_epoch,
    }
}

fn admitted_checkpoint_pair() -> AdmittedCheckpointPair {
    let predecessor_preimage = checkpoint_preimage(1);
    let predecessor_digest = ControlStateCheckpointDigest::hash_canonical(&predecessor_preimage)
        .expect("predecessor checkpoint digest");
    let predecessor_wire = lineage::signed_wire(
        &predecessor_preimage,
        "canonical_control_checkpoint_sha256",
        &predecessor_digest,
    );
    let predecessor = ControlStateCheckpointV1::admit_initial_wire(
        predecessor_wire,
        &predecessor_preimage.schema_sha256,
        &checkpoint_authority(&predecessor_preimage),
        |_, _, _, _| Ok(()),
    )
    .expect("admitted predecessor checkpoint");
    let predecessor_ref =
        ResidentRecordRefV1::parse("checkpoint-1").expect("predecessor checkpoint reference");
    let mut successor_preimage = checkpoint_preimage(2);
    successor_preimage.previous_control_checkpoint_ref = Some(predecessor_ref.clone());
    successor_preimage.previous_control_checkpoint_sha256 =
        Some(predecessor.canonical_control_checkpoint_sha256().clone());
    let successor_digest = ControlStateCheckpointDigest::hash_canonical(&successor_preimage)
        .expect("successor checkpoint digest");
    let successor_wire = lineage::signed_wire(
        &successor_preimage,
        "canonical_control_checkpoint_sha256",
        &successor_digest,
    );
    let successor = ControlStateCheckpointV1::admit_successor_wire(
        successor_wire,
        &predecessor_ref,
        &predecessor,
        &successor_preimage.schema_sha256,
        &checkpoint_authority(&successor_preimage),
        |_, _, _, _| Ok(()),
    )
    .expect("admitted successor checkpoint");
    AdmittedCheckpointPair {
        predecessor_ref,
        predecessor,
        successor_ref: ResidentRecordRefV1::parse("checkpoint-2")
            .expect("successor checkpoint reference"),
        successor,
    }
}

fn member(
    kind: EffectCaseTransitionMemberKind,
    reference: &str,
    byte: char,
) -> EffectCaseTransitionMember {
    let schema = match kind {
        EffectCaseTransitionMemberKind::CommonRequest => COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
        EffectCaseTransitionMemberKind::Action => ACTION_ENVELOPE_V1_SCHEMA,
        EffectCaseTransitionMemberKind::ConflictClaim => EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
        EffectCaseTransitionMemberKind::Admission => EFFECT_ADMISSION_V1_SCHEMA,
        EffectCaseTransitionMemberKind::DispatchIntent => EFFECT_DISPATCH_INTENT_V1_SCHEMA,
        EffectCaseTransitionMemberKind::Receipt => EFFECT_RECEIPT_V1_SCHEMA,
        EffectCaseTransitionMemberKind::ReconciliationCase => RECONCILIATION_CASE_V1_SCHEMA,
        EffectCaseTransitionMemberKind::ReconciliationRequest => RECONCILIATION_REQUEST_V1_SCHEMA,
        EffectCaseTransitionMemberKind::ReconciliationCustody => RECONCILIATION_CUSTODY_V1_SCHEMA,
        EffectCaseTransitionMemberKind::ReconciliationResult => RECONCILIATION_RESULT_V1_SCHEMA,
    };
    let record = record(reference, byte, schema);
    match kind {
        EffectCaseTransitionMemberKind::CommonRequest => {
            EffectCaseTransitionMember::CommonRequest(record)
        }
        EffectCaseTransitionMemberKind::Action => EffectCaseTransitionMember::Action(record),
        EffectCaseTransitionMemberKind::ConflictClaim => {
            EffectCaseTransitionMember::ConflictClaim(record)
        }
        EffectCaseTransitionMemberKind::Admission => EffectCaseTransitionMember::Admission(record),
        EffectCaseTransitionMemberKind::DispatchIntent => {
            EffectCaseTransitionMember::DispatchIntent(record)
        }
        EffectCaseTransitionMemberKind::Receipt => EffectCaseTransitionMember::Receipt(record),
        EffectCaseTransitionMemberKind::ReconciliationCase => {
            EffectCaseTransitionMember::ReconciliationCase(record)
        }
        EffectCaseTransitionMemberKind::ReconciliationRequest => {
            EffectCaseTransitionMember::ReconciliationRequest(record)
        }
        EffectCaseTransitionMemberKind::ReconciliationCustody => {
            EffectCaseTransitionMember::ReconciliationCustody(record)
        }
        EffectCaseTransitionMemberKind::ReconciliationResult => {
            EffectCaseTransitionMember::ReconciliationResult(record)
        }
    }
}

fn disposition(
    expected: ControlRecord,
    successor: ControlRecordCoordinate,
) -> EffectCaseIndexMutation {
    EffectCaseIndexMutation::ActionDisposition {
        key: key('9'),
        expected: EffectCaseIndexPredecessor::Exact(expected),
        successor,
    }
}

fn candidate(kind: EffectCaseTransitionKind) -> EffectCaseTransitionCandidate {
    let (members, mut mutations, assertions) = shape(kind);
    let disposition_shape = match kind {
        EffectCaseTransitionKind::CommitDispatchIntent => Some((
            record("admission-current", 'a', EFFECT_ADMISSION_V1_SCHEMA),
            EffectCaseTransitionMemberKind::DispatchIntent,
        )),
        EffectCaseTransitionKind::CancelBeforeDispatch => Some((
            record("admission-current", 'a', EFFECT_ADMISSION_V1_SCHEMA),
            EffectCaseTransitionMemberKind::ConflictClaim,
        )),
        EffectCaseTransitionKind::SettleWithoutReconciliation => Some((
            record("intent-current", 'a', EFFECT_DISPATCH_INTENT_V1_SCHEMA),
            EffectCaseTransitionMemberKind::Receipt,
        )),
        EffectCaseTransitionKind::OpenReconciliation => Some((
            record("intent-current", 'a', EFFECT_DISPATCH_INTENT_V1_SCHEMA),
            EffectCaseTransitionMemberKind::ReconciliationCase,
        )),
        EffectCaseTransitionKind::AdvanceReconciliation(_) => Some((
            record("result-current", 'a', RECONCILIATION_RESULT_V1_SCHEMA),
            EffectCaseTransitionMemberKind::ReconciliationResult,
        )),
        EffectCaseTransitionKind::ReopenReconciliation => Some((
            record("result-terminal", 'a', RECONCILIATION_RESULT_V1_SCHEMA),
            EffectCaseTransitionMemberKind::ReconciliationCase,
        )),
        EffectCaseTransitionKind::AdmitAction(_) | EffectCaseTransitionKind::TransferCustody => {
            None
        }
    };
    if let Some((expected, successor_kind)) = disposition_shape {
        let successor = members
            .iter()
            .find(|value| value.kind() == successor_kind)
            .expect("disposition successor")
            .record()
            .coordinate()
            .clone();
        mutations.push(disposition(expected, successor));
    }
    EffectCaseTransitionCandidate::new(
        store(),
        kind,
        members,
        mutations,
        assertions,
        if matches!(
            kind,
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting)
        ) {
            EffectCaseCheckpointMutation::Unchanged
        } else {
            EffectCaseCheckpointMutation::advance(checkpoint_transition())
        },
    )
    .expect("closed effect-case candidate")
}

fn shape(
    kind: EffectCaseTransitionKind,
) -> (
    Vec<EffectCaseTransitionMember>,
    Vec<EffectCaseIndexMutation>,
    Vec<EffectCaseIndexAssertion>,
) {
    match kind {
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New) => (
            vec![
                member(
                    EffectCaseTransitionMemberKind::CommonRequest,
                    "request",
                    '1',
                ),
                member(EffectCaseTransitionMemberKind::Action, "action", '2'),
                member(EffectCaseTransitionMemberKind::ConflictClaim, "claim", '3'),
                member(EffectCaseTransitionMemberKind::Admission, "admission", '4'),
            ],
            vec![
                EffectCaseIndexMutation::RequestIdentity {
                    key: key('1'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("request", '1'),
                },
                EffectCaseIndexMutation::LogicalAction {
                    key: key('2'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("admission", '4'),
                },
                EffectCaseIndexMutation::ActionDisposition {
                    key: key('2'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("admission", '4'),
                },
                EffectCaseIndexMutation::ActiveClaim {
                    key: key('3'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("claim", '3'),
                },
            ],
            vec![EffectCaseIndexAssertion::ActiveCustody {
                key: key('3'),
                expected: EffectCaseIndexPredecessor::Absent,
            }],
        ),
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting) => (
            vec![member(
                EffectCaseTransitionMemberKind::CommonRequest,
                "request-alias",
                '1',
            )],
            vec![EffectCaseIndexMutation::RequestIdentity {
                key: key('1'),
                expected: EffectCaseIndexPredecessor::Absent,
                successor: coordinate("request-alias", '1'),
            }],
            vec![
                EffectCaseIndexAssertion::LogicalAction {
                    key: key('2'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "admission-current",
                        'a',
                        EFFECT_ADMISSION_V1_SCHEMA,
                    )),
                },
                EffectCaseIndexAssertion::ActionDisposition {
                    key: key('2'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "receipt-current",
                        'b',
                        EFFECT_RECEIPT_V1_SCHEMA,
                    )),
                },
            ],
        ),
        EffectCaseTransitionKind::CommitDispatchIntent => (
            vec![member(
                EffectCaseTransitionMemberKind::DispatchIntent,
                "intent",
                '4',
            )],
            vec![EffectCaseIndexMutation::ActionIntent {
                key: key('9'),
                expected: EffectCaseIndexPredecessor::Absent,
                successor: coordinate("intent", '4'),
            }],
            vec![EffectCaseIndexAssertion::ActiveClaim {
                key: key('3'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "claim-current",
                    '3',
                    EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                )),
            }],
        ),
        EffectCaseTransitionKind::CancelBeforeDispatch => (
            vec![member(
                EffectCaseTransitionMemberKind::ConflictClaim,
                "claim-released",
                '4',
            )],
            vec![EffectCaseIndexMutation::ActiveClaim {
                key: key('3'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "claim-current",
                    '3',
                    EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                )),
                successor: coordinate("claim-released", '4'),
            }],
            vec![EffectCaseIndexAssertion::ActionIntent {
                key: key('9'),
                expected: EffectCaseIndexPredecessor::Absent,
            }],
        ),
        EffectCaseTransitionKind::SettleWithoutReconciliation => (
            vec![
                member(EffectCaseTransitionMemberKind::Receipt, "receipt", '4'),
                member(
                    EffectCaseTransitionMemberKind::ConflictClaim,
                    "claim-released",
                    '5',
                ),
            ],
            vec![EffectCaseIndexMutation::ActiveClaim {
                key: key('3'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "claim-current",
                    '3',
                    EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                )),
                successor: coordinate("claim-released", '5'),
            }],
            vec![EffectCaseIndexAssertion::ActionIntent {
                key: key('9'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "intent-current",
                    'a',
                    EFFECT_DISPATCH_INTENT_V1_SCHEMA,
                )),
            }],
        ),
        EffectCaseTransitionKind::OpenReconciliation => (
            vec![
                member(EffectCaseTransitionMemberKind::Receipt, "receipt", '4'),
                member(
                    EffectCaseTransitionMemberKind::ConflictClaim,
                    "claim-transferred",
                    '5',
                ),
                member(
                    EffectCaseTransitionMemberKind::ReconciliationCase,
                    "case",
                    '6',
                ),
                member(
                    EffectCaseTransitionMemberKind::ReconciliationCustody,
                    "custody",
                    '7',
                ),
            ],
            vec![
                EffectCaseIndexMutation::ActiveClaim {
                    key: key('3'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "claim-current",
                        '3',
                        EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                    )),
                    successor: coordinate("claim-transferred", '5'),
                },
                EffectCaseIndexMutation::ActiveCustody {
                    key: key('3'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("custody", '7'),
                },
                EffectCaseIndexMutation::CaseResult {
                    key: key('6'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("case", '6'),
                },
            ],
            vec![EffectCaseIndexAssertion::ActionIntent {
                key: key('9'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "intent-current",
                    'a',
                    EFFECT_DISPATCH_INTENT_V1_SCHEMA,
                )),
            }],
        ),
        EffectCaseTransitionKind::AdvanceReconciliation(advance) => {
            let terminal = advance == EffectCaseReconciliationAdvanceKind::Terminal;
            let mut members = vec![
                member(
                    EffectCaseTransitionMemberKind::CommonRequest,
                    "reconciliation-request",
                    '4',
                ),
                member(
                    EffectCaseTransitionMemberKind::ReconciliationRequest,
                    "reconciliation-body",
                    '5',
                ),
                member(
                    EffectCaseTransitionMemberKind::ReconciliationResult,
                    "result",
                    '6',
                ),
            ];
            let mut mutations = vec![
                EffectCaseIndexMutation::RequestIdentity {
                    key: key('1'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("reconciliation-request", '4'),
                },
                EffectCaseIndexMutation::CaseResult {
                    key: key('6'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "result-current",
                        'a',
                        RECONCILIATION_RESULT_V1_SCHEMA,
                    )),
                    successor: coordinate("result", '6'),
                },
            ];
            let assertions = if terminal {
                members.push(member(
                    EffectCaseTransitionMemberKind::ReconciliationCustody,
                    "custody-closed",
                    '7',
                ));
                mutations.push(EffectCaseIndexMutation::ActiveCustody {
                    key: key('3'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "custody-current",
                        '3',
                        RECONCILIATION_CUSTODY_V1_SCHEMA,
                    )),
                    successor: coordinate("custody-closed", '7'),
                });
                Vec::new()
            } else {
                vec![EffectCaseIndexAssertion::ActiveCustody {
                    key: key('3'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "custody-current",
                        '3',
                        RECONCILIATION_CUSTODY_V1_SCHEMA,
                    )),
                }]
            };
            (members, mutations, assertions)
        }
        EffectCaseTransitionKind::TransferCustody => (
            vec![member(
                EffectCaseTransitionMemberKind::ReconciliationCustody,
                "custody-next",
                '4',
            )],
            vec![EffectCaseIndexMutation::ActiveCustody {
                key: key('3'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "custody-current",
                    '3',
                    RECONCILIATION_CUSTODY_V1_SCHEMA,
                )),
                successor: coordinate("custody-next", '4'),
            }],
            vec![EffectCaseIndexAssertion::ActionDisposition {
                key: key('9'),
                expected: EffectCaseIndexPredecessor::Exact(record(
                    "case-current",
                    'a',
                    RECONCILIATION_CASE_V1_SCHEMA,
                )),
            }],
        ),
        EffectCaseTransitionKind::ReopenReconciliation => (
            vec![
                member(
                    EffectCaseTransitionMemberKind::ReconciliationCase,
                    "case-next",
                    '4',
                ),
                member(
                    EffectCaseTransitionMemberKind::ReconciliationCustody,
                    "custody-next",
                    '5',
                ),
            ],
            vec![
                EffectCaseIndexMutation::ActiveCustody {
                    key: key('3'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "custody-closed",
                        '3',
                        RECONCILIATION_CUSTODY_V1_SCHEMA,
                    )),
                    successor: coordinate("custody-next", '5'),
                },
                EffectCaseIndexMutation::CaseResult {
                    key: key('6'),
                    expected: EffectCaseIndexPredecessor::Absent,
                    successor: coordinate("case-next", '4'),
                },
            ],
            vec![
                EffectCaseIndexAssertion::ActiveClaim {
                    key: key('3'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "claim-transferred",
                        '7',
                        EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
                    )),
                },
                EffectCaseIndexAssertion::CaseResult {
                    key: key('8'),
                    expected: EffectCaseIndexPredecessor::Exact(record(
                        "result-terminal",
                        'a',
                        RECONCILIATION_RESULT_V1_SCHEMA,
                    )),
                },
            ],
        ),
    }
}

fn prepared(kind: EffectCaseTransitionKind) -> PreparedEffectCaseTransition {
    let candidate = candidate(kind);
    let anchor = candidate.expected_checkpoint().map(|expected| {
        let successor = candidate
            .successor_checkpoint()
            .expect("checkpoint successor");
        PreparedAntiRollbackAnchorTransition::successor(
            ControlDomainAntiRollbackAnchor::new(
                candidate.store().store_id().clone(),
                candidate.store().incarnation_id().clone(),
                expected.checkpoint_domain_id().clone(),
                expected.domain_epoch(),
                expected.domain_sequence(),
                expected.coordinate().clone(),
            )
            .into(),
            ControlDomainAntiRollbackAnchor::new(
                candidate.store().store_id().clone(),
                candidate.store().incarnation_id().clone(),
                successor.checkpoint_domain_id().clone(),
                successor.domain_epoch(),
                successor.domain_sequence(),
                successor.coordinate().clone(),
            )
            .into(),
        )
        .expect("prepared anchor")
    });
    let dispatch_intent = matches!(kind, EffectCaseTransitionKind::CommitDispatchIntent)
        .then(|| coordinate("intent", '4'));
    PreparedEffectCaseTransition {
        candidate,
        anchor,
        dispatch_intent,
    }
}

struct TestStore {
    commit: Option<Result<EffectCaseCommitOutcome, ControlStoreError>>,
    read_back: Option<Result<EffectCaseReadBackOutcome, ControlStoreError>>,
    reads: VecDeque<Result<EffectCaseIndexObservation, ControlStoreError>>,
    records: Vec<ControlRecord>,
    commits: usize,
    observations: usize,
}

impl TestStore {
    fn new(commit: Result<EffectCaseCommitOutcome, ControlStoreError>) -> Self {
        Self {
            commit: Some(commit),
            read_back: None,
            reads: VecDeque::new(),
            records: Vec::new(),
            commits: 0,
            observations: 0,
        }
    }
}

impl pale_ports::ControlTransactionPort for TestStore {
    fn prepare_transaction(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        panic!("this physical-custody/readback fixture does not prepare a new checkpoint")
    }
}

impl EffectCaseCustodyPort for TestStore {
    fn compare_and_commit(
        &mut self,
        _: &EffectCaseTransitionCandidate,
    ) -> Result<EffectCaseCommitOutcome, ControlStoreError> {
        self.commits += 1;
        self.commit.take().expect("one commit attempt")
    }

    fn observe_exact(
        &mut self,
        _: &EffectCaseTransitionCandidate,
    ) -> Result<EffectCaseReadBackOutcome, ControlStoreError> {
        self.observations += 1;
        self.read_back.take().expect("one read-back")
    }

    fn read_index(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &EffectCaseIndexQuery,
    ) -> Result<EffectCaseIndexObservation, ControlStoreError> {
        self.reads.pop_front().expect("bounded point read")
    }

    fn read_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
    ) -> Result<EffectCaseRecordObservation, ControlStoreError> {
        match self
            .records
            .iter()
            .find(|record| record.coordinate() == coordinate)
        {
            Some(record) => EffectCaseRecordObservation::exact(coordinate.clone(), record.clone())
                .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)),
            None => Ok(EffectCaseRecordObservation::absent(coordinate.clone())),
        }
    }
}

struct TestAnchor {
    advance: Option<Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError>>,
    observation: Option<Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError>>,
    advances: usize,
    observations: usize,
}

impl TestAnchor {
    fn new(advance: Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError>) -> Self {
        Self {
            advance: Some(advance),
            observation: None,
            advances: 0,
            observations: 0,
        }
    }
}

impl AntiRollbackAnchorPort for TestAnchor {
    fn read(
        &mut self,
        key: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, AntiRollbackAnchorError> {
        let _ = key;
        panic!("effect transition test must not read a recovery anchor")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.advances += 1;
        self.advance.take().expect("one anchor advance")
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations += 1;
        self.observation.take().expect("one anchor observation")
    }
}

fn anchor_pending(
    kind: EffectCaseTransitionKind,
    database: Result<EffectCaseCommitOutcome, ControlStoreError>,
) -> (DatabaseCommittedEffectCaseAnchorPending, TestStore) {
    let mut store = TestStore::new(database);
    let phase = prepared(kind).commit(&mut store).expect("database custody");
    let EffectCaseCommitSuccess::AnchorPending(pending) = phase else {
        panic!("transition must require anchoring")
    };
    (*pending, store)
}

#[test]
fn every_closed_transition_shape_enters_its_expected_custody_phase() {
    let kinds = [
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting),
        EffectCaseTransitionKind::CommitDispatchIntent,
        EffectCaseTransitionKind::CancelBeforeDispatch,
        EffectCaseTransitionKind::SettleWithoutReconciliation,
        EffectCaseTransitionKind::OpenReconciliation,
        EffectCaseTransitionKind::AdvanceReconciliation(EffectCaseReconciliationAdvanceKind::Open),
        EffectCaseTransitionKind::AdvanceReconciliation(
            EffectCaseReconciliationAdvanceKind::Terminal,
        ),
        EffectCaseTransitionKind::TransferCustody,
        EffectCaseTransitionKind::ReopenReconciliation,
    ];
    for kind in kinds {
        let mut store = TestStore::new(Ok(EffectCaseCommitOutcome::Committed));
        let outcome = prepared(kind).commit(&mut store).expect("database custody");
        assert_eq!(store.commits, 1, "{kind:?}");
        assert_eq!(
            matches!(outcome, EffectCaseCommitSuccess::Complete(_)),
            matches!(
                kind,
                EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting)
            ),
            "{kind:?}",
        );
    }
}

#[test]
fn exact_replay_at_either_custody_layer_never_authorizes_dispatch() {
    for (database, anchor) in [
        (
            EffectCaseCommitOutcome::ExactReplay,
            AntiRollbackAnchorAdvanceDisposition::Advanced,
        ),
        (
            EffectCaseCommitOutcome::Committed,
            AntiRollbackAnchorAdvanceDisposition::ExactReplay,
        ),
    ] {
        let (pending, _) =
            anchor_pending(EffectCaseTransitionKind::CommitDispatchIntent, Ok(database));
        let mut anchor = TestAnchor::new(Ok(anchor));
        let mut completed = pending
            .advance_anchor(&mut anchor)
            .expect("anchored custody");
        assert!(completed.take_dispatch_authorization().is_none());
    }
}

#[test]
fn only_a_newly_proven_database_and_anchor_pair_yields_one_affine_intent() {
    let (pending, _) = anchor_pending(
        EffectCaseTransitionKind::CommitDispatchIntent,
        Ok(EffectCaseCommitOutcome::Committed),
    );
    let mut anchor = TestAnchor::new(Ok(AntiRollbackAnchorAdvanceDisposition::Advanced));
    let mut completed = pending
        .advance_anchor(&mut anchor)
        .expect("anchored custody");
    let authorization = completed
        .take_dispatch_authorization()
        .expect("dispatch authorization");
    assert_eq!(authorization.into_intent(), coordinate("intent", '4'));
    assert!(completed.take_dispatch_authorization().is_none());
}

#[test]
fn ambiguous_database_commit_is_observed_once_and_never_blindly_retried() {
    let mut store = TestStore::new(Err(ControlStoreError::new(
        ControlStoreErrorKind::OutcomeUnknown,
    )));
    store.read_back = Some(Ok(EffectCaseReadBackOutcome::Predecessor));
    let failure = match prepared(EffectCaseTransitionKind::CommitDispatchIntent).commit(&mut store)
    {
        Ok(_) => panic!("predecessor must exclude commit"),
        Err(failure) => failure,
    };
    assert!(matches!(
        failure.reason(),
        EffectCaseCommitFailureReason::CommitExcludedByReadBack { .. }
    ));
    assert_eq!((store.commits, store.observations), (1, 1));
}

#[test]
fn ambiguous_anchor_state_is_observe_only_and_retains_the_pending_phase() {
    let (pending, _) = anchor_pending(
        EffectCaseTransitionKind::CommitDispatchIntent,
        Ok(EffectCaseCommitOutcome::Committed),
    );
    let mut anchor = TestAnchor::new(Err(AntiRollbackAnchorError::new(
        AntiRollbackAnchorErrorKind::OutcomeUnknown,
    )));
    anchor.observation = Some(Ok(AntiRollbackAnchorObservation::ExactPredecessor));
    let failure = match pending.advance_anchor(&mut anchor) {
        Ok(_) => panic!("non-successor must remain ambiguous"),
        Err(failure) => failure,
    };
    assert_eq!((anchor.advances, anchor.observations), (1, 1));
    let _retained = failure.into_pending();
}

#[test]
fn private_key_domains_cannot_substitute_for_one_another() {
    let value = serde_json::json!({"same": "preimage"});
    let keys = [
        derive_key(REQUEST_IDENTITY_KEY_DOMAIN, &value).expect("request key"),
        derive_key(LOGICAL_ACTION_KEY_DOMAIN, &value).expect("logical key"),
        derive_key(CONFLICT_RESOURCE_KEY_DOMAIN, &value).expect("conflict key"),
        derive_key(CASE_RESULT_KEY_DOMAIN, &value).expect("case key"),
    ];
    assert_eq!(
        keys.iter()
            .map(EffectCaseIndexKey::as_str)
            .collect::<BTreeSet<_>>()
            .len(),
        4,
    );
}

#[test]
fn terminal_reconciliation_cannot_smuggle_compensation_effect_records() {
    let terminal_kind = EffectCaseTransitionKind::AdvanceReconciliation(
        EffectCaseReconciliationAdvanceKind::Terminal,
    );
    let terminal = candidate(terminal_kind);
    assert!(terminal.members().iter().all(|member| !matches!(
        member.kind(),
        EffectCaseTransitionMemberKind::Action
            | EffectCaseTransitionMemberKind::Admission
            | EffectCaseTransitionMemberKind::DispatchIntent
            | EffectCaseTransitionMemberKind::Receipt
    )));

    let (mut members, mutations, assertions) = shape(terminal_kind);
    members.push(member(
        EffectCaseTransitionMemberKind::Action,
        "smuggled-compensation-action",
        'e',
    ));
    assert!(EffectCaseTransitionCandidate::new(
        store(),
        terminal_kind,
        members,
        mutations,
        assertions,
        EffectCaseCheckpointMutation::advance(checkpoint_transition()),
    )
    .is_err());

    let compensation = candidate(EffectCaseTransitionKind::AdmitAction(
        EffectCaseActionAdmissionKind::New,
    ));
    assert_eq!(
        compensation
            .members()
            .iter()
            .map(EffectCaseTransitionMember::kind)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            EffectCaseTransitionMemberKind::CommonRequest,
            EffectCaseTransitionMemberKind::Action,
            EffectCaseTransitionMemberKind::ConflictClaim,
            EffectCaseTransitionMemberKind::Admission,
        ]),
    );
}

#[test]
fn bounded_replay_classification_uses_exact_indexes_without_scanning() {
    let keys = EffectCaseKeys {
        request: key('1'),
        logical: key('2'),
        conflict: key('3'),
    };
    let expected = record("request", '1', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);
    let admission = record("admission", '2', EFFECT_ADMISSION_V1_SCHEMA);
    let disposition = record("receipt", '3', EFFECT_RECEIPT_V1_SCHEMA);
    let mut port = TestStore::new(Ok(EffectCaseCommitOutcome::Committed));
    port.reads.extend([
        Ok(EffectCaseIndexObservation::exact_record(
            EffectCaseIndexQuery::RequestIdentity(key('1')),
            expected.clone(),
        )
        .expect("request observation")),
        Ok(EffectCaseIndexObservation::exact_record(
            EffectCaseIndexQuery::LogicalAction(key('2')),
            admission.clone(),
        )
        .expect("admission observation")),
        Ok(EffectCaseIndexObservation::exact_record(
            EffectCaseIndexQuery::ActionDisposition(key('2')),
            disposition.clone(),
        )
        .expect("disposition observation")),
    ]);
    let replay = classify_effect_case_replay(&mut port, &store(), keys, expected.clone())
        .expect("exact replay");
    assert_eq!(
        replay,
        EffectCaseReplayDisposition::ExactRequest {
            request: expected,
            admission,
            disposition,
        },
    );
    assert!(port.reads.is_empty());
}

#[test]
fn typed_admission_and_intent_use_the_same_semantic_projection_owner() {
    let fixture = lineage::EffectFixture::new("primary");
    let admission = project_transition(EffectCaseTransitionInput::AdmitNew {
        request: AdmittedEffectCaseRecord::new(&fixture.request_ref, &fixture.request),
        action: AdmittedEffectCaseRecord::new(&fixture.action_ref, &fixture.action),
        claim: AdmittedEffectCaseRecord::new(&fixture.claim_ref, &fixture.claim),
        admission: AdmittedEffectCaseRecord::new(&fixture.admission_ref, &fixture.admission),
        prior_claim: None,
        prior_custody: None,
    })
    .expect("typed admission projection");
    assert_eq!(
        admission.kind,
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
    );

    let intent = project_transition(EffectCaseTransitionInput::CommitDispatchIntent {
        request: AdmittedEffectCaseRecord::new(&fixture.request_ref, &fixture.request),
        action: AdmittedEffectCaseRecord::new(&fixture.action_ref, &fixture.action),
        admission: AdmittedEffectCaseRecord::new(&fixture.admission_ref, &fixture.admission),
        active_claim: AdmittedEffectCaseRecord::new(&fixture.claim_ref, &fixture.claim),
        intent: AdmittedEffectCaseRecord::new(&fixture.intent_ref, &fixture.intent),
    })
    .expect("typed intent projection");
    assert_eq!(intent.kind, EffectCaseTransitionKind::CommitDispatchIntent);
    assert_eq!(
        intent.dispatch_intent.as_ref(),
        intent
            .members
            .first()
            .map(EffectCaseTransitionMember::record)
            .map(ControlRecord::coordinate),
    );
}

/// Request lookup may read physical history but must never retry a transaction.
#[derive(Default)]
struct OperationLookupStore {
    records: Vec<(OperationCustodyIndexQuery, ControlRecord)>,
    reads: Vec<OperationCustodyIndexQuery>,
    returned_buffers: Vec<usize>,
}

impl pale_ports::ControlTransactionPort for OperationLookupStore {
    fn prepare_transaction(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        panic!("this physical-custody/readback fixture does not prepare a new checkpoint")
    }
}

impl OperationCustodyPort for OperationLookupStore {
    fn read_a2a_task_bindings(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationA2aBindingScope,
        _: Option<&pale_ports::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationA2aBindings, pale_ports::ControlStoreError> {
        panic!("remote-task discovery is outside this fixture's tested journey")
    }

    fn read_publication_sources(
        &mut self,
        _store: &pale_ports::AuthorityStoreCoordinate,
        _cursor: Option<&pale_ports::OperationPublicationCursor>,
        _maximum_sources: usize,
        _maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        panic!("publication discovery is outside this fixture's tested journey")
    }

    fn read_checkpoint_head(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_core::records::ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<
        Option<(pale_ports::ControlCheckpointHead, pale_ports::ControlRecord)>,
        pale_ports::ControlStoreError,
    > {
        panic!("this owning phase test does not perform locator recovery")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("this owning phase test does not perform membership recovery")
    }

    fn prepare_accumulator(
        &mut self,
        _: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("unexpected accumulator preparation in custody-only fixture")
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("request-only lookup must not retain operation leases")
    }

    fn read_operation_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        panic!("request-only lookup must not read operation leases")
    }

    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("request-only lookup must not commit")
    }

    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        panic!("request-only lookup must not reconstruct a transaction")
    }

    fn read_index(
        &mut self,
        coordinate: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        _maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        assert_eq!(coordinate, &store());
        self.reads.push(query.clone());
        let record = self
            .records
            .iter()
            .find(|(key, _)| key == query)
            .map(|(_, record)| record.clone());
        if let Some(record) = &record {
            self.returned_buffers
                .push(record.canonical_record_jcs().as_slice().as_ptr().addr());
        }
        Ok(record.map_or_else(
            || OperationCustodyIndexObservation::absent(query.clone()),
            |record| {
                OperationCustodyIndexObservation::exact(query.clone(), record)
                    .expect("fixture record matches its index role")
            },
        ))
    }
}

#[test]
fn typed_operation_lookup_reuses_request_bytes_and_rejects_changed_evidence() {
    let fixture = lineage::EffectFixture::new("lookup");
    let admission = project_transition(EffectCaseTransitionInput::AdmitNew {
        request: AdmittedEffectCaseRecord::new(&fixture.request_ref, &fixture.request),
        action: AdmittedEffectCaseRecord::new(&fixture.action_ref, &fixture.action),
        claim: AdmittedEffectCaseRecord::new(&fixture.claim_ref, &fixture.claim),
        admission: AdmittedEffectCaseRecord::new(&fixture.admission_ref, &fixture.admission),
        prior_claim: None,
        prior_custody: None,
    })
    .expect("effect admission supplies the same request record");
    let expected_request = admission
        .members
        .iter()
        .find_map(|member| match member {
            EffectCaseTransitionMember::CommonRequest(record) => Some(record.clone()),
            _ => None,
        })
        .expect("effect candidate retains its signed request");
    let mut port = OperationLookupStore::default();
    assert!(
        read_operation_start_for_request(&mut port, &store(), &fixture.request, 16 * 1024,)
            .unwrap()
            .is_none()
    );
    assert_eq!(port.reads.len(), 2, "absence requires both request indexes");
    let key = port.reads[0].key().clone();
    assert_eq!(port.reads[1].key(), &key);
    let effect_key = EffectCaseKeys::request(&fixture.request).expect("effect request key");
    assert_ne!(
        key.as_str(),
        effect_key.as_str(),
        "different index domains cannot substitute"
    );

    // This is the predecessor effect-index law, independent of the new shared projector.
    let original_preimage = serde_json::json!({
        "initial_v1_domain": "request_identity",
        "caller_identity": fixture.request.caller().identity.as_str(),
        "signer_key_epoch": fixture.request.caller().signer_key_epoch.get(),
        "request_id": fixture.request.request_id().as_str(),
    });
    assert_eq!(
        effect_key,
        derive_key(REQUEST_IDENTITY_KEY_DOMAIN, &original_preimage).unwrap()
    );

    let initial = record(
        "initial-observation",
        '6',
        pale_core::records::OPERATION_OBSERVATION_V1_SCHEMA,
    );
    let checkpoint = record("start-checkpoint", '7', CONTROL_STATE_CHECKPOINT_V1_SCHEMA);
    port.records = vec![
        (
            OperationCustodyIndexQuery::RequestOperation(key.clone()),
            record(
                "operation-handle",
                '8',
                pale_core::records::OPERATION_HANDLE_V1_SCHEMA,
            ),
        ),
        (
            OperationCustodyIndexQuery::RequestEvidence(key.clone()),
            expected_request.clone(),
        ),
        (
            OperationCustodyIndexQuery::StartObservation(key.clone()),
            initial.clone(),
        ),
        (
            OperationCustodyIndexQuery::ObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(initial.coordinate()),
            ),
            checkpoint.clone(),
        ),
    ];
    port.reads.clear();
    let recovered =
        read_operation_start_for_request(&mut port, &store(), &fixture.request, 16 * 1024)
            .unwrap()
            .expect("existing operation history");
    assert_eq!(recovered.request(), &expected_request);
    assert_eq!(recovered.initial_observation(), &initial);
    assert_eq!(recovered.checkpoint(), &checkpoint);
    assert_eq!(port.reads.len(), 4);
    for (record, allocation) in [
        recovered.handle(),
        recovered.request(),
        recovered.initial_observation(),
        recovered.checkpoint(),
    ]
    .into_iter()
    .zip(&port.returned_buffers)
    {
        assert_eq!(
            record.canonical_record_jcs().as_slice().as_ptr().addr(),
            *allocation,
            "recovery takes each returned buffer without another body copy"
        );
    }

    let mut changed = fixture.request_preimage.clone();
    changed.expires_at = UnixTimeV1::new(99).unwrap();
    let changed = lineage::request(&changed);
    assert_ne!(
        changed.canonical_request_sha256(),
        fixture.request.canonical_request_sha256()
    );
    port.reads.clear();
    assert!(matches!(
        read_operation_start_for_request(&mut port, &store(), &changed, 16 * 1024,),
        Err(OperationRecoveryError::RequestConflict)
    ));
    assert_eq!(port.reads.len(), 2);
    assert_eq!(
        port.reads[0].key(),
        &key,
        "changed contents cannot evade the existing slot"
    );

    port.reads.clear();
    let replacement = ControlRecord::new(
        ControlRecordCoordinate::new(
            lineage::record_ref("replacement-reference"),
            expected_request.coordinate().record_sha256().clone(),
        ),
        expected_request.schema_id().clone(),
        expected_request.schema_sha256().clone(),
        expected_request.canonical_record_jcs().clone(),
    );
    assert!(matches!(
        read_operation_start(&mut port, &store(), &key, &replacement, 16 * 1024,),
        Err(OperationRecoveryError::RequestConflict)
    ));
    assert_eq!(
        port.reads.len(),
        2,
        "a prepared candidate cannot rename the immutable request"
    );

    // Physical metadata and signed bytes must each agree, including partial history.
    for has_handle in [true, false] {
        for mismatch in 0..3 {
            let stored = ControlRecord::new(
                if mismatch == 0 {
                    coordinate("request-metadata", 'c')
                } else {
                    expected_request.coordinate().clone()
                },
                expected_request.schema_id().clone(),
                if mismatch == 1 {
                    SchemaDigest::parse(digest('c')).unwrap()
                } else {
                    expected_request.schema_sha256().clone()
                },
                if mismatch == 2 {
                    ControlRecordBytes::new(b"{}".to_vec()).unwrap()
                } else {
                    expected_request.canonical_record_jcs().clone()
                },
            );
            port.records = vec![(
                OperationCustodyIndexQuery::RequestEvidence(key.clone()),
                stored,
            )];
            if has_handle {
                port.records.push((
                    OperationCustodyIndexQuery::RequestOperation(key.clone()),
                    recovered.handle().clone(),
                ));
            }
            port.reads.clear();
            assert!(matches!(
                read_operation_start_for_request(&mut port, &store(), &fixture.request, 16 * 1024,),
                Err(OperationRecoveryError::RequestConflict)
            ));
            assert_eq!(port.reads.len(), 2);
        }
    }
    port.records = vec![(
        OperationCustodyIndexQuery::RequestEvidence(key),
        expected_request,
    )];
    assert!(matches!(
        read_operation_start_for_request(&mut port, &store(), &fixture.request, 16 * 1024,),
        Err(OperationRecoveryError::IncompleteRequestIndexes)
    ));
}

fn reconciliation_operation_store(
    fixture: &lineage::ReconciliationResultFixture,
    handle: ControlRecord,
) -> OperationLookupStore {
    let effect = &fixture.open.effect;
    let candidate = project_transition(EffectCaseTransitionInput::AdmitNew {
        request: AdmittedEffectCaseRecord::new(&effect.request_ref, &effect.request),
        action: AdmittedEffectCaseRecord::new(&effect.action_ref, &effect.action),
        claim: AdmittedEffectCaseRecord::new(&effect.claim_ref, &effect.claim),
        admission: AdmittedEffectCaseRecord::new(&effect.admission_ref, &effect.admission),
        prior_claim: None,
        prior_custody: None,
    })
    .expect("effect request projection");
    let request = candidate
        .members
        .iter()
        .find_map(|member| match member {
            EffectCaseTransitionMember::CommonRequest(record) => Some(record.clone()),
            _ => None,
        })
        .expect("effect candidate retains its request");
    let mut port = OperationLookupStore::default();
    assert!(
        read_operation_start_for_request(&mut port, &store(), &effect.request, 16 * 1024)
            .expect("request key derivation")
            .is_none()
    );
    let key = port.reads[0].key().clone();
    let initial = record(
        "association-initial-observation",
        '6',
        pale_core::records::OPERATION_OBSERVATION_V1_SCHEMA,
    );
    let checkpoint = record(
        "association-start-checkpoint",
        '7',
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
    );
    port.records = vec![
        (
            OperationCustodyIndexQuery::RequestOperation(key.clone()),
            handle,
        ),
        (
            OperationCustodyIndexQuery::RequestEvidence(key.clone()),
            request,
        ),
        (
            OperationCustodyIndexQuery::StartObservation(key),
            initial.clone(),
        ),
        (
            OperationCustodyIndexQuery::ObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(initial.coordinate()),
            ),
            checkpoint,
        ),
    ];
    port.reads.clear();
    port
}

fn effect_attempt_input<'a>(
    fixture: &'a lineage::ReconciliationResultFixture,
    store: &'a AuthorityStoreCoordinate,
) -> ReconciliationEffectAttemptInput<'a> {
    let effect = &fixture.open.effect;
    ReconciliationEffectAttemptInput {
        store,
        effect_request: AdmittedEffectCaseRecord::new(&effect.request_ref, &effect.request),
        dispatch_intent: AdmittedEffectCaseRecord::new(&effect.intent_ref, &effect.intent),
        effect_receipt: AdmittedEffectCaseRecord::new(
            &fixture.open.receipt_ref,
            &fixture.open.receipt,
        ),
        reconciliation_result: AdmittedEffectCaseRecord::new(&fixture.result_ref, &fixture.result),
    }
}

fn recover_effect_attempt<'a>(
    fixture: &'a lineage::ReconciliationResultFixture,
    store: &'a AuthorityStoreCoordinate,
) -> RecoveredReconciliationEffectAttempt<'a> {
    let input = effect_attempt_input(fixture, store);
    let mut effect_store = TestStore::new(Ok(EffectCaseCommitOutcome::Committed));
    effect_store.records = vec![
        project_request(input.effect_request).unwrap(),
        project_intent(input.dispatch_intent).unwrap(),
        project_receipt(input.effect_receipt).unwrap(),
        project_result(input.reconciliation_result).unwrap(),
    ];
    recover_reconciliation_effect_attempt(input, &mut effect_store)
        .expect("exact stored effect attempt")
}

fn association_input<'a>(
    effect_attempt: RecoveredReconciliationEffectAttempt<'a>,
    expected_operation_handle: &'a ControlRecordCoordinate,
) -> ReconciliationOperationAssociationInput<'a> {
    ReconciliationOperationAssociationInput {
        effect_attempt,
        expected_operation_handle,
        maximum_record_bytes: 16 * 1024,
    }
}

#[test]
fn reconciliation_result_recovers_one_exact_operation_attempt_without_a_write() {
    let fixture = lineage::ReconciliationResultFixture::new("association-positive");
    let handle = record(
        "association-operation-handle",
        '8',
        pale_core::records::OPERATION_HANDLE_V1_SCHEMA,
    );
    let expected = handle.coordinate().clone();
    let store_coordinate = store();
    let effect_attempt = recover_effect_attempt(&fixture, &store_coordinate);
    let mut port = reconciliation_operation_store(&fixture, handle);
    let first = read_reconciliation_operation_association(
        association_input(effect_attempt, &expected),
        &mut port,
    )
    .expect("exact association");
    let second = read_reconciliation_operation_association(
        association_input(effect_attempt, &expected),
        &mut port,
    )
    .expect("exact replay is the same read-only association");
    assert_eq!(first, second);
    assert_eq!(first.operation_handle(), &expected);
    assert_eq!(
        first.effect_attempt_id(),
        fixture.open.effect.intent.attempt_id()
    );
    assert_eq!(
        port.reads.len(),
        8,
        "each read uses four bounded point reads"
    );
    assert!(port
        .reads
        .iter()
        .all(|query| !matches!(query, OperationCustodyIndexQuery::LatestObservation(_))));
}

#[test]
fn reconciliation_result_rejects_wrong_operation_and_attempt() {
    let fixture = lineage::ReconciliationResultFixture::new("association-negative");
    let handle = record(
        "association-operation-handle",
        '8',
        pale_core::records::OPERATION_HANDLE_V1_SCHEMA,
    );
    let store_coordinate = store();
    let effect_attempt = recover_effect_attempt(&fixture, &store_coordinate);
    let mut port = reconciliation_operation_store(&fixture, handle);
    let foreign = coordinate("foreign-operation-handle", '9');
    assert!(matches!(
        read_reconciliation_operation_association(
            association_input(effect_attempt, &foreign),
            &mut port,
        ),
        Err(ReconciliationOperationAssociationError::OperationMismatch)
    ));

    let other = lineage::ReconciliationResultFixture::new("association-other-attempt");
    let effect = &fixture.open.effect;
    let wrong_attempt_input = ReconciliationEffectAttemptInput {
        store: &store_coordinate,
        effect_request: AdmittedEffectCaseRecord::new(&effect.request_ref, &effect.request),
        dispatch_intent: AdmittedEffectCaseRecord::new(
            &other.open.effect.intent_ref,
            &other.open.effect.intent,
        ),
        effect_receipt: AdmittedEffectCaseRecord::new(
            &fixture.open.receipt_ref,
            &fixture.open.receipt,
        ),
        reconciliation_result: AdmittedEffectCaseRecord::new(&fixture.result_ref, &fixture.result),
    };
    let mut effect_store = TestStore::new(Ok(EffectCaseCommitOutcome::Committed));
    assert!(matches!(
        recover_reconciliation_effect_attempt(wrong_attempt_input, &mut effect_store),
        Err(ReconciliationOperationAssociationError::EffectLineage)
    ));
}

#[test]
fn reconciliation_result_requires_exact_persisted_effect_evidence() {
    let fixture = lineage::ReconciliationResultFixture::new("association-missing-evidence");
    let store_coordinate = store();
    let mut empty_store = TestStore::new(Ok(EffectCaseCommitOutcome::Committed));

    assert!(matches!(
        recover_reconciliation_effect_attempt(
            effect_attempt_input(&fixture, &store_coordinate),
            &mut empty_store,
        ),
        Err(ReconciliationOperationAssociationError::MissingEffectEvidence)
    ));
}

#[test]
fn operation_request_lookup_separates_every_identity_coordinate() {
    let fixture = lineage::EffectFixture::new("identity");
    let mut requests = vec![fixture.request.clone()];
    let mut caller = fixture.request_preimage.clone();
    caller.caller.identity = pale_core::records::RequestCallerIdV1::parse("caller-2").unwrap();
    requests.push(lineage::request(&caller));
    let mut request_id = fixture.request_preimage.clone();
    request_id.request_id = pale_core::records::ResidentRequestIdV1::parse("request-2").unwrap();
    requests.push(lineage::request(&request_id));
    let mut epoch = fixture.request_preimage.clone();
    epoch.caller.signer_key_epoch = SigningKeyEpochV1::new(2).unwrap();
    let mut authority = serde_json::to_value(&epoch.signing_authority_binding).unwrap();
    authority["signer_key_epoch"] = serde_json::json!(2);
    epoch.signing_authority_binding = serde_json::from_value(authority).unwrap();
    requests.push(lineage::request(&epoch));

    let mut keys = BTreeSet::new();
    for request in requests {
        let mut port = OperationLookupStore::default();
        assert!(
            read_operation_start_for_request(&mut port, &store(), &request, 16 * 1024,)
                .unwrap()
                .is_none()
        );
        assert_eq!(port.reads.len(), 2);
        keys.insert(port.reads[0].key().clone());
    }
    assert_eq!(
        keys.len(),
        4,
        "caller, signer epoch, and request ID each distinguish a request"
    );
}

#[test]
fn admitted_lineage_prepares_and_fences_open_reconciliation_without_dispatch() {
    let fixture = lineage::OpenReconciliationFixture::new("open-positive");
    let effect = &fixture.effect;
    let checkpoints = admitted_checkpoint_pair();
    let verifier = |construction: &EffectCaseCheckpointConstruction<'_>| {
        assert_eq!(
            construction.kind(),
            EffectCaseTransitionKind::OpenReconciliation
        );
        assert_eq!(construction.members().len(), 4);
        assert_eq!(construction.mutations().len(), 4);
        assert_eq!(construction.assertions().len(), 1);
        Ok(())
    };
    let prepared = prepare_effect_case_transition(
        store(),
        EffectCaseTransitionInput::OpenReconciliation {
            request: AdmittedEffectCaseRecord::new(&effect.request_ref, &effect.request),
            action: AdmittedEffectCaseRecord::new(&effect.action_ref, &effect.action),
            intent: AdmittedEffectCaseRecord::new(&effect.intent_ref, &effect.intent),
            active_claim: AdmittedEffectCaseRecord::new(&effect.claim_ref, &effect.claim),
            transferred_claim: AdmittedEffectCaseRecord::new(
                &fixture.transferred_claim_ref,
                &fixture.transferred_claim,
            ),
            receipt: AdmittedEffectCaseRecord::new(&fixture.receipt_ref, &fixture.receipt),
            case: AdmittedEffectCaseRecord::new(&fixture.case_ref, &fixture.case),
            custody: AdmittedEffectCaseRecord::new(&fixture.custody_ref, &fixture.custody),
            prior_custody: None,
        },
        Some(AdmittedEffectCaseCheckpointTransition::new(
            &checkpoints.predecessor_ref,
            &checkpoints.predecessor,
            &checkpoints.successor_ref,
            &checkpoints.successor,
            &verifier,
        )),
    )
    .expect("typed reconciliation opening prepares");
    let mut store = TestStore::new(Ok(EffectCaseCommitOutcome::Committed));
    let pending = match prepared.commit(&mut store).expect("database commit") {
        EffectCaseCommitSuccess::AnchorPending(pending) => pending,
        EffectCaseCommitSuccess::Complete(_) => panic!("hard-domain transition requires anchor"),
    };
    let mut anchor = TestAnchor::new(Ok(AntiRollbackAnchorAdvanceDisposition::Advanced));
    let mut completed = pending.advance_anchor(&mut anchor).expect("anchor advance");
    assert_eq!(
        completed.candidate().kind(),
        EffectCaseTransitionKind::OpenReconciliation
    );
    assert!(completed.take_dispatch_authorization().is_none());
}

#[test]
fn typed_projection_rejects_request_action_and_claim_substitution() {
    let primary = lineage::EffectFixture::new("primary-negative");
    let other = lineage::EffectFixture::new("other-negative");
    let request_action = project_transition(EffectCaseTransitionInput::AdmitNew {
        request: AdmittedEffectCaseRecord::new(&primary.request_ref, &primary.request),
        action: AdmittedEffectCaseRecord::new(&other.action_ref, &other.action),
        claim: AdmittedEffectCaseRecord::new(&primary.claim_ref, &primary.claim),
        admission: AdmittedEffectCaseRecord::new(&primary.admission_ref, &primary.admission),
        prior_claim: None,
        prior_custody: None,
    });
    assert!(matches!(
        request_action,
        Err(EffectCaseProjectionError::Semantic(_))
    ));

    let claim = project_transition(EffectCaseTransitionInput::CommitDispatchIntent {
        request: AdmittedEffectCaseRecord::new(&primary.request_ref, &primary.request),
        action: AdmittedEffectCaseRecord::new(&primary.action_ref, &primary.action),
        admission: AdmittedEffectCaseRecord::new(&primary.admission_ref, &primary.admission),
        active_claim: AdmittedEffectCaseRecord::new(&other.claim_ref, &other.claim),
        intent: AdmittedEffectCaseRecord::new(&primary.intent_ref, &primary.intent),
    });
    assert!(matches!(claim, Err(EffectCaseProjectionError::Lineage)));
}

#[test]
fn typed_projection_rejects_case_and_custody_substitution() {
    let primary = lineage::OpenReconciliationFixture::new("open-primary");
    let other = lineage::OpenReconciliationFixture::new("open-other");
    let effect = &primary.effect;

    let changed_case = project_transition(EffectCaseTransitionInput::OpenReconciliation {
        request: AdmittedEffectCaseRecord::new(&effect.request_ref, &effect.request),
        action: AdmittedEffectCaseRecord::new(&effect.action_ref, &effect.action),
        intent: AdmittedEffectCaseRecord::new(&effect.intent_ref, &effect.intent),
        active_claim: AdmittedEffectCaseRecord::new(&effect.claim_ref, &effect.claim),
        transferred_claim: AdmittedEffectCaseRecord::new(
            &primary.transferred_claim_ref,
            &primary.transferred_claim,
        ),
        receipt: AdmittedEffectCaseRecord::new(&primary.receipt_ref, &primary.receipt),
        case: AdmittedEffectCaseRecord::new(&other.case_ref, &other.case),
        custody: AdmittedEffectCaseRecord::new(&primary.custody_ref, &primary.custody),
        prior_custody: None,
    });
    assert!(matches!(
        changed_case,
        Err(EffectCaseProjectionError::Lineage)
    ));

    let changed_custody = project_transition(EffectCaseTransitionInput::OpenReconciliation {
        request: AdmittedEffectCaseRecord::new(&effect.request_ref, &effect.request),
        action: AdmittedEffectCaseRecord::new(&effect.action_ref, &effect.action),
        intent: AdmittedEffectCaseRecord::new(&effect.intent_ref, &effect.intent),
        active_claim: AdmittedEffectCaseRecord::new(&effect.claim_ref, &effect.claim),
        transferred_claim: AdmittedEffectCaseRecord::new(
            &primary.transferred_claim_ref,
            &primary.transferred_claim,
        ),
        receipt: AdmittedEffectCaseRecord::new(&primary.receipt_ref, &primary.receipt),
        case: AdmittedEffectCaseRecord::new(&primary.case_ref, &primary.case),
        custody: AdmittedEffectCaseRecord::new(&other.custody_ref, &other.custody),
        prior_custody: None,
    });
    assert!(matches!(
        changed_custody,
        Err(EffectCaseProjectionError::Lineage)
    ));
}
