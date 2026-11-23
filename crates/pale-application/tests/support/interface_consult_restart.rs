//! Shared service-restart journey over caller-supplied operation custody.
//! Authority, signatures, clock and independent anchor remain explicit models.

use super::*;
use std::{
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

// The scheduler is bounded, but storage and policy lenders can defer a
// turn. The test therefore proves eventual recovery rather than coupling
// the public frame journey to the current number of internal phases.
pub(super) const CONSULT_INITIAL_FRONTIER_ATTEMPTS: usize = 18;
const TEST_PEER: LocalIpcPeerCredentials = LocalIpcPeerCredentials::new(501, 20, Some(8_081));

struct OneFrameCarrier {
    inbound: Option<Box<[u8]>>,
    response: Vec<u8>,
}

impl OneFrameCarrier {
    fn new(frame: &[u8]) -> Self {
        Self {
            inbound: Some(frame.to_vec().into_boxed_slice()),
            response: Vec::new(),
        }
    }
}

impl LocalIpcFramePort for OneFrameCarrier {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        Ok(TEST_PEER)
    }

    fn send_frame(&mut self, payload: &[u8], _: Instant) -> Result<(), LocalIpcCarrierError> {
        self.response.extend_from_slice(payload);
        Ok(())
    }

    fn receive_frame(&mut self, _: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        Ok(self.inbound.take().expect("one request frame"))
    }
}

pub(super) fn serve_operation_frame(
    service: &mut ResidentFrameService<'_>,
    frame: &[u8],
) -> ResidentOperationFrameResponse {
    let mut carrier = OneFrameCarrier::new(frame);
    let deadline = Instant::now() + Duration::from_secs(1);
    assert_eq!(
        service
            .serve_frame(&mut carrier, deadline, deadline)
            .expect("the resident serves one admitted operation frame"),
        crate::resident::ResidentFrameOutcome::Operation,
    );
    let value = canon::parse_strict(&carrier.response).expect("resident response is canonical");
    serde_json::from_value(value).expect("resident response has the operation-frame shape")
}

/// Exercise two fresh services against the same retained operation store.
///
/// The lender is called once for each service and may reopen the store between
/// calls. The journey checks exact Start replay and collection continuation
/// without repeating start policy, identity allocation or handle signing.
/// Authority recovery uses the existing in-memory conformance evidence.
///
/// # Panics
/// Panics if either service cannot start or resume the exact consultation, if
/// the custody lender skips or repeats a service, or if an invariant fails.
#[doc(hidden)]
pub fn exercise_consult_service_restart(
    with_custody: impl FnMut(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &DeploymentBindingV1,
        &mut dyn FnMut(&mut dyn OperationCustodyPort),
    ),
) {
    exercise_consult_restart(with_custody, false);
}

/// Recover a committed expiry through the public service's background turn.
/// The first service creates the actual start and collection. The existing stop
/// owner then commits expiry with the independent anchor unavailable. The next
/// service must repair that exact boundary without receiving a request frame.
///
/// # Panics
/// Panics when a custody lender or an exact recovery assertion fails.
#[doc(hidden)]
pub fn exercise_consult_service_expiry_restart(
    with_custody: impl FnMut(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &DeploymentBindingV1,
        &mut dyn FnMut(&mut dyn OperationCustodyPort),
    ),
) {
    exercise_consult_restart(with_custody, true);
}

fn exercise_consult_restart(
    mut with_custody: impl FnMut(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &DeploymentBindingV1,
        &mut dyn FnMut(&mut dyn OperationCustodyPort),
    ),
    recover_expiry: bool,
) {
    let mut state = ConformanceResidentState::prepare();
    let mut anchors = ConformanceAntiRollbackAnchors::default();
    let mut sources = [
        state.take_generation_source(),
        state.take_generation_source(),
    ];
    let counters = Arc::new(ConsultConformanceCounters::default());

    state.session.with_admitted_session(
        state.generation.record(),
        &state.profile,
        &state.lifecycle.active,
        &state.lifecycle.fence,
        &state.trust_ref,
        &state.trust,
        |session, current, lease| {
            let start = start_request(session, current, lease);
            let requests = OperationConformanceRequestFactory {
                start_signed_common_request: canon::to_canonical_string(&start).unwrap(),
                activation_fence: session.lease().activation_fence_digest().clone(),
            };
            let advance = phase_request(
                &start,
                ConformanceRequestPhase::Advance,
                None,
                "interface-conformance-consult-advance-policy",
                ConformanceRequestFreshness::Current,
            );
            let repository = session.lease().repository_id().clone();
            let policies = ConformanceConsultRequestPolicies::new(
                service_policy(&state.generation, repository.clone(), &start),
                service_policy(&state.generation, repository.clone(), &advance),
                observe_service_policy(
                    &state.generation,
                    repository,
                    &start,
                    session.lease().activation_fence_digest(),
                ),
            );
            let exact_frame =
                canon::to_canonical_string(&ResidentOperationFrameRequest::StartConsult {
                    signed_common_request: requests.start_signed_common_request().to_owned(),
                })
                .unwrap()
                .into_bytes();
            let mut original = None;
            let mut expiry: Option<(ConsultationCollection, ConsultationCollection, UnixTimeV1)> =
                None;
            for (round, source) in sources.iter_mut().enumerate() {
                let mut events = UnavailableEventCustody;
                let mut calls = 0;
                let mut run_service = |custody: &mut dyn OperationCustodyPort| {
                    calls += 1;
                    let mut invoke_inputs = ConformanceInvokeInputs::new_for_consult(
                        &state.store,
                        &state.historical,
                        &state.session,
                        &start,
                    );
                    let mut consult_inputs = ConformanceConsultInputs::new(
                        &state.store,
                        &state.session,
                        &start,
                        Arc::clone(&counters),
                    );
                    if let Some((previous, stopped, at)) = &expiry {
                        invoke_inputs = invoke_inputs.with_background_at(*at);
                        consult_inputs = consult_inputs
                            .expect_expiry_recovery(previous.clone(), stopped.clone());
                    }
                    let advances = anchors.advances;
                    with_cold_recovered_consult_observe_service(
                        &state,
                        source,
                        &mut events,
                        custody,
                        &mut anchors,
                        current_evidence(current),
                        policies.clone(),
                        consult_policy(lease, &start, Arc::clone(&counters)),
                        ConformanceCryptography::new_for_consult(
                            &state.store,
                            &start,
                            &state.session,
                            Arc::clone(&counters),
                        ),
                        invoke_inputs,
                        consult_inputs,
                        UnavailableObserveInputs,
                        |service| {
                            let mut scheduler = ResidentWorkTurnCoordinator::new();
                            if round == 1 && recover_expiry {
                                for _ in 0..CONSULT_INITIAL_FRONTIER_ATTEMPTS {
                                    let outcome = scheduler
                                        .run_one(
                                            service,
                                            ResidentWorkTurnInput::idle().with_recovery(),
                                        )
                                        .unwrap();
                                    if counters.current_stopped_collections.load(Ordering::SeqCst)
                                        > 0
                                    {
                                        assert_eq!(
                                            outcome,
                                            ResidentWorkTurnOutcome::RecoveryAdvanced
                                        );
                                        break;
                                    }
                                }
                                assert_eq!(
                                    counters.current_stopped_collections.load(Ordering::SeqCst),
                                    1
                                );
                                assert_eq!(
                                    counters.historical_previous_collections.load(Ordering::SeqCst),
                                    1,
                                    "historical admission must visit the previous collection exactly once"
                                );
                                assert_eq!(
                                    counters.historical_stopped_collections.load(Ordering::SeqCst),
                                    1,
                                    "historical admission must visit the stopped collection exactly once"
                                );
                                assert_eq!(counters.executor_polls.load(Ordering::SeqCst), 1);
                                assert_eq!(counters.completion_policy.load(Ordering::SeqCst), 0);
                                assert!(
                                    !service.has_retained_phase(),
                                    "the established stop must not open another continuation"
                                );
                                for _ in 0..CONSULT_INITIAL_FRONTIER_ATTEMPTS {
                                    scheduler
                                        .run_one(
                                            service,
                                            ResidentWorkTurnInput::idle().with_recovery(),
                                        )
                                        .unwrap();
                                }
                                assert_eq!(
                                    counters.current_stopped_collections.load(Ordering::SeqCst),
                                    1
                                );
                                assert_eq!(
                                    counters.historical_previous_collections.load(Ordering::SeqCst),
                                    1,
                                    "historical admission must visit the previous collection exactly once"
                                );
                                assert_eq!(
                                    counters.historical_stopped_collections.load(Ordering::SeqCst),
                                    1,
                                    "historical admission must visit the stopped collection exactly once"
                                );
                                assert_eq!(counters.executor_polls.load(Ordering::SeqCst), 1);
                                return;
                            }
                            let mut started = None;
                            for _ in 0..CONSULT_INITIAL_FRONTIER_ATTEMPTS {
                                match serve_operation_frame(service, &exact_frame) {
                                    ResidentOperationFrameResponse::Started { records } => {
                                        started = Some(records);
                                        break;
                                    }
                                    ResidentOperationFrameResponse::Rejected {
                                        failure: ResidentOperationFrameFailure::ResidentUnavailable,
                                    } if round == 0 => {}
                                    other => {
                                        panic!("round {round} cannot resume saved start: {other:?}")
                                    }
                                }
                                scheduler
                                    .run_one(service, ResidentWorkTurnInput::idle())
                                    .unwrap();
                            }
                            let records = started.expect("bounded start reaches custody");
                            if round == 0 {
                                original = Some(records);
                                return;
                            }
                            assert_eq!(
                                Some(&records),
                                original.as_ref(),
                                "cold replay returns the original signed records"
                            );
                            let handle = canon::parse_strict(
                                records.operation_handle_canonical().as_bytes(),
                            )
                            .unwrap();
                            let locator = ResidentOperationFrameLocator::new(
                                records.operation_handle_record_ref().to_owned(),
                                handle["canonical_operation_handle_sha256"]
                                    .as_str()
                                    .unwrap()
                                    .to_owned(),
                            );
                            let advance_frame = canon::to_canonical_string(
                                &ResidentOperationFrameRequest::AdvanceConsult {
                                    signed_common_request: requests
                                        .advance_signed_common_request(&locator)
                                        .unwrap(),
                                    locator: locator.clone(),
                                },
                            )
                            .unwrap();
                            let response = serve_operation_frame(service, advance_frame.as_bytes());
                            assert_eq!(
                                response,
                                ResidentOperationFrameResponse::Pending { locator },
                                "cold Advance must retain the original operation"
                            );
                            for _ in 0..CONSULT_INITIAL_FRONTIER_ATTEMPTS {
                                scheduler
                                    .run_one(service, ResidentWorkTurnInput::idle())
                                    .unwrap();
                                if counters.completion_policy.load(Ordering::SeqCst) > 0 {
                                    break;
                                }
                            }
                            assert!(
                                counters.completion_policy.load(Ordering::SeqCst) > 0,
                                "the recovered collection must reach its completion policy"
                            );
                            assert!(
                                service.has_retained_phase(),
                                "Continue(None) keeps the resumed collection"
                            );
                        },
                    )
                    .expect("actual service composition cold-opens the retained state");
                    if recover_expiry {
                        if round == 0 {
                            let records = original.as_ref().unwrap();
                            let handle = canon::parse_strict(
                                records.operation_handle_canonical().as_bytes(),
                            )
                            .unwrap();
                            let locator = ResidentOperationFrameLocator::new(
                                records.operation_handle_record_ref().to_owned(),
                                handle["canonical_operation_handle_sha256"]
                                    .as_str()
                                    .unwrap()
                                    .to_owned(),
                            );
                            expiry = Some(prepare_interrupted_consult_expiry(
                                &state,
                                &start,
                                &locator,
                                custody,
                                &mut anchors,
                            ));
                        } else {
                            assert_eq!(
                                anchors.advances,
                                advances + 1,
                                "background recovery advances only the missing anchor"
                            );
                        }
                    }
                };
                with_custody(
                    &state.store,
                    &state.session.binding_ref,
                    &state.session.binding,
                    &mut run_service,
                );
                assert_eq!(calls, 1, "each service must receive one custody view");
            }
            assert_eq!(counters.start_policy.load(Ordering::SeqCst), 1);
            assert_eq!(counters.initial_collection.load(Ordering::SeqCst), 1);
            assert_eq!(counters.handle_signatures.load(Ordering::SeqCst), 1);
            assert_eq!(counters.identity_entropy_fills.load(Ordering::SeqCst), 7);
            assert_eq!(
                counters
                    .child_stop_join_or_acceptance_attempts
                    .load(Ordering::SeqCst),
                0
            );
        },
    );
}
