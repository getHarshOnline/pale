//! Recover an actually issued fresh child through a separately composed service.
//! Signatures, session authority, time and the independent anchor are fixtures.

use super::super::super::request;
use super::super::super::{
    restart_test_support::serve_operation_frame, OperationConformanceRequestFactory,
};
use super::*;
use std::rc::Rc;

#[path = "interface_consult_child_owners.rs"]
mod physical;
use physical::*;

#[path = "interface_consult_child_anchor.rs"]
mod anchor;
use anchor::*;

fn child_request(parent: &CommonRequestEnvelopeV1) -> CommonRequestEnvelopeV1 {
    let mut preimage = parent.to_preimage();
    preimage.request_id = ResidentRequestIdV1::parse("interface-consult-fresh-child").unwrap();
    preimage.idempotency_key =
        RequestIdempotencyKeyV1::parse("interface-consult-fresh-child").unwrap();
    preimage.scope.purpose = BoundedResidentTextV1::parse("consult-child").unwrap();
    let digest = CommonRequestEnvelopeDigest::hash_canonical(&preimage).unwrap();
    let mut value = serde_json::to_value(preimage).unwrap();
    value["canonical_request_sha256"] = serde_json::to_value(digest).unwrap();
    value["caller_signature"] = serde_json::to_value(signature()).unwrap();
    admit_historical_request(
        &record_ref("child-request"),
        serde_json::from_value(value).unwrap(),
    )
    .unwrap()
}

/// Exercise real parent/child issuance and cold recovery over caller-owned custody.
/// The lender is called once per service and may reopen SQLite between calls.
/// No child dispatch, completion, late contribution or parent acceptance is claimed.
///
/// # Panics
/// Panics if issuance, exact replay, child identity or once-only work checks fail.
#[doc(hidden)]
pub fn exercise_consult_child_service_restart(
    with_custody: impl FnMut(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &DeploymentBindingV1,
        &mut dyn FnMut(&mut dyn OperationCustodyPort),
    ),
) {
    exercise_child_restart(with_custody, ChildRestartBoundary::Collecting);
}

/// Reopen the service after child issuance but before initial collection custody.
/// The original signed parent and child must survive collection opening unchanged.
///
/// # Panics
/// Panics if the interruption, exact recovery or once-only issuance checks fail.
#[doc(hidden)]
pub fn exercise_consult_child_prefix_service_restart(
    with_custody: impl FnMut(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &DeploymentBindingV1,
        &mut dyn FnMut(&mut dyn OperationCustodyPort),
    ),
) {
    exercise_child_restart(with_custody, ChildRestartBoundary::BeforeCollection);
}

/// Refuse to infer unsigned state from an absent child handle after restart.
///
/// # Panics
/// Panics if recovery replaces the plan, repeats prior work or infers signature absence.
#[doc(hidden)]
pub fn exercise_consult_child_suffix_service_restart(
    with_custody: impl FnMut(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &DeploymentBindingV1,
        &mut dyn FnMut(&mut dyn OperationCustodyPort),
    ),
) {
    exercise_child_restart(with_custody, ChildRestartBoundary::BeforeChild);
}

/// Refuse an absent child whose signature may already have been issued.
///
/// # Panics
/// Panics if cold recovery signs the child again or treats absent custody as unsigned.
#[doc(hidden)]
pub fn exercise_consult_child_ambiguous_restart(
    with_custody: impl FnMut(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &DeploymentBindingV1,
        &mut dyn FnMut(&mut dyn OperationCustodyPort),
    ),
) {
    exercise_child_restart(with_custody, ChildRestartBoundary::AfterChildSignature);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChildRestartBoundary {
    Collecting,
    BeforeCollection,
    BeforeChild,
    AfterChildSignature,
    AfterCollectionCommit,
}

/// Recover committed collection custody after its independent anchor update failed.
///
/// # Panics
/// Panics if recovery replaces records, repeats issuance or fails to repair the anchor once.
#[doc(hidden)]
pub fn exercise_consult_child_collection_anchor_restart(
    with_custody: impl FnMut(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &DeploymentBindingV1,
        &mut dyn FnMut(&mut dyn OperationCustodyPort),
    ),
) {
    exercise_child_restart(with_custody, ChildRestartBoundary::AfterCollectionCommit);
}

fn exercise_child_restart(
    mut with_custody: impl FnMut(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &DeploymentBindingV1,
        &mut dyn FnMut(&mut dyn OperationCustodyPort),
    ),
    boundary: ChildRestartBoundary,
) {
    let interrupt_before_collection = boundary == ChildRestartBoundary::BeforeCollection;
    let interrupt_before_child = matches!(
        boundary,
        ChildRestartBoundary::BeforeChild | ChildRestartBoundary::AfterChildSignature
    );
    let mut state = ConformanceResidentState::prepare();
    let mut sources = [
        state.take_generation_source(),
        state.take_generation_source(),
    ];
    let mut anchors = ConformanceAntiRollbackAnchors::default();
    let counters = Arc::new(ConsultConformanceCounters::default());
    let handles = Rc::new(RefCell::new(Vec::new()));
    let saved: SavedCollection = Rc::new(RefCell::new(None));
    let interrupted: InterruptedParent = Rc::new(RefCell::new(None));
    let interrupted_child: InterruptedChild = Rc::new(RefCell::new(None));
    let interrupted_transition: InterruptedTransition = Rc::new(RefCell::new(None));
    let historical_mode = Rc::new(std::cell::Cell::new(HistoricalAdmissionMode::Missing));
    let historical_attempts = Rc::new(std::cell::Cell::new(0));
    let mut committed_collection = None;
    let mut original_child = None;
    state.session.with_admitted_session(
        state.generation.record(), &state.profile, &state.lifecycle.active, &state.lifecycle.fence, &state.trust_ref, &state.trust,
        |session, current, lease| {
            let start = request::start_request(session, current, lease);
            let child = child_request(&start);
            let requests = OperationConformanceRequestFactory {
                start_signed_common_request: pale_core::canon::to_canonical_string(&start).unwrap(),
                activation_fence: session.lease().activation_fence_digest().clone(),
            };
            let advance = request::phase_request(&start, ConformanceRequestPhase::Advance, None, "interface-conformance-consult-advance-policy", request::ConformanceRequestFreshness::Current);
            let frame = pale_core::canon::to_canonical_string(&ResidentOperationFrameRequest::StartConsult {
                signed_common_request: requests.start_signed_common_request().to_owned(),
            }).unwrap();
            let mut original = None;
            for (round, source) in sources.iter_mut().enumerate() {
                let mut calls = 0;
                let mut run = |custody: &mut dyn OperationCustodyPort| {
                    calls += 1;
                    let mut events = UnavailableEventCustody;
                    let advances = anchors.advances;
                    let result: Result<(), OperationConformanceFrontierError> = (|| {
                        let mut anchor = CollectionAnchor {
                            base: &mut anchors,
                            interrupt: boundary == ChildRestartBoundary::AfterCollectionCommit && round == 0,
                            collection: Rc::clone(&saved),
                            parent: Rc::clone(&interrupted),
                            transition: Rc::clone(&interrupted_transition),
                        };
                        with_cold_recovered_process!(&state, source, &mut events, custody, &mut anchor, |process| {
                            let repository = session.lease().repository_id().clone();
                            let policies = ResidentServiceOperationRequestPolicies {
                                start: vec![service_policy(&state.generation, repository.clone(), &child)], advance: Vec::new(), start_delegate: Vec::new(), advance_delegate: Vec::new(), cancel: Vec::new(),
                                start_consult: vec![service_policy(&state.generation, repository.clone(), &start)],
                                advance_consult: vec![service_policy(&state.generation, repository.clone(), &advance)],
                                observe: vec![observe_service_policy(&state.generation, repository, &start, session.lease().activation_fence_digest())],
                            };
                            let mut consult_inputs = ChildInputs::new(ConformanceConsultInputs::new(&state.store, &state.session, &start, Arc::clone(&counters)), Rc::clone(&handles), Rc::clone(&saved));
                            if boundary == ChildRestartBoundary::AfterCollectionCommit && round == 1 {
                                consult_inputs = consult_inputs.with_historical_collection(Rc::clone(&saved), Rc::clone(&historical_mode), Rc::clone(&historical_attempts));
                            }
                            if interrupt_before_collection && round == 0 {
                                consult_inputs = consult_inputs.interrupt_before_collection(Rc::clone(&interrupted));
                            }
                            let owners = ResidentServiceInvokeConsultObserveCancelOwners {
                                peer_admission: Box::new(ConformancePeerAdmission(state.expected_peer)),
                                maximum_request_bytes: MAXIMUM_RECORD_BYTES,
                                request_policies: policies,
                                historical: &state.historical,
                                evidence: ConformanceEvidence::new(current_evidence(current)),
                                clock: ConformanceConsultClock { counters: Arc::clone(&counters) },
                                session_authority: proving_authority(),
                                caller_signature: ConformanceCallerSignature,
                                random: ConformanceEntropy::for_consult(Arc::clone(&counters)),
                                operation_authority: proving_authority(),
                                policy: ChildPolicy { base: consult_policy(lease, &start, Arc::clone(&counters)), request: child.clone(), start: invoke_policy(lease, &child) },
                                cryptography: ChildCryptography {
                                    template: checkpoint_template(&state.store, SupportedOperationCheckpointProfile::conflict_domain(&start).unwrap(), &state.session),
                                    handles: Rc::clone(&handles), counters: Arc::clone(&counters),
                                    interrupt_before_child: (boundary == ChildRestartBoundary::BeforeChild && round == 0).then(|| Rc::clone(&interrupted_child)),
                                    interrupt_after_child_signature: (boundary == ChildRestartBoundary::AfterChildSignature && round == 0).then(|| Rc::clone(&interrupted_child)),
                                    signing_available: boundary != ChildRestartBoundary::AfterCollectionCommit || round == 0,
                                },
                                invoke_inputs: ChildRecoveryInputs { base: ConformanceInvokeInputs::new_for_consult(&state.store, &state.historical, &state.session, &start).with_background_at(conformance_ingress_time()), handles: Rc::clone(&handles) },
                                consult_inputs,
                                cancel_inputs: UnavailableCancellationInputs,
                                observe_inputs: UnavailableObserveInputs,
                            };
                            let mut service = ResidentFrameService::recover_invoke_consult_observe_cancel(process, owners)?;
                            let mut scheduler = ResidentWorkTurnCoordinator::new();
                            let mut started = None;
                            let mut admitted_history_count = None;
                            if boundary == ChildRestartBoundary::AfterCollectionCommit && round == 1 {
                                for mode in [HistoricalAdmissionMode::Missing, HistoricalAdmissionMode::Refused] {
                                    historical_mode.set(mode);
                                    for _ in 0..8 {
                                        assert_eq!(serve_operation_frame(&mut service, frame.as_bytes()), ResidentOperationFrameResponse::Rejected { failure: ResidentOperationFrameFailure::ResidentUnavailable }, "missing or refused historical collection admission cannot release Start");
                                        scheduler.run_one(&mut service, ResidentWorkTurnInput::idle().with_recovery()).unwrap();
                                    }
                                    if mode == HistoricalAdmissionMode::Missing { assert_eq!(historical_attempts.get(), 0); }
                                    else { assert!(historical_attempts.get() > 0, "the historical owner actually rejected the stored collection"); }
                                }
                                let attempts = historical_attempts.get();
                                historical_mode.set(HistoricalAdmissionMode::Allowed);
                                for _ in 0..8 {
                                    scheduler.run_one(&mut service, ResidentWorkTurnInput::idle().with_recovery()).unwrap();
                                }
                                assert_eq!(historical_attempts.get(), attempts + 1, "the repaired anchor needs no repeated historical collection admission");
                                admitted_history_count = Some(historical_attempts.get());
                            }
                            for _ in 0..32 {
                                match serve_operation_frame(&mut service, frame.as_bytes()) {
                                    ResidentOperationFrameResponse::Started { records } => { started = Some(records); break; }
                                    ResidentOperationFrameResponse::Rejected { failure: ResidentOperationFrameFailure::ResidentUnavailable } if round == 0 || boundary != ChildRestartBoundary::Collecting => {},
                                    other => panic!("round {round} cannot resume original child start: {other:?}"),
                                }
                                if round == 0 && interrupted.borrow().is_some() { break; }
                                scheduler.run_one(&mut service, ResidentWorkTurnInput::idle()).unwrap();
                                if round == 0 {
                                    if let Some(child) = interrupted_child.borrow().as_ref() {
                                        let parent = child.parent_operation.as_ref().unwrap();
                                        *interrupted.borrow_mut() = Some(ControlRecordCoordinate::new(parent.operation_handle_ref.clone(), ControlRecordDigest::parse(parent.operation_handle_sha256.as_str().to_owned()).unwrap()));
                                    }
                                }
                                if round == 0 && interrupted.borrow().is_some() { break; }
                            }
                            if boundary != ChildRestartBoundary::Collecting && round == 0 {
                                assert!(interrupted.borrow().is_some(), "the actual service reaches the selected interruption");
                                assert!(started.is_none(), "Start has not completed at the interruption");
                                assert_eq!(saved.borrow().is_some(), boundary == ChildRestartBoundary::AfterCollectionCommit);
                                assert_eq!(handles.borrow().len(), if boundary == ChildRestartBoundary::BeforeChild { 1 } else { 2 });
                                assert_eq!(counters.initial_collection.load(Ordering::SeqCst), usize::from(boundary == ChildRestartBoundary::AfterCollectionCommit));
                                return Ok(());
                            }
                            if interrupt_before_child {
                                assert!(started.is_none(), "an absent handle cannot establish that signing never ran");
                                assert!(saved.borrow().is_none());
                                assert_eq!(counters.handle_signatures.load(Ordering::SeqCst), if boundary == ChildRestartBoundary::BeforeChild { 1 } else { 2 });
                                return Ok(());
                            }
                            let records = started.unwrap_or_else(|| panic!("bounded child start stopped with {} handles, {} initial collections and {} identity fills", handles.borrow().len(), counters.initial_collection.load(Ordering::SeqCst), counters.identity_entropy_fills.load(Ordering::SeqCst)));
                            assert!(saved.borrow().is_some(), "the real initial collection admitted the signed child");
                            if round == 0 {
                                original = Some(records);
                            } else {
                                assert_eq!(Some(&records), original.as_ref(), "cold replay returns the original parent records");
                                assert_eq!(serve_operation_frame(&mut service, frame.as_bytes()), ResidentOperationFrameResponse::Started { records: records.clone() }, "repeating Start after collection opening returns the same records");
                                let handle = pale_core::canon::parse_strict(records.operation_handle_canonical().as_bytes()).unwrap();
                                let locator = ResidentOperationFrameLocator::new(records.operation_handle_record_ref().to_owned(), handle["canonical_operation_handle_sha256"].as_str().unwrap().to_owned());
                                let advance = pale_core::canon::to_canonical_string(&ResidentOperationFrameRequest::AdvanceConsult { signed_common_request: requests.advance_signed_common_request(&locator).unwrap(), locator: locator.clone() }).unwrap();
                                assert_eq!(serve_operation_frame(&mut service, advance.as_bytes()), ResidentOperationFrameResponse::Pending { locator });
                                for _ in 0..32 {
                                    scheduler.run_one(&mut service, ResidentWorkTurnInput::idle()).unwrap();
                                    if counters.completion_policy.load(Ordering::SeqCst) > 0 { break; }
                                }
                                assert_eq!(counters.completion_policy.load(Ordering::SeqCst), 1, "cold recovery reaches policy with the exact saved child");
                                assert!(service.has_retained_phase());
                            }
                            if let Some(count) = admitted_history_count { assert_eq!(historical_attempts.get(), count); }
                            Ok(())
                        })
                    })();
                    result.expect("the actual Consult service composes over retained custody");
                    if boundary == ChildRestartBoundary::AfterCollectionCommit {
                        let transition = interrupted_transition.borrow();
                        let transition = transition.as_ref().expect("the service attempted the independent anchor update");
                        assert_eq!(anchors.observe(transition).unwrap(), if round == 0 { AntiRollbackAnchorObservation::ExactPredecessor } else { AntiRollbackAnchorObservation::ExactSuccessor });
                        let snapshot = snapshot_committed_collection(&state.store, &start, saved.borrow().as_ref().unwrap(), custody);
                        if round == 0 {
                            original = Some(snapshot.0.clone());
                            committed_collection = Some(snapshot);
                        } else {
                            assert_eq!(Some(&snapshot), committed_collection.as_ref(), "recovery preserves the committed parent, child and collection bytes");
                        }
                    } else if let Some(parent) = interrupted.borrow().as_ref() {
                        let (records, child) = snapshot_child_prefix(&state, &start, &handles, parent, custody, &mut anchors);
                        if round == 0 {
                            assert_eq!(child.is_none(), interrupt_before_child, "only the interrupted child-start cases lack a child record");
                            original = Some(records);
                            original_child = Some(child);
                        } else {
                            assert_eq!(Some(&records), original.as_ref());
                            if interrupt_before_child {
                                assert!(child.is_none());
                            } else {
                                assert_eq!(Some(&child), original_child.as_ref(), "the original child record survives collection opening unchanged");
                            }
                        }
                    }
                    if round == 1 { assert_eq!(anchors.advances, advances + usize::from(interrupt_before_collection || boundary == ChildRestartBoundary::AfterCollectionCommit), "only collection opening or its interrupted anchor update may advance the checkpoint"); }
                };
                with_custody(&state.store, &state.session.binding_ref, &state.session.binding, &mut run);
                assert_eq!(calls, 1);
            }
            assert_eq!(handles.borrow().len(), if boundary == ChildRestartBoundary::BeforeChild { 1 } else { 2 });
            assert_eq!(counters.start_policy.load(Ordering::SeqCst), 1);
            assert_eq!(counters.initial_collection.load(Ordering::SeqCst), usize::from(!interrupt_before_child));
            assert_eq!(counters.handle_signatures.load(Ordering::SeqCst), if boundary == ChildRestartBoundary::BeforeChild { 1 } else { 2 });
            assert_eq!(counters.identity_entropy_fills.load(Ordering::SeqCst), 14);
            assert_eq!(counters.child_stop_join_or_acceptance_attempts.load(Ordering::SeqCst), 0);
        },
    );
}
