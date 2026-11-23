//! Private selection data over source-compiled definitions and an issued-start fixture.
//! The fixture's signature and custody models establish no native dispatch proof.

use super::*;
use crate::local_operation_start_fixture::{
    host_source_generation_inputs, record_ref, schema_digest, source_generation_inputs_with,
};
use crate::provider_invocation::ProviderInvocationSafeFailureProof;
use crate::resident::operation_observation_custody::tests::local_start;
use pale_ports::OperationCustodyCandidate;

#[path = "operation_dispatch_selection/preparation.rs"]
mod preparation;

#[path = "operation_dispatch_selection/context_binding.rs"]
mod context_binding;

#[path = "operation_dispatch_selection/provider_dispatch.rs"]
pub(in crate::resident) mod provider_dispatch;

#[path = "operation_dispatch_selection/provider_preflight.rs"]
mod provider_preflight;

#[path = "operation_dispatch_selection/provider.rs"]
pub(in crate::resident) mod provider;

#[path = "operation_dispatch_selection/result.rs"]
mod result;

#[path = "operation_dispatch_selection/native.rs"]
mod native;

#[path = "operation_dispatch_selection/shell.rs"]
mod shell;

#[path = "operation_dispatch_selection/a2a_readback_service.rs"]
pub(in crate::resident) mod a2a_readback_service;

fn host_generation(tool_name: &str) -> (AdmittedCompiledGeneration, EffectiveProjectProfileV1) {
    let (compilation, materializations, profile) = host_source_generation_inputs(tool_name);
    (
        AdmittedCompiledGeneration::admit(
            compilation,
            materializations,
            crate::resident_fixture::synthetic_set_policy(),
        )
        .unwrap(),
        profile,
    )
}

pub(in crate::resident) fn admission<'a>(
    success: CapabilitySuccessAdmissionContextV1<'a>,
    failure_schema: &'a SchemaDigest,
) -> ProviderInvocationAdmissionContext<'a> {
    ProviderInvocationAdmissionContext::new(
        success,
        AdapterFailureAdmissionContextV1 {
            expected_schema_sha256: failure_schema,
            expected_request_binding: success.result.expected_request_binding,
            expected_attempt_id: success.result.expected_attempt_id,
            expected_implementation_kind: success.result.expected_implementation_kind,
            expected_implementation_version: success.result.expected_implementation_version,
            signing_authority: success.result.signing_authority,
        },
        ProviderInvocationSafeFailureProof::BeforeDispatch,
    )
}

/// Build one provider selection for an already issued operation.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn with_provider_selection_for_operation<R>(
    operation: &IssuedLocalOperation,
    request_ref: &ResidentRecordRefV1,
    request: &CommonRequestEnvelopeV1,
    generation: &AdmittedCompiledGeneration,
    parents: &crate::local_operation_start_fixture::RecoveryParents<'_>,
    attempt_id: &str,
    selection_ref: ResidentRecordRefV1,
    run: impl FnOnce(
        ProviderInvocationRequest<'_>,
        ProviderInvocationAdmissionContext<'_>,
        &CapabilityAttemptIdV1,
        ControlRecord,
    ) -> R,
) -> R {
    let context = AttemptSelectionContext {
        operation,
        request_ref,
        request,
        session: operation.historical_session_lease(),
        lease: operation.historical_operation_lease(),
        generation,
    };
    let binding = request.local_request_binding().unwrap();
    let attempt = CapabilityAttemptIdV1::parse(attempt_id).unwrap();
    let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
    let version = BoundedResidentTextV1::parse("1").unwrap();
    let contracts = crate::session_admission_fixture::local_start_contract_set();
    let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
    let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
    let provider_admission = admission(
        CapabilitySuccessAdmissionContextV1 {
            result: CapabilityResultAdmissionContextV1 {
                expected_schema_sha256: &result_schema,
                expected_request_binding: &binding,
                expected_attempt_id: &attempt,
                expected_implementation_kind: &kind,
                expected_implementation_version: &version,
                signing_authority: parents.signing,
            },
            contract_set_ref: binding.contract_set_ref(),
            contract_set: &contracts,
        },
        &failure_schema,
    );
    let provider_request = ProviderInvocationRequest::new(
        ProviderInvocationCoordinate::new(
            generation.record().compiled_generation_digest(),
            generation.record().instance_digest(),
            operation.historical_operation_lease().operation(),
            &binding,
            &attempt,
        ),
        generation
            .provider_binding("provider-binding-claude-code")
            .unwrap(),
        &kind,
        &version,
    )
    .unwrap();
    let selection = AttemptSelection::capture(AttemptSelectionInput {
        parents: context,
        invocation: provider_request,
        admission: provider_admission,
    })
    .unwrap();
    let selection_record = selection.record(selection_ref, 65_536).unwrap();
    run(
        provider_request,
        provider_admission,
        &attempt,
        selection_record,
    )
}

/// Build one exact provider selection record for read-only restart tests.
pub(in crate::resident) fn restart_provider_selection_record(
    operation: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    generation: &AdmittedCompiledGeneration,
    parents: &crate::local_operation_start_fixture::RecoveryParents<'_>,
) -> (CapabilityAttemptIdV1, ControlRecord) {
    with_provider_selection_for_operation(
        operation,
        operation.request_coordinate().record_ref(),
        request,
        generation,
        parents,
        "restart-provider-attempt",
        record_ref("restart/provider-selection"),
        |_, _, attempt, record| (attempt.clone(), record),
    )
}

/// Build one exact built-in selection record for read-only restart tests.
pub(in crate::resident) fn restart_native_selection_record(
    operation: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    generation: &AdmittedCompiledGeneration,
    parents: &crate::local_operation_start_fixture::RecoveryParents<'_>,
    registry: &crate::actions::NativeActionRegistry,
) -> (CapabilityAttemptIdV1, ControlRecord) {
    let context = AttemptSelectionContext {
        operation,
        request_ref: operation.request_coordinate().record_ref(),
        request,
        session: parents.session.lease(),
        lease: parents.lease,
        generation,
    };
    native::with_admission(context, parents.signing, |attempt, admission| {
        let (selection, _) =
            AttemptSelection::capture_native(context, registry, attempt, admission).unwrap();
        let record = selection
            .record(record_ref("restart/native-selection"), 1024 * 1024)
            .unwrap();
        (attempt.clone(), record)
    })
}

fn with_selection<R>(
    run: impl FnOnce(
        AttemptSelection,
        AttemptSelectionContext<'_>,
        ProviderInvocationAdmissionContext<'_>,
    ) -> R,
) -> R {
    with_selection_context(|selection, context, admission, _| run(selection, context, admission))
}

fn with_selection_context<R>(
    run: impl FnOnce(
        AttemptSelection,
        AttemptSelectionContext<'_>,
        ProviderInvocationAdmissionContext<'_>,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_selection_custody_context(
        crate::session_admission_fixture::LocalStartContext::Inherited,
        |selection, context, admission, parents, _, _| run(selection, context, admission, parents),
    )
}

fn with_selection_custody_context<R>(
    context_selection: crate::session_admission_fixture::LocalStartContext,
    run: impl FnOnce(
        AttemptSelection,
        AttemptSelectionContext<'_>,
        ProviderInvocationAdmissionContext<'_>,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
        &ControlStateCheckpointV1,
        &ControlRecord,
    ) -> R,
) -> R {
    with_selection_recovery_context(
        context_selection,
        |selection,
         context,
         admission,
         parents,
         checkpoint,
         observation,
         _initial_candidate,
         _restart_generation| {
            run(
                selection,
                context,
                admission,
                parents,
                checkpoint,
                observation,
            )
        },
    )
}

fn with_selection_recovery_context<R>(
    context_selection: crate::session_admission_fixture::LocalStartContext,
    run: impl FnOnce(
        AttemptSelection,
        AttemptSelectionContext<'_>,
        ProviderInvocationAdmissionContext<'_>,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
        &ControlStateCheckpointV1,
        &ControlRecord,
        &OperationCustodyCandidate,
        AdmittedCompiledGeneration,
    ) -> R,
) -> R {
    let (mut generations, profile) = local_start::host_source_generation_copies(2);
    let restart_generation = generations.pop().expect("restart generation copy");
    let generation = generations.pop().expect("selected generation copy");
    local_start::with_issued_context(
        &generation,
        &profile,
        "selection",
        |_| {},
        context_selection,
        |issued, request, parents, initial_checkpoint, initial_record, initial_candidate| {
            let binding = request.local_request_binding().unwrap();
            let attempt = CapabilityAttemptIdV1::parse("attempt-1").unwrap();
            let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
            let version = BoundedResidentTextV1::parse("1").unwrap();
            let contracts = crate::session_admission_fixture::local_start_contract_set();
            let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
            let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
            let context = admission(
                CapabilitySuccessAdmissionContextV1 {
                    result: CapabilityResultAdmissionContextV1 {
                        expected_schema_sha256: &result_schema,
                        expected_request_binding: &binding,
                        expected_attempt_id: &attempt,
                        expected_implementation_kind: &kind,
                        expected_implementation_version: &version,
                        signing_authority: parents.signing,
                    },
                    contract_set_ref: binding.contract_set_ref(),
                    contract_set: &contracts,
                },
                &failure_schema,
            );
            let invocation = ProviderInvocationRequest::new(
                ProviderInvocationCoordinate::new(
                    generation.record().compiled_generation_digest(),
                    generation.record().instance_digest(),
                    parents.lease.operation(),
                    &binding,
                    &attempt,
                ),
                generation
                    .provider_binding("provider-binding-claude-code")
                    .unwrap(),
                &kind,
                &version,
            )
            .unwrap();
            let request_ref = record_ref("request/selection");
            let selection_context = AttemptSelectionContext {
                operation: issued,
                request_ref: &request_ref,
                request,
                session: parents.session.lease(),
                lease: parents.lease,
                generation: &generation,
            };
            let selection = AttemptSelection::capture(AttemptSelectionInput {
                parents: selection_context,
                invocation,
                admission: context,
            })
            .unwrap();
            run(
                selection,
                selection_context,
                context,
                parents,
                initial_checkpoint,
                initial_record,
                initial_candidate,
                restart_generation,
            )
        },
    )
}

pub(in crate::resident) fn with_provider_selection_recovery_context<R>(
    context_selection: crate::session_admission_fixture::LocalStartContext,
    run: impl FnOnce(
        pale_ports::ProviderInvocationRequest<'_>,
        &IssuedLocalOperation,
        &CommonRequestEnvelopeV1,
        &AdmittedCompiledGeneration,
        ProviderInvocationAdmissionContext<'_>,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
        &ControlStateCheckpointV1,
        &ControlRecord,
        &OperationCustodyCandidate,
    ) -> R,
) -> R {
    with_provider_selection_recovery_context_at_store(
        context_selection,
        "selection",
        &crate::local_operation_start_fixture::store_coordinate(),
        |provider_request,
         operation,
         request,
         generation,
         admission,
         parents,
         checkpoint,
         observation,
         candidate,
         _selection| {
            run(
                provider_request,
                operation,
                request,
                generation,
                admission,
                parents,
                checkpoint,
                observation,
                candidate,
            )
        },
    )
}

pub(in crate::resident) fn with_provider_selection_restart_context<R>(
    context_selection: crate::session_admission_fixture::LocalStartContext,
    run: impl FnOnce(
        pale_ports::ProviderInvocationRequest<'_>,
        &IssuedLocalOperation,
        &CommonRequestEnvelopeV1,
        &AdmittedCompiledGeneration,
        ProviderInvocationAdmissionContext<'_>,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
        &ControlStateCheckpointV1,
        &ControlRecord,
        &OperationCustodyCandidate,
        &ControlRecord,
        AdmittedCompiledGeneration,
    ) -> R,
) -> R {
    with_selection_recovery_context(
        context_selection,
        |selection,
         context,
         admission,
         parents,
         checkpoint,
         observation,
         candidate,
         restart_generation| {
            let provider_request = selection
                .provider_request(context)
                .expect("source-selected provider request");
            let selection_record = selection
                .record(record_ref("provider/selection"), 65_536)
                .expect("bounded provider selection record");
            run(
                provider_request,
                context.operation,
                context.request,
                context.generation,
                admission,
                parents,
                checkpoint,
                observation,
                candidate,
                &selection_record,
                restart_generation,
            )
        },
    )
}

pub(in crate::resident) fn with_provider_selection_recovery_context_at_store<R>(
    context_selection: crate::session_admission_fixture::LocalStartContext,
    suffix: &str,
    store: &pale_ports::AuthorityStoreCoordinate,
    run: impl FnOnce(
        pale_ports::ProviderInvocationRequest<'_>,
        &IssuedLocalOperation,
        &CommonRequestEnvelopeV1,
        &AdmittedCompiledGeneration,
        ProviderInvocationAdmissionContext<'_>,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
        &ControlStateCheckpointV1,
        &ControlRecord,
        &OperationCustodyCandidate,
        &ControlRecord,
    ) -> R,
) -> R {
    let (generation, profile) = host_generation("bounded_compute");
    local_start::with_issued_context_at_store(
        &generation,
        &profile,
        suffix,
        |_| {},
        context_selection,
        store,
        |operation, request, parents, checkpoint, observation, candidate| {
            let binding = request
                .local_request_binding()
                .expect("local request binding");
            let attempt = CapabilityAttemptIdV1::parse("attempt-1").unwrap();
            let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
            let version = BoundedResidentTextV1::parse("1").unwrap();
            let contracts = crate::session_admission_fixture::local_start_contract_set();
            let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
            let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
            let admission = admission(
                CapabilitySuccessAdmissionContextV1 {
                    result: CapabilityResultAdmissionContextV1 {
                        expected_schema_sha256: &result_schema,
                        expected_request_binding: &binding,
                        expected_attempt_id: &attempt,
                        expected_implementation_kind: &kind,
                        expected_implementation_version: &version,
                        signing_authority: parents.signing,
                    },
                    contract_set_ref: binding.contract_set_ref(),
                    contract_set: &contracts,
                },
                &failure_schema,
            );
            let invocation = ProviderInvocationRequest::new(
                ProviderInvocationCoordinate::new(
                    generation.record().compiled_generation_digest(),
                    generation.record().instance_digest(),
                    parents.lease.operation(),
                    &binding,
                    &attempt,
                ),
                generation
                    .provider_binding("provider-binding-claude-code")
                    .unwrap(),
                &kind,
                &version,
            )
            .unwrap();
            let selection_ref = record_ref(&format!("provider/{suffix}/selection"));
            let request_ref = record_ref(&format!("request/{suffix}"));
            let context = AttemptSelectionContext {
                operation,
                request_ref: &request_ref,
                request,
                session: parents.session.lease(),
                lease: parents.lease,
                generation: &generation,
            };
            let selection = AttemptSelection::capture(AttemptSelectionInput {
                parents: context,
                invocation,
                admission,
            })
            .unwrap();
            let provider_request = selection
                .provider_request(context)
                .expect("source-selected provider request");
            let selection_record = selection
                .record(selection_ref.clone(), 65_536)
                .expect("bounded provider selection record");
            run(
                provider_request,
                operation,
                request,
                &generation,
                admission,
                parents,
                checkpoint,
                observation,
                candidate,
                &selection_record,
            )
        },
    )
}

fn evidence(record: &ControlRecord) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::parse(record.coordinate().record_sha256().as_str()).unwrap(),
    }
}

// Rehash altered bytes so syntax and canonicalization tests cannot pass merely
// because they rejected a stale digest before reaching the intended check.
fn replace_bytes(record: &ControlRecord, bytes: Vec<u8>) -> ControlRecord {
    ControlRecord::new(
        ControlRecordCoordinate::new(
            record.coordinate().record_ref().clone(),
            ControlRecordDigest::parse(pale_core::sha256_hex(&bytes)).unwrap(),
        ),
        record.schema_id().clone(),
        record.schema_sha256().clone(),
        ControlRecordBytes::new(bytes).unwrap(),
    )
}

#[test]
fn canonical_selection_roundtrips_at_its_exact_bound_and_uses_the_actual_schema() {
    with_selection(|selection, parents, _| {
        let record = selection
            .record(record_ref("attempt-selection"), 65_536)
            .unwrap();
        let length = record.canonical_record_jcs().as_slice().len();
        let decoded = AttemptSelection::decode(&record, &evidence(&record), length).unwrap();
        assert_eq!(decoded, selection);
        decoded.validate(&parents).unwrap();
        assert_eq!(
            decoded
                .provider_request(parents)
                .unwrap()
                .provider_binding()
                .binding_identity,
            selection.provider().unwrap().provider_binding_identity
        );
        assert_eq!(
            selection
                .record(record_ref("attempt-selection"), length)
                .unwrap(),
            record
        );
        assert!(matches!(
            AttemptSelection::decode(&record, &evidence(&record), length - 1).unwrap_err(),
            AttemptSelectionError::BoundExceeded
        ));
        assert!(matches!(
            selection
                .record(record_ref("attempt-selection"), length - 1)
                .unwrap_err(),
            AttemptSelectionError::BoundExceeded
        ));
        let schema = schemars::schema_for!(AttemptSelection);
        let bytes = canon::to_canonical_string(&schema).unwrap();
        assert_eq!(
            record.schema_sha256().as_str(),
            pale_core::sha256_hex(bytes.as_bytes())
        );
        assert_ne!(
            record.schema_sha256().as_str(),
            pale_core::sha256_hex(SELECTION_SCHEMA.as_bytes())
        );
    });
}

#[test]
fn private_decode_rejects_unknown_duplicate_noncanonical_and_substituted_records() {
    with_selection(|selection, _, _| {
        let record = selection
            .record(record_ref("attempt-selection"), 65_536)
            .unwrap();
        let mut unknown = serde_json::to_value(&selection).unwrap();
        unknown
            .as_object_mut()
            .unwrap()
            .insert("unselected".into(), serde_json::json!(true));
        let canonical =
            String::from_utf8(record.canonical_record_jcs().as_slice().to_vec()).unwrap();
        let duplicate = format!("{{\"attempt_id\":\"attempt-1\",{}", &canonical[1..]);
        for bytes in [
            canon::to_canonical_string(&unknown).unwrap().into_bytes(),
            serde_json::to_vec_pretty(&selection).unwrap(),
            duplicate.into_bytes(),
        ] {
            let altered = replace_bytes(&record, bytes);
            assert!(matches!(
                AttemptSelection::decode(&altered, &evidence(&altered), 65_536).unwrap_err(),
                AttemptSelectionError::Encoding
            ));
        }
        let alternate = selection
            .record(record_ref("different-selection"), 65_536)
            .unwrap();
        assert!(matches!(
            AttemptSelection::decode(&record, &evidence(&alternate), 65_536).unwrap_err(),
            AttemptSelectionError::Encoding
        ));
        let changed = replace_bytes(&record, br#"{}"#.to_vec());
        assert!(matches!(
            AttemptSelection::decode(&changed, &evidence(&record), 65_536).unwrap_err(),
            AttemptSelectionError::Encoding
        ));
        let wrong_schema = ControlRecord::new(
            record.coordinate().clone(),
            record.schema_id().clone(),
            SchemaDigest::parse("0".repeat(64)).unwrap(),
            record.canonical_record_jcs().clone(),
        );
        assert!(matches!(
            AttemptSelection::decode(&wrong_schema, &evidence(&record), 65_536).unwrap_err(),
            AttemptSelectionError::Encoding
        ));
    });
}

#[test]
fn decoded_selection_rejects_foreign_request_handle_generation_and_provider() {
    with_selection(|selection, parents, _| {
        for change in 0..4 {
            let mut changed = selection.clone();
            match change {
                0 => changed.request_ref = record_ref("different-request"),
                1 => {
                    changed.request_sha256 =
                        CommonRequestEnvelopeDigest::parse("0".repeat(64)).unwrap()
                }
                2 => changed.handle_ref = record_ref("different-handle"),
                3 => changed.handle_sha256 = OperationHandleDigest::parse("0".repeat(64)).unwrap(),
                _ => unreachable!(),
            }
            let record = changed
                .record(record_ref("altered-selection"), 65_536)
                .unwrap();
            let decoded = AttemptSelection::decode(&record, &evidence(&record), 65_536).unwrap();
            assert!(matches!(
                decoded.validate(&parents),
                Err(AttemptSelectionError::Association)
            ));
        }
        let (other_generation, _) = host_generation("other_compute");
        assert_ne!(
            parents.generation.record().compiled_generation_digest(),
            other_generation.record().compiled_generation_digest()
        );
        assert!(matches!(
            selection.validate(&AttemptSelectionContext {
                generation: &other_generation,
                ..parents
            }),
            Err(AttemptSelectionError::Association)
        ));
        for change in 0..2 {
            let mut changed = selection.clone();
            let AttemptImplementation::HostNative {
                provider: selected_provider,
            } = &mut changed.implementation
            else {
                panic!("host selection fixture");
            };
            if change == 0 {
                selected_provider.provider_binding_id = "different-provider".into();
            } else {
                let mut provider = parents
                    .generation
                    .provider_binding(&selected_provider.provider_binding_id)
                    .unwrap()
                    .to_preimage();
                provider.id = "different-provider".into();
                selected_provider.provider_binding_identity =
                    ProviderBindingV1::seal(provider).unwrap().binding_identity;
            }
            assert!(matches!(
                changed.provider_request(parents),
                Err(AttemptSelectionError::Implementation)
            ));
        }
    });
}

#[test]
fn capture_rejects_a_same_byte_request_alias_and_an_unselected_provider() {
    with_selection(|selection, parents, context| {
        let invocation = selection.provider_request(parents).unwrap();
        let alias = record_ref("request-alias");
        assert!(matches!(
            AttemptSelection::capture(AttemptSelectionInput {
                parents: AttemptSelectionContext {
                    request_ref: &alias,
                    ..parents
                },
                invocation,
                admission: context,
            })
            .unwrap_err(),
            AttemptSelectionError::Association
        ));

        let mut preimage = invocation.provider_binding().to_preimage();
        preimage.id = "unselected-provider".into();
        let provider = ProviderBindingV1::seal(preimage).unwrap();
        let unselected = ProviderInvocationRequest::new(
            ProviderInvocationCoordinate::new(
                invocation.compiled_generation_digest(),
                invocation.instance_digest(),
                invocation.operation(),
                invocation.request_binding(),
                invocation.attempt_id(),
            ),
            &provider,
            invocation.implementation_kind(),
            invocation.implementation_version(),
        )
        .unwrap();
        assert!(matches!(
            AttemptSelection::capture(AttemptSelectionInput {
                parents,
                invocation: unselected,
                admission: context,
            })
            .unwrap_err(),
            AttemptSelectionError::Implementation
        ));
    });
}
