//! Behavioral laws for private operation-start identity allocation.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

use pale_core::records::{
    BoundedResidentTextV1, CommonRequestEnvelopeV1, ControlIncarnationIdV1, SchemaId,
    COMMON_REQUEST_ENVELOPE_V1_SCHEMA, CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID, OPERATION_HANDLE_V1_SCHEMA,
    OPERATION_HANDLE_V1_SCHEMA_ID, OPERATION_OBSERVATION_V1_SCHEMA,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlRecordDigest, CryptographicRandomSourceError, CryptographicRandomSourcePort,
    OperationCustodyIndexKey, OperationCustodyIndexObservation, OperationCustodyIndexQuery,
};

use crate::local_operation_start_fixture::{record_ref, with_invoke_start_input};
use crate::resident::operation_custody::operation_request_key;
use crate::resident::operation_observation_custody::tests::{
    local_start::source_generation, RecordingStore,
};

fn request(
    suffix: &str,
    change: impl FnOnce(&mut pale_core::records::CommonRequestEnvelopePreimageV1),
) -> CommonRequestEnvelopeV1 {
    let (generation, profile) = source_generation();
    with_invoke_start_input(&generation, &profile, suffix, change, |_, request, _, _| {
        request.clone()
    })
}

fn store_coordinate() -> AuthorityStoreCoordinate {
    crate::local_operation_start_fixture::store_coordinate()
}

fn role_record(reference: &str, schema: &str, schema_id: &str, digest_byte: char) -> ControlRecord {
    let identity = pale_contracts::record_schema_identity(schema_id).expect("registered schema");
    ControlRecord::new(
        ControlRecordCoordinate::new(
            record_ref(reference),
            ControlRecordDigest::parse(std::iter::repeat_n(digest_byte, 64).collect::<String>())
                .expect("record digest"),
        ),
        SchemaId::parse(schema).expect("schema"),
        identity.schema_sha256,
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes())
            .expect("record bytes"),
    )
}

pub(in crate::resident) fn absent_reads(
    request: &CommonRequestEnvelopeV1,
) -> Vec<OperationCustodyIndexObservation> {
    let key = operation_request_key(request).expect("request key");
    vec![
        OperationCustodyIndexObservation::absent(OperationCustodyIndexQuery::RequestOperation(
            key.clone(),
        )),
        OperationCustodyIndexObservation::absent(OperationCustodyIndexQuery::RequestEvidence(key)),
    ]
}

pub(in crate::resident) fn existing_reads(
    request: &CommonRequestEnvelopeV1,
) -> Vec<OperationCustodyIndexObservation> {
    let key = operation_request_key(request).expect("request key");
    let request_record =
        crate::resident::record_projection::request_record(&record_ref("stored/request"), request)
            .expect("request projection");
    let handle = role_record(
        "stored/handle",
        OPERATION_HANDLE_V1_SCHEMA,
        OPERATION_HANDLE_V1_SCHEMA_ID,
        '2',
    );
    let observation = role_record(
        "stored/observation",
        OPERATION_OBSERVATION_V1_SCHEMA,
        OPERATION_OBSERVATION_V1_SCHEMA_ID,
        '3',
    );
    let checkpoint = role_record(
        "stored/checkpoint",
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
        '4',
    );
    vec![
        OperationCustodyIndexObservation::exact(
            OperationCustodyIndexQuery::RequestOperation(key.clone()),
            handle,
        )
        .expect("handle index"),
        OperationCustodyIndexObservation::exact(
            OperationCustodyIndexQuery::RequestEvidence(key.clone()),
            request_record,
        )
        .expect("request index"),
        OperationCustodyIndexObservation::exact(
            OperationCustodyIndexQuery::StartObservation(key),
            observation.clone(),
        )
        .expect("initial observation index"),
        OperationCustodyIndexObservation::exact(
            OperationCustodyIndexQuery::ObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(observation.coordinate()),
            ),
            checkpoint,
        )
        .expect("checkpoint index"),
    ]
}

fn existing_store(request: &CommonRequestEnvelopeV1) -> RecordingStore {
    RecordingStore::recovering(existing_reads(request))
}

#[derive(Default)]
struct IncrementingRandom {
    calls: AtomicUsize,
}

impl IncrementingRandom {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl CryptographicRandomSourcePort for IncrementingRandom {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        let value = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        destination.fill(u8::try_from(value).expect("bounded test draw"));
        Ok(())
    }
}

#[derive(Default)]
struct ConstantRandom {
    calls: AtomicUsize,
}

impl CryptographicRandomSourcePort for ConstantRandom {
    fn fill_bytes(&self, destination: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        destination.fill(7);
        Ok(())
    }
}

#[derive(Default)]
struct FailingRandom {
    calls: AtomicUsize,
}

impl CryptographicRandomSourcePort for FailingRandom {
    fn fill_bytes(&self, _: &mut [u8]) -> Result<(), CryptographicRandomSourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(CryptographicRandomSourceError::unavailable())
    }
}

fn fresh(
    outcome: OperationStartAllocation,
) -> crate::resident::operation_start_allocation::FreshOperationStartAllocation {
    match outcome {
        OperationStartAllocation::Fresh(fresh) => *fresh,
        OperationStartAllocation::Existing(_) => panic!("fixture request must be absent"),
    }
}

fn expected_spelling(prefix: &str, domain: &[u8], nonce_byte: u8) -> String {
    let mut preimage = Vec::with_capacity(domain.len() + RANDOM_BYTES);
    preimage.extend_from_slice(domain);
    preimage.extend_from_slice(&[nonce_byte; RANDOM_BYTES]);
    format!("{prefix}:{}", pale_core::sha256_hex(&preimage))
}

#[test]
fn absent_request_allocates_each_type_in_its_own_domain() {
    let request = request("allocated-domains", |_| {});
    let mut store =
        crate::resident::operation_observation_custody::tests::local_start::absent_start_store(
            &request,
        );
    let random = IncrementingRandom::default();
    let allocation = fresh(
        begin_operation_start_allocation(
            &mut store,
            &random,
            &store_coordinate(),
            &request,
            1 << 20,
        )
        .expect("fresh allocation"),
    );

    assert_eq!(random.calls(), 7);
    assert_eq!(allocation.store(), &store_coordinate());
    assert_eq!(
        allocation.request_ref().as_str(),
        format!(
            "operation-request-record:v1:{}",
            request.canonical_request_sha256().as_str()
        )
    );
    assert!(allocation
        .lease_id()
        .as_str()
        .starts_with("operation-lease:v1:"));
    assert!(allocation
        .lease_ref()
        .as_str()
        .starts_with("operation-lease-record:v1:"));
    assert!(allocation
        .handle_id()
        .as_str()
        .starts_with("operation-handle:v1:"));
    assert!(allocation
        .handle_ref()
        .as_str()
        .starts_with("operation-handle-record:v1:"));
    assert!(allocation
        .observation_ref()
        .as_str()
        .starts_with("operation-observation-record:v1:"));
    assert!(allocation
        .checkpoint_id()
        .as_str()
        .starts_with("operation-checkpoint:v1:"));
    assert!(allocation
        .checkpoint_ref()
        .as_str()
        .starts_with("operation-checkpoint-record:v1:"));
    assert_eq!(
        allocation.lease_id().as_str(),
        expected_spelling("operation-lease:v1", LEASE_ID_DOMAIN, 1)
    );
    assert_eq!(
        allocation.lease_ref().as_str(),
        expected_spelling("operation-lease-record:v1", LEASE_REF_DOMAIN, 2)
    );
    assert_eq!(
        allocation.handle_id().as_str(),
        expected_spelling("operation-handle:v1", HANDLE_ID_DOMAIN, 3)
    );
    assert_eq!(
        allocation.handle_ref().as_str(),
        expected_spelling("operation-handle-record:v1", HANDLE_REF_DOMAIN, 4)
    );
    assert_eq!(
        allocation.observation_ref().as_str(),
        expected_spelling("operation-observation-record:v1", OBSERVATION_REF_DOMAIN, 5,)
    );
    assert_eq!(
        allocation.checkpoint_id().as_str(),
        expected_spelling("operation-checkpoint:v1", CHECKPOINT_ID_DOMAIN, 6)
    );
    assert_eq!(
        allocation.checkpoint_ref().as_str(),
        expected_spelling("operation-checkpoint-record:v1", CHECKPOINT_REF_DOMAIN, 7,)
    );
}

#[test]
fn exact_request_replay_returns_custody_without_entropy() {
    let request = request("allocated-replay", |_| {});
    let random = IncrementingRandom::default();
    let mut store = existing_store(&request);

    let outcome = begin_operation_start_allocation(
        &mut store,
        &random,
        &store_coordinate(),
        &request,
        1 << 20,
    )
    .expect("exact request replay");

    let OperationStartAllocation::Existing(existing) = outcome else {
        panic!("existing request must not allocate")
    };
    assert_eq!(existing.store(), &store_coordinate());
    assert_eq!(
        existing.request().schema_id().as_str(),
        COMMON_REQUEST_ENVELOPE_V1_SCHEMA
    );
    assert_eq!(random.calls(), 0);
}

#[test]
fn changed_request_bytes_conflict_before_entropy() {
    let original = request("allocated-conflict", |_| {});
    let changed = request("allocated-conflict", |request| {
        request.scope.purpose = BoundedResidentTextV1::parse("different purpose").expect("purpose");
    });
    assert_eq!(
        operation_request_key(&original).expect("original key"),
        operation_request_key(&changed).expect("changed key")
    );
    let mut store = existing_store(&original);
    let random = IncrementingRandom::default();

    assert!(matches!(
        begin_operation_start_allocation(
            &mut store,
            &random,
            &store_coordinate(),
            &changed,
            1 << 20,
        ),
        Err(OperationStartAllocationError::Recovery(
            OperationRecoveryError::RequestConflict
        ))
    ));
    assert_eq!(random.calls(), 0);
}

#[test]
fn repeated_entropy_is_bounded_and_fails_closed() {
    let request = request("allocated-duplicate-entropy", |_| {});
    let mut store =
        crate::resident::operation_observation_custody::tests::local_start::absent_start_store(
            &request,
        );
    let random = ConstantRandom::default();

    assert!(matches!(
        begin_operation_start_allocation(
            &mut store,
            &random,
            &store_coordinate(),
            &request,
            1 << 20,
        ),
        Err(OperationStartAllocationError::RandomDrawsExhausted)
    ));
    assert_eq!(random.calls.load(Ordering::SeqCst), MAXIMUM_RANDOM_DRAWS);
    assert_eq!(store.mutation_counts(), (0, 0));
}

#[test]
fn entropy_failure_returns_no_partial_bundle_or_mutation() {
    let request = request("allocated-entropy-failure", |_| {});
    let mut store =
        crate::resident::operation_observation_custody::tests::local_start::absent_start_store(
            &request,
        );
    let random = FailingRandom::default();

    assert!(matches!(
        begin_operation_start_allocation(
            &mut store,
            &random,
            &store_coordinate(),
            &request,
            1 << 20,
        ),
        Err(OperationStartAllocationError::Random(_))
    ));
    assert_eq!(random.calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.mutation_counts(), (0, 0));
}

#[test]
fn raw_spelling_reuse_remains_scoped_to_the_store_incarnation() {
    let request = request("allocated-incarnation", |_| {});
    let first_store = store_coordinate();
    let second_store = AuthorityStoreCoordinate::new(
        first_store.store_id().clone(),
        ControlIncarnationIdV1::parse("incarnation-2").expect("second incarnation"),
    );
    let allocate = |coordinate: &AuthorityStoreCoordinate| {
        allocate_fresh_operation_start(&IncrementingRandom::default(), coordinate, &request)
            .expect("fresh allocation")
    };
    let first = allocate(&first_store);
    let second = allocate(&second_store);

    assert_eq!(first.handle_id(), second.handle_id());
    assert_ne!(first.store(), second.store());
}

#[test]
fn request_reference_is_deterministic_without_collapsing_request_identity() {
    let original = request("allocated-request-reference", |_| {});
    let changed = request("allocated-request-reference", |request| {
        request.scope.purpose = BoundedResidentTextV1::parse("changed meaning").expect("purpose");
    });
    let allocate = |request: &CommonRequestEnvelopeV1| {
        let mut store =
            crate::resident::operation_observation_custody::tests::local_start::absent_start_store(
                request,
            );
        fresh(
            begin_operation_start_allocation(
                &mut store,
                &IncrementingRandom::default(),
                &store_coordinate(),
                request,
                1 << 20,
            )
            .expect("fresh allocation"),
        )
    };
    let first = allocate(&original);
    let repeated = allocate(&original);
    let unlike = allocate(&changed);

    assert_eq!(first.request_ref(), repeated.request_ref());
    assert_ne!(first.request_ref(), unlike.request_ref());
    assert_eq!(
        original.request_id(),
        changed.request_id(),
        "record-reference derivation must not redefine the caller's request identity"
    );
}
