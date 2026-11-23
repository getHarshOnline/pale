//! SDK-side transport and response-admission models for resident tests.

use std::collections::VecDeque;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use pale_cli::command::{
    AuthenticatedResidentCarrier, AuthenticatedResidentCarrierFailure,
    AuthenticatedResidentCarrierSource, AuthenticatedResidentCarrierSourceFailure,
    ResidentResponseAdmission, ResidentResponseAdmissionFailure,
};
use pale_core::records::{
    ArtifactDigest, ResidentRecordRefV1, OPERATION_HANDLE_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};

use crate::{BoundedFrameClient, BoundedFrameClientError, PendingResidentExchange};

const TEST_SIGNATURE: &str =
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

/// Channel carrier that hands each frame to one persistent in-memory service.
#[derive(Clone)]
pub struct InMemoryResidentCarrier {
    client: BoundedFrameClient,
}

impl AuthenticatedResidentCarrierSource for InMemoryResidentCarrier {
    fn open(
        &mut self,
        _deadline: Instant,
    ) -> Result<Box<dyn AuthenticatedResidentCarrier>, AuthenticatedResidentCarrierSourceFailure>
    {
        Ok(Box::new(self.clone()))
    }
}

impl InMemoryResidentCarrier {
    /// Create a bounded carrier and its application-side receiver.
    #[must_use]
    pub fn bounded(capacity: usize) -> (Self, Receiver<PendingResidentExchange>, Arc<AtomicUsize>) {
        let (client, receiver) = BoundedFrameClient::new(capacity);
        let calls = client.handoff_counter();
        (Self { client }, receiver, calls)
    }
}

impl AuthenticatedResidentCarrier for InMemoryResidentCarrier {
    fn exchange(
        &mut self,
        request: &[u8],
        deadline: Instant,
    ) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure> {
        self.client
            .exchange(request, deadline)
            .map_err(|error| match error {
                BoundedFrameClientError::NotSent => AuthenticatedResidentCarrierFailure::NotSent,
                BoundedFrameClientError::OutcomeUnknown => {
                    AuthenticatedResidentCarrierFailure::OutcomeUnknown
                }
                BoundedFrameClientError::ServiceUnavailable => {
                    AuthenticatedResidentCarrierFailure::ConnectionUnavailable
                }
            })
    }
}

/// Exact resident response retained by the deterministic admission model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedResidentResponse {
    /// Exact operation-handle reference.
    pub handle_ref: String,
    /// Exact canonical operation-handle bytes.
    pub handle_bytes: Vec<u8>,
    /// Exact operation-observation reference.
    pub observation_ref: String,
    /// Exact canonical operation-observation bytes.
    pub observation_bytes: Vec<u8>,
    /// Monotone observation revision.
    pub revision: u64,
}

#[derive(Default)]
struct AdmissionState {
    calls: usize,
    expected: VecDeque<AdmittedResidentResponse>,
    last: Option<AdmittedResidentResponse>,
}

/// Strict synthetic authority and anti-rollback admission for interface tests.
///
/// The verifier recognizes only the deterministic signature used by Pale's
/// application fixtures. It reproduces both canonical digests, joins the
/// observation to its exact handle, compares signer coordinates, and moves one
/// in-memory revision frontier atomically. It is not a production key,
/// historical authority resolver, or independent durable anchor.
#[derive(Default)]
pub struct DeterministicResponseAdmission {
    state: Mutex<AdmissionState>,
}

impl DeterministicResponseAdmission {
    /// Register one application-produced pair before the SDK receives it.
    ///
    /// # Errors
    ///
    /// Returns an error when either canonical record, digest, signature, join,
    /// or monotone revision is invalid.
    pub fn expect(
        &self,
        operation_handle_record_ref: &str,
        operation_handle_canonical_bytes: &[u8],
        operation_observation_record_ref: &str,
        operation_observation_canonical_bytes: &[u8],
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        let response = parse_application_response(
            operation_handle_record_ref,
            operation_handle_canonical_bytes,
            operation_observation_record_ref,
            operation_observation_canonical_bytes,
        )?;
        self.state
            .lock()
            .map_err(|_| ResidentResponseAdmissionFailure)?
            .expected
            .push_back(response);
        Ok(())
    }

    /// Return how many response pairs crossed authority admission.
    #[must_use]
    pub fn calls(&self) -> usize {
        self.state
            .lock()
            .expect("response-admission state remains available")
            .calls
    }

    /// Clone the last admitted response pair.
    #[must_use]
    pub fn last(&self) -> Option<AdmittedResidentResponse> {
        self.state
            .lock()
            .expect("response-admission state remains available")
            .last
            .clone()
    }
}

impl ResidentResponseAdmission for DeterministicResponseAdmission {
    fn admit(
        &self,
        operation_handle_record_ref: &str,
        operation_handle_canonical_bytes: &[u8],
        operation_observation_record_ref: &str,
        operation_observation_canonical_bytes: &[u8],
        deadline: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        if Instant::now() >= deadline {
            return Err(ResidentResponseAdmissionFailure);
        }
        let admitted = parse_application_response(
            operation_handle_record_ref,
            operation_handle_canonical_bytes,
            operation_observation_record_ref,
            operation_observation_canonical_bytes,
        )?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| ResidentResponseAdmissionFailure)?;
        if state.expected.front() != Some(&admitted) {
            return Err(ResidentResponseAdmissionFailure);
        }
        if let Some(previous) = state.last.as_ref() {
            let exact_replay = previous == &admitted;
            let successor = previous.handle_ref == admitted.handle_ref
                && previous.handle_bytes == admitted.handle_bytes
                && previous.revision < admitted.revision;
            if !exact_replay && !successor {
                return Err(ResidentResponseAdmissionFailure);
            }
        }
        let _ = state.expected.pop_front();
        state.calls += 1;
        state.last = Some(admitted);
        Ok(())
    }
}

fn parse_application_response(
    operation_handle_record_ref: &str,
    operation_handle_canonical_bytes: &[u8],
    operation_observation_record_ref: &str,
    operation_observation_canonical_bytes: &[u8],
) -> Result<AdmittedResidentResponse, ResidentResponseAdmissionFailure> {
    ResidentRecordRefV1::parse(operation_handle_record_ref)
        .map_err(|_| ResidentResponseAdmissionFailure)?;
    ResidentRecordRefV1::parse(operation_observation_record_ref)
        .map_err(|_| ResidentResponseAdmissionFailure)?;

    let handle = admitted_record(
        operation_handle_canonical_bytes,
        OPERATION_HANDLE_V1_SCHEMA_ID,
        "canonical_operation_handle_sha256",
    )?;
    let observation = admitted_record(
        operation_observation_canonical_bytes,
        OPERATION_OBSERVATION_V1_SCHEMA_ID,
        "canonical_operation_observation_sha256",
    )?;
    if handle.signature != TEST_SIGNATURE || observation.signature != TEST_SIGNATURE {
        return Err(ResidentResponseAdmissionFailure);
    }
    for field in [
        "deployment_runtime_authority_ref",
        "deployment_runtime_authority_sha256",
        "issuer_principal_id",
        "issuer_key_id",
        "issuer_key_epoch",
    ] {
        if handle.value.get(field) != observation.value.get(field) {
            return Err(ResidentResponseAdmissionFailure);
        }
    }
    if observation.value.get("operation_handle_ref")
        != Some(&serde_json::Value::String(
            operation_handle_record_ref.to_owned(),
        ))
        || observation.value.get("operation_handle_sha256")
            != Some(&serde_json::Value::String(handle.digest.clone()))
        || observation.value.get("operation_handle_id") != handle.value.get("operation_handle_id")
    {
        return Err(ResidentResponseAdmissionFailure);
    }
    let revision = observation
        .value
        .get("revision")
        .and_then(serde_json::Value::as_u64)
        .filter(|revision| *revision > 0)
        .ok_or(ResidentResponseAdmissionFailure)?;
    Ok(AdmittedResidentResponse {
        handle_ref: operation_handle_record_ref.to_owned(),
        handle_bytes: operation_handle_canonical_bytes.to_vec(),
        observation_ref: operation_observation_record_ref.to_owned(),
        observation_bytes: operation_observation_canonical_bytes.to_vec(),
        revision,
    })
}

struct ParsedRecord {
    value: serde_json::Value,
    digest: String,
    signature: String,
}

fn admitted_record(
    bytes: &[u8],
    expected_schema_id: &str,
    digest_field: &str,
) -> Result<ParsedRecord, ResidentResponseAdmissionFailure> {
    let mut value =
        pale_core::canon::parse_strict(bytes).map_err(|_| ResidentResponseAdmissionFailure)?;
    let canonical = pale_core::canon::to_canonical_string(&value)
        .map_err(|_| ResidentResponseAdmissionFailure)?;
    if canonical.as_bytes() != bytes {
        return Err(ResidentResponseAdmissionFailure);
    }
    let descriptor = pale_contracts::schema_descriptor(expected_schema_id)
        .map_err(|_| ResidentResponseAdmissionFailure)?;
    if value.get("schema_id").and_then(serde_json::Value::as_str) != descriptor.wire_schema_id
        || value
            .get("schema_sha256")
            .and_then(serde_json::Value::as_str)
            != Some(descriptor.schema_sha256)
    {
        return Err(ResidentResponseAdmissionFailure);
    }
    let object = value
        .as_object_mut()
        .ok_or(ResidentResponseAdmissionFailure)?;
    let digest = object
        .remove(digest_field)
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or(ResidentResponseAdmissionFailure)?;
    let signature = object
        .remove("signature")
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or(ResidentResponseAdmissionFailure)?;
    let preimage = pale_core::canon::to_canonical_string(&value)
        .map_err(|_| ResidentResponseAdmissionFailure)?;
    if ArtifactDigest::hash_bytes(preimage.as_bytes()).as_str() != digest {
        return Err(ResidentResponseAdmissionFailure);
    }
    Ok(ParsedRecord {
        value,
        digest,
        signature,
    })
}
