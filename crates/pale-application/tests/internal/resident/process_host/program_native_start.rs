//! Actual frame admission binds a verified transport key before native custody.

use super::*;
use crate::resident::process_host::tests::invoke_request::{
    exact_invoke_policy, ExactInvokePolicy,
};

#[test]
fn native_hook_head_check_rejects_request_substitution_and_changed_custody() {
    use crate::resident::operation_observation_custody::tests::local_recovery::with_process_host_restart_fixtures;
    use crate::resident::process_host::program::native_hook_turn::verify_current_head;
    for missing_head in [true, false] {
        with_process_host_restart_fixtures(&mut move |case| {
            let maximum = case.recovery.operation.context.maximum_record_bytes;
            let recovered = crate::resident::recover_operation_control(
                case.recovery,
                case.custody,
                case.sessions,
                case.anchor,
                case.admission,
            )
            .unwrap();
            let operation = recovered.operation().operation();
            let request = recovered.operation().request();
            let (reference, checkpoint) = recovered.observation().current_checkpoint();
            let head = pale_ports::ControlCheckpointHead::new(
                checkpoint.checkpoint_domain_id().clone(),
                *checkpoint.domain_epoch(),
                *checkpoint.domain_sequence(),
                *checkpoint.committed_transaction_sequence(),
                pale_ports::ControlRecordCoordinate::new(
                    reference.clone(),
                    pale_ports::ControlRecordDigest::parse(
                        checkpoint
                            .canonical_control_checkpoint_sha256()
                            .as_str()
                            .to_owned(),
                    )
                    .unwrap(),
                ),
            );
            case.anchor
                .replace_with_control_head(recovered.operation().store(), &head, 4);
            verify_current_head(
                request,
                operation,
                &recovered,
                case.custody,
                case.anchor,
                maximum,
            )
            .unwrap();
            let mut substituted = request.to_preimage();
            substituted.request_id = ResidentRequestIdV1::parse("another-native-request").unwrap();
            let substituted = crate::effect_case_lineage_fixture::request(&substituted);
            let reads = case.custody.access_counts();
            assert_eq!(
                verify_current_head(
                    &substituted,
                    operation,
                    &recovered,
                    case.custody,
                    case.anchor,
                    maximum
                ),
                Err(ResidentOperationFrameFailure::ReplayConflict)
            );
            assert_eq!(case.custody.access_counts(), reads);
            if missing_head {
                case.custody.remove_current_observation();
            } else {
                case.anchor.make_unavailable();
            }
            assert_eq!(
                verify_current_head(
                    request,
                    operation,
                    &recovered,
                    case.custody,
                    case.anchor,
                    maximum
                ),
                Err(ResidentOperationFrameFailure::ResidentUnavailable)
            );
        });
    }
}

impl ResidentProcessConsultPolicyEvaluator for ExactInvokePolicy {
    fn evaluate_consult(
        &mut self,
        _: ResidentProcessConsultPolicyContext<'_>,
    ) -> Result<ResidentProcessConsultPolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl ResidentProcessDelegatePolicyEvaluator for ExactInvokePolicy {
    fn evaluate_delegate(
        &mut self,
        _: ResidentProcessDelegatePolicyContext<'_>,
    ) -> Result<ResidentProcessDelegatePolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

struct NativePeer;

impl ResidentProgramPeerAdmissionPort for NativePeer {
    type Witness = ();
    fn admit_peer(
        &mut self,
        _: LocalIpcPeerCredentials,
        _: &ResidentOperationFrameRequest,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }
    fn admit_hook_peer(
        &mut self,
        _: LocalIpcPeerCredentials,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }
}

impl ResidentProgramObservePeerAdmissionPort for NativePeer {
    fn admit_observe_peer(
        &mut self,
        _: LocalIpcPeerCredentials,
        _: &ResidentObserveFrameRequest,
    ) -> Result<(), InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "native fixture peer",
            invariant: "Observe is outside this fixture",
        })
    }
}

struct InterruptedNativeRecovery<'a> {
    semantic: &'a crate::resident::LocalOperationStartInput<'a>,
    attempts: Arc<AtomicUsize>,
}

impl ResidentProgramRootRecoveryInputPort for InterruptedNativeRecovery<'_> {
    fn with_root_recovery<Output, UseInputs>(
        &mut self,
        _: &ResidentDeploymentAuthoritySelector,
        _: &CommonRequestEnvelopeV1,
        locator: Option<
            &crate::resident::process_host::operation_locator::ResidentProgramOperationLocator,
        >,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'restart, 'admission, 'checkpoint> FnOnce(
            ResidentProcessOperationRestartInput<'restart>,
            OperationControlRecoveryAdmission<'admission>,
            &'checkpoint ControlCheckpointSigningAuthorityAdmissionContextV1<'checkpoint>,
            &mut Self,
        ) -> Output,
    {
        if locator.is_some() {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            return Err(ResidentOperationFrameFailure::ResidentUnavailable);
        }
        let semantic = self.semantic;
        Ok(with_unreachable_restart_inputs(
            semantic,
            |restart, admission| {
                use_inputs(restart, admission, semantic.checkpoint_authority, self)
            },
        ))
    }
    fn retain_deferred_recovery(
        &mut self,
        _: Box<crate::resident::process_host::restart::ResidentProcessOperationRestartDisposition>,
    ) {
        panic!("native hook does not start a background dispatch")
    }
}

impl_unreachable_root_provider_recovery_owner!(InterruptedNativeRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_delegate_recovery_owner!(InterruptedNativeRecovery<'_>, UnprovedHeldWriter);
impl_unavailable_cancellation_inputs!(InterruptedNativeRecovery<'_>);
impl_unavailable_observe_inputs!(InterruptedNativeRecovery<'_>);

struct AuthenticatedFrame {
    inner: OperationFramePort,
    key: Option<[u8; 32]>,
}

impl LocalIpcFramePort for AuthenticatedFrame {
    fn authenticated_caller_key(&self) -> Option<[u8; 32]> {
        self.inner.inbound.is_none().then_some(self.key).flatten()
    }
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        self.inner.peer_credentials()
    }
    fn send_frame(&mut self, bytes: &[u8], deadline: Instant) -> Result<(), LocalIpcCarrierError> {
        self.inner.send_frame(bytes, deadline)
    }
    fn receive_frame(&mut self, deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.inner.receive_frame(deadline)
    }
}

#[test]
fn native_start_cannot_use_kernel_identity_or_another_authenticated_key() {
    for (key, expected) in [
        (None, ResidentOperationFrameFailure::AdmissionDenied),
        (
            Some([8; 32]),
            ResidentOperationFrameFailure::AdmissionDenied,
        ),
        // Only the exact verified key reaches native-body admission. The
        // deliberately malformed body then fails before any start allocation.
        (Some([7; 32]), ResidentOperationFrameFailure::InvalidRequest),
    ] {
        let now = UnixTimeV1::new(61).unwrap();
        with_admission_for_request(
            |exact| vec![Ok(exact.clone())],
            vec![now],
            true,
            ResidentOperationRequestKind::StartInvoke,
            None,
            |request| {
                let descriptor =
                    pale_contracts::schema_descriptor(NATIVE_TOOL_REQUEST_V1_SCHEMA_ID).unwrap();
                request.body.schema_id = SchemaId::parse(descriptor.schema_id).unwrap();
                request.body.schema_sha256 = SchemaDigest::parse(descriptor.schema_sha256).unwrap();
                request.body.payload = serde_json::json!({});
                request.body.payload_sha256 = ArtifactDigest::hash_bytes(b"{}");
            },
            |admission,
             process,
             authorities,
             session,
             original,
             exact,
             _,
             _,
             semantic,
             checkpoint,
             _| {
                let request = resealed_request(original, exact, session, now, |_| {});
                let store = crate::local_operation_start_fixture::store_coordinate();
                let physical = Arc::new(AtomicUsize::new(0));
                let mut custody = absent_start_store(&request)
                    .at_store(&store)
                    .with_index_read_counter(Arc::clone(&physical));
                let sessions = MemoryAuthorityStore::default();
                let mut anchor = initial_anchor();
                let owner = ResidentProgramOperationTurnOwner::new(
                    OperationOnlyPeer,
                    admission,
                    vec![ResidentProgramDeploymentOperationPorts::new(
                        authorities[0].fixture_selector().clone(),
                        store,
                        &mut custody,
                        &sessions,
                        &mut anchor,
                    )],
                    FreshEntropy::new(Arc::clone(&physical)),
                    CountingAuthority::default(),
                    StartCryptography {
                        checkpoint: RefCell::new(Some(checkpoint)),
                    },
                    RootRecovery { semantic },
                    ResidentProgramConsultRecoveryUnavailable,
                )
                .unwrap();
                let mut program = ResidentProgram::fixture_from_process_with_authorities(
                    process,
                    authorities.to_vec(),
                    consult_policy(semantic, now, request.expires_at()),
                    owner,
                );
                let mut carrier = AuthenticatedFrame {
                    inner: OperationFramePort::new(&frame(&request)),
                    key,
                };
                let deadline = Instant::now() + std::time::Duration::from_secs(2);
                program
                    .serve_resident_frame(&mut carrier, deadline, deadline)
                    .unwrap();
                let response: ResidentOperationFrameResponse =
                    serde_json::from_slice(&carrier.inner.sent).unwrap();
                assert_eq!(
                    response,
                    ResidentOperationFrameResponse::Rejected { failure: expected }
                );
                assert_eq!(
                    physical.load(Ordering::SeqCst),
                    0,
                    "native rejection must precede custody or allocation"
                );
            },
        );
    }
}

#[test]
fn native_start_registers_the_actual_durable_result_without_allowing_the_hook() {
    let now = UnixTimeV1::new(61).unwrap();
    with_admission_for_request(
        |exact| vec![Ok(exact.clone()); 4],
        vec![now; 4],
        true,
        ResidentOperationRequestKind::StartInvoke,
        Some("native-tool-1"),
        |_| {},
        |admission,
         process,
         authorities,
         session,
         original,
         exact,
         _,
         _,
         semantic,
         checkpoint,
         _| {
            let request = resealed_request(original, exact, session, now, |_| {});
            let store = crate::local_operation_start_fixture::store_coordinate();
            let physical = Arc::new(AtomicUsize::new(0));
            let mut custody = absent_start_store(&request)
                .at_store(&store)
                .with_index_read_counter(Arc::clone(&physical))
                .with_unlisted_indexes_absent()
                .with_lease_retention(Ok(OperationCustodyCommitOutcome::Committed))
                .with_preparation(
                    initial_response,
                    ControlTransactionSequenceV1::new(1).unwrap(),
                );
            let sessions = MemoryAuthorityStore::default();
            let mut anchor = initial_anchor();
            let recovery_attempts = Arc::new(AtomicUsize::new(0));
            let owner = ResidentProgramOperationTurnOwner::new(
                NativePeer,
                admission,
                vec![ResidentProgramDeploymentOperationPorts::new(
                    authorities[0].fixture_selector().clone(),
                    store,
                    &mut custody,
                    &sessions,
                    &mut anchor,
                )],
                FreshEntropy::new(Arc::clone(&physical)),
                CountingAuthority::default(),
                StartCryptography {
                    checkpoint: RefCell::new(Some(checkpoint)),
                },
                InterruptedNativeRecovery {
                    semantic,
                    attempts: Arc::clone(&recovery_attempts),
                },
                ResidentProgramConsultRecoveryUnavailable,
            )
            .unwrap();
            let mut program = ResidentProgram::fixture_from_process_with_authorities(
                process,
                authorities.to_vec(),
                // Resealing gives this frame a shorter lifetime than the
                // original fixture; its issued handle must expire with it.
                exact_invoke_policy(semantic, 1 << 20).with_handle_expiry(request.expires_at()),
                owner,
            );
            let deadline = Instant::now() + std::time::Duration::from_secs(2);
            let mut carrier = AuthenticatedFrame {
                inner: OperationFramePort::new(&frame(&request)),
                key: Some([7; 32]),
            };
            program
                .serve_resident_frame(&mut carrier, deadline, deadline)
                .unwrap();
            let response: ResidentOperationFrameResponse =
                serde_json::from_slice(&carrier.inner.sent).unwrap();
            assert!(
                matches!(response, ResidentOperationFrameResponse::Started { .. }),
                "native start did not issue: {response:?}"
            );
            let after_start = physical.load(Ordering::SeqCst);
            assert!(
                after_start > 0,
                "the native start must use actual operation custody"
            );

            let mut substitute = AuthenticatedFrame {
                inner: OperationFramePort::new(&frame(&request)),
                key: Some([8; 32]),
            };
            program
                .serve_resident_frame(&mut substitute, deadline, deadline)
                .unwrap();
            let response: ResidentOperationFrameResponse =
                serde_json::from_slice(&substitute.inner.sent).unwrap();
            assert_eq!(
                response,
                ResidentOperationFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::AdmissionDenied
                }
            );
            assert_eq!(physical.load(Ordering::SeqCst), after_start);

            let native = crate::resident::ResidentNativeToolStart::admit(request.body()).unwrap();
            let hook = pale_ports::ResidentHookFrameRequest::new(
                native.request().routing().arguments(),
                request.body().payload["native_event"]
                    .as_str()
                    .unwrap()
                    .into(),
            );
            let mut substitute_hook = AuthenticatedFrame {
                inner: OperationFramePort::new(&hook),
                key: Some([8; 32]),
            };
            program
                .serve_resident_frame(&mut substitute_hook, deadline, deadline)
                .unwrap();
            assert_eq!(
                serde_json::from_slice::<pale_ports::ResidentHookFrameResponse>(
                    &substitute_hook.inner.sent
                )
                .unwrap(),
                pale_ports::ResidentHookFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::AdmissionDenied,
                }
            );
            assert_eq!(recovery_attempts.load(Ordering::SeqCst), 0);
            assert_eq!(physical.load(Ordering::SeqCst), after_start);
            let mut carrier = AuthenticatedFrame {
                inner: OperationFramePort::new(&hook),
                key: Some([7; 32]),
            };
            program
                .serve_resident_frame(&mut carrier, deadline, deadline)
                .unwrap();
            let response: pale_ports::ResidentHookFrameResponse =
                serde_json::from_slice(&carrier.inner.sent).unwrap();
            assert_eq!(
                response,
                pale_ports::ResidentHookFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::ResidentUnavailable,
                }
            );
            assert_eq!(
                physical.load(Ordering::SeqCst),
                after_start,
                "the hook still grants no native work"
            );
            assert_eq!(recovery_attempts.load(Ordering::SeqCst), 1);
            let mut retried = AuthenticatedFrame {
                inner: OperationFramePort::new(&hook),
                key: Some([7; 32]),
            };
            program
                .serve_resident_frame(&mut retried, deadline, deadline)
                .unwrap();
            assert_eq!(
                serde_json::from_slice::<pale_ports::ResidentHookFrameResponse>(
                    &retried.inner.sent
                )
                .unwrap(),
                response,
                "retry retains the actual recovery refusal"
            );
            assert_eq!(
                recovery_attempts.load(Ordering::SeqCst),
                2,
                "interrupted correlation remains resumable after fresh admission"
            );
            assert_eq!(
                physical.load(Ordering::SeqCst),
                after_start,
                "resuming the hook cannot allocate another operation"
            );
        },
    );
}
