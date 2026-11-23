//! Saved-locator recovery over the shared admitted-start fixture.
//! Custody and membership are controlled doubles, not native backend proof.

use super::*;
use crate::resident::LocalOperationHandleRecoveryInput;

pub(super) const CASES: &[&str] = &[
    "none",
    "after-observation",
    "zero-bound",
    "handle-missing",
    "handle-wrong-coordinate",
    "handle-malformed",
    "handle-body-substitution",
    "handle-body-bound",
    "request-for-handle-missing",
    "request-for-handle-wrong-query",
    "request-schema-digest",
    "request-alias",
    "request-body-bound",
    "request-callback-substitution",
    "historical-caller-rejection",
    "anchor-after",
    "head-before-proof",
];

pub(super) struct Lookup {
    pub(super) expected: ControlRecordCoordinate,
    pub(super) key: OperationCustodyIndexKey,
    pub(super) handle: Option<ControlRecord>,
    pub(super) request: Option<ControlRecord>,
    pub(super) maximum: usize,
    pub(super) wrong_query: bool,
    pub(super) handle_reads: usize,
    pub(super) index_reads: usize,
}

pub(super) struct Case<'a> {
    pub(super) fault: &'a str,
    pub(super) request: &'a CommonRequestEnvelopeV1,
    pub(super) substitute: &'a CommonRequestEnvelopeV1,
    pub(super) expected: &'a (ResidentRecordRefV1, OperationHandleV1),
    pub(super) retained: &'a RecoveryParents<'a>,
    pub(super) profile: &'a crate::resident::SupportedOperationCheckpointProfile,
    pub(super) authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(super) resolver: &'a RecoveryResolver,
}

pub(super) fn check(
    case: Case<'_>,
    store: &mut RecoveryStore,
    sessions: &SessionRead,
    anchor: &mut ReadAnchor,
) {
    let fault = case.fault;
    let handle = store.handle.as_ref().unwrap().clone();
    let request = store.request.as_ref().unwrap().clone();
    let mut lookup = Lookup {
        expected: handle.coordinate().clone(),
        key: crate::resident::operation_observation_custody::operation_handle_key(
            case.expected.1.operation_handle_id(),
        )
        .unwrap(),
        handle: Some(handle.clone()),
        request: Some(request.clone()),
        maximum: 1024 * 1024,
        wrong_query: fault == "request-for-handle-wrong-query",
        handle_reads: 0,
        index_reads: 0,
    };
    match fault {
        "zero-bound" => lookup.maximum = 0,
        "handle-missing" => lookup.handle = None,
        "handle-wrong-coordinate" => {
            lookup.expected = ControlRecordCoordinate::new(
                record_ref("absent-handle"),
                handle.coordinate().record_sha256().clone(),
            );
        }
        "handle-malformed" => lookup.handle = Some(without_signature(&handle)),
        "handle-body-substitution" => {
            let mut value =
                pale_core::canon::parse_strict(handle.canonical_record_jcs().as_slice()).unwrap();
            value["created_at"] = serde_json::json!(case.expected.1.created_at().get() + 1);
            lookup.handle = Some(with_bytes(
                &handle,
                pale_core::canon::to_canonical_string(&value)
                    .unwrap()
                    .into_bytes(),
            ));
        }
        "handle-body-bound" => lookup.maximum = handle.canonical_record_jcs().as_slice().len() - 1,
        "request-for-handle-missing" => lookup.request = None,
        "request-schema-digest" => {
            lookup.request = Some(ControlRecord::new(
                request.coordinate().clone(),
                request.schema_id().clone(),
                SchemaDigest::parse(sha('f')).unwrap(),
                request.canonical_record_jcs().clone(),
            ));
        }
        "request-alias" => {
            lookup.request = Some(ControlRecord::new(
                ControlRecordCoordinate::new(
                    record_ref("same-request-at-another-reference"),
                    request.coordinate().record_sha256().clone(),
                ),
                request.schema_id().clone(),
                request.schema_sha256().clone(),
                request.canonical_record_jcs().clone(),
            ));
        }
        "request-body-bound" => {
            lookup.maximum = handle
                .canonical_record_jcs()
                .as_slice()
                .len()
                .max(request.canonical_record_jcs().as_slice().len());
            let mut bytes = request.canonical_record_jcs().as_slice().to_vec();
            bytes.resize(lookup.maximum + 1, b' ');
            lookup.request = Some(with_bytes(&request, bytes));
        }
        _ => {}
    }
    // The doubles deliberately return oversized/misdirected rows in failure
    // cases. The application must defend its boundary even against such a port.
    let expected_locator = lookup.expected.clone();
    let maximum = lookup.maximum;
    let selected_request_ref = lookup
        .request
        .as_ref()
        .map(|value| value.coordinate().record_ref().clone());
    store.handle_lookup = Some(lookup);
    let request_calls = Cell::new(0);
    let caller_signature_calls = Cell::new(0);
    let handle_authority_calls = Cell::new(0);
    let handle_signature_calls = Cell::new(0);
    let checkpoint_signature_calls = Cell::new(0);
    let historical = case.request.to_preimage();
    let context = CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &historical.envelope_schema_sha256,
        expected_body_schema_id: &historical.body.schema_id,
        expected_body_schema_sha256: &historical.body.schema_sha256,
        caller: &historical.caller,
        signing_authority: &historical.signing_authority_binding,
        route: &historical.route,
        session_admission: &historical.session_admission,
        operation_target: historical.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: None,
        workspace_id: &historical.workspace_id,
        capability_id: &historical.capability_id,
        operation: &historical.operation,
        scope: &historical.scope,
        risk_class: &historical.risk_class,
        resource_constraints: &historical.resource_constraints,
        expected_precondition: None,
        idempotency_key: &historical.idempotency_key,
        issued_at: historical.issued_at,
        expires_at: historical.expires_at,
        generation: historical.generation.as_ref(),
        fence_token: historical.fence_token.as_ref(),
    };
    let result = recover_local_operation_from_handle(
        LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store: &store_coordinate(),
                profile: case.profile,
                handle_signing_authority: case.retained.signing,
                checkpoint_authority: case.authority,
                historical: case.resolver,
                maximum_record_bytes: maximum,
                maximum_operation_lease_bytes: 1024 * 1024,
                maximum_session_lease_bytes: 1024 * 1024,
                membership_limits: AccumulatorMembershipLimits::new(3, 10_000, 100_000).unwrap(),
                observation_limits: OperationObservationRecoveryLimits::new(
                    4,
                    1024 * 1024,
                    1024 * 1024,
                )
                .unwrap(),
                checkpoint_limits: OperationCheckpointAncestryLimits::new(4, 1024 * 1024, 4)
                    .unwrap(),
            },
            expected_handle: &expected_locator,
            maximum_parent_depth: 0,
        },
        store,
        sessions,
        anchor,
        |reference, wire| {
            request_calls.set(request_calls.get() + 1);
            assert_eq!(Some(reference), selected_request_ref.as_ref());
            if fault == "request-callback-substitution" {
                // Deliberately violate the callback's association contract;
                // the wrapper must reject even another well-formed request.
                return Ok(case.substitute.clone());
            }
            CommonRequestEnvelopeV1::admit_contextual_wire(
                wire,
                &context,
                |preimage, digest, actual| {
                    caller_signature_calls.set(caller_signature_calls.get() + 1);
                    assert_eq!(preimage, &historical);
                    assert_eq!(digest, case.request.canonical_request_sha256());
                    assert_eq!(actual, case.request.caller_signature());
                    if fault == "historical-caller-rejection" {
                        Err(rejected())
                    } else {
                        Ok(())
                    }
                },
            )
        },
        LocalOperationRecoveryAdmission {
            admit_historical_authority:
                |_: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
                    handle_authority_calls.set(handle_authority_calls.get() + 1);
                    assert!(parent.is_none());
                    Ok(())
                },
            verify_handle_signature:
                |domain: &[u8],
                 _: &OperationHandlePreimageV1,
                 digest: &OperationHandleDigest,
                 actual: &CanonicalSignatureV1| {
                    handle_signature_calls.set(handle_signature_calls.get() + 1);
                    assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
                    assert_eq!(digest, case.expected.1.canonical_operation_handle_sha256());
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            verify_checkpoint_signature:
                |domain: &[u8],
                 _: &ControlStateCheckpointPreimageV1,
                 _: &ControlStateCheckpointDigest,
                 actual: &CanonicalSignatureV1| {
                    checkpoint_signature_calls.set(checkpoint_signature_calls.get() + 1);
                    assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            initial_observation: OperationObservationRecoveryAdmission {
                admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                    ControlStateCheckpointV1::admit_signed_wire(
                        wire,
                        &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                        case.authority,
                        |domain, _, _, actual| {
                            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                            assert_eq!(actual, &signature());
                            Ok(())
                        },
                    )
                },
                resolve_signer: |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                    Ok(case.retained.signing.clone())
                },
                admit_state: |_: &OperationObservationStateV1| Ok(()),
                admit_rotation: |_: &ResidentRecordRefV1,
                                 _: &ArtifactDigest,
                                 _: &ResidentRecordRefV1,
                                 _: &ArtifactDigest| {
                    panic!("saved-locator fixture does not rotate runtime authority")
                },
                verify_signature:
                    |domain: &[u8],
                     _: &OperationObservationPreimageV1,
                     _: &OperationObservationDigest,
                     actual: &CanonicalSignatureV1| {
                        assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                        assert_eq!(actual, &signature());
                        Ok(())
                    },
            },
        },
    );
    let lookup = store.handle_lookup.as_ref().unwrap();
    let read_counts = || {
        [
            lookup.handle_reads,
            lookup.index_reads,
            request_calls.get(),
            caller_signature_calls.get(),
            store.index_reads,
            store.lease_reads,
            sessions.1.load(Ordering::Relaxed),
            handle_authority_calls.get(),
            handle_signature_calls.get(),
            checkpoint_signature_calls.get(),
            store.head_reads,
            anchor.reads,
            store.membership_calls,
        ]
    };
    let calls = read_counts();
    let full = [1, 1, 1, 1, 8, 1, 1, 1, 1, 2, 2, 4, 2];
    if ["none", "after-observation"].contains(&fault) {
        let recovered = result.unwrap_or_else(|error| panic!("{fault}: {error:?}"));
        assert_eq!(
            recovered.operation().handle(),
            (&case.expected.0, &case.expected.1)
        );
        assert_eq!(recovered.request(), case.request);
        let recovered_start = recovered
            .initial_observation()
            .expect("protected handle recovery retains admitted revision one");
        assert_eq!(recovered_start.selected().1.revision().get(), 1);
        assert_eq!(
            recovered_start.selected().1.operation_handle_id(),
            case.expected.1.operation_handle_id()
        );
        assert!(recovered.recovered_parent().is_none());
        assert_eq!(
            recovered.operation().historical_operation_lease(),
            case.retained.lease
        );
        assert_eq!(
            recovered.operation().historical_session_lease(),
            case.retained.session.lease()
        );
        recovered
            .operation()
            .historical_operation_lease()
            .verify_against_parent(recovered.operation().historical_session_lease())
            .unwrap();
        assert_eq!(calls, full, "{fault}: bounded read and admission calls");
        assert_eq!(
            read_counts(),
            full,
            "borrowed historical leases perform no additional reads or admission"
        );
        // Recovery has no clock or renewal input. Preserve the original times;
        // this is not proof of current permission to inspect an expired operation.
        assert_eq!(historical.expires_at, case.request.expires_at());
        assert_eq!(
            recovered.operation().handle().1.expires_at(),
            case.expected.1.expires_at()
        );
        return;
    }
    let Err(error) = result else {
        panic!("{fault}: exposed an operation")
    };
    let expected_calls = match (fault, &error) {
        ("zero-bound", LocalOperationRecoveryError::Bounds) => [0; 13],
        ("handle-missing", LocalOperationRecoveryError::Missing)
        | ("handle-wrong-coordinate", LocalOperationRecoveryError::Association)
        | ("handle-body-bound", LocalOperationRecoveryError::Bounds) => {
            [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        }
        ("handle-malformed", LocalOperationRecoveryError::Decode(_)) => {
            [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        }
        ("request-for-handle-missing", LocalOperationRecoveryError::Missing)
        | ("request-schema-digest", LocalOperationRecoveryError::Association)
        | (
            "request-for-handle-wrong-query",
            LocalOperationRecoveryError::Lookup(
                crate::resident::OperationRecoveryError::MisdirectedObservation,
            ),
        )
        | (
            "request-body-bound",
            LocalOperationRecoveryError::Lookup(
                crate::resident::OperationRecoveryError::RecordBoundExceeded,
            ),
        ) => [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ("request-callback-substitution", LocalOperationRecoveryError::Association) => {
            [1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        }
        (
            "historical-caller-rejection",
            LocalOperationRecoveryError::Record(InvalidResidentRecordV1::ContextualAdmission {
                record: "recovery test",
                invariant: "resolved historical authority or verifier rejected",
            }),
        ) => [1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        (
            "request-alias" | "handle-body-substitution",
            LocalOperationRecoveryError::Association,
        ) => full,
        ("anchor-after", LocalOperationRecoveryError::Association) => {
            [1, 1, 1, 1, 4, 1, 1, 1, 1, 1, 1, 2, 1]
        }
        (
            "head-before-proof",
            LocalOperationRecoveryError::Membership(OperationAccumulatorPreparationError::Store(
                value,
            )),
        ) if value.kind() == ControlStoreErrorKind::CandidateConflict => {
            [1, 1, 1, 1, 4, 1, 1, 1, 1, 1, 1, 1, 1]
        }
        _ => panic!("{fault}: wrong refusal {error:?}"),
    };
    assert_eq!(calls, expected_calls, "{fault}: rejection stage");
}

fn with_bytes(record: &ControlRecord, bytes: Vec<u8>) -> ControlRecord {
    ControlRecord::new(
        record.coordinate().clone(),
        record.schema_id().clone(),
        record.schema_sha256().clone(),
        ControlRecordBytes::new(bytes).unwrap(),
    )
}
