//! Rebuild from reopened physical records after discarding the pending expiry.

use super::*;
use pale_application::diagnostic::AdmittedDiagnostic;
use pale_application::resident::{
    recover_committed_pre_dispatch_expiry, CommittedObservationCheckpointRecovery,
    OperationCustodyDatabaseEvidence, OperationObservationTransition,
};

pub(super) fn cold_sweep(
    process: &mut RestartFixture<'_>,
    input: &history::Input<'_>,
    request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    port: &mut SqliteOperationCustody<'_>,
    sessions: &dyn SessionLeaseCustodyPort,
    anchor: &mut IndependentAnchor,
) {
    let before = retained_custody_shape(port.connection);
    let advances = anchor.advance_count();
    let limits = AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap();
    let mut missing = IndependentAnchor::default();
    let failed = history::with_sweep_admission(input, request, parents, |context, admission| {
        process.cold_sweep((context, limits), port, sessions, &mut missing, admission)
    });
    assert!(failed.is_err());
    assert_eq!(missing.advance_count(), 0);
    let mut runtime = input
        .signing
        .integrated_runtime_authority()
        .unwrap()
        .clone();
    runtime.deployment_runtime_authority_ref = record_ref("runtime/cold-sweep-other");
    let other_signing = SigningAuthorityBindingV1::integrated_runtime(
        input.signing.principal_id().clone(),
        ResidentKeyIdV1::parse(input.signing.signer_key_id()).unwrap(),
        *input.signing.signer_key_epoch(),
        runtime,
    )
    .unwrap();
    let failed = history::with_sweep_admission(input, request, parents, |context, admission| {
        let context = LocalOperationRecoveryContext {
            handle_signing_authority: &other_signing,
            ..context
        };
        process.cold_sweep((context, limits), port, sessions, anchor, admission)
    });
    assert!(
        failed.is_err(),
        "selected deployment authority is checked before anchor repair"
    );
    let failed = history::with_sweep_admission(input, request, parents, |context, admission| {
        process.cold_sweep(
            (
                context,
                AccumulatorPreparationLimits::new(1, 1, 1, 1, 1).unwrap(),
            ),
            port,
            sessions,
            anchor,
            admission,
        )
    });
    assert!(failed.is_err());
    assert_eq!(anchor.advance_count(), advances);
    let failed =
        history::with_sweep_admission(input, request, parents, |mut context, admission| {
            context.observation_limits =
                OperationObservationRecoveryLimits::new(1, 65_536, 65_536).unwrap();
            process.cold_sweep((context, limits), port, sessions, anchor, admission)
        });
    assert!(
        failed.is_err(),
        "history traversal must retain its revision ceiling"
    );
    let failed = history::with_sweep_admission(input, request, parents, |context, admission| {
        let mut reject = |_: &[u8],
                          _: &OperationObservationPreimageV1,
                          _: &OperationObservationDigest,
                          _: &CanonicalSignatureV1| {
            Err(InvalidResidentRecordV1::NonCanonical {
                relation: "fixture rejected historical signature",
            })
        };
        let admission = OperationControlRecoveryAdmission {
            verify_observation_signature: &mut reject,
            admit_request: &mut *admission.admit_request,
            admit_handle_authority: &mut *admission.admit_handle_authority,
            verify_handle_signature: &mut *admission.verify_handle_signature,
            admit_checkpoint: &mut *admission.admit_checkpoint,
            resolve_observation_signer: &mut *admission.resolve_observation_signer,
            admit_observation_state: &mut *admission.admit_observation_state,
            admit_rotation: &mut *admission.admit_rotation,
            verify_checkpoint_signature: &mut *admission.verify_checkpoint_signature,
        };
        process.cold_sweep((context, limits), port, sessions, anchor, admission)
    });
    assert!(
        failed.is_err(),
        "stored signatures require historical owner admission"
    );
    assert_eq!(anchor.advance_count(), advances);
    // Neither failure may consume the frozen source: retry discovers the same start.
    history::with_sweep_admission(input, request, parents, |context, admission| {
        process.cold_sweep((context, limits), port, sessions, anchor, admission)
    })
    .unwrap();
    assert_eq!(anchor.advance_count(), advances + 1);
    assert_eq!(retained_custody_shape(port.connection), before);
    assert_eq!(port.connection.total_changes(), 0);
}

fn admit_checkpoint(
    record: &ControlRecord,
    fields: &ControlStateCheckpointPreimageV1,
) -> ControlStateCheckpointV1 {
    assert_eq!(record.schema_sha256(), &fields.schema_sha256);
    let admitted = ControlStateCheckpointV1::admit_signed_wire(
        serde_json::from_slice(record.canonical_record_jcs().as_slice()).unwrap(),
        &fields.schema_sha256,
        &checkpoint_authority(fields),
        |domain, _, _, actual| {
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        record.coordinate().record_sha256().as_str(),
        admitted.canonical_control_checkpoint_sha256().as_str()
    );
    assert_eq!(
        record.canonical_record_jcs().as_slice(),
        pale_core::canon::to_canonical_string(&admitted)
            .unwrap()
            .as_bytes()
    );
    admitted
}

pub(super) fn complete(
    input: &history::Input<'_>,
    fields: &ControlStateCheckpointPreimageV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) {
    let handle = input.operation.handle();
    let evidence =
        read_operation_observation_for_handle(port, input.coordinate, handle.0, handle.1, 65_536)
            .unwrap()
            .unwrap();
    let next = admit_checkpoint(evidence.checkpoint(), fields);
    let prior_coordinate = ControlRecordCoordinate::new(
        next.previous_control_checkpoint_ref()
            .as_ref()
            .unwrap()
            .clone(),
        ControlRecordDigest::parse(
            next.previous_control_checkpoint_sha256()
                .as_ref()
                .unwrap()
                .as_str(),
        )
        .unwrap(),
    );
    let prior_record = port
        .read_operation_record(input.coordinate, &prior_coordinate, 65_536)
        .unwrap()
        .unwrap();
    let prior = admit_checkpoint(&prior_record, fields);
    let predecessor = input.known.last().unwrap();
    let observation = OperationObservationV1::admit_contextual_wire(
        serde_json::from_slice(evidence.observation().canonical_record_jcs().as_slice()).unwrap(),
        &OperationObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                signing_authority: input.signing,
            },
            handle_ref: handle.0,
            handle: handle.1,
            predecessor: Some(*predecessor),
        },
        |state| {
            assert!(matches!(state, OperationObservationStateV1::Expired { .. }));
            Ok(())
        },
        |_, _, _, _| panic!("fixture expiry has no signer rotation"),
        |domain, _, _, actual| {
            assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        evidence.observation().coordinate().record_sha256().as_str(),
        observation
            .canonical_operation_observation_sha256()
            .as_str()
    );
    assert_eq!(
        evidence.observation().canonical_record_jcs().as_slice(),
        pale_core::canon::to_canonical_string(&observation)
            .unwrap()
            .as_bytes()
    );
    let OperationObservationStateV1::Expired {
        diagnostic: reference,
    } = observation.state()
    else {
        panic!("expiry")
    };
    let diagnostic_record = port
        .read_operation_record(
            input.coordinate,
            &ControlRecordCoordinate::new(
                reference.record_ref.clone(),
                ControlRecordDigest::parse(reference.record_sha256.as_str()).unwrap(),
            ),
            65_536,
        )
        .unwrap()
        .unwrap();
    let diagnostic =
        AdmittedDiagnostic::from_bytes(diagnostic_record.canonical_record_jcs().as_slice())
            .unwrap();
    let resolve = |_: &OperationObservationV1| Ok(input.coordinate.clone());
    let transition = OperationObservationTransition::new(
        handle.0,
        handle.1,
        predecessor.0,
        predecessor.1,
        evidence.observation().coordinate().record_ref(),
        &observation,
    );
    let limits = AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap();
    let recover =
        |port: &mut SqliteOperationCustody<'_>, prior: &ControlStateCheckpointV1, limits| {
            recover_committed_pre_dispatch_expiry(
                transition,
                &diagnostic,
                CommittedObservationCheckpointRecovery {
                    selection: input.operation.checkpoint_selection(),
                    predecessor: (prior_record.coordinate().record_ref(), prior),
                    successor: (evidence.checkpoint().coordinate().record_ref(), &next),
                    limits,
                },
                &resolve,
                port,
            )
        };
    let before = retained_custody_shape(port.connection);
    let advances = anchor.advance_count();
    assert!(
        recover(port, &next, limits).is_err(),
        "wrong predecessor cannot recover"
    );
    assert!(
        recover(
            port,
            &prior,
            AccumulatorPreparationLimits::new(1, 1, 1, 1, 1).unwrap()
        )
        .is_err(),
        "proof limits are enforced"
    );
    let pending = recover(port, &prior, limits).unwrap();
    assert_eq!(
        pending.database_evidence(),
        OperationCustodyDatabaseEvidence::ExactReplay
    );
    let mut missing = IndependentAnchor::default();
    assert!(
        pending.advance_anchor(&mut missing).is_err(),
        "a missing anchor cannot be reconstructed from SQLite"
    );
    assert_eq!(missing.advance_count(), 0);
    assert_eq!(anchor.advance_count(), advances);
    let completed = recover(port, &prior, limits)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    assert_eq!(
        completed.database_evidence(),
        OperationCustodyDatabaseEvidence::ExactReplay
    );
    assert_eq!(anchor.advance_count(), advances + 1);
    recover(port, &prior, limits)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    assert_eq!(
        anchor.advance_count(),
        advances + 1,
        "exact replay does not repeat anchor CAS"
    );
    assert_eq!(retained_custody_shape(port.connection), before);
    assert_eq!(port.connection.total_changes(), 0);
}
