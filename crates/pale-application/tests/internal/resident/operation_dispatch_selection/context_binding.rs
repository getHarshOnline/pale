//! Context binding uses the existing observation and custody owners.
//! Test signing, evidence admission, and accumulator callbacks remain models.

use super::*;
use crate::local_operation_start_fixture::{
    artifact_digest, issue_observation, signature, store_coordinate,
};
use crate::resident::operation_dispatch::provider::{
    prepare_provider_resolution, ProviderOperationInput,
};
use crate::resident::{
    prepare_operation_context_binding, A2aSendDispatch, OperationAttemptHistoryInput,
    OperationContextBindingInput, OperationObservationAuthorityStoreRejection,
    OperationObservationAuthorityStoreResolver, RecoveredOperationObservation,
};
use crate::session_admission_fixture::{LocalStartContext, ProvingAuthority};
use pale_ports::AuthorityStoreCoordinate;
use std::cell::Cell;

struct Store;

impl OperationObservationAuthorityStoreResolver for Store {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(store_coordinate())
    }
}

#[test]
fn admitted_context_must_enter_custody_before_provider_resolution() {
    with_selection_custody_context(
        LocalStartContext::EvidenceOnly,
        |selection, selected, admission, parents, initial_checkpoint, initial_record| {
            let (handle_ref, handle) = selected.operation.handle();
            let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            assert_eq!(
                initial_record.canonical_record_jcs().as_slice(),
                canon::to_canonical_string(&initial).unwrap().as_bytes()
            );
            let initial_admission = OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema,
                    signing_authority: parents.signing,
                },
                handle_ref,
                handle,
                predecessor: Some((initial_record.coordinate().record_ref(), &initial)),
            };
            let expected_evidence = ImmutableRecordReferenceV1 {
                record_ref: record_ref("context/projection"),
                record_sha256: artifact_digest('8'),
            };
            let admissions = Cell::new(0);
            let signs = Cell::new(0);
            let prepared = prepare_operation_context_binding(
                OperationContextBindingInput {
                    operation: selected.operation,
                    request: selected.request,
                    session: parents.session,
                    current: parents.current,
                    lease: selected.lease,
                    evidence: expected_evidence.clone(),
                    observation_ref: record_ref("context/observation"),
                    observation: &initial_admission,
                },
                &mut ProvingAuthority,
                |actual, obligations| {
                    admissions.set(admissions.get() + 1);
                    assert_eq!(actual, &expected_evidence);
                    assert_eq!(obligations, selected.lease.evidence_obligations());
                    Ok(())
                },
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    signs.set(signs.get() + 1);
                    Ok(signature())
                },
            )
            .unwrap();
            assert_eq!(admissions.get(), 1);
            assert_eq!(signs.get(), 1);
            assert!(matches!(
                prepared.observation().1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ContextBound { evidence }
                } if evidence == &expected_evidence
            ));

            let (completed, _) =
                crate::resident::operation_observation_custody::tests::native_execution::complete(
                    prepared.prepare_append(&Store).unwrap(),
                    selected.request,
                    (&record_ref("operation/checkpoint-1"), initial_checkpoint),
                    crate::resident::OperationCustodyDatabaseEvidence::New,
                    61,
                );
            let binding = prepared.finish(&completed, &Store).unwrap();
            let observation = OperationObservationAdmissionContextV1 {
                signed: initial_admission.signed,
                handle_ref,
                handle,
                predecessor: Some(binding.observation()),
            };
            let missing_binding = prepare_provider_resolution(
                ProviderOperationInput {
                    context: crate::resident::OperationAttemptContext {
                        operation: selected.operation,
                        request: selected.request,
                        session: parents.session,
                        current: parents.current,
                        lease: selected.lease,
                        context_binding: None,
                        generation: selected.generation,
                        selection_ref: record_ref("context/missing-binding-selection"),
                        maximum_selection_bytes: 65_536,
                        observation_ref: record_ref("context/missing-binding-preparation"),
                        observation,
                    },
                    invocation: selection.provider_request(selected).unwrap(),
                    result_admission: admission,
                    resolution_ref: record_ref("context/missing-binding-resolution"),
                },
                &mut ProvingAuthority,
                |_, _, _, _| panic!("a missing binding token must not rotate authority"),
                |_, _, _| panic!("a missing binding token must not sign"),
            );
            assert!(matches!(
                missing_binding,
                Err(crate::resident::OperationAttemptError::Association)
            ));
            let resolution = prepare_provider_resolution(
                ProviderOperationInput {
                    context: crate::resident::OperationAttemptContext {
                        operation: selected.operation,
                        request: selected.request,
                        session: parents.session,
                        current: parents.current,
                        lease: selected.lease,
                        context_binding: Some(&binding),
                        generation: selected.generation,
                        selection_ref: record_ref("context/provider-selection"),
                        maximum_selection_bytes: 65_536,
                        observation_ref: record_ref("context/provider-preparation"),
                        observation,
                    },
                    invocation: selection.provider_request(selected).unwrap(),
                    result_admission: admission,
                    resolution_ref: record_ref("context/provider-resolution"),
                },
                &mut ProvingAuthority,
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
            )
            .unwrap();
            assert!(matches!(
                resolution.observation().1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ProviderResolved { .. }
                }
            ));
        },
    );
}

#[test]
fn recovered_provider_dispatch_rejects_an_evidence_lease_without_context_history() {
    with_selection_custody_context(
        LocalStartContext::EvidenceOnly,
        |selection, selected, _, parents, initial_checkpoint, _| {
            let (handle_ref, handle) = selected.operation.handle();
            let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let selection_ref = record_ref("context-skipping/selection");
            let selection_record = selection.record(selection_ref.clone(), 65_536).unwrap();
            let selection_evidence = ImmutableRecordReferenceV1 {
                record_ref: selection_ref,
                record_sha256: ArtifactDigest::parse(
                    selection_record.coordinate().record_sha256().as_str(),
                )
                .unwrap(),
            };
            let resolving_ref = record_ref("context-skipping/resolving");
            let resolving = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            let provider_resolution_ref = record_ref("context-skipping/provider-resolution");
            let provider_resolution = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&resolving_ref, &resolving)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ProviderResolved {
                        provider_binding: selection.provider().unwrap().clone(),
                    },
                },
                61,
            );
            let preparation_ref = record_ref("context-skipping/preparation");
            let preparation = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema.clone(),
                OperationObservationRevisionV1::new(3).unwrap(),
                Some((&provider_resolution_ref, &provider_resolution)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::DispatchPrepared {
                        attempt_id: selection.attempt_id().clone(),
                        evidence: selection_evidence.clone(),
                    },
                },
                62,
            );
            let dispatch_ref = record_ref("context-skipping/dispatched");
            let dispatch = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema,
                OperationObservationRevisionV1::new(4).unwrap(),
                Some((&preparation_ref, &preparation)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched {
                        attempt_id: selection.attempt_id().clone(),
                        evidence: selection_evidence,
                    },
                },
                63,
            );
            let handle_coordinate = crate::resident::record_projection::coordinate(
                handle_ref,
                handle.canonical_operation_handle_sha256().as_str(),
            )
            .unwrap();
            let recovered = RecoveredOperationObservation::history_fixture_for_handle(
                handle_coordinate,
                (dispatch_ref.clone(), dispatch.clone()),
                Some((preparation_ref, preparation)),
                None,
                (dispatch_ref, dispatch.clone()),
                (
                    record_ref("operation/checkpoint-1"),
                    initial_checkpoint.clone(),
                ),
                store_coordinate(),
            );
            let result = A2aSendDispatch::recover(
                OperationAttemptHistoryInput {
                    operation: selected.operation,
                    request: selected.request,
                    session: selected.operation.historical_session_lease(),
                    lease: selected.lease,
                    generation: selected.generation,
                    dispatched: &dispatch,
                    selection: &selection_record,
                    maximum_selection_bytes: 65_536,
                },
                &recovered,
            );
            let Err(error) = result else {
                panic!("context-skipping history must not recover provider dispatch")
            };
            assert!(matches!(
                error,
                crate::resident::OperationAttemptError::ContextRequired
            ));
        },
    );
}

#[test]
fn context_binding_rejects_a_self_contained_operation_before_admission_or_signing() {
    with_selection_custody_context(
        LocalStartContext::SelfContained,
        |_, selected, _, parents, _, initial_record| {
            let (handle_ref, handle) = selected.operation.handle();
            let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            let observation = OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema,
                    signing_authority: parents.signing,
                },
                handle_ref,
                handle,
                predecessor: Some((initial_record.coordinate().record_ref(), &initial)),
            };
            let result = prepare_operation_context_binding(
                OperationContextBindingInput {
                    operation: selected.operation,
                    request: selected.request,
                    session: parents.session,
                    current: parents.current,
                    lease: selected.lease,
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: record_ref("context/unneeded"),
                        record_sha256: artifact_digest('7'),
                    },
                    observation_ref: record_ref("context/unneeded-observation"),
                    observation: &observation,
                },
                &mut ProvingAuthority,
                |_, _| panic!("self-contained operation must not admit context"),
                |_, _, _, _| panic!("self-contained operation must not rotate authority"),
                |_, _, _| panic!("self-contained operation must not sign context"),
            );
            assert!(matches!(
                result,
                Err(crate::resident::OperationAttemptError::ContextNotRequired)
            ));
        },
    );
}

#[test]
fn rejected_context_evidence_cannot_be_signed_or_enter_custody() {
    with_selection_custody_context(
        LocalStartContext::EvidenceOnly,
        |_, selected, _, parents, _, initial_record| {
            let (handle_ref, handle) = selected.operation.handle();
            let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            let observation = OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema,
                    signing_authority: parents.signing,
                },
                handle_ref,
                handle,
                predecessor: Some((initial_record.coordinate().record_ref(), &initial)),
            };
            let result = prepare_operation_context_binding(
                OperationContextBindingInput {
                    operation: selected.operation,
                    request: selected.request,
                    session: parents.session,
                    current: parents.current,
                    lease: selected.lease,
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: record_ref("context/rejected"),
                        record_sha256: artifact_digest('6'),
                    },
                    observation_ref: record_ref("context/rejected-observation"),
                    observation: &observation,
                },
                &mut ProvingAuthority,
                |_, _| {
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "context projection",
                        invariant: "owner admission",
                    })
                },
                |_, _, _, _| panic!("rejected context must not rotate authority"),
                |_, _, _| panic!("rejected context must not be signed"),
            );
            assert!(matches!(
                result,
                Err(crate::resident::OperationAttemptError::Admission(
                    InvalidResidentRecordV1::ContextualAdmission {
                        record: "context projection",
                        invariant: "owner admission",
                    }
                ))
            ));
        },
    );
}

#[test]
fn context_binding_rejects_a_valid_sibling_lease_before_evidence_admission() {
    with_selection_custody_context(
        LocalStartContext::EvidenceOnly,
        |_, selected, _, parents, _, initial_record| {
            let sibling = OperationLeaseV1::seal(
                parents.lease.schema_sha256().clone(),
                parents.session.lease(),
                OperationLeaseRequestV1 {
                    operation_lease_id: OperationLeaseIdV1::parse("sibling-context-lease").unwrap(),
                    operation: parents.lease.operation().clone(),
                    capabilities: parents.lease.capabilities().to_vec(),
                    effects: parents.lease.effects().to_vec(),
                    read_set: parents.lease.read_set().to_vec(),
                    evidence_obligations: vec![record_ref("evidence-a"), record_ref("evidence-b")],
                    budget_units: *parents.lease.budget_units(),
                    timeout_millis: *parents.lease.timeout_millis(),
                    expires_at: parents.lease.expires_at(),
                    invocation_correlation: parents.lease.invocation_correlation().clone(),
                },
            )
            .unwrap();
            assert_ne!(
                sibling.operation_lease_digest(),
                parents.lease.operation_lease_digest()
            );
            let (handle_ref, handle) = selected.operation.handle();
            let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                60,
            );
            let observation = OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema,
                    signing_authority: parents.signing,
                },
                handle_ref,
                handle,
                predecessor: Some((initial_record.coordinate().record_ref(), &initial)),
            };
            let result = prepare_operation_context_binding(
                OperationContextBindingInput {
                    operation: selected.operation,
                    request: selected.request,
                    session: parents.session,
                    current: parents.current,
                    lease: &sibling,
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: record_ref("context/sibling"),
                        record_sha256: artifact_digest('5'),
                    },
                    observation_ref: record_ref("context/sibling-observation"),
                    observation: &observation,
                },
                &mut ProvingAuthority,
                |_, _| panic!("a sibling lease must not reach evidence admission"),
                |_, _, _, _| panic!("a sibling lease must not rotate authority"),
                |_, _, _| panic!("a sibling lease must not sign"),
            );
            assert!(matches!(
                result,
                Err(crate::resident::OperationAttemptError::Association)
            ));
        },
    );
}
