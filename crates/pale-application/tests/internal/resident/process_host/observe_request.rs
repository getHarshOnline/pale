//! Fresh-request admission laws for process-host Observe.

use std::cell::Cell;
use std::time::{Duration, Instant};

use pale_core::records::{
    CapabilityIdV1, CommonRequestAdmissionContextV1, CommonRequestEnvelopeDigest,
    CommonRequestEnvelopePreimageV1, CommonRequestEnvelopeV1, ResidentRequestIdV1,
};
use pale_ports::{
    ControlRecordCoordinate, ControlRecordDigest, LocalIpcCarrierError, LocalIpcCarrierOperation,
    LocalIpcFramePort, LocalIpcFrameSection, LocalIpcPeerCredentials, ResidentObserveFrameRequest,
    ResidentObserveFrameResponse,
};

use super::*;
use crate::resident::process_host::observe_request::{
    ResidentObserveReplayAdmissionError, ResidentObserveReplayDisposition,
    ResidentProcessObserveRequestAdmission, ResidentProcessObserveRequestError,
    ResidentProcessObserveRequestInput,
};
use crate::resident::process_host::observe_request_host::{
    frame_failure, ResidentObserveRequestHost, ResidentObserveRequestHostError,
    ResidentObserveRequestResolution,
};
use crate::resident::{OperationControlError, OperationObservationSelection};

struct ScriptedObserveFramePort {
    peer: LocalIpcPeerCredentials,
    inbound: Option<Result<Box<[u8]>, LocalIpcCarrierError>>,
    sent: Vec<u8>,
    sent_deadline: Option<Instant>,
}

impl ScriptedObserveFramePort {
    fn frame(bytes: Vec<u8>) -> Self {
        Self {
            peer: LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
            inbound: Some(Ok(bytes.into_boxed_slice())),
            sent: Vec::new(),
            sent_deadline: None,
        }
    }

    fn failure(error: LocalIpcCarrierError) -> Self {
        Self {
            peer: LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
            inbound: Some(Err(error)),
            sent: Vec::new(),
            sent_deadline: None,
        }
    }
}

impl LocalIpcFramePort for ScriptedObserveFramePort {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        Ok(self.peer)
    }

    fn send_frame(
        &mut self,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        self.sent.extend_from_slice(payload);
        self.sent_deadline = Some(deadline);
        Ok(())
    }

    fn receive_frame(&mut self, _deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.inbound
            .take()
            .expect("script contains one inbound result")
    }
}

fn canonical_request(request: &CommonRequestEnvelopeV1) -> Vec<u8> {
    pale_core::canon::to_canonical_string(request)
        .expect("admitted request remains canonical")
        .into_bytes()
}

fn canonical_value(value: &serde_json::Value) -> Vec<u8> {
    pale_core::canon::to_canonical_string(value)
        .expect("test value remains canonical")
        .into_bytes()
}

fn request_value(request: &CommonRequestEnvelopeV1) -> serde_json::Value {
    serde_json::to_value(request).expect("admitted request serializes")
}

fn reseal_request(mut value: serde_json::Value) -> Vec<u8> {
    let object = value.as_object_mut().expect("request is an object");
    let signature = object
        .remove("caller_signature")
        .expect("request has a signature");
    object.remove("canonical_request_sha256");
    let digest = CommonRequestEnvelopeDigest::hash_canonical(&value)
        .expect("request preimage remains canonical");
    let object = value.as_object_mut().expect("request remains an object");
    object.insert(
        "canonical_request_sha256".to_owned(),
        serde_json::json!(digest.as_str()),
    );
    object.insert("caller_signature".to_owned(), signature);
    canonical_value(&value)
}

fn reseal_handle(mut value: serde_json::Value) -> serde_json::Value {
    let object = value.as_object_mut().expect("handle is an object");
    let signature = object.remove("signature").expect("handle has a signature");
    object.remove("canonical_operation_handle_sha256");
    let digest =
        OperationHandleDigest::hash_canonical(&value).expect("handle preimage remains canonical");
    let object = value.as_object_mut().expect("handle remains an object");
    object.insert(
        "canonical_operation_handle_sha256".to_owned(),
        serde_json::json!(digest.as_str()),
    );
    object.insert("signature".to_owned(), signature);
    value
}

fn observe_request_input<'a>(
    base: &'a LocalOperationObservationInput<'_>,
    request_bytes: &'a [u8],
    maximum_request_bytes: usize,
    locator: &'a ControlRecordCoordinate,
    recovery_store: &'a AuthorityStoreCoordinate,
    historical_authority: &'a SigningAuthorityBindingV1,
    observation: OperationObservationSelection<'a>,
) -> ResidentProcessObserveRequestInput<'a> {
    ResidentProcessObserveRequestInput {
        request_bytes,
        maximum_request_bytes,
        locator,
        recovery: OperationControlRecoveryInput {
            operation: LocalOperationHandleRecoveryInput {
                context: LocalOperationRecoveryContext {
                    store: recovery_store,
                    handle_signing_authority: historical_authority,
                    ..base.recovery.operation.context
                },
                expected_handle: base.recovery.operation.expected_handle,
                maximum_parent_depth: base.recovery.operation.maximum_parent_depth,
            },
            observation,
            observation_limits: base.recovery.observation_limits,
            checkpoint_limits: base.recovery.checkpoint_limits,
            membership_limits: base.recovery.membership_limits,
        },
        authorized_at: base.authorized_at,
    }
}

fn with_request_context<Result>(
    request: &CommonRequestEnvelopeV1,
    capability_override: Option<&CapabilityIdV1>,
    run: impl FnOnce(&CommonRequestAdmissionContextV1<'_>) -> Result,
) -> Result {
    let preimage = request.to_preimage();
    assert!(
        preimage.execution_owner_fence.is_none(),
        "this focused fixture carries no external execution fence"
    );
    let capability = capability_override.unwrap_or(&preimage.capability_id);
    let context = CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &preimage.envelope_schema_sha256,
        expected_body_schema_id: &preimage.body.schema_id,
        expected_body_schema_sha256: &preimage.body.schema_sha256,
        caller: &preimage.caller,
        signing_authority: &preimage.signing_authority_binding,
        route: &preimage.route,
        session_admission: &preimage.session_admission,
        operation_target: preimage.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: preimage.federation_claim.as_ref(),
        workspace_id: &preimage.workspace_id,
        capability_id: capability,
        operation: &preimage.operation,
        scope: &preimage.scope,
        risk_class: &preimage.risk_class,
        resource_constraints: &preimage.resource_constraints,
        expected_precondition: preimage.expected_precondition.as_ref(),
        idempotency_key: &preimage.idempotency_key,
        issued_at: preimage.issued_at,
        expires_at: preimage.expires_at,
        generation: preimage.generation.as_ref(),
        fence_token: preimage.fence_token.as_ref(),
    };
    run(&context)
}

fn verify_fixture_signature(
    calls: &Cell<usize>,
) -> impl FnMut(
    &CommonRequestEnvelopePreimageV1,
    &CommonRequestEnvelopeDigest,
    &CanonicalSignatureV1,
) -> Result<(), InvalidResidentRecordV1>
       + '_ {
    move |_, _, signature| {
        calls.set(calls.get() + 1);
        if signature.as_str() == "A".repeat(86) {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "common request envelope",
                invariant: "caller signature",
            })
        }
    }
}

struct UnreachableRecoveryRequestAdmission<'a> {
    replay_result:
        Option<Result<ResidentObserveReplayDisposition, ResidentObserveReplayAdmissionError>>,
    verify_caller_signature: &'a mut dyn FnMut(
        &CommonRequestEnvelopePreimageV1,
        &CommonRequestEnvelopeDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
}

fn observe_with_unreachable_recovery<Authority>(
    host: &mut ResidentProcessHost<'_, UnprovedHeldWriter>,
    selector: &ResidentDeploymentAuthoritySelector,
    access: ResidentDeploymentOperationAccess<'_>,
    authority: &mut Authority,
    input: ResidentProcessObserveRequestInput<'_>,
    request_context: &CommonRequestAdmissionContextV1<'_>,
    request_admission: UnreachableRecoveryRequestAdmission<'_>,
) -> Result<OperationControlObservation, ResidentProcessObserveRequestError> {
    let mut admit_request = |_: &ResidentRecordRefV1, _: CommonRequestEnvelopeWireV1| {
        panic!("rejected fresh request must not read historical request custody")
    };
    let mut admit_handle_authority =
        |_: &OperationHandlePreimageV1, _: Option<&OperationHandleV1>| {
            panic!("rejected fresh request must not admit a historical handle")
        };
    let mut verify_handle_signature =
        |_: &[u8],
         _: &OperationHandlePreimageV1,
         _: &OperationHandleDigest,
         _: &CanonicalSignatureV1| {
            panic!("rejected fresh request must not verify a historical handle")
        };
    let mut admit_checkpoint = |_: &ResidentRecordRefV1, _: ControlStateCheckpointWireV1| {
        panic!("rejected fresh request must not admit a checkpoint")
    };
    let mut resolve_observation_signer =
        |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
            panic!("rejected fresh request must not resolve an observation signer")
        };
    let mut admit_observation_state = |_: &OperationObservationStateV1| {
        panic!("rejected fresh request must not admit observation state")
    };
    let mut admit_rotation = |_: &ResidentRecordRefV1,
                              _: &ArtifactDigest,
                              _: &ResidentRecordRefV1,
                              _: &ArtifactDigest| {
        panic!("rejected fresh request must not admit authority rotation")
    };
    let mut verify_observation_signature =
        |_: &[u8],
         _: &OperationObservationPreimageV1,
         _: &OperationObservationDigest,
         _: &CanonicalSignatureV1| {
            panic!("rejected fresh request must not verify an observation")
        };
    let mut verify_checkpoint_signature =
        |_: &[u8],
         _: &ControlStateCheckpointPreimageV1,
         _: &ControlStateCheckpointDigest,
         _: &CanonicalSignatureV1| {
            panic!("rejected fresh request must not verify a checkpoint")
        };
    let mut authorize_disclosure = |_: OperationControlAuthorizationContext<'_>| {
        panic!("rejected fresh request must not reach disclosure authorization")
    };
    let mut replay_result = request_admission.replay_result;
    let mut admit_replay = |_: &ResidentRequestIdV1, _: &CommonRequestEnvelopeDigest, _: &[u8]| {
        replay_result
            .take()
            .unwrap_or_else(|| panic!("rejected fresh request must not reach replay admission"))
    };
    host.observe_request(
        selector,
        access,
        authority,
        input,
        ResidentProcessObserveRequestAdmission {
            request: request_context,
            verify_caller_signature: request_admission.verify_caller_signature,
            admit_replay: &mut admit_replay,
            recovery: OperationControlRecoveryAdmission {
                admit_request: &mut admit_request,
                admit_handle_authority: &mut admit_handle_authority,
                verify_handle_signature: &mut verify_handle_signature,
                admit_checkpoint: &mut admit_checkpoint,
                resolve_observation_signer: &mut resolve_observation_signer,
                admit_observation_state: &mut admit_observation_state,
                admit_rotation: &mut admit_rotation,
                verify_observation_signature: &mut verify_observation_signature,
                verify_checkpoint_signature: &mut verify_checkpoint_signature,
            },
            authorize_disclosure: &mut authorize_disclosure,
        },
    )
}

fn host_for_observe<'store>(
    selector: &ResidentDeploymentAuthoritySelector,
    selected_store: &AuthorityStoreCoordinate,
    application_port: &'store MemoryAuthorityStore,
) -> ResidentProcessHost<'store, UnprovedHeldWriter> {
    let application = ready_application(application_port, selected_store.clone(), 1);
    let deployment =
        unproved_deployment_for_selector(selector, selected_store.clone(), application);
    let writer = unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]);
    ResidentProcessHost::new(
        writer,
        vec![deployment],
        NonZeroUsize::new(1).expect("positive deployment bound"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact Observe deployment")
}

#[test]
fn framed_host_rejects_bad_frames_before_resident_resolution() {
    let deadline = Instant::now() + Duration::from_secs(1);
    for bytes in [Vec::new(), b"{ }".to_vec(), vec![b'x'; 262_145]] {
        let mut port = ScriptedObserveFramePort::frame(bytes);
        let failure = ResidentObserveRequestHost::new(&mut port, deadline)
            .receive()
            .expect_err("bad frame must fail before a resolver is involved");
        assert!(matches!(failure, ResidentObserveRequestHostError::Frame));
        assert!(port.sent.is_empty());
    }

    for carrier_failure in [
        LocalIpcCarrierError::DeadlineElapsed {
            operation: LocalIpcCarrierOperation::ReceiveFrame,
            frame_bytes_transferred: 0,
        },
        LocalIpcCarrierError::DeadlineElapsed {
            operation: LocalIpcCarrierOperation::ReceiveFrame,
            frame_bytes_transferred: 3,
        },
        LocalIpcCarrierError::TruncatedFrame {
            section: LocalIpcFrameSection::Payload,
            expected_bytes: 8,
            received_bytes: 3,
        },
        LocalIpcCarrierError::ConnectionUnusable,
    ] {
        let mut port = ScriptedObserveFramePort::failure(carrier_failure);
        let failure = ResidentObserveRequestHost::new(&mut port, deadline)
            .receive()
            .expect_err("carrier failure must remain typed");
        assert!(matches!(
            failure,
            ResidentObserveRequestHostError::Carrier(_)
        ));
        assert!(port.sent.is_empty());
    }
}

#[test]
fn framed_host_returns_the_exact_authenticated_observation() {
    with_process_host_observe_fixture(&mut |fixture: ProcessHostObserveFixture<'_>| {
        let ProcessHostObserveFixture {
            input,
            admission,
            signing_authority,
            custody,
            sessions,
            anchor,
        } = fixture;
        let selected_store = input.recovery.operation.context.store.clone();
        let locator = input.recovery.operation.expected_handle.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let application_port = MemoryAuthorityStore::default();
        let mut resident = host_for_observe(&selector, &selected_store, &application_port);
        let mut authority = ProvingAuthority;
        let request_bytes = canonical_request(input.request);
        let frame = ResidentObserveFrameRequest::new(
            locator.record_ref().as_str().to_owned(),
            locator.record_sha256().as_str().to_owned(),
            String::from_utf8(request_bytes.clone()).expect("canonical request is UTF-8"),
        );
        let frame_bytes = pale_core::canon::to_canonical_string(&frame)
            .expect("frame remains canonical")
            .into_bytes();
        let mut port = ScriptedObserveFramePort::frame(frame_bytes);
        let deadline = Instant::now() + Duration::from_secs(2);
        let response_deadline = Instant::now() + Duration::from_secs(1);

        let observed = {
            let mut host = ResidentObserveRequestHost::new(&mut port, deadline);
            let received = host.receive().expect("exact frame is admitted");
            let signature_calls = Cell::new(0);
            let mut verify_caller_signature = verify_fixture_signature(&signature_calls);
            let mut admit_replay =
                |_: &ResidentRequestIdV1, _: &CommonRequestEnvelopeDigest, _: &[u8]| {
                    Ok(ResidentObserveReplayDisposition::FirstAdmission)
                };
            let peer_calls = Cell::new(0);
            let mut admit_peer =
                |peer: LocalIpcPeerCredentials, request: &ResidentObserveFrameRequest| {
                    peer_calls.set(peer_calls.get() + 1);
                    assert_eq!(peer.user_id(), 501);
                    assert_eq!(
                        request.operation_handle_record_ref(),
                        locator.record_ref().as_str()
                    );
                    Ok(())
                };
            let LocalOperationObservationAdmission {
                recovery,
                authorize,
            } = admission;
            let admitted = host
                .admit_peer(received, &mut admit_peer)
                .expect("supplementary peer evidence is admitted first");
            assert_eq!(admitted.peer(), &());

            let observed = with_request_context(input.request, None, |request_context| {
                host.answer_admitted(
                    &admitted,
                    &mut resident,
                    ResidentObserveRequestResolution {
                        selector: &selector,
                        access: ResidentDeploymentOperationAccess {
                            store: &selected_store,
                            custody,
                            sessions,
                            anchor,
                        },
                        authority: &mut authority,
                        input: observe_request_input(
                            &input,
                            &request_bytes,
                            request_bytes.len(),
                            &locator,
                            &selected_store,
                            signing_authority,
                            OperationObservationSelection::SelectedHead,
                        ),
                        admission: ResidentProcessObserveRequestAdmission {
                            request: request_context,
                            verify_caller_signature: &mut verify_caller_signature,
                            admit_replay: &mut admit_replay,
                            recovery,
                            authorize_disclosure: authorize,
                        },
                        response_deadline,
                    },
                )
            })
            .expect("exact authenticated Observe frame succeeds")
            .expect("successful host answer retains the observed records");
            assert_eq!(signature_calls.get(), 1);
            assert_eq!(peer_calls.get(), 1);
            observed
        };

        let response: ResidentObserveFrameResponse =
            serde_json::from_slice(&port.sent).expect("host response is typed JSON");
        assert_eq!(port.sent_deadline, Some(response_deadline));
        let ResidentObserveFrameResponse::Observed {
            operation_handle_record_ref,
            operation_handle_canonical,
            operation_observation_record_ref: _,
            operation_observation_canonical: _,
        } = response
        else {
            panic!("successful resident Observe returned a rejection")
        };
        assert_eq!(operation_handle_record_ref, locator.record_ref().as_str());
        let handle: serde_json::Value =
            serde_json::from_str(&operation_handle_canonical).expect("handle is JSON");
        assert_eq!(
            handle["canonical_operation_handle_sha256"],
            locator.record_sha256().as_str()
        );

        Ok(observed)
    });
}

#[test]
fn framed_failure_distinguishes_denied_admission_from_resident_unavailability() {
    let denied =
        ResidentProcessObserveRequestError::Observation(ResidentProcessObservationError::Observe(
            LocalOperationObservationError::Authorization(OperationControlError::Time),
        ));
    assert_eq!(
        frame_failure(&denied),
        pale_ports::ResidentObserveFrameFailure::AdmissionDenied
    );
}

#[test]
fn exact_fresh_request_returns_the_current_anchored_head_without_mutation() {
    with_process_host_observe_fixture(&mut |fixture: ProcessHostObserveFixture<'_>| {
        let ProcessHostObserveFixture {
            input,
            admission,
            signing_authority,
            custody,
            sessions,
            anchor,
        } = fixture;
        let selected_store = input.recovery.operation.context.store.clone();
        let locator = input.recovery.operation.expected_handle.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let application_port = MemoryAuthorityStore::default();
        let mut host = host_for_observe(&selector, &selected_store, &application_port);
        let mut authority = ProvingAuthority;
        let request_bytes = canonical_request(input.request);
        let signature_calls = Cell::new(0);
        let mut verify_caller_signature = verify_fixture_signature(&signature_calls);
        let mut admit_replay =
            |_: &ResidentRequestIdV1, _: &CommonRequestEnvelopeDigest, _: &[u8]| {
                Ok(ResidentObserveReplayDisposition::ExactReplay)
            };
        let LocalOperationObservationAdmission {
            recovery,
            authorize,
        } = admission;

        let observed = with_request_context(input.request, None, |request_context| {
            host.observe_request(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &request_bytes,
                    request_bytes.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                ResidentProcessObserveRequestAdmission {
                    request: request_context,
                    verify_caller_signature: &mut verify_caller_signature,
                    admit_replay: &mut admit_replay,
                    recovery,
                    authorize_disclosure: authorize,
                },
            )
        })
        .expect("exact authenticated Observe request succeeds");

        assert_eq!(signature_calls.get(), 1);
        assert_eq!(observed.operation().0, locator.record_ref());
        assert_eq!(
            observed
                .operation()
                .1
                .canonical_operation_handle_sha256()
                .as_str(),
            locator.record_sha256().as_str()
        );
        Ok(observed)
    });
}

#[test]
fn replay_owner_rejects_changed_bytes_before_resident_reads() {
    with_process_host_observe_fixture(&mut |fixture: ProcessHostObserveFixture<'_>| {
        let ProcessHostObserveFixture {
            input,
            admission,
            signing_authority,
            custody,
            sessions,
            anchor,
        } = fixture;
        let selected_store = input.recovery.operation.context.store.clone();
        let locator = input.recovery.operation.expected_handle.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let application_port = MemoryAuthorityStore::default();
        let mut host = host_for_observe(&selector, &selected_store, &application_port);
        let mut authority = ProvingAuthority;
        let request_bytes = canonical_request(input.request);
        let signature_calls = Cell::new(0);

        with_request_context(input.request, None, |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &request_bytes,
                    request_bytes.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Err(ResidentObserveReplayAdmissionError::ChangedBytes)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObserveRequestError::ReplayAdmission(
                    ResidentObserveReplayAdmissionError::ChangedBytes
                ))
            ));
        });

        let LocalOperationObservationAdmission {
            recovery,
            authorize,
        } = admission;
        let mut verify = verify_fixture_signature(&signature_calls);
        let mut admit_replay =
            |_: &ResidentRequestIdV1, _: &CommonRequestEnvelopeDigest, _: &[u8]| {
                Ok(ResidentObserveReplayDisposition::FirstAdmission)
            };
        let observed = with_request_context(input.request, None, |request_context| {
            host.observe_request(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &request_bytes,
                    request_bytes.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                ResidentProcessObserveRequestAdmission {
                    request: request_context,
                    verify_caller_signature: &mut verify,
                    admit_replay: &mut admit_replay,
                    recovery,
                    authorize_disclosure: authorize,
                },
            )
        })
        .expect("replay rejection left protected recovery untouched");
        assert_eq!(signature_calls.get(), 2);
        Ok(observed)
    });
}

#[test]
fn denied_disclosure_preserves_the_existing_observe_failure() {
    with_denied_process_host_observe_fixture(&mut |fixture: ProcessHostObserveFixture<'_>| {
        let ProcessHostObserveFixture {
            input,
            admission,
            signing_authority,
            custody,
            sessions,
            anchor,
        } = fixture;
        let selected_store = input.recovery.operation.context.store.clone();
        let locator = input.recovery.operation.expected_handle.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let application_port = MemoryAuthorityStore::default();
        let mut host = host_for_observe(&selector, &selected_store, &application_port);
        let mut authority = ProvingAuthority;
        let request_bytes = canonical_request(input.request);
        let signature_calls = Cell::new(0);
        let mut verify_caller_signature = verify_fixture_signature(&signature_calls);
        let mut admit_replay =
            |_: &ResidentRequestIdV1, _: &CommonRequestEnvelopeDigest, _: &[u8]| {
                Ok(ResidentObserveReplayDisposition::FirstAdmission)
            };
        let LocalOperationObservationAdmission {
            recovery,
            authorize,
        } = admission;

        let result = with_request_context(input.request, None, |request_context| {
            host.observe_request(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &request_bytes,
                    request_bytes.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                ResidentProcessObserveRequestAdmission {
                    request: request_context,
                    verify_caller_signature: &mut verify_caller_signature,
                    admit_replay: &mut admit_replay,
                    recovery,
                    authorize_disclosure: authorize,
                },
            )
        });
        assert_eq!(signature_calls.get(), 1);
        match result {
            Err(ResidentProcessObserveRequestError::Observation(
                ResidentProcessObservationError::Observe(failure),
            )) => Err(failure),
            Err(other) => panic!("denied disclosure changed failure owner: {other}"),
            Ok(_) => panic!("denied disclosure cannot return an observation"),
        }
    });
}

#[test]
fn malformed_or_substituted_fresh_inputs_fail_before_resident_reads() {
    with_process_host_observe_fixture(&mut |fixture: ProcessHostObserveFixture<'_>| {
        let ProcessHostObserveFixture {
            input,
            admission,
            signing_authority,
            custody,
            sessions,
            anchor,
        } = fixture;
        let selected_store = input.recovery.operation.context.store.clone();
        let locator = input.recovery.operation.expected_handle.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let application_port = MemoryAuthorityStore::default();
        let mut host = host_for_observe(&selector, &selected_store, &application_port);
        let mut authority = ProvingAuthority;
        let exact_bytes = canonical_request(input.request);
        let signature_calls = Cell::new(0);

        let foreign_locator = ControlRecordCoordinate::new(
            ResidentRecordRefV1::parse("operations/foreign-handle")
                .expect("foreign locator reference"),
            ControlRecordDigest::parse("f".repeat(64)).expect("foreign locator digest"),
        );
        let foreign_observation_ref = ResidentRecordRefV1::parse("observations/foreign")
            .expect("foreign observation reference");
        let foreign_observation_digest =
            OperationObservationDigest::parse("e".repeat(64)).expect("foreign observation digest");

        with_request_context(input.request, None, |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &exact_bytes,
                    exact_bytes.len() - 1,
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObserveRequestError::RequestBounds)
            ));
        });

        let pretty = serde_json::to_string_pretty(&request_value(input.request))
            .expect("request has a pretty representation")
            .into_bytes();
        with_request_context(input.request, None, |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &pretty,
                    pretty.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObserveRequestError::RequestNotCanonical)
            ));
        });

        let mut wrong_schema = request_value(input.request);
        wrong_schema["envelope_schema_id"] = serde_json::json!("pale/operation-handle.v1");
        let wrong_schema = canonical_value(&wrong_schema);
        with_request_context(input.request, None, |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &wrong_schema,
                    wrong_schema.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObserveRequestError::RequestContract(_))
            ));
        });

        let mut wrong_digest = request_value(input.request);
        wrong_digest["canonical_request_sha256"] = serde_json::json!("0".repeat(64));
        let wrong_digest = canonical_value(&wrong_digest);
        with_request_context(input.request, None, |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &wrong_digest,
                    wrong_digest.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObserveRequestError::RequestAdmission(
                    InvalidResidentRecordV1::DigestMismatch { .. }
                ))
            ));
        });

        let foreign_capability =
            CapabilityIdV1::parse("operation.cancel").expect("foreign capability remains bounded");
        with_request_context(input.request, Some(&foreign_capability), |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &exact_bytes,
                    exact_bytes.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObserveRequestError::RequestAdmission(
                    InvalidResidentRecordV1::ContextualAdmission { .. }
                ))
            ));
        });

        let mut wrong_signature = request_value(input.request);
        wrong_signature["caller_signature"] = serde_json::json!(format!("{}Q", "B".repeat(85)));
        let wrong_signature = canonical_value(&wrong_signature);
        let calls_before_signature = signature_calls.get();
        with_request_context(input.request, None, |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &wrong_signature,
                    wrong_signature.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(
                matches!(
                    result,
                    Err(ResidentProcessObserveRequestError::RequestAdmission(
                        InvalidResidentRecordV1::ContextualAdmission { .. }
                    ))
                ),
                "unexpected signature rejection: {result:?}"
            );
        });
        assert_eq!(
            signature_calls.get(),
            calls_before_signature + 1,
            "a contract-valid request must reach caller-signature verification"
        );

        for (candidate_locator, selection) in [
            (
                &foreign_locator,
                OperationObservationSelection::SelectedHead,
            ),
            (
                &locator,
                OperationObservationSelection::HistoryMember {
                    reference: &foreign_observation_ref,
                    digest: &foreign_observation_digest,
                },
            ),
        ] {
            with_request_context(input.request, None, |context| {
                let mut verify = verify_fixture_signature(&signature_calls);
                let result = observe_with_unreachable_recovery(
                    &mut host,
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody,
                        sessions,
                        anchor,
                    },
                    &mut authority,
                    observe_request_input(
                        &input,
                        &exact_bytes,
                        exact_bytes.len(),
                        candidate_locator,
                        &selected_store,
                        signing_authority,
                        selection,
                    ),
                    context,
                    UnreachableRecoveryRequestAdmission {
                        replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                        verify_caller_signature: &mut verify,
                    },
                );
                assert!(matches!(
                    result,
                    Err(ResidentProcessObserveRequestError::LocatorAssociation)
                ));
            });
        }

        for authorized_at in [
            UnixTimeV1::new(
                input
                    .request
                    .issued_at()
                    .get()
                    .checked_sub(1)
                    .expect("fixture issue time has a predecessor"),
            )
            .expect("pre-issue time remains valid"),
            UnixTimeV1::new(
                input
                    .request
                    .expires_at()
                    .get()
                    .checked_add(1)
                    .expect("fixture expiry has a successor"),
            )
            .expect("post-expiry time remains valid"),
        ] {
            with_request_context(input.request, None, |context| {
                let mut verify = verify_fixture_signature(&signature_calls);
                let mut request_input = observe_request_input(
                    &input,
                    &exact_bytes,
                    exact_bytes.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                );
                request_input.authorized_at = authorized_at;
                let result = observe_with_unreachable_recovery(
                    &mut host,
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &selected_store,
                        custody,
                        sessions,
                        anchor,
                    },
                    &mut authority,
                    request_input,
                    context,
                    UnreachableRecoveryRequestAdmission {
                        replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                        verify_caller_signature: &mut verify,
                    },
                );
                assert!(matches!(
                    result,
                    Err(ResidentProcessObserveRequestError::Observation(
                        ResidentProcessObservationError::Observe(
                            LocalOperationObservationError::Authorization(
                                OperationControlError::Time
                            )
                        )
                    ))
                ));
            });
        }

        let mut wrong_body = request_value(input.request);
        let mut handle = wrong_body["body"]["payload"].clone();
        handle["operation_handle_id"] = serde_json::json!("operation-substituted");
        let handle = reseal_handle(handle);
        wrong_body["body"]["payload_sha256"] =
            serde_json::json!(ArtifactDigest::hash_bytes(&canonical_value(&handle))
                .as_str()
                .to_owned());
        wrong_body["body"]["payload"] = handle;
        let wrong_body = reseal_request(wrong_body);
        with_request_context(input.request, None, |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &wrong_body,
                    wrong_body.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObserveRequestError::LocatorAssociation)
            ));
        });

        let foreign_store = AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("foreign-observe-store").expect("foreign store"),
            ControlIncarnationIdV1::parse("foreign-observe-incarnation")
                .expect("foreign incarnation"),
        );
        with_request_context(input.request, None, |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &exact_bytes,
                    exact_bytes.len(),
                    &locator,
                    &foreign_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObserveRequestError::Observation(
                    ResidentProcessObservationError::Selection(
                        ResidentProcessHostSelectionError::RecoveryStoreAssociation
                    )
                ))
            ));
        });

        let foreign_selector = authority_axis_substitutions(&selector)
            .pop()
            .expect("authority substitution exists");
        let foreign_authority = integrated_signing_authority(&foreign_selector);
        let foreign_request = request_with_signing_authority(input.request, foreign_authority);
        let foreign_request_bytes = canonical_request(&foreign_request);
        with_request_context(&foreign_request, None, |context| {
            let mut verify = verify_fixture_signature(&signature_calls);
            let result = observe_with_unreachable_recovery(
                &mut host,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &foreign_request_bytes,
                    foreign_request_bytes.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                context,
                UnreachableRecoveryRequestAdmission {
                    replay_result: Some(Ok(ResidentObserveReplayDisposition::FirstAdmission)),
                    verify_caller_signature: &mut verify,
                },
            );
            assert!(matches!(
                result,
                Err(ResidentProcessObserveRequestError::Observation(
                    ResidentProcessObservationError::Selection(
                        ResidentProcessHostSelectionError::CurrentRequestAuthority
                    )
                ))
            ));
        });

        let LocalOperationObservationAdmission {
            recovery,
            authorize,
        } = admission;
        let mut verify = verify_fixture_signature(&signature_calls);
        let mut admit_replay =
            |_: &ResidentRequestIdV1, _: &CommonRequestEnvelopeDigest, _: &[u8]| {
                Ok(ResidentObserveReplayDisposition::FirstAdmission)
            };
        let observed = with_request_context(input.request, None, |context| {
            host.observe_request(
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                &mut authority,
                observe_request_input(
                    &input,
                    &exact_bytes,
                    exact_bytes.len(),
                    &locator,
                    &selected_store,
                    signing_authority,
                    OperationObservationSelection::SelectedHead,
                ),
                ResidentProcessObserveRequestAdmission {
                    request: context,
                    verify_caller_signature: &mut verify,
                    admit_replay: &mut admit_replay,
                    recovery,
                    authorize_disclosure: authorize,
                },
            )
        })
        .expect("exact request still owns the only resident read");
        Ok(observed)
    });
}
