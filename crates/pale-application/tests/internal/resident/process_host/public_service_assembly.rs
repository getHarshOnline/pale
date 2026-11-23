//! Public resident-service assembly over the existing application owners.

use super::*;
use crate::local_operation_start_fixture::schema_digest;
use crate::resident::deployment_finalization::tests::recovery::{
    with_evidence_for_deployment, StoredChange,
};
use crate::resident::journal::tests::event_custody::UnavailableEventCustody;
use crate::resident::operation_observation_custody::tests::local_start::{
    absent_start_store, initial_anchor, initial_response, source_generation,
    source_generation_copies_with_reconstruction,
};
use crate::resident::principal_trust_frontier::tests::recovered_cold_start_trust_frontier;
use crate::resident::process_host::service_assembly::ResidentServiceInvokeOwners;
use crate::resident::process_host::tests::invoke_request::with_unreachable_restart_inputs;
use crate::resident::process_host::tests::program::{
    checkpoint_for_writer, writer_snapshot, HeldWriter,
};
use crate::resident::{
    GenerationLifecycleRecoveryResolver, LocalOperationPublicationPolicy,
    LocalOperationPublicationPolicyInput, ResidentServiceCallerSignaturePort,
    ResidentServiceCurrentRequestEvidence, ResidentServiceCurrentRequestEvidencePort,
    ResidentServiceCurrentRequestSelector, ResidentServiceCurrentWriterAdmissionPort,
    ResidentServiceCurrentWriterEvidence, ResidentServiceDelegatePolicyContext,
    ResidentServiceDelegatePolicyPort, ResidentServiceDeploymentOpen,
    ResidentServiceInvokeCryptographyPort, ResidentServiceInvokeInputPort,
    ResidentServiceInvokePolicyContext, ResidentServiceInvokePolicyPort,
    ResidentServiceInvokeRecoveryInputs, ResidentServiceInvokeRecoveryLimits,
    ResidentServiceObserveInputPort, ResidentServiceOperationRequestPolicies,
    ResidentServiceOperationStartPolicy, ResidentServicePolicyInput,
    ResidentServicePolicyProjection, ResidentServiceProcessInput, ResidentServiceTaskLimits,
};
use crate::resident_fixture::{synthetic_set_policy, FixedInputSource, RecoveryResolver};
use crate::session_admission_fixture::ProvingAuthority;
use pale_core::records::*;
use pale_ports::{
    AccumulatorPreparationLimits, ControlRecordCoordinate, CryptographicRandomSourceError,
    CryptographicRandomSourcePort, OperationCustodyCommitOutcome, PrincipalResidentWriterSnapshot,
    ResidentObserveFrameFailure, ResidentOperationFrameFailure, ResidentOperationFrameLocator,
    ResidentOperationFrameResponse, WallClockSourceError, WallClockSourcePort,
};
use std::cell::RefCell;
use std::collections::{BTreeSet, VecDeque};
use std::io::Write;
use std::num::NonZeroUsize;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

const MAXIMUM_APPLICATION_INVOKE_FIXTURE_BYTES: usize = 1 << 20;

fn application_invoke_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/resident/application-invoke-sdk.v1.json")
}

fn normalize_application_invoke_machine_identity(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text)
            if text.len() == 64
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) =>
        {
            *text = "<sha256>".to_owned();
        }
        serde_json::Value::String(text) => {
            let Ok(mut nested) = serde_json::from_str::<serde_json::Value>(text) else {
                return;
            };
            if !nested.is_object() && !nested.is_array() {
                return;
            }
            normalize_application_invoke_machine_identity(&mut nested);
            *text = serde_json::to_string(&nested).expect("normalized canonical record serializes");
        }
        serde_json::Value::Array(values) => {
            for value in values {
                normalize_application_invoke_machine_identity(value);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                normalize_application_invoke_machine_identity(value);
            }
        }
        _ => {}
    }
}

fn assert_or_update_application_invoke_fixture(
    request_frame: &ResidentOperationFrameRequest,
    response_frame: &ResidentOperationFrameResponse,
) {
    let actual = serde_json::json!({
        "fixture_version": 1,
        "request_frame": request_frame,
        "response_frame": response_frame,
    });
    let mut actual_bytes = serde_json::to_vec_pretty(&actual).expect("fixture serializes");
    actual_bytes.push(b'\n');
    assert!(
        actual_bytes.len() <= MAXIMUM_APPLICATION_INVOKE_FIXTURE_BYTES,
        "application Invoke fixture exceeds its byte ceiling"
    );

    let path = application_invoke_fixture_path();
    if std::env::var_os("PALE_UPDATE_CONFORMANCE").as_deref() == Some("1".as_ref()) {
        let parent = path.parent().expect("fixture has an owner directory");
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).expect("create bounded fixture update");
        temporary
            .write_all(&actual_bytes)
            .expect("write bounded fixture update");
        temporary
            .flush()
            .expect("flush bounded fixture update to the temporary file");
        temporary
            .persist(&path)
            .unwrap_or_else(|error| panic!("atomically replace application fixture: {error}"));
        return;
    }

    let expected_bytes = std::fs::read(&path).expect("application Invoke fixture is present");
    assert!(
        expected_bytes.len() <= MAXIMUM_APPLICATION_INVOKE_FIXTURE_BYTES,
        "stored application Invoke fixture exceeds its byte ceiling"
    );
    let expected: serde_json::Value =
        serde_json::from_slice(&expected_bytes).expect("application Invoke fixture parses");
    let mut expected_pretty =
        serde_json::to_vec_pretty(&expected).expect("stored fixture serializes");
    expected_pretty.push(b'\n');
    assert_eq!(
        expected_bytes, expected_pretty,
        "stored application Invoke fixture uses the canonical pretty form"
    );
    let mut comparable_actual = actual;
    let mut comparable_expected = expected;
    // Source-backed compilation carries machine-local path identities. The
    // application proves every exact digest join before this stable fixture
    // comparison; upper surfaces consume the unmodified recorded frames.
    normalize_application_invoke_machine_identity(&mut comparable_actual);
    normalize_application_invoke_machine_identity(&mut comparable_expected);
    assert_eq!(
        comparable_actual, comparable_expected,
        "application Invoke output matches the public fixture"
    );
}

struct ServiceWriterAdmission {
    writer: PrincipalResidentWriterFenceV1,
    checkpoint: ControlStateCheckpointV1,
}

impl ResidentServiceCurrentWriterAdmissionPort for ServiceWriterAdmission {
    fn admit_current_writer(
        &mut self,
        _: &PrincipalResidentWriterSnapshot,
    ) -> Result<ResidentServiceCurrentWriterEvidence, InvalidResidentRecordV1> {
        Ok(ResidentServiceCurrentWriterEvidence {
            writer: self.writer.clone(),
            checkpoint: self.checkpoint.clone(),
        })
    }
}

struct ServiceEvidencePort {
    values: VecDeque<ResidentServiceCurrentRequestEvidence>,
    reads: Arc<AtomicUsize>,
}

impl ResidentServiceCurrentRequestEvidencePort for ServiceEvidencePort {
    fn read_current_request_evidence(
        &mut self,
        _: ResidentServiceCurrentRequestSelector<'_>,
    ) -> Result<ResidentServiceCurrentRequestEvidence, InvalidResidentRecordV1> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.values
            .pop_front()
            .ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "service request evidence",
                invariant: "the fixture has no remaining current evidence",
            })
    }
}

struct ServiceSignature;

impl ResidentServiceCallerSignaturePort for ServiceSignature {
    fn verify_caller_signature(
        &mut self,
        _: &CommonRequestEnvelopePreimageV1,
        _: &CommonRequestEnvelopeDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }
}

struct Clock(Mutex<VecDeque<UnixTimeV1>>);

impl WallClockSourcePort for Clock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.0
            .lock()
            .expect("clock queue")
            .pop_front()
            .ok_or_else(|| {
                WallClockSourceError::from_source(std::io::Error::other("clock queue is empty"))
            })
    }
}

struct ServiceEntropy(std::sync::atomic::AtomicU8);

impl CryptographicRandomSourcePort for ServiceEntropy {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        let byte = self
            .0
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            .checked_add(1)
            .expect("bounded fixture entropy calls");
        destination.fill(byte);
        Ok(())
    }
}

struct UnavailableObserveInputs;

impl<'recovery> ResidentServiceObserveInputPort<'recovery> for UnavailableObserveInputs {
    fn with_observe_inputs<Output, UseInputs>(
        &mut self,
        _: &ControlRecordCoordinate,
        _: UseInputs,
    ) -> Result<Output, ResidentObserveFrameFailure>
    where
        UseInputs: for<'request> FnOnce(
            crate::resident::ResidentServiceObserveInputs<'request, 'recovery>,
        ) -> Output,
    {
        Err(ResidentObserveFrameFailure::ResidentUnavailable)
    }
}

struct StartInputs<'a> {
    semantic: &'a crate::resident::LocalOperationStartInput<'a>,
}

impl ResidentServiceInvokeInputPort for StartInputs<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        locator: Option<&ControlRecordCoordinate>,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        assert!(locator.is_none(), "fresh StartInvoke has no locator");
        Ok(with_unreachable_restart_inputs(
            self.semantic,
            |restart, recovery_admission| {
                use_inputs(ResidentServiceInvokeRecoveryInputs {
                    operation: restart.operation,
                    maximum_parent_depth: restart.maximum_parent_depth,
                    limits: ResidentServiceInvokeRecoveryLimits {
                        observation: restart.current_head.observation,
                        checkpoints: restart.current_head.checkpoints,
                        membership: restart.current_head.membership,
                        maximum_selection_bytes: restart.maximum_selection_bytes,
                    },
                    recovery_admission,
                    checkpoint_authority: self.semantic.checkpoint_authority,
                })
            },
        ))
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            crate::resident::ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }
}

struct UnavailableInvokeInputs;

impl ResidentServiceInvokeInputPort for UnavailableInvokeInputs {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<&ControlRecordCoordinate>,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            ResidentServiceInvokeRecoveryInputs<'operation, 'admission, 'checkpoint>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }

    fn with_background_invoke_inputs<Output, UseInputs>(
        &mut self,
        _: &crate::resident::ResidentServiceDeploymentSelector,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission> FnOnce(
            crate::resident::ResidentServiceInvokeBackgroundRecoveryInputs<'operation, 'admission>,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    }
}

#[test]
fn existing_invoke_input_owner_does_not_claim_provider_restart_recovery() {
    let selector = crate::resident::ResidentServiceDeploymentSelector {
        principal_id: ResidentPrincipalIdV1::parse("principal-provider-recovery")
            .expect("principal ID"),
        deployment_binding_ref: ResidentRecordRefV1::parse("deployment-provider-recovery")
            .expect("deployment reference"),
        deployment_binding_sha256: DeploymentBindingDigest::parse("a".repeat(64))
            .expect("deployment digest"),
        runtime_authority_ref: ResidentRecordRefV1::parse("runtime-provider-recovery")
            .expect("runtime reference"),
        runtime_authority_sha256: DeploymentRuntimeAuthorityDigest::parse("b".repeat(64))
            .expect("runtime digest"),
    };
    let locator = ResidentOperationFrameLocator::new(
        "operation-provider-recovery".to_owned(),
        "c".repeat(64),
    );
    let callback_ran = std::cell::Cell::new(false);
    let result = UnavailableInvokeInputs
        .with_provider_recovery_inputs(&selector, &locator, |_| callback_ran.set(true));

    assert_eq!(
        result,
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    );
    assert!(!callback_ran.get());
}

struct ExactPolicy(Option<ResidentServiceOperationStartPolicy>);

impl ResidentServiceInvokePolicyPort for ExactPolicy {
    fn evaluate_invoke(
        &mut self,
        _: ResidentServiceInvokePolicyContext<'_>,
    ) -> Result<ResidentServiceOperationStartPolicy, InvalidResidentRecordV1> {
        self.0
            .take()
            .ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "service Invoke policy",
                invariant: "the fixture policy was already consumed",
            })
    }
}

impl ResidentServiceDelegatePolicyPort for ExactPolicy {
    fn evaluate_delegate(
        &mut self,
        _: ResidentServiceDelegatePolicyContext<'_>,
    ) -> Result<ResidentServiceOperationStartPolicy, InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "service Delegate policy",
            invariant: "this fixture admits only root Invoke",
        })
    }
}

struct StartCryptography {
    checkpoint: RefCell<Option<ControlStateCheckpointPreimageV1>>,
}

impl ResidentServiceInvokeCryptographyPort for StartCryptography {
    fn admit_compiled_authority(
        &self,
        _: &CommonRequestEnvelopeV1,
        _: &crate::resident::AdmittedAmbientSession,
        _: &crate::resident::AmbientSessionAdmissionContext<'_>,
        _: &OperationHandlePreimageV1,
        _: &OperationLeaseV1,
        _: &crate::semantic::ResolvedOperation,
        _: &LocalOperationPublicationPolicyInput<'_>,
        _: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }

    fn sign_handle(
        &self,
        _: &[u8],
        _: &OperationHandlePreimageV1,
        _: &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        Ok(crate::local_operation_start_fixture::signature())
    }

    fn sign_observation(
        &self,
        _: &[u8],
        _: &OperationObservationPreimageV1,
        _: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        Ok(crate::local_operation_start_fixture::signature())
    }

    fn prepare_checkpoint(
        &self,
        prepared: &crate::resident::PreparedLocalOperationCheckpoint,
        checkpoint_id: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, crate::resident::LocalOperationStartError> {
        let mut checkpoint = self
            .checkpoint
            .borrow_mut()
            .take()
            .ok_or(crate::resident::LocalOperationStartError::Coordinate)?;
        checkpoint.control_checkpoint_id = checkpoint_id.clone();
        let issued_at = checkpoint.issued_at;
        let mut checkpoint = super::start::checkpoint_preimage(prepared, checkpoint)?;
        checkpoint.issued_at = issued_at;
        Ok(checkpoint)
    }

    fn sign_checkpoint(
        &self,
        _: &[u8],
        _: &ControlStateCheckpointPreimageV1,
        _: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        Ok(crate::local_operation_start_fixture::signature())
    }
}

struct Historical {
    records: RecoveryResolver,
    session_signing_authority: SigningAuthorityBindingV1,
}

impl GenerationLifecycleRecoveryResolver for Historical {
    fn session_signing_authority(
        &self,
        _: &ResidentRecordRefV1,
        _: &ArtifactDigest,
        _: &ResidentPrincipalIdV1,
        _: &ResidentKeyIdV1,
        _: SigningKeyEpochV1,
    ) -> Option<SigningAuthorityBindingV1> {
        Some(self.session_signing_authority.clone())
    }

    fn verify_session_admission_lease_signature(
        &self,
        _: &SigningAuthorityBindingV1,
        _: &[u8],
        _: &SessionAdmissionLeasePreimageV1,
        _: &SessionAdmissionLeaseDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }

    fn compiled_generation(
        &self,
        digest: &CompiledGenerationDigest,
    ) -> Option<&CompiledGenerationV1> {
        self.records.generations.get(digest)
    }

    fn published_head(
        &self,
        digest: &PublishedGenerationHeadDigest,
    ) -> Option<&PublishedGenerationHeadV1> {
        self.records.published.get(digest)
    }

    fn advertised_head(
        &self,
        digest: &AdvertisedGenerationHeadDigest,
    ) -> Option<&AdvertisedGenerationHeadV1> {
        self.records.advertised.get(digest)
    }

    fn active_head(&self, digest: &ActiveGenerationHeadDigest) -> Option<&ActiveGenerationHeadV1> {
        self.records.active.get(digest)
    }

    fn activation_fence(&self, digest: &ActivationFenceDigest) -> Option<&ActivationFenceV1> {
        self.records.fences.get(digest)
    }

    fn effective_profile(&self, digest: &ProfileDigest) -> Option<&EffectiveProjectProfileV1> {
        self.records.profiles.get(digest)
    }
}

fn service_evidence_from(
    current: &crate::resident::AmbientSessionAdmissionContext<'_>,
) -> ResidentServiceCurrentRequestEvidence {
    let grant = current.grant.as_ref().expect("fixture grant");
    let capability = current
        .capability_revocations
        .as_ref()
        .expect("fixture capability frontier");
    ResidentServiceCurrentRequestEvidence {
        grant: crate::resident::ResidentServiceGrantEvidence {
            grant_ref: grant.grant_ref.clone(),
            grant: grant.grant.clone(),
            issued_trust_checkpoint_ref: grant.issued_trust_checkpoint_ref.clone(),
            issued_trust_checkpoint: grant.issued_trust_checkpoint.clone(),
        },
        capability_revocations: crate::resident::ResidentServiceCapabilityRevocationEvidence {
            checkpoint_ref: capability.checkpoint_ref.clone(),
            checkpoint: capability.checkpoint.clone(),
            grant_head: capability.grant_head.map(|head| {
                crate::resident::ResidentServiceGrantRevocationEvidence {
                    revocation_ref: head.revocation_ref.clone(),
                    revocation: head.revocation.clone(),
                }
            }),
        },
        deployment_revocations: crate::resident::ResidentServiceDeploymentRevocationEvidence {
            epoch: current.deployment_revocations.epoch,
            binding_head: current.deployment_revocations.binding_head.map(|head| {
                crate::resident::ResidentServiceDeploymentRevocationHeadEvidence {
                    revocation_ref: head.revocation_ref.clone(),
                    revocation: head.revocation.clone(),
                }
            }),
        },
        session_revocations: crate::resident::ResidentServiceSessionRevocationEvidence {
            epoch: current.session_revocations.epoch,
            revocation_ref: current.session_revocations.revocation_ref.cloned(),
            revocation_sha256: current.session_revocations.revocation_sha256.cloned(),
        },
    }
}

fn policy(
    generation: &crate::resident::AdmittedCompiledGeneration,
    repository_id: LogicalRepositoryIdV1,
    request: &CommonRequestEnvelopeV1,
    body_schema_id: SchemaId,
    body_schema_sha256: SchemaDigest,
) -> ResidentServicePolicyProjection {
    let value = request.to_preimage();
    ResidentServicePolicyProjection::admit(
        generation,
        ResidentServicePolicyInput {
            repository_id,
            body_schema_id,
            body_schema_sha256,
            caller: value.caller,
            signing_authority: value.signing_authority_binding,
            route: value.route,
            execution_owner_fence: None,
            federation_claim: value.federation_claim,
            workspace_id: value.workspace_id,
            capability_id: value.capability_id,
            operation: value.operation,
            scope: value.scope,
            risk_class: value.risk_class,
            resource_constraints: value.resource_constraints,
            expected_precondition: value.expected_precondition,
            maximum_request_lifetime_seconds: 30,
            fence_token: value.fence_token,
        },
    )
    .expect("service policy is admitted from the retained generation")
}

#[test]
fn service_policy_rejects_changed_body_digest_and_request_identity() {
    let (generation, profile) = source_generation();
    let generation_digest = generation.record().compiled_generation_digest().clone();
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "public-service-policy",
        |request| request.generation = Some(generation_digest),
        |_, request, _, parents| {
            let request_preimage = request.to_preimage();
            let retained = policy(
                &generation,
                parents.session.lease().repository_id().clone(),
                request,
                request_preimage.body.schema_id.clone(),
                request_preimage.body.schema_sha256.clone(),
            )
            .into_inner();
            assert!(retained.matches_fixed_claim(&request_preimage));

            let mut changed_digest = request_preimage.clone();
            changed_digest.body.schema_sha256 =
                SchemaDigest::parse("0".repeat(64)).expect("different schema digest");
            assert!(
                !retained.matches_fixed_claim(&changed_digest),
                "the same schema ID with different bytes must not satisfy policy"
            );

            let mut substituted_request = request_preimage;
            substituted_request.operation = PackageResourceRefV1 {
                package_id: PackageIdV1::parse("other.package").expect("other Package"),
                resource_id: PackageResourceIdV1::parse("other-operation")
                    .expect("other operation"),
            };
            assert!(
                !retained.matches_fixed_claim(&substituted_request),
                "an unrelated request identity must not satisfy policy"
            );
        },
    );
}

#[test]
fn recovered_invoke_service_starts_one_durable_operation() {
    with_evidence_for_deployment(
        StoredChange::None,
        "control-store",
        "incarnation-1",
        "deployment-binding-1",
        "runtime-authority-1",
        |finalization_snapshot, finalization, _, _, _, _, _| {
            let recovered_trust =
                recovered_cold_start_trust_frontier(finalization_snapshot, finalization);
            let current_trust = recovered_trust.current_checkpoint();
            let runtime_authority = recovered_trust
                .runtime_authorities()
                .first()
                .expect("finalized deployment retains runtime authority");
            let trust = crate::resident::ResidentServiceTrustFrontier::from_recovered_fixture(
                recovered_trust.clone(),
            );
            let store = AuthorityStoreCoordinate::new(
                runtime_authority.authority().control_store_id().clone(),
                runtime_authority
                    .authority()
                    .control_incarnation_id()
                    .clone(),
            );
            let (mut generations, profile, reconstruction) =
                source_generation_copies_with_reconstruction(2);
            let retained = generations.pop().expect("retained generation");
            let semantic = generations.pop().expect("semantic generation");
            crate::local_operation_start_fixture::with_invoke_start_input_at_store_and_authorities(
                &semantic,
                &profile,
                "public-service-start",
                crate::local_operation_start_fixture::native_request_body,
                &store,
                current_trust.coordinate().record_ref(),
                current_trust.checkpoint(),
                runtime_authority.binding_ref(),
                runtime_authority.binding(),
                runtime_authority.record_ref(),
                runtime_authority.authority(),
                |start, request, checkpoint_preimage, parents| {
                    let application_store =
                        crate::resident::journal::tests::MemoryAuthorityStore::default();
                    let mut application = ready_application(&application_store, store.clone(), 112);
                    let published = crate::resident_fixture::published(
                        parents.current.generation.compiled_generation,
                        1,
                        None,
                    );
                    let advertised = crate::resident_fixture::advertised(&published, 2, None);
                    application.fixture_retain_generation(retained);
                    let _ = application
                        .publish(published.clone())
                        .expect("publish generation");
                    let _ = application
                        .advertise(advertised.clone())
                        .expect("advertise generation");
                    let _ = application
                        .activate(
                            parents.current.generation.active_head.clone(),
                            parents.current.generation.activation_fence.clone(),
                        )
                        .expect("activate generation");
                    let _ = application
                        .admit_session(
                            SessionAdmissionLeaseWireV1::from(parents.session.lease()),
                            parents.current,
                            &mut ProvingAuthority,
                        )
                        .expect("retain exact session");
                    application.checkpoint().expect("checkpoint resident state");
                    drop(application);

                    let mut records = RecoveryResolver::default();
                    records.insert_generation(parents.current.generation.compiled_generation);
                    records.insert_effective_profile(parents.current.generation.effective_profile);
                    records.insert_published(&published);
                    records.insert_advertised(&advertised);
                    records.insert_fence(parents.current.generation.activation_fence);
                    records.insert_active(parents.current.generation.active_head);
                    let historical = Historical {
                        records,
                        session_signing_authority: parents.signing.clone(),
                    };
                    let mut source = FixedInputSource {
                        calls: 0,
                        inputs: vec![reconstruction],
                        requested: None,
                    };
                    let mut events = UnavailableEventCustody;
                    let cold_open = crate::resident::ResidentColdOpenInput::new(
                        store.clone(),
                        crate::resident::journal::tests::limits(),
                        crate::resident::journal::tests::limits().maximum_entry_bytes(),
                        crate::resident::RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                        &historical,
                        &mut source,
                        crate::resident::ResidentReconstructionParameters::new(
                            synthetic_set_policy(),
                            4,
                            4,
                            WakeEpochV1::new(203),
                        ),
                    );
                    let writer = crate::local_operation_start_fixture::publication_writer();
                    let checkpoint = checkpoint_for_writer(&writer, 'c');
                    let snapshot = writer_snapshot(&writer, &checkpoint);
                    let mut custody = absent_start_store(request)
                        .at_store(&store)
                        .with_unlisted_indexes_absent()
                        .with_lease_retention(Ok(OperationCustodyCommitOutcome::Committed))
                        .with_preparation(
                            initial_response,
                            ControlTransactionSequenceV1::new(1)
                                .expect("initial transaction sequence"),
                        );
                    let mut anchor = initial_anchor();
                    let request_preimage = request.to_preimage();
                    let repository = parents.session.lease().repository_id().clone();
                    let request_policies = ResidentServiceOperationRequestPolicies {
                        start: vec![policy(
                            &semantic,
                            repository.clone(),
                            request,
                            request_preimage.body.schema_id.clone(),
                            request_preimage.body.schema_sha256.clone(),
                        )],
                        advance: vec![policy(
                            &semantic,
                            repository.clone(),
                            request,
                            SchemaId::parse(CAPABILITY_RESULT_V1_SCHEMA_ID)
                                .expect("registered placeholder schema"),
                            schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID),
                        )],
                        start_consult: Vec::new(),
                        advance_consult: Vec::new(),
                        start_delegate: Vec::new(),
                        advance_delegate: Vec::new(),
                        observe: vec![policy(
                            &semantic,
                            repository,
                            request,
                            SchemaId::parse(DIAGNOSTIC_V1_SCHEMA_ID)
                                .expect("registered placeholder schema"),
                            schema_digest(DIAGNOSTIC_V1_SCHEMA_ID),
                        )],
                        cancel: Vec::new(),
                    };
                    let lease = parents.lease;
                    let invoke_policy = ResidentServiceOperationStartPolicy {
                        checkpoint_profile: start.checkpoint_profile.clone(),
                        publication_policy: LocalOperationPublicationPolicy::from_admitted_input(
                            LocalOperationPublicationPolicyInput {
                                target_scope: start.publication_policy.target_scope,
                                classification: start.publication_policy.classification,
                                visibility: start.publication_policy.visibility,
                                disclosure_ceiling: start.publication_policy.disclosure_ceiling,
                                payload_resolution_policy: start
                                    .publication_policy
                                    .payload_resolution_policy,
                                actor_provenance: start.publication_policy.actor_provenance,
                            },
                        ),
                        maximum_record_bytes: 1 << 20,
                        lease: crate::resident::ResidentServiceOperationLeasePolicy {
                            capabilities: lease.capabilities().to_vec(),
                            effects: lease.effects().to_vec(),
                            read_set: lease.read_set().to_vec(),
                            evidence_obligations: lease.evidence_obligations().to_vec(),
                            budget_units: *lease.budget_units(),
                            timeout_millis: *lease.timeout_millis(),
                            expires_at: lease.expires_at(),
                            invocation_correlation: lease.invocation_correlation().clone(),
                        },
                        handle_expires_at: start.handle.expires_at,
                        handle_retain_until: start.handle.retain_until,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            4, 1024, 1024, 512, 65_536,
                        )
                        .expect("bounded accumulator"),
                    };
                    let process = ResidentServiceProcessInput {
                        held_writer: HeldWriter(snapshot.binding().principal_id().clone()),
                        writer_snapshot: &snapshot,
                        writer_admission: ServiceWriterAdmission { writer, checkpoint },
                        trust: &trust,
                        deployments: vec![ResidentServiceDeploymentOpen {
                            store: store.clone(),
                            durable: &application_store,
                            sessions: &application_store,
                            events: &mut events,
                            maximum_event_custody_bytes: 65_536,
                            cold_open,
                            operations: &mut custody,
                            anti_rollback: &mut anchor,
                        }],
                        maximum_ready_deployments: NonZeroUsize::new(1).expect("one deployment"),
                        task_limits: ResidentServiceTaskLimits {
                            maximum_provider_tasks: NonZeroUsize::new(1)
                                .expect("one provider task"),
                            maximum_consultation_batches: NonZeroUsize::new(1)
                                .expect("one consultation batch"),
                            maximum_aggregate_tasks: NonZeroUsize::new(1)
                                .expect("one aggregate task"),
                        },
                    };
                    let now = parents.current.now;
                    let evidence_reads = Arc::new(AtomicUsize::new(0));
                    let owners = ResidentServiceInvokeOwners {
                        peer_admission: Box::new(ExactTestPeer::new(
                            LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                        )),
                        maximum_request_bytes: 1 << 20,
                        request_policies,
                        historical: &historical,
                        evidence: ServiceEvidencePort {
                            values: VecDeque::from([service_evidence_from(parents.current)]),
                            reads: Arc::clone(&evidence_reads),
                        },
                        clock: Clock(Mutex::new(VecDeque::from([now]))),
                        session_authority: ProvingAuthority,
                        caller_signature: ServiceSignature,
                        random: ServiceEntropy(std::sync::atomic::AtomicU8::new(0)),
                        operation_authority: ProvingAuthority,
                        policy: ExactPolicy(Some(invoke_policy)),
                        cryptography: StartCryptography {
                            checkpoint: RefCell::new(Some(checkpoint_preimage)),
                        },
                        invoke_inputs: StartInputs { semantic: &start },
                        publication_inputs: Box::new(
                            crate::resident::process_host::service_assembly::UnavailableResidentServicePublicationInputs,
                        ),
                        delegate_inputs: crate::resident::process_host::service_assembly::UnavailableResidentServiceDelegateInputs,
                        cancel_inputs: crate::resident::process_host::service_assembly::cancellation_test_support::UnavailableResidentServiceCancellationInputs,
                        observe_inputs: UnavailableObserveInputs,
                    };
                    let mut service =
                        crate::resident::ResidentFrameService::recover_invoke(process, owners)
                            .expect("recover public Invoke service");
                    let frame = ResidentOperationFrameRequest::StartInvoke {
                        signed_common_request: pale_core::canon::to_canonical_string(request)
                            .expect("canonical StartInvoke request"),
                    };
                    let mut denied_transport = OperationFramePort::for_peer(
                        &frame,
                        LocalIpcPeerCredentials::new(501, 20, Some(8_082)),
                    );
                    let deadline = std::time::Instant::now()
                        .checked_add(std::time::Duration::from_secs(2))
                        .expect("test deadline");
                    assert!(service
                        .serve_frame(&mut denied_transport, deadline, deadline)
                        .is_err());
                    assert!(denied_transport.sent.is_empty());
                    assert_eq!(
                        evidence_reads.load(Ordering::SeqCst),
                        0,
                        "peer admission must fail before current authority is read"
                    );
                    let mut transport = OperationFramePort::new(&frame);
                    assert_eq!(
                        service
                            .serve_frame(&mut transport, deadline, deadline)
                            .expect("StartInvoke reaches one response"),
                        crate::resident::ResidentFrameOutcome::Operation,
                    );
                    assert_eq!(evidence_reads.load(Ordering::SeqCst), 1);
                    let response: ResidentOperationFrameResponse =
                        serde_json::from_slice(&transport.sent).expect("response decodes");
                    assert!(
                        matches!(&response, ResidentOperationFrameResponse::Started { .. }),
                        "unexpected public StartInvoke response: {response:?}"
                    );
                    assert_or_update_application_invoke_fixture(&frame, &response);
                },
            );
        },
    );
}
