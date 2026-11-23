//! Restart proof for the private reconciliation-to-operation association.
//!
//! Semantic records use the application test fixture. The production `SQLite`
//! adapters own physical custody and point reads. Signatures and authorities
//! remain declared fixture models; this test proves storage continuity only.

use super::*;
use crate::SqliteEffectCaseCustody;
use pale_application::resident::{
    read_reconciliation_operation_association, recover_reconciliation_effect_attempt,
    AdmittedEffectCaseRecord, ReconciliationEffectAttemptInput,
    ReconciliationOperationAssociationError, ReconciliationOperationAssociationInput,
};
use pale_core::canon;
use pale_core::records::CommonRequestEnvelopeV1;
use pale_ports::{ControlRecordError, OperationCustodyRecordKind};
use serde::Serialize;

#[path = "../../../../../crates/pale-application/tests/support/effect_case_lineage.rs"]
#[allow(dead_code, clippy::duplicate_mod)]
mod lineage;

#[derive(Serialize)]
struct RequestKeyPreimage<'a> {
    initial_v1_domain: &'static str,
    caller_identity: &'a str,
    signer_key_epoch: u64,
    request_id: &'a str,
}

fn request_key(request: &CommonRequestEnvelopeV1) -> OperationCustodyIndexKey {
    let preimage = RequestKeyPreimage {
        initial_v1_domain: "request_identity",
        caller_identity: request.caller().identity.as_str(),
        signer_key_epoch: request.caller().signer_key_epoch.get(),
        request_id: request.request_id().as_str(),
    };
    let canonical = canon::to_canonical_string(&preimage).expect("canonical request key");
    let mut bytes = b"PALE_OPERATION_REQUEST_IDENTITY_INDEX_KEY_V1\0".to_vec();
    bytes.extend_from_slice(canonical.as_bytes());
    OperationCustodyIndexKey::parse(pale_core::sha256_hex(&bytes)).expect("request key")
}

fn semantic_record<T: Serialize>(
    record_ref: &ResidentRecordRefV1,
    record_digest: &str,
    schema_id: &SchemaId,
    schema_sha256: &SchemaDigest,
    value: &T,
) -> Result<ControlRecord, ControlRecordError> {
    let bytes = canon::to_canonical_string(value)
        .expect("canonical semantic record")
        .into_bytes();
    Ok(ControlRecord::new(
        ControlRecordCoordinate::new(
            record_ref.clone(),
            ControlRecordDigest::parse(record_digest).expect("record digest"),
        ),
        schema_id.clone(),
        schema_sha256.clone(),
        ControlRecordBytes::new(bytes).expect("record bytes"),
    ))
}

fn request_record(
    record_ref: &ResidentRecordRefV1,
    request: &CommonRequestEnvelopeV1,
) -> ControlRecord {
    semantic_record(
        record_ref,
        request.canonical_request_sha256().as_str(),
        request.envelope_schema_id(),
        request.envelope_schema_sha256(),
        request,
    )
    .expect("request record")
}

fn start_for_request(
    typed_request: &CommonRequestEnvelopeV1,
    request: ControlRecord,
) -> OperationCustodyCandidate {
    let handle = record(
        "reconciliation-operation-handle",
        '2',
        OPERATION_HANDLE_V1_SCHEMA,
    );
    let observation = record(
        "reconciliation-operation-observation-1",
        '3',
        OPERATION_OBSERVATION_V1_SCHEMA,
    );
    let checkpoint = checkpoint_transition(
        8,
        "checkpoint-8",
        '8',
        9,
        "checkpoint-9-reconciliation",
        '9',
    );
    let basis = pale_ports::OperationSessionBasis::new(
        handle.coordinate().clone(),
        request.coordinate().clone(),
        pale_core::records::SessionAdmissionLeaseDigest::parse(digest('a')).unwrap(),
        br#"{"physical_fixture":"reconciliation-association"}"#.to_vec(),
        4096,
    )
    .unwrap();
    let request_key = request_key(typed_request);
    OperationCustodyCandidate::new_start(
        store(),
        OperationCustodyTransitionKind::StartRoot,
        vec![
            OperationCustodyRecord::CommonRequest(request.clone()),
            OperationCustodyRecord::OperationHandle(handle.clone()),
            OperationCustodyRecord::OperationObservation(observation.clone()),
        ],
        vec![
            OperationCustodyIndexMutation::RequestEvidence {
                key: request_key.clone(),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: request.coordinate().clone(),
            },
            OperationCustodyIndexMutation::RequestOperation {
                key: request_key,
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: handle.coordinate().clone(),
            },
            OperationCustodyIndexMutation::OperationHandle {
                key: key('2'),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: handle.coordinate().clone(),
            },
            OperationCustodyIndexMutation::LatestObservation {
                key: key('2'),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: observation.coordinate().clone(),
            },
        ],
        checkpoint.clone(),
        physical_accumulator(store(), &checkpoint),
        basis,
    )
    .expect("operation start candidate")
}

fn effect_records(fixture: &lineage::ReconciliationResultFixture) -> Vec<ControlRecord> {
    let effect = &fixture.open.effect;
    vec![
        semantic_record(
            &effect.intent_ref,
            effect.intent.canonical_dispatch_intent_sha256().as_str(),
            effect.intent.schema_id(),
            effect.intent.schema_sha256(),
            &effect.intent,
        )
        .unwrap(),
        semantic_record(
            &fixture.open.receipt_ref,
            fixture.open.receipt.canonical_receipt_sha256().as_str(),
            fixture.open.receipt.schema_id(),
            fixture.open.receipt.schema_sha256(),
            &fixture.open.receipt,
        )
        .unwrap(),
        semantic_record(
            &fixture.result_ref,
            fixture.result.canonical_reconciliation_sha256().as_str(),
            fixture.result.schema_id(),
            fixture.result.schema_sha256(),
            &fixture.result,
        )
        .unwrap(),
    ]
}

#[test]
fn late_reconciliation_result_rejoins_the_same_attempt_after_sqlite_restart() {
    let fixture = lineage::ReconciliationResultFixture::new("sqlite-restart");
    let directory = tempfile::tempdir().expect("temporary store");
    let path = directory.path().join("reconciliation-operation.sqlite3");
    let start = start_for_request(
        &fixture.open.effect.request,
        request_record(
            &fixture.open.effect.request_ref,
            &fixture.open.effect.request,
        ),
    );
    let later = observation_candidate(
        start
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
        "reconciliation",
        '4',
    );
    {
        let mut connection = Connection::open(&path).expect("initial store");
        initialize_store(&connection);
        let mut operation = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        operation.compare_and_commit(&start).unwrap();
        operation.compare_and_commit(&later).unwrap();
        for record in effect_records(&fixture) {
            insert_record(&connection, &record);
        }
    }

    let mut reopened = Connection::open(&path).expect("reopened store");
    let store_coordinate = store();
    let other = lineage::ReconciliationResultFixture::new("sqlite-wrong-attempt");
    let wrong_attempt = ReconciliationEffectAttemptInput {
        store: &store_coordinate,
        effect_request: AdmittedEffectCaseRecord::new(
            &fixture.open.effect.request_ref,
            &fixture.open.effect.request,
        ),
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
    assert!(matches!(
        recover_reconciliation_effect_attempt(
            wrong_attempt,
            &mut SqliteEffectCaseCustody {
                connection: &mut reopened,
                coordinate: store_coordinate.clone(),
            },
        ),
        Err(ReconciliationOperationAssociationError::EffectLineage)
    ));
    let effect_input = ReconciliationEffectAttemptInput {
        store: &store_coordinate,
        effect_request: AdmittedEffectCaseRecord::new(
            &fixture.open.effect.request_ref,
            &fixture.open.effect.request,
        ),
        dispatch_intent: AdmittedEffectCaseRecord::new(
            &fixture.open.effect.intent_ref,
            &fixture.open.effect.intent,
        ),
        effect_receipt: AdmittedEffectCaseRecord::new(
            &fixture.open.receipt_ref,
            &fixture.open.receipt,
        ),
        reconciliation_result: AdmittedEffectCaseRecord::new(&fixture.result_ref, &fixture.result),
    };
    let recovered_effect = recover_reconciliation_effect_attempt(
        effect_input,
        &mut SqliteEffectCaseCustody {
            connection: &mut reopened,
            coordinate: store_coordinate.clone(),
        },
    )
    .expect("effect records survive restart");
    let expected_handle = start
        .record(OperationCustodyRecordKind::OperationHandle)
        .unwrap()
        .coordinate()
        .clone();
    let changes = reopened.total_changes();
    let mut operation = SqliteOperationCustody {
        connection: &mut reopened,
        coordinate: store_coordinate.clone(),
    };
    assert!(matches!(
        read_reconciliation_operation_association(
            ReconciliationOperationAssociationInput {
                effect_attempt: recovered_effect,
                expected_operation_handle: &coordinate("foreign-operation-handle", 'f'),
                maximum_record_bytes: 1 << 20,
            },
            &mut operation,
        ),
        Err(ReconciliationOperationAssociationError::OperationMismatch)
    ));
    let first = read_reconciliation_operation_association(
        ReconciliationOperationAssociationInput {
            effect_attempt: recovered_effect,
            expected_operation_handle: &expected_handle,
            maximum_record_bytes: 1 << 20,
        },
        &mut operation,
    )
    .expect("late result rejoins original operation attempt");
    let replay = read_reconciliation_operation_association(
        ReconciliationOperationAssociationInput {
            effect_attempt: recovered_effect,
            expected_operation_handle: &expected_handle,
            maximum_record_bytes: 1 << 20,
        },
        &mut operation,
    )
    .expect("replay is the same read-only association");
    assert_eq!(first, replay);
    assert_eq!(first.operation_handle(), &expected_handle);
    assert_eq!(
        first.effect_attempt_id(),
        fixture.open.effect.intent.attempt_id()
    );
    assert_eq!(
        operation.connection.total_changes(),
        changes,
        "association recovery performs no write"
    );
}
