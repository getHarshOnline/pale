//! Complete current-request admission through the resident program boundary.

use super::*;
use crate::resident::consultation::tests::{
    prepare_collection_accumulator, with_issued_delegate_child_named, with_test_child_start_input,
};
use crate::resident::deployment_finalization::tests::recovery::{with_evidence, StoredChange};
use crate::resident::journal::tests::event_custody::UnavailableEventCustody;
use crate::resident::operation_dispatch::selection::tests::provider_dispatch::with_delegate_dispatch_input_named;
use crate::resident::operation_observation_custody::tests::a2a_exchange;
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_resident_program_cancellation_fixture, with_resident_program_observe_fixture,
    with_resident_program_observe_fixture_at_authorities, ProcessHostRestartFixture,
    ResidentProgramCancellationFixture, ResidentProgramObserveFixture,
};
use crate::resident::operation_observation_custody::tests::local_start::{
    absent_start_store, initial_anchor, initial_response, successor_anchor, with_issued_context,
};
use crate::resident::principal_trust_frontier::tests::recovered_cold_start_trust_frontier;
use crate::resident::process_host::a2a_delegate::{
    ResidentProcessA2aDelegateLaunch, ResidentProcessA2aDelegateLaunchPlan,
};
use crate::resident::process_host::a2a_delegate_recovery_owner::ResidentA2aDelegateRecoveryPort;
use crate::resident::process_host::current_request::*;
use crate::resident::process_host::operation_request_host::ResidentOperationRequestKind;
use crate::resident::process_host::program::*;
use crate::resident::process_host::resident_frame_ingress::ResidentProgramFrameOutcome;
use crate::resident::process_host::restart::{
    ResidentProcessRecoveredLocalCancellation, ResidentProcessRecoveredOperationResolution,
    ResidentProcessRecoveredPreparedAttempt, ResidentProcessRecoveredProviderResolution,
};
use crate::resident::process_host::tests::consult_child_recovery_sweep_hostile::with_cold_terminal_consult_child_handoff;
use crate::resident::process_host::tests::invoke_request::{
    with_unreachable_restart_inputs, FreshEntropy,
};
use crate::resident::process_host::tests::start::{checkpoint_preimage, CountingAuthority};
use crate::resident::GenerationLifecycleRecoveryResolver;
use crate::resident::{
    A2aRouteAssociation, OperationCheckpointAncestryLimits, OperationObservationRecoveryLimits,
    OperationObservationSelection, ProviderOperationDispatchInput,
};
use crate::resident_fixture::{synthetic_set_policy, FixedInputSource, RecoveryResolver};
use crate::session_admission_fixture::{schema_digest, unsigned_wire, ProvingAuthority};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use pale_adapter_uds::{UnixSocketCarrier, UnixSocketListener};
use pale_core::records::*;
use pale_ports::{
    AccumulatorPreparationLimits, ControlRecordCoordinate, OperationCustodyCommitOutcome,
    OperationCustodyIndexPredecessor, ResidentObserveFrameFailure, ResidentObserveFrameResponse,
    ResidentOperationFrameFailure, ResidentOperationFrameResponse, WallClockSourceError,
    WallClockSourcePort,
};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use pale_ports::{LocalIpcFrameLimits, LocalIpcFramePort};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, VecDeque};
use std::io::Write;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const MAXIMUM_APPLICATION_OBSERVE_FIXTURE_BYTES: usize = 1 << 20;
const MAXIMUM_APPLICATION_DELEGATE_FIXTURE_BYTES: usize = 1 << 20;
const MAXIMUM_APPLICATION_CANCEL_FIXTURE_BYTES: usize = 1 << 20;
const MAXIMUM_APPLICATION_INVOKE_ADVANCE_FIXTURE_BYTES: usize = 1 << 20;
const MAXIMUM_APPLICATION_DELEGATE_ADVANCE_FIXTURE_BYTES: usize = 1 << 20;

fn application_observe_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/resident/application-observe-sdk.v1.json")
}

fn application_delegate_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/resident/application-delegate-sdk.v1.json")
}

fn application_cancel_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/resident/application-cancel-sdk.v1.json")
}

fn application_invoke_advance_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/resident/application-invoke-advance-sdk.v1.json")
}

fn application_delegate_advance_fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/resident/application-delegate-advance-sdk.v1.json")
}

fn normalize_application_observe_machine_identity(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text)
            if text.len() == 64
                && text
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) =>
        {
            *text = "<sha256>".to_owned();
        }
        serde_json::Value::Array(values) => {
            for value in values {
                normalize_application_observe_machine_identity(value);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                normalize_application_observe_machine_identity(value);
            }
        }
        _ => {}
    }
}

fn normalize_application_operation_machine_identity(value: &mut serde_json::Value) {
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
            normalize_application_operation_machine_identity(&mut nested);
            *text = serde_json::to_string(&nested).expect("normalized canonical record serializes");
        }
        serde_json::Value::Array(values) => {
            for value in values {
                normalize_application_operation_machine_identity(value);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                normalize_application_operation_machine_identity(value);
            }
        }
        _ => {}
    }
}

fn assert_or_update_application_delegate_fixture(
    request_frame: &ResidentOperationFrameRequest,
    response_frame: &ResidentOperationFrameResponse,
) {
    assert_or_update_application_operation_fixture(
        request_frame,
        response_frame,
        application_delegate_fixture_path(),
        MAXIMUM_APPLICATION_DELEGATE_FIXTURE_BYTES,
        "Delegate",
    );
}

fn assert_or_update_application_cancel_fixture(
    request_frame: &ResidentOperationFrameRequest,
    response_frame: &ResidentOperationFrameResponse,
) {
    assert_or_update_application_operation_fixture(
        request_frame,
        response_frame,
        application_cancel_fixture_path(),
        MAXIMUM_APPLICATION_CANCEL_FIXTURE_BYTES,
        "Cancel",
    );
}

fn assert_or_update_application_invoke_advance_fixture(
    request_frame: &ResidentOperationFrameRequest,
    response_frame: &ResidentOperationFrameResponse,
) {
    assert_or_update_application_operation_fixture(
        request_frame,
        response_frame,
        application_invoke_advance_fixture_path(),
        MAXIMUM_APPLICATION_INVOKE_ADVANCE_FIXTURE_BYTES,
        "AdvanceInvoke",
    );
}

fn assert_or_update_application_delegate_advance_fixture(
    request_frame: &ResidentOperationFrameRequest,
    response_frame: &ResidentOperationFrameResponse,
) {
    assert_or_update_application_operation_fixture(
        request_frame,
        response_frame,
        application_delegate_advance_fixture_path(),
        MAXIMUM_APPLICATION_DELEGATE_ADVANCE_FIXTURE_BYTES,
        "AdvanceDelegate",
    );
}

fn assert_or_update_application_operation_fixture(
    request_frame: &ResidentOperationFrameRequest,
    response_frame: &ResidentOperationFrameResponse,
    path: std::path::PathBuf,
    maximum_bytes: usize,
    operation: &str,
) {
    let actual = serde_json::json!({
        "fixture_version": 1,
        "request_frame": request_frame,
        "response_frame": response_frame,
    });
    let mut actual_bytes = serde_json::to_vec_pretty(&actual).expect("fixture serializes");
    actual_bytes.push(b'\n');
    assert!(
        actual_bytes.len() <= maximum_bytes,
        "application {operation} fixture exceeds its byte ceiling"
    );

    if std::env::var_os("PALE_UPDATE_FIXTURES").as_deref() == Some("1".as_ref()) {
        let parent = path.parent().expect("fixture has an owner directory");
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).expect("create bounded fixture update");
        temporary
            .write_all(&actual_bytes)
            .expect("write bounded fixture update");
        temporary
            .flush()
            .expect("flush bounded fixture update to the temporary file");
        temporary.persist(&path).unwrap_or_else(|error| {
            panic!("atomically replace application {operation} fixture: {error}")
        });
        return;
    }

    let expected_bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("application {operation} fixture is present: {error}"));
    assert!(
        expected_bytes.len() <= maximum_bytes,
        "stored application {operation} fixture exceeds its byte ceiling"
    );
    let expected: serde_json::Value = serde_json::from_slice(&expected_bytes)
        .unwrap_or_else(|error| panic!("application {operation} fixture parses: {error}"));
    let mut expected_pretty =
        serde_json::to_vec_pretty(&expected).expect("stored fixture serializes");
    expected_pretty.push(b'\n');
    assert_eq!(
        expected_bytes, expected_pretty,
        "stored application {operation} fixture uses the canonical pretty form"
    );
    let mut comparable_actual = actual;
    let mut comparable_expected = expected;
    normalize_application_operation_machine_identity(&mut comparable_actual);
    normalize_application_operation_machine_identity(&mut comparable_expected);
    assert_eq!(
        comparable_actual, comparable_expected,
        "application {operation} output matches the public fixture"
    );
}

fn application_observe_fixture(
    request: &CommonRequestEnvelopeV1,
    response_bytes: &[u8],
) -> serde_json::Value {
    let response: ResidentObserveFrameResponse =
        serde_json::from_slice(response_bytes).expect("application Observe response decodes");
    assert_eq!(
        response_bytes,
        pale_core::canon::to_canonical_string(&response)
            .expect("application Observe response is canonical")
            .as_bytes(),
        "the public resident service must emit canonical frame bytes"
    );
    let (
        operation_handle_record_ref,
        operation_handle_canonical,
        operation_observation_record_ref,
        operation_observation_canonical,
    ) = match response {
        ResidentObserveFrameResponse::Observed {
            operation_handle_record_ref,
            operation_handle_canonical,
            operation_observation_record_ref,
            operation_observation_canonical,
        } => (
            operation_handle_record_ref,
            operation_handle_canonical,
            operation_observation_record_ref,
            operation_observation_canonical,
        ),
        ResidentObserveFrameResponse::Rejected { .. } => {
            panic!("application Observe did not return a retained observation")
        }
    };
    let operation_handle_record: serde_json::Value =
        serde_json::from_str(&operation_handle_canonical).expect("handle record parses");
    let operation_observation_record: serde_json::Value =
        serde_json::from_str(&operation_observation_canonical).expect("observation record parses");
    let operation_handle_sha256 = operation_handle_record
        .get("canonical_operation_handle_sha256")
        .and_then(serde_json::Value::as_str)
        .expect("handle record carries its canonical digest")
        .to_owned();
    serde_json::json!({
        "fixture_version": 1,
        "signed_common_request": request,
        "operation_handle_record_ref": operation_handle_record_ref,
        "operation_handle_sha256": operation_handle_sha256,
        "operation_handle_record": operation_handle_record,
        "operation_observation_record_ref": operation_observation_record_ref,
        "operation_observation_record": operation_observation_record,
    })
}

fn assert_or_update_application_observe_fixture(actual: &serde_json::Value) {
    let path = application_observe_fixture_path();
    let mut actual_bytes = serde_json::to_vec_pretty(actual).expect("fixture serializes");
    actual_bytes.push(b'\n');
    assert!(
        actual_bytes.len() <= MAXIMUM_APPLICATION_OBSERVE_FIXTURE_BYTES,
        "application Observe fixture exceeds its byte ceiling"
    );
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

    let expected_bytes = std::fs::read(&path).expect("application Observe fixture is present");
    assert!(
        expected_bytes.len() <= MAXIMUM_APPLICATION_OBSERVE_FIXTURE_BYTES,
        "stored application Observe fixture exceeds its byte ceiling"
    );
    let expected: serde_json::Value =
        serde_json::from_slice(&expected_bytes).expect("application Observe fixture parses");
    let mut expected_pretty =
        serde_json::to_vec_pretty(&expected).expect("stored fixture serializes");
    expected_pretty.push(b'\n');
    assert_eq!(
        expected_bytes, expected_pretty,
        "stored application Observe fixture uses the canonical pretty form"
    );
    let mut comparable_actual = actual.clone();
    let mut comparable_expected = expected;
    // Source-backed compilation carries machine-local path identities. The
    // recovered application proves every exact digest join before this stable
    // cross-process fixture comparison.
    normalize_application_observe_machine_identity(&mut comparable_actual);
    normalize_application_observe_machine_identity(&mut comparable_expected);
    assert_eq!(
        comparable_actual, comparable_expected,
        "recovered application Observe output matches the public fixture"
    );
}

fn active_predecessors(
    generation: &CompiledGenerationV1,
) -> (PublishedGenerationHeadV1, AdvertisedGenerationHeadV1) {
    let published_preimage = PublishedGenerationHeadPreimageV1 {
        schema: SchemaId::parse(PUBLISHED_GENERATION_HEAD_V1_SCHEMA).unwrap(),
        schema_sha256: schema_digest('2'),
        schema_version: PUBLISHED_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        journal_sequence: JournalSequenceV1::new(1).unwrap(),
        predecessor_published_generation_head_digest: None,
    };
    let published_digest =
        PublishedGenerationHeadDigest::hash_canonical(&published_preimage).unwrap();
    let published = PublishedGenerationHeadV1::admit_contextual_wire(
        unsigned_wire(
            &published_preimage,
            "published_generation_head_digest",
            &published_digest,
        ),
        &PublishedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &published_preimage.schema_sha256,
            compiled_generation: generation,
            predecessor: None,
        },
    )
    .unwrap();
    let advertised_preimage = AdvertisedGenerationHeadPreimageV1 {
        schema: SchemaId::parse(ADVERTISED_GENERATION_HEAD_V1_SCHEMA).unwrap(),
        schema_sha256: schema_digest('3'),
        schema_version: ADVERTISED_GENERATION_HEAD_V1_SCHEMA_VERSION,
        compiled_generation_digest: generation.compiled_generation_digest().clone(),
        published_generation_head_digest: published.published_generation_head_digest().clone(),
        journal_sequence: JournalSequenceV1::new(2).unwrap(),
        predecessor_advertised_generation_head_digest: None,
    };
    let advertised_digest =
        AdvertisedGenerationHeadDigest::hash_canonical(&advertised_preimage).unwrap();
    let advertised = AdvertisedGenerationHeadV1::admit_contextual_wire(
        unsigned_wire(
            &advertised_preimage,
            "advertised_generation_head_digest",
            &advertised_digest,
        ),
        &AdvertisedGenerationHeadAdmissionContextV1 {
            expected_schema_sha256: &advertised_preimage.schema_sha256,
            published_head: &published,
            predecessor: None,
        },
    )
    .unwrap();
    (published, advertised)
}

fn evidence_from(
    current: &crate::resident::AmbientSessionAdmissionContext<'_>,
) -> ResidentCurrentRequestEvidence {
    let grant = current.grant.as_ref().unwrap();
    let capability = current.capability_revocations.as_ref().unwrap();
    ResidentCurrentRequestEvidence {
        grant: ResidentCurrentGrantEvidence {
            grant_ref: grant.grant_ref.clone(),
            grant: grant.grant.clone(),
            issued_trust_checkpoint_ref: grant.issued_trust_checkpoint_ref.clone(),
            issued_trust_checkpoint: grant.issued_trust_checkpoint.clone(),
        },
        capability_revocations: ResidentCurrentCapabilityRevocationEvidence {
            checkpoint_ref: capability.checkpoint_ref.clone(),
            checkpoint: capability.checkpoint.clone(),
            grant_head: capability
                .grant_head
                .map(|head| ResidentCurrentGrantRevocationEvidence {
                    revocation_ref: head.revocation_ref.clone(),
                    revocation: head.revocation.clone(),
                }),
        },
        deployment_revocations: ResidentCurrentDeploymentRevocationEvidence {
            epoch: current.deployment_revocations.epoch,
            binding_head: current.deployment_revocations.binding_head.map(|head| {
                ResidentCurrentDeploymentRevocationHeadEvidence {
                    revocation_ref: head.revocation_ref.clone(),
                    revocation: head.revocation.clone(),
                }
            }),
        },
        session_revocations: ResidentCurrentSessionRevocationEvidence {
            epoch: current.session_revocations.epoch,
            revocation_ref: current.session_revocations.revocation_ref.cloned(),
            revocation_sha256: current.session_revocations.revocation_sha256.cloned(),
        },
    }
}

fn service_evidence_from(
    current: &crate::resident::AmbientSessionAdmissionContext<'_>,
) -> crate::resident::ResidentServiceCurrentRequestEvidence {
    let grant = current.grant.as_ref().expect("fixture grant");
    let capability = current
        .capability_revocations
        .as_ref()
        .expect("fixture capability frontier");
    crate::resident::ResidentServiceCurrentRequestEvidence {
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

fn service_policy_from(
    generation: &crate::resident::AdmittedCompiledGeneration,
    repository_id: LogicalRepositoryIdV1,
    request: &CommonRequestEnvelopeV1,
) -> crate::resident::ResidentServicePolicyProjection {
    let value = request.to_preimage();
    assert!(value.execution_owner_fence.is_none());
    let maximum_request_lifetime_seconds = value
        .expires_at
        .get()
        .checked_sub(value.issued_at.get())
        .filter(|seconds| *seconds > 0)
        .expect("the service fixture request has a positive lifetime");
    crate::resident::ResidentServicePolicyProjection::admit(
        generation,
        crate::resident::ResidentServicePolicyInput {
            repository_id,
            body_schema_id: value.body.schema_id,
            body_schema_sha256: value.body.schema_sha256,
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
            maximum_request_lifetime_seconds,
            fence_token: value.fence_token,
        },
    )
    .expect("service policy is admitted from the retained generation")
}

struct ServiceEvidencePort(
    VecDeque<
        Result<crate::resident::ResidentServiceCurrentRequestEvidence, InvalidResidentRecordV1>,
    >,
);

impl crate::resident::ResidentServiceCurrentRequestEvidencePort for ServiceEvidencePort {
    fn read_current_request_evidence(
        &mut self,
        _: crate::resident::ResidentServiceCurrentRequestSelector<'_>,
    ) -> Result<crate::resident::ResidentServiceCurrentRequestEvidence, InvalidResidentRecordV1>
    {
        self.0
            .pop_front()
            .unwrap_or(Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "service request evidence",
                invariant: "the fixture has no remaining current evidence",
            }))
    }
}

struct ServiceWriterAdmission {
    writer: PrincipalResidentWriterFenceV1,
    checkpoint: ControlStateCheckpointV1,
}

impl crate::resident::ResidentServiceCurrentWriterAdmissionPort for ServiceWriterAdmission {
    fn admit_current_writer(
        &mut self,
        _: &pale_ports::PrincipalResidentWriterSnapshot,
    ) -> Result<crate::resident::ResidentServiceCurrentWriterEvidence, InvalidResidentRecordV1>
    {
        Ok(crate::resident::ResidentServiceCurrentWriterEvidence {
            writer: self.writer.clone(),
            checkpoint: self.checkpoint.clone(),
        })
    }
}

struct ServiceSignature;

impl crate::resident::ResidentServiceCallerSignaturePort for ServiceSignature {
    fn verify_caller_signature(
        &mut self,
        _: &CommonRequestEnvelopePreimageV1,
        _: &CommonRequestEnvelopeDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }
}

struct ServiceEntropy {
    calls: Arc<AtomicUsize>,
}

impl pale_ports::CryptographicRandomSourcePort for ServiceEntropy {
    fn fill_bytes(
        &self,
        destination: &mut [u8],
    ) -> Result<(), pale_ports::CryptographicRandomSourceError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        destination.fill(u8::try_from(call).expect("bounded Delegate identity allocation"));
        Ok(())
    }
}

struct ServiceStartCryptography {
    checkpoint: RefCell<Option<ControlStateCheckpointPreimageV1>>,
    handle_signs: Arc<AtomicUsize>,
    observation_signs: Arc<AtomicUsize>,
    checkpoint_signs: Arc<AtomicUsize>,
}

impl crate::resident::ResidentServiceInvokeCryptographyPort for ServiceStartCryptography {
    fn admit_compiled_authority(
        &self,
        _: &CommonRequestEnvelopeV1,
        _: &crate::resident::AdmittedAmbientSession,
        _: &crate::resident::AmbientSessionAdmissionContext<'_>,
        _: &OperationHandlePreimageV1,
        _: &OperationLeaseV1,
        _: &crate::semantic::ResolvedOperation,
        _: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
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
        self.handle_signs.fetch_add(1, Ordering::SeqCst);
        Ok(crate::local_operation_start_fixture::signature())
    }

    fn sign_observation(
        &self,
        _: &[u8],
        _: &OperationObservationPreimageV1,
        _: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        self.observation_signs.fetch_add(1, Ordering::SeqCst);
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
        let mut checkpoint = checkpoint_preimage(prepared, checkpoint)?;
        checkpoint.issued_at = issued_at;
        Ok(checkpoint)
    }

    fn sign_checkpoint(
        &self,
        _: &[u8],
        _: &ControlStateCheckpointPreimageV1,
        _: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        self.checkpoint_signs.fetch_add(1, Ordering::SeqCst);
        Ok(crate::local_operation_start_fixture::signature())
    }
}

struct ServiceDelegatePolicy {
    policy: crate::resident::ResidentServiceOperationStartPolicy,
    expected_parent_ref: ResidentRecordRefV1,
    expected_parent_sha256: OperationHandleDigest,
    calls: Arc<AtomicUsize>,
}

impl crate::resident::ResidentServiceInvokePolicyPort for ServiceDelegatePolicy {
    fn evaluate_invoke(
        &mut self,
        _: crate::resident::ResidentServiceInvokePolicyContext<'_>,
    ) -> Result<crate::resident::ResidentServiceOperationStartPolicy, InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "service Invoke policy",
            invariant: "the Delegate fixture does not admit root Invoke",
        })
    }
}

impl crate::resident::ResidentServiceDelegatePolicyPort for ServiceDelegatePolicy {
    fn evaluate_delegate(
        &mut self,
        context: crate::resident::ResidentServiceDelegatePolicyContext<'_>,
    ) -> Result<crate::resident::ResidentServiceOperationStartPolicy, InvalidResidentRecordV1> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let (parent_ref, parent) = context.parent.operation();
        if parent_ref != &self.expected_parent_ref
            || parent.canonical_operation_handle_sha256() != &self.expected_parent_sha256
        {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "service Delegate policy",
                invariant: "the protected parent differs from the selected parent",
            });
        }
        Ok(self.policy.clone())
    }
}

struct ServiceObserveRecovery<'a> {
    recovery: Option<OperationControlRecoveryInput<'a>>,
    admission: Option<OperationControlRecoveryAdmission<'a>>,
    authorize_disclosure: Option<&'a mut OperationAuthorization<'a>>,
    response_deadline: std::time::Instant,
}

struct ServiceObserveUnavailable;

impl<'recovery> crate::resident::ResidentServiceObserveInputPort<'recovery>
    for ServiceObserveUnavailable
{
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

impl<'a> crate::resident::ResidentServiceObserveInputPort<'a> for ServiceObserveRecovery<'a> {
    fn with_observe_inputs<Output, UseInputs>(
        &mut self,
        locator: &pale_ports::ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentObserveFrameFailure>
    where
        UseInputs: for<'request> FnOnce(
            crate::resident::ResidentServiceObserveInputs<'request, 'a>,
        ) -> Output,
    {
        let recovery = self
            .recovery
            .take()
            .ok_or(ResidentObserveFrameFailure::ReplayConflict)?;
        if recovery.operation.expected_handle != locator {
            return Err(ResidentObserveFrameFailure::InvalidRequest);
        }
        let recovery_admission = self
            .admission
            .take()
            .ok_or(ResidentObserveFrameFailure::ReplayConflict)?;
        let mut verify = |_: &CommonRequestEnvelopePreimageV1,
                          _: &CommonRequestEnvelopeDigest,
                          _: &CanonicalSignatureV1| { Ok(()) };
        let mut replay = |_: &ResidentRequestIdV1, _: &CommonRequestEnvelopeDigest, _: &[u8]| {
            Ok(crate::resident::ResidentServiceObserveReplayDisposition::FirstAdmission)
        };
        let authorize_disclosure = self
            .authorize_disclosure
            .take()
            .ok_or(ResidentObserveFrameFailure::ReplayConflict)?;
        Ok(use_inputs(crate::resident::ResidentServiceObserveInputs {
            recovery,
            verify_caller_signature: &mut verify,
            admit_replay: &mut replay,
            recovery_admission,
            authorize_disclosure,
            response_deadline: self.response_deadline,
        }))
    }
}

pub(super) fn policy_from(
    request: &CommonRequestEnvelopeV1,
    generation: CompiledGenerationDigest,
) -> ResidentRetainedCommonRequestPolicy {
    let value = request.to_preimage();
    assert!(value.execution_owner_fence.is_none());
    ResidentRetainedCommonRequestPolicy {
        expected_schema_sha256: value.envelope_schema_sha256,
        expected_body_schema_id: value.body.schema_id,
        expected_body_schema_sha256: value.body.schema_sha256,
        caller: value.caller,
        signing_authority: value.signing_authority_binding,
        route: value.route,
        operation_target: None,
        execution_owner_fence: None,
        federation_claim: value.federation_claim,
        workspace_id: value.workspace_id,
        capability_id: value.capability_id,
        operation: value.operation,
        scope: value.scope,
        risk_class: value.risk_class,
        resource_constraints: value.resource_constraints,
        expected_precondition: value.expected_precondition,
        maximum_request_lifetime_seconds: 12,
        generation: Some(generation),
        fence_token: value.fence_token,
    }
}

#[derive(Clone)]
struct EvidencePort {
    values: Arc<Mutex<VecDeque<Result<ResidentCurrentRequestEvidence, ()>>>>,
    reads: Arc<Mutex<usize>>,
}

impl ResidentCurrentRequestEvidencePort<()> for EvidencePort {
    fn read_current_request_evidence(
        &mut self,
        _: ResidentCurrentRequestSelector<'_, ()>,
    ) -> Result<ResidentCurrentRequestEvidence, ResidentCurrentRequestEvidenceError> {
        *self.reads.lock().unwrap() += 1;
        self.values
            .lock()
            .unwrap()
            .pop_front()
            .expect("one current-evidence result per turn")
            .map_err(|_| ResidentCurrentRequestEvidenceError::Unavailable)
    }
}

#[derive(Clone)]
struct Clock {
    values: Arc<Mutex<VecDeque<UnixTimeV1>>>,
    reads: Arc<Mutex<usize>>,
}

struct ExactSessionStore {
    store: AuthorityStoreCoordinate,
    body: Mutex<Option<pale_ports::SessionLeaseBody>>,
}

impl ExactSessionStore {
    fn new(store: AuthorityStoreCoordinate) -> Self {
        Self {
            store,
            body: Mutex::new(None),
        }
    }
}

impl pale_ports::SessionLeaseCustodyPort for ExactSessionStore {
    fn compare_and_retain(
        &self,
        body: &pale_ports::SessionLeaseBody,
    ) -> Result<pale_ports::SessionLeaseBodyRetention, pale_ports::ControlStoreError> {
        if body.store() != &self.store {
            return Err(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            ));
        }
        let mut retained = self.body.lock().expect("session body");
        match retained.as_ref() {
            None => {
                *retained = Some(body.clone());
                Ok(pale_ports::SessionLeaseBodyRetention::Retained)
            }
            Some(existing) if existing == body => {
                Ok(pale_ports::SessionLeaseBodyRetention::ExactReplay)
            }
            Some(_) => Err(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::RecoveryRequired,
            )),
        }
    }

    fn read(
        &self,
        request: &pale_ports::SessionLeaseBodyRead,
    ) -> Result<Option<pale_ports::SessionLeaseBody>, pale_ports::ControlStoreError> {
        if request.store() != &self.store {
            return Err(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            ));
        }
        let retained = self.body.lock().expect("session body");
        let Some(body) = retained.as_ref() else {
            return Ok(None);
        };
        if body.digest() != request.digest() {
            return Ok(None);
        }
        let retained_bytes = body.canonical_wire_jcs().as_slice().len()
            + body
                .canonical_private_admission_basis_jcs()
                .map_or(0, |basis| basis.as_slice().len());
        if retained_bytes > request.maximum_body_bytes() {
            return Err(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        Ok(Some(body.clone()))
    }
}

impl WallClockSourcePort for Clock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        *self.reads.lock().unwrap() += 1;
        Ok(self
            .values
            .lock()
            .unwrap()
            .pop_front()
            .expect("one physical time per turn"))
    }
}

struct Signature(bool);

impl ResidentProgramCallerSignaturePort for Signature {
    fn admit_native_caller_key(
        &mut self,
        _: &RequestCallerV1,
        key: &[u8; 32],
    ) -> Result<(), InvalidResidentRecordV1> {
        if self.0 && key == &[7; 32] {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "native caller fixture",
                invariant: "key mismatch",
            })
        }
    }

    fn verify_caller_signature(
        &mut self,
        _: &CommonRequestEnvelopePreimageV1,
        _: &CommonRequestEnvelopeDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.0
            .then_some(())
            .ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "common request",
                invariant: "test signature rejected",
            })
    }
}

struct AcceptPeer;

impl ResidentProgramPeerAdmissionPort for AcceptPeer {
    type Witness = ();

    fn admit_peer(
        &mut self,
        _: pale_ports::LocalIpcPeerCredentials,
        _: &ResidentOperationFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        Ok(())
    }
}

impl ResidentProgramObservePeerAdmissionPort for AcceptPeer {
    fn admit_observe_peer(
        &mut self,
        _: pale_ports::LocalIpcPeerCredentials,
        _: &ResidentObserveFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        Ok(())
    }
}

struct OperationOnlyPeer;

impl ResidentProgramPeerAdmissionPort for OperationOnlyPeer {
    type Witness = ();

    fn admit_peer(
        &mut self,
        _: pale_ports::LocalIpcPeerCredentials,
        _: &ResidentOperationFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        Ok(())
    }
}

impl ResidentProgramObservePeerAdmissionPort for OperationOnlyPeer {
    fn admit_observe_peer(
        &mut self,
        _: pale_ports::LocalIpcPeerCredentials,
        _: &ResidentObserveFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "resident Observe peer",
            invariant: "operation-only peer must not admit Observe",
        })
    }
}

struct ObserveOnlyPeer;

impl ResidentProgramPeerAdmissionPort for ObserveOnlyPeer {
    type Witness = ();

    fn admit_peer(
        &mut self,
        _: pale_ports::LocalIpcPeerCredentials,
        _: &ResidentOperationFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "resident operation peer",
            invariant: "Observe-only peer must not admit operation frames",
        })
    }
}

impl ResidentProgramObservePeerAdmissionPort for ObserveOnlyPeer {
    fn admit_observe_peer(
        &mut self,
        _: pale_ports::LocalIpcPeerCredentials,
        _: &ResidentObserveFrameRequest,
    ) -> Result<Self::Witness, InvalidResidentRecordV1> {
        Ok(())
    }
}

#[test]
fn operation_and_observe_peer_admission_cannot_masquerade_as_each_other() {
    let peer = pale_ports::LocalIpcPeerCredentials::new(501, 20, Some(8_081));
    let mut operation_only = OperationOnlyPeer;
    let mut observe_only = ObserveOnlyPeer;
    let operation = ResidentOperationFrameRequest::StartInvoke {
        signed_common_request: "{}".to_owned(),
    };
    let observe = ResidentObserveFrameRequest::new(
        "operation/handle".to_owned(),
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
        "{}".to_owned(),
    );

    assert!(operation_only.admit_peer(peer, &operation).is_ok());
    assert!(operation_only.admit_observe_peer(peer, &observe).is_err());
    assert!(observe_only.admit_peer(peer, &operation).is_err());
    assert!(observe_only.admit_observe_peer(peer, &observe).is_ok());
}

#[test]
fn terminal_consult_child_survives_an_unavailable_physical_input_lender() {
    with_cold_terminal_consult_child_handoff("program-terminal-input-unavailable", |handoff| {
        let mut inputs = RejectTerminalConsultInputs::default();
        let result = inputs.with_consult_child_terminal_inputs(|_| {
            panic!("the unavailable lender must not invoke the commit callback")
        });
        assert!(matches!(
            result,
            Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
        ));
        let error = retain_consult_child_terminal_input_refusal::<(), _>(&mut inputs, handoff);
        assert!(matches!(
            error,
            ResidentProgramTurnError::RecoveryUnavailable
        ));
        assert_eq!(inputs.retained_terminal_handoffs, 1);
    });
}

struct RecoveryThenWriteAnchor {
    recovered: Option<pale_ports::AntiRollbackAnchor>,
    write: crate::resident::operation_observation_custody::tests::RecordingAnchor,
}

impl pale_ports::AntiRollbackAnchorPort for RecoveryThenWriteAnchor {
    fn read(
        &mut self,
        key: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, pale_ports::AntiRollbackAnchorError> {
        let _ = key;
        Ok(self.recovered.clone())
    }

    fn compare_and_advance(
        &mut self,
        transition: &pale_ports::PreparedAntiRollbackAnchorTransition,
    ) -> Result<pale_ports::AntiRollbackAnchorAdvanceDisposition, pale_ports::AntiRollbackAnchorError>
    {
        self.write.compare_and_advance(transition)
    }

    fn observe(
        &mut self,
        transition: &pale_ports::PreparedAntiRollbackAnchorTransition,
    ) -> Result<pale_ports::AntiRollbackAnchorObservation, pale_ports::AntiRollbackAnchorError>
    {
        self.write.observe(transition)
    }
}

struct RootRecovery<'a> {
    semantic: &'a crate::resident::LocalOperationStartInput<'a>,
}

struct DelegateSchedulerRecovery {
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    calls: Rc<Cell<usize>>,
}

impl ResidentProgramRootRecoveryInputPort for DelegateSchedulerRecovery {
    fn with_root_recovery<Output, UseInputs>(
        &mut self,
        _: &ResidentDeploymentAuthoritySelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<
            &crate::resident::process_host::operation_locator::ResidentProgramOperationLocator,
        >,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'restart, 'admission, 'checkpoint> FnOnce(
            ResidentProcessOperationRestartInput<'restart>,
            OperationControlRecoveryAdmission<'admission>,
            &'checkpoint ControlCheckpointSigningAuthorityAdmissionContextV1<'checkpoint>,
            &mut Self,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn retain_deferred_recovery(
        &mut self,
        _: Box<crate::resident::process_host::restart::ResidentProcessOperationRestartDisposition>,
    ) {
        panic!("the scheduler probe does not run cold recovery")
    }

    fn has_runnable_retained_delegate(&self) -> bool {
        self.calls.get() == 0
    }

    fn retained_delegate_selector(&self) -> Option<&ResidentDeploymentAuthoritySelector> {
        (self.calls.get() == 0).then_some(&self.selector)
    }

    fn continue_one_retained_delegate<WriterGuard, Authority>(
        &mut self,
        _: &mut ResidentProcessHost<'_, WriterGuard>,
        access: ResidentDeploymentOperationAccess<'_>,
        _: &mut Authority,
    ) -> ResidentProgramRetainedPhaseTurn
    where
        WriterGuard: PrincipalWriterGuardPort,
        Authority: crate::resident::AmbientSessionAuthorityPort,
    {
        assert_eq!(access.store, &self.store);
        self.calls.set(self.calls.get() + 1);
        ResidentProgramRetainedPhaseTurn::Advanced
    }
}

struct ParentRecovery<'a> {
    store: AuthorityStoreCoordinate,
    parent_request: &'a CommonRequestEnvelopeV1,
    parent_checkpoint: &'a ControlStateCheckpointV1,
    historical: &'a Historical,
}

struct ExistingRecovery<'a> {
    recovery: OperationControlRecoveryInput<'a>,
    admission: Option<OperationControlRecoveryAdmission<'a>>,
    retained_deferred: Arc<AtomicUsize>,
}

struct JoinedProgramA2aPort<'operation, 'call, 'context> {
    dispatch: Option<ProviderOperationDispatchInput<'operation, 'call>>,
    frame_request: CommonRequestEnvelopeV1,
    completed_context: &'context crate::resident::process_host::operation_resolution::CompletedResidentProcessOperationContext<'operation>,
    launch_current: crate::resident::AmbientSessionAdmissionContext<'context>,
    route_ref: ResidentRecordRefV1,
    route: A2aRouteAdmissionV1,
    request_id: A2aJsonRpcRequestIdV1,
    clock: super::a2a_delegate::A2aProcessClock,
    exchange: Rc<super::a2a_delegate::A2aProcessExchangePort>,
    authority: ProvingAuthority,
    awaiting: Rc<Cell<usize>>,
}

struct JoinedProgramA2aRecovery<'recovery, 'operation, 'call, 'context> {
    recovery: OperationControlRecoveryInput<'recovery>,
    admission: Option<OperationControlRecoveryAdmission<'recovery>>,
    a2a: JoinedProgramA2aPort<'operation, 'call, 'context>,
}

impl ResidentProgramRootRecoveryInputPort for JoinedProgramA2aRecovery<'_, '_, '_, '_> {
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
        let locator = locator.ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
        if locator.coordinate().record_ref() != self.recovery.operation.expected_handle.record_ref()
            || locator.coordinate().record_sha256().as_str()
                != self
                    .recovery
                    .operation
                    .expected_handle
                    .record_sha256()
                    .as_str()
        {
            return Err(ResidentOperationFrameFailure::InvalidRequest);
        }
        let LocalOperationHandleRecoveryInput {
            context,
            expected_handle,
            maximum_parent_depth,
        } = self.recovery.operation;
        let restart = crate::resident::process_host::tests::restart::restart_input(
            OperationControlRecoveryInput {
                operation: LocalOperationHandleRecoveryInput {
                    context,
                    expected_handle,
                    maximum_parent_depth,
                },
                observation: self.recovery.observation,
                observation_limits: self.recovery.observation_limits,
                checkpoint_limits: self.recovery.checkpoint_limits,
                membership_limits: self.recovery.membership_limits,
            },
            1 << 20,
        );
        let checkpoint_authority = context.checkpoint_authority;
        let admission = self
            .admission
            .take()
            .ok_or(ResidentOperationFrameFailure::ReplayConflict)?;
        Ok(use_inputs(restart, admission, checkpoint_authority, self))
    }

    fn retain_deferred_recovery(
        &mut self,
        _: Box<crate::resident::process_host::restart::ResidentProcessOperationRestartDisposition>,
    ) {
        panic!("the point-addressed A2A proof cannot defer another phase")
    }
}

impl ResidentA2aDelegateRecoveryPort<UnprovedHeldWriter> for JoinedProgramA2aPort<'_, '_, '_> {
    type Output =
        crate::resident::process_host::operation_request_host::ResidentOperationRequestOutcome;
    type Error = ();

    fn recover_delegate_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        recovered: Box<ResidentProcessRecoveredOperationResolution>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProgramDelegateResolutionInput<'_, '_>,
    ) -> Result<Self::Output, Self::Error> {
        let dispatch = self.dispatch.take().ok_or(())?;
        assert_eq!(input.request(), &self.frame_request);
        let admitted_request = input.request().to_preimage();
        let committed_request = dispatch.operation.context.request.to_preimage();
        assert_eq!(admitted_request.request_id, committed_request.request_id);
        assert_eq!(
            admitted_request.correlation_id,
            committed_request.correlation_id
        );
        assert_eq!(
            admitted_request.idempotency_key,
            committed_request.idempotency_key
        );
        assert_eq!(
            admitted_request.workspace_id,
            committed_request.workspace_id
        );
        assert_eq!(
            admitted_request.capability_id,
            committed_request.capability_id
        );
        assert_eq!(admitted_request.operation, committed_request.operation);
        assert_eq!(
            admitted_request.body.payload_sha256,
            committed_request.body.payload_sha256
        );
        assert_eq!(
            input.session().lease(),
            dispatch.operation.context.session.lease()
        );
        assert_eq!(
            input.current().deployment_binding_ref,
            dispatch.operation.context.current.deployment_binding_ref
        );
        assert_eq!(
            input
                .current()
                .generation
                .compiled_generation
                .compiled_generation_digest(),
            dispatch.operation.invocation.compiled_generation_digest()
        );
        assert_eq!(
            input
                .current()
                .generation
                .compiled_generation
                .instance_digest(),
            dispatch.operation.invocation.instance_digest()
        );
        assert_eq!(
            self.route.provider_binding().provider_binding_id,
            dispatch.operation.invocation.provider_binding().id
        );
        assert_eq!(
            self.route.provider_binding().provider_binding_identity,
            dispatch
                .operation
                .invocation
                .provider_binding()
                .binding_identity
        );
        let maximum_selection_bytes = input
            .maximum_selection_bytes()
            .min(dispatch.operation.context.maximum_selection_bytes);

        let signing = dispatch.signing;
        let selection_ref = dispatch.operation.context.selection_ref.clone();
        let observation_ref = dispatch.operation.context.observation_ref.clone();
        let checkpoint_schema = dispatch.checkpoint_schema;
        let checkpoint_authority = dispatch.checkpoint_authority;
        let resolve_store = dispatch.resolve_store;
        let maximum_record_bytes = input.maximum_record_bytes();
        let completed_context = match recovered.phase {
            crate::resident::process_host::restart::ResidentProcessOperationResolutionPhase::Resolving => {
                Some(self.completed_context)
            }
            crate::resident::process_host::restart::ResidentProcessOperationResolutionPhase::ContextBound => {
                None
            }
        };
        let plan = ResidentProcessA2aDelegateLaunchPlan {
            current: &self.launch_current,
            route: (&self.route_ref, &self.route),
            route_predecessor: OperationCustodyIndexPredecessor::Absent,
            route_association: A2aRouteAssociation::Insert,
            request_id: &self.request_id,
            message_id: "program-a2a-message",
            text: "Inspect the selected workspace.",
            request_evidence_ref: local_operation_start_fixture::record_ref("program-a2a/request"),
            preparation_ref: local_operation_start_fixture::record_ref("program-a2a/preparation"),
            exchange_limits: a2a_exchange::limits(),
            maximum_evidence_bytes: 8_192,
            maximum_record_bytes,
            maximum_dispatch_record_bytes: maximum_record_bytes,
            resolve_store,
            checkpoint: OperationCheckpointInput {
                reference: local_operation_start_fixture::record_ref("program-a2a/send-checkpoint"),
                id: ControlCheckpointIdV1::parse("program-a2a-send-checkpoint").unwrap(),
            },
            checkpoint_schema,
            checkpoint_authority,
            response_ref: local_operation_start_fixture::record_ref("program-a2a/response"),
            clock: &self.clock,
            accumulator_limits: AccumulatorPreparationLimits::new(
                8, 20_000, 20_000, 8_192, 200_000,
            )
            .unwrap(),
            membership_limits: pale_ports::AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                .unwrap(),
        };
        let launched = host
            .launch_recovered_pairwise_a2a_delegate(
                &recovered,
                access,
                crate::resident::process_host::operation_resolution::ResidentProcessOperationResolutionInput {
                    session: input.session(),
                    current: input.current(),
                    signing,
                    selection_ref,
                    observation_ref,
                    maximum_selection_bytes,
                    maximum_record_bytes,
                },
                completed_context,
                &mut self.authority,
                dispatch,
                || plan,
                self.exchange.as_ref(),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
                |_, _, _| Ok(()),
                |_, _, _| Ok(()),
                |_, _, _| Ok(local_operation_start_fixture::signature()),
            )
            .expect("the admitted Delegate reaches the retained A2A launch");
        let launched = match launched {
            Ok(launched) => launched,
            Err(
                crate::resident::process_host::a2a_delegate::ResidentProcessPairwiseA2aDelegateLaunchFailure::Selection {
                    ..
                },
            ) => panic!("the retained A2A launch rejected the exact selection"),
            Err(
                crate::resident::process_host::a2a_delegate::ResidentProcessPairwiseA2aDelegateLaunchFailure::Provider(_),
            ) => panic!("the retained A2A launch could not retain provider dispatch"),
            Err(
                crate::resident::process_host::a2a_delegate::ResidentProcessPairwiseA2aDelegateLaunchFailure::A2a(failure),
            ) => match *failure {
                crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateLaunchFailure::Selection { .. } => {
                    panic!("the retained A2A launch rejected its process selection")
                }
                crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateLaunchFailure::RouteSaturated(_) => {
                    panic!("the retained A2A launch exhausted process routes")
                }
                crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateLaunchFailure::RouteConflict(_) => {
                    panic!("the retained A2A launch reused an active route")
                }
                crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateLaunchFailure::Rejected(failure) => {
                    match *failure {
                        crate::resident::a2a_delegate_task_service::ResidentA2aDelegateLaunchFailure::Association { reason, .. } => {
                            panic!("the retained A2A launch failed association: {reason:?}")
                        }
                        crate::resident::a2a_delegate_task_service::ResidentA2aDelegateLaunchFailure::Reservation { reason, .. } => {
                            panic!("the retained A2A launch failed reservation: {reason:?}")
                        }
                        crate::resident::a2a_delegate_task_service::ResidentA2aDelegateLaunchFailure::Launch(reason) => {
                            panic!("the retained A2A launch failed before carrier start: {reason:?}")
                        }
                        _ => unreachable!("only proved-unstarted failures enter Rejected"),
                    }
                }
                crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateLaunchFailure::Rollback { .. } => {
                    panic!("the retained A2A launch could not roll back its unopened route")
                }
                crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateLaunchFailure::Custody { .. } => {
                    panic!("the retained A2A launch has uncertain request custody")
                }
                crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateLaunchFailure::Unopened { .. } => {
                    panic!("the retained A2A launch retained an unopened exchange")
                }
                crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateLaunchFailure::StartedAssociation { .. } => {
                    panic!("the retained A2A launch failed its started-exchange association")
                }
                crate::resident::process_host::a2a_delegate::ResidentProcessA2aDelegateLaunchFailure::OfferSettlement { .. } => {
                    panic!("the retained A2A launch could not settle the closed exchange")
                }
            },
        };
        let ResidentProcessA2aDelegateLaunch::Awaiting(_) = launched else {
            return Err(());
        };
        self.awaiting.set(self.awaiting.get() + 1);
        Ok(
            crate::resident::process_host::operation_request_host::ResidentOperationRequestOutcome::Pending(
                operation_frame_locator(recovered.start.control.operation().operation()),
            ),
        )
    }

    fn recover_pre_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<crate::resident::process_host::restart::ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("the joined launch starts from operation resolution")
    }

    fn recover_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<crate::resident::process_host::restart::ResidentProcessRecoveredA2aTask>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("the joined launch has not yet identified a remote Task")
    }

    fn recover_cancel_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<
            crate::resident::process_host::restart::ResidentProcessRecoveredA2aCancelTaskRequest,
        >,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("the joined launch has no retained remote cancellation")
    }
}

#[derive(Default)]
struct RejectTerminalConsultInputs {
    retained_terminal_handoffs: usize,
}

impl ResidentProgramConsultRecoveryInputPort<()> for RejectTerminalConsultInputs {
    type Work = ();
    type Output = ();
    type ChildRecoveryError = InvalidResidentRecordV1;
    type RetentionFailure = InvalidResidentRecordV1;
    type Executor = ResidentProgramUnavailableConsultExecutor;
    type StoreResolver = fn(
        &OperationObservationV1,
    ) -> Result<
        AuthorityStoreCoordinate,
        crate::resident::OperationObservationAuthorityStoreRejection,
    >;
    type AdmitCollection = fn(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>;
    type RecoverChild =
        fn(
            &crate::resident::consultation::ConsultationChildDispatchCandidate,
        )
            -> Result<crate::resident::RecoveredOperationObservation, InvalidResidentRecordV1>;
    type RetainChild = fn(
        &crate::resident::consultation::ConsultationChildDispatchCandidate,
        crate::resident::RecoveredOperationObservation,
        &mut dyn pale_ports::OperationCustodyPort,
        &mut dyn pale_ports::AntiRollbackAnchorPort,
    ) -> crate::resident::consultation::ConsultationChildRetentionOutcome<
        (),
        InvalidResidentRecordV1,
    >;

    fn with_consult_inputs<Return, UseInputs>(
        &mut self,
        _: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::consultation::ConsultationCollectionLimits,
            crate::resident::consultation::ResidentConsultationTaskLimits,
            &Self::StoreResolver,
            &mut Self::AdmitCollection,
            &mut Self::Executor,
            &mut Self::RecoverChild,
            &mut Self::RetainChild,
            Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        ) -> Return,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn retain_consult_turn(
        &mut self,
        _: ResidentProgramConsultTurn<
            Self::Work,
            Self::Output,
            Self::ChildRecoveryError,
            Self::RetentionFailure,
            (),
        >,
    ) {
        panic!("terminal-input refusal must retain the child handoff, not a root turn")
    }

    fn retain_consult_child_turn(
        &mut self,
        turn: crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn,
    ) {
        let crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn::Recovered(handoff) = turn else {
            panic!("terminal-input refusal must retain the exact recovered child")
        };
        assert!(matches!(
            handoff.state(),
            crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryState::Terminal(_)
        ));
        self.retained_terminal_handoffs += 1;
    }
}

type OperationAuthorization<'a> = dyn for<'context> FnMut(
        crate::resident::OperationControlAuthorizationContext<'context>,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type RotationAdmission<'a> = dyn FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type ObservationSigner<'a> = dyn FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;
type CheckpointSigner<'a> = dyn FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;

struct CancellationRecovery<'a> {
    expected_deployment: ResidentDeploymentAuthoritySelector,
    expected_request_id: ResidentRequestIdV1,
    expected_request_sha256: CommonRequestEnvelopeDigest,
    expected_locator: ControlRecordCoordinate,
    release_calls: Rc<Cell<usize>>,
    selection_checks: Rc<Cell<usize>>,
    input_lends: Rc<Cell<usize>>,
    observation_signs: Rc<Cell<usize>>,
    checkpoint_signs: Rc<Cell<usize>>,
    recovery: OperationControlRecoveryInput<'a>,
    admission: Option<OperationControlRecoveryAdmission<'a>>,
    request_ref: Option<ResidentRecordRefV1>,
    observation_ref: Option<ResidentRecordRefV1>,
    signing: SignedResidentAdmissionContextV1<'a>,
    checkpoint: Option<OperationCheckpointInput>,
    checkpoint_issued_at: UnixTimeV1,
    checkpoint_schema: &'a SchemaDigest,
    accumulator_limits: pale_ports::AccumulatorPreparationLimits,
    local_completion: Option<ResidentLocalCancellationCompletionInput>,
    authorize: &'a mut OperationAuthorization<'a>,
    resolve_store: &'a dyn crate::resident::OperationObservationAuthorityStoreResolver,
    admit_rotation: &'a mut RotationAdmission<'a>,
    sign_observation: &'a mut ObservationSigner<'a>,
    sign_checkpoint: &'a mut CheckpointSigner<'a>,
}

struct CancellationMutationAnchor<'a> {
    inner: &'a mut dyn pale_ports::AntiRollbackAnchorPort,
    advances: Rc<Cell<usize>>,
}

impl pale_ports::AntiRollbackAnchorPort for CancellationMutationAnchor<'_> {
    fn read(
        &mut self,
        key: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, pale_ports::AntiRollbackAnchorError> {
        self.inner.read(key)
    }

    fn compare_and_advance(
        &mut self,
        transition: &pale_ports::PreparedAntiRollbackAnchorTransition,
    ) -> Result<pale_ports::AntiRollbackAnchorAdvanceDisposition, pale_ports::AntiRollbackAnchorError>
    {
        self.advances.set(self.advances.get().saturating_add(1));
        self.inner.compare_and_advance(transition)
    }

    fn observe(
        &mut self,
        transition: &pale_ports::PreparedAntiRollbackAnchorTransition,
    ) -> Result<pale_ports::AntiRollbackAnchorObservation, pale_ports::AntiRollbackAnchorError>
    {
        self.inner.observe(transition)
    }
}

struct ObserveRecovery<'a> {
    recovery: Option<OperationControlRecoveryInput<'a>>,
    admission: Option<OperationControlRecoveryAdmission<'a>>,
    authorize_disclosure: Option<&'a mut OperationAuthorization<'a>>,
    response_deadline: std::time::Instant,
}

impl ResidentProgramRootRecoveryInputPort for ExistingRecovery<'_> {
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
        let locator = locator.ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
        if locator.coordinate().record_ref() != self.recovery.operation.expected_handle.record_ref()
            || locator.coordinate().record_sha256().as_str()
                != self
                    .recovery
                    .operation
                    .expected_handle
                    .record_sha256()
                    .as_str()
        {
            return Err(ResidentOperationFrameFailure::InvalidRequest);
        }
        let LocalOperationHandleRecoveryInput {
            context,
            expected_handle,
            maximum_parent_depth,
        } = self.recovery.operation;
        let restart = crate::resident::process_host::tests::restart::restart_input(
            OperationControlRecoveryInput {
                operation: LocalOperationHandleRecoveryInput {
                    context,
                    expected_handle,
                    maximum_parent_depth,
                },
                observation: self.recovery.observation,
                observation_limits: self.recovery.observation_limits,
                checkpoint_limits: self.recovery.checkpoint_limits,
                membership_limits: self.recovery.membership_limits,
            },
            1 << 20,
        );
        let checkpoint_authority = context.checkpoint_authority;
        let admission = self
            .admission
            .take()
            .ok_or(ResidentOperationFrameFailure::ReplayConflict)?;
        Ok(use_inputs(restart, admission, checkpoint_authority, self))
    }

    fn with_background_recovery<UseInputs>(
        &mut self,
        _: &ResidentDeploymentAuthoritySelector,
        use_inputs: UseInputs,
    ) -> Result<(), ResidentProgramTurnError>
    where
        UseInputs: for<'restart, 'admission> FnOnce(
            ResidentProcessOperationRestartInput<'restart>,
            OperationControlRecoveryAdmission<'admission>,
            usize,
            UnixTimeV1,
            &mut Self,
        )
            -> Result<(), ResidentProgramTurnError>,
    {
        let LocalOperationHandleRecoveryInput {
            context,
            expected_handle,
            maximum_parent_depth,
        } = self.recovery.operation;
        let restart = crate::resident::process_host::tests::restart::restart_input(
            OperationControlRecoveryInput {
                operation: LocalOperationHandleRecoveryInput {
                    context,
                    expected_handle,
                    maximum_parent_depth,
                },
                observation: self.recovery.observation,
                observation_limits: self.recovery.observation_limits,
                checkpoint_limits: self.recovery.checkpoint_limits,
                membership_limits: self.recovery.membership_limits,
            },
            1 << 20,
        );
        let admission = self
            .admission
            .take()
            .ok_or(ResidentProgramTurnError::RecoveryUnavailable)?;
        use_inputs(
            restart,
            admission,
            1 << 20,
            UnixTimeV1::new(70).expect("background recovery time"),
            self,
        )
    }

    fn retain_deferred_recovery(
        &mut self,
        _: Box<crate::resident::process_host::restart::ResidentProcessOperationRestartDisposition>,
    ) {
        self.retained_deferred.fetch_add(1, Ordering::SeqCst);
    }
}

impl ResidentProgramRootRecoveryInputPort for CancellationRecovery<'_> {
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
        let locator = locator.ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
        if locator.coordinate().record_ref() != self.recovery.operation.expected_handle.record_ref()
            || locator.coordinate().record_sha256().as_str()
                != self
                    .recovery
                    .operation
                    .expected_handle
                    .record_sha256()
                    .as_str()
        {
            return Err(ResidentOperationFrameFailure::InvalidRequest);
        }
        let LocalOperationHandleRecoveryInput {
            context,
            expected_handle,
            maximum_parent_depth,
        } = self.recovery.operation;
        let restart = crate::resident::process_host::tests::restart::restart_input(
            OperationControlRecoveryInput {
                operation: LocalOperationHandleRecoveryInput {
                    context,
                    expected_handle,
                    maximum_parent_depth,
                },
                observation: self.recovery.observation,
                observation_limits: self.recovery.observation_limits,
                checkpoint_limits: self.recovery.checkpoint_limits,
                membership_limits: self.recovery.membership_limits,
            },
            1 << 20,
        );
        let checkpoint_authority = context.checkpoint_authority;
        let admission = self
            .admission
            .take()
            .ok_or(ResidentOperationFrameFailure::ReplayConflict)?;
        Ok(use_inputs(restart, admission, checkpoint_authority, self))
    }

    fn retain_deferred_recovery(
        &mut self,
        _: Box<crate::resident::process_host::restart::ResidentProcessOperationRestartDisposition>,
    ) {
        panic!("Cancel does not run a deferred background lane")
    }
}

impl ResidentProgramRootRecoveryInputPort for ObserveRecovery<'_> {
    fn with_root_recovery<Output, UseInputs>(
        &mut self,
        _: &ResidentDeploymentAuthoritySelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<
            &crate::resident::process_host::operation_locator::ResidentProgramOperationLocator,
        >,
        _: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'restart, 'admission, 'checkpoint> FnOnce(
            ResidentProcessOperationRestartInput<'restart>,
            OperationControlRecoveryAdmission<'admission>,
            &'checkpoint ControlCheckpointSigningAuthorityAdmissionContextV1<'checkpoint>,
            &mut Self,
        ) -> Output,
    {
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn retain_deferred_recovery(
        &mut self,
        _: Box<crate::resident::process_host::restart::ResidentProcessOperationRestartDisposition>,
    ) {
        panic!("Observe never runs operation recovery")
    }
}

impl ResidentProgramRootRecoveryInputPort for ParentRecovery<'_> {
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
        let locator = locator.ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
        let store = self.store.clone();
        let profile = crate::local_operation_start_fixture::profile();
        let checkpoint_authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
            authority_ref: self.parent_checkpoint.checkpoint_signing_authority_ref(),
            authority_sha256: self.parent_checkpoint.checkpoint_signing_authority_sha256(),
            principal_id: self.parent_checkpoint.issuer_principal_id(),
            control_store_id: self.parent_checkpoint.control_store_id(),
            control_incarnation_id: self.parent_checkpoint.control_incarnation_id(),
            checkpoint_domain_id: self.parent_checkpoint.checkpoint_domain_id(),
            checkpoint_profile_ref: self.parent_checkpoint.checkpoint_profile_ref(),
            checkpoint_profile_sha256: self.parent_checkpoint.checkpoint_profile_sha256(),
            subject_key_id: self.parent_checkpoint.issuer_key_id(),
            subject_key_epoch: *self.parent_checkpoint.issuer_key_epoch(),
        };
        let context = LocalOperationRecoveryContext {
            store: &store,
            profile: &profile,
            handle_signing_authority: &self.historical.session_signing_authority,
            checkpoint_authority: &checkpoint_authority,
            historical: self.historical,
            maximum_record_bytes: 1 << 20,
            maximum_operation_lease_bytes: 1 << 20,
            maximum_session_lease_bytes: 1 << 20,
            membership_limits: pale_ports::AccumulatorMembershipLimits::new(3, 1024, 1 << 20)
                .expect("bounded membership"),
            observation_limits: OperationObservationRecoveryLimits::new(1, 1 << 20, 1 << 20)
                .expect("bounded observations"),
            checkpoint_limits: OperationCheckpointAncestryLimits::new(1, 1 << 20, 1)
                .expect("bounded checkpoints"),
        };
        let limits = crate::resident::process_host::restart::ResidentProcessRestartReadLimits {
            observation: context.observation_limits,
            checkpoints: context.checkpoint_limits,
            membership: context.membership_limits,
        };
        let restart = ResidentProcessOperationRestartInput {
            committed_observation_limits: None,
            operation: context,
            maximum_parent_depth: 0,
            current_head: limits,
            maximum_selection_bytes: 1 << 20,
            a2a_task: None,
            a2a_cancel_task: None,
        };
        let parent_request = self.parent_request;
        let parent_checkpoint = self.parent_checkpoint;
        let signing = &self.historical.session_signing_authority;
        let mut admit_request = |_: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
            let actual = serde_json::to_value(&wire).map_err(|_| {
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "common request",
                    invariant: "fixture wire did not serialize",
                }
            })?;
            if actual != serde_json::to_value(parent_request).expect("request serializes") {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "common request",
                    invariant: "recovered request differs from the committed parent",
                });
            }
            Ok(parent_request.clone())
        };
        let mut admit_handle_authority =
            |_: &OperationHandlePreimageV1, _: Option<&OperationHandleV1>| Ok(());
        let mut verify_handle_signature =
            |_: &[u8],
             _: &OperationHandlePreimageV1,
             _: &OperationHandleDigest,
             _: &CanonicalSignatureV1| Ok(());
        let mut admit_checkpoint = |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
            let actual = serde_json::to_value(&wire).map_err(|_| {
                InvalidResidentRecordV1::ContextualAdmission {
                    record: "control checkpoint",
                    invariant: "fixture wire did not serialize",
                }
            })?;
            if actual != serde_json::to_value(parent_checkpoint).expect("checkpoint serializes") {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "control checkpoint",
                    invariant: "recovered checkpoint differs from the committed parent",
                });
            }
            Ok(parent_checkpoint.clone())
        };
        let mut resolve_observation_signer =
            |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| Ok(signing.clone());
        let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
        let mut admit_rotation = |_: &ResidentRecordRefV1,
                                  _: &ArtifactDigest,
                                  _: &ResidentRecordRefV1,
                                  _: &ArtifactDigest| Ok(());
        let mut verify_observation_signature =
            |_: &[u8],
             _: &OperationObservationPreimageV1,
             _: &OperationObservationDigest,
             _: &CanonicalSignatureV1| Ok(());
        let mut verify_checkpoint_signature =
            |_: &[u8],
             _: &ControlStateCheckpointPreimageV1,
             _: &ControlStateCheckpointDigest,
             _: &CanonicalSignatureV1| Ok(());
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
        let _ = locator;
        Ok(use_inputs(restart, admission, &checkpoint_authority, self))
    }

    fn retain_deferred_recovery(
        &mut self,
        _: Box<crate::resident::process_host::restart::ResidentProcessOperationRestartDisposition>,
    ) {
        panic!("parent lookup does not run a deferred background lane")
    }
}

impl crate::resident::ResidentServiceInvokeInputPort for ParentRecovery<'_> {
    fn with_invoke_inputs<Output, UseInputs>(
        &mut self,
        deployment: &crate::resident::ResidentServiceDeploymentSelector,
        request: &CommonRequestEnvelopeV1,
        locator: Option<&ControlRecordCoordinate>,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentOperationFrameFailure>
    where
        UseInputs: for<'operation, 'admission, 'checkpoint> FnOnce(
            crate::resident::ResidentServiceInvokeRecoveryInputs<
                'operation,
                'admission,
                'checkpoint,
            >,
        ) -> Output,
    {
        let selector = ResidentDeploymentAuthoritySelector {
            principal_id: deployment.principal_id.clone(),
            deployment_binding_ref: deployment.deployment_binding_ref.clone(),
            deployment_binding_sha256: deployment.deployment_binding_sha256.clone(),
            runtime_authority_ref: deployment.runtime_authority_ref.clone(),
            runtime_authority_sha256: deployment.runtime_authority_sha256.clone(),
        };
        let locator = locator
            .map(|coordinate| {
                ResidentOperationFrameLocator::new(
                    coordinate.record_ref().as_str().to_owned(),
                    coordinate.record_sha256().as_str().to_owned(),
                )
            })
            .ok_or(ResidentOperationFrameFailure::InvalidRequest)?;
        let locator = crate::resident::process_host::operation_locator::ResidentProgramOperationLocator::parse(&locator)
            .map_err(|_| ResidentOperationFrameFailure::InvalidRequest)?;
        <Self as ResidentProgramRootRecoveryInputPort>::with_root_recovery(
            self,
            &selector,
            request,
            Some(&locator),
            |restart, recovery_admission, checkpoint_authority, _| {
                use_inputs(crate::resident::ResidentServiceInvokeRecoveryInputs {
                    operation: restart.operation,
                    maximum_parent_depth: restart.maximum_parent_depth,
                    limits: crate::resident::ResidentServiceInvokeRecoveryLimits {
                        observation: restart.current_head.observation,
                        checkpoints: restart.current_head.checkpoints,
                        membership: restart.current_head.membership,
                        maximum_selection_bytes: restart.maximum_selection_bytes,
                    },
                    recovery_admission,
                    checkpoint_authority,
                })
            },
        )
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

impl ResidentProgramRootRecoveryInputPort for RootRecovery<'_> {
    fn with_root_recovery<Output, UseInputs>(
        &mut self,
        _: &ResidentDeploymentAuthoritySelector,
        _: &CommonRequestEnvelopeV1,
        _: Option<
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
        let semantic = self.semantic;
        let checkpoint_authority = semantic.checkpoint_authority;
        Ok(with_unreachable_restart_inputs(
            semantic,
            |restart, admission| use_inputs(restart, admission, checkpoint_authority, self),
        ))
    }

    fn retain_deferred_recovery(
        &mut self,
        _: Box<crate::resident::process_host::restart::ResidentProcessOperationRestartDisposition>,
    ) {
        panic!("fresh-start fixture cannot retain deferred background recovery")
    }
}

macro_rules! impl_unreachable_root_provider_recovery_owner {
    ($owner:ty, $writer:ty) => {
        impl
            crate::resident::process_host::operation_recovery_sweep::ResidentProcessRootProviderRecoveryOwner<
                $writer,
            > for $owner
        {
            type Output = crate::resident::process_host::operation_request_host::ResidentOperationRequestOutcome;
            type Error = ();

            fn complete_local_cancellation<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                _: Box<ResidentProcessRecoveredLocalCancellation>,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<Self::Output, Self::Error> {
                panic!("start-only fixture must not recover cancellation")
            }

            fn recover_operation_resolution<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                recovered: Box<ResidentProcessRecoveredOperationResolution>,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<
                crate::resident::process_host::operation_recovery_sweep::ResidentProcessRootProviderResolution<
                    Self::Output,
                >,
                Self::Error,
            > {
                let operation = recovered.start.control.operation().operation();
                let (handle_ref, handle) = operation.handle();
                Ok(crate::resident::process_host::operation_recovery_sweep::ResidentProcessRootProviderResolution::Provider(
                    crate::resident::process_host::operation_request_host::ResidentOperationRequestOutcome::Pending(
                        pale_ports::ResidentOperationFrameLocator::new(
                            handle_ref.as_str().to_owned(),
                            handle
                                .canonical_operation_handle_sha256()
                                .as_str()
                                .to_owned(),
                        ),
                    ),
                ))
            }

            fn recover_provider_resolved<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                _: Box<ResidentProcessRecoveredProviderResolution>,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<Self::Output, Self::Error> {
                panic!("start-only fixture must not recover provider resolution")
            }

            fn recover_provider_prepared<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                _: Box<ResidentProcessRecoveredPreparedAttempt>,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<Self::Output, Self::Error> {
                panic!("start-only fixture must not recover provider preparation")
            }

            fn recover_provider_dispatched<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                _: Box<ResidentProcessRecoveredAttempt>,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<Self::Output, Self::Error> {
                panic!("start-only fixture must not recover provider dispatch")
            }

            fn recover_native_prepared<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                _: Box<ResidentProcessRecoveredPreparedAttempt>,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<Self::Output, Self::Error> {
                panic!("start-only fixture must not recover native preparation")
            }

            fn recover_native_dispatched<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                _: Box<ResidentProcessRecoveredAttempt>,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<Self::Output, Self::Error> {
                panic!("start-only fixture must not recover native dispatch")
            }
        }
    };
}

impl_unreachable_root_provider_recovery_owner!(RootRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_root_provider_recovery_owner!(ParentRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_root_provider_recovery_owner!(ExistingRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_root_provider_recovery_owner!(CancellationRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_root_provider_recovery_owner!(ObserveRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_root_provider_recovery_owner!(DelegateSchedulerRecovery, UnprovedHeldWriter);
impl_unreachable_root_provider_recovery_owner!(ParentRecovery<'_>, super::program::HeldWriter);
impl_unreachable_root_provider_recovery_owner!(
    JoinedProgramA2aRecovery<'_, '_, '_, '_>,
    UnprovedHeldWriter
);

macro_rules! impl_unreachable_delegate_recovery_owner {
    ($owner:ty, $writer:ty) => {
        impl
            crate::resident::process_host::delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner<
                $writer,
            > for $owner
        {
            fn recover_delegate_operation_resolution<'store>(
                &mut self,
                host: &mut ResidentProcessHost<'store, $writer>,
                recovered: Box<ResidentProcessRecoveredOperationResolution>,
                access: ResidentDeploymentOperationAccess<'_>,
                input: crate::resident::process_host::program::ResidentProgramDelegateResolutionInput<
                    '_,
                    '_,
                >,
            ) -> Result<Self::Output, Self::Error> {
                assert!(input.current().now >= input.request().issued_at());
                assert!(input.maximum_selection_bytes() > 0);
                assert!(input.maximum_record_bytes() > 0);
                let _ = input.session();
                match crate::resident::process_host::operation_recovery_sweep::ResidentProcessRootProviderRecoveryOwner::recover_operation_resolution(
                    self,
                    host,
                    recovered,
                    access,
                )? {
                    crate::resident::process_host::operation_recovery_sweep::ResidentProcessRootProviderResolution::Provider(output)
                    | crate::resident::process_host::operation_recovery_sweep::ResidentProcessRootProviderResolution::Native(output) => Ok(output),
                }
            }

            fn recover_a2a_pre_task<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                _: Box<ResidentProcessRecoveredAttempt>,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<Self::Output, Self::Error> {
                panic!("non-A2A fixture must not recover a pre-Task send")
            }

            fn recover_a2a_task<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                _: Box<crate::resident::process_host::restart::ResidentProcessRecoveredA2aTask>,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<Self::Output, Self::Error> {
                panic!("non-A2A fixture must not recover a remote Task")
            }

            fn recover_a2a_cancel_task<'store>(
                &mut self,
                _: &mut ResidentProcessHost<'store, $writer>,
                _: Box<
                    crate::resident::process_host::restart::ResidentProcessRecoveredA2aCancelTaskRequest,
                >,
                _: ResidentDeploymentOperationAccess<'_>,
            ) -> Result<Self::Output, Self::Error> {
                panic!("non-A2A fixture must not recover remote cancellation")
            }
        }
    };
}

impl_unreachable_delegate_recovery_owner!(RootRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_delegate_recovery_owner!(ParentRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_delegate_recovery_owner!(ExistingRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_delegate_recovery_owner!(CancellationRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_delegate_recovery_owner!(ObserveRecovery<'_>, UnprovedHeldWriter);
impl_unreachable_delegate_recovery_owner!(DelegateSchedulerRecovery, UnprovedHeldWriter);
impl_unreachable_delegate_recovery_owner!(ParentRecovery<'_>, super::program::HeldWriter);

impl
    crate::resident::process_host::delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner<
        UnprovedHeldWriter,
    > for JoinedProgramA2aRecovery<'_, '_, '_, '_>
{
    fn recover_delegate_operation_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        recovered: Box<ResidentProcessRecoveredOperationResolution>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProgramDelegateResolutionInput<'_, '_>,
    ) -> Result<Self::Output, Self::Error> {
        let mut owner = crate::resident::process_host::service_assembly::
            resident_service_delegate_recovery_owner::<_, UnprovedHeldWriter>(&mut self.a2a);
        crate::resident::process_host::delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner::recover_delegate_operation_resolution(
            &mut owner,
            host,
            recovered,
            access,
            input,
        )
    }

    fn recover_a2a_pre_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        recovered: Box<crate::resident::process_host::restart::ResidentProcessRecoveredAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        let mut owner = crate::resident::process_host::service_assembly::
            resident_service_delegate_recovery_owner::<_, UnprovedHeldWriter>(&mut self.a2a);
        crate::resident::process_host::delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner::recover_a2a_pre_task(
            &mut owner,
            host,
            recovered,
            access,
        )
    }

    fn recover_a2a_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        recovered: Box<crate::resident::process_host::restart::ResidentProcessRecoveredA2aTask>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        let mut owner = crate::resident::process_host::service_assembly::
            resident_service_delegate_recovery_owner::<_, UnprovedHeldWriter>(&mut self.a2a);
        crate::resident::process_host::delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner::recover_a2a_task(
            &mut owner,
            host,
            recovered,
            access,
        )
    }

    fn recover_a2a_cancel_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        recovered: Box<
            crate::resident::process_host::restart::ResidentProcessRecoveredA2aCancelTaskRequest,
        >,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        let mut owner = crate::resident::process_host::service_assembly::
            resident_service_delegate_recovery_owner::<_, UnprovedHeldWriter>(&mut self.a2a);
        crate::resident::process_host::delegate_recovery_sweep::ResidentProcessDelegateRecoveryOwner::recover_a2a_cancel_task(
            &mut owner,
            host,
            recovered,
            access,
        )
    }
}

fn reject_cancel_stop(
    _: &crate::resident::consultation::ConsultationCollection,
    _: &crate::resident::consultation::ConsultationRequirementSummary,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("non-Consult cancellation must not admit a stop")
}

fn reject_cancel_collection(
    _: &crate::resident::consultation::ConsultationCollection,
) -> Result<(), InvalidResidentRecordV1> {
    panic!("non-Consult cancellation must not admit a collection")
}

impl ResidentProgramCancellationInputPort for CancellationRecovery<'_> {
    type Reservation = ();

    fn reserve_cancellation(
        &mut self,
        _: &ResidentDeploymentAuthoritySelector,
        _: &CommonRequestEnvelopeV1,
        _: &ControlRecordCoordinate,
    ) -> Result<Self::Reservation, ResidentOperationFrameFailure> {
        Ok(())
    }

    fn release_cancellation(&mut self, _: Self::Reservation) {
        self.release_calls
            .set(self.release_calls.get().saturating_add(1));
    }

    fn with_selected_cancellation_inputs<Return, UseInputs>(
        &mut self,
        deployment: &ResidentDeploymentAuthoritySelector,
        request: &CommonRequestEnvelopeV1,
        locator: &ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: for<'input> FnOnce(ResidentProgramCancellationInputs<'input>) -> Return,
    {
        self.selection_checks
            .set(self.selection_checks.get().saturating_add(1));
        if deployment != &self.expected_deployment
            || request.request_id() != &self.expected_request_id
            || request.canonical_request_sha256() != &self.expected_request_sha256
            || locator != &self.expected_locator
        {
            return Err(ResidentOperationFrameFailure::ConfigurationUnavailable);
        }
        self.input_lends
            .set(self.input_lends.get().saturating_add(1));
        let mut admit_stop = reject_cancel_stop;
        let mut admit_collection = reject_cancel_collection;
        let observation_signs = Rc::clone(&self.observation_signs);
        let sign_observation = &mut *self.sign_observation;
        let mut counted_sign_observation =
            move |key: &[u8],
                  preimage: &OperationObservationPreimageV1,
                  digest: &OperationObservationDigest| {
                observation_signs.set(observation_signs.get().saturating_add(1));
                sign_observation(key, preimage, digest)
            };
        let checkpoint_signs = Rc::clone(&self.checkpoint_signs);
        let sign_checkpoint = &mut *self.sign_checkpoint;
        let mut counted_sign_checkpoint =
            move |key: &[u8],
                  preimage: &ControlStateCheckpointPreimageV1,
                  digest: &ControlStateCheckpointDigest| {
                checkpoint_signs.set(checkpoint_signs.get().saturating_add(1));
                sign_checkpoint(key, preimage, digest)
            };
        let allocation = ResidentProgramCancellationAllocation {
            request_ref: self
                .request_ref
                .take()
                .ok_or(ResidentOperationFrameFailure::ReplayConflict)?,
            observation_ref: self
                .observation_ref
                .take()
                .ok_or(ResidentOperationFrameFailure::ReplayConflict)?,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: self.signing.expected_schema_sha256,
                signing_authority: self.signing.signing_authority,
            },
            checkpoint: self
                .checkpoint
                .take()
                .ok_or(ResidentOperationFrameFailure::ReplayConflict)?,
            checkpoint_issued_at: self.checkpoint_issued_at,
            checkpoint_schema: self.checkpoint_schema,
            accumulator_limits: self.accumulator_limits,
            local_completion: self
                .local_completion
                .take()
                .ok_or(ResidentOperationFrameFailure::ReplayConflict)?,
            consultation_limits: crate::resident::consultation::ConsultationCollectionLimits::new(
                1, 16_384, 4096,
            )
            .map_err(|_| ResidentOperationFrameFailure::ConfigurationUnavailable)?,
        };
        Ok(use_inputs(ResidentProgramCancellationInputs {
            allocation,
            authorize: &mut *self.authorize,
            resolve_store: self.resolve_store,
            admit_rotation: &mut *self.admit_rotation,
            sign_observation: &mut counted_sign_observation,
            sign_checkpoint: &mut counted_sign_checkpoint,
            admit_stop: &mut admit_stop,
            admit_collection: &mut admit_collection,
        }))
    }

    fn retain_dispatched_cancellation(
        &mut self,
        _: Self::Reservation,
        _: Box<ResidentProcessDispatchedCancellationCustody>,
    ) {
        panic!("successful cancellation fixture must not lose dispatched custody")
    }

    fn retain_cancellation_custody(
        &mut self,
        _: Self::Reservation,
        _: Box<ResidentProcessCancellationCustody>,
    ) {
        panic!("successful cancellation fixture must not retain cancellation custody")
    }

    fn retain_a2a_cancellation(&mut self, _: Self::Reservation, _: ResidentProcessA2aDelegatePoll) {
        panic!("non-A2A cancellation fixture must not retain A2A uncertainty")
    }

    fn retain_consultation_cancellation(
        &mut self,
        _: Self::Reservation,
        _: Box<ResidentProcessConsultationCancellationCustody>,
    ) {
        panic!("non-Consult cancellation fixture must not retain Consult custody")
    }

    fn retain_consultation_cancellation_preparation(
        &mut self,
        _: Self::Reservation,
        _: Box<ResidentProcessConsultationCancellationPreparation>,
    ) {
        panic!("non-Consult cancellation fixture must not retain Consult preparation")
    }
}

macro_rules! impl_unavailable_cancellation_inputs {
    ($owner:ty) => {
        impl ResidentProgramCancellationInputPort for $owner {
            type Reservation = ();

            fn reserve_cancellation(
                &mut self,
                _: &ResidentDeploymentAuthoritySelector,
                _: &CommonRequestEnvelopeV1,
                _: &ControlRecordCoordinate,
            ) -> Result<Self::Reservation, ResidentOperationFrameFailure> {
                Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
            }

            fn release_cancellation(&mut self, _: Self::Reservation) {
                unreachable!("unavailable Cancel inputs cannot reserve capacity")
            }

            fn retain_dispatched_cancellation(
                &mut self,
                _: Self::Reservation,
                _: Box<ResidentProcessDispatchedCancellationCustody>,
            ) {
                unreachable!("unavailable Cancel inputs cannot retain custody")
            }

            fn retain_cancellation_custody(
                &mut self,
                _: Self::Reservation,
                _: Box<ResidentProcessCancellationCustody>,
            ) {
                unreachable!("unavailable Cancel inputs cannot retain custody")
            }

            fn retain_a2a_cancellation(
                &mut self,
                _: Self::Reservation,
                _: ResidentProcessA2aDelegatePoll,
            ) {
                unreachable!("unavailable Cancel inputs cannot retain A2A uncertainty")
            }

            fn retain_consultation_cancellation(
                &mut self,
                _: Self::Reservation,
                _: Box<ResidentProcessConsultationCancellationCustody>,
            ) {
                unreachable!("unavailable Cancel inputs cannot retain Consult custody")
            }

            fn retain_consultation_cancellation_preparation(
                &mut self,
                _: Self::Reservation,
                _: Box<ResidentProcessConsultationCancellationPreparation>,
            ) {
                unreachable!("unavailable Cancel inputs cannot retain Consult preparation")
            }
        }
    };
}

impl_unavailable_cancellation_inputs!(RootRecovery<'_>);
impl_unavailable_cancellation_inputs!(DelegateSchedulerRecovery);
impl_unavailable_cancellation_inputs!(ParentRecovery<'_>);
impl_unavailable_cancellation_inputs!(ExistingRecovery<'_>);
impl_unavailable_cancellation_inputs!(JoinedProgramA2aRecovery<'_, '_, '_, '_>);
impl_unavailable_cancellation_inputs!(ObserveRecovery<'_>);

impl<'a> ResidentProgramObserveInputPort<'a> for ObserveRecovery<'a> {
    fn with_observe_inputs<Output, UseInputs>(
        &mut self,
        locator: &pale_ports::ControlRecordCoordinate,
        use_inputs: UseInputs,
    ) -> Result<Output, ResidentObserveFrameFailure>
    where
        UseInputs: for<'request> FnOnce(ResidentProgramObserveInputs<'request, 'a>) -> Output,
    {
        let recovery = self
            .recovery
            .take()
            .ok_or(ResidentObserveFrameFailure::ReplayConflict)?;
        if recovery.operation.expected_handle != locator {
            return Err(ResidentObserveFrameFailure::InvalidRequest);
        }
        let recovery_admission = self
            .admission
            .take()
            .ok_or(ResidentObserveFrameFailure::ReplayConflict)?;
        let mut verify_caller_signature =
            |_: &CommonRequestEnvelopePreimageV1,
             _: &CommonRequestEnvelopeDigest,
             _: &CanonicalSignatureV1| { Ok(()) };
        let mut admit_replay = |_: &ResidentRequestIdV1,
                                _: &CommonRequestEnvelopeDigest,
                                _: &[u8]| {
            Ok(crate::resident::process_host::observe_request::ResidentObserveReplayDisposition::FirstAdmission)
        };
        let authorize_disclosure = self
            .authorize_disclosure
            .take()
            .ok_or(ResidentObserveFrameFailure::ReplayConflict)?;
        Ok(use_inputs(ResidentProgramObserveInputs {
            recovery,
            verify_caller_signature: &mut verify_caller_signature,
            admit_replay: &mut admit_replay,
            recovery_admission,
            authorize_disclosure,
            response_deadline: self.response_deadline,
        }))
    }
}

macro_rules! impl_unavailable_observe_inputs {
    ($owner:ty) => {
        impl<'recovery> ResidentProgramObserveInputPort<'recovery> for $owner {
            fn with_observe_inputs<Output, UseInputs>(
                &mut self,
                _: &ControlRecordCoordinate,
                _: UseInputs,
            ) -> Result<Output, ResidentObserveFrameFailure>
            where
                UseInputs: for<'request> FnOnce(
                    ResidentProgramObserveInputs<'request, 'recovery>,
                ) -> Output,
            {
                Err(ResidentObserveFrameFailure::ResidentUnavailable)
            }
        }
    };
}

impl_unavailable_observe_inputs!(RootRecovery<'_>);
impl_unavailable_observe_inputs!(DelegateSchedulerRecovery);
impl_unavailable_observe_inputs!(ParentRecovery<'_>);
impl_unavailable_observe_inputs!(ExistingRecovery<'_>);
impl_unavailable_observe_inputs!(JoinedProgramA2aRecovery<'_, '_, '_, '_>);
impl_unavailable_observe_inputs!(CancellationRecovery<'_>);

struct StartCryptography {
    checkpoint: RefCell<Option<ControlStateCheckpointPreimageV1>>,
}

struct FailingCheckpointCryptography {
    checkpoint: RefCell<Option<ControlStateCheckpointPreimageV1>>,
    handle_signs: Arc<AtomicUsize>,
    observation_signs: Arc<AtomicUsize>,
    checkpoint_preparations: Arc<AtomicUsize>,
    checkpoint_signs: Arc<AtomicUsize>,
}

struct ConsultRosterCryptography {
    checkpoints: RefCell<VecDeque<ControlStateCheckpointPreimageV1>>,
    fail_handle_calls: BTreeSet<usize>,
    handle_ids: Arc<Mutex<Vec<OperationHandleIdV1>>>,
    observation_signs: Arc<AtomicUsize>,
}

struct RejectCryptography;

impl ResidentProgramInvokeCryptographyPort for RejectCryptography {
    fn admit_compiled_authority(
        &self,
        _: &CommonRequestEnvelopeV1,
        _: &crate::resident::AdmittedAmbientSession,
        _: &crate::resident::AmbientSessionAdmissionContext<'_>,
        _: &OperationHandlePreimageV1,
        _: &OperationLeaseV1,
        _: &crate::semantic::ResolvedOperation,
        _: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
        _: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("cold recovery must not re-admit a new operation start")
    }

    fn sign_handle(
        &self,
        _: &[u8],
        _: &OperationHandlePreimageV1,
        _: &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        panic!("cold recovery must not sign another operation handle")
    }

    fn sign_observation(
        &self,
        _: &[u8],
        _: &OperationObservationPreimageV1,
        _: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        panic!("cold recovery must not sign another initial observation")
    }

    fn prepare_checkpoint(
        &self,
        _: &crate::resident::PreparedLocalOperationCheckpoint,
        _: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, crate::resident::LocalOperationStartError> {
        panic!("cold recovery must not prepare another start checkpoint")
    }

    fn sign_checkpoint(
        &self,
        _: &[u8],
        _: &ControlStateCheckpointPreimageV1,
        _: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        panic!("cold recovery must not sign another start checkpoint")
    }
}

impl ResidentProgramInvokeCryptographyPort for StartCryptography {
    fn admit_compiled_authority(
        &self,
        _: &CommonRequestEnvelopeV1,
        _: &crate::resident::AdmittedAmbientSession,
        _: &crate::resident::AmbientSessionAdmissionContext<'_>,
        _: &OperationHandlePreimageV1,
        _: &OperationLeaseV1,
        _: &crate::semantic::ResolvedOperation,
        _: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
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
        let mut checkpoint = checkpoint_preimage(prepared, checkpoint)?;
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

impl ResidentProgramInvokeCryptographyPort for ConsultRosterCryptography {
    fn admit_compiled_authority(
        &self,
        _: &CommonRequestEnvelopeV1,
        _: &crate::resident::AdmittedAmbientSession,
        _: &crate::resident::AmbientSessionAdmissionContext<'_>,
        _: &OperationHandlePreimageV1,
        _: &OperationLeaseV1,
        _: &crate::semantic::ResolvedOperation,
        _: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
        _: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }

    fn sign_handle(
        &self,
        _: &[u8],
        preimage: &OperationHandlePreimageV1,
        _: &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        let mut handle_ids = self.handle_ids.lock().expect("handle identity log");
        handle_ids.push(preimage.operation_handle_id.clone());
        if self.fail_handle_calls.contains(&handle_ids.len()) {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test Consult parent handle",
                invariant: "parent signing unavailable",
            });
        }
        Ok(crate::local_operation_start_fixture::signature())
    }

    fn sign_observation(
        &self,
        _: &[u8],
        _: &OperationObservationPreimageV1,
        _: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        self.observation_signs.fetch_add(1, Ordering::SeqCst);
        Ok(crate::local_operation_start_fixture::signature())
    }

    fn prepare_checkpoint(
        &self,
        prepared: &crate::resident::PreparedLocalOperationCheckpoint,
        checkpoint_id: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, crate::resident::LocalOperationStartError> {
        let mut checkpoint = self
            .checkpoints
            .borrow_mut()
            .pop_front()
            .ok_or(crate::resident::LocalOperationStartError::Coordinate)?;
        checkpoint.control_checkpoint_id = checkpoint_id.clone();
        let issued_at = checkpoint.issued_at;
        let mut checkpoint = checkpoint_preimage(prepared, checkpoint)?;
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

impl crate::resident::ResidentServiceInvokeCryptographyPort for StartCryptography {
    fn admit_compiled_authority(
        &self,
        request: &CommonRequestEnvelopeV1,
        session: &crate::resident::AdmittedAmbientSession,
        current: &crate::resident::AmbientSessionAdmissionContext<'_>,
        handle: &OperationHandlePreimageV1,
        lease: &OperationLeaseV1,
        operation: &crate::semantic::ResolvedOperation,
        publication: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
        checkpoint: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1> {
        <Self as ResidentProgramInvokeCryptographyPort>::admit_compiled_authority(
            self,
            request,
            session,
            current,
            handle,
            lease,
            operation,
            publication,
            checkpoint,
        )
    }

    fn sign_handle(
        &self,
        bytes: &[u8],
        preimage: &OperationHandlePreimageV1,
        digest: &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        <Self as ResidentProgramInvokeCryptographyPort>::sign_handle(self, bytes, preimage, digest)
    }

    fn sign_observation(
        &self,
        bytes: &[u8],
        preimage: &OperationObservationPreimageV1,
        digest: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        <Self as ResidentProgramInvokeCryptographyPort>::sign_observation(
            self, bytes, preimage, digest,
        )
    }

    fn prepare_checkpoint(
        &self,
        prepared: &crate::resident::PreparedLocalOperationCheckpoint,
        checkpoint_id: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, crate::resident::LocalOperationStartError> {
        <Self as ResidentProgramInvokeCryptographyPort>::prepare_checkpoint(
            self,
            prepared,
            checkpoint_id,
        )
    }

    fn sign_checkpoint(
        &self,
        bytes: &[u8],
        preimage: &ControlStateCheckpointPreimageV1,
        digest: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        <Self as ResidentProgramInvokeCryptographyPort>::sign_checkpoint(
            self, bytes, preimage, digest,
        )
    }
}

impl ResidentProgramInvokeCryptographyPort for FailingCheckpointCryptography {
    fn admit_compiled_authority(
        &self,
        _: &CommonRequestEnvelopeV1,
        _: &crate::resident::AdmittedAmbientSession,
        _: &crate::resident::AmbientSessionAdmissionContext<'_>,
        _: &OperationHandlePreimageV1,
        _: &OperationLeaseV1,
        _: &crate::semantic::ResolvedOperation,
        _: &crate::resident::LocalOperationPublicationPolicyInput<'_>,
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
        self.handle_signs.fetch_add(1, Ordering::SeqCst);
        Ok(crate::local_operation_start_fixture::signature())
    }

    fn sign_observation(
        &self,
        _: &[u8],
        _: &OperationObservationPreimageV1,
        _: &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        self.observation_signs.fetch_add(1, Ordering::SeqCst);
        Ok(crate::local_operation_start_fixture::signature())
    }

    fn prepare_checkpoint(
        &self,
        prepared: &crate::resident::PreparedLocalOperationCheckpoint,
        checkpoint_id: &ControlCheckpointIdV1,
    ) -> Result<ControlStateCheckpointPreimageV1, crate::resident::LocalOperationStartError> {
        self.checkpoint_preparations.fetch_add(1, Ordering::SeqCst);
        let mut checkpoint = self
            .checkpoint
            .borrow_mut()
            .take()
            .ok_or(crate::resident::LocalOperationStartError::Coordinate)?;
        checkpoint.control_checkpoint_id = checkpoint_id.clone();
        let issued_at = checkpoint.issued_at;
        let mut checkpoint = checkpoint_preimage(prepared, checkpoint)?;
        checkpoint.issued_at = issued_at;
        Ok(checkpoint)
    }

    fn sign_checkpoint(
        &self,
        _: &[u8],
        _: &ControlStateCheckpointPreimageV1,
        _: &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1> {
        self.checkpoint_signs.fetch_add(1, Ordering::SeqCst);
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "test checkpoint signer",
            invariant: "unavailable",
        })
    }
}

struct ConsultPolicy(Option<ResidentProcessConsultPolicy>);

impl ResidentProcessInvokePolicyEvaluator for ConsultPolicy {
    fn evaluate_invoke(
        &mut self,
        _: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

struct DelegatePolicy(Option<ResidentProcessDelegatePolicy>);

impl ResidentProcessInvokePolicyEvaluator for DelegatePolicy {
    fn evaluate_invoke(
        &mut self,
        _: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl ResidentProcessConsultPolicyEvaluator for DelegatePolicy {
    fn evaluate_consult(
        &mut self,
        _: ResidentProcessConsultPolicyContext<'_>,
    ) -> Result<ResidentProcessConsultPolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl ResidentProcessDelegatePolicyEvaluator for DelegatePolicy {
    fn evaluate_delegate(
        &mut self,
        context: ResidentProcessDelegatePolicyContext<'_>,
    ) -> Result<ResidentProcessDelegatePolicy, ResidentProcessHostSelectionError> {
        assert_eq!(context.request.operation(), &context.operation.record.id);
        assert_eq!(
            context.parent.operation().1.operation_class(),
            &OperationClassV1::Invoke,
        );
        self.0
            .take()
            .ok_or(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl crate::resident::process_host::consult_recovery::ResidentConsultationPolicyEvaluator<()>
    for DelegatePolicy
{
    type Error = ();

    fn evaluate(
        &mut self,
        _: &ImmutableRecordReferenceV1,
        _: &crate::resident::consultation::ResidentConsultationPolicyFrontier,
    ) -> Result<
        crate::resident::consultation::ResidentConsultationSupervisionDecision<()>,
        Self::Error,
    > {
        panic!("start-only Delegate policy cannot supervise Consult recovery")
    }
}

impl ResidentProcessConsultPolicyEvaluator for ConsultPolicy {
    fn evaluate_consult(
        &mut self,
        context: ResidentProcessConsultPolicyContext<'_>,
    ) -> Result<ResidentProcessConsultPolicy, ResidentProcessHostSelectionError> {
        assert_eq!(context.request.operation(), &context.operation.record.id);
        assert_eq!(
            &context.request.session_admission().session_admission_id,
            context.session.lease().session_admission_id()
        );
        self.0
            .take()
            .ok_or(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl ResidentProcessDelegatePolicyEvaluator for ConsultPolicy {
    fn evaluate_delegate(
        &mut self,
        _: ResidentProcessDelegatePolicyContext<'_>,
    ) -> Result<ResidentProcessDelegatePolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl crate::resident::process_host::consult_recovery::ResidentConsultationPolicyEvaluator<()>
    for ConsultPolicy
{
    type Error = ();

    fn evaluate(
        &mut self,
        _: &ImmutableRecordReferenceV1,
        _: &crate::resident::consultation::ResidentConsultationPolicyFrontier,
    ) -> Result<
        crate::resident::consultation::ResidentConsultationSupervisionDecision<()>,
        Self::Error,
    > {
        panic!("start-only Consult policy cannot supervise recovery")
    }
}

struct AdvanceConsultPolicy {
    calls: Arc<AtomicUsize>,
}

impl ResidentProcessInvokePolicyEvaluator for AdvanceConsultPolicy {
    fn evaluate_invoke(
        &mut self,
        _: ResidentProcessInvokePolicyContext<'_>,
    ) -> Result<ResidentProcessInvokePolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl ResidentProcessConsultPolicyEvaluator for AdvanceConsultPolicy {
    fn evaluate_consult(
        &mut self,
        _: ResidentProcessConsultPolicyContext<'_>,
    ) -> Result<ResidentProcessConsultPolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl ResidentProcessDelegatePolicyEvaluator for AdvanceConsultPolicy {
    fn evaluate_delegate(
        &mut self,
        _: ResidentProcessDelegatePolicyContext<'_>,
    ) -> Result<ResidentProcessDelegatePolicy, ResidentProcessHostSelectionError> {
        Err(ResidentProcessHostSelectionError::StartRequestAssociation)
    }
}

impl crate::resident::process_host::consult_recovery::ResidentConsultationPolicyEvaluator<()>
    for AdvanceConsultPolicy
{
    type Error = ();

    fn evaluate(
        &mut self,
        _: &ImmutableRecordReferenceV1,
        _: &crate::resident::consultation::ResidentConsultationPolicyFrontier,
    ) -> Result<
        crate::resident::consultation::ResidentConsultationSupervisionDecision<()>,
        Self::Error,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(crate::resident::consultation::ResidentConsultationSupervisionDecision::Stop(()))
    }
}

struct AdvanceConsultExecutor {
    polls: Arc<AtomicUsize>,
}

impl crate::resident::task_owner::ResidentOperationTaskExecutor<u8, u16>
    for AdvanceConsultExecutor
{
    fn try_accept(
        &mut self,
        task: crate::resident::task_owner::ResidentOperationTask<u8>,
    ) -> Result<(), crate::resident::task_owner::ResidentOperationTask<u8>> {
        Err(task)
    }

    fn try_completion(
        &mut self,
    ) -> Option<crate::resident::task_owner::ResidentOperationTaskCompletion<u8, u16>> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        None
    }
}

struct AdvanceConsultStoreResolver {
    store: AuthorityStoreCoordinate,
}

impl crate::resident::OperationObservationAuthorityStoreResolver for AdvanceConsultStoreResolver {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<
        AuthorityStoreCoordinate,
        crate::resident::OperationObservationAuthorityStoreRejection,
    > {
        Ok(self.store.clone())
    }
}

fn admit_advance_consult_collection(
    _: &crate::resident::consultation::ConsultationCollection,
) -> Result<(), InvalidResidentRecordV1> {
    Ok(())
}

fn unreachable_advance_consult_child_recovery(
    _: &crate::resident::consultation::ConsultationChildDispatchCandidate,
) -> Result<crate::resident::RecoveredOperationObservation, &'static str> {
    panic!("a selected Consult stop cannot recover a child")
}

fn unreachable_advance_consult_child_retention(
    _: &crate::resident::consultation::ConsultationChildDispatchCandidate,
    _: crate::resident::RecoveredOperationObservation,
    _: &mut dyn pale_ports::OperationCustodyPort,
    _: &mut dyn pale_ports::AntiRollbackAnchorPort,
) -> crate::resident::consultation::ConsultationChildRetentionOutcome<u8, &'static str> {
    panic!("a selected Consult stop cannot retain a child")
}

type AdvanceConsultChildRetention =
    fn(
        &crate::resident::consultation::ConsultationChildDispatchCandidate,
        crate::resident::RecoveredOperationObservation,
        &mut dyn pale_ports::OperationCustodyPort,
        &mut dyn pale_ports::AntiRollbackAnchorPort,
    ) -> crate::resident::consultation::ConsultationChildRetentionOutcome<u8, &'static str>;

struct AdvanceConsultInputs {
    resolver: AdvanceConsultStoreResolver,
    executor: AdvanceConsultExecutor,
    admit_collection: fn(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>,
    recover_child: fn(
        &crate::resident::consultation::ConsultationChildDispatchCandidate,
    ) -> Result<crate::resident::RecoveredOperationObservation, &'static str>,
    retain_child: AdvanceConsultChildRetention,
    retained: Arc<AtomicUsize>,
    fail_after_callback: bool,
}

impl ResidentProgramConsultRecoveryInputPort<()> for AdvanceConsultInputs {
    type Work = u8;
    type Output = u16;
    type ChildRecoveryError = &'static str;
    type RetentionFailure = &'static str;
    type Executor = AdvanceConsultExecutor;
    type StoreResolver = AdvanceConsultStoreResolver;
    type AdmitCollection = fn(
        &crate::resident::consultation::ConsultationCollection,
    ) -> Result<(), InvalidResidentRecordV1>;
    type RecoverChild = fn(
        &crate::resident::consultation::ConsultationChildDispatchCandidate,
    )
        -> Result<crate::resident::RecoveredOperationObservation, &'static str>;
    type RetainChild =
        fn(
            &crate::resident::consultation::ConsultationChildDispatchCandidate,
            crate::resident::RecoveredOperationObservation,
            &mut dyn pale_ports::OperationCustodyPort,
            &mut dyn pale_ports::AntiRollbackAnchorPort,
        )
            -> crate::resident::consultation::ConsultationChildRetentionOutcome<u8, &'static str>;

    fn with_consult_inputs<Return, UseInputs>(
        &mut self,
        use_inputs: UseInputs,
    ) -> Result<Return, ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(
            crate::resident::consultation::ConsultationCollectionLimits,
            crate::resident::consultation::ResidentConsultationTaskLimits,
            &Self::StoreResolver,
            &mut Self::AdmitCollection,
            &mut Self::Executor,
            &mut Self::RecoverChild,
            &mut Self::RetainChild,
            Option<&mut crate::resident::HistoricalConsultationAdmission<'_>>,
        ) -> Return,
    {
        let output = use_inputs(
            crate::resident::consultation::ConsultationCollectionLimits::new(1, 65_536, 65_536)
                .expect("bounded collection"),
            crate::resident::consultation::ResidentConsultationTaskLimits::new(
                std::num::NonZeroUsize::new(1).expect("one in-flight Consult task"),
                std::num::NonZeroUsize::new(1).expect("one Consult batch member"),
            ),
            &self.resolver,
            &mut self.admit_collection,
            &mut self.executor,
            &mut self.recover_child,
            &mut self.retain_child,
            None,
        );
        if self.fail_after_callback {
            Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
        } else {
            Ok(output)
        }
    }

    fn retain_consult_turn(
        &mut self,
        turn: ResidentProgramConsultTurn<
            Self::Work,
            Self::Output,
            Self::ChildRecoveryError,
            Self::RetentionFailure,
            (),
        >,
    ) {
        let ResidentProgramConsultTurn::Policy(continuation) = turn else {
            panic!("the exact no-completion frontier must apply its selected policy")
        };
        let (_, disposition) = (*continuation).into_parts();
        assert!(matches!(
            disposition,
            crate::resident::consultation::ResidentConsultationTurnDisposition::Stopped(
                crate::resident::consultation::ResidentConsultationTurnStopped::Selected(_),
            )
        ));
        self.retained.fetch_add(1, Ordering::SeqCst);
    }

    fn can_retain_consult_key(&self, _: &ResidentProgramConsultTurnKey) -> bool {
        true
    }

    fn retain_keyed_consult_turn(
        &mut self,
        retained: ResidentProgramKeyedConsultTurn<
            ResidentProgramConsultTurn<
                Self::Work,
                Self::Output,
                Self::ChildRecoveryError,
                Self::RetentionFailure,
                (),
            >,
        >,
    ) -> Result<
        (),
        ResidentProgramConsultRetentionFailure<
            ResidentProgramConsultTurn<
                Self::Work,
                Self::Output,
                Self::ChildRecoveryError,
                Self::RetentionFailure,
                (),
            >,
        >,
    > {
        let (_, turn) = retained.into_parts();
        self.retain_consult_turn(turn);
        Ok(())
    }

    fn retain_consult_child_turn(
        &mut self,
        _: crate::resident::process_host::consult_child_recovery_sweep::ResidentProcessConsultChildRecoveryTurn,
    ) {
        panic!("point-addressed root Consult cannot produce a child sweep handoff")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CallbackProducedConsultTurn {
    Policy(Box<u8>),
    PrePolicy(Box<u8>),
    Refusal(Box<u8>),
    Completion(Box<u8>),
}

#[test]
fn consult_callback_output_precedes_a_post_callback_lender_error() {
    let turns = [
        CallbackProducedConsultTurn::Policy(Box::new(1)),
        CallbackProducedConsultTurn::PrePolicy(Box::new(2)),
        CallbackProducedConsultTurn::Refusal(Box::new(3)),
        CallbackProducedConsultTurn::Completion(Box::new(4)),
    ];
    for expected in turns {
        let mut inputs = AdvanceConsultInputs {
            resolver: AdvanceConsultStoreResolver {
                store: crate::local_operation_start_fixture::store_coordinate(),
            },
            executor: AdvanceConsultExecutor {
                polls: Arc::new(AtomicUsize::new(0)),
            },
            admit_collection: admit_advance_consult_collection,
            recover_child: unreachable_advance_consult_child_recovery,
            retain_child: unreachable_advance_consult_child_retention,
            retained: Arc::new(AtomicUsize::new(0)),
            fail_after_callback: true,
        };
        let actual = inputs
            .use_consult_inputs(|_, _, _, _, _, _, _, _| expected.clone())
            .expect("the callback-produced linear turn takes precedence");
        assert_eq!(actual, expected);
    }
}

fn operation_lease_basis(lease: &OperationLeaseV1) -> ResidentProcessOperationLeaseBasis {
    ResidentProcessOperationLeaseBasis {
        capabilities: lease.capabilities().to_vec(),
        effects: lease.effects().to_vec(),
        read_set: lease.read_set().to_vec(),
        evidence_obligations: lease.evidence_obligations().to_vec(),
        budget_units: *lease.budget_units(),
        timeout_millis: *lease.timeout_millis(),
        expires_at: lease.expires_at(),
        invocation_correlation: lease.invocation_correlation().clone(),
    }
}

fn consult_policy(
    semantic: &crate::resident::LocalOperationStartInput<'_>,
    opened_at: UnixTimeV1,
    handle_expires_at: UnixTimeV1,
) -> ConsultPolicy {
    let evidence = consult_immutable("consult/policy");
    let slot = consult_roster_slot(
        "existing-evidence",
        crate::resident::consultation::ConsultationRosterTemplateSource::EvidenceResolved {
            evidence: evidence.clone(),
        },
    );
    consult_policy_with_slots(semantic, opened_at, handle_expires_at, evidence, vec![slot])
}

fn consult_immutable(value: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: crate::local_operation_start_fixture::record_ref(value),
        record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
    }
}

fn consult_roster_slot<Child>(
    id: &str,
    source: crate::resident::consultation::ConsultationRosterTemplateSource<Child>,
) -> crate::resident::consultation::ConsultationRosterTemplateSlot<Child> {
    crate::resident::consultation::ConsultationRosterTemplateSlot::new(
        crate::resident::consultation::ConsultationSlotId::parse(id).expect("slot id"),
        crate::resident::consultation::ConsultationRequirement::Required,
        consult_immutable(&format!("consult/requirement/{id}")),
        ParticipantRefV1::Application {
            application_id: id.into(),
        },
        None,
        None,
        source,
    )
}

fn compiled_consult_start_policy(
    semantic: &crate::resident::LocalOperationStartInput<'_>,
    handle_expires_at: UnixTimeV1,
) -> start_factories::ResidentCompiledOperationStartPolicy {
    let publication = crate::resident::LocalOperationPublicationPolicy::from_admitted_input(
        crate::resident::LocalOperationPublicationPolicyInput {
            target_scope: semantic.publication_policy.target_scope,
            classification: semantic.publication_policy.classification,
            visibility: semantic.publication_policy.visibility,
            disclosure_ceiling: semantic.publication_policy.disclosure_ceiling,
            payload_resolution_policy: semantic.publication_policy.payload_resolution_policy,
            actor_provenance: semantic.publication_policy.actor_provenance,
        },
    );
    start_factories::ResidentCompiledOperationStartPolicy::from_compiled_projection(
        semantic.checkpoint_profile.clone(),
        publication,
        1 << 20,
        handle_expires_at,
        semantic.handle.retain_until,
        pale_ports::AccumulatorPreparationLimits::new(4, 1024, 1024, 512, 65_536)
            .expect("bounded accumulator"),
    )
}

fn consult_policy_with_slots(
    semantic: &crate::resident::LocalOperationStartInput<'_>,
    opened_at: UnixTimeV1,
    handle_expires_at: UnixTimeV1,
    completion_policy: ImmutableRecordReferenceV1,
    slots: Vec<
        crate::resident::consultation::ConsultationRosterTemplateSlot<
            ResidentProcessConsultFreshChildPolicy,
        >,
    >,
) -> ConsultPolicy {
    ConsultPolicy(Some(ResidentProcessConsultPolicy {
        start: start_factories::ResidentStartConsultPolicy::from_compiled_projection(
            compiled_consult_start_policy(semantic, handle_expires_at),
            crate::resident::consultation::ConsultationRosterTemplate::new(
                completion_policy,
                opened_at,
                handle_expires_at,
                slots,
                crate::resident::consultation::ConsultationCollectionLimits::new(4, 65_536, 4096)
                    .expect("bounded collection"),
            )
            .expect("bounded declarative Consult roster"),
        ),
        lease: operation_lease_basis(semantic.context.operation_lease),
    }))
}

fn fresh_consult_child_policy(
    semantic: &crate::resident::LocalOperationStartInput<'_>,
    request: CommonRequestEnvelopeV1,
    operation_class: OperationClassV1,
) -> ResidentProcessConsultFreshChildPolicy {
    ResidentProcessConsultFreshChildPolicy {
        request: Box::new(request),
        operation_class,
        start: compiled_consult_start_policy(semantic, semantic.handle.expires_at),
        lease: operation_lease_basis(semantic.context.operation_lease),
    }
}

fn delegate_policy(semantic: &crate::resident::LocalOperationStartInput<'_>) -> DelegatePolicy {
    let publication = crate::resident::LocalOperationPublicationPolicy::from_admitted_input(
        crate::resident::LocalOperationPublicationPolicyInput {
            target_scope: semantic.publication_policy.target_scope,
            classification: semantic.publication_policy.classification,
            visibility: semantic.publication_policy.visibility,
            disclosure_ceiling: semantic.publication_policy.disclosure_ceiling,
            payload_resolution_policy: semantic.publication_policy.payload_resolution_policy,
            actor_provenance: semantic.publication_policy.actor_provenance,
        },
    );
    DelegatePolicy(Some(ResidentProcessDelegatePolicy {
        start: start_factories::ResidentStartDelegatePolicy::from_compiled_projection(
            start_factories::ResidentCompiledOperationStartPolicy::from_compiled_projection(
                semantic.checkpoint_profile.clone(),
                publication,
                1 << 20,
                semantic.handle.expires_at,
                semantic.handle.retain_until,
                pale_ports::AccumulatorPreparationLimits::new(4, 1024, 1024, 512, 65_536)
                    .expect("bounded accumulator"),
            ),
        ),
        lease: operation_lease_basis(semantic.context.operation_lease),
    }))
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

fn resealed_request(
    original: &CommonRequestEnvelopeV1,
    evidence: &ResidentCurrentRequestEvidence,
    session: &crate::resident::AdmittedAmbientSession,
    now: UnixTimeV1,
    change: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
) -> CommonRequestEnvelopeV1 {
    let mut value = original.to_preimage();
    value.session_admission = evidence
        .session_admission(session, now)
        .unwrap()
        .as_record()
        .clone();
    value.generation = Some(session.lease().compiled_generation_digest().clone());
    value.issued_at = now;
    value.expires_at = UnixTimeV1::new(now.get() + 10).unwrap();
    change(&mut value);
    crate::effect_case_lineage_fixture::request(&value)
}

fn resealed_operation_target_request(
    original: &CommonRequestEnvelopeV1,
    evidence: &ResidentCurrentRequestEvidence,
    session: &crate::resident::AdmittedAmbientSession,
    now: UnixTimeV1,
    operation_target: OperationTargetReferenceV1,
    change: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
) -> CommonRequestEnvelopeV1 {
    resealed_request(original, evidence, session, now, |value| {
        bind_request_to_operation_target(value, operation_target);
        change(value);
    })
}

fn bind_request_to_operation_target(
    request: &mut CommonRequestEnvelopePreimageV1,
    operation_target: OperationTargetReferenceV1,
) {
    let original_body = request.body.clone();
    assert!(request.operation_target.replace(operation_target).is_none());
    assert_eq!(
        request.body, original_body,
        "target binding preserves the body"
    );
}

fn operation_handle_payload(handle: &OperationHandleV1) -> serde_json::Value {
    serde_json::to_value(handle).expect("operation handle serializes")
}

fn operation_target_reference(
    record_ref: &ResidentRecordRefV1,
    digest: &OperationHandleDigest,
) -> OperationTargetReferenceV1 {
    OperationTargetReferenceV1 {
        operation_handle_ref: record_ref.clone(),
        operation_handle_sha256: digest.clone(),
    }
}

fn retained_operation_target_reference(
    evidence: &crate::resident::operation_custody::OperationStartRecoveryEvidence,
) -> OperationTargetReferenceV1 {
    OperationTargetReferenceV1 {
        operation_handle_ref: evidence.handle().coordinate().record_ref().clone(),
        operation_handle_sha256: OperationHandleDigest::parse(
            evidence
                .handle()
                .coordinate()
                .record_sha256()
                .as_str()
                .to_owned(),
        )
        .expect("retained operation-handle digest is typed"),
    }
}

fn same_class_foreign_operation_handle_payload(handle: &OperationHandleV1) -> serde_json::Value {
    let mut payload = operation_handle_payload(handle);
    let members = payload
        .as_object_mut()
        .expect("operation handle projects as an object");
    members.insert(
        "operation_handle_id".to_owned(),
        serde_json::Value::String("same-class-foreign-operation".to_owned()),
    );
    let signature = members
        .remove("signature")
        .expect("operation handle carries a signature");
    members
        .remove("canonical_operation_handle_sha256")
        .expect("operation handle carries its canonical digest");
    let digest = OperationHandleDigest::hash_canonical(&payload)
        .expect("foreign operation handle preimage canonicalizes");
    let members = payload
        .as_object_mut()
        .expect("operation handle remains an object");
    members.insert(
        "canonical_operation_handle_sha256".to_owned(),
        serde_json::Value::String(digest.as_str().to_owned()),
    );
    members.insert("signature".to_owned(), signature);
    payload
}

#[test]
fn signed_target_rejects_same_class_handle_swaps_for_every_point_addressed_kind() {
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(1);
    let generation = generations.pop().expect("one fixture generation");
    with_issued_context(
        &generation,
        &profile,
        "signed-target-swap",
        |_| {},
        crate::session_admission_fixture::LocalStartContext::Inherited,
        |issued, original, _, _, _, _| {
            let exact_payload = operation_handle_payload(issued.handle().1);
            let foreign_payload = same_class_foreign_operation_handle_payload(issued.handle().1);
            assert_eq!(
                exact_payload.get("operation_class"),
                foreign_payload.get("operation_class"),
                "the hostile target keeps the same operation class",
            );

            let mut preimage = original.to_preimage();
            bind_request_to_operation_target(
                &mut preimage,
                operation_target_reference(
                    issued.handle().0,
                    issued.handle().1.canonical_operation_handle_sha256(),
                ),
            );
            let request = crate::effect_case_lineage_fixture::request(&preimage);
            assert_eq!(
                request.body(),
                original.body(),
                "signed target binding preserves the operation-specific body",
            );
            let signed_common_request = pale_core::canon::to_canonical_string(&request)
                .expect("target-bound request is canonical");
            let exact_locator = operation_frame_locator(issued);
            let foreign_digest = foreign_payload
                .get("canonical_operation_handle_sha256")
                .and_then(serde_json::Value::as_str)
                .expect("foreign handle carries a digest");
            let foreign_locator = ResidentOperationFrameLocator::new(
                crate::local_operation_start_fixture::record_ref("operation/same-class-foreign")
                    .as_str()
                    .to_owned(),
                foreign_digest.to_owned(),
            );

            for (kind, exact_frame, swapped_frame) in [
                (
                    ResidentOperationRequestKind::AdvanceInvoke,
                    ResidentOperationFrameRequest::AdvanceInvoke {
                        locator: exact_locator.clone(),
                        signed_common_request: signed_common_request.clone(),
                    },
                    ResidentOperationFrameRequest::AdvanceInvoke {
                        locator: foreign_locator.clone(),
                        signed_common_request: signed_common_request.clone(),
                    },
                ),
                (
                    ResidentOperationRequestKind::AdvanceConsult,
                    ResidentOperationFrameRequest::AdvanceConsult {
                        locator: exact_locator.clone(),
                        signed_common_request: signed_common_request.clone(),
                    },
                    ResidentOperationFrameRequest::AdvanceConsult {
                        locator: foreign_locator.clone(),
                        signed_common_request: signed_common_request.clone(),
                    },
                ),
                (
                    ResidentOperationRequestKind::StartDelegate,
                    ResidentOperationFrameRequest::StartDelegate {
                        parent: exact_locator.clone(),
                        signed_common_request: signed_common_request.clone(),
                    },
                    ResidentOperationFrameRequest::StartDelegate {
                        parent: foreign_locator.clone(),
                        signed_common_request: signed_common_request.clone(),
                    },
                ),
                (
                    ResidentOperationRequestKind::AdvanceDelegate,
                    ResidentOperationFrameRequest::AdvanceDelegate {
                        locator: exact_locator.clone(),
                        signed_common_request: signed_common_request.clone(),
                    },
                    ResidentOperationFrameRequest::AdvanceDelegate {
                        locator: foreign_locator.clone(),
                        signed_common_request: signed_common_request.clone(),
                    },
                ),
            ] {
                ResidentProgramOperationResolution::admit(
                    &exact_frame,
                    request.clone(),
                    kind,
                    Some(exact_locator.clone()),
                )
                .expect("the signed operation target admits its exact locator");
                assert!(matches!(
                    ResidentProgramOperationResolution::admit(
                        &swapped_frame,
                        request.clone(),
                        kind,
                        Some(foreign_locator.clone()),
                    ),
                    Err(ResidentOperationFrameFailure::InvalidRequest)
                ));
            }
        },
    );
}

fn point_addressed_frame(
    kind: ResidentOperationRequestKind,
    locator: ResidentOperationFrameLocator,
    request: &CommonRequestEnvelopeV1,
) -> ResidentOperationFrameRequest {
    let signed_common_request =
        pale_core::canon::to_canonical_string(request).expect("signed request is canonical");
    match kind {
        ResidentOperationRequestKind::AdvanceInvoke => {
            ResidentOperationFrameRequest::AdvanceInvoke {
                locator,
                signed_common_request,
            }
        }
        ResidentOperationRequestKind::AdvanceConsult => {
            ResidentOperationFrameRequest::AdvanceConsult {
                locator,
                signed_common_request,
            }
        }
        ResidentOperationRequestKind::StartDelegate => {
            ResidentOperationFrameRequest::StartDelegate {
                parent: locator,
                signed_common_request,
            }
        }
        ResidentOperationRequestKind::AdvanceDelegate => {
            ResidentOperationFrameRequest::AdvanceDelegate {
                locator,
                signed_common_request,
            }
        }
        _ => panic!("the fixture requires one point-addressed request kind"),
    }
}

fn frame(request: &CommonRequestEnvelopeV1) -> ResidentOperationFrameRequest {
    ResidentOperationFrameRequest::StartInvoke {
        signed_common_request: pale_core::canon::to_canonical_string(request).unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
fn with_admission(
    evidence_values: impl FnOnce(
        &ResidentCurrentRequestEvidence,
    ) -> Vec<Result<ResidentCurrentRequestEvidence, ()>>,
    times: Vec<UnixTimeV1>,
    signature_ok: bool,
    kind: ResidentOperationRequestKind,
    run: impl FnOnce(
        ResidentProgramCurrentRequestAdmission<
            '_,
            (),
            EvidencePort,
            Clock,
            ProvingAuthority,
            Signature,
        >,
        ResidentProcessHost<'_, UnprovedHeldWriter>,
        &[ResidentProgramDeploymentAuthority],
        &crate::resident::AdmittedAmbientSession,
        &CommonRequestEnvelopeV1,
        &ResidentCurrentRequestEvidence,
        &Arc<Mutex<usize>>,
        &Arc<Mutex<usize>>,
        &crate::resident::LocalOperationStartInput<'_>,
        ControlStateCheckpointPreimageV1,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
    ),
) {
    with_admission_for_request(
        evidence_values,
        times,
        signature_ok,
        kind,
        None,
        |_| {},
        run,
    )
}

#[allow(clippy::too_many_arguments)]
fn with_admission_for_request(
    evidence_values: impl FnOnce(
        &ResidentCurrentRequestEvidence,
    ) -> Vec<Result<ResidentCurrentRequestEvidence, ()>>,
    times: Vec<UnixTimeV1>,
    signature_ok: bool,
    kind: ResidentOperationRequestKind,
    native_tool: Option<&str>,
    change_request: impl FnOnce(&mut CommonRequestEnvelopePreimageV1),
    run: impl FnOnce(
        ResidentProgramCurrentRequestAdmission<
            '_,
            (),
            EvidencePort,
            Clock,
            ProvingAuthority,
            Signature,
        >,
        ResidentProcessHost<'_, UnprovedHeldWriter>,
        &[ResidentProgramDeploymentAuthority],
        &crate::resident::AdmittedAmbientSession,
        &CommonRequestEnvelopeV1,
        &ResidentCurrentRequestEvidence,
        &Arc<Mutex<usize>>,
        &Arc<Mutex<usize>>,
        &crate::resident::LocalOperationStartInput<'_>,
        ControlStateCheckpointPreimageV1,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
    ),
) {
    let (mut generations, profile) = if native_tool.is_some() {
        let inputs = crate::local_operation_start_fixture::host_source_generation_input_copies_with(
            "Read",
            2,
            |operation| operation.capability_requirements.truncate(1),
        );
        let profile = inputs[0].2.clone();
        let generations = inputs
            .into_iter()
            .map(|(compilation, materializations, _)| {
                crate::resident::AdmittedCompiledGeneration::admit(
                    compilation,
                    materializations,
                    synthetic_set_policy(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        (generations, profile)
    } else {
        crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(
            2,
        )
    };
    let retained = generations.pop().unwrap();
    let semantic = generations.pop().unwrap();
    let store = crate::local_operation_start_fixture::store_coordinate();
    crate::local_operation_start_fixture::with_operation_start_input_at_store_and_native(
        &semantic,
        &profile,
        "program-current-request",
        change_request,
        OperationClassV1::Invoke,
        &store,
        native_tool,
        |input, original, checkpoint, parents| {
            let port = MemoryAuthorityStore::default();
            let mut application = ready_application(&port, store.clone(), 101);
            application.fixture_retain_generation(retained);
            let (published, advertised) =
                active_predecessors(input.current.generation.compiled_generation);
            assert_eq!(
                advertised.advertised_generation_head_digest(),
                input
                    .current
                    .generation
                    .active_head
                    .advertised_generation_head_digest()
            );
            let _ = application.publish(published.clone()).unwrap();
            let _ = application.advertise(advertised.clone()).unwrap();
            let _ = application
                .activate(
                    input.current.generation.active_head.clone(),
                    input.current.generation.activation_fence.clone(),
                )
                .unwrap();
            let (session, _) = application
                .admit_session(
                    SessionAdmissionLeaseWireV1::from(parents.session.lease()),
                    parents.current,
                    &mut ProvingAuthority,
                )
                .unwrap();
            let authority =
                ResidentProgramDeploymentAuthority::fixture_from_current(parents.current)
                    .expect("complete current authority");
            let selector = authority.fixture_selector().clone();
            let authorities = [authority];
            let process = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
                vec![unproved_deployment_for_selector(
                    &selector,
                    store.clone(),
                    application,
                )],
                std::num::NonZeroUsize::new(1).unwrap(),
                ResidentOperationHost::new(task_state()),
            )
            .unwrap();
            let mut records = RecoveryResolver::default();
            records.insert_generation(input.current.generation.compiled_generation);
            records.insert_effective_profile(input.current.generation.effective_profile);
            records.insert_published(&published);
            records.insert_advertised(&advertised);
            records.insert_fence(input.current.generation.activation_fence);
            records.insert_active(input.current.generation.active_head);
            let historical = Historical {
                records,
                session_signing_authority: parents.signing.clone(),
            };
            process.deployments[0]
                .application
                .recover_exact_retained_session(
                    session.lease().session_admission_id(),
                    session.lease().session_admission_lease_digest(),
                    &historical,
                )
                .unwrap_or_else(|error| panic!("exact retained session recovery: {error:?}"));
            let exact_evidence = evidence_from(parents.current);
            let evidence_reads = Arc::new(Mutex::new(0));
            let clock_reads = Arc::new(Mutex::new(0));
            let evidence = EvidencePort {
                values: Arc::new(Mutex::new(evidence_values(&exact_evidence).into())),
                reads: Arc::clone(&evidence_reads),
            };
            let clock = Clock {
                values: Arc::new(Mutex::new(times.into())),
                reads: Arc::clone(&clock_reads),
            };
            let admission = ResidentProgramCurrentRequestAdmission::new(
                1 << 20,
                vec![ResidentProgramRequestPolicy::new(
                    kind,
                    policy_from(
                        original,
                        session.lease().compiled_generation_digest().clone(),
                    ),
                )],
                evidence,
                clock,
                &historical,
                ProvingAuthority,
                Signature(signature_ok),
            )
            .unwrap();
            run(
                admission,
                process,
                &authorities,
                &session,
                original,
                &exact_evidence,
                &evidence_reads,
                &clock_reads,
                &input,
                checkpoint,
                parents,
            );
        },
    );
}

#[test]
fn retained_delegate_work_turn_advances_once_at_the_exact_deployment() {
    with_admission(
        |_| Vec::new(),
        Vec::new(),
        true,
        ResidentOperationRequestKind::StartDelegate,
        |admission, process, authorities, _, original, _, _, _, _, _, _| {
            let selector = authorities[0].fixture_selector().clone();
            let store = crate::local_operation_start_fixture::store_coordinate();
            let calls = Rc::new(Cell::new(0));
            let mut custody = absent_start_store(original)
                .at_store(&store)
                .with_unlisted_indexes_absent();
            let sessions = MemoryAuthorityStore::default();
            let mut anchor = initial_anchor();
            let owner = ResidentProgramOperationTurnOwner::new(
                AcceptPeer,
                admission,
                vec![ResidentProgramDeploymentOperationPorts::new(
                    selector.clone(),
                    store.clone(),
                    &mut custody,
                    &sessions,
                    &mut anchor,
                )],
                FreshEntropy::new(Arc::new(AtomicUsize::new(0))),
                CountingAuthority::default(),
                StartCryptography {
                    checkpoint: RefCell::new(None),
                },
                DelegateSchedulerRecovery {
                    selector,
                    store,
                    calls: Rc::clone(&calls),
                },
                ResidentProgramConsultRecoveryUnavailable,
            )
            .expect("the exact Delegate scheduler owner assembles");
            let mut program = ResidentProgram::fixture_from_process_with_authorities(
                process,
                authorities.to_vec(),
                DelegatePolicy(None),
                owner,
            );

            assert!(program.has_retained_phase());
            assert_eq!(
                program.continue_one_retained_phase(),
                ResidentProgramRetainedPhaseTurn::Advanced
            );
            assert_eq!(calls.get(), 1);
            assert!(!program.has_retained_phase());
            assert_eq!(
                program.continue_one_retained_phase(),
                ResidentProgramRetainedPhaseTurn::Idle
            );
            assert_eq!(calls.get(), 1);
        },
    );
}

#[test]
fn exact_resealed_request_admits_against_retained_policy_and_current_evidence() {
    let now = UnixTimeV1::new(61).unwrap();
    with_admission(
        |exact| vec![Ok(exact.clone())],
        vec![now],
        true,
        ResidentOperationRequestKind::StartInvoke,
        |mut admission, process, authorities, session, original, exact, reads, clocks, _, _, _| {
            let request = resealed_request(original, exact, session, now, |_| {});
            if let Err(error) = admission.admit(&process, authorities, &(), &frame(&request)) {
                panic!(
                    "exact admission failed: {error:?}; evidence reads {}; clock reads {}",
                    *reads.lock().unwrap(),
                    *clocks.lock().unwrap()
                );
            }
            assert_eq!(*reads.lock().unwrap(), 1);
            assert_eq!(*clocks.lock().unwrap(), 1);
        },
    );
}

#[test]
fn point_addressed_target_is_signed_without_replacing_the_operation_body() {
    let now = UnixTimeV1::new(61).unwrap();
    let target_ref = crate::local_operation_start_fixture::record_ref("operation/signed-target");
    let target_digest = OperationHandleDigest::hash_canonical(&serde_json::json!({
        "operation_handle_id": "signed-target",
    }))
    .expect("target digest canonicalizes");
    let locator = ResidentOperationFrameLocator::new(
        target_ref.as_str().to_owned(),
        target_digest.as_str().to_owned(),
    );

    for kind in [
        ResidentOperationRequestKind::AdvanceInvoke,
        ResidentOperationRequestKind::AdvanceConsult,
        ResidentOperationRequestKind::StartDelegate,
        ResidentOperationRequestKind::AdvanceDelegate,
    ] {
        with_admission(
            |exact| vec![Ok(exact.clone())],
            vec![now],
            true,
            kind,
            |mut admission,
             process,
             authorities,
             session,
             original,
             exact,
             reads,
             clocks,
             _,
             _,
             _| {
                let request = resealed_operation_target_request(
                    original,
                    exact,
                    session,
                    now,
                    operation_target_reference(&target_ref, &target_digest),
                    |_| {},
                );
                assert_eq!(request.body(), original.body());
                admission
                    .admit(
                        &process,
                        authorities,
                        &(),
                        &point_addressed_frame(kind, locator.clone(), &request),
                    )
                    .unwrap_or_else(|error| panic!("exact signed target admission: {error:?}"));
                assert_eq!(*reads.lock().unwrap(), 1);
                assert_eq!(*clocks.lock().unwrap(), 1);
            },
        );
    }
}

#[test]
fn malformed_point_addressed_target_stops_before_current_owner_callbacks() {
    let now = UnixTimeV1::new(61).unwrap();
    let exact_ref = crate::local_operation_start_fixture::record_ref("operation/signed-target");
    let exact_digest = OperationHandleDigest::hash_canonical(&serde_json::json!({
        "operation_handle_id": "signed-target",
    }))
    .expect("target digest canonicalizes");
    let exact_locator = ResidentOperationFrameLocator::new(
        exact_ref.as_str().to_owned(),
        exact_digest.as_str().to_owned(),
    );
    let foreign_digest = OperationHandleDigest::hash_canonical(&serde_json::json!({
        "operation_handle_id": "same-class-foreign-target",
    }))
    .expect("foreign target digest canonicalizes");
    let foreign_locator = ResidentOperationFrameLocator::new(
        crate::local_operation_start_fixture::record_ref("operation/same-class-foreign")
            .as_str()
            .to_owned(),
        foreign_digest.as_str().to_owned(),
    );

    with_admission(
        |_| vec![],
        vec![],
        true,
        ResidentOperationRequestKind::AdvanceInvoke,
        |mut admission, process, authorities, session, original, exact, reads, clocks, _, _, _| {
            let target_bound = resealed_operation_target_request(
                original,
                exact,
                session,
                now,
                operation_target_reference(&exact_ref, &exact_digest),
                |_| {},
            );
            assert!(matches!(
                admission.admit(
                    &process,
                    authorities,
                    &(),
                    &point_addressed_frame(
                        ResidentOperationRequestKind::AdvanceInvoke,
                        foreign_locator,
                        &target_bound,
                    ),
                ),
                Err(ResidentOperationFrameFailure::InvalidRequest)
            ));

            let missing = resealed_request(original, exact, session, now, |_| {});
            assert!(matches!(
                admission.admit(
                    &process,
                    authorities,
                    &(),
                    &point_addressed_frame(
                        ResidentOperationRequestKind::AdvanceInvoke,
                        exact_locator.clone(),
                        &missing,
                    ),
                ),
                Err(ResidentOperationFrameFailure::InvalidRequest)
            ));

            let duplicate_representation = resealed_operation_target_request(
                original,
                exact,
                session,
                now,
                operation_target_reference(&exact_ref, &exact_digest),
                |value| {
                    value
                        .scope
                        .authority_evidence
                        .push(AuthorityEvidenceV1::new(
                            exact_ref.clone(),
                            ArtifactDigest::parse(exact_digest.as_str().to_owned())
                                .expect("operation digest uses SHA-256 representation"),
                        ));
                },
            );
            let duplicate_frame = point_addressed_frame(
                ResidentOperationRequestKind::AdvanceInvoke,
                exact_locator,
                &duplicate_representation,
            );
            assert!(matches!(
                admission.admit(&process, authorities, &(), &duplicate_frame),
                Err(ResidentOperationFrameFailure::InvalidRequest)
            ));
            assert_eq!(*reads.lock().unwrap(), 0);
            assert_eq!(*clocks.lock().unwrap(), 0);
        },
    );
}

#[test]
fn retained_policy_rejects_cross_kind_overlap_but_allows_distinct_body_schemas() {
    let now = UnixTimeV1::new(61).unwrap();
    with_admission(
        |exact| vec![Ok(exact.clone())],
        vec![now],
        true,
        ResidentOperationRequestKind::StartInvoke,
        |_, _, _, session, original, _, _, _, _, _, _| {
            let generation = session.lease().compiled_generation_digest().clone();
            let invoke = policy_from(original, generation.clone());
            let overlap = invoke.clone();
            let historical = RecoveryResolver::default();
            let overlapping = ResidentProgramCurrentRequestAdmission::<
                (),
                EvidencePort,
                Clock,
                ProvingAuthority,
                Signature,
            >::new(
                1 << 20,
                vec![
                    ResidentProgramRequestPolicy::new(
                        ResidentOperationRequestKind::StartInvoke,
                        invoke.clone(),
                    ),
                    ResidentProgramRequestPolicy::new(
                        ResidentOperationRequestKind::StartConsult,
                        overlap,
                    ),
                ],
                EvidencePort {
                    values: Arc::new(Mutex::new(VecDeque::new())),
                    reads: Arc::new(Mutex::new(0)),
                },
                Clock {
                    values: Arc::new(Mutex::new(VecDeque::new())),
                    reads: Arc::new(Mutex::new(0)),
                },
                &historical,
                ProvingAuthority,
                Signature(true),
            );
            assert!(matches!(
                overlapping,
                Err(ResidentProgramCurrentRequestBuildError::DuplicatePolicy)
            ));

            let mut distinct = invoke.clone();
            distinct.expected_body_schema_id =
                SchemaId::parse("pale/distinct-consult-request.v1").unwrap();
            distinct.expected_body_schema_sha256 = schema_digest('d');
            let distinct_domains = ResidentProgramCurrentRequestAdmission::<
                (),
                EvidencePort,
                Clock,
                ProvingAuthority,
                Signature,
            >::new(
                1 << 20,
                vec![
                    ResidentProgramRequestPolicy::new(
                        ResidentOperationRequestKind::StartInvoke,
                        invoke,
                    ),
                    ResidentProgramRequestPolicy::new(
                        ResidentOperationRequestKind::StartConsult,
                        distinct,
                    ),
                ],
                EvidencePort {
                    values: Arc::new(Mutex::new(VecDeque::new())),
                    reads: Arc::new(Mutex::new(0)),
                },
                Clock {
                    values: Arc::new(Mutex::new(VecDeque::new())),
                    reads: Arc::new(Mutex::new(0)),
                },
                &historical,
                ProvingAuthority,
                Signature(true),
            );
            assert!(distinct_domains.is_ok());
        },
    );
}

#[test]
fn unavailable_and_stale_current_evidence_deny_before_request_signature() {
    let now = UnixTimeV1::new(61).unwrap();
    with_admission(
        |_| vec![Err(())],
        vec![],
        true,
        ResidentOperationRequestKind::StartInvoke,
        |mut admission, process, authorities, session, original, exact, reads, clocks, _, _, _| {
            let request = resealed_request(original, exact, session, now, |_| {});
            assert!(matches!(
                admission.admit(&process, authorities, &(), &frame(&request)),
                Err(ResidentOperationFrameFailure::AdmissionDenied)
            ));
            assert_eq!(*reads.lock().unwrap(), 1);
            assert_eq!(*clocks.lock().unwrap(), 0);
        },
    );
    with_admission(
        |exact| {
            let mut stale = exact.clone();
            stale.session_revocations.epoch =
                PrincipalRevocationEpochV1::new(stale.session_revocations.epoch.get() + 1).unwrap();
            vec![Ok(stale)]
        },
        vec![now],
        true,
        ResidentOperationRequestKind::StartInvoke,
        |mut admission, process, authorities, session, original, exact, reads, clocks, _, _, _| {
            let request = resealed_request(original, exact, session, now, |_| {});
            assert!(matches!(
                admission.admit(&process, authorities, &(), &frame(&request)),
                Err(ResidentOperationFrameFailure::AdmissionDenied)
            ));
            assert_eq!(*reads.lock().unwrap(), 1);
            assert_eq!(*clocks.lock().unwrap(), 1);
        },
    );
}

#[test]
fn request_window_and_signature_failures_are_denied() {
    let now = UnixTimeV1::new(61).unwrap();
    with_admission(
        |exact| vec![Ok(exact.clone()), Ok(exact.clone())],
        vec![now, now],
        true,
        ResidentOperationRequestKind::StartInvoke,
        |mut admission, process, authorities, session, original, exact, _, _, _, _, _| {
            let future = resealed_request(original, exact, session, now, |value| {
                value.issued_at = UnixTimeV1::new(now.get() + 1).unwrap();
                value.expires_at = UnixTimeV1::new(now.get() + 2).unwrap();
            });
            assert!(matches!(
                admission.admit(&process, authorities, &(), &frame(&future)),
                Err(ResidentOperationFrameFailure::AdmissionDenied)
            ));
            let overlong = resealed_request(original, exact, session, now, |value| {
                value.expires_at = UnixTimeV1::new(now.get() + 13).unwrap();
            });
            assert!(matches!(
                admission.admit(&process, authorities, &(), &frame(&overlong)),
                Err(ResidentOperationFrameFailure::AdmissionDenied)
            ));
        },
    );
    with_admission(
        |exact| vec![Ok(exact.clone())],
        vec![now],
        false,
        ResidentOperationRequestKind::StartInvoke,
        |mut admission, process, authorities, session, original, exact, _, _, _, _, _| {
            let request = resealed_request(original, exact, session, now, |_| {});
            assert!(matches!(
                admission.admit(&process, authorities, &(), &frame(&request)),
                Err(ResidentOperationFrameFailure::AdmissionDenied)
            ));
        },
    );
}

#[test]
fn package_operation_and_generation_substitution_stop_before_current_evidence() {
    let now = UnixTimeV1::new(61).unwrap();
    with_admission(
        |_| vec![],
        vec![],
        true,
        ResidentOperationRequestKind::StartInvoke,
        |mut admission, process, authorities, session, original, exact, reads, clocks, _, _, _| {
            let package = resealed_request(original, exact, session, now, |value| {
                value.operation.package_id = PackageIdV1::parse("substituted-package").unwrap();
            });
            assert!(matches!(
                admission.admit(&process, authorities, &(), &frame(&package)),
                Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
            ));
            let operation = resealed_request(original, exact, session, now, |value| {
                value.operation.resource_id =
                    PackageResourceIdV1::parse("substituted-operation").unwrap();
            });
            assert!(matches!(
                admission.admit(&process, authorities, &(), &frame(&operation)),
                Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
            ));
            let generation = resealed_request(original, exact, session, now, |value| {
                value.generation = Some(
                    CompiledGenerationDigest::hash_canonical("substituted-generation").unwrap(),
                );
            });
            assert!(matches!(
                admission.admit(&process, authorities, &(), &frame(&generation)),
                Err(ResidentOperationFrameFailure::ConfigurationUnavailable)
            ));
            assert_eq!(*reads.lock().unwrap(), 0);
            assert_eq!(*clocks.lock().unwrap(), 0);
        },
    );
}

#[test]
fn every_turn_reads_fresh_current_evidence_and_physical_time() {
    let first = UnixTimeV1::new(61).unwrap();
    let second = UnixTimeV1::new(62).unwrap();
    with_admission(
        |exact| vec![Ok(exact.clone()), Ok(exact.clone())],
        vec![first, second],
        true,
        ResidentOperationRequestKind::StartInvoke,
        |mut admission, process, authorities, session, original, exact, reads, clocks, _, _, _| {
            for now in [first, second] {
                let request = resealed_request(original, exact, session, now, |_| {});
                assert!(admission
                    .admit(&process, authorities, &(), &frame(&request))
                    .is_ok());
            }
            assert_eq!(*reads.lock().unwrap(), 2);
            assert_eq!(*clocks.lock().unwrap(), 2);
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn with_consult_roster_program(
    evidence_values: impl FnOnce(
        &ResidentCurrentRequestEvidence,
    ) -> Vec<Result<ResidentCurrentRequestEvidence, ()>>,
    times: Vec<UnixTimeV1>,
    turns: usize,
    fail_handle_calls: impl IntoIterator<Item = usize>,
    build_policy: impl FnOnce(
        &crate::resident::LocalOperationStartInput<'_>,
        &CommonRequestEnvelopeV1,
        &CommonRequestEnvelopeV1,
        &ResidentCurrentRequestEvidence,
        &crate::resident::AdmittedAmbientSession,
    ) -> ConsultPolicy,
    inspect: impl FnOnce(
        &[ResidentOperationFrameResponse],
        usize,
        usize,
        &[OperationHandleIdV1],
        usize,
        usize,
        &[usize],
    ),
) {
    let now = UnixTimeV1::new(61).expect("Consult request time");
    with_admission(
        evidence_values,
        times,
        true,
        ResidentOperationRequestKind::StartConsult,
        |admission,
         process,
         authorities,
         session,
         original,
         exact,
         evidence_reads,
         clock_reads,
         semantic,
         checkpoint,
         _| {
            let request = resealed_request(original, exact, session, now, |_| {});
            let store = AuthorityStoreCoordinate::new(
                semantic.checkpoint_authority.control_store_id.clone(),
                semantic.checkpoint_authority.control_incarnation_id.clone(),
            );
            let lookup_reads = Arc::new(AtomicUsize::new(0));
            let mut custody = absent_start_store(&request)
                .at_store(&store)
                .with_index_read_counter(Arc::clone(&lookup_reads))
                .with_unlisted_indexes_absent()
                .with_lease_retentions([
                    Ok(OperationCustodyCommitOutcome::Committed),
                    Ok(OperationCustodyCommitOutcome::Committed),
                    Ok(OperationCustodyCommitOutcome::Committed),
                ])
                .with_preparation(
                    initial_response,
                    ControlTransactionSequenceV1::new(1).expect("initial transaction"),
                );
            let sessions = MemoryAuthorityStore::default();
            let mut anchor = initial_anchor();
            let selector = authorities[0].fixture_selector().clone();
            let handle_ids = Arc::new(Mutex::new(Vec::new()));
            let observation_signs = Arc::new(AtomicUsize::new(0));
            let entropy_calls = Arc::new(AtomicUsize::new(0));
            let owner = ResidentProgramOperationTurnOwner::new(
                OperationOnlyPeer,
                admission,
                vec![ResidentProgramDeploymentOperationPorts::new(
                    selector,
                    store,
                    &mut custody,
                    &sessions,
                    &mut anchor,
                )],
                ServiceEntropy {
                    calls: Arc::clone(&entropy_calls),
                },
                CountingAuthority::default(),
                ConsultRosterCryptography {
                    checkpoints: RefCell::new(
                        std::iter::repeat_n(checkpoint, turns.saturating_add(1)).collect(),
                    ),
                    fail_handle_calls: fail_handle_calls.into_iter().collect(),
                    handle_ids: Arc::clone(&handle_ids),
                    observation_signs: Arc::clone(&observation_signs),
                },
                RootRecovery { semantic },
                ResidentProgramConsultRecoveryUnavailable,
            )
            .expect("one exact retained Consult owner");
            let policy = build_policy(semantic, &request, original, exact, session);
            let mut program = ResidentProgram::fixture_from_process_with_authorities(
                process,
                authorities.to_vec(),
                policy,
                owner,
            );
            let frame = ResidentOperationFrameRequest::StartConsult {
                signed_common_request: pale_core::canon::to_canonical_string(&request)
                    .expect("canonical Consult request"),
            };
            let deadline = std::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            let mut responses = Vec::with_capacity(turns);
            let mut turn_handle_counts = Vec::with_capacity(turns);
            for _ in 0..turns {
                let mut transport = OperationFramePort::new(&frame);
                let response = program
                    .serve_operation_frame(&mut transport, deadline, deadline)
                    .expect("Consult turn returns one bounded response");
                let decoded: ResidentOperationFrameResponse =
                    serde_json::from_slice(&transport.sent).expect("one response decodes");
                assert_eq!(decoded, response);
                responses.push(response);
                turn_handle_counts.push(handle_ids.lock().expect("handle identity log").len());
            }
            let handles = handle_ids.lock().expect("handle identity log").clone();
            inspect(
                &responses,
                *evidence_reads.lock().expect("evidence reads"),
                *clock_reads.lock().expect("clock reads"),
                &handles,
                observation_signs.load(Ordering::SeqCst),
                entropy_calls.load(Ordering::SeqCst),
                &turn_handle_counts,
            );
        },
    );
}

#[test]
fn consult_program_materializes_mixed_evidence_and_fresh_roster_in_order() {
    let now = UnixTimeV1::new(61).expect("Consult request time");
    with_consult_roster_program(
        |exact| vec![Ok(exact.clone()), Ok(exact.clone())],
        vec![now, now],
        1,
        [],
        |semantic, parent, original, exact, session| {
            let evidence = consult_immutable("consult/evidence/already-resolved");
            let child = resealed_request(original, exact, session, now, |value| {
                value.request_id =
                    ResidentRequestIdV1::parse("consult-fresh-child").expect("child request id");
            });
            consult_policy_with_slots(
                semantic,
                now,
                parent.expires_at(),
                consult_immutable("consult/policy/mixed"),
                vec![
                    consult_roster_slot(
                        "resolved",
                        crate::resident::consultation::ConsultationRosterTemplateSource::EvidenceResolved {
                            evidence,
                        },
                    ),
                    consult_roster_slot(
                        "fresh",
                        crate::resident::consultation::ConsultationRosterTemplateSource::FreshChild {
                            child: fresh_consult_child_policy(
                                semantic,
                                child,
                                OperationClassV1::Consult,
                            ),
                        },
                    ),
                ],
            )
        },
        |responses,
         evidence_reads,
         clock_reads,
         handles,
         observation_signs,
         entropy_calls,
         _turn_handle_counts| {
            assert_eq!(
                responses,
                [ResidentOperationFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::ResidentUnavailable,
                }]
            );
            assert_eq!(evidence_reads, 2, "parent and fresh child admit once");
            assert_eq!(clock_reads, 2, "each admission reads physical time");
            assert_eq!(handles.len(), 1, "only the parent is signed in preparation");
            assert_eq!(
                entropy_calls, 14,
                "parent and child identities allocate once"
            );
            assert_eq!(observation_signs, 1, "the complete roster signs once");
        },
    );
}

#[test]
fn consult_program_rejects_unsupported_fresh_child_class_without_allocating_it() {
    let now = UnixTimeV1::new(61).expect("Consult request time");
    with_consult_roster_program(
        |exact| vec![Ok(exact.clone()), Ok(exact.clone())],
        vec![now, now],
        2,
        [],
        |semantic, parent, original, exact, session| {
            let child = resealed_request(original, exact, session, now, |value| {
                value.request_id = ResidentRequestIdV1::parse("unsupported-consult-child")
                    .expect("child request id");
            });
            consult_policy_with_slots(
                semantic,
                now,
                parent.expires_at(),
                consult_immutable("consult/policy/unsupported"),
                vec![consult_roster_slot(
                    "unsupported",
                    crate::resident::consultation::ConsultationRosterTemplateSource::FreshChild {
                        child: fresh_consult_child_policy(
                            semantic,
                            child,
                            OperationClassV1::Delegate,
                        ),
                    },
                )],
            )
        },
        |responses,
         evidence_reads,
         clock_reads,
         handles,
         observation_signs,
         _,
         _turn_handle_counts| {
            assert!(responses.iter().all(|response| {
                response
                    == &ResidentOperationFrameResponse::Rejected {
                        failure: ResidentOperationFrameFailure::ResidentUnavailable,
                    }
            }));
            assert_eq!(evidence_reads, 2, "only the two parent turns admit");
            assert_eq!(clock_reads, 2);
            assert_eq!(handles.len(), 1, "the unsupported child has no identity");
            assert_eq!(observation_signs, 0, "an incomplete roster is unsigned");
        },
    );
}

#[test]
fn consult_program_retry_preserves_the_allocated_child_prefix() {
    let now = UnixTimeV1::new(61).expect("Consult request time");
    with_consult_roster_program(
        |exact| {
            vec![
                Ok(exact.clone()),
                Ok(exact.clone()),
                Err(()),
                Ok(exact.clone()),
                Ok(exact.clone()),
            ]
        },
        vec![now, now, now, now, now],
        2,
        [],
        |semantic, parent, original, exact, session| {
            let first = resealed_request(original, exact, session, now, |value| {
                value.request_id =
                    ResidentRequestIdV1::parse("consult-child-first").expect("first request id");
            });
            let second = resealed_request(original, exact, session, now, |value| {
                value.request_id =
                    ResidentRequestIdV1::parse("consult-child-second").expect("second request id");
            });
            consult_policy_with_slots(
                semantic,
                now,
                parent.expires_at(),
                consult_immutable("consult/policy/retry"),
                vec![
                    consult_roster_slot(
                        "first",
                        crate::resident::consultation::ConsultationRosterTemplateSource::FreshChild {
                            child: fresh_consult_child_policy(
                                semantic,
                                first,
                                OperationClassV1::Consult,
                            ),
                        },
                    ),
                    consult_roster_slot(
                        "second",
                        crate::resident::consultation::ConsultationRosterTemplateSource::FreshChild {
                            child: fresh_consult_child_policy(
                                semantic,
                                second,
                                OperationClassV1::Consult,
                            ),
                        },
                    ),
                ],
            )
        },
        |responses,
         evidence_reads,
         clock_reads,
         handles,
         observation_signs,
         entropy_calls,
         turn_handle_counts| {
            assert!(responses.iter().all(|response| {
                response
                    == &ResidentOperationFrameResponse::Rejected {
                        failure: ResidentOperationFrameFailure::ResidentUnavailable,
                    }
            }));
            assert_eq!(evidence_reads, 5);
            assert_eq!(clock_reads, 4, "failed evidence stops before physical time");
            assert_eq!(handles.len(), 1, "only the parent is signed in preparation");
            assert_eq!(
                entropy_calls, 21,
                "retry reuses both allocated child identities instead of drawing another"
            );
            assert_eq!(observation_signs, 1, "retry completes one roster");
            assert_eq!(
                turn_handle_counts,
                [1, 1],
                "retry keeps the issued parent and allocated child prefix"
            );
        },
    );
}

#[test]
fn consult_program_never_admits_a_child_before_parent_handle_signing() {
    let now = UnixTimeV1::new(61).expect("Consult request time");
    with_consult_roster_program(
        |exact| vec![Ok(exact.clone())],
        vec![now],
        1,
        [1],
        |semantic, parent, original, exact, session| {
            let child = resealed_request(original, exact, session, now, |value| {
                value.request_id =
                    ResidentRequestIdV1::parse("blocked-consult-child").expect("child request id");
            });
            consult_policy_with_slots(
                semantic,
                now,
                parent.expires_at(),
                consult_immutable("consult/policy/parent-signing"),
                vec![consult_roster_slot(
                    "blocked-child",
                    crate::resident::consultation::ConsultationRosterTemplateSource::FreshChild {
                        child: fresh_consult_child_policy(
                            semantic,
                            child,
                            OperationClassV1::Consult,
                        ),
                    },
                )],
            )
        },
        |responses,
         evidence_reads,
         clock_reads,
         handles,
         observation_signs,
         _,
         _turn_handle_counts| {
            assert_eq!(responses.len(), 1);
            assert!(!matches!(
                responses[0],
                ResidentOperationFrameResponse::Started { .. }
            ));
            assert_eq!(evidence_reads, 1, "only the parent request is admitted");
            assert_eq!(clock_reads, 1);
            assert_eq!(handles.len(), 1, "only parent signing is attempted");
            assert_eq!(observation_signs, 0);
        },
    );
}

fn retained_consult_key(name: &str) -> ResidentProgramConsultTurnKey {
    ResidentProgramConsultTurnKey::new(
        ResidentDeploymentAuthoritySelector {
            principal_id: ResidentPrincipalIdV1::parse("consult-retention-principal")
                .expect("retention principal"),
            deployment_binding_ref: ResidentRecordRefV1::parse("consult/deployment-binding")
                .expect("deployment binding reference"),
            deployment_binding_sha256: DeploymentBindingDigest::parse("1".repeat(64))
                .expect("deployment binding digest"),
            runtime_authority_ref: ResidentRecordRefV1::parse("consult/runtime-authority")
                .expect("runtime authority reference"),
            runtime_authority_sha256: DeploymentRuntimeAuthorityDigest::parse("2".repeat(64))
                .expect("runtime authority digest"),
        },
        ControlRecordCoordinate::new(
            ResidentRecordRefV1::parse(format!("consult/{name}"))
                .expect("retained Consult reference"),
            pale_ports::ControlRecordDigest::parse("3".repeat(64))
                .expect("retained Consult digest"),
        ),
    )
}

#[test]
fn consult_program_retained_turns_are_bounded_keyed_and_fifo() {
    let mut retained = ResidentProgramRetainedConsultTurns::new(
        std::num::NonZeroUsize::new(2).expect("two retained Consult turns"),
    );
    let first = retained_consult_key("first");
    let second = retained_consult_key("second");
    let third = retained_consult_key("third");

    assert!(retained.retain(first.clone(), 11_u8).is_ok());
    let duplicate = retained
        .retain(first.clone(), 12_u8)
        .expect_err("one parent cannot own two retained turns")
        .into_retained();
    assert_eq!(duplicate.into_parts(), (first.clone(), 12));
    assert!(retained.retain(second.clone(), 21_u8).is_ok());
    assert!(retained.contains(&first));
    assert!(retained.contains(&second));
    assert!(!retained.contains(&third));
    let saturated = retained
        .retain(third.clone(), 31_u8)
        .expect_err("the explicit bound rejects another parent")
        .into_retained();
    assert_eq!(saturated.into_parts(), (third, 31));

    assert!(retained.has_retained());
    assert_eq!(retained.next_selector(), Some(first.selector()));
    assert_eq!(
        retained
            .take_next()
            .expect("first turn remains first")
            .into_parts(),
        (first, 11)
    );
    assert!(!retained.contains(&retained_consult_key("first")));
    assert_eq!(
        retained
            .take_next()
            .expect("second turn remains second")
            .into_parts(),
        (second, 21)
    );
    assert!(!retained.has_retained());
}

#[test]
fn consult_program_retained_stop_fails_closed_without_semantic_input() {
    assert!(().stop_meaning().is_none());
}

#[test]
fn matching_frame_reenters_the_same_retained_start_phase_without_resigning() {
    let now = UnixTimeV1::new(61).unwrap();
    with_admission(
        |exact| vec![Ok(exact.clone()), Ok(exact.clone())],
        vec![now, now],
        true,
        ResidentOperationRequestKind::StartConsult,
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
            let store = AuthorityStoreCoordinate::new(
                semantic.checkpoint_authority.control_store_id.clone(),
                semantic.checkpoint_authority.control_incarnation_id.clone(),
            );
            let physical_reads = Arc::new(AtomicUsize::new(0));
            let mut custody = absent_start_store(&request)
                .at_store(&store)
                .with_index_read_counter(Arc::clone(&physical_reads))
                .with_unlisted_indexes_absent()
                .with_lease_retention(Ok(OperationCustodyCommitOutcome::Committed))
                .with_preparation(
                    initial_response,
                    ControlTransactionSequenceV1::new(1).expect("initial transaction"),
                );
            let sessions = MemoryAuthorityStore::default();
            let mut anchor = initial_anchor();
            let selector = authorities[0].fixture_selector().clone();
            let handle_signs = Arc::new(AtomicUsize::new(0));
            let observation_signs = Arc::new(AtomicUsize::new(0));
            let checkpoint_preparations = Arc::new(AtomicUsize::new(0));
            let checkpoint_signs = Arc::new(AtomicUsize::new(0));
            let owner = ResidentProgramOperationTurnOwner::new(
                OperationOnlyPeer,
                admission,
                vec![ResidentProgramDeploymentOperationPorts::new(
                    selector,
                    store,
                    &mut custody,
                    &sessions,
                    &mut anchor,
                )],
                FreshEntropy::new(Arc::clone(&physical_reads)),
                CountingAuthority::default(),
                FailingCheckpointCryptography {
                    checkpoint: RefCell::new(Some(checkpoint)),
                    handle_signs: Arc::clone(&handle_signs),
                    observation_signs: Arc::clone(&observation_signs),
                    checkpoint_preparations: Arc::clone(&checkpoint_preparations),
                    checkpoint_signs: Arc::clone(&checkpoint_signs),
                },
                RootRecovery { semantic },
                ResidentProgramConsultRecoveryUnavailable,
            )
            .expect("one exact retained start owner");
            let mut program = ResidentProgram::fixture_from_process_with_authorities(
                process,
                authorities.to_vec(),
                consult_policy(semantic, now, request.expires_at()),
                owner,
            );
            let frame = ResidentOperationFrameRequest::StartConsult {
                signed_common_request: pale_core::canon::to_canonical_string(&request)
                    .expect("canonical request"),
            };
            let deadline = std::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            for turn in 1..=2 {
                let mut transport = OperationFramePort::new(&frame);
                let response = program
                    .serve_operation_frame(&mut transport, deadline, deadline)
                    .expect("the retained start returns one bounded response");
                assert_eq!(
                    response,
                    ResidentOperationFrameResponse::Rejected {
                        failure: ResidentOperationFrameFailure::ResidentUnavailable,
                    },
                    "turn {turn} must preserve the unfinished phase"
                );
                let decoded: ResidentOperationFrameResponse =
                    serde_json::from_slice(&transport.sent).expect("one response decodes");
                assert_eq!(decoded, response);
            }
            assert_eq!(handle_signs.load(Ordering::SeqCst), 1);
            assert_eq!(observation_signs.load(Ordering::SeqCst), 1);
            assert_eq!(checkpoint_preparations.load(Ordering::SeqCst), 1);
            assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 1);
        },
    );
}

#[test]
fn framed_start_consult_hands_the_prepared_start_to_the_retained_phase_scheduler() {
    let now = UnixTimeV1::new(61).unwrap();
    with_admission(
        |exact| vec![Ok(exact.clone())],
        vec![now],
        true,
        ResidentOperationRequestKind::StartConsult,
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
            let store = AuthorityStoreCoordinate::new(
                semantic.checkpoint_authority.control_store_id.clone(),
                semantic.checkpoint_authority.control_incarnation_id.clone(),
            );
            let reads = Arc::new(AtomicUsize::new(0));
            let mut custody = absent_start_store(&request)
                .at_store(&store)
                .with_index_read_counter(Arc::clone(&reads))
                .with_unlisted_indexes_absent()
                .with_lease_retention(Ok(OperationCustodyCommitOutcome::Committed))
                .with_preparation(
                    initial_response,
                    ControlTransactionSequenceV1::new(1).expect("initial transaction"),
                );
            let sessions = MemoryAuthorityStore::default();
            let mut anchor = initial_anchor();
            let selector = authorities[0].fixture_selector().clone();
            let owner = ResidentProgramOperationTurnOwner::new(
                OperationOnlyPeer,
                admission,
                vec![ResidentProgramDeploymentOperationPorts::new(
                    selector,
                    store.clone(),
                    &mut custody,
                    &sessions,
                    &mut anchor,
                )],
                FreshEntropy::new(Arc::clone(&reads)),
                CountingAuthority::default(),
                StartCryptography {
                    checkpoint: RefCell::new(Some(checkpoint)),
                },
                RootRecovery { semantic },
                ResidentProgramConsultRecoveryUnavailable,
            )
            .expect("one exact retained start owner");
            let policy = consult_policy(semantic, now, request.expires_at());
            let mut program = ResidentProgram::fixture_from_process_with_authorities(
                process,
                authorities.to_vec(),
                policy,
                owner,
            );
            let frame = ResidentOperationFrameRequest::StartConsult {
                signed_common_request: pale_core::canon::to_canonical_string(&request)
                    .expect("canonical request"),
            };
            let mut port = OperationFramePort::new(&frame);
            let deadline = std::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            let ResidentProgramFrameOutcome::Operation = program
                .serve_resident_frame(&mut port, deadline, deadline)
                .expect("Consult reaches one durable framed response")
            else {
                panic!("Consult request reached the Observe owner")
            };
            let response: ResidentOperationFrameResponse =
                serde_json::from_slice(&port.sent).expect("one response decodes");
            assert_eq!(
                response,
                ResidentOperationFrameResponse::Rejected {
                    failure: ResidentOperationFrameFailure::ResidentUnavailable,
                },
                "the first frame cannot claim a Started result before retained phases run"
            );
            assert!(
                program.has_retained_phase(),
                "the prepared Consult start remains owned by the production scheduler"
            );
            assert_eq!(
                program.continue_one_retained_phase(),
                ResidentProgramRetainedPhaseTurn::Advanced,
                "one scheduler turn advances only the prepared-to-parent phase"
            );
            assert!(
                program.has_retained_phase(),
                "the next exact Consult start phase remains retained"
            );
            assert_eq!(reads.load(Ordering::SeqCst), 4);
        },
    );
}

#[test]
fn framed_advance_invoke_recovers_the_exact_locator_before_phase_handoff() {
    let now = UnixTimeV1::new(61).expect("advance request time");
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(2);
    let retained = generations.pop().expect("retained host generation");
    let semantic = generations.pop().expect("Invoke fixture generation");
    with_issued_context(
        &semantic,
        &profile,
        "program-advance-invoke",
        |_| {},
        crate::session_admission_fixture::LocalStartContext::Inherited,
        |issued, original_request, parents, initial_checkpoint, _, candidate| {
            let store = crate::local_operation_start_fixture::store_coordinate();
            let session_store = MemoryAuthorityStore::default();
            let mut application = ready_application(&session_store, store.clone(), 103);
            application.fixture_retain_generation(retained);
            let (published, advertised) = active_predecessors(semantic.record());
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
            let (session, _) = application
                .admit_session(
                    SessionAdmissionLeaseWireV1::from(parents.session.lease()),
                    parents.current,
                    &mut ProvingAuthority,
                )
                .expect("admit Invoke session");
            let authority =
                ResidentProgramDeploymentAuthority::fixture_from_current(parents.current)
                    .expect("complete deployment authority");
            let selector = authority.fixture_selector().clone();
            let authorities = vec![authority];
            let process = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
                vec![unproved_deployment_for_selector(
                    &selector,
                    store.clone(),
                    application,
                )],
                std::num::NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one exact Invoke deployment");

            let mut records = RecoveryResolver::default();
            records.insert_generation(semantic.record());
            records.insert_effective_profile(&profile);
            records.insert_published(&published);
            records.insert_advertised(&advertised);
            records.insert_fence(parents.current.generation.activation_fence);
            records.insert_active(parents.current.generation.active_head);
            let historical = Historical {
                records,
                session_signing_authority: parents.signing.clone(),
            };
            process.deployments[0]
                .application
                .recover_exact_retained_session(
                    session.lease().session_admission_id(),
                    session.lease().session_admission_lease_digest(),
                    &historical,
                )
                .expect("recover exact retained Invoke session");

            let evidence = evidence_from(parents.current);
            let request = resealed_operation_target_request(
                original_request,
                &evidence,
                &session,
                now,
                operation_target_reference(
                    issued.handle().0,
                    issued.handle().1.canonical_operation_handle_sha256(),
                ),
                |_| {},
            );
            let expected_locator = operation_frame_locator(issued);
            let frame = ResidentOperationFrameRequest::AdvanceInvoke {
                locator: expected_locator.clone(),
                signed_common_request: pale_core::canon::to_canonical_string(&request)
                    .expect("canonical AdvanceInvoke request"),
            };
            let admission = ResidentProgramCurrentRequestAdmission::new(
                1 << 20,
                vec![ResidentProgramRequestPolicy::new(
                    ResidentOperationRequestKind::AdvanceInvoke,
                    policy_from(
                        original_request,
                        session.lease().compiled_generation_digest().clone(),
                    ),
                )],
                EvidencePort {
                    values: Arc::new(Mutex::new(vec![Ok(evidence)].into())),
                    reads: Arc::new(Mutex::new(0)),
                },
                Clock {
                    values: Arc::new(Mutex::new(vec![now].into())),
                    reads: Arc::new(Mutex::new(0)),
                },
                &historical,
                ProvingAuthority,
                Signature(true),
            )
            .expect("one exact AdvanceInvoke policy");

            let operation_lease = issued.historical_operation_lease();
            let lease_coordinate = crate::resident::record_projection::coordinate(
                issued.handle().1.operation_lease_ref(),
                operation_lease.operation_lease_digest().as_str(),
            )
            .expect("operation lease coordinate");
            let lease_record =
                crate::resident::record_projection::record(lease_coordinate, operation_lease)
                    .expect("operation lease record");
            let mut custody =
                crate::resident::operation_observation_custody::tests::RecordingStore::committing(
                    OperationCustodyCommitOutcome::Committed,
                )
                .at_store(&store)
                .with_unlisted_indexes_absent()
                .with_committed_candidate(candidate.clone())
                .with_retained_lease_record(lease_record);
            let current = candidate.checkpoint().successor();
            let mut anchor = RecoveryThenWriteAnchor {
                recovered: Some(pale_ports::AntiRollbackAnchor::ControlDomain(
                    pale_ports::ControlDomainAntiRollbackAnchor::new(
                        store.store_id().clone(),
                        store.incarnation_id().clone(),
                        current.checkpoint_domain_id().clone(),
                        current.domain_epoch(),
                        current.domain_sequence(),
                        current.coordinate().clone(),
                    ),
                )),
                write: successor_anchor(),
            };
            let owner = ResidentProgramOperationTurnOwner::new(
                AcceptPeer,
                admission,
                vec![ResidentProgramDeploymentOperationPorts::new(
                    selector,
                    store.clone(),
                    &mut custody,
                    &session_store,
                    &mut anchor,
                )],
                FreshEntropy::new(Arc::new(AtomicUsize::new(0))),
                CountingAuthority::default(),
                StartCryptography {
                    checkpoint: RefCell::new(None),
                },
                ParentRecovery {
                    store,
                    parent_request: original_request,
                    parent_checkpoint: initial_checkpoint,
                    historical: &historical,
                },
                ResidentProgramConsultRecoveryUnavailable,
            )
            .expect("one exact retained AdvanceInvoke owner");
            let program = ResidentProgram::fixture_from_process_with_authorities(
                process,
                authorities,
                ConsultPolicy(None),
                owner,
            );
            let mut service = crate::resident::ResidentFrameService::from_program(program);
            let mut transport = OperationFramePort::new(&frame);
            let deadline = std::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            assert_eq!(
                service
                    .serve_frame(&mut transport, deadline, deadline)
                    .expect("AdvanceInvoke reaches one framed response"),
                crate::resident::ResidentFrameOutcome::Operation,
            );
            let response: ResidentOperationFrameResponse =
                serde_json::from_slice(&transport.sent).expect("AdvanceInvoke response decodes");
            assert_eq!(
                response,
                ResidentOperationFrameResponse::Pending {
                    locator: expected_locator.clone(),
                }
            );
            assert_or_update_application_invoke_advance_fixture(&frame, &response);
        },
    );
}

#[test]
fn cold_program_recovery_rediscovers_the_same_operation_without_replay() {
    let mut generation_inputs =
        crate::local_operation_start_fixture::source_generation_input_copies_with(
            |_| {},
            |_| {},
            4,
        );
    let (semantic_compilation, semantic_materializations, profile) =
        generation_inputs.pop().expect("semantic generation input");
    let semantic = crate::resident::AdmittedCompiledGeneration::admit(
        semantic_compilation,
        semantic_materializations,
        synthetic_set_policy(),
    )
    .expect("semantic generation admits");
    let (application_compilation, application_materializations, _) = generation_inputs
        .pop()
        .expect("application generation input");
    let application_generation = crate::resident::AdmittedCompiledGeneration::admit(
        application_compilation,
        application_materializations,
        synthetic_set_policy(),
    )
    .expect("application generation admits");

    with_evidence(
        StoredChange::None,
        |finalization_snapshot, finalization, _, _, _, _, _| {
            let trust = recovered_cold_start_trust_frontier(finalization_snapshot, finalization);
            let current_trust = trust.current_checkpoint();
            let runtime = &trust.runtime_authorities()[0];
            let store = AuthorityStoreCoordinate::new(
                runtime.authority().control_store_id().clone(),
                runtime.authority().control_incarnation_id().clone(),
            );
            crate::resident::operation_observation_custody::tests::local_start::
                with_issued_context_at_store_and_authorities(
                    &semantic,
                    &profile,
                    "program-cold-joined",
                    &store,
                    current_trust.coordinate().record_ref(),
                    current_trust.checkpoint(),
                    runtime.binding_ref(),
                    runtime.binding(),
                    runtime.record_ref(),
                    runtime.authority(),
                    |issued, request, parents, initial_checkpoint, _, candidate| {
                        let published = crate::resident_fixture::published(semantic.record(), 1, None);
                        let advertised = crate::resident_fixture::advertised(&published, 2, None);
                        assert_eq!(
                            advertised.advertised_generation_head_digest(),
                            parents
                                .current
                                .generation
                                .active_head
                                .advertised_generation_head_digest()
                        );
                        let authority_store = MemoryAuthorityStore::default();
                        let session_store = ExactSessionStore::new(store.clone());
                        {
                            let resolver = RecoveryResolver::default();
                            let mut source = FixedInputSource::default();
                            let opened = ResidentApplication::cold_open(
                                &authority_store,
                                &session_store,
                                &mut UnavailableEventCustody,
                                limits().maximum_entry_bytes(),
                                ResidentColdOpenInput::new(
                                    store.clone(),
                                    limits(),
                                    limits().maximum_entry_bytes(),
                                    RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                                    &resolver,
                                    &mut source,
                                    ResidentReconstructionParameters::new(
                                        synthetic_set_policy(),
                                        4,
                                        4,
                                        WakeEpochV1::new(201),
                                    ),
                                ),
                            )
                            .expect("empty resident cold-open");
                            let ResidentColdOpenOutcome::Ready(mut application) = opened else {
                                panic!("empty resident must be ready")
                            };
                            application.fixture_retain_generation(application_generation);
                            let _ = application
                                .publish(published.clone())
                                .expect("publish retained generation");
                            let _ = application
                                .advertise(advertised.clone())
                                .expect("advertise retained generation");
                            let _ = application
                                .activate(
                                    parents.current.generation.active_head.clone(),
                                    parents.current.generation.activation_fence.clone(),
                                )
                                .expect("activate retained generation");
                            let (session, _) = application
                                .admit_session(
                                    SessionAdmissionLeaseWireV1::from(parents.session.lease()),
                                    parents.current,
                                    &mut ProvingAuthority,
                                )
                                .expect("retain exact ambient session");
                            assert_eq!(session.lease(), parents.session.lease());
                            application
                                .checkpoint()
                                .expect("checkpoint generation and session");
                        }

                        let mut records = RecoveryResolver::default();
                        records.insert_generation(semantic.record());
                        records.insert_effective_profile(&profile);
                        records.insert_published(&published);
                        records.insert_advertised(&advertised);
                        records.insert_fence(parents.current.generation.activation_fence);
                        records.insert_active(parents.current.generation.active_head);
                        let historical = Historical {
                            records,
                            session_signing_authority: parents.signing.clone(),
                        };
                        let authority =
                            ResidentProgramDeploymentAuthority::fixture_from_current(
                                parents.current,
                            )
                            .expect("complete deployment authority");
                        let selector = authority.fixture_selector().clone();
                        let expected_locator = operation_frame_locator(issued);
                        let exact_evidence = evidence_from(parents.current);
                        let advance_request = resealed_operation_target_request(
                            request,
                            &exact_evidence,
                            parents.session,
                            request.issued_at(),
                            operation_target_reference(
                                issued.handle().0,
                                issued.handle().1.canonical_operation_handle_sha256(),
                            ),
                            |_| {},
                        );
                        let frame = ResidentOperationFrameRequest::AdvanceInvoke {
                            locator: expected_locator.clone(),
                            signed_common_request: pale_core::canon::to_canonical_string(
                                &advance_request,
                            )
                            .expect("canonical retained request"),
                        };

                        let lease = issued.historical_operation_lease();
                        let lease_coordinate = crate::resident::record_projection::coordinate(
                            issued.handle().1.operation_lease_ref(),
                            lease.operation_lease_digest().as_str(),
                        )
                        .expect("operation lease coordinate");
                        let lease_record =
                            crate::resident::record_projection::record(lease_coordinate, lease)
                                .expect("operation lease record");
                        let mut custody = crate::resident::operation_observation_custody::tests::
                            RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
                            .at_store(&store)
                            .with_unlisted_indexes_absent()
                            .with_committed_candidate(candidate.clone())
                            .with_retained_lease_record(lease_record);
                        let current_head = candidate.checkpoint().successor().coordinate().clone();

                        for cold_open in 0..2 {
                            let (compilation, materializations, _) = generation_inputs
                                .pop()
                                .expect("one immutable reconstruction input per cold open");
                            let mut source = FixedInputSource {
                                calls: 0,
                                inputs: vec![
                                    crate::resident::ResidentGenerationReconstructionInput::new(
                                        semantic.record().compiled_generation_digest().clone(),
                                        compilation,
                                        materializations,
                                    ),
                                ],
                                requested: None,
                            };
                            let writer = crate::local_operation_start_fixture::publication_writer();
                            let checkpoint = super::program::checkpoint_for_writer(&writer, 'c');
                            let snapshot = super::program::writer_snapshot(&writer, &checkpoint);
                            let mut writer_admission =
                                move |_: &pale_ports::PrincipalResidentWriterSnapshot| {
                                    Ok(AdmittedCurrentPrincipalWriter {
                                        writer: writer.clone(),
                                        checkpoint: checkpoint.clone(),
                                    })
                                };
                            let mut event_custody = UnavailableEventCustody;
                            let open = ResidentProgramDeploymentOpen {
                                durable: &authority_store,
                                sessions: &session_store,
                                events: &mut event_custody,
                                maximum_event_custody_bytes: 65_536,
                                input: ResidentColdOpenInput::new(
                                    store.clone(),
                                    limits(),
                                    limits().maximum_entry_bytes(),
                                    RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                                    &historical,
                                    &mut source,
                                    ResidentReconstructionParameters::new(
                                        synthetic_set_policy(),
                                        4,
                                        4,
                                        WakeEpochV1::new(202 + cold_open),
                                    ),
                                ),
                            };
                            let mut retained_policy = policy_from(
                                &advance_request,
                                parents
                                    .session
                                    .lease()
                                    .compiled_generation_digest()
                                    .clone(),
                            );
                            retained_policy.maximum_request_lifetime_seconds = 40;
                            let admission = ResidentProgramCurrentRequestAdmission::new(
                                1 << 20,
                                vec![ResidentProgramRequestPolicy::new(
                                    ResidentOperationRequestKind::AdvanceInvoke,
                                    retained_policy,
                                )],
                                EvidencePort {
                                    values: Arc::new(Mutex::new(VecDeque::from([Ok(
                                        exact_evidence.clone(),
                                    )]))),
                                    reads: Arc::new(Mutex::new(0)),
                                },
                                Clock {
                                    values: Arc::new(Mutex::new(VecDeque::from([
                                        advance_request.issued_at(),
                                    ]))),
                                    reads: Arc::new(Mutex::new(0)),
                                },
                                &historical,
                                ProvingAuthority,
                                Signature(true),
                            )
                            .expect("one exact retained AdvanceInvoke policy");
                            let current = candidate.checkpoint().successor();
                            assert_eq!(current.coordinate(), &current_head);
                            let mut anchor = RecoveryThenWriteAnchor {
                                recovered: Some(pale_ports::AntiRollbackAnchor::ControlDomain(
                                    pale_ports::ControlDomainAntiRollbackAnchor::new(
                                        store.store_id().clone(),
                                        store.incarnation_id().clone(),
                                        current.checkpoint_domain_id().clone(),
                                        current.domain_epoch(),
                                        current.domain_sequence(),
                                        current.coordinate().clone(),
                                    ),
                                )),
                                write: successor_anchor(),
                            };
                            let owner = ResidentProgramOperationTurnOwner::new(
                                AcceptPeer,
                                admission,
                                vec![ResidentProgramDeploymentOperationPorts::new(
                                    selector.clone(),
                                    store.clone(),
                                    &mut custody,
                                    &session_store,
                                    &mut anchor,
                                )],
                                super::program::RejectProgramEntropy,
                                CountingAuthority::default(),
                                RejectCryptography,
                                ParentRecovery {
                                    store: store.clone(),
                                    parent_request: request,
                                    parent_checkpoint: initial_checkpoint,
                                    historical: &historical,
                                },
                                ResidentProgramConsultRecoveryUnavailable,
                            )
                            .expect("one retained recovery owner");
                            let mut program = ResidentProgram::recover(
                                super::program::HeldWriter(
                                    snapshot.binding().principal_id().clone(),
                                ),
                                &snapshot,
                                &mut writer_admission,
                                &trust,
                                vec![open],
                                std::num::NonZeroUsize::new(1).expect("one deployment"),
                                task_state(),
                                ConsultPolicy(None),
                                owner,
                            )
                            .expect("cold resident program recovers exact deployment");
                            let mut transport = OperationFramePort::new(&frame);
                            let deadline = std::time::Instant::now()
                                .checked_add(std::time::Duration::from_secs(2))
                                .expect("test deadline");
                            let response = program
                                .serve_operation_frame(&mut transport, deadline, deadline)
                                .expect("cold recovery reaches the retained operation");
                            assert_eq!(
                                response,
                                ResidentOperationFrameResponse::Pending {
                                    locator: expected_locator.clone(),
                                }
                            );
                            assert_eq!(source.calls, 1);
                            drop(program);
                        }
                    },
                );
        },
    );
}

#[test]
fn framed_advance_consult_retains_its_policy_turn_after_a_post_callback_lender_error() {
    with_process_host_root_consult_policy_fixture(&mut |fixture| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            session,
            current,
            request: original,
            generation,
            evidence: start_evidence,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let store = recovery.operation.context.store.clone();
        let locator = ResidentOperationFrameLocator::new(
            recovery
                .operation
                .expected_handle
                .record_ref()
                .as_str()
                .to_owned(),
            recovery
                .operation
                .expected_handle
                .record_sha256()
                .as_str()
                .to_owned(),
        );
        let authority = ResidentProgramDeploymentAuthority::fixture_from_current(current)
            .expect("the Consult fixture retains complete deployment authority");
        let selector = authority.fixture_selector().clone();
        assert_eq!(selector, selector_from_signing_authority(signing_authority));

        let application_store = MemoryAuthorityStore::default();
        let mut application = ready_application(&application_store, store.clone(), 71);
        application.fixture_retain_generation(generation);
        let (published, advertised) = active_predecessors(current.generation.compiled_generation);
        assert_eq!(
            advertised.advertised_generation_head_digest(),
            current
                .generation
                .active_head
                .advertised_generation_head_digest()
        );
        let _ = application
            .publish(published.clone())
            .expect("the Consult generation publishes");
        let _ = application
            .advertise(advertised.clone())
            .expect("the Consult generation advertises");
        let _ = application
            .activate(
                current.generation.active_head.clone(),
                current.generation.activation_fence.clone(),
            )
            .expect("the Consult generation activates");
        let (retained_session, _) = application
            .admit_session(
                SessionAdmissionLeaseWireV1::from(session.lease()),
                current,
                &mut ProvingAuthority,
            )
            .expect("the exact Consult session admits into the selected deployment");
        let process = ResidentProcessHost::new(
            unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
            vec![unproved_deployment_for_selector(
                &selector,
                store.clone(),
                application,
            )],
            std::num::NonZeroUsize::new(1).expect("one deployment"),
            ResidentOperationHost::new(task_state()),
        )
        .expect("one exact Consult deployment");
        let mut records = RecoveryResolver::default();
        records.insert_generation(current.generation.compiled_generation);
        records.insert_effective_profile(current.generation.effective_profile);
        records.insert_published(&published);
        records.insert_advertised(&advertised);
        records.insert_fence(current.generation.activation_fence);
        records.insert_active(current.generation.active_head);
        let historical = Historical {
            records,
            session_signing_authority: signing_authority.clone(),
        };
        assert_eq!(retained_session.lease(), session.lease());

        let now = UnixTimeV1::new(70).expect("Consult recovery time");
        let exact_evidence = evidence_from(current);
        let request = resealed_operation_target_request(
            original,
            &exact_evidence,
            session,
            now,
            retained_operation_target_reference(&start_evidence),
            |_| {},
        );
        let current_admission = ResidentProgramCurrentRequestAdmission::new(
            1 << 20,
            vec![ResidentProgramRequestPolicy::new(
                ResidentOperationRequestKind::AdvanceConsult,
                policy_from(
                    original,
                    session.lease().compiled_generation_digest().clone(),
                ),
            )],
            EvidencePort {
                values: Arc::new(Mutex::new(VecDeque::from([Ok(exact_evidence)]))),
                reads: Arc::new(Mutex::new(0)),
            },
            Clock {
                values: Arc::new(Mutex::new(VecDeque::from([now]))),
                reads: Arc::new(Mutex::new(0)),
            },
            &historical,
            ProvingAuthority,
            Signature(true),
        )
        .expect("one exact AdvanceConsult policy");
        let policy_calls = Arc::new(AtomicUsize::new(0));
        let retained_turns = Arc::new(AtomicUsize::new(0));
        let executor_polls = Arc::new(AtomicUsize::new(0));
        let owner = ResidentProgramOperationTurnOwner::new(
            AcceptPeer,
            current_admission,
            vec![ResidentProgramDeploymentOperationPorts::new(
                selector,
                store.clone(),
                custody,
                sessions,
                anchor,
            )],
            FreshEntropy::new(Arc::new(AtomicUsize::new(0))),
            CountingAuthority::default(),
            StartCryptography {
                checkpoint: RefCell::new(None),
            },
            ExistingRecovery {
                recovery,
                admission: Some(admission),
                retained_deferred: Arc::new(AtomicUsize::new(0)),
            },
            AdvanceConsultInputs {
                resolver: AdvanceConsultStoreResolver { store },
                executor: AdvanceConsultExecutor {
                    polls: Arc::clone(&executor_polls),
                },
                admit_collection: admit_advance_consult_collection,
                recover_child: unreachable_advance_consult_child_recovery,
                retain_child: unreachable_advance_consult_child_retention,
                retained: Arc::clone(&retained_turns),
                fail_after_callback: true,
            },
        )
        .expect("one exact retained AdvanceConsult owner");
        let mut program = ResidentProgram::fixture_from_process_with_authorities(
            process,
            vec![authority],
            AdvanceConsultPolicy {
                calls: Arc::clone(&policy_calls),
            },
            owner,
        );
        let frame = ResidentOperationFrameRequest::AdvanceConsult {
            locator: locator.clone(),
            signed_common_request: pale_core::canon::to_canonical_string(&request)
                .expect("canonical AdvanceConsult request"),
        };
        let mut transport = OperationFramePort::new(&frame);
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(2))
            .expect("test deadline");
        let response = program
            .serve_operation_frame(&mut transport, deadline, deadline)
            .expect("AdvanceConsult reaches the point-addressed Consult owner");
        assert_eq!(
            response,
            ResidentOperationFrameResponse::Pending { locator }
        );
        assert_eq!(policy_calls.load(Ordering::SeqCst), 1);
        assert_eq!(executor_polls.load(Ordering::SeqCst), 1);
        assert_eq!(retained_turns.load(Ordering::SeqCst), 1);
        let decoded: ResidentOperationFrameResponse =
            serde_json::from_slice(&transport.sent).expect("one response decodes");
        assert_eq!(decoded, response);
    });
}

#[test]
fn framed_advance_delegate_recovers_exact_pairwise_lineage_before_phase_handoff() {
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::
            host_source_generation_copies(2);
    let restart_generation = generations.pop().expect("restart generation copy");
    let generation = generations.pop().expect("source generation copy");
    with_issued_delegate_child_named(
        &generation,
        &profile,
        "program-advance-delegate",
        |child, parent, parent_candidate, parent_checkpoint, lease_custody, _, parents| {
            let store = child.initial_candidate.store().clone();
            let authority =
                ResidentProgramDeploymentAuthority::fixture_from_current(parents.current)
                    .expect("the Delegate fixture retains complete deployment authority");
            let selector = authority.fixture_selector().clone();
            assert_eq!(selector, selector_from_signing_authority(parents.signing));
            let (sessions, mut resolver) = super::restart::cold_restart_sessions(parents, &store);
            let checkpoint_authority = super::a2a_cancel_task::checkpoint_signer(&child.checkpoint);
            let application_store = MemoryAuthorityStore::default();
            let mut application = ready_application(&application_store, store.clone(), 72);
            application.fixture_retain_generation(restart_generation);
            let (published, advertised) =
                active_predecessors(parents.current.generation.compiled_generation);
            let _ = application
                .publish(published.clone())
                .expect("the Delegate generation publishes");
            let _ = application
                .advertise(advertised.clone())
                .expect("the Delegate generation advertises");
            let _ = application
                .activate(
                    parents.current.generation.active_head.clone(),
                    parents.current.generation.activation_fence.clone(),
                )
                .expect("the Delegate generation activates");
            let (retained_session, _) = application
                .admit_session(
                    SessionAdmissionLeaseWireV1::from(parents.session.lease()),
                    parents.current,
                    &mut ProvingAuthority,
                )
                .expect("the exact Delegate session admits");
            resolver.insert_published(&published);
            resolver.insert_advertised(&advertised);
            let historical = Historical {
                records: resolver,
                session_signing_authority: parents.signing.clone(),
            };
            let process = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
                vec![unproved_deployment_for_selector(
                    &selector,
                    store.clone(),
                    application,
                )],
                std::num::NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one exact Delegate deployment");
            let now = child.request.issued_at();
            let evidence = evidence_from(parents.current);
            let request = resealed_operation_target_request(
                &child.request,
                &evidence,
                &retained_session,
                now,
                operation_target_reference(
                    child.operation.handle().0,
                    child
                        .operation
                        .handle()
                        .1
                        .canonical_operation_handle_sha256(),
                ),
                |_| {},
            );
            let admission = ResidentProgramCurrentRequestAdmission::new(
                1 << 20,
                vec![ResidentProgramRequestPolicy::new(
                    ResidentOperationRequestKind::AdvanceDelegate,
                    policy_from(
                        &child.request,
                        retained_session
                            .lease()
                            .compiled_generation_digest()
                            .clone(),
                    ),
                )],
                EvidencePort {
                    values: Arc::new(Mutex::new(VecDeque::from([Ok(evidence)]))),
                    reads: Arc::new(Mutex::new(0)),
                },
                Clock {
                    values: Arc::new(Mutex::new(VecDeque::from([now]))),
                    reads: Arc::new(Mutex::new(0)),
                },
                &historical,
                ProvingAuthority,
                Signature(true),
            )
            .expect("one exact AdvanceDelegate policy");

            crate::resident::operation_observation_custody::tests::native_execution::
                with_service_custody_faults(
                    &parent_checkpoint,
                    Some(&child.initial_candidate),
                    vec![parent_candidate],
                    None,
                    vec![lease_custody.parent, lease_custody.child],
                    0,
                    0,
                    0,
                    0,
                    0,
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    Vec::new(),
                    false,
                    None,
                    Rc::new(RefCell::new(None)),
                    |custody, anchor| {
                        let checkpoint_profile =
                            crate::local_operation_start_fixture::profile();
                        let observation_limits = OperationObservationRecoveryLimits::new(
                            16,
                            1 << 20,
                            1 << 20,
                        )
                        .expect("bounded observations");
                        let checkpoint_limits = OperationCheckpointAncestryLimits::new(
                            16,
                            1 << 20,
                            16,
                        )
                        .expect("bounded checkpoints");
                        let membership_limits = pale_ports::AccumulatorMembershipLimits::new(
                            64,
                            100_000,
                            1_000_000,
                        )
                        .expect("bounded membership");
                        let recovery_context = LocalOperationRecoveryContext {
                            store: &store,
                            profile: &checkpoint_profile,
                            handle_signing_authority: parents.signing,
                            checkpoint_authority: &checkpoint_authority,
                            historical: &historical.records,
                            maximum_record_bytes: 1 << 20,
                            maximum_operation_lease_bytes: 1 << 20,
                            maximum_session_lease_bytes: 1 << 20,
                            membership_limits,
                            observation_limits,
                            checkpoint_limits,
                        };
                        let child_coordinate = ControlRecordCoordinate::new(
                            child.operation.handle().0.clone(),
                            pale_ports::ControlRecordDigest::parse(
                                child
                                    .operation
                                    .handle()
                                    .1
                                    .canonical_operation_handle_sha256()
                                    .as_str()
                                    .to_owned(),
                            )
                            .expect("Delegate handle digest"),
                        );
                        let recovery = OperationControlRecoveryInput {
                            operation: LocalOperationHandleRecoveryInput {
                                context: recovery_context,
                                expected_handle: &child_coordinate,
                                maximum_parent_depth: 1,
                            },
                            observation: OperationObservationSelection::SelectedHead,
                            observation_limits,
                            checkpoint_limits,
                            membership_limits,
                        };
                        let child_handle = child.operation.handle().1;
                        let parent_handle = parent.handle().1;
                        let mut admit_request =
                            |reference: &ResidentRecordRefV1,
                             wire: CommonRequestEnvelopeWireV1| {
                                let expected = if reference
                                    == child.operation.request_coordinate().record_ref()
                                {
                                    &child.request
                                } else if reference == child.parent_request.0.record_ref() {
                                    &child.parent_request.1
                                } else {
                                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                                        record: "common request envelope",
                                        invariant: "known committed Delegate request",
                                    });
                                };
                                let preimage = expected.to_preimage();
                                CommonRequestEnvelopeV1::admit_contextual_wire(
                                    wire,
                                    &super::restart::request_admission_context(&preimage),
                                    |_, _, actual| {
                                        assert_eq!(actual, expected.caller_signature());
                                        Ok(())
                                    },
                                )
                            };
                        let mut admit_handle_authority =
                            |preimage: &OperationHandlePreimageV1,
                             recovered_parent: Option<&OperationHandleV1>| {
                                if preimage.operation_handle_id
                                    == *child_handle.operation_handle_id()
                                {
                                    if recovered_parent != Some(parent_handle) {
                                        return Err(
                                            InvalidResidentRecordV1::ContextualAdmission {
                                                record: "operation handle",
                                                invariant: "exact recovered Delegate parent",
                                            },
                                        );
                                    }
                                } else if preimage.operation_handle_id
                                    != *parent_handle.operation_handle_id()
                                    || recovered_parent.is_some()
                                {
                                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                                        record: "operation handle",
                                        invariant: "one admitted Delegate parent",
                                    });
                                }
                                Ok(())
                            };
                        let mut verify_handle_signature =
                            |_: &[u8],
                             _: &OperationHandlePreimageV1,
                             _: &OperationHandleDigest,
                             actual: &CanonicalSignatureV1| {
                                assert_eq!(
                                    actual,
                                    &crate::local_operation_start_fixture::signature()
                                );
                                Ok(())
                            };
                        let mut admit_checkpoint =
                            |_: &ResidentRecordRefV1,
                             wire: ControlStateCheckpointWireV1| {
                                ControlStateCheckpointV1::admit_signed_wire(
                                    wire,
                                    child.checkpoint.schema_sha256(),
                                    &checkpoint_authority,
                                    |_, _, _, actual| {
                                        assert_eq!(
                                            actual,
                                            &crate::local_operation_start_fixture::signature()
                                        );
                                        Ok(())
                                    },
                                )
                            };
                        let mut resolve_observation_signer =
                            |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                                Ok(parents.signing.clone())
                            };
                        let mut admit_observation_state =
                            |_: &OperationObservationStateV1| Ok(());
                        let mut admit_rotation =
                            |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| { Ok(()) };
                        let mut verify_observation_signature =
                            |_: &[u8],
                             _: &OperationObservationPreimageV1,
                             _: &OperationObservationDigest,
                             actual: &CanonicalSignatureV1| {
                                assert_eq!(
                                    actual,
                                    &crate::local_operation_start_fixture::signature()
                                );
                                Ok(())
                            };
                        let mut verify_checkpoint_signature =
                            |_: &[u8],
                             _: &ControlStateCheckpointPreimageV1,
                             _: &ControlStateCheckpointDigest,
                             actual: &CanonicalSignatureV1| {
                                assert_eq!(
                                    actual,
                                    &crate::local_operation_start_fixture::signature()
                                );
                                Ok(())
                            };
                        let recovery_admission = OperationControlRecoveryAdmission {
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
                        let expected_locator = operation_frame_locator(&child.operation);
                        let owner = ResidentProgramOperationTurnOwner::new(
                            AcceptPeer,
                            admission,
                            vec![ResidentProgramDeploymentOperationPorts::new(
                                selector,
                                store.clone(),
                                custody,
                                &sessions,
                                anchor,
                            )],
                            FreshEntropy::new(Arc::new(AtomicUsize::new(0))),
                            CountingAuthority::default(),
                            StartCryptography {
                                checkpoint: RefCell::new(None),
                            },
                            ExistingRecovery {
                                recovery,
                                admission: Some(recovery_admission),
                                retained_deferred: Arc::new(AtomicUsize::new(0)),
                            },
                            ResidentProgramConsultRecoveryUnavailable,
                        )
                        .expect("one exact retained AdvanceDelegate owner");
                        let mut program = ResidentProgram::fixture_from_process_with_authorities(
                            process,
                            vec![authority],
                            ConsultPolicy(None),
                            owner,
                        );
                        let frame = ResidentOperationFrameRequest::AdvanceDelegate {
                            locator: expected_locator.clone(),
                            signed_common_request: pale_core::canon::to_canonical_string(&request)
                                .expect("canonical AdvanceDelegate request"),
                        };
                        let mut transport = OperationFramePort::new(&frame);
                        let deadline = std::time::Instant::now()
                            .checked_add(std::time::Duration::from_secs(2))
                            .expect("test deadline");
                        let response = program
                            .serve_operation_frame(&mut transport, deadline, deadline)
                            .expect("AdvanceDelegate reaches the exact pairwise phase owner");
                        assert_eq!(
                            response,
                            ResidentOperationFrameResponse::Pending {
                                locator: expected_locator,
                            }
                        );
                        let decoded: ResidentOperationFrameResponse =
                            serde_json::from_slice(&transport.sent)
                                .expect("one response decodes");
                        assert_eq!(decoded, response);
                    },
                );
        },
    );
}

#[test]
fn application_advance_delegate_launches_one_exact_a2a_exchange_without_replay() {
    with_delegate_dispatch_input_named(
        super::a2a_delegate::joined_program_a2a_custody_model(),
        "program-a2a-joined",
        |dispatch,
         recovered_fixture,
         completed_context,
         checkpoint_schema,
         checkpoint_authority,
         _provider_clock,
         _resolve_store,
         _fixture_authority,
         custody,
         anchor,
         parents,
         parent_request,
         restart_generation,
         _initial_candidate| {
            let selected_store = recovered_fixture
                .start
                .control
                .observation()
                .store()
                .clone();
            let authority =
                ResidentProgramDeploymentAuthority::fixture_from_current(parents.current)
                    .expect("the A2A fixture retains complete deployment authority");
            let selector = authority.fixture_selector().clone();
            assert_eq!(selector, recovered_fixture.start.selector);
            let (sessions, mut resolver) =
                super::restart::cold_restart_sessions(parents, &selected_store);
            let application_store = MemoryAuthorityStore::default();
            let mut application = ready_application(&application_store, selected_store.clone(), 73);
            application.fixture_retain_generation(restart_generation);
            let (published, advertised) =
                active_predecessors(parents.current.generation.compiled_generation);
            let _ = application
                .publish(published.clone())
                .expect("the A2A generation publishes");
            let _ = application
                .advertise(advertised.clone())
                .expect("the A2A generation advertises");
            let _ = application
                .activate(
                    parents.current.generation.active_head.clone(),
                    parents.current.generation.activation_fence.clone(),
                )
                .expect("the A2A generation activates");
            let (retained_session, _) = application
                .admit_session(
                    SessionAdmissionLeaseWireV1::from(parents.session.lease()),
                    parents.current,
                    &mut ProvingAuthority,
                )
                .expect("the exact A2A session admits");
            resolver.insert_published(&published);
            resolver.insert_advertised(&advertised);
            let historical = Historical {
                records: resolver,
                session_signing_authority: parents.signing.clone(),
            };
            let process = ResidentProcessHost::new(
                unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
                vec![unproved_deployment_for_selector(
                    &selector,
                    selected_store.clone(),
                    application,
                )],
                std::num::NonZeroUsize::new(1).expect("one deployment"),
                ResidentOperationHost::new(task_state()),
            )
            .expect("one exact A2A deployment");

            let operation = dispatch.operation.context.operation;
            let now = dispatch.operation.context.request.issued_at();
            let evidence = evidence_from(parents.current);
            let request = resealed_operation_target_request(
                dispatch.operation.context.request,
                &evidence,
                &retained_session,
                now,
                operation_target_reference(
                    operation.handle().0,
                    operation.handle().1.canonical_operation_handle_sha256(),
                ),
                |_| {},
            );
            let admission = ResidentProgramCurrentRequestAdmission::new(
                1 << 20,
                vec![ResidentProgramRequestPolicy::new(
                    ResidentOperationRequestKind::AdvanceDelegate,
                    policy_from(
                        dispatch.operation.context.request,
                        retained_session
                            .lease()
                            .compiled_generation_digest()
                            .clone(),
                    ),
                )],
                EvidencePort {
                    values: Arc::new(Mutex::new(VecDeque::from([Ok(evidence)]))),
                    reads: Arc::new(Mutex::new(0)),
                },
                Clock {
                    values: Arc::new(Mutex::new(VecDeque::from([now]))),
                    reads: Arc::new(Mutex::new(0)),
                },
                &historical,
                ProvingAuthority,
                Signature(true),
            )
            .expect("one exact A2A AdvanceDelegate policy");

            let checkpoint_profile = crate::local_operation_start_fixture::profile();
            let observation_limits = OperationObservationRecoveryLimits::new(16, 1 << 20, 1 << 20)
                .expect("bounded observations");
            let checkpoint_limits = OperationCheckpointAncestryLimits::new(16, 1 << 20, 16)
                .expect("bounded checkpoints");
            let membership_limits =
                pale_ports::AccumulatorMembershipLimits::new(64, 100_000, 1_000_000)
                    .expect("bounded membership");
            let recovery_context = LocalOperationRecoveryContext {
                store: &selected_store,
                profile: &checkpoint_profile,
                handle_signing_authority: parents.signing,
                checkpoint_authority,
                historical: &historical.records,
                maximum_record_bytes: 1 << 20,
                maximum_operation_lease_bytes: 1 << 20,
                maximum_session_lease_bytes: 1 << 20,
                membership_limits,
                observation_limits,
                checkpoint_limits,
            };
            let child_coordinate = ControlRecordCoordinate::new(
                operation.handle().0.clone(),
                pale_ports::ControlRecordDigest::parse(
                    operation
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str()
                        .to_owned(),
                )
                .expect("A2A Delegate handle digest"),
            );
            let recovery = OperationControlRecoveryInput {
                operation: LocalOperationHandleRecoveryInput {
                    context: recovery_context,
                    expected_handle: &child_coordinate,
                    maximum_parent_depth: 1,
                },
                observation: OperationObservationSelection::SelectedHead,
                observation_limits,
                checkpoint_limits,
                membership_limits,
            };
            let child_handle = operation.handle().1;
            let parent = child_handle
                .parent_operation()
                .as_ref()
                .expect("the A2A Delegate has one parent");
            let child_request = dispatch.operation.context.request;
            let child_preimage = child_request.to_preimage();
            let parent_preimage = parent_request.1.to_preimage();
            let mut admit_request =
                |reference: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
                    let (expected, preimage) =
                        if reference == operation.request_coordinate().record_ref() {
                            (child_request, &child_preimage)
                        } else if reference == parent_request.0.record_ref() {
                            (&parent_request.1, &parent_preimage)
                        } else {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "common request envelope",
                                invariant: "known committed A2A Delegate request",
                            });
                        };
                    CommonRequestEnvelopeV1::admit_contextual_wire(
                        wire,
                        &super::restart::request_admission_context(preimage),
                        |_, _, actual| {
                            assert_eq!(actual, expected.caller_signature());
                            Ok(())
                        },
                    )
                };
            let mut admit_handle_authority =
                |preimage: &OperationHandlePreimageV1,
                 recovered_parent: Option<&OperationHandleV1>| {
                    if preimage.operation_handle_id == *child_handle.operation_handle_id() {
                        let recovered_parent = recovered_parent.ok_or(
                            InvalidResidentRecordV1::ContextualAdmission {
                                record: "operation handle",
                                invariant: "exact recovered A2A Delegate parent",
                            },
                        )?;
                        if recovered_parent.operation_handle_id() != &parent.operation_handle_id
                            || recovered_parent.canonical_operation_handle_sha256()
                                != &parent.operation_handle_sha256
                        {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "operation handle",
                                invariant: "exact recovered A2A Delegate parent",
                            });
                        }
                    } else if preimage.operation_handle_id != parent.operation_handle_id
                        || recovered_parent.is_some()
                    {
                        return Err(InvalidResidentRecordV1::ContextualAdmission {
                            record: "operation handle",
                            invariant: "one admitted A2A Delegate parent",
                        });
                    }
                    Ok(())
                };
            let mut verify_handle_signature =
                |_: &[u8],
                 _: &OperationHandlePreimageV1,
                 _: &OperationHandleDigest,
                 actual: &CanonicalSignatureV1| {
                    assert_eq!(actual, &local_operation_start_fixture::signature());
                    Ok(())
                };
            let mut admit_checkpoint =
                |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                    ControlStateCheckpointV1::admit_signed_wire(
                        wire,
                        checkpoint_schema,
                        checkpoint_authority,
                        |_, _, _, actual| {
                            assert_eq!(actual, &local_operation_start_fixture::signature());
                            Ok(())
                        },
                    )
                };
            let mut resolve_observation_signer =
                |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                    Ok(parents.signing.clone())
                };
            let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
            let mut admit_rotation =
                |_: &ResidentRecordRefV1,
                 _: &ArtifactDigest,
                 _: &ResidentRecordRefV1,
                 _: &ArtifactDigest| { Ok(()) };
            let mut verify_observation_signature =
                |_: &[u8],
                 _: &OperationObservationPreimageV1,
                 _: &OperationObservationDigest,
                 actual: &CanonicalSignatureV1| {
                    assert_eq!(actual, &local_operation_start_fixture::signature());
                    Ok(())
                };
            let mut verify_checkpoint_signature =
                |_: &[u8],
                 _: &ControlStateCheckpointPreimageV1,
                 _: &ControlStateCheckpointDigest,
                 actual: &CanonicalSignatureV1| {
                    assert_eq!(actual, &local_operation_start_fixture::signature());
                    Ok(())
                };
            let recovery_admission = OperationControlRecoveryAdmission {
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

            let provider = dispatch.operation.invocation.provider_binding().clone();
            let route = a2a_exchange::route_for_handle_at_with_authority(
                &provider,
                child_handle,
                "https://agent.example.test/a2a",
                true,
                10,
                100,
                110,
                parents.signing,
            );
            let exchange = Rc::new(super::a2a_delegate::A2aProcessExchangePort::direct_message());
            let awaiting = Rc::new(Cell::new(0));
            let a2a = JoinedProgramA2aPort {
                dispatch: Some(dispatch),
                frame_request: request.clone(),
                completed_context,
                launch_current: parents.current.at_time(UnixTimeV1::new(65).unwrap()),
                route_ref: local_operation_start_fixture::record_ref("program-a2a-joined/route"),
                route,
                request_id: A2aJsonRpcRequestIdV1::parse("program-a2a-rpc").unwrap(),
                clock: super::a2a_delegate::A2aProcessClock::new([Ok(66), Ok(67), Ok(68)]),
                exchange: Rc::clone(&exchange),
                authority: ProvingAuthority,
                awaiting: Rc::clone(&awaiting),
            };
            let recovery = JoinedProgramA2aRecovery {
                recovery,
                admission: Some(recovery_admission),
                a2a,
            };
            let expected_locator = operation_frame_locator(operation);
            let owner = ResidentProgramOperationTurnOwner::new(
                AcceptPeer,
                admission,
                vec![ResidentProgramDeploymentOperationPorts::new(
                    selector,
                    selected_store.clone(),
                    custody,
                    &sessions,
                    anchor,
                )],
                FreshEntropy::new(Arc::new(AtomicUsize::new(0))),
                CountingAuthority::default(),
                StartCryptography {
                    checkpoint: RefCell::new(None),
                },
                recovery,
                ResidentProgramConsultRecoveryUnavailable,
            )
            .expect("one exact joined A2A owner");
            let program = ResidentProgram::fixture_from_process_with_authorities(
                process,
                vec![authority],
                ConsultPolicy(None),
                owner,
            );
            let mut service = crate::resident::ResidentFrameService::from_program(program);
            let frame = ResidentOperationFrameRequest::AdvanceDelegate {
                locator: expected_locator.clone(),
                signed_common_request: pale_core::canon::to_canonical_string(&request)
                    .expect("canonical A2A AdvanceDelegate request"),
            };
            let mut transport = OperationFramePort::new(&frame);
            let deadline = std::time::Instant::now()
                .checked_add(std::time::Duration::from_secs(2))
                .expect("test deadline");
            assert_eq!(
                service
                    .serve_frame(&mut transport, deadline, deadline)
                    .expect("AdvanceDelegate launches its admitted A2A route"),
                crate::resident::ResidentFrameOutcome::Operation,
            );
            let response: ResidentOperationFrameResponse =
                serde_json::from_slice(&transport.sent).expect("AdvanceDelegate response decodes");
            assert_eq!(
                response,
                ResidentOperationFrameResponse::Pending {
                    locator: expected_locator,
                }
            );
            assert_eq!(exchange.start_count(), 1);
            assert_eq!(awaiting.get(), 1);
            assert_or_update_application_delegate_advance_fixture(&frame, &response);
        },
    );
}

#[test]
fn framed_start_delegate_recovers_the_exact_parent_before_durable_child_start() {
    let now = UnixTimeV1::new(62).expect("child request time");
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::source_generation_copies(2);
    let retained = generations.pop().expect("retained host generation");
    let semantic = generations.pop().expect("Delegate fixture generation");
    with_issued_context(
        &semantic,
        &profile,
        "program-delegate-parent",
        |_| {},
        crate::session_admission_fixture::LocalStartContext::Inherited,
        |parent, parent_request, parents, parent_checkpoint, _, parent_candidate| {
            with_test_child_start_input(
                parent.handle(),
                parent.historical_operation_lease(),
                Some((
                    &crate::local_operation_start_fixture::record_ref("operation/checkpoint-1"),
                    parent_checkpoint,
                )),
                parent_request,
                &semantic,
                &parents,
                OperationClassV1::Delegate,
                "program-delegate-child",
                |child, mut child_checkpoint| {
                    let child_request = child.semantic.request;
                    let store = crate::local_operation_start_fixture::store_coordinate();
                    let port = MemoryAuthorityStore::default();
                    let mut application = ready_application(&port, store.clone(), 102);
                    application.fixture_retain_generation(retained);
                    let (published, advertised) =
                        active_predecessors(child.semantic.current.generation.compiled_generation);
                    let _ = application
                        .publish(published.clone())
                        .expect("publish generation");
                    let _ = application
                        .advertise(advertised.clone())
                        .expect("advertise generation");
                    let _ = application
                        .activate(
                            child.semantic.current.generation.active_head.clone(),
                            child.semantic.current.generation.activation_fence.clone(),
                        )
                        .expect("activate generation");
                    let (session, _) = application
                        .admit_session(
                            SessionAdmissionLeaseWireV1::from(parents.session.lease()),
                            parents.current,
                            &mut ProvingAuthority,
                        )
                        .expect("admit Delegate session");
                    let authority =
                        ResidentProgramDeploymentAuthority::fixture_from_current(parents.current)
                            .expect("complete deployment authority");
                    let selector = authority.fixture_selector().clone();
                    let authorities = vec![authority];
                    let process = ResidentProcessHost::new(
                        unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
                        vec![unproved_deployment_for_selector(
                            &selector,
                            store.clone(),
                            application,
                        )],
                        std::num::NonZeroUsize::new(1).expect("one deployment"),
                        ResidentOperationHost::new(task_state()),
                    )
                    .expect("one exact Delegate deployment");
                    let mut records = RecoveryResolver::default();
                    records
                        .insert_generation(child.semantic.current.generation.compiled_generation);
                    records.insert_effective_profile(
                        child.semantic.current.generation.effective_profile,
                    );
                    records.insert_published(&published);
                    records.insert_advertised(&advertised);
                    records.insert_fence(child.semantic.current.generation.activation_fence);
                    records.insert_active(child.semantic.current.generation.active_head);
                    let historical = Historical {
                        records,
                        session_signing_authority: parents.signing.clone(),
                    };
                    process.deployments[0]
                        .application
                        .recover_exact_retained_session(
                            session.lease().session_admission_id(),
                            session.lease().session_admission_lease_digest(),
                            &historical,
                        )
                        .expect("recover exact retained Delegate session");
                    let evidence = evidence_from(parents.current);
                    let request = resealed_operation_target_request(
                        child_request,
                        &evidence,
                        &session,
                        now,
                        operation_target_reference(
                            parent.handle().0,
                            parent.handle().1.canonical_operation_handle_sha256(),
                        ),
                        |value| value.expires_at = child.semantic.handle.expires_at,
                    );
                    let mut request_policy = policy_from(
                        child_request,
                        session.lease().compiled_generation_digest().clone(),
                    );
                    request_policy.maximum_request_lifetime_seconds = request
                        .expires_at()
                        .get()
                        .checked_sub(now.get())
                        .expect("Delegate request expires after admission");
                    let admission = ResidentProgramCurrentRequestAdmission::new(
                        1 << 20,
                        vec![ResidentProgramRequestPolicy::new(
                            ResidentOperationRequestKind::StartDelegate,
                            request_policy,
                        )],
                        EvidencePort {
                            values: Arc::new(Mutex::new(vec![Ok(evidence)].into())),
                            reads: Arc::new(Mutex::new(0)),
                        },
                        Clock {
                            values: Arc::new(Mutex::new(vec![now].into())),
                            reads: Arc::new(Mutex::new(0)),
                        },
                        &historical,
                        ProvingAuthority,
                        Signature(true),
                    )
                    .expect("one exact Delegate policy");
                    let sequence = parent_checkpoint.domain_sequence().get() + 1;
                    let parent_lease = parent.historical_operation_lease();
                    let parent_lease_coordinate = crate::resident::record_projection::coordinate(
                        parent.handle().1.operation_lease_ref(),
                        parent_lease.operation_lease_digest().as_str(),
                    )
                    .expect("parent lease coordinate");
                    let parent_lease_record = crate::resident::record_projection::record(
                        parent_lease_coordinate,
                        parent_lease,
                    )
                    .expect("parent lease record");
                    let parent_basis = parent_candidate
                        .session_basis()
                        .expect("committed parent retains its private session basis");
                    assert_eq!(
                        parent_basis.handle().record_ref(),
                        parent.handle().0,
                        "parent basis must name the framed parent handle",
                    );
                    let mut custody = crate::resident::operation_observation_custody::tests::RecordingStore::committing(
                        OperationCustodyCommitOutcome::Committed,
                    )
                    .at_store(&store)
                    .with_preparation(
                        prepare_collection_accumulator,
                        ControlTransactionSequenceV1::new(sequence)
                            .expect("child transaction sequence"),
                    )
                    .with_lease_retention(Ok(OperationCustodyCommitOutcome::Committed))
                    .with_unlisted_indexes_absent()
                    .with_committed_candidate(parent_candidate.clone())
                    .with_retained_lease_record(parent_lease_record);
                    let parent_head = parent_candidate.checkpoint().successor();
                    let mut anchor = RecoveryThenWriteAnchor {
                        recovered: Some(pale_ports::AntiRollbackAnchor::ControlDomain(
                            pale_ports::ControlDomainAntiRollbackAnchor::new(
                                store.store_id().clone(),
                                store.incarnation_id().clone(),
                                parent_head.checkpoint_domain_id().clone(),
                                parent_head.domain_epoch(),
                                parent_head.domain_sequence(),
                                parent_head.coordinate().clone(),
                            ),
                        )),
                        write: successor_anchor(),
                    };
                    child_checkpoint.issued_at =
                        UnixTimeV1::new(parent_checkpoint.issued_at().get() + 1)
                            .expect("successor checkpoint time");
                    let owner = ResidentProgramOperationTurnOwner::new(
                        AcceptPeer,
                        admission,
                        vec![ResidentProgramDeploymentOperationPorts::new(
                            selector,
                            store.clone(),
                            &mut custody,
                            &port,
                            &mut anchor,
                        )],
                        FreshEntropy::new(Arc::new(AtomicUsize::new(2))),
                        CountingAuthority::default(),
                        StartCryptography {
                            checkpoint: RefCell::new(Some(child_checkpoint)),
                        },
                        ParentRecovery {
                            store,
                            parent_request,
                            parent_checkpoint,
                            historical: &historical,
                        },
                        ResidentProgramConsultRecoveryUnavailable,
                    )
                    .expect("one exact retained Delegate owner");
                    let policy = delegate_policy(&child.semantic);
                    let mut program = ResidentProgram::fixture_from_process_with_authorities(
                        process,
                        authorities,
                        policy,
                        owner,
                    );
                    let parent_locator = operation_frame_locator(parent);
                    let frame = ResidentOperationFrameRequest::StartDelegate {
                        parent: parent_locator,
                        signed_common_request: pale_core::canon::to_canonical_string(&request)
                            .expect("canonical Delegate request"),
                    };
                    let mut transport = OperationFramePort::new(&frame);
                    let deadline = std::time::Instant::now()
                        .checked_add(std::time::Duration::from_secs(2))
                        .expect("test deadline");
                    let response = program
                        .serve_operation_frame(&mut transport, deadline, deadline)
                        .expect("Delegate reaches one durable framed response");
                    assert!(
                        matches!(response, ResidentOperationFrameResponse::Started { .. }),
                        "unexpected Delegate response: {response:?}",
                    );
                    let decoded: ResidentOperationFrameResponse =
                        serde_json::from_slice(&transport.sent).expect("one response decodes");
                    assert_eq!(decoded, response);
                },
            );
        },
    );
}

#[test]
fn recovered_delegate_service_starts_one_durable_narrowed_child() {
    crate::resident::deployment_finalization::tests::recovery::with_evidence_for_deployment(
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
                crate::resident::operation_observation_custody::tests::local_start::
                    source_generation_copies_with_reconstruction(2);
            let retained = generations.pop().expect("retained host generation");
            let semantic = generations.pop().expect("Delegate fixture generation");

            crate::resident::operation_observation_custody::tests::local_start::
                with_issued_context_at_store_and_authorities(
                    &semantic,
                    &profile,
                    "public-service-delegate-parent",
                    &store,
                    current_trust.coordinate().record_ref(),
                    current_trust.checkpoint(),
                    runtime_authority.binding_ref(),
                    runtime_authority.binding(),
                    runtime_authority.record_ref(),
                    runtime_authority.authority(),
                    |parent,
                     parent_request,
                     parents,
                     parent_checkpoint,
                     _,
                     parent_candidate| {
                        with_test_child_start_input(
                            parent.handle(),
                            parent.historical_operation_lease(),
                            Some((
                                &crate::local_operation_start_fixture::record_ref(
                                    "operation/checkpoint-1",
                                ),
                                parent_checkpoint,
                            )),
                            parent_request,
                            &semantic,
                            &parents,
                            OperationClassV1::Delegate,
                            "public-service-delegate-child",
                            |child, mut child_checkpoint| {
                                let application_store = MemoryAuthorityStore::default();
                                let mut application =
                                    ready_application(&application_store, store.clone(), 112);
                                let published = crate::resident_fixture::published(
                                    parents.current.generation.compiled_generation,
                                    1,
                                    None,
                                );
                                let advertised =
                                    crate::resident_fixture::advertised(&published, 2, None);
                                application.fixture_retain_generation(retained);
                                let _ = application
                                    .publish(published.clone())
                                    .expect("publish Delegate generation");
                                let _ = application
                                    .advertise(advertised.clone())
                                    .expect("advertise Delegate generation");
                                let _ = application
                                    .activate(
                                        parents.current.generation.active_head.clone(),
                                        parents.current.generation.activation_fence.clone(),
                                    )
                                    .expect("activate Delegate generation");
                                let (session, _) = application
                                    .admit_session(
                                        SessionAdmissionLeaseWireV1::from(parents.session.lease()),
                                        parents.current,
                                        &mut ProvingAuthority,
                                    )
                                    .expect("retain exact Delegate session");
                                application
                                    .checkpoint()
                                    .expect("checkpoint Delegate resident state");
                                drop(application);

                                let mut records = RecoveryResolver::default();
                                records.insert_generation(
                                    parents.current.generation.compiled_generation,
                                );
                                records.insert_effective_profile(
                                    parents.current.generation.effective_profile,
                                );
                                records.insert_published(&published);
                                records.insert_advertised(&advertised);
                                records.insert_fence(
                                    parents.current.generation.activation_fence,
                                );
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
                                let cold_open = ResidentColdOpenInput::new(
                                    store.clone(),
                                    limits(),
                                    limits().maximum_entry_bytes(),
                                    RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                                    &historical,
                                    &mut source,
                                    ResidentReconstructionParameters::new(
                                        synthetic_set_policy(),
                                        4,
                                        4,
                                        WakeEpochV1::new(203),
                                    ),
                                );

                                let evidence = service_evidence_from(parents.current);
                                let now = parents.current.now;
                                let request = resealed_operation_target_request(
                                    child.semantic.request,
                                    &evidence_from(parents.current),
                                    &session,
                                    now,
                                    operation_target_reference(
                                        parent.handle().0,
                                        parent
                                            .handle()
                                            .1
                                            .canonical_operation_handle_sha256(),
                                    ),
                                    |value| {
                                        value.expires_at = child.semantic.handle.expires_at
                                    },
                                );
                                let substituted_request = resealed_operation_target_request(
                                    child.semantic.request,
                                    &evidence_from(parents.current),
                                    &session,
                                    now,
                                    operation_target_reference(
                                        parent.handle().0,
                                        parent
                                            .handle()
                                            .1
                                            .canonical_operation_handle_sha256(),
                                    ),
                                    |value| {
                                        value.expires_at = child.semantic.handle.expires_at;
                                        value.operation = PackageResourceRefV1 {
                                            package_id: PackageIdV1::parse("other.package")
                                                .expect("other Package"),
                                            resource_id: PackageResourceIdV1::parse(
                                                "other-operation",
                                            )
                                            .expect("other operation"),
                                        };
                                    },
                                );
                                let policy = service_policy_from(
                                    &semantic,
                                    session.lease().repository_id().clone(),
                                    child.semantic.request,
                                );

                                let sequence = parent_checkpoint.domain_sequence().get() + 1;
                                let parent_lease = parent.historical_operation_lease();
                                let parent_lease_coordinate =
                                    crate::resident::record_projection::coordinate(
                                        parent.handle().1.operation_lease_ref(),
                                        parent_lease.operation_lease_digest().as_str(),
                                    )
                                    .expect("parent lease coordinate");
                                let parent_lease_record =
                                    crate::resident::record_projection::record(
                                        parent_lease_coordinate,
                                        parent_lease,
                                    )
                                    .expect("parent lease record");
                                let mut custody = crate::resident::operation_observation_custody::tests::RecordingStore::committing(
                                    OperationCustodyCommitOutcome::Committed,
                                )
                                .at_store(&store)
                                .with_preparation(
                                    prepare_collection_accumulator,
                                    ControlTransactionSequenceV1::new(sequence)
                                        .expect("child transaction sequence"),
                                )
                                .with_lease_retention(Ok(
                                    OperationCustodyCommitOutcome::Committed,
                                ))
                                .with_unlisted_indexes_absent()
                                .with_committed_candidate(parent_candidate.clone())
                                .with_retained_lease_record(parent_lease_record);
                                let parent_head = parent_candidate.checkpoint().successor();
                                let mut anchor = RecoveryThenWriteAnchor {
                                    recovered: Some(pale_ports::AntiRollbackAnchor::ControlDomain(
                                        pale_ports::ControlDomainAntiRollbackAnchor::new(
                                            store.store_id().clone(),
                                            store.incarnation_id().clone(),
                                            parent_head.checkpoint_domain_id().clone(),
                                            parent_head.domain_epoch(),
                                            parent_head.domain_sequence(),
                                            parent_head.coordinate().clone(),
                                        ),
                                    )),
                                    write: successor_anchor(),
                                };
                                child_checkpoint.issued_at = UnixTimeV1::new(
                                    parent_checkpoint.issued_at().get() + 1,
                                )
                                .expect("successor checkpoint time");

                                let writer =
                                    crate::local_operation_start_fixture::publication_writer();
                                let checkpoint =
                                    super::program::checkpoint_for_writer(&writer, 'c');
                                let snapshot =
                                    super::program::writer_snapshot(&writer, &checkpoint);
                                let process = crate::resident::ResidentServiceProcessInput {
                                    held_writer: super::program::HeldWriter(
                                        snapshot.binding().principal_id().clone(),
                                    ),
                                    writer_snapshot: &snapshot,
                                    writer_admission: ServiceWriterAdmission {
                                        writer,
                                        checkpoint,
                                    },
                                    trust: &trust,
                                    deployments: vec![
                                        crate::resident::ResidentServiceDeploymentOpen {
                                            store: store.clone(),
                                            durable: &application_store,
                                            sessions: &application_store,
                                            events: &mut events,
                                            maximum_event_custody_bytes: 65_536,
                                            cold_open,
                                            operations: &mut custody,
                                            anti_rollback: &mut anchor,
                                        },
                                    ],
                                    maximum_ready_deployments: NonZeroUsize::new(1)
                                        .expect("one deployment"),
                                    task_limits: crate::resident::ResidentServiceTaskLimits {
                                        maximum_provider_tasks: NonZeroUsize::new(1)
                                            .expect("one provider task"),
                                        maximum_consultation_batches: NonZeroUsize::new(1)
                                            .expect("one consultation batch"),
                                        maximum_aggregate_tasks: NonZeroUsize::new(1)
                                            .expect("one aggregate task"),
                                    },
                                };
                                let lease = child.semantic.context.operation_lease;
                                let operation_policy =
                                    crate::resident::ResidentServiceOperationStartPolicy {
                                        checkpoint_profile: child
                                            .semantic
                                            .checkpoint_profile
                                            .clone(),
                                        publication_policy: crate::resident::
                                            LocalOperationPublicationPolicy::from_admitted_input(
                                                crate::resident::
                                                    LocalOperationPublicationPolicyInput {
                                                    target_scope: child
                                                        .semantic
                                                        .publication_policy
                                                        .target_scope,
                                                    classification: child
                                                        .semantic
                                                        .publication_policy
                                                        .classification,
                                                    visibility: child
                                                        .semantic
                                                        .publication_policy
                                                        .visibility,
                                                    disclosure_ceiling: child
                                                        .semantic
                                                        .publication_policy
                                                        .disclosure_ceiling,
                                                    payload_resolution_policy: child
                                                        .semantic
                                                        .publication_policy
                                                        .payload_resolution_policy,
                                                    actor_provenance: child
                                                        .semantic
                                                        .publication_policy
                                                        .actor_provenance,
                                                },
                                            ),
                                        maximum_record_bytes: 1 << 20,
                                        lease: crate::resident::
                                            ResidentServiceOperationLeasePolicy {
                                            capabilities: lease.capabilities().to_vec(),
                                            effects: lease.effects().to_vec(),
                                            read_set: lease.read_set().to_vec(),
                                            evidence_obligations: lease
                                                .evidence_obligations()
                                                .to_vec(),
                                            budget_units: *lease.budget_units(),
                                            timeout_millis: *lease.timeout_millis(),
                                            expires_at: lease.expires_at(),
                                            invocation_correlation: lease
                                                .invocation_correlation()
                                                .clone(),
                                        },
                                        handle_expires_at: child.semantic.handle.expires_at,
                                        handle_retain_until: child.semantic.handle.retain_until,
                                        accumulator_limits: AccumulatorPreparationLimits::new(
                                            4, 1024, 1024, 512, 65_536,
                                        )
                                        .expect("bounded accumulator"),
                                    };
                                let policy_calls = Arc::new(AtomicUsize::new(0));
                                let entropy_calls = Arc::new(AtomicUsize::new(0));
                                let handle_signs = Arc::new(AtomicUsize::new(0));
                                let observation_signs = Arc::new(AtomicUsize::new(0));
                                let checkpoint_signs = Arc::new(AtomicUsize::new(0));
                                let parent_handle = parent.handle();
                                let owners = crate::resident::process_host::service_assembly::ResidentServiceInvokeOwners {
                                    peer_admission: Box::new(ExactTestPeer::new(
                                        LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                                    )),
                                    maximum_request_bytes: 1 << 20,
                                    request_policies:
                                        crate::resident::ResidentServiceOperationRequestPolicies {
                                            start: Vec::new(),
                                            advance: Vec::new(),
                                            start_consult: Vec::new(),
                                            advance_consult: Vec::new(),
                                            start_delegate: vec![policy],
                                            advance_delegate: Vec::new(),
                                            observe: Vec::new(),
                                            cancel: Vec::new(),
                                        },
                                    historical: &historical,
                                    evidence: ServiceEvidencePort(VecDeque::from([
                                        Ok(evidence.clone()),
                                        Ok(evidence.clone()),
                                        Ok(evidence),
                                    ])),
                                    clock: Clock {
                                        values: Arc::new(Mutex::new(VecDeque::from([
                                            now, now, now,
                                        ]))),
                                        reads: Arc::new(Mutex::new(0)),
                                    },
                                    session_authority: ProvingAuthority,
                                    caller_signature: ServiceSignature,
                                    random: ServiceEntropy {
                                        calls: Arc::clone(&entropy_calls),
                                    },
                                    operation_authority: ProvingAuthority,
                                    policy: ServiceDelegatePolicy {
                                        policy: operation_policy,
                                        expected_parent_ref: parent_handle.0.clone(),
                                        expected_parent_sha256: parent_handle
                                            .1
                                            .canonical_operation_handle_sha256()
                                            .clone(),
                                        calls: Arc::clone(&policy_calls),
                                    },
                                    cryptography: ServiceStartCryptography {
                                        checkpoint: RefCell::new(Some(child_checkpoint)),
                                        handle_signs: Arc::clone(&handle_signs),
                                        observation_signs: Arc::clone(&observation_signs),
                                        checkpoint_signs: Arc::clone(&checkpoint_signs),
                                    },
                                    invoke_inputs: ParentRecovery {
                                        store: store.clone(),
                                        parent_request,
                                        parent_checkpoint,
                                        historical: &historical,
                                    },
                                    publication_inputs: Box::new(
                                        crate::resident::process_host::service_assembly::UnavailableResidentServicePublicationInputs,
                                    ),
                                    delegate_inputs: crate::resident::process_host::service_assembly::UnavailableResidentServiceDelegateInputs,
                                    cancel_inputs: crate::resident::process_host::service_assembly::cancellation_test_support::UnavailableResidentServiceCancellationInputs,
                                    observe_inputs: ServiceObserveUnavailable,
                                };
                                let mut service =
                                    crate::resident::ResidentFrameService::recover_invoke(
                                        process, owners,
                                    )
                                    .expect("recover public Delegate service");
                                let parent_locator = operation_frame_locator(parent);
                                let substituted_frame =
                                    ResidentOperationFrameRequest::StartDelegate {
                                        parent: parent_locator.clone(),
                                        signed_common_request:
                                            pale_core::canon::to_canonical_string(
                                                &substituted_request,
                                            )
                                            .expect("canonical substituted Delegate request"),
                                    };
                                let mut substituted_transport =
                                    OperationFramePort::new(&substituted_frame);
                                let substituted_deadline = std::time::Instant::now()
                                    .checked_add(std::time::Duration::from_secs(2))
                                    .expect("test deadline");
                                assert_eq!(
                                    service
                                        .serve_frame(
                                            &mut substituted_transport,
                                            substituted_deadline,
                                            substituted_deadline,
                                        )
                                        .expect("the unrelated child reaches one rejection"),
                                    crate::resident::ResidentFrameOutcome::Operation,
                                );
                                assert_eq!(
                                    serde_json::from_slice::<ResidentOperationFrameResponse>(
                                        &substituted_transport.sent,
                                    )
                                    .expect("the unrelated-child rejection decodes"),
                                    ResidentOperationFrameResponse::Rejected {
                                        failure:
                                            ResidentOperationFrameFailure::ConfigurationUnavailable,
                                    },
                                );
                                assert_eq!(policy_calls.load(Ordering::SeqCst), 0);
                                assert_eq!(entropy_calls.load(Ordering::SeqCst), 0);
                                assert_eq!(handle_signs.load(Ordering::SeqCst), 0);
                                assert_eq!(observation_signs.load(Ordering::SeqCst), 0);
                                assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 0);

                                let wrong_parent = ResidentOperationFrameLocator::new(
                                    parent_locator.operation_handle_record_ref().to_owned(),
                                    "0".repeat(64),
                                );
                                let wrong_parent_frame =
                                    ResidentOperationFrameRequest::StartDelegate {
                                        parent: wrong_parent,
                                        signed_common_request:
                                            pale_core::canon::to_canonical_string(&request)
                                                .expect("canonical Delegate request"),
                                    };
                                let mut wrong_parent_transport =
                                    OperationFramePort::new(&wrong_parent_frame);
                                let wrong_parent_deadline = std::time::Instant::now()
                                    .checked_add(std::time::Duration::from_secs(2))
                                    .expect("test deadline");
                                assert_eq!(
                                    service
                                        .serve_frame(
                                            &mut wrong_parent_transport,
                                            wrong_parent_deadline,
                                            wrong_parent_deadline,
                                        )
                                        .expect("the unlike parent reaches one rejection"),
                                    crate::resident::ResidentFrameOutcome::Operation,
                                );
                                assert_eq!(
                                    serde_json::from_slice::<ResidentOperationFrameResponse>(
                                        &wrong_parent_transport.sent,
                                    )
                                    .expect("the unlike-parent rejection decodes"),
                                    ResidentOperationFrameResponse::Rejected {
                                        failure: ResidentOperationFrameFailure::InvalidRequest,
                                    },
                                );
                                assert_eq!(policy_calls.load(Ordering::SeqCst), 0);
                                assert_eq!(entropy_calls.load(Ordering::SeqCst), 0);
                                assert_eq!(handle_signs.load(Ordering::SeqCst), 0);
                                assert_eq!(observation_signs.load(Ordering::SeqCst), 0);
                                assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 0);

                                let frame = ResidentOperationFrameRequest::StartDelegate {
                                    parent: parent_locator,
                                    signed_common_request: pale_core::canon::to_canonical_string(
                                        &request,
                                    )
                                    .expect("canonical Delegate request"),
                                };
                                let mut transport = OperationFramePort::new(&frame);
                                let start_deadline = std::time::Instant::now()
                                    .checked_add(std::time::Duration::from_secs(2))
                                    .expect("test deadline");
                                assert_eq!(
                                    service
                                        .serve_frame(
                                            &mut transport,
                                            start_deadline,
                                            start_deadline,
                                        )
                                        .expect("StartDelegate reaches one response"),
                                    crate::resident::ResidentFrameOutcome::Operation,
                                );
                                let response: ResidentOperationFrameResponse =
                                    serde_json::from_slice(&transport.sent)
                                        .expect("Delegate response decodes");
                                assert!(
                                    matches!(
                                        &response,
                                        ResidentOperationFrameResponse::Started { .. }
                                    ),
                                    "unexpected public StartDelegate response: {response:?}"
                                );
                                assert_eq!(policy_calls.load(Ordering::SeqCst), 1);
                                assert_eq!(entropy_calls.load(Ordering::SeqCst), 7);
                                assert_eq!(handle_signs.load(Ordering::SeqCst), 1);
                                assert_eq!(observation_signs.load(Ordering::SeqCst), 1);
                                assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 1);
                                assert_or_update_application_delegate_fixture(&frame, &response);

                                drop(service);
                                assert_eq!(custody.mutation_counts().0, 2);
                            },
                        );
                    },
                );
        },
    );
}

fn assert_framed_cancel(
    fault: &str,
    expected: pale_ports::ResidentOperationFrameCancellationDisposition,
) {
    assert_framed_cancel_selection(fault, Ok(expected), None);
}

#[derive(Clone, Copy)]
enum CancellationSelectionMismatch {
    DeploymentBindingReference,
    DeploymentBindingDigest,
    RuntimeAuthorityReference,
    RuntimeAuthorityDigest,
    RequestId,
    RequestDigest,
    LocatorReference,
    LocatorDigest,
}

fn assert_framed_cancel_selection(
    fault: &str,
    expected: Result<
        pale_ports::ResidentOperationFrameCancellationDisposition,
        ResidentOperationFrameFailure,
    >,
    mismatch: Option<CancellationSelectionMismatch>,
) {
    let records_application_fixture = fault == "cancel-active-allowed" && mismatch.is_none();
    with_resident_program_cancellation_fixture(fault, &mut move |fixture| {
        let ResidentProgramCancellationFixture {
            cancellation,
            session,
            current,
            generation,
        } = fixture;
        let crate::resident::operation_observation_custody::tests::local_recovery::ProcessHostCancellationFixture {
            input,
            admission,
            signing_authority,
            operation_handle,
            custody,
            sessions,
            anchor,
        } = cancellation;
        let local_completion = super::local_cancellation_completion(&input);
        let store = input.recovery.operation.context.store.clone();
        let now = input.authorized_at;
        let exact_evidence = evidence_from(current);
        let request = input.request.clone();
        let locator = ResidentOperationFrameLocator::new(
            operation_handle.0.as_str().to_owned(),
            operation_handle
                .1
                .canonical_operation_handle_sha256()
                .as_str()
                .to_owned(),
        );
        let frame = ResidentOperationFrameRequest::Cancel {
            locator: locator.clone(),
            signed_common_request: pale_core::canon::to_canonical_string(&request)
                .expect("canonical Cancel request"),
        };

        let authority = ResidentProgramDeploymentAuthority::fixture_from_current(current)
            .expect("the Cancel fixture retains complete deployment authority");
        let selector = authority.fixture_selector().clone();
        assert_eq!(selector, selector_from_signing_authority(signing_authority));
        let mut expected_deployment = selector.clone();
        let mut expected_request_id = request.request_id().clone();
        let mut expected_request_sha256 = request.canonical_request_sha256().clone();
        let mut expected_locator = input.recovery.operation.expected_handle.clone();
        match mismatch {
            Some(CancellationSelectionMismatch::DeploymentBindingReference) => {
                expected_deployment.deployment_binding_ref =
                    ResidentRecordRefV1::parse("unrelated-deployment-binding")
                        .expect("unrelated binding reference is valid");
            }
            Some(CancellationSelectionMismatch::DeploymentBindingDigest) => {
                expected_deployment.deployment_binding_sha256 =
                    DeploymentBindingDigest::parse("0".repeat(64))
                        .expect("unrelated binding digest is valid");
            }
            Some(CancellationSelectionMismatch::RuntimeAuthorityReference) => {
                expected_deployment.runtime_authority_ref =
                    ResidentRecordRefV1::parse("unrelated-runtime-authority")
                        .expect("unrelated runtime-authority reference is valid");
            }
            Some(CancellationSelectionMismatch::RuntimeAuthorityDigest) => {
                expected_deployment.runtime_authority_sha256 =
                    DeploymentRuntimeAuthorityDigest::parse("0".repeat(64))
                        .expect("unrelated runtime-authority digest is valid");
            }
            Some(CancellationSelectionMismatch::RequestId) => {
                expected_request_id = ResidentRequestIdV1::parse("unrelated-request")
                    .expect("unrelated request is valid");
            }
            Some(CancellationSelectionMismatch::RequestDigest) => {
                expected_request_sha256 = CommonRequestEnvelopeDigest::parse("0".repeat(64))
                    .expect("unrelated request digest is valid");
            }
            Some(CancellationSelectionMismatch::LocatorReference) => {
                expected_locator = ControlRecordCoordinate::new(
                    ResidentRecordRefV1::parse("unrelated-operation-handle")
                        .expect("unrelated handle reference is valid"),
                    expected_locator.record_sha256().clone(),
                );
            }
            Some(CancellationSelectionMismatch::LocatorDigest) => {
                expected_locator = ControlRecordCoordinate::new(
                    expected_locator.record_ref().clone(),
                    pale_ports::ControlRecordDigest::parse("0".repeat(64))
                        .expect("unrelated handle digest is valid"),
                );
            }
            None => {}
        }
        let selection_checks = Rc::new(Cell::new(0));
        let release_calls = Rc::new(Cell::new(0));
        let input_lends = Rc::new(Cell::new(0));
        let observation_signs = Rc::new(Cell::new(0));
        let checkpoint_signs = Rc::new(Cell::new(0));
        let anchor_advances = Rc::new(Cell::new(0));
        let mut anchor = CancellationMutationAnchor {
            inner: anchor,
            advances: Rc::clone(&anchor_advances),
        };
        let application_store = MemoryAuthorityStore::default();
        let mut application = ready_application(&application_store, store.clone(), 111);
        let (published, advertised) = active_predecessors(current.generation.compiled_generation);
        let mut records = RecoveryResolver::default();
        records.insert_generation(current.generation.compiled_generation);
        records.insert_effective_profile(current.generation.effective_profile);
        records.insert_published(&published);
        records.insert_advertised(&advertised);
        records.insert_fence(current.generation.activation_fence);
        records.insert_active(current.generation.active_head);
        let historical = Historical {
            records,
            session_signing_authority: signing_authority.clone(),
        };
        application.fixture_retain_generation(generation);
        let _ = application
            .publish(published)
            .expect("publish Cancel generation");
        let _ = application
            .advertise(advertised)
            .expect("advertise Cancel generation");
        let _ = application
            .activate(
                current.generation.active_head.clone(),
                current.generation.activation_fence.clone(),
            )
            .expect("activate Cancel generation");
        let (retained_session, _) = application
            .admit_session(
                SessionAdmissionLeaseWireV1::from(session.lease()),
                current,
                &mut ProvingAuthority,
            )
            .expect("admit exact Cancel session");
        assert_eq!(retained_session.lease(), session.lease());
        let process = ResidentProcessHost::new(
            unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
            vec![unproved_deployment_for_selector(
                &selector,
                store.clone(),
                application,
            )],
            std::num::NonZeroUsize::new(1).expect("one deployment"),
            ResidentOperationHost::new(task_state()),
        )
        .expect("one exact Cancel deployment");
        process.deployments[0]
            .application
            .recover_exact_retained_session(
                session.lease().session_admission_id(),
                session.lease().session_admission_lease_digest(),
                &historical,
            )
            .expect("recover exact retained Cancel session");

        let current_admission = ResidentProgramCurrentRequestAdmission::new(
            1 << 20,
            vec![ResidentProgramRequestPolicy::new(
                ResidentOperationRequestKind::Cancel,
                policy_from(
                    &request,
                    session.lease().compiled_generation_digest().clone(),
                ),
            )],
            EvidencePort {
                values: Arc::new(Mutex::new(VecDeque::from([Ok(exact_evidence)]))),
                reads: Arc::new(Mutex::new(0)),
            },
            Clock {
                values: Arc::new(Mutex::new(VecDeque::from([now]))),
                reads: Arc::new(Mutex::new(0)),
            },
            &historical,
            ProvingAuthority,
            Signature(true),
        )
        .expect("one exact Cancel policy");
        let crate::resident::operation_service::ResidentOperationCancellationInput {
            recovery,
            request: _,
            authorized_at: _,
            request_ref,
            observation_ref,
            signing,
            checkpoint,
            checkpoint_issued_at,
            checkpoint_schema,
            checkpoint_authority: _,
            accumulator_limits,
        } = input;
        let crate::resident::operation_service::ResidentOperationCancellationAdmission {
            recovery: recovery_admission,
            authorize,
            resolve_store,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        } = admission;
        let recovery = CancellationRecovery {
            expected_deployment,
            expected_request_id,
            expected_request_sha256,
            expected_locator,
            release_calls: Rc::clone(&release_calls),
            selection_checks: Rc::clone(&selection_checks),
            input_lends: Rc::clone(&input_lends),
            observation_signs: Rc::clone(&observation_signs),
            checkpoint_signs: Rc::clone(&checkpoint_signs),
            recovery,
            admission: Some(recovery_admission),
            request_ref: Some(request_ref),
            observation_ref: Some(observation_ref),
            signing,
            checkpoint: Some(checkpoint),
            checkpoint_issued_at,
            checkpoint_schema,
            accumulator_limits,
            local_completion: Some(local_completion),
            authorize,
            resolve_store,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        };
        let owner = ResidentProgramOperationTurnOwner::new(
            OperationOnlyPeer,
            current_admission,
            vec![ResidentProgramDeploymentOperationPorts::new(
                selector,
                store,
                custody,
                sessions,
                &mut anchor,
            )],
            FreshEntropy::new(Arc::new(AtomicUsize::new(0))),
            CountingAuthority::default(),
            StartCryptography {
                checkpoint: RefCell::new(None),
            },
            recovery,
            ResidentProgramConsultRecoveryUnavailable,
        )
        .expect("one exact retained Cancel owner");
        let program = ResidentProgram::fixture_from_process_with_authorities(
            process,
            vec![authority],
            ConsultPolicy(None),
            owner,
        );
        let mut service = crate::resident::ResidentFrameService::from_program(program);
        let mut transport = OperationFramePort::new(&frame);
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(2))
            .expect("test deadline");
        assert_eq!(
            service
                .serve_frame(&mut transport, deadline, deadline)
                .expect("Cancel reaches the exact existing cancellation owner"),
            crate::resident::ResidentFrameOutcome::Operation,
        );
        let expected_response = match expected {
            Ok(disposition) => ResidentOperationFrameResponse::CancellationDisposition {
                locator,
                disposition,
            },
            Err(failure) => ResidentOperationFrameResponse::Rejected { failure },
        };
        let response: ResidentOperationFrameResponse =
            serde_json::from_slice(&transport.sent).expect("one response decodes");
        assert_eq!(response, expected_response);
        if records_application_fixture {
            assert_or_update_application_cancel_fixture(&frame, &response);
        }
        assert_eq!(release_calls.get(), 1);
        assert_eq!(selection_checks.get(), 1);
        assert_eq!(input_lends.get(), usize::from(mismatch.is_none()));
        if mismatch.is_some() {
            assert_eq!(observation_signs.get(), 0);
            assert_eq!(checkpoint_signs.get(), 0);
            assert_eq!(anchor_advances.get(), 0);
        }
    });
}

#[test]
fn framed_cancel_appends_one_open_request_and_reports_terminal_completion() {
    assert_framed_cancel(
        "cancel-active-allowed",
        pale_ports::ResidentOperationFrameCancellationDisposition::LocallyCancelled,
    );
}

#[test]
fn framed_cancel_reports_an_existing_terminal_state_without_another_append() {
    assert_framed_cancel(
        "cancel-terminal-allowed",
        pale_ports::ResidentOperationFrameCancellationDisposition::TerminalAlreadyEstablished,
    );
}

#[test]
fn framed_cancel_preserves_replay_and_uncertainty_as_nonterminal_dispositions() {
    assert_framed_cancel(
        "cancel-already-requested",
        pale_ports::ResidentOperationFrameCancellationDisposition::AlreadyRequested,
    );
    assert_framed_cancel(
        "cancel-provider-dispatched",
        pale_ports::ResidentOperationFrameCancellationDisposition::DispatchOwnerRequired,
    );
    assert_framed_cancel(
        "cancel-reconciliation",
        pale_ports::ResidentOperationFrameCancellationDisposition::ReconciliationOnly,
    );
}

#[test]
fn framed_cancel_rejects_unlike_physical_input_selections_before_lending_inputs() {
    for mismatch in [
        CancellationSelectionMismatch::DeploymentBindingReference,
        CancellationSelectionMismatch::DeploymentBindingDigest,
        CancellationSelectionMismatch::RuntimeAuthorityReference,
        CancellationSelectionMismatch::RuntimeAuthorityDigest,
        CancellationSelectionMismatch::RequestId,
        CancellationSelectionMismatch::RequestDigest,
        CancellationSelectionMismatch::LocatorReference,
        CancellationSelectionMismatch::LocatorDigest,
    ] {
        assert_framed_cancel_selection(
            "cancel-active-allowed",
            Err(ResidentOperationFrameFailure::ConfigurationUnavailable),
            Some(mismatch),
        );
    }
}

#[test]
fn a2a_cancel_handoff_retains_the_exact_pending_route_before_returning() {
    let expected_key = crate::resident::task_owner::ResidentOperationTaskKey::new(
        crate::local_operation_start_fixture::store_coordinate(),
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse("a2a/cancel-pending")
                .expect("A2A cancellation route reference is valid"),
            record_sha256: ArtifactDigest::parse("a".repeat(64))
                .expect("A2A cancellation route digest is valid"),
        },
    );
    let locator =
        ResidentOperationFrameLocator::new("operation/a2a-cancel".to_owned(), "b".repeat(64));
    let mut retention = ProgramCancellationRetentionProbe::default();

    let outcome = crate::resident::process_host::program::settle_cancellation_handoff(
        &mut retention,
        (),
        crate::resident::process_host::program::ResidentProgramCancellationHandoff::A2aCancellation {
            pending: Box::new(ResidentProcessA2aDelegatePoll::Pending(expected_key.clone())),
            outcome: crate::resident::process_host::operation_request_host::ResidentOperationRequestOutcome::CancellationDisposition {
                locator: locator.clone(),
                disposition: pale_ports::ResidentOperationFrameCancellationDisposition::A2aResponseReadStopped,
            },
        },
    )
    .expect("retained A2A cancellation can return its bounded disposition");

    assert_eq!(retention.retention_calls, 1);
    assert_eq!(retention.release_calls, 0);
    match retention.a2a.take() {
        Some(ResidentProcessA2aDelegatePoll::Pending(actual_key)) => {
            assert_eq!(actual_key, expected_key);
        }
        _ => panic!("the exact pending A2A cancellation route must be retained"),
    }
    match outcome {
        crate::resident::process_host::operation_request_host::ResidentOperationRequestOutcome::CancellationDisposition {
            locator: actual_locator,
            disposition,
        } => {
            assert_eq!(actual_locator, locator);
            assert_eq!(
                disposition,
                pale_ports::ResidentOperationFrameCancellationDisposition::A2aResponseReadStopped
            );
        }
        _ => panic!("the retained A2A cancellation must return its original disposition"),
    }
}

#[test]
fn terminal_cancel_outcome_releases_its_reservation() {
    let mut retention = ProgramCancellationRetentionProbe::default();
    let locator =
        ResidentOperationFrameLocator::new("operation/cancel-terminal".to_owned(), "c".repeat(64));

    let outcome = crate::resident::process_host::program::settle_cancellation_handoff(
        &mut retention,
        (),
        crate::resident::process_host::program::ResidentProgramCancellationHandoff::Outcome(
            crate::resident::process_host::operation_request_host::ResidentOperationRequestOutcome::Pending(
                locator.clone(),
            ),
        ),
    )
    .expect("terminal cancellation outcome releases its reservation");

    assert_eq!(retention.release_calls, 1);
    assert_eq!(retention.retention_calls, 0);
    assert!(matches!(
        outcome,
        crate::resident::process_host::operation_request_host::ResidentOperationRequestOutcome::Pending(actual)
            if actual == locator
    ));
}

#[test]
fn unowned_cancellation_failure_releases_its_reservation() {
    let mut retention = ProgramCancellationRetentionProbe::default();

    let failure = match crate::resident::process_host::program::settle_cancellation_handoff(
        &mut retention,
        (),
        crate::resident::process_host::program::ResidentProgramCancellationHandoff::Failure(
            ResidentOperationFrameFailure::ResidentUnavailable,
        ),
    ) {
        Ok(_) => panic!("an unowned cancellation failure must not succeed"),
        Err(failure) => failure,
    };

    assert_eq!(failure, ResidentOperationFrameFailure::ResidentUnavailable);
    assert_eq!(retention.release_calls, 1);
    assert_eq!(retention.retention_calls, 0);
}

#[test]
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn framed_observe_uses_current_admission_and_the_existing_read_only_owner() {
    with_resident_program_observe_fixture(&mut |fixture| {
        let ResidentProgramObserveFixture {
            observation,
            session,
            current,
            generation,
            reconstruction: _,
        } = fixture;
        let crate::resident::operation_observation_custody::tests::local_recovery::ProcessHostObserveFixture {
            input,
            admission,
            signing_authority,
            custody,
            sessions,
            anchor,
        } = observation;
        let store = input.recovery.operation.context.store.clone();
        let locator = input.recovery.operation.expected_handle.clone();
        let request = input.request.clone();
        let frame = ResidentObserveFrameRequest::new(
            locator.record_ref().as_str().to_owned(),
            locator.record_sha256().as_str().to_owned(),
            pale_core::canon::to_canonical_string(&request).expect("canonical Observe request"),
        );
        let authority = ResidentProgramDeploymentAuthority::fixture_from_current(current)
            .expect("the Observe fixture retains complete deployment authority");
        let selector = authority.fixture_selector().clone();
        assert_eq!(selector, selector_from_signing_authority(signing_authority));

        let application_store = MemoryAuthorityStore::default();
        let mut application = ready_application(&application_store, store.clone(), 112);
        let published =
            crate::resident_fixture::published(current.generation.compiled_generation, 1, None);
        let advertised = crate::resident_fixture::advertised(&published, 2, None);
        let mut records = RecoveryResolver::default();
        records.insert_generation(current.generation.compiled_generation);
        records.insert_effective_profile(current.generation.effective_profile);
        records.insert_published(&published);
        records.insert_advertised(&advertised);
        records.insert_fence(current.generation.activation_fence);
        records.insert_active(current.generation.active_head);
        let historical = Historical {
            records,
            session_signing_authority: signing_authority.clone(),
        };
        application.fixture_retain_generation(generation);
        let _ = application
            .publish(published)
            .expect("publish Observe generation");
        let _ = application
            .advertise(advertised)
            .expect("advertise Observe generation");
        let _ = application
            .activate(
                current.generation.active_head.clone(),
                current.generation.activation_fence.clone(),
            )
            .expect("activate Observe generation");
        let (retained_session, _) = application
            .admit_session(
                SessionAdmissionLeaseWireV1::from(session.lease()),
                current,
                &mut ProvingAuthority,
            )
            .expect("admit exact Observe session");
        assert_eq!(retained_session.lease(), session.lease());
        let process = ResidentProcessHost::new(
            unproved_writer(selector.principal_id.clone(), vec![store.clone()]),
            vec![unproved_deployment_for_selector(
                &selector,
                store.clone(),
                application,
            )],
            std::num::NonZeroUsize::new(1).expect("one deployment"),
            ResidentOperationHost::new(task_state()),
        )
        .expect("one exact Observe deployment");
        process.deployments[0]
            .application
            .recover_exact_retained_session(
                session.lease().session_admission_id(),
                session.lease().session_admission_lease_digest(),
                &historical,
            )
            .expect("recover exact retained Observe session");

        let now = input.authorized_at;
        let exact_evidence = evidence_from(current);
        let current_admission = ResidentProgramCurrentRequestAdmission::new(
            1 << 20,
            vec![ResidentProgramRequestPolicy::observe(policy_from(
                &request,
                session.lease().compiled_generation_digest().clone(),
            ))],
            EvidencePort {
                values: Arc::new(Mutex::new(VecDeque::from([Ok(exact_evidence)]))),
                reads: Arc::new(Mutex::new(0)),
            },
            Clock {
                values: Arc::new(Mutex::new(VecDeque::from([now]))),
                reads: Arc::new(Mutex::new(0)),
            },
            &historical,
            ProvingAuthority,
            Signature(true),
        )
        .expect("one exact Observe policy");
        let LocalOperationObservationAdmission {
            recovery: recovery_admission,
            authorize,
        } = admission;
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_secs(2))
            .expect("test deadline");
        let recovery = ObserveRecovery {
            recovery: Some(input.recovery),
            admission: Some(recovery_admission),
            authorize_disclosure: Some(authorize),
            response_deadline: deadline,
        };
        let owner = ResidentProgramOperationTurnOwner::new(
            ObserveOnlyPeer,
            current_admission,
            vec![ResidentProgramDeploymentOperationPorts::new(
                selector, store, custody, sessions, anchor,
            )],
            FreshEntropy::new(Arc::new(AtomicUsize::new(0))),
            CountingAuthority::default(),
            StartCryptography {
                checkpoint: RefCell::new(None),
            },
            recovery,
            ResidentProgramConsultRecoveryUnavailable,
        )
        .expect("one exact retained Observe owner");
        let mut program = ResidentProgram::fixture_from_process_with_authorities(
            process,
            vec![authority],
            ConsultPolicy(None),
            owner,
        );
        let operation_locator = pale_ports::ResidentOperationFrameLocator::new(
            locator.record_ref().as_str().to_owned(),
            locator.record_sha256().as_str().to_owned(),
        );
        for operation_frame in [
            ResidentOperationFrameRequest::StartInvoke {
                signed_common_request: "{}".to_owned(),
            },
            ResidentOperationFrameRequest::AdvanceInvoke {
                locator: operation_locator.clone(),
                signed_common_request: "{}".to_owned(),
            },
            ResidentOperationFrameRequest::StartConsult {
                signed_common_request: "{}".to_owned(),
            },
            ResidentOperationFrameRequest::AdvanceConsult {
                locator: operation_locator.clone(),
                signed_common_request: "{}".to_owned(),
            },
            ResidentOperationFrameRequest::StartDelegate {
                parent: operation_locator.clone(),
                signed_common_request: "{}".to_owned(),
            },
            ResidentOperationFrameRequest::AdvanceDelegate {
                locator: operation_locator.clone(),
                signed_common_request: "{}".to_owned(),
            },
            ResidentOperationFrameRequest::Cancel {
                locator: operation_locator,
                signed_common_request: "{}".to_owned(),
            },
        ] {
            let mut denied = OperationFramePort::new(&operation_frame);
            assert!(matches!(
                program.serve_resident_frame(&mut denied, deadline, deadline),
                Err(crate::resident::process_host::resident_frame_ingress::ResidentFrameIngressError::Program(_))
            ));
            assert!(
                denied.sent.is_empty(),
                "peer denial must not disclose an operation response"
            );
        }
        let directory = tempfile::tempdir().expect("temporary resident socket directory");
        let endpoint = directory.path().join("resident.sock");
        let limits =
            LocalIpcFrameLimits::new(NonZeroU32::new(262_144).expect("positive frame limit"));
        let listener = UnixSocketListener::bind(&endpoint, limits).expect("bind resident socket");
        let request_bytes = serde_json::to_vec(&frame).expect("Observe frame encodes");
        let client = std::thread::spawn(move || {
            let mut carrier = UnixSocketCarrier::connect(&endpoint, limits, deadline)
                .expect("connect to resident service");
            carrier
                .send_frame(&request_bytes, deadline)
                .expect("send one SDK-compatible Observe frame");
            carrier
                .receive_frame(deadline)
                .expect("receive one resident response")
        });
        let mut transport = listener.accept(deadline).expect("accept one local client");
        let mut service = crate::resident::ResidentFrameService::from_program(program);
        assert_eq!(
            service
                .serve_frame(&mut transport, deadline, deadline)
                .expect("Observe reaches one framed response"),
            crate::resident::ResidentFrameOutcome::Observe,
        );
        let response: ResidentObserveFrameResponse =
            serde_json::from_slice(&client.join().expect("local client joins"))
                .expect("one Observe response decodes");
        let ResidentObserveFrameResponse::Observed {
            operation_handle_record_ref,
            ..
        } = response
        else {
            panic!("Observe did not return the authenticated current head")
        };
        assert_eq!(operation_handle_record_ref, locator.record_ref().as_str());
    });
}

#[test]
fn recovered_observe_service_reproduces_the_public_sdk_fixture() {
    crate::resident::deployment_finalization::tests::recovery::with_evidence_for_deployment(
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
                .expect("finalized deployment retains its runtime authority");
            let authorities = crate::session_admission_fixture::LocalStartExistingAuthorities {
                trust_ref: current_trust.coordinate().record_ref(),
                trust: current_trust.checkpoint(),
                binding_ref: runtime_authority.binding_ref(),
                binding: runtime_authority.binding(),
                runtime_ref: runtime_authority.record_ref(),
                runtime: runtime_authority.authority(),
            };
            let trust = crate::resident::ResidentServiceTrustFrontier::from_recovered_fixture(
                recovered_trust.clone(),
            );
            with_resident_program_observe_fixture_at_authorities(
                authorities,
                &mut move |fixture| {
                    let ResidentProgramObserveFixture {
                        observation,
                        session,
                        current,
                        generation,
                        reconstruction,
                    } = fixture;
                    let crate::resident::operation_observation_custody::tests::local_recovery::ProcessHostObserveFixture {
                    input,
                    admission,
                    signing_authority,
                    custody,
                    sessions: _,
                    anchor,
                } = observation;
                    let store = input.recovery.operation.context.store.clone();
                    let locator = input.recovery.operation.expected_handle.clone();
                    let request = input.request.clone();
                    let authority =
                        ResidentProgramDeploymentAuthority::fixture_from_current(current)
                            .expect("Observe fixture retains deployment authority");
                    let selector = authority.fixture_selector().clone();
                    assert_eq!(selector, selector_from_signing_authority(signing_authority));

                    let request_preimage = request.to_preimage();
                    assert!(request_preimage.execution_owner_fence.is_none());
                    let policy = crate::resident::ResidentServicePolicyProjection::admit(
                        &generation,
                        crate::resident::ResidentServicePolicyInput {
                            repository_id: session.lease().repository_id().clone(),
                            body_schema_id: request_preimage.body.schema_id,
                            body_schema_sha256: request_preimage.body.schema_sha256,
                            caller: request_preimage.caller,
                            signing_authority: request_preimage.signing_authority_binding,
                            route: request_preimage.route,
                            execution_owner_fence: None,
                            federation_claim: request_preimage.federation_claim,
                            workspace_id: request_preimage.workspace_id,
                            capability_id: request_preimage.capability_id,
                            operation: request_preimage.operation,
                            scope: request_preimage.scope,
                            risk_class: request_preimage.risk_class,
                            resource_constraints: request_preimage.resource_constraints,
                            expected_precondition: request_preimage.expected_precondition,
                            maximum_request_lifetime_seconds: 12,
                            fence_token: request_preimage.fence_token,
                        },
                    )
                    .expect("Observe policy is admitted from the retained generation");

                    let application_store = MemoryAuthorityStore::default();
                    let mut application = ready_application(&application_store, store.clone(), 112);
                    let published = crate::resident_fixture::published(
                        current.generation.compiled_generation,
                        1,
                        None,
                    );
                    let advertised = crate::resident_fixture::advertised(&published, 2, None);
                    let mut records = RecoveryResolver::default();
                    records.insert_generation(current.generation.compiled_generation);
                    records.insert_effective_profile(current.generation.effective_profile);
                    records.insert_published(&published);
                    records.insert_advertised(&advertised);
                    records.insert_fence(current.generation.activation_fence);
                    records.insert_active(current.generation.active_head);
                    let historical = Historical {
                        records,
                        session_signing_authority: signing_authority.clone(),
                    };
                    application.fixture_retain_generation(generation);
                    let _ = application
                        .publish(published)
                        .expect("publish Observe generation");
                    let _ = application
                        .advertise(advertised)
                        .expect("advertise Observe generation");
                    let _ = application
                        .activate(
                            current.generation.active_head.clone(),
                            current.generation.activation_fence.clone(),
                        )
                        .expect("activate Observe generation");
                    let _ = application
                        .admit_session(
                            SessionAdmissionLeaseWireV1::from(session.lease()),
                            current,
                            &mut ProvingAuthority,
                        )
                        .expect("retain exact Observe session");
                    application.checkpoint().expect("checkpoint Observe state");
                    drop(application);

                    let mut source = FixedInputSource {
                        calls: 0,
                        inputs: vec![reconstruction],
                        requested: None,
                    };
                    let deadline = std::time::Instant::now()
                        .checked_add(std::time::Duration::from_secs(2))
                        .expect("test deadline");
                    let exact_evidence = service_evidence_from(current);
                    let LocalOperationObservationAdmission {
                        recovery: recovery_admission,
                        authorize: authorize_disclosure,
                    } = admission;
                    let authorized_at = input.authorized_at;
                    let actual_response = {
                        let writer = crate::local_operation_start_fixture::publication_writer();
                        let checkpoint = super::program::checkpoint_for_writer(&writer, 'c');
                        let snapshot = super::program::writer_snapshot(&writer, &checkpoint);
                        let mut events = UnavailableEventCustody;
                        let cold_open = ResidentColdOpenInput::new(
                            store.clone(),
                            limits(),
                            limits().maximum_entry_bytes(),
                            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                            &historical,
                            &mut source,
                            ResidentReconstructionParameters::new(
                                synthetic_set_policy(),
                                4,
                                4,
                                WakeEpochV1::new(203),
                            ),
                        );
                        let observe_inputs = ServiceObserveRecovery {
                            recovery: Some(input.recovery),
                            admission: Some(OperationControlRecoveryAdmission {
                                admit_request: &mut *recovery_admission.admit_request,
                                admit_handle_authority: &mut *recovery_admission
                                    .admit_handle_authority,
                                verify_handle_signature: &mut *recovery_admission
                                    .verify_handle_signature,
                                admit_checkpoint: &mut *recovery_admission.admit_checkpoint,
                                resolve_observation_signer: &mut *recovery_admission
                                    .resolve_observation_signer,
                                admit_observation_state: &mut *recovery_admission
                                    .admit_observation_state,
                                admit_rotation: &mut *recovery_admission.admit_rotation,
                                verify_observation_signature: &mut *recovery_admission
                                    .verify_observation_signature,
                                verify_checkpoint_signature: &mut *recovery_admission
                                    .verify_checkpoint_signature,
                            }),
                            authorize_disclosure: Some(&mut *authorize_disclosure),
                            response_deadline: deadline,
                        };
                        let process = crate::resident::ResidentServiceProcessInput {
                            held_writer: super::program::HeldWriter(
                                snapshot.binding().principal_id().clone(),
                            ),
                            writer_snapshot: &snapshot,
                            writer_admission: ServiceWriterAdmission { writer, checkpoint },
                            trust: &trust,
                            deployments: vec![crate::resident::ResidentServiceDeploymentOpen {
                                store,
                                durable: &application_store,
                                sessions: &application_store,
                                events: &mut events,
                                maximum_event_custody_bytes: 65_536,
                                cold_open,
                                operations: custody,
                                anti_rollback: anchor,
                            }],
                            maximum_ready_deployments: NonZeroUsize::new(1)
                                .expect("one deployment"),
                            task_limits: crate::resident::ResidentServiceTaskLimits {
                                maximum_provider_tasks: NonZeroUsize::new(1)
                                    .expect("one provider task"),
                                maximum_consultation_batches: NonZeroUsize::new(1)
                                    .expect("one consultation batch"),
                                maximum_aggregate_tasks: NonZeroUsize::new(1)
                                    .expect("one aggregate task"),
                            },
                        };
                        let owners = crate::resident::ResidentServiceObserveOwners {
                            peer_admission: Box::new(ExactTestPeer::new(
                                LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
                            )),
                            maximum_request_bytes: 1 << 20,
                            policies: vec![policy],
                            historical: &historical,
                            evidence: ServiceEvidencePort(VecDeque::from([Ok(exact_evidence)])),
                            clock: Clock {
                                values: Arc::new(Mutex::new(VecDeque::from([authorized_at]))),
                                reads: Arc::new(Mutex::new(0)),
                            },
                            session_authority: ProvingAuthority,
                            caller_signature: ServiceSignature,
                            operation_authority: ProvingAuthority,
                            observe_inputs,
                        };
                        let mut service =
                            crate::resident::ResidentFrameService::recover_observe(process, owners)
                                .expect("recover public Observe service");
                        let frame = ResidentObserveFrameRequest::new(
                            locator.record_ref().as_str().to_owned(),
                            locator.record_sha256().as_str().to_owned(),
                            pale_core::canon::to_canonical_string(&request)
                                .expect("canonical Observe request"),
                        );
                        let mut transport = OperationFramePort::new(&frame);
                        assert_eq!(
                            service
                                .serve_frame(&mut transport, deadline, deadline)
                                .expect("Observe reaches one framed response"),
                            crate::resident::ResidentFrameOutcome::Observe,
                        );
                        transport.sent
                    };
                    assert_eq!(source.calls, 1);
                    let actual_fixture = application_observe_fixture(&request, &actual_response);
                    assert_or_update_application_observe_fixture(&actual_fixture);
                },
            );
        },
    );
}

#[path = "program_native_start.rs"]
mod native_start;
