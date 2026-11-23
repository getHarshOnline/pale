//! A caller cancellation request and provider dispatch race on one real head.
//! Signatures, present authority, and the independent anchor are explicit models.

use super::*;
use crate::resident_fixture::effect_case_lineage_fixture as request_fixture;
use crate::resident_fixture::local_operation_start_fixture::{profile, RecoveryParents};
use crate::resident_fixture::shared_resident::RecoveryResolver;

pub(super) fn control_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
    capability: &str,
    name: &str,
    issued_at: u64,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = original.to_preimage();
    preimage.request_id = ResidentRequestIdV1::parse(format!("control-{name}-request")).unwrap();
    preimage.correlation_id =
        ResidentCorrelationIdV1::parse(format!("control-{name}-correlation")).unwrap();
    preimage.capability_id = CapabilityIdV1::parse(capability).unwrap();
    preimage.idempotency_key =
        RequestIdempotencyKeyV1::parse(format!("control-{name}-idempotency")).unwrap();
    preimage.issued_at = UnixTimeV1::new(issued_at).unwrap();
    preimage.expires_at = *handle.expires_at();
    preimage.generation = Some(handle.compiled_generation_sha256().clone());
    preimage.fence_token = Some(handle.activation_fence_sha256().clone());
    preimage.body.schema_id = SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).unwrap();
    preimage.body.schema_sha256 = handle.schema_sha256().clone();
    preimage.body.payload = serde_json::to_value(handle).unwrap();
    preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&preimage.body.payload)
            .unwrap()
            .as_bytes(),
    );
    request_fixture::request(&preimage)
}

pub(super) struct PublishedDispatchedCancellation {
    pub(super) request: CommonRequestEnvelopeV1,
    pub(super) reference: ResidentRecordRefV1,
    pub(super) observation: OperationObservationV1,
    pub(super) candidate: OperationCustodyCandidate,
    pub(super) checkpoint: ControlStateCheckpointV1,
    pub(super) fields: ControlStateCheckpointPreimageV1,
}

/// Publish one post-dispatch cancellation request after a durable staged outcome.
///
/// This fixture uses the real protected control recovery and cancellation
/// preparation paths; only signature authority remains the explicit test model.
#[allow(clippy::too_many_arguments)]
pub(super) fn publish_after_staged_outcome(
    operation: &IssuedLocalOperation,
    original_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    sessions: &dyn SessionLeaseCustodyPort,
    signing: SignedResidentAdmissionContextV1<'_>,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> PublishedDispatchedCancellation {
    let recovered = recover_control(
        operation,
        original_request,
        parents,
        checkpoint_fields,
        sessions,
        port,
        anchor,
        4,
    );
    let cancel_request = control_request(
        original_request,
        operation.handle().1,
        OPERATION_CANCEL_CAPABILITY_ID,
        "staged-cancel",
        69,
    );
    let authorized = recovered
        .authorize_cancellation(&cancel_request, UnixTimeV1::new(69).unwrap(), |facts| {
            assert!(matches!(
                facts.observation.1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched { .. }
                }
            ));
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        authorized.disposition(),
        OperationCancellationDisposition::Dispatched { .. }
    ));
    let prepared_request = authorized
        .prepare_dispatched_request(
            DispatchedOperationCancellationRequestInput {
                request_ref: record_ref("native/staged-cancel-request"),
                observation_ref: record_ref("native/staged-cancellation-requested"),
                signing,
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    let reference = prepared_request.observation().0.clone();
    let observation = prepared_request.observation().1.clone();
    let coordinate = port.coordinate.clone();
    let prepared_checkpoint = prepared_request
        .prepare_checkpoint(
            AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap(),
            &|_: &OperationObservationV1| Ok(coordinate.clone()),
            port,
        )
        .unwrap();
    let predecessor = prepared_request.predecessor_checkpoint();
    let mut fields = checkpoint_preimage(
        predecessor.1.domain_sequence().get() + 1,
        Some((
            predecessor.0.clone(),
            predecessor.1.canonical_control_checkpoint_sha256().clone(),
        )),
    );
    fields.checkpoint_domain_id = predecessor.1.checkpoint_domain_id().clone();
    fields.checkpoint_profile_ref = predecessor.1.checkpoint_profile_ref().clone();
    fields.checkpoint_profile_sha256 = predecessor.1.checkpoint_profile_sha256().clone();
    fields.committed_transaction_sequence = prepared_checkpoint.transaction_sequence();
    fields.state_root_sha256 = prepared_checkpoint.state_root().clone();
    fields.unresolved_custody_root_sha256 = prepared_checkpoint.unresolved_root().clone();
    fields.changed_leaf_keys = prepared_checkpoint.changed_keys().to_vec();
    fields.leaf_count = prepared_checkpoint.leaf_count();
    fields.issued_at = UnixTimeV1::new(70).unwrap();
    let (custody, checkpoint) = prepared_checkpoint
        .issue_checkpoint(
            &record_ref("native/staged-cancel-checkpoint"),
            fields.clone(),
            &fields.schema_sha256,
            &checkpoint_authority(&fields),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    let completed = custody
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    PublishedDispatchedCancellation {
        request: cancel_request,
        reference,
        observation,
        candidate: completed.candidate().clone(),
        checkpoint,
        fields,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn recover_control(
    operation: &IssuedLocalOperation,
    original_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    sessions: &dyn SessionLeaseCustodyPort,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_revisions: usize,
) -> RecoveredOperationControl {
    recover_control_selected(
        operation,
        original_request,
        parents,
        checkpoint_fields,
        sessions,
        port,
        anchor,
        maximum_revisions,
        OperationObservationSelection::SelectedHead,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn recover_control_selected(
    operation: &IssuedLocalOperation,
    original_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    sessions: &dyn SessionLeaseCustodyPort,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_revisions: usize,
    observation: OperationObservationSelection<'_>,
) -> RecoveredOperationControl {
    match read_control(
        operation,
        original_request,
        parents,
        checkpoint_fields,
        sessions,
        port,
        anchor,
        maximum_revisions,
        ControlRead::Recover { observation },
    ) {
        ControlReadOutcome::Recovered(value) => *value,
        ControlReadOutcome::Observed(_) => unreachable!("recovery returned an observation"),
    }
}

enum ControlRead<'a> {
    Recover {
        observation: OperationObservationSelection<'a>,
    },
    Observe {
        observation: OperationObservationSelection<'a>,
        request: &'a CommonRequestEnvelopeV1,
        authorized_at: UnixTimeV1,
        authorize: &'a mut dyn FnMut(
            OperationControlAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    },
}

impl<'a> ControlRead<'a> {
    const fn selection(&self) -> OperationObservationSelection<'a> {
        match self {
            Self::Recover { observation } | Self::Observe { observation, .. } => *observation,
        }
    }
}

enum ControlReadOutcome {
    Recovered(Box<RecoveredOperationControl>),
    Observed(Box<Result<OperationControlObservation, LocalOperationObservationError>>),
}

/// Expose only session reads while testing the application Observe path.
struct ObservationSessions<'a>(&'a dyn SessionLeaseCustodyPort);

impl SessionLeaseCustodyPort for ObservationSessions<'_> {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("Observe must not retain or renew a session")
    }

    fn read(
        &self,
        query: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        self.0.read(query)
    }
}

#[allow(clippy::too_many_arguments)]
fn observe_control(
    operation: &IssuedLocalOperation,
    original_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    sessions: &dyn SessionLeaseCustodyPort,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_revisions: usize,
    request: &CommonRequestEnvelopeV1,
    authorized_at: UnixTimeV1,
    authorize: &mut dyn FnMut(
        OperationControlAuthorizationContext<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
) -> Result<OperationControlObservation, LocalOperationObservationError> {
    match read_control(
        operation,
        original_request,
        parents,
        checkpoint_fields,
        sessions,
        port,
        anchor,
        maximum_revisions,
        ControlRead::Observe {
            observation: OperationObservationSelection::SelectedHead,
            request,
            authorized_at,
            authorize,
        },
    ) {
        ControlReadOutcome::Observed(value) => *value,
        ControlReadOutcome::Recovered(_) => unreachable!("observation returned protected state"),
    }
}

#[allow(clippy::too_many_arguments)]
fn read_control(
    operation: &IssuedLocalOperation,
    original_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    sessions: &dyn SessionLeaseCustodyPort,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_revisions: usize,
    read: ControlRead<'_>,
) -> ControlReadOutcome {
    let coordinate = port.coordinate.clone();
    let (handle_ref, handle) = operation.handle();
    let locator = ControlRecordCoordinate::new(
        handle_ref.clone(),
        ControlRecordDigest::parse(handle.canonical_operation_handle_sha256().as_str()).unwrap(),
    );
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(parents.current.generation.compiled_generation);
    resolver.insert_active(parents.current.generation.active_head);
    resolver.insert_fence(parents.current.generation.activation_fence);
    resolver.insert_effective_profile(parents.current.generation.effective_profile);
    let supported_profile = profile();
    let admitted_checkpoint_authority = checkpoint_authority(checkpoint_fields);
    let expected_request = original_request.to_preimage();
    let request_admission = CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &expected_request.envelope_schema_sha256,
        expected_body_schema_id: &expected_request.body.schema_id,
        expected_body_schema_sha256: &expected_request.body.schema_sha256,
        caller: &expected_request.caller,
        signing_authority: &expected_request.signing_authority_binding,
        route: &expected_request.route,
        session_admission: &expected_request.session_admission,
        operation_target: expected_request.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: None,
        workspace_id: &expected_request.workspace_id,
        capability_id: &expected_request.capability_id,
        operation: &expected_request.operation,
        scope: &expected_request.scope,
        risk_class: &expected_request.risk_class,
        resource_constraints: &expected_request.resource_constraints,
        expected_precondition: None,
        idempotency_key: &expected_request.idempotency_key,
        issued_at: expected_request.issued_at,
        expires_at: expected_request.expires_at,
        generation: expected_request.generation.as_ref(),
        fence_token: expected_request.fence_token.as_ref(),
    };
    let mut admit_request = |reference: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
        assert_eq!(reference, operation.request_coordinate().record_ref());
        CommonRequestEnvelopeV1::admit_contextual_wire(
            wire,
            &request_admission,
            |preimage, digest, actual| {
                assert_eq!(preimage, &expected_request);
                assert_eq!(digest, original_request.canonical_request_sha256());
                assert_eq!(actual, original_request.caller_signature());
                Ok(())
            },
        )
    };
    let mut admit_handle_authority =
        |preimage: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
            assert!(parent.is_none());
            assert!(preimage.parent_operation.is_none());
            Ok(())
        };
    let mut verify_handle_signature =
        |domain: &[u8],
         _: &OperationHandlePreimageV1,
         _: &OperationHandleDigest,
         actual: &CanonicalSignatureV1| {
            assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        };
    let mut admit_checkpoint = |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
        ControlStateCheckpointV1::admit_signed_wire(
            wire,
            &checkpoint_fields.schema_sha256,
            &admitted_checkpoint_authority,
            |domain, _, _, actual| {
                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                assert_eq!(actual, &signature());
                Ok(())
            },
        )
    };
    let mut resolve_observation_signer =
        |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| Ok(parents.signing.clone());
    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
    let mut admit_rotation =
        |_: &ResidentRecordRefV1,
         _: &ArtifactDigest,
         _: &ResidentRecordRefV1,
         _: &ArtifactDigest| { panic!("the fixture has no authority rotation") };
    let mut verify_observation_signature =
        |domain: &[u8],
         _: &OperationObservationPreimageV1,
         _: &OperationObservationDigest,
         actual: &CanonicalSignatureV1| {
            assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        };
    let mut verify_checkpoint_signature =
        |domain: &[u8],
         _: &ControlStateCheckpointPreimageV1,
         _: &ControlStateCheckpointDigest,
         actual: &CanonicalSignatureV1| {
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        };
    let recovery = OperationControlRecoveryInput {
        operation: LocalOperationHandleRecoveryInput {
            context: LocalOperationRecoveryContext {
                store: &coordinate,
                profile: &supported_profile,
                handle_signing_authority: parents.signing,
                checkpoint_authority: &admitted_checkpoint_authority,
                historical: &resolver,
                maximum_record_bytes: 1 << 20,
                maximum_operation_lease_bytes: 1 << 20,
                maximum_session_lease_bytes: 1 << 20,
                membership_limits: AccumulatorMembershipLimits::new(3, 10_000, 100_000).unwrap(),
                observation_limits: OperationObservationRecoveryLimits::new(
                    maximum_revisions,
                    1 << 20,
                    1 << 20,
                )
                .unwrap(),
                checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).unwrap(),
            },
            expected_handle: &locator,
            maximum_parent_depth: 0,
        },
        observation: read.selection(),
        observation_limits: OperationObservationRecoveryLimits::new(
            maximum_revisions,
            1 << 20,
            1 << 20,
        )
        .unwrap(),
        checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).unwrap(),
        membership_limits: AccumulatorMembershipLimits::new(1, 10_000, 100_000).unwrap(),
    };
    let admission = OperationControlRecoveryAdmission {
        admit_request: &mut admit_request,
        admit_handle_authority: &mut admit_handle_authority,
        verify_handle_signature: &mut verify_handle_signature,
        admit_checkpoint: &mut admit_checkpoint,
        resolve_observation_signer: &mut resolve_observation_signer,
        admit_observation_state: &mut admit_observation_state,
        admit_rotation: &mut admit_rotation,
        verify_observation_signature: &mut verify_observation_signature,
        verify_checkpoint_signature: &mut verify_checkpoint_signature,
    };
    match read {
        ControlRead::Recover { .. } => ControlReadOutcome::Recovered(Box::new(
            recover_operation_control(recovery, port, sessions, anchor, admission).unwrap(),
        )),
        ControlRead::Observe {
            request,
            authorized_at,
            authorize,
            ..
        } => ControlReadOutcome::Observed(Box::new(observe_local_operation(
            LocalOperationObservationInput {
                recovery,
                request,
                authorized_at,
            },
            port,
            sessions,
            anchor,
            LocalOperationObservationAdmission {
                recovery: admission,
                authorize,
            },
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn race(
    attempt: PreparedNativeOperationAttempt<'_, '_>,
    prepared_dispatch: &CompletedOperationObservationCustody,
    operation: &IssuedLocalOperation,
    original_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    signing: SignedResidentAdmissionContextV1<'_>,
    checkpoint: &ControlStateCheckpointV1,
    checkpoint_fields: &ControlStateCheckpointPreimageV1,
    sessions: &dyn SessionLeaseCustodyPort,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    cancellation_wins: bool,
    handler_fails_after_cancellation: bool,
) {
    assert!(
        !cancellation_wins || !handler_fails_after_cancellation,
        "a handler outcome requires dispatch to win the first race"
    );
    let coordinate = port.coordinate.clone();
    let (_, handle) = operation.handle();
    let issued_at = checkpoint.issued_at().get() + 1;
    let authorized_at = UnixTimeV1::new(issued_at + 10).unwrap();
    assert!(authorized_at <= *handle.expires_at());
    let observe_request = control_request(
        original_request,
        handle,
        OPERATION_OBSERVE_CAPABILITY_ID,
        "observe",
        issued_at,
    );
    let writes_before_observe = port.connection.total_changes();
    let advances_before_observe = anchor.advance_count();
    let read_only_sessions = ObservationSessions(sessions);
    let mut read_only_anchor = anchor.recovery_view();
    let denied = observe_control(
        operation,
        original_request,
        parents,
        checkpoint_fields,
        &read_only_sessions,
        port,
        &mut read_only_anchor,
        2,
        &observe_request,
        authorized_at,
        &mut |_| {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "operation observation",
                invariant: "present disclosure authority rejected",
            })
        },
    );
    assert!(matches!(
        denied,
        Err(LocalOperationObservationError::Authorization(
            OperationControlError::Admission(InvalidResidentRecordV1::ContextualAdmission {
                record: "operation observation",
                invariant: "present disclosure authority rejected",
            })
        ))
    ));
    let observed = observe_control(
        operation,
        original_request,
        parents,
        checkpoint_fields,
        &read_only_sessions,
        port,
        &mut read_only_anchor,
        2,
        &observe_request,
        authorized_at,
        &mut |facts| {
            assert_eq!(facts.original_request, original_request);
            assert_eq!(facts.operation, operation.handle());
            assert_eq!(facts.observation, attempt.observation());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(observed.operation(), operation.handle());
    assert_eq!(observed.observation(), attempt.observation());
    assert_eq!(port.connection.total_changes(), writes_before_observe);
    assert_eq!(anchor.advance_count(), advances_before_observe);

    let recovered = recover_control(
        operation,
        original_request,
        parents,
        checkpoint_fields,
        sessions,
        port,
        anchor,
        2,
    );
    assert_eq!(
        recovered.observation().selected_head(),
        attempt.observation()
    );

    let cancel_request = control_request(
        original_request,
        handle,
        OPERATION_CANCEL_CAPABILITY_ID,
        "cancel",
        issued_at,
    );
    let authorized = recovered
        .authorize_cancellation(&cancel_request, authorized_at, |facts| {
            assert_eq!(facts.original_request, original_request);
            assert_eq!(facts.operation, operation.handle());
            assert_eq!(facts.observation, attempt.observation());
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        authorized.disposition(),
        OperationCancellationDisposition::BeforeDispatch
    ));
    let rejected_signatures = AtomicUsize::new(0);
    for (index, reused) in [
        handle.operation_lease_ref().clone(),
        recovered.observation().current_checkpoint().0.clone(),
    ]
    .into_iter()
    .enumerate()
    {
        let rejected = authorized.prepare_local_request(
            LocalOperationCancellationRequestInput {
                request_ref: reused,
                observation_ref: record_ref(&format!("control/rejected-cancel-{index}")),
                signing,
            },
            |_, _, _, _| panic!("known aliases fail before rotation admission"),
            |_, _, _| {
                rejected_signatures.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
        );
        assert!(matches!(
            rejected,
            Err(OperationCancellationRequestError::Association)
        ));
    }
    assert_eq!(rejected_signatures.load(Ordering::SeqCst), 0);
    let cancel = authorized
        .prepare_local_request(
            LocalOperationCancellationRequestInput {
                request_ref: record_ref("control/cancel-request"),
                observation_ref: record_ref("control/cancellation-requested"),
                signing,
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    assert!(matches!(
        cancel.observation().1.state(),
        OperationObservationStateV1::CancellationRequested { request }
            if request.record_ref == *cancel.request_record().coordinate().record_ref()
    ));
    let cancel_prepared = cancel
        .prepare_checkpoint(
            AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap(),
            &|_: &OperationObservationV1| Ok(coordinate.clone()),
            port,
        )
        .unwrap();
    let mut cancel_fields = checkpoint_preimage(
        checkpoint.domain_sequence().get() + 1,
        Some((
            prepared_dispatch
                .candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref()
                .clone(),
            checkpoint.canonical_control_checkpoint_sha256().clone(),
        )),
    );
    cancel_fields.checkpoint_domain_id = checkpoint.checkpoint_domain_id().clone();
    cancel_fields.checkpoint_profile_ref = checkpoint.checkpoint_profile_ref().clone();
    cancel_fields.checkpoint_profile_sha256 = checkpoint.checkpoint_profile_sha256().clone();
    cancel_fields.committed_transaction_sequence = cancel_prepared.transaction_sequence();
    cancel_fields.state_root_sha256 = cancel_prepared.state_root().clone();
    cancel_fields.unresolved_custody_root_sha256 = cancel_prepared.unresolved_root().clone();
    cancel_fields.changed_leaf_keys = cancel_prepared.changed_keys().to_vec();
    cancel_fields.leaf_count = cancel_prepared.leaf_count();
    cancel_fields.issued_at = UnixTimeV1::new(authorized_at.get() + 1).unwrap();
    let cancel_checkpoint_ref = record_ref("control/cancel-checkpoint");
    let (cancel_custody, cancel_checkpoint) = cancel_prepared
        .issue_checkpoint(
            &cancel_checkpoint_ref,
            cancel_fields.clone(),
            &cancel_fields.schema_sha256,
            &checkpoint_authority(&cancel_fields),
            |_, _, _| Ok(signature()),
        )
        .unwrap();

    let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
    let dispatch = attempt
        .prepare_dispatch(
            NativeOperationDispatchContext {
                completed: prepared_dispatch,
                current: parents.current,
                signing,
                observation_ref: record_ref("control/racing-dispatch"),
                resolve_store: &resolve,
            },
            &mut ProvingAuthority,
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    let (dispatch_custody, _) = prepare_append(
        dispatch.prepare_append(&resolve).unwrap(),
        operation,
        (
            prepared_dispatch
                .candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref(),
            checkpoint,
        ),
        port,
        authorized_at.get() + 1,
    );
    let (winner, loser) = if cancellation_wins {
        (cancel_custody, dispatch_custody)
    } else {
        (dispatch_custody, cancel_custody)
    };
    let completed = winner
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    let writes = port.connection.total_changes();
    assert!(loser.commit(port, anchor).is_err());
    assert_eq!(port.connection.total_changes(), writes);
    let selected = completed
        .candidate()
        .record(OperationCustodyRecordKind::OperationObservation)
        .unwrap();
    let dispatched = (!cancellation_wins).then(|| {
        (
            dispatch.observation().0.clone(),
            dispatch.observation().1.clone(),
        )
    });
    let expected = if cancellation_wins {
        cancel.observation().1.clone()
    } else {
        dispatched
            .as_ref()
            .expect("the dispatch winner has an exact observation coordinate")
            .1
            .clone()
    };
    assert_eq!(
        selected.canonical_record_jcs().as_slice(),
        pale_core::canon::to_canonical_string(&expected)
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        port.read_operation_record(&coordinate, cancel.request_record().coordinate(), 1 << 20,)
            .unwrap()
            .is_some(),
        cancellation_wins,
        "the caller request is retained only with the winning cancellation"
    );
    let cancel_request_coordinate = cancel.request_record().coordinate().clone();
    let outcome = if cancellation_wins {
        drop(completed);
        None
    } else {
        let release_clock = super::Clock(AtomicUsize::new(0));
        let outcome = dispatch
            .invoke(
                completed,
                NativeOperationExecutionContext {
                    current: parents.current,
                    resolve_store: &resolve,
                    clock: &release_clock,
                    maximum_record_bytes: 65_536,
                },
                &mut ProvingAuthority,
                port,
                anchor,
            )
            .expect("one committed dispatch releases its retained handler once");
        assert_eq!(
            release_clock.0.load(Ordering::SeqCst),
            3,
            "the single release completes all current checks without a retry"
        );
        Some(outcome)
    };
    drop(cancel);
    let recovered_after_race = recover_control(
        operation,
        original_request,
        parents,
        checkpoint_fields,
        sessions,
        port,
        anchor,
        3,
    );
    if cancellation_wins {
        assert!(matches!(
            recovered_after_race.observation().selected_head().1.state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ));
        assert_eq!(
            recovered_after_race.observation().current_checkpoint(),
            (&cancel_checkpoint_ref, &cancel_checkpoint)
        );
        let cancel_preimage = cancel_request.to_preimage();
        let cancel_admission = CommonRequestAdmissionContextV1 {
            expected_schema_sha256: &cancel_preimage.envelope_schema_sha256,
            expected_body_schema_id: &cancel_preimage.body.schema_id,
            expected_body_schema_sha256: &cancel_preimage.body.schema_sha256,
            caller: &cancel_preimage.caller,
            signing_authority: &cancel_preimage.signing_authority_binding,
            route: &cancel_preimage.route,
            session_admission: &cancel_preimage.session_admission,
            operation_target: cancel_preimage.operation_target.as_ref(),
            execution_owner_fence: None,
            federation_claim: None,
            workspace_id: &cancel_preimage.workspace_id,
            capability_id: &cancel_preimage.capability_id,
            operation: &cancel_preimage.operation,
            scope: &cancel_preimage.scope,
            risk_class: &cancel_preimage.risk_class,
            resource_constraints: &cancel_preimage.resource_constraints,
            expected_precondition: None,
            idempotency_key: &cancel_preimage.idempotency_key,
            issued_at: cancel_preimage.issued_at,
            expires_at: cancel_preimage.expires_at,
            generation: cancel_preimage.generation.as_ref(),
            fence_token: cancel_preimage.fence_token.as_ref(),
        };
        let recovered_cancel = recovered_after_race
            .recover_local_cancellation_request(port, 1 << 20, |reference, wire| {
                assert_eq!(reference, cancel_request_coordinate.record_ref());
                CommonRequestEnvelopeV1::admit_contextual_wire(
                    wire,
                    &cancel_admission,
                    |preimage, digest, actual| {
                        assert_eq!(preimage, &cancel_preimage);
                        assert_eq!(digest, cancel_request.canonical_request_sha256());
                        assert_eq!(actual, cancel_request.caller_signature());
                        Ok(())
                    },
                )
            })
            .unwrap();
        assert_eq!(recovered_cancel.request(), &cancel_request);
        let rejected_signatures = AtomicUsize::new(0);
        let rejected = recovered_cancel.prepare_completion(
            LocalOperationCancellationCompletionInput {
                diagnostic_ref: handle.operation_lease_ref().clone(),
                observation_ref: record_ref("control/rejected-cancelled"),
                established_at: UnixTimeV1::new(authorized_at.get() + 2).unwrap(),
                signing,
                maximum_diagnostic_bytes: 65_536,
                resolve_store: &resolve,
            },
            |_, _, _, _| panic!("known aliases fail before rotation admission"),
            |_, _, _| {
                rejected_signatures.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
        );
        assert!(matches!(
            rejected,
            Err(OperationCancellationCompletionError::Association)
        ));
        assert_eq!(rejected_signatures.load(Ordering::SeqCst), 0);
        let completion = recovered_cancel
            .prepare_completion(
                LocalOperationCancellationCompletionInput {
                    diagnostic_ref: record_ref("control/cancelled-diagnostic"),
                    observation_ref: record_ref("control/cancelled"),
                    established_at: UnixTimeV1::new(authorized_at.get() + 2).unwrap(),
                    signing,
                    maximum_diagnostic_bytes: 65_536,
                    resolve_store: &resolve,
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
            )
            .unwrap();
        assert_eq!(
            completion.predecessor_checkpoint(),
            (&cancel_checkpoint_ref, &cancel_checkpoint)
        );
        assert_eq!(
            completion.diagnostic().wire().code,
            "operation_cancelled_before_dispatch"
        );
        assert_eq!(
            completion.diagnostic().wire().effect,
            DiagnosticEffectCertaintyV1::NotStarted
        );
        assert!(matches!(
            completion.observation().1.state(),
            OperationObservationStateV1::Cancelled { .. }
        ));
        let terminal = completion
            .prepare_checkpoint(
                AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap(),
                &resolve,
                port,
            )
            .unwrap();
        let terminal_predecessor = completion.predecessor_checkpoint();
        let mut terminal_fields = checkpoint_preimage(
            terminal_predecessor.1.domain_sequence().get() + 1,
            Some((
                terminal_predecessor.0.clone(),
                terminal_predecessor
                    .1
                    .canonical_control_checkpoint_sha256()
                    .clone(),
            )),
        );
        terminal_fields.checkpoint_domain_id =
            terminal_predecessor.1.checkpoint_domain_id().clone();
        terminal_fields.checkpoint_profile_ref =
            terminal_predecessor.1.checkpoint_profile_ref().clone();
        terminal_fields.checkpoint_profile_sha256 =
            terminal_predecessor.1.checkpoint_profile_sha256().clone();
        terminal_fields.committed_transaction_sequence = terminal.transaction_sequence();
        terminal_fields.state_root_sha256 = terminal.state_root().clone();
        terminal_fields.unresolved_custody_root_sha256 = terminal.unresolved_root().clone();
        terminal_fields.changed_leaf_keys = terminal.changed_keys().to_vec();
        terminal_fields.leaf_count = terminal.leaf_count();
        terminal_fields.issued_at = UnixTimeV1::new(authorized_at.get() + 3).unwrap();
        let (terminal_custody, _) = terminal
            .issue_checkpoint(
                &record_ref("control/cancelled-checkpoint"),
                terminal_fields.clone(),
                &terminal_fields.schema_sha256,
                &checkpoint_authority(&terminal_fields),
                |_, _, _| Ok(signature()),
            )
            .unwrap();
        let terminal = terminal_custody
            .commit(port, anchor)
            .unwrap()
            .advance_anchor(anchor)
            .unwrap();
        assert_eq!(
            terminal
                .candidate()
                .record(OperationCustodyRecordKind::OperationObservation)
                .unwrap()
                .canonical_record_jcs()
                .as_slice(),
            pale_core::canon::to_canonical_string(completion.observation().1)
                .unwrap()
                .as_bytes()
        );
    } else {
        let writes = port.connection.total_changes();
        let rejected =
            recovered_after_race.recover_local_cancellation_request(port, 1 << 20, |_, _| {
                panic!("a losing cancellation request must not be admitted")
            });
        assert!(matches!(
            rejected,
            Err(OperationCancellationRecoveryError::NotRequested)
        ));
        assert_eq!(port.connection.total_changes(), writes);

        let authorized = recovered_after_race
            .authorize_cancellation(&cancel_request, authorized_at, |facts| {
                assert!(matches!(
                    facts.observation.1.state(),
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Dispatched { .. }
                    }
                ));
                Ok(())
            })
            .unwrap();
        let dispatched_request = authorized
            .prepare_dispatched_request(
                DispatchedOperationCancellationRequestInput {
                    request_ref: record_ref("control/dispatched-cancel-request"),
                    observation_ref: record_ref("control/dispatched-cancellation-requested"),
                    signing,
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
            )
            .unwrap();
        assert!(matches!(
            dispatched_request.observation().1.state(),
            OperationObservationStateV1::CancellationRequested { request }
                if request.record_ref
                    == *dispatched_request.request_record().coordinate().record_ref()
        ));
        let dispatched_request_coordinate =
            dispatched_request.request_record().coordinate().clone();
        let prepared_request = dispatched_request
            .prepare_checkpoint(
                AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap(),
                &|_: &OperationObservationV1| Ok(coordinate.clone()),
                port,
            )
            .unwrap();
        let request_predecessor = dispatched_request.predecessor_checkpoint();
        let mut request_fields = checkpoint_preimage(
            request_predecessor.1.domain_sequence().get() + 1,
            Some((
                request_predecessor.0.clone(),
                request_predecessor
                    .1
                    .canonical_control_checkpoint_sha256()
                    .clone(),
            )),
        );
        request_fields.checkpoint_domain_id = request_predecessor.1.checkpoint_domain_id().clone();
        request_fields.checkpoint_profile_ref =
            request_predecessor.1.checkpoint_profile_ref().clone();
        request_fields.checkpoint_profile_sha256 =
            request_predecessor.1.checkpoint_profile_sha256().clone();
        request_fields.committed_transaction_sequence = prepared_request.transaction_sequence();
        request_fields.state_root_sha256 = prepared_request.state_root().clone();
        request_fields.unresolved_custody_root_sha256 = prepared_request.unresolved_root().clone();
        request_fields.changed_leaf_keys = prepared_request.changed_keys().to_vec();
        request_fields.leaf_count = prepared_request.leaf_count();
        request_fields.issued_at = UnixTimeV1::new(authorized_at.get() + 2).unwrap();
        let request_checkpoint_ref = record_ref("control/dispatched-cancel-checkpoint");
        let (request_custody, request_checkpoint) = prepared_request
            .issue_checkpoint(
                &request_checkpoint_ref,
                request_fields.clone(),
                &request_fields.schema_sha256,
                &checkpoint_authority(&request_fields),
                |_, _, _| Ok(signature()),
            )
            .unwrap();
        request_custody
            .commit(port, anchor)
            .unwrap()
            .advance_anchor(anchor)
            .unwrap();
        drop(dispatched_request);
        drop(authorized);
        drop(recovered_after_race);

        let restart_path = port
            .connection
            .path()
            .expect("file-backed store")
            .to_owned();
        let mut restarted_sql = rusqlite::Connection::open(&restart_path).unwrap();
        let mut restarted_port = SqliteOperationCustody {
            connection: &mut restarted_sql,
            coordinate: coordinate.clone(),
        };
        let restarted_head = recover_control(
            operation,
            original_request,
            parents,
            checkpoint_fields,
            sessions,
            &mut restarted_port,
            anchor,
            4,
        );
        assert_eq!(
            restarted_head.observation().current_checkpoint(),
            (&request_checkpoint_ref, &request_checkpoint)
        );
        assert!(matches!(
            restarted_head.observation().selected_head().1.state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ));
        assert!(matches!(
            restarted_head
                .observation()
                .selected_predecessor()
                .expect("the cancellation request follows dispatch")
                .1
                .state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { .. }
            }
        ));
        let cancel_preimage = cancel_request.to_preimage();
        let cancel_admission = CommonRequestAdmissionContextV1 {
            expected_schema_sha256: &cancel_preimage.envelope_schema_sha256,
            expected_body_schema_id: &cancel_preimage.body.schema_id,
            expected_body_schema_sha256: &cancel_preimage.body.schema_sha256,
            caller: &cancel_preimage.caller,
            signing_authority: &cancel_preimage.signing_authority_binding,
            route: &cancel_preimage.route,
            session_admission: &cancel_preimage.session_admission,
            operation_target: cancel_preimage.operation_target.as_ref(),
            execution_owner_fence: None,
            federation_claim: None,
            workspace_id: &cancel_preimage.workspace_id,
            capability_id: &cancel_preimage.capability_id,
            operation: &cancel_preimage.operation,
            scope: &cancel_preimage.scope,
            risk_class: &cancel_preimage.risk_class,
            resource_constraints: &cancel_preimage.resource_constraints,
            expected_precondition: None,
            idempotency_key: &cancel_preimage.idempotency_key,
            issued_at: cancel_preimage.issued_at,
            expires_at: cancel_preimage.expires_at,
            generation: cancel_preimage.generation.as_ref(),
            fence_token: cancel_preimage.fence_token.as_ref(),
        };
        let recovered_request = restarted_head
            .recover_local_cancellation_request(&mut restarted_port, 1 << 20, |reference, wire| {
                assert_eq!(reference, dispatched_request_coordinate.record_ref());
                CommonRequestEnvelopeV1::admit_contextual_wire(
                    wire,
                    &cancel_admission,
                    |preimage, digest, actual| {
                        assert_eq!(preimage, &cancel_preimage);
                        assert_eq!(digest, cancel_request.canonical_request_sha256());
                        assert_eq!(actual, cancel_request.caller_signature());
                        Ok(())
                    },
                )
            })
            .unwrap();
        assert_eq!(recovered_request.request(), &cancel_request);
        let completion_signatures = AtomicUsize::new(0);
        let local_completion = recovered_request.prepare_completion(
            LocalOperationCancellationCompletionInput {
                diagnostic_ref: record_ref("control/dispatched-local-diagnostic"),
                observation_ref: record_ref("control/dispatched-local-cancelled"),
                established_at: UnixTimeV1::new(authorized_at.get() + 3).unwrap(),
                signing,
                maximum_diagnostic_bytes: 65_536,
                resolve_store: &|_: &OperationObservationV1| Ok(coordinate.clone()),
            },
            |_, _, _, _| panic!("post-dispatch request cannot rotate for local completion"),
            |_, _, _| {
                completion_signatures.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
        );
        assert!(matches!(
            local_completion,
            Err(OperationCancellationCompletionError::Association)
        ));
        assert_eq!(completion_signatures.load(Ordering::SeqCst), 0);
        drop(recovered_request);
        drop(restarted_head);

        let dispatched = dispatched.expect("the dispatch winner produced one exact observation");
        let restarted = recover_control_selected(
            operation,
            original_request,
            parents,
            checkpoint_fields,
            sessions,
            &mut restarted_port,
            anchor,
            4,
            OperationObservationSelection::HistoryMember {
                reference: &dispatched.0,
                digest: dispatched.1.canonical_operation_observation_sha256(),
            },
        );
        assert!(matches!(
            restarted.observation().selected_head().1.state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ));
        assert!(matches!(
            restarted.observation().selected().1.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { .. }
            }
        ));

        let outcome = outcome.expect("the dispatch winner already produced one retained outcome");
        let terminal = match outcome.outcome() {
            NativeOperationOutcome::Succeeded(executed) => {
                assert!(!handler_fails_after_cancellation);
                super::TerminalEvidence::Succeeded(Box::new(
                    executed
                        .prepare_completion(
                            NativeOperationCompletionInput {
                                recovered_history: Some(restarted.observation()),
                                result_ref: record_ref("control/dispatched-late-result"),
                                observation_ref: record_ref("control/dispatched-late-success"),
                                completed_at: UnixTimeV1::new(authorized_at.get() - 1).unwrap(),
                                observed_at: UnixTimeV1::new(authorized_at.get() + 3).unwrap(),
                                signing,
                            },
                            |_, _, _| Ok(signature()),
                            |_, _, _, _| Ok(()),
                            |_, _, _| Ok(signature()),
                        )
                        .expect("late success follows the recovered cancellation request"),
                ))
            }
            NativeOperationOutcome::Failed(failed) => {
                assert!(handler_fails_after_cancellation);
                let failure = failed
                    .prepare_failure(
                        NativeOperationFailureInput {
                            recovered_history: Some(restarted.observation()),
                            diagnostic_ref: record_ref("control/dispatched-late-diagnostic"),
                            observation_ref: record_ref("control/dispatched-late-failure"),
                            completed_at: UnixTimeV1::new(authorized_at.get() - 1).unwrap(),
                            observed_at: UnixTimeV1::new(authorized_at.get() + 3).unwrap(),
                            signing,
                            maximum_diagnostic_bytes: 65_536,
                        },
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                    )
                    .expect("late failure follows the recovered cancellation request");
                assert_eq!(failure.diagnostic().wire().code, "invalid_path");
                assert_eq!(
                    failure.diagnostic().wire().effect,
                    DiagnosticEffectCertaintyV1::NotApplicable
                );
                assert!(matches!(
                    failure.observation().1.state(),
                    OperationObservationStateV1::Failed { result: None, .. }
                ));
                super::TerminalEvidence::Failed(Box::new(failure))
            }
        };
        assert_eq!(
            terminal.observation().1.predecessor_ref().as_ref(),
            Some(restarted.observation().selected_head().0),
            "late terminal evidence keeps the cancellation request as predecessor"
        );
        if handler_fails_after_cancellation {
            assert!(matches!(
                terminal.observation().1.state(),
                OperationObservationStateV1::Failed { result: None, .. }
            ));
        } else {
            assert!(matches!(
                terminal.observation().1.state(),
                OperationObservationStateV1::Succeeded { .. }
            ));
        }
        let (terminal_custody, _) = prepare_append(
            terminal.prepare_append(&|_: &OperationObservationV1| Ok(coordinate.clone())),
            operation,
            (&request_checkpoint_ref, &request_checkpoint),
            &mut restarted_port,
            authorized_at.get() + 3,
        );
        let terminal = terminal_custody
            .commit(&mut restarted_port, anchor)
            .unwrap()
            .advance_anchor(anchor)
            .unwrap();
        assert_eq!(
            terminal.database_evidence(),
            OperationCustodyDatabaseEvidence::New
        );
        let terminal_candidate = terminal.candidate().clone();
        drop(terminal);
        drop(restarted);
        drop(restarted_port);
        drop(restarted_sql);

        let mut final_sql = rusqlite::Connection::open(restart_path).unwrap();
        let mut final_port = SqliteOperationCustody {
            connection: &mut final_sql,
            coordinate,
        };
        assert_eq!(
            final_port.observe_exact(&terminal_candidate).unwrap(),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
        let recovered_terminal = recover_control(
            operation,
            original_request,
            parents,
            checkpoint_fields,
            sessions,
            &mut final_port,
            anchor,
            5,
        );
        if handler_fails_after_cancellation {
            assert!(matches!(
                recovered_terminal.observation().selected_head().1.state(),
                OperationObservationStateV1::Failed { result: None, .. }
            ));
        } else {
            assert!(matches!(
                recovered_terminal.observation().selected_head().1.state(),
                OperationObservationStateV1::Succeeded { .. }
            ));
        }
        assert!(matches!(
            recovered_terminal
                .observation()
                .selected_predecessor()
                .expect("terminal evidence retains the cancellation request")
                .1
                .state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ));
    }
}
