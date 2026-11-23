//! Behavioral laws for current operation observation and cancellation admission.

use super::*;
use crate::effect_case_lineage_fixture as request_fixture;
use crate::local_operation_start_fixture::{
    issue_observation, record_ref, schema_digest, source_generation_inputs_with, store_coordinate,
    with_start, StartAdmission,
};
use pale_ports::AuthorityStoreCoordinate;

impl AuthorizedOperationCancellation<'_> {
    pub(in crate::resident) fn fixture<'a>(
        request: &'a CommonRequestEnvelopeV1,
        recovered_operation: &'a RecoveredLocalOperation,
        recovered_observation: &'a RecoveredOperationObservation,
        authorized_at: UnixTimeV1,
    ) -> AuthorizedOperationCancellation<'a> {
        let operation = recovered_operation.operation().handle();
        let observation = recovered_observation.selected_head();
        AuthorizedOperationCancellation {
            request,
            recovered_operation,
            recovered_observation,
            operation,
            observation,
            authorized_at,
        }
    }
}

struct Facts {
    request: CommonRequestEnvelopeV1,
    original_request: CommonRequestEnvelopeV1,
    operation_ref: ResidentRecordRefV1,
    operation: OperationHandleV1,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    signing_authority: SigningAuthorityBindingV1,
    store: AuthorityStoreCoordinate,
}

fn source_generation() -> (
    crate::resident::AdmittedCompiledGeneration,
    EffectiveProjectProfileV1,
) {
    let (compilation, materializations, profile) = source_generation_inputs_with(|_| {}, |_| {});
    let generation = crate::resident::AdmittedCompiledGeneration::admit(
        compilation,
        materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .expect("actual resident generation");
    (generation, profile)
}

fn control_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
    capability: &str,
    issued_at: u64,
    expires_at: u64,
    change: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
) -> CommonRequestEnvelopeV1 {
    let mut preimage = original.to_preimage();
    preimage.request_id = ResidentRequestIdV1::parse(format!("control-{capability}-{issued_at}"))
        .expect("request id");
    preimage.correlation_id =
        ResidentCorrelationIdV1::parse(format!("control-{capability}-{issued_at}"))
            .expect("correlation id");
    preimage.capability_id = CapabilityIdV1::parse(capability).expect("control capability");
    preimage.idempotency_key =
        RequestIdempotencyKeyV1::parse(format!("control-{capability}-{issued_at}"))
            .expect("idempotency key");
    preimage.issued_at = UnixTimeV1::new(issued_at).expect("issue time");
    preimage.expires_at = UnixTimeV1::new(expires_at).expect("expiry time");
    preimage.generation = Some(handle.compiled_generation_sha256().clone());
    preimage.fence_token = Some(handle.activation_fence_sha256().clone());
    preimage.body.schema_id =
        SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).expect("operation handle schema");
    preimage.body.schema_sha256 = handle.schema_sha256().clone();
    preimage.body.payload = serde_json::to_value(handle).expect("operation handle payload");
    preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&preimage.body.payload)
            .expect("canonical handle payload")
            .as_bytes(),
    );
    change(&mut preimage);
    request_fixture::request(&preimage)
}

fn with_facts(run: impl FnOnce(Facts)) {
    let (generation, profile) = source_generation();
    with_start(
        &generation,
        &profile,
        "operation-control",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, original, _, _, retained| {
            let prepared = prepared.expect("admitted start");
            let (operation_ref, operation) = prepared.prepared_handle();
            let observation_ref = record_ref("operation/control-observation");
            let observation = issue_observation(
                retained.signing,
                operation_ref,
                operation,
                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                OperationObservationRevisionV1::new(1).expect("revision"),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                operation.created_at().get() + 1,
            );
            let request = control_request(
                original,
                operation,
                OPERATION_OBSERVE_CAPABILITY_ID,
                operation.created_at().get() + 2,
                operation.retain_until().get(),
                |_| {},
            );
            run(Facts {
                request,
                original_request: original.clone(),
                operation_ref: operation_ref.clone(),
                operation: operation.clone(),
                observation_ref,
                observation,
                signing_authority: retained.signing.clone(),
                store: store_coordinate(),
            });
        },
    );
}

fn input<'a>(
    facts: &'a Facts,
    request: &'a CommonRequestEnvelopeV1,
    observation_store: &'a AuthorityStoreCoordinate,
    operation_ref: &'a ResidentRecordRefV1,
    selected: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    head: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    authorized_at: u64,
) -> OperationControlFactsInput<'a> {
    OperationControlFactsInput {
        request,
        original_request: &facts.original_request,
        operation_store: &facts.store,
        observation_store,
        operation: (operation_ref, &facts.operation),
        observation: selected,
        observation_head: head,
        authorized_at: UnixTimeV1::new(authorized_at).expect("authorization time"),
    }
}

#[test]
fn exact_control_facts_admit_only_one_current_head_and_time_interval() {
    with_facts(|facts| {
        let selected = (&facts.observation_ref, &facts.observation);
        let authorized_at = facts.request.issued_at().get() + 1;
        let admitted = validate_control_facts(
            input(
                &facts,
                &facts.request,
                &facts.store,
                &facts.operation_ref,
                selected,
                selected,
                authorized_at,
            ),
            OPERATION_OBSERVE_CAPABILITY_ID,
        )
        .expect("exact current observation");
        assert_eq!(admitted.request, &facts.request);
        assert_eq!(admitted.operation, (&facts.operation_ref, &facts.operation));
        assert_eq!(admitted.observation, selected);

        let wrong_capability = control_request(
            &facts.original_request,
            &facts.operation,
            OPERATION_CANCEL_CAPABILITY_ID,
            facts.request.issued_at().get(),
            facts.request.expires_at().get(),
            |_| {},
        );
        assert!(matches!(
            validate_control_facts(
                input(
                    &facts,
                    &wrong_capability,
                    &facts.store,
                    &facts.operation_ref,
                    selected,
                    selected,
                    authorized_at,
                ),
                OPERATION_OBSERVE_CAPABILITY_ID,
            ),
            Err(OperationControlError::Association)
        ));

        assert!(matches!(
            validate_control_facts(
                input(
                    &facts,
                    &facts.original_request,
                    &facts.store,
                    &facts.operation_ref,
                    selected,
                    selected,
                    authorized_at,
                ),
                OPERATION_OBSERVE_CAPABILITY_ID,
            ),
            Err(OperationControlError::Admission(_))
        ));

        let other_store = AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("other-store").expect("store"),
            ControlIncarnationIdV1::parse("other-incarnation").expect("incarnation"),
        );
        let historical_ref = record_ref("operation/historical-observation");
        for changed in [
            input(
                &facts,
                &facts.request,
                &other_store,
                &facts.operation_ref,
                selected,
                selected,
                authorized_at,
            ),
            input(
                &facts,
                &facts.request,
                &facts.store,
                &facts.operation_ref,
                (&historical_ref, &facts.observation),
                selected,
                authorized_at,
            ),
            input(
                &facts,
                &facts.request,
                &facts.store,
                &record_ref("operation/other-handle"),
                selected,
                selected,
                authorized_at,
            ),
        ] {
            assert!(matches!(
                validate_control_facts(changed, OPERATION_OBSERVE_CAPABILITY_ID),
                Err(OperationControlError::Association)
            ));
        }

        for time in [
            facts.operation.created_at().get(),
            facts.observation.observed_at().get() - 1,
            facts.request.expires_at().get() + 1,
            facts.operation.retain_until().get() + 1,
        ] {
            assert!(matches!(
                validate_control_facts(
                    input(
                        &facts,
                        &facts.request,
                        &facts.store,
                        &facts.operation_ref,
                        selected,
                        selected,
                        time,
                    ),
                    OPERATION_OBSERVE_CAPABILITY_ID,
                ),
                Err(OperationControlError::Time)
            ));
        }
    });
}

#[test]
fn authorized_observation_projects_only_owned_public_records() {
    with_facts(|facts| {
        let authorized = AuthorizedOperationObservation {
            request: &facts.request,
            operation: (&facts.operation_ref, &facts.operation),
            observation: (&facts.observation_ref, &facts.observation),
        };
        let projected = authorized.to_owned_observation();

        assert_eq!(
            projected.operation(),
            (&facts.operation_ref, &facts.operation)
        );
        assert_eq!(
            projected.observation(),
            (&facts.observation_ref, &facts.observation)
        );
        assert_eq!(
            projected.observation().1.operation_handle_ref(),
            projected.operation().0
        );
        assert_eq!(
            projected.observation().1.operation_handle_sha256(),
            projected.operation().1.canonical_operation_handle_sha256()
        );
    });
}

fn evidence(value: &str, byte: char) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record_ref(value),
        record_sha256: ArtifactDigest::parse(request_fixture::sha(byte)).expect("digest"),
    }
}

#[test]
fn cancellation_disposition_preserves_state_and_never_claims_a_remote_outcome() {
    with_facts(|facts| {
        let attempt_id = CapabilityAttemptIdV1::parse("attempt-1").expect("attempt");
        let dispatch = evidence("dispatch-evidence", 'a');
        let collection = evidence("collection-evidence", 'b');
        let cancellation = evidence("cancellation-request", 'c');
        let cases = [
            (
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                "before-dispatch",
            ),
            (
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched {
                        attempt_id: attempt_id.clone(),
                        evidence: dispatch.clone(),
                    },
                },
                "dispatched",
            ),
            (
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::CollectingConsultation {
                        evidence: collection.clone(),
                    },
                },
                "consultation",
            ),
            (
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ConsultationCollected {
                        evidence: collection.clone(),
                    },
                },
                "consultation-collected",
            ),
            (
                OperationObservationStateV1::CancellationRequested {
                    request: cancellation.clone(),
                },
                "already-requested",
            ),
            (
                OperationObservationStateV1::Expired {
                    diagnostic: evidence("expired", 'd'),
                },
                "reconciliation",
            ),
        ];
        for (index, (state, expected)) in cases.into_iter().enumerate() {
            let reference = record_ref(&format!("operation/cancel-state-{index}"));
            let observation = issue_observation(
                &facts.signing_authority,
                &facts.operation_ref,
                &facts.operation,
                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                OperationObservationRevisionV1::new(1).expect("revision"),
                None,
                state,
                facts.operation.created_at().get() + 1,
            );
            let actual = match cancellation_disposition(
                &facts.operation,
                (&reference, &observation),
                UnixTimeV1::new(facts.operation.created_at().get() + 3)
                    .expect("authorization time"),
            ) {
                OperationCancellationDisposition::BeforeDispatch => "before-dispatch",
                OperationCancellationDisposition::Dispatched {
                    attempt_id: actual_attempt,
                    evidence,
                } => {
                    assert_eq!(actual_attempt, &attempt_id);
                    assert_eq!(evidence, &dispatch);
                    "dispatched"
                }
                OperationCancellationDisposition::CollectingConsultation { evidence } => {
                    assert_eq!(evidence, &collection);
                    "consultation"
                }
                OperationCancellationDisposition::ConsultationCollected { evidence } => {
                    assert_eq!(evidence, &collection);
                    "consultation-collected"
                }
                OperationCancellationDisposition::AlreadyRequested { request } => {
                    assert_eq!(request, &cancellation);
                    "already-requested"
                }
                OperationCancellationDisposition::ReconciliationOnly(_) => "reconciliation",
                OperationCancellationDisposition::Established(_) => "established",
            };
            assert_eq!(actual, expected);
        }

        assert!(matches!(
            cancellation_disposition(
                &facts.operation,
                (&facts.observation_ref, &facts.observation),
                UnixTimeV1::new(facts.operation.expires_at().get() + 1)
                    .expect("late authorization"),
            ),
            OperationCancellationDisposition::ReconciliationOnly(_)
        ));
    });
}
