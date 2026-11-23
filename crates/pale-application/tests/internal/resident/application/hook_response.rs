//! Real preparation/dispatch owners with explicit model authority and custody.
//! These tests establish ordering and binding, not production signing or host enforcement.

use super::*;
use crate::resident::operation_dispatch::hook_response::*;
use crate::resident::operation_observation_custody::tests::native_execution::with_service_custody_and_handle;
use crate::resident::{
    NativeHookPermissionRequest, NativeHookResponseAuthorityError, NativeHookResponseAuthorityPort,
};
use pale_ports::{
    AccumulatorPreparationLimits, OperationCustodyRecordKind, ResidentHookFrameResponse,
    WallClockSourceError, WallClockSourcePort,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Mutex,
};

struct Clock {
    now: UnixTimeV1,
    release_time: Mutex<Option<UnixTimeV1>>,
}
impl WallClockSourcePort for Clock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(self.release_time.lock().unwrap().unwrap_or(self.now))
    }
}

struct PermissionModel<'a> {
    request: CommonRequestEnvelopeV1,
    calls: usize,
    revoke: bool,
    expire_on_release: Option<&'a Clock>,
}
impl NativeHookResponseAuthorityPort for PermissionModel<'_> {
    fn admit_release(
        &mut self,
        request: &CommonRequestEnvelopeV1,
        response: &ResidentHookFrameResponse,
        _: UnixTimeV1,
    ) -> Result<(), NativeHookResponseAuthorityError> {
        assert_eq!(request, &self.request);
        assert!(matches!(
            response,
            ResidentHookFrameResponse::Prepared { .. }
        ));
        if let Some(clock) = self.expire_on_release {
            *clock.release_time.lock().unwrap() =
                Some(UnixTimeV1::new(clock.now.get() + 100_000).unwrap());
        }
        Ok(())
    }
    fn admit_action(
        &mut self,
        input: &NativeHookPermissionRequest<'_>,
    ) -> Result<ImmutableRecordReferenceV1, NativeHookResponseAuthorityError> {
        self.calls += 1;
        if input.request != &self.request || (self.revoke && self.calls > 1) {
            return Err(NativeHookResponseAuthorityError);
        }
        assert_eq!(input.binding.capability_id().as_str(), "read");
        assert!(input.binding.requirement().provider.binding().is_some());
        Ok(evidence("native-permission"))
    }
    fn admit_disclosure(
        &mut self,
        _: &NativeHookPermissionRequest<'_>,
        output: &crate::hooks::HookOutput,
    ) -> Result<ImmutableRecordReferenceV1, NativeHookResponseAuthorityError> {
        assert_eq!(output.exit_code, 0);
        assert!(output.stderr.is_empty());
        Ok(evidence("native-disclosure"))
    }
}
fn evidence(name: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: local::record_ref(name),
        record_sha256: ArtifactDigest::hash_bytes(name.as_bytes()),
    }
}

enum ResponseCase {
    Release,
    Revoke,
    AnchorFailure,
    CommitFailure,
    ExpireOnRelease,
    ProjectionContinuation,
    ChangedContinuationIdentity,
    ReturnedResponse,
}

fn response_case(case: ResponseCase) -> TestResult {
    let projection_failure = matches!(
        case,
        ResponseCase::ProjectionContinuation | ResponseCase::ChangedContinuationIdentity
    );
    let changed_identity = matches!(case, ResponseCase::ChangedContinuationIdentity);
    let returned_response = matches!(case, ResponseCase::ReturnedResponse);
    let revoke = matches!(case, ResponseCase::Revoke);
    let anchor_failure = matches!(case, ResponseCase::AnchorFailure);
    let commit_failure = matches!(case, ResponseCase::CommitFailure);
    let expire_on_release = matches!(case, ResponseCase::ExpireOnRelease);
    with_response_custody(
        NativeMapping::Read,
        &["call-1"],
        |app, history, fixture, current, native, admitted, operations, _, requests, states| {
            let mut registry = ResidentHookOperationRegistry::new(NonZeroUsize::new(1).unwrap());
            registry.register(
                native,
                admitted,
                &operations[0],
                current,
                &mut ProvingAuthority,
            )?;
            let correlated =
                registry.consume(native, app, history, current, &mut ProvingAuthority)?;
            let (candidate, checkpoint) = &states[0];
            let signing = ambient::signing_authority(
                &fixture.binding_ref,
                &fixture.binding,
                &fixture.runtime_ref,
                &fixture.runtime,
                &fixture.trust_ref,
                &fixture.trust,
            );
            let mut checkpoint_preimage = local::checkpoint_preimage(2, None);
            checkpoint_preimage.checkpoint_domain_id = checkpoint.checkpoint_domain_id().clone();
            checkpoint_preimage.checkpoint_profile_ref =
                checkpoint.checkpoint_profile_ref().clone();
            checkpoint_preimage.checkpoint_profile_sha256 =
                checkpoint.checkpoint_profile_sha256().clone();
            let checkpoint_authority = local::checkpoint_authority(&checkpoint_preimage);
            let schema = local::schema_digest("operation-observation.v1");
            let store = candidate.store().clone();
            let resolve_calls = Cell::new(0);
            let resolver = |_: &OperationObservationV1| {
                let count = resolve_calls.get();
                resolve_calls.set(count + 1);
                if projection_failure && count == 0 {
                    Err(crate::resident::OperationObservationAuthorityStoreRejection)
                } else {
                    Ok(store.clone())
                }
            };
            let clock = Clock {
                now: UnixTimeV1::new(checkpoint.issued_at().get() + 1)?,
                release_time: Mutex::new(None),
            };
            let mut permission = PermissionModel {
                request: requests[0].clone(),
                calls: 0,
                revoke,
                expire_on_release: expire_on_release.then_some(&clock),
            };
            let filesystem = pale_adapter_fs::LocalMachineFilesystem;
            let mut rotation = |_: &ResidentRecordRefV1,
                                _: &ArtifactDigest,
                                _: &ResidentRecordRefV1,
                                _: &ArtifactDigest| Ok(());
            let observation_signatures = Cell::new(0);
            let checkpoint_signatures = Cell::new(0);
            let mut observation_signer =
                |_: &[u8], _: &OperationObservationPreimageV1, _: &OperationObservationDigest| {
                    observation_signatures.set(observation_signatures.get() + 1);
                    Ok(local::signature())
                };
            let mut checkpoint_signer =
                |_: &[u8],
                 _: &ControlStateCheckpointPreimageV1,
                 _: &ControlStateCheckpointDigest| {
                    checkpoint_signatures.set(checkpoint_signatures.get() + 1);
                    Ok(local::signature())
                };
            let stopped = anchor_failure || commit_failure;
            with_service_custody_and_handle(
                checkpoint,
                Some(candidate),
                None,
                if stopped || revoke { 1 } else { 2 },
                if commit_failure {
                    0
                } else if anchor_failure || revoke {
                    1
                } else {
                    2
                },
                anchor_failure,
                if commit_failure { &[1] } else { &[] },
                candidate
                    .record(OperationCustodyRecordKind::OperationHandle)
                    .cloned(),
                Rc::new(RefCell::new(None)),
                |custody, anchor| {
                    let mut retained = None;
                    let mut execute = |retained: &mut Option<NativeHookResponseContinuation>,
                                       substitute| {
                        let inputs = NativeHookResponseInputs {
                            authority: &mut permission,
                            filesystem: &filesystem,
                            clock: &clock,
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &schema,
                                signing_authority: &signing,
                            },
                            checkpoint_schema: &checkpoint_preimage.schema_sha256,
                            checkpoint_authority: &checkpoint_authority,
                            resolve_store: &resolver,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                32,
                                1024,
                                1024,
                                512,
                                1 << 20,
                            )
                            .unwrap(),
                            maximum_record_bytes: 1 << 20,
                            response_ref: local::record_ref(if substitute {
                                "native/substituted-response"
                            } else {
                                "native/response"
                            }),
                            preparation_ref: local::record_ref("native/preparation"),
                            dispatch_ref: local::record_ref("native/dispatch"),
                            attempt_id: CapabilityAttemptIdV1::parse("native-response-attempt")
                                .unwrap(),
                            preparation_checkpoint:
                                crate::resident::operation_dispatch::OperationCheckpointInput {
                                    reference: local::record_ref("native/checkpoint-2"),
                                    id: ControlCheckpointIdV1::parse("native-checkpoint-2")
                                        .unwrap(),
                                },
                            dispatch_checkpoint:
                                crate::resident::operation_dispatch::OperationCheckpointInput {
                                    reference: local::record_ref("native/checkpoint-3"),
                                    id: ControlCheckpointIdV1::parse("native-checkpoint-3")
                                        .unwrap(),
                                },
                            context_binding: None,
                            admit_rotation: &mut rotation,
                            sign_observation: &mut observation_signer,
                            sign_checkpoint: &mut checkpoint_signer,
                        };
                        prepare_and_release_native_hook_response(
                            NativeHookResponseTurn {
                                correlated: &correlated,
                                request: &requests[0],
                                application: app,
                                current,
                                start: Some(NativeHookResponseStart {
                                    predecessor: operations[0].initial_observation(),
                                    checkpoint: (
                                        candidate
                                            .checkpoint()
                                            .successor()
                                            .coordinate()
                                            .record_ref(),
                                        checkpoint,
                                    ),
                                }),
                                custody,
                                anchor,
                            },
                            inputs,
                            retained,
                            &mut ProvingAuthority,
                            |authority, permission, filesystem| {
                                let current = &current.at_time(clock.now);
                                correlated
                                    .operation()
                                    .revalidate_session(current, authority)
                                    .map_err(|_| NativeHookResponseFailure::Authority)?;
                                correlated
                                    .admit_response_policy(
                                        &requests[0],
                                        app,
                                        current.now,
                                        filesystem,
                                        permission,
                                    )
                                    .map_err(|_| NativeHookResponseFailure::Authority)
                            },
                        )
                    };
                    if projection_failure {
                        assert!(matches!(
                            execute(&mut retained, false),
                            Err(NativeHookResponseFailure::Custody)
                        ));
                        assert_eq!(observation_signatures.get(), 1);
                        assert_eq!(checkpoint_signatures.get(), 0);
                        if changed_identity {
                            assert!(matches!(
                                execute(&mut retained, true),
                                Err(NativeHookResponseFailure::Release)
                            ));
                            assert_eq!(observation_signatures.get(), 1);
                            assert_eq!(checkpoint_signatures.get(), 0);
                        }
                    }
                    let result = execute(&mut retained, false);
                    if returned_response {
                        assert!(result.is_ok());
                        assert!(matches!(
                            execute(&mut retained, false),
                            Err(NativeHookResponseFailure::Release)
                        ));
                        assert_eq!(observation_signatures.get(), 2);
                        assert_eq!(checkpoint_signatures.get(), 2);
                    }
                    let expected_signatures = if stopped || revoke { 1 } else { 2 };
                    assert_eq!(observation_signatures.get(), expected_signatures);
                    assert_eq!(checkpoint_signatures.get(), expected_signatures);
                    if stopped {
                        assert!(
                            matches!(result, Err(NativeHookResponseFailure::Custody)),
                            "{result:?}"
                        );
                    } else if revoke || expire_on_release {
                        assert!(
                            matches!(result, Err(NativeHookResponseFailure::Release)),
                            "{result:?}"
                        );
                    } else {
                        let ResidentHookFrameResponse::Prepared {
                            records,
                            response_canonical,
                        } = result.unwrap()
                        else {
                            panic!("expected admitted response")
                        };
                        let response: pale_ports::ResidentHookResponseRecord =
                            serde_json::from_str(&response_canonical).unwrap();
                        assert_eq!(response.exit_code, 0);
                        assert_eq!(
                            response.native_request_sha256,
                            requests[0].body().payload_sha256
                        );
                        let observation: serde_json::Value =
                            serde_json::from_str(records.operation_observation_canonical())
                                .unwrap();
                        assert_eq!(observation["state"]["phase"]["phase"], "dispatched");
                        assert_eq!(
                            observation["state"]["phase"]["evidence"]["record_sha256"],
                            pale_core::sha256_hex(response_canonical.as_bytes())
                        );
                    }
                },
            );
            assert_eq!(
                permission.calls,
                if stopped {
                    1
                } else if revoke {
                    2
                } else {
                    3
                }
            );
            Ok(())
        },
    )
}

#[test]
fn native_response_releases_only_after_both_anchored_phases() -> TestResult {
    response_case(ResponseCase::Release)
}
#[test]
fn native_response_withdrawn_permission_does_not_release() -> TestResult {
    response_case(ResponseCase::Revoke)
}
#[test]
fn native_response_anchor_failure_retains_custody_without_output() -> TestResult {
    response_case(ResponseCase::AnchorFailure)
}
#[test]
fn native_response_commit_conflict_does_not_release() -> TestResult {
    response_case(ResponseCase::CommitFailure)
}

#[test]
fn native_response_expiry_during_final_admission_does_not_release() -> TestResult {
    response_case(ResponseCase::ExpireOnRelease)
}

#[test]
fn native_response_projection_failure_continues_without_resigning() -> TestResult {
    response_case(ResponseCase::ProjectionContinuation)
}
#[test]
fn native_response_continuation_rejects_replacement_identity() -> TestResult {
    response_case(ResponseCase::ChangedContinuationIdentity)
}
#[test]
fn native_response_returned_frame_cannot_be_automatically_reissued() -> TestResult {
    response_case(ResponseCase::ReturnedResponse)
}
