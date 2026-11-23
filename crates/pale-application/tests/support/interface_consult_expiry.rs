//! Prepare the committed expiry boundary before reopening the public service.

use super::*;

struct InterruptedAnchor<'a>(&'a mut dyn AntiRollbackAnchorPort);

impl AntiRollbackAnchorPort for InterruptedAnchor<'_> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.0.read(key)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.0.observe(transition)
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))
    }
}

pub(in super::super::super) fn prepare_interrupted_consult_expiry(
    state: &ConformanceResidentState,
    request: &CommonRequestEnvelopeV1,
    locator: &ResidentOperationFrameLocator,
    custody: &mut dyn OperationCustodyPort,
    anchors: &mut ConformanceAntiRollbackAnchors,
) -> (ConsultationCollection, ConsultationCollection, UnixTimeV1) {
    let coordinate = ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(locator.operation_handle_record_ref().to_owned()).unwrap(),
        ControlRecordDigest::parse(locator.operation_handle_sha256().to_owned()).unwrap(),
    );
    let domain = SupportedOperationCheckpointProfile::conflict_domain(request).unwrap();
    let profile = supported_profile();
    let authority = ConformanceCheckpointAuthority::new(&state.session);
    let checkpoint = authority.context(&state.store, &domain, &state.session);
    let checkpoint_schema = schema(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
    let control = with_recovery_admissions!(
        &state.session,
        &checkpoint_schema,
        &checkpoint,
        |admissions| {
            recover_operation_control(
                recovery_input(
                    &state.store,
                    &profile,
                    &state.session.signing,
                    &checkpoint,
                    &state.historical,
                    &coordinate,
                    consult_recovery_limits(),
                ),
                custody,
                &state.store_owner,
                anchors,
                admissions.recovery,
            )
            .expect("recover the service's actual issued parent and collection observation")
        }
    );
    let operation = control.operation().operation();
    let observation = control.observation();
    let resolve = |_: &OperationObservationV1| Ok(state.store.clone());
    let limits = ConsultationCollectionLimits::new(1, 65_536, 65_536).unwrap();
    let collection = ConsultationCollection::recover_current(
        operation,
        observation.selected().1,
        limits,
        &resolve,
        custody,
        |_| Ok(()),
    )
    .unwrap()
    .expect("the first service committed its collection");
    let stopped_at = *operation.handle().1.expires_at();
    let reason = ImmutableRecordReferenceV1 {
        record_ref: record_ref("interface-conformance-expiry"),
        record_sha256: ArtifactDigest::hash_bytes(b"interface-conformance expiry"),
    };
    let expected = collection
        .stop(
            ConsultationStopCause::OperationExpiry,
            reason.clone(),
            stopped_at,
            limits,
            |_, _| Ok(()),
        )
        .unwrap();
    let observation_schema = schema(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let advances = anchors.advances;
    let result = commit_recovered_consultation_stop(
        &collection,
        ConsultationStopInput {
            cause: ConsultationStopCause::OperationExpiry,
            reason,
            stopped_at,
            parent: operation,
            parent_observation: observation.selected(),
            parent_checkpoint: observation.current_checkpoint(),
            observation_ref: record_ref("interface-conformance-expired-observation"),
            observed_at: stopped_at,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &observation_schema,
                signing_authority: &state.session.signing,
            },
            limits,
        },
        ConsultationSuccessorCommitInput {
            accumulator_limits: accumulator_limits(),
            identity: OperationCheckpointInput {
                reference: record_ref("interface-conformance-expiry-checkpoint"),
                id: ControlCheckpointIdV1::parse("interface-conformance-expiry-checkpoint")
                    .unwrap(),
            },
            issued_at: stopped_at,
            schema: &checkpoint_schema,
            authority: &checkpoint,
        },
        &resolve,
        custody,
        &mut InterruptedAnchor(anchors),
        |_, summary| {
            assert!(summary.requirements_met());
            Ok(())
        },
        |_| Ok(()),
        |_, _, _, _| panic!("the expiry has no authority rotation"),
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
    );
    assert!(
        matches!(
            result,
            Err(ConsultationStopFailure::Custody(
                ConsultationCollectionCommitFailure::Anchor { .. }
            ))
        ),
        "expiry must commit before the independent anchor interruption"
    );
    drop(result);
    assert_eq!(anchors.advances, advances);
    (collection.collection().clone(), expected, stopped_at)
}
