//! Real response classification and expiry across reopened `SQLite` connections.
//! Deployment association, signatures and independent anchoring remain fixtures.

use super::*;
use crate::resident_fixture::local_operation_start_fixture::RecoveryParents;
use crate::resident_fixture::shared_resident::{
    synthetic_set_policy, FixedInputSource, RecoveryResolver,
};
use pale_application::resident::hook_response_restart_test_support::RestartFixture;
use std::cell::RefCell;

#[path = "native_hook_response_expiry_recovery.rs"]
mod recovery;

struct ExpiryClock(u64);
impl WallClockSourcePort for ExpiryClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(UnixTimeV1::new(self.0).unwrap())
    }
}

pub(in crate::operation::tests) fn restart_process<'a>(
    sessions: &'a DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    signing: &SigningAuthorityBindingV1,
    generation: super::super::super::super::local_recovery::GenerationInputs,
) -> RestartFixture<'a> {
    let resolver = RecoveryResolver::default();
    let mut source = FixedInputSource::default();
    let opened = ResidentApplication::cold_open(
        sessions,
        sessions,
        &mut &*sessions,
        1 << 20,
        ResidentColdOpenInput::new(
            coordinate.clone(),
            AuthorityRecoveryReadLimits::new(1 << 20, 1 << 20, 8, 64).unwrap(),
            1 << 20,
            RecoveryReferenceKnowledge::Known(std::collections::BTreeSet::new()),
            &resolver,
            &mut source,
            ResidentReconstructionParameters::new(
                synthetic_set_policy(),
                8,
                8,
                WakeEpochV1::new(2),
            ),
        ),
    )
    .unwrap();
    let mut application = match opened {
        ResidentColdOpenOutcome::Ready(application) => application,
        ResidentColdOpenOutcome::Degraded(degraded) => panic!(
            "fixture authority journal quarantined at {:?}: {:?}",
            degraded.quarantine().first_untrusted_sequence(),
            degraded.quarantine().class(),
        ),
    };
    let (compilation, materializations, _) = generation;
    application
        .admit_generation(compilation, materializations, synthetic_set_policy())
        .unwrap();
    RestartFixture::new(application, signing)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    case: Case,
    saved: &ColdSnapshot,
    generations: [super::super::super::super::local_recovery::GenerationInputs; 2],
    file: &std::path::Path,
    sessions: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    fields: &ControlStateCheckpointPreimageV1,
    history_input: &history::Input<'_>,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) {
    let [generation, restarted_generation] = generations;
    let process = restart_process(sessions, coordinate, parents.signing, generation);
    let before = retained_custody_shape(port.connection);
    let advances = anchor.advance_count();
    let classified =
        history::with_recovery_admission(history_input, request, parents, |context, admission| {
            process.classify(request, context, port, sessions, anchor, admission)
        })
        .unwrap();
    assert_eq!(classified.response(), Some(&saved.response));
    assert!(classified.expiry_eligible());
    assert_eq!(retained_custody_shape(port.connection), before);
    assert_eq!(anchor.advance_count(), advances);
    port.connection
        .pragma_update(None, "query_only", false)
        .unwrap();

    let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
    let authority = checkpoint_authority(fields);
    let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let observation_signs = Cell::new(0);
    let checkpoint_signs = Cell::new(0);
    let signed_expiry = RefCell::new(None);
    let mut sign_observation =
        |_: &[u8], preimage: &OperationObservationPreimageV1, _: &OperationObservationDigest| {
            observation_signs.set(observation_signs.get() + 1);
            signed_expiry.replace(Some(preimage.clone()));
            Ok(signature())
        };
    let mut sign_checkpoint =
        |_: &[u8], _: &ControlStateCheckpointPreimageV1, _: &ControlStateCheckpointDigest| {
            checkpoint_signs.set(checkpoint_signs.get() + 1);
            Ok(signature())
        };
    let mut rotation =
        |_: &ResidentRecordRefV1,
         _: &ArtifactDigest,
         _: &ResidentRecordRefV1,
         _: &ArtifactDigest| { panic!("fixture expiry has no signer rotation") };
    let limits = AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap();
    let expiry_ref = record_ref("native/hook-expired");
    let expiry_time = history_input.operation.handle().1.expires_at().get();
    let clock = ExpiryClock(expiry_time);
    let mut other_runtime = parents
        .signing
        .integrated_runtime_authority()
        .unwrap()
        .clone();
    other_runtime.deployment_runtime_authority_ref = record_ref("runtime/other");
    let other_signing = SigningAuthorityBindingV1::integrated_runtime(
        parents.signing.principal_id().clone(),
        ResidentKeyIdV1::parse(parents.signing.signer_key_id()).unwrap(),
        *parents.signing.signer_key_epoch(),
        other_runtime,
    )
    .unwrap();
    // Time and current-signer refusals precede signing and physical changes.
    for (time, signer, refusal) in [
        (expiry_time - 1, parents.signing, None),
        (
            history_input.operation.handle().1.retain_until().get() + 1,
            parents.signing,
            Some("RetentionElapsed"),
        ),
        (
            expiry_time,
            &other_signing,
            Some("PreDispatchExpirySigningAuthority"),
        ),
    ] {
        let inputs = ResidentServicePreDispatchExpiryPreparationInputs {
            clock: &ExpiryClock(time),
            diagnostic_ref: record_ref("native/hook-expiry-diagnostic"),
            observation_ref: expiry_ref.clone(),
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: signer,
            },
            checkpoint: OperationCheckpointInput {
                reference: record_ref("checkpoint/hook-expired"),
                id: ControlCheckpointIdV1::parse("hook-expired").unwrap(),
            },
            checkpoint_schema: &fields.schema_sha256,
            checkpoint_authority: &authority,
            resolve_store: &resolve,
            maximum_diagnostic_bytes: 65_536,
            maximum_record_bytes: 65_536,
            accumulator_limits: limits,
            admit_rotation: &mut rotation,
            sign_observation: &mut sign_observation,
            sign_checkpoint: &mut sign_checkpoint,
        };
        let attempt = process.expire(&classified, port, sessions, anchor, inputs);
        if let Some(refusal) = refusal {
            assert!(attempt.err().unwrap().contains(refusal));
        } else {
            let not_expired = attempt.unwrap();
            assert!(!not_expired.complete() && !not_expired.pending());
        }
        assert_eq!(observation_signs.get(), 0);
        assert_eq!(checkpoint_signs.get(), 0);
        assert_eq!(retained_custody_shape(port.connection), before);
    }
    if matches!(case, Case::ColdExpiryRollback) {
        port.connection.execute_batch("CREATE TEMP TRIGGER reject_hook_expiry BEFORE INSERT ON pale_deployment_control_record WHEN NEW.record_ref = 'native/hook-expired' BEGIN SELECT RAISE(ABORT, 'fixture expiry storage unavailable'); END;").unwrap();
    }
    let inputs = ResidentServicePreDispatchExpiryPreparationInputs {
        clock: &clock,
        diagnostic_ref: record_ref("native/hook-expiry-diagnostic"),
        observation_ref: expiry_ref.clone(),
        signing: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &schema,
            signing_authority: parents.signing,
        },
        checkpoint: OperationCheckpointInput {
            reference: record_ref("checkpoint/hook-expired"),
            id: ControlCheckpointIdV1::parse("hook-expired").unwrap(),
        },
        checkpoint_schema: &fields.schema_sha256,
        checkpoint_authority: &authority,
        resolve_store: &resolve,
        maximum_diagnostic_bytes: 65_536,
        maximum_record_bytes: 65_536,
        accumulator_limits: limits,
        admit_rotation: &mut rotation,
        sign_observation: &mut sign_observation,
        sign_checkpoint: &mut sign_checkpoint,
    };
    let pending = if matches!(case, Case::ColdExpiryAnchor | Case::ColdExpiryHost) {
        process.expire(
            &classified,
            port,
            sessions,
            &mut ports::Anchor {
                inner: anchor,
                case: super::super::Case::PreparationAnchorFailure,
                advances: 0,
            },
            inputs,
        )
    } else {
        process.expire(&classified, port, sessions, anchor, inputs)
    }
    .unwrap();
    assert!(pending.pending());
    assert_eq!(observation_signs.get(), 1);
    assert_eq!(checkpoint_signs.get(), 1);
    if matches!(case, Case::ColdExpiryRollback) {
        assert_eq!(retained_custody_shape(port.connection), before);
        port.connection
            .execute_batch("DROP TRIGGER reject_hook_expiry")
            .unwrap();
    } else {
        assert_ne!(retained_custody_shape(port.connection), before);
        assert_eq!(anchor.advance_count(), advances);
    }
    let expiry = signed_expiry.into_inner().unwrap();
    assert!(matches!(
        expiry.state,
        OperationObservationStateV1::Expired { .. }
    ));
    let predecessor = saved.known.last().unwrap();
    let observation = issue_observation(
        parents.signing,
        history_input.operation.handle().0,
        history_input.operation.handle().1,
        schema,
        expiry.revision,
        Some((&predecessor.0, &predecessor.1)),
        expiry.state,
        expiry.observed_at.get(),
    );
    let mut known = history_input.known.to_vec();
    known.push((&expiry_ref, &observation));
    let input = history::Input {
        selected_member: (&expiry_ref, &observation),
        known: &known,
        ..*history_input
    };
    // New physical connection; the production continuation retains exact signed work.
    let mut reopened = connection(file);
    let mut resumed = SqliteOperationCustody {
        connection: &mut reopened,
        coordinate: coordinate.clone(),
    };
    let mut restarted =
        restart_process(sessions, coordinate, parents.signing, restarted_generation);
    if matches!(case, Case::ColdExpiryAnchor | Case::ColdExpiryHost) {
        let blocked =
            history::with_recovery_admission(&input, request, parents, |context, admission| {
                process.classify(request, context, &mut resumed, sessions, anchor, admission)
            });
        assert!(
            blocked.is_err(),
            "a committed expiry without independent anchoring cannot be admitted after reopening"
        );
        assert_eq!(resumed.connection.total_changes(), 0);
        assert_eq!(anchor.advance_count(), advances);
    }
    if matches!(case, Case::ColdExpiryAnchor | Case::ColdExpiryHost) {
        // Lose the entire signed in-memory phase before recovering stored work.
        drop(pending);
        resumed
            .connection
            .pragma_update(None, "query_only", true)
            .unwrap();
        if matches!(case, Case::ColdExpiryHost) {
            recovery::cold_sweep(
                &mut restarted,
                &input,
                request,
                parents,
                &mut resumed,
                sessions,
                anchor,
            );
        } else {
            recovery::complete(history_input, fields, &mut resumed, anchor);
        }
    } else {
        let complete = pending.continue_custody(
            &mut resumed,
            anchor,
            ResidentServicePreDispatchExpiryContinuationInputs {
                checkpoint_schema: &fields.schema_sha256,
                checkpoint_authority: &authority,
                resolve_store: &resolve,
                accumulator_limits: limits,
                sign_checkpoint: &mut sign_checkpoint,
            },
        );
        assert!(complete.complete());
    }
    assert_eq!(observation_signs.get(), 1);
    assert_eq!(checkpoint_signs.get(), 1);
    assert_eq!(anchor.advance_count(), advances + 1);
    if matches!(case, Case::ColdExpiryAnchor | Case::ColdExpiryHost) {
        assert_eq!(
            resumed.connection.total_changes(),
            0,
            "anchor continuation cannot repeat the committed expiry write"
        );
    }
    assert_eq!(
        resumed
            .read_operation_record(coordinate, saved.response.coordinate(), 65_536)
            .unwrap(),
        Some(saved.response.clone())
    );
    drop(resumed);
    drop(reopened);
    let mut reopened = connection(file);
    let mut resumed = SqliteOperationCustody {
        connection: &mut reopened,
        coordinate: coordinate.clone(),
    };
    resumed
        .connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    let classified =
        history::with_recovery_admission(&input, request, parents, |context, admission| {
            restarted.classify(request, context, &mut resumed, sessions, anchor, admission)
        })
        .unwrap();
    assert!(classified.response().is_none());
    assert!(!classified.expiry_eligible());
    assert!(classified.recovery_only());
    assert_eq!(resumed.connection.total_changes(), 0);
}
