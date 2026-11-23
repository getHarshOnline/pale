//! Behavioral laws for typed operation-observation custody.

#[path = "operation_observation_custody/local_recovery.rs"]
pub(in crate::resident) mod local_recovery;
#[path = "operation_observation_custody/local_start.rs"]
pub(in crate::resident) mod local_start;

#[path = "operation_observation_custody/checkpoint_leaf.rs"]
mod checkpoint_leaf;

#[path = "operation_observation_custody/checkpoint_preparation.rs"]
pub(in crate::resident) mod checkpoint_preparation;

#[path = "operation_observation_custody/a2a_exchange.rs"]
pub(in crate::resident) mod a2a_exchange;

#[path = "operation_observation_custody/dispatch_custody.rs"]
pub(in crate::resident) mod dispatch_custody;

#[path = "operation_observation_custody/native_execution.rs"]
pub(in crate::resident) mod native_execution;

#[path = "operation_observation_custody/reconciliation_append.rs"]
mod reconciliation_append;

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use pale_core::records::*;
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorErrorKind, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    AuthorityStoreCoordinate, ControlDomainAntiRollbackAnchor, ControlRecordCoordinate,
    ControlStoreError, ControlStoreErrorKind, OperationCustodyCandidate,
    OperationCustodyCommitOutcome, OperationCustodyPort, OperationCustodyReadBackOutcome,
    PreparedAntiRollbackAnchorTransition,
};

use super::*;
use crate::local_operation_start_fixture::{
    artifact_digest, checkpoint_authority, checkpoint_preimage, issue_observation, record_ref,
    schema_digest, sha, signature, store_coordinate,
};
use crate::resident::journal::tests::session_lease;
use crate::resident::lifecycle::tests::{compiled_generation, initial_active};
use crate::resident::{
    DatabaseCommittedOperationObservationAnchorPending, OperationCustodyAnchorEvidence,
    OperationCustodyAnchorFailureReason, OperationCustodyDatabaseEvidence,
    OperationCustodyDatabaseFailureReason, OperationObservationCustodyCommitFailure,
    OperationObservationCustodyCommitFailureReason,
};

fn evidence(value: &str, byte: char) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record_ref(value),
        record_sha256: artifact_digest(byte),
    }
}

pub(in crate::resident) fn signing_authority() -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        ResidentKeyIdV1::parse("key-1").expect("key"),
        SigningKeyEpochV1::new(1).expect("key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact_digest('1'),
            deployment_binding_ref: record_ref("deployment-binding"),
            deployment_binding_sha256: artifact_digest('2'),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: artifact_digest('3'),
        },
    )
    .expect("signing authority")
}

fn request_binding() -> RequestBindingV1 {
    RequestBindingV1::new(RequestBindingPreimageV1 {
        request_id: ResidentRequestIdV1::parse("request-1").expect("request"),
        correlation_id: ResidentCorrelationIdV1::parse("correlation-1").expect("correlation"),
        capability_id: CapabilityIdV1::parse("operation.observe").expect("capability"),
        canonical_request_sha256: CommonRequestEnvelopeDigest::parse(sha('4'))
            .expect("request digest"),
        session_grant_id: SessionGrantIdV1::parse("grant-1").expect("grant"),
        session_grant_sha256: artifact_digest('5'),
        deployment_binding_ref: record_ref("deployment-binding"),
        deployment_binding_sha256: artifact_digest('2'),
        route_binding_schema_id: SchemaId::parse("pale/local-route-binding.v1")
            .expect("route schema"),
        route_binding_schema_sha256: schema_digest("local-route-binding.v1"),
        route_binding_ref: record_ref("route-binding"),
        route_binding_sha256: artifact_digest('6'),
        route_generation: RouteGenerationV1::new(1).expect("route generation"),
        contract_set_ref: record_ref("contract-set"),
        contract_set_sha256: artifact_digest('7'),
        execution_owner_fence_ref: None,
        execution_owner_fence_sha256: None,
        federation_request_admission_ref: None,
        federation_request_admission_sha256: None,
    })
    .expect("request binding")
}

pub(in crate::resident) struct OperationFixture {
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    predecessor_ref: ResidentRecordRefV1,
    predecessor: OperationObservationV1,
    successor_ref: ResidentRecordRefV1,
    successor: OperationObservationV1,
}

impl OperationFixture {
    pub(in crate::resident) fn exact() -> Self {
        Self::named("1")
    }

    pub(in crate::resident) fn handle(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        (&self.handle_ref, &self.handle)
    }

    pub(in crate::resident) fn predecessor(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.predecessor_ref, &self.predecessor)
    }

    pub(in crate::resident) fn successor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.successor_ref, &self.successor)
    }

    fn named(suffix: &str) -> Self {
        Self::with_handle_identity(suffix, &format!("operation-{suffix}"), 120)
    }

    fn with_handle_identity(suffix: &str, handle_id: &str, retain_until: u64) -> Self {
        Self::with_binding(suffix, handle_id, retain_until, request_binding())
    }

    fn with_binding(
        suffix: &str,
        handle_id: &str,
        retain_until: u64,
        binding: RequestBindingV1,
    ) -> Self {
        let (handle_ref, handle, authority) = issue_handle(
            suffix,
            handle_id,
            retain_until,
            binding,
            OperationClassV1::Invoke,
        );
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let predecessor_ref = record_ref(format!("operation/observation-{suffix}-1").as_str());
        let predecessor = issue_observation(
            &authority,
            &handle_ref,
            &handle,
            observation_schema.clone(),
            OperationObservationRevisionV1::new(1).expect("revision"),
            None,
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving,
            },
            30,
        );
        let successor_ref = record_ref(format!("operation/observation-{suffix}-2").as_str());
        let successor = issue_observation(
            &authority,
            &handle_ref,
            &handle,
            observation_schema,
            OperationObservationRevisionV1::new(2).expect("revision"),
            Some((&predecessor_ref, &predecessor)),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ContextBound {
                    evidence: evidence("context/bound", 'a'),
                },
            },
            31,
        );
        Self {
            handle_ref,
            handle,
            predecessor_ref,
            predecessor,
            successor_ref,
            successor,
        }
    }

    fn transition(&self) -> OperationObservationTransition<'_> {
        OperationObservationTransition::new(
            &self.handle_ref,
            &self.handle,
            &self.predecessor_ref,
            &self.predecessor,
            &self.successor_ref,
            &self.successor,
        )
    }
}

pub(in crate::resident) fn operation_publication_case(
    suffix: &str,
    operation_class: OperationClassV1,
    state: OperationObservationStateV1,
) -> (
    (ResidentRecordRefV1, OperationHandleV1),
    (ResidentRecordRefV1, OperationObservationV1),
) {
    let (handle_ref, handle, authority) = issue_handle(
        suffix,
        &format!("operation-{suffix}"),
        120,
        request_binding(),
        operation_class,
    );
    let observation_ref = record_ref(&format!("operation/observation-{suffix}"));
    let observation = issue_observation(
        &authority,
        &handle_ref,
        &handle,
        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        OperationObservationRevisionV1::new(1).expect("revision"),
        None,
        state,
        30,
    );
    ((handle_ref, handle), (observation_ref, observation))
}

fn issue_handle(
    suffix: &str,
    handle_id: &str,
    retain_until: u64,
    binding: RequestBindingV1,
    operation_class: OperationClassV1,
) -> (
    ResidentRecordRefV1,
    OperationHandleV1,
    SigningAuthorityBindingV1,
) {
    let generation = compiled_generation('8');
    let (_, _, _, fence, active) = initial_active(&generation);
    let session = session_lease(&generation, &active, &fence);
    let lease = OperationLeaseV1::seal(
        schema_digest(OPERATION_LEASE_V1_SCHEMA_ID),
        &session,
        OperationLeaseRequestV1 {
            operation_lease_id: OperationLeaseIdV1::parse("lease-1").expect("lease"),
            operation: PackageResourceRefV1 {
                package_id: PackageIdV1::parse("operations").expect("package"),
                resource_id: PackageResourceIdV1::parse("observe").expect("operation"),
            },
            capabilities: vec![CapabilityIdV1::parse("read").expect("capability")],
            effects: vec![EffectClass::Read],
            read_set: vec![record_ref("read-a")],
            evidence_obligations: vec![record_ref("evidence-a")],
            budget_units: LeaseBudgetUnitsV1::new(5).expect("budget"),
            timeout_millis: LeaseTimeoutMillisV1::new(500).expect("timeout"),
            expires_at: UnixTimeV1::new(90).expect("expiry"),
            invocation_correlation: InvocationCorrelationV1::Unobserved {
                reason: ResidentReasonV1::parse("unavailable").expect("reason"),
            },
        },
    )
    .expect("operation lease");
    let authority = signing_authority();
    let handle_ref = record_ref(&format!("operation/handle-{suffix}"));
    let handle_schema = schema_digest(OPERATION_HANDLE_V1_SCHEMA_ID);
    let lease_ref = record_ref("operation/lease-1");
    let handle = OperationHandleV1::issue_contextual(
        OperationHandlePreimageV1 {
            schema_id: SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).expect("handle schema"),
            schema_sha256: handle_schema.clone(),
            operation_handle_id: OperationHandleIdV1::parse(handle_id).expect("operation"),
            operation_class,
            parent_operation: None,
            request_binding: binding.clone(),
            operation_lease_ref: lease_ref.clone(),
            operation_lease_sha256: lease.operation_lease_digest().clone(),
            compiled_generation_sha256: generation.compiled_generation_digest().clone(),
            activation_fence_sha256: fence.activation_fence_digest().clone(),
            instance_sha256: generation.instance_digest().clone(),
            created_at: UnixTimeV1::new(20).expect("created"),
            expires_at: UnixTimeV1::new(80).expect("expiry"),
            retain_until: UnixTimeV1::new(retain_until).expect("retention"),
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact_digest('1'),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("key epoch"),
        },
        &OperationHandleAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &handle_schema,
                signing_authority: &authority,
            },
            request_binding: &binding,
            operation_lease_ref: &lease_ref,
            operation_lease: &lease,
            parent_operation: None,
            compiled_generation_sha256: generation.compiled_generation_digest(),
            activation_fence_sha256: fence.activation_fence_digest(),
            instance_sha256: generation.instance_digest(),
        },
        |_, _| Ok(()),
        |_, _, _| Ok(signature()),
    )
    .expect("handle admits before signing");
    (handle_ref, handle, authority)
}

struct CheckpointPair {
    predecessor_ref: ResidentRecordRefV1,
    predecessor: ControlStateCheckpointV1,
    successor_ref: ResidentRecordRefV1,
    successor: ControlStateCheckpointV1,
}

impl CheckpointPair {
    // Physical representation only; the separate construction verifier remains required.
    fn physical_batch(&self) -> pale_ports::OperationAccumulatorBatch {
        crate::operation_accumulator_fixture::for_checkpoints(
            store_coordinate(),
            (&self.predecessor_ref, &self.predecessor),
            (&self.successor_ref, &self.successor),
        )
    }

    fn exact() -> Self {
        let predecessor_ref = record_ref("operation/checkpoint-1");
        let predecessor_preimage = checkpoint_preimage(1, None);
        let predecessor = admit_initial_checkpoint(&predecessor_preimage);
        let successor_ref = record_ref("operation/checkpoint-2");
        let successor_preimage = checkpoint_preimage(
            2,
            Some((
                predecessor_ref.clone(),
                predecessor.canonical_control_checkpoint_sha256().clone(),
            )),
        );
        let successor =
            admit_successor_checkpoint(&successor_preimage, &predecessor_ref, &predecessor);
        Self {
            predecessor_ref,
            predecessor,
            successor_ref,
            successor,
        }
    }

    fn transition<'a>(
        &'a self,
        verifier: &'a dyn OperationObservationCheckpointConstructionVerifier,
    ) -> OperationObservationCheckpointTransition<'a> {
        OperationObservationCheckpointTransition::new(
            &self.predecessor_ref,
            &self.predecessor,
            &self.successor_ref,
            &self.successor,
            verifier,
        )
    }
}

fn checkpoint_wire(value: &ControlStateCheckpointPreimageV1) -> ControlStateCheckpointWireV1 {
    let digest = ControlStateCheckpointDigest::hash_canonical(value).expect("checkpoint digest");
    let mut wire = serde_json::to_value(value).expect("checkpoint serializes");
    let object = wire.as_object_mut().expect("checkpoint is an object");
    object.insert(
        "canonical_control_checkpoint_sha256".into(),
        serde_json::to_value(digest).expect("digest serializes"),
    );
    object.insert(
        "signature".into(),
        serde_json::to_value(signature()).expect("signature serializes"),
    );
    serde_json::from_value(wire).expect("checkpoint wire")
}

pub(in crate::resident) fn admit_initial_checkpoint(
    value: &ControlStateCheckpointPreimageV1,
) -> ControlStateCheckpointV1 {
    ControlStateCheckpointV1::admit_initial_wire(
        checkpoint_wire(value),
        &value.schema_sha256,
        &checkpoint_authority(value),
        |_, _, _, _| Ok(()),
    )
    .expect("initial checkpoint admits")
}

pub(in crate::resident) fn admit_successor_checkpoint(
    value: &ControlStateCheckpointPreimageV1,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &ControlStateCheckpointV1,
) -> ControlStateCheckpointV1 {
    ControlStateCheckpointV1::admit_successor_wire(
        checkpoint_wire(value),
        predecessor_ref,
        predecessor,
        &value.schema_sha256,
        &checkpoint_authority(value),
        |_, _, _, _| Ok(()),
    )
    .expect("successor checkpoint admits")
}

type AccumulatorPreparation = fn(
    &pale_ports::OperationAccumulatorPreparation,
) -> Result<
    pale_ports::PreparedOperationAccumulator,
    pale_ports::OperationAccumulatorPreparationError,
>;

pub(in crate::resident) struct RecordingStore {
    store: AuthorityStoreCoordinate,
    preparation: Option<(AccumulatorPreparation, ControlTransactionSequenceV1)>,
    commit_result: Option<Result<OperationCustodyCommitOutcome, ControlStoreError>>,
    follow_up_commit_results: VecDeque<Result<OperationCustodyCommitOutcome, ControlStoreError>>,
    read_back_results: VecDeque<Result<OperationCustodyReadBackOutcome, ControlStoreError>>,
    candidate: Option<OperationCustodyCandidate>,
    commits: usize,
    read_backs: usize,
    index_reads: VecDeque<pale_ports::OperationCustodyIndexObservation>,
    index_queries: Vec<OperationCustodyIndexQuery>,
    index_read_counter: Option<Arc<AtomicUsize>>,
    index_error: Option<ControlStoreError>,
    unlisted_indexes_are_absent: bool,
    checkpoint_reads:
        VecDeque<Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError>>,
    checkpoint_queries: Vec<(AuthorityStoreCoordinate, ControlCheckpointDomainIdV1, usize)>,
    lease_retentions: VecDeque<Result<OperationCustodyCommitOutcome, ControlStoreError>>,
    retained_lease: Option<ControlRecord>,
    session_basis_override: Option<pale_ports::OperationSessionBasis>,
    recover_indexes_from_committed_candidate: bool,
}

impl RecordingStore {
    pub(in crate::resident) fn committing(outcome: OperationCustodyCommitOutcome) -> Self {
        let precommit = match outcome {
            OperationCustodyCommitOutcome::Committed => {
                OperationCustodyReadBackOutcome::Predecessor
            }
            OperationCustodyCommitOutcome::ExactReplay => {
                OperationCustodyReadBackOutcome::ExactSuccessor
            }
        };
        Self {
            store: store_coordinate(),
            preparation: None,
            commit_result: Some(Ok(outcome)),
            follow_up_commit_results: VecDeque::new(),
            read_back_results: VecDeque::from([Ok(precommit)]),
            candidate: None,
            commits: 0,
            read_backs: 0,
            index_reads: VecDeque::new(),
            index_queries: Vec::new(),
            index_read_counter: None,
            index_error: None,
            unlisted_indexes_are_absent: false,
            checkpoint_reads: VecDeque::new(),
            checkpoint_queries: Vec::new(),
            lease_retentions: VecDeque::new(),
            retained_lease: None,
            session_basis_override: None,
            recover_indexes_from_committed_candidate: false,
        }
    }

    pub(in crate::resident) fn recovering(
        reads: Vec<pale_ports::OperationCustodyIndexObservation>,
    ) -> Self {
        let mut store = Self::committing(OperationCustodyCommitOutcome::Committed);
        store.index_reads = reads.into();
        store
    }

    pub(in crate::resident) fn with_index_read_counter(
        mut self,
        counter: Arc<AtomicUsize>,
    ) -> Self {
        self.index_read_counter = Some(counter);
        self
    }

    pub(in crate::resident) fn with_index_error(mut self, error: ControlStoreError) -> Self {
        self.index_error = Some(error);
        self
    }

    pub(in crate::resident) const fn index_query_count(&self) -> usize {
        self.index_queries.len()
    }

    pub(in crate::resident) fn at_store(mut self, store: &AuthorityStoreCoordinate) -> Self {
        self.store = store.clone();
        self
    }

    pub(in crate::resident) fn failing_commit(
        error: ControlStoreError,
        read_backs: impl IntoIterator<Item = Result<OperationCustodyReadBackOutcome, ControlStoreError>>,
    ) -> Self {
        Self {
            store: store_coordinate(),
            preparation: None,
            commit_result: Some(Err(error)),
            follow_up_commit_results: VecDeque::new(),
            read_back_results: read_backs.into_iter().collect(),
            candidate: None,
            commits: 0,
            read_backs: 0,
            index_reads: VecDeque::new(),
            index_queries: Vec::new(),
            index_read_counter: None,
            index_error: None,
            unlisted_indexes_are_absent: false,
            checkpoint_reads: VecDeque::new(),
            checkpoint_queries: Vec::new(),
            lease_retentions: VecDeque::new(),
            retained_lease: None,
            session_basis_override: None,
            recover_indexes_from_committed_candidate: false,
        }
    }

    pub(in crate::resident) const fn mutation_counts(&self) -> (usize, usize) {
        (self.commits, self.read_backs)
    }

    pub(in crate::resident) const fn committed_candidate(
        &self,
    ) -> Option<&OperationCustodyCandidate> {
        self.candidate.as_ref()
    }

    pub(in crate::resident) fn replace_session_basis_override(
        &mut self,
        basis: Option<pale_ports::OperationSessionBasis>,
    ) {
        self.session_basis_override = basis;
    }

    pub(in crate::resident) fn replace_commit_result(
        &mut self,
        result: Result<OperationCustodyCommitOutcome, ControlStoreError>,
    ) {
        self.commit_result = Some(result);
    }

    pub(in crate::resident) fn replace_read_back_results(
        &mut self,
        results: impl IntoIterator<Item = Result<OperationCustodyReadBackOutcome, ControlStoreError>>,
    ) {
        self.read_back_results = results.into_iter().collect();
    }

    pub(in crate::resident) fn with_preparation(
        mut self,
        prepare: AccumulatorPreparation,
        transaction: ControlTransactionSequenceV1,
    ) -> Self {
        self.preparation = Some((prepare, transaction));
        self
    }

    pub(in crate::resident) fn replace_preparation(
        &mut self,
        prepare: AccumulatorPreparation,
        transaction: ControlTransactionSequenceV1,
    ) {
        self.preparation = Some((prepare, transaction));
    }

    /// Admit one later transaction after this fixture's first committed append.
    ///
    /// The later preparation starts from the checkpoint retained by the first
    /// append. This models two ordered commits; it does not make either commit
    /// atomic with the other.
    pub(in crate::resident) fn with_committed_follow_up(mut self) -> Self {
        self.follow_up_commit_results
            .push_back(Ok(OperationCustodyCommitOutcome::Committed));
        self.read_back_results
            .push_back(Ok(OperationCustodyReadBackOutcome::Predecessor));
        self
    }

    pub(in crate::resident) fn with_lease_retention(
        mut self,
        result: Result<OperationCustodyCommitOutcome, ControlStoreError>,
    ) -> Self {
        self.lease_retentions.push_back(result);
        self
    }

    pub(in crate::resident) fn with_lease_retentions(
        mut self,
        results: impl IntoIterator<Item = Result<OperationCustodyCommitOutcome, ControlStoreError>>,
    ) -> Self {
        self.lease_retentions.extend(results);
        self
    }

    pub(in crate::resident) const fn retained_lease(&self) -> Option<&ControlRecord> {
        self.retained_lease.as_ref()
    }

    /// Treat every index not explicitly queued by this fixture as absent.
    ///
    /// This is used only when an actual child-start candidate contributes a
    /// dynamically derived parent-membership key that the test must not copy.
    pub(in crate::resident) fn with_unlisted_indexes_absent(mut self) -> Self {
        self.unlisted_indexes_are_absent = true;
        self
    }

    /// Resolve later start checks from the last candidate this store committed.
    pub(in crate::resident) fn with_committed_candidate_index_recovery(mut self) -> Self {
        self.recover_indexes_from_committed_candidate = true;
        self
    }

    /// Seed exact already-committed evidence for a later point-addressed turn.
    pub(in crate::resident) fn with_committed_candidate(
        mut self,
        candidate: OperationCustodyCandidate,
    ) -> Self {
        self.candidate = Some(candidate);
        self.commits = 1;
        self.recover_indexes_from_committed_candidate = true;
        self
    }

    /// Seed the exact separately retained lease for committed start evidence.
    pub(in crate::resident) fn with_retained_lease_record(mut self, record: ControlRecord) -> Self {
        self.retained_lease = Some(record);
        self
    }

    pub(in crate::resident) fn enable_committed_candidate_recovery(&mut self) {
        self.recover_indexes_from_committed_candidate = true;
    }
}

pub(in crate::resident) fn candidate_indexed_record(
    candidate: &OperationCustodyCandidate,
    query: &OperationCustodyIndexQuery,
) -> Option<ControlRecord> {
    let coordinate = candidate
        .indexes()
        .iter()
        .find_map(|mutation| matching_index_successor(mutation, query))?;
    candidate
        .records()
        .iter()
        .map(OperationCustodyRecord::record)
        .chain(candidate.a2a_retained_records())
        .chain(candidate.a2a_associated_route())
        .chain(std::iter::once(candidate.checkpoint().successor_record()))
        .chain(candidate.checkpoint().predecessor_record())
        .find(|record| record.coordinate() == coordinate)
        .cloned()
}

fn matching_index_successor<'a>(
    mutation: &'a OperationCustodyIndexMutation,
    query: &OperationCustodyIndexQuery,
) -> Option<&'a ControlRecordCoordinate> {
    use OperationCustodyIndexMutation as Mutation;
    use OperationCustodyIndexQuery as Query;

    match (mutation, query) {
        (Mutation::NativeOutcome { key, successor, .. }, Query::NativeOutcome(expected))
        | (
            Mutation::A2aSendPreparation { key, successor, .. },
            Query::A2aSendPreparation(expected),
        )
        | (
            Mutation::A2aSendPreparationCheckpoint { key, successor },
            Query::A2aSendPreparationCheckpoint(expected),
        )
        | (Mutation::A2aTaskBinding { key, successor, .. }, Query::A2aTaskBinding(expected))
        | (
            Mutation::FirstA2aTaskObservation { key, successor },
            Query::FirstA2aTaskObservation(expected),
        )
        | (
            Mutation::LatestA2aTaskObservation { key, successor, .. },
            Query::LatestA2aTaskObservation(expected),
        )
        | (
            Mutation::A2aTaskObservationCheckpoint { key, successor },
            Query::A2aTaskObservationCheckpoint(expected),
        )
        | (Mutation::RequestForHandle { key, successor }, Query::RequestForHandle(expected))
        | (Mutation::StartObservation { key, successor }, Query::StartObservation(expected))
        | (
            Mutation::ObservationCheckpoint { key, successor },
            Query::ObservationCheckpoint(expected),
        )
        | (Mutation::RequestEvidence { key, successor, .. }, Query::RequestEvidence(expected))
        | (Mutation::RequestOperation { key, successor, .. }, Query::RequestOperation(expected))
        | (Mutation::OperationHandle { key, successor, .. }, Query::OperationHandle(expected))
        | (
            Mutation::ParentChildMembership { key, successor, .. },
            Query::ParentChildMembership(expected),
        )
        | (Mutation::CollectionHead { key, successor, .. }, Query::CollectionHead(expected))
        | (
            Mutation::LatestObservation { key, successor, .. },
            Query::LatestObservation(expected),
        ) if key == expected => Some(successor),
        _ => None,
    }
}

impl pale_ports::ControlTransactionPort for RecordingStore {
    fn prepare_transaction(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        let Some((_, initial_transaction)) = self.preparation else {
            panic!("this physical-custody/readback fixture does not prepare a new checkpoint")
        };
        let transaction = if self.commits == 0 {
            initial_transaction
        } else {
            self.candidate
                .as_ref()
                .expect("a follow-up transaction follows a committed candidate")
                .checkpoint()
                .successor()
                .committed_transaction_sequence()
        };
        pale_ports::ControlTransactionPreparation::new(store.clone(), transaction)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
    }
}

impl OperationCustodyPort for RecordingStore {
    fn read_operation_session_basis(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<pale_ports::OperationSessionBasis>, ControlStoreError> {
        if !self.recover_indexes_from_committed_candidate {
            return Ok(None);
        }
        assert_eq!(store, &self.store);
        let candidate = self
            .candidate
            .as_ref()
            .expect("session-basis recovery follows a committed candidate");
        let basis = self
            .session_basis_override
            .as_ref()
            .filter(|basis| basis.handle() == handle)
            .cloned()
            .or_else(|| {
                candidate
                    .session_basis()
                    .filter(|basis| basis.handle() == handle)
                    .cloned()
            });
        if let Some(basis) = &basis {
            assert!(basis.canonical_basis_jcs().len() <= maximum_bytes);
        }
        Ok(basis)
    }

    fn read_a2a_task_bindings(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationA2aBindingScope,
        _: Option<&pale_ports::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationA2aBindings, pale_ports::ControlStoreError> {
        panic!("remote-task discovery is outside this fixture's tested journey")
    }

    fn read_publication_sources(
        &mut self,
        _store: &pale_ports::AuthorityStoreCoordinate,
        _cursor: Option<&pale_ports::OperationPublicationCursor>,
        _maximum_sources: usize,
        _maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        panic!("publication discovery is outside this fixture's tested journey")
    }

    fn read_checkpoint_head(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        domain: &pale_core::records::ControlCheckpointDomainIdV1,
        maximum: usize,
    ) -> Result<
        Option<(pale_ports::ControlCheckpointHead, pale_ports::ControlRecord)>,
        pale_ports::ControlStoreError,
    > {
        self.checkpoint_queries
            .push((store.clone(), domain.clone(), maximum));
        match self.checkpoint_reads.pop_front() {
            Some(read) => read,
            None if self.recover_indexes_from_committed_candidate => {
                let candidate = self
                    .candidate
                    .as_ref()
                    .expect("checkpoint recovery follows a committed candidate");
                let checkpoint = candidate.checkpoint();
                assert_eq!(store, candidate.store());
                assert_eq!(domain, checkpoint.successor().checkpoint_domain_id());
                assert!(
                    checkpoint
                        .successor_record()
                        .canonical_record_jcs()
                        .as_slice()
                        .len()
                        <= maximum
                );
                Ok(Some((
                    checkpoint.successor().clone(),
                    checkpoint.successor_record().clone(),
                )))
            }
            None => panic!("expected checkpoint-head read"),
        }
    }

    fn verify_accumulator_membership(
        &mut self,
        request: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        if self.recover_indexes_from_committed_candidate {
            Ok(pale_ports::VerifiedOperationAccumulatorMembership::new(
                request.clone(),
            ))
        } else {
            panic!("this owning phase test does not perform membership recovery")
        }
    }

    fn prepare_accumulator(
        &mut self,
        request: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        let Some((prepare, _)) = self.preparation else {
            panic!("unexpected accumulator preparation in custody-only fixture")
        };
        prepare(request)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        record: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        let outcome = self
            .lease_retentions
            .pop_front()
            .expect("observation fixture does not retain operation leases")?;
        self.retained_lease = Some(record.clone());
        Ok(outcome)
    }

    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        if !self.recover_indexes_from_committed_candidate {
            panic!("observation fixture does not read operation leases")
        }
        assert_eq!(store, &self.store);
        let candidate = self
            .candidate
            .as_ref()
            .expect("record recovery follows a committed candidate");
        let record = candidate
            .records()
            .iter()
            .map(OperationCustodyRecord::record)
            .chain(std::iter::once(candidate.checkpoint().successor_record()))
            .chain(candidate.checkpoint().predecessor_record())
            .chain(self.retained_lease.iter())
            .find(|record| record.coordinate() == coordinate)
            .cloned();
        if let Some(record) = &record {
            assert!(record.canonical_record_jcs().as_slice().len() <= maximum_bytes);
        }
        Ok(record)
    }

    fn compare_and_commit(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.commits += 1;
        self.candidate = Some(candidate.clone());
        self.commit_result.take().map_or_else(
            || {
                self.follow_up_commit_results
                    .pop_front()
                    .expect("one commit result")
            },
            std::convert::identity,
        )
    }

    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        self.read_backs += 1;
        self.read_back_results
            .pop_front()
            .expect("one read-back result")
    }

    fn read_index(
        &mut self,
        coordinate: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        _maximum_record_bytes: usize,
    ) -> Result<pale_ports::OperationCustodyIndexObservation, ControlStoreError> {
        assert_eq!(coordinate, &self.store);
        self.index_queries.push(query.clone());
        if let Some(counter) = &self.index_read_counter {
            counter.fetch_add(1, Ordering::SeqCst);
        }
        if let Some(error) = self.index_error.take() {
            return Err(error);
        }
        match self.index_reads.pop_front() {
            Some(observation) => Ok(observation),
            None if self.recover_indexes_from_committed_candidate && self.candidate.is_none() => {
                Ok(pale_ports::OperationCustodyIndexObservation::absent(
                    query.clone(),
                ))
            }
            None if self.recover_indexes_from_committed_candidate => {
                let candidate = self
                    .candidate
                    .as_ref()
                    .expect("index recovery follows a committed candidate");
                candidate_indexed_record(candidate, query).map_or_else(
                    || {
                        Ok(pale_ports::OperationCustodyIndexObservation::absent(
                            query.clone(),
                        ))
                    },
                    |record| {
                        pale_ports::OperationCustodyIndexObservation::exact(query.clone(), record)
                            .map_err(|_| {
                                ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
                            })
                    },
                )
            }
            None if self.unlisted_indexes_are_absent => Ok(
                pale_ports::OperationCustodyIndexObservation::absent(query.clone()),
            ),
            None => panic!("expected recovery read"),
        }
    }
}

fn observation_recovery_reads(
    fixture: &OperationFixture,
) -> Vec<pale_ports::OperationCustodyIndexObservation> {
    let key = operation_handle_key(fixture.handle.operation_handle_id()).expect("writer key");
    let handle = record_projection::record(
        record_projection::coordinate(
            &fixture.handle_ref,
            fixture.handle.canonical_operation_handle_sha256().as_str(),
        )
        .expect("handle coordinate"),
        &fixture.handle,
    )
    .expect("handle projection");
    let observation = project_observation(&fixture.successor_ref, &fixture.successor)
        .expect("observation projection");
    let checkpoints = CheckpointPair::exact();
    let checkpoint = project_checkpoint_record(&checkpoints.successor_ref, &checkpoints.successor)
        .expect("checkpoint projection");
    [
        (
            OperationCustodyIndexQuery::OperationHandle(key.clone()),
            handle,
        ),
        (
            OperationCustodyIndexQuery::LatestObservation(key),
            observation.clone(),
        ),
        (
            OperationCustodyIndexQuery::ObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(observation.coordinate()),
            ),
            checkpoint,
        ),
    ]
    .into_iter()
    .map(|(query, record)| {
        pale_ports::OperationCustodyIndexObservation::exact(query, record).expect("index role")
    })
    .collect()
}

fn recover_observation(
    fixture: &OperationFixture,
    store: &mut RecordingStore,
    maximum: usize,
) -> Result<Option<OperationObservationRecoveryEvidence>, OperationRecoveryError> {
    read_operation_observation_for_handle(
        store,
        &store_coordinate(),
        &fixture.handle_ref,
        &fixture.handle,
        maximum,
    )
}

#[test]
fn handle_recovery_selects_exact_physical_observation_and_its_checkpoint_without_writes() {
    let fixture = OperationFixture::exact();
    let reads = observation_recovery_reads(&fixture);
    let mut store = RecordingStore::recovering(reads.clone());
    let evidence = recover_observation(&fixture, &mut store, 16 * 1024)
        .expect("bounded lookup")
        .expect("retained operation");
    assert_eq!(evidence.store(), &store_coordinate());
    assert_eq!(Some(evidence.handle()), reads[0].record());
    assert_eq!(Some(evidence.observation()), reads[1].record());
    assert_eq!(Some(evidence.checkpoint()), reads[2].record());
    assert_eq!(
        store.index_queries,
        reads
            .iter()
            .map(|read| read.query().clone())
            .collect::<Vec<_>>()
    );
    assert_eq!((store.commits, store.read_backs), (0, 0));
    assert!(store.index_reads.is_empty());
}

#[test]
fn observation_recovery_distinguishes_absent_from_reachable_partial_indexes() {
    let fixture = OperationFixture::exact();
    for present in [
        [false, false, false],
        [true, false, false],
        [false, true, false],
        [true, true, false],
    ] {
        let reads = observation_recovery_reads(&fixture)
            .into_iter()
            .zip(present)
            .map(|(read, present)| {
                if present {
                    read
                } else {
                    pale_ports::OperationCustodyIndexObservation::absent(read.query().clone())
                }
            })
            .collect();
        let mut store = RecordingStore::recovering(reads);
        let result = recover_observation(&fixture, &mut store, 16 * 1024);
        if present == [false, false, false] {
            assert!(result.expect("both absent").is_none());
        } else {
            assert!(matches!(
                result,
                Err(OperationRecoveryError::IncompleteObservationIndexes)
            ));
        }
        assert_eq!((store.commits, store.read_backs), (0, 0));
    }
}

#[test]
fn observation_recovery_rejects_each_misdirected_or_oversized_point_response() {
    let fixture = OperationFixture::exact();
    for role in 0..3 {
        let mut reads = observation_recovery_reads(&fixture);
        let unrelated_key = OperationCustodyIndexKey::parse(sha('f')).expect("unrelated key");
        let query = match role {
            0 => OperationCustodyIndexQuery::OperationHandle(unrelated_key),
            1 => OperationCustodyIndexQuery::LatestObservation(unrelated_key),
            _ => OperationCustodyIndexQuery::ObservationCheckpoint(unrelated_key),
        };
        reads[role] = pale_ports::OperationCustodyIndexObservation::exact(
            query,
            reads[role].record().expect("present").clone(),
        )
        .expect("same schema role, wrong query");
        let mut store = RecordingStore::recovering(reads);
        assert!(matches!(
            recover_observation(&fixture, &mut store, 16 * 1024),
            Err(OperationRecoveryError::MisdirectedObservation)
        ));
        assert_eq!(store.index_queries.len(), role + 1);
        assert_eq!((store.commits, store.read_backs), (0, 0));

        let mut reads = observation_recovery_reads(&fixture);
        let original = reads[role].record().expect("present");
        let oversized = ControlRecord::new(
            original.coordinate().clone(),
            original.schema_id().clone(),
            original.schema_sha256().clone(),
            pale_ports::ControlRecordBytes::new(vec![b' '; 16 * 1024 + 1]).expect("opaque bytes"),
        );
        reads[role] = pale_ports::OperationCustodyIndexObservation::exact(
            reads[role].query().clone(),
            oversized,
        )
        .expect("schema role");
        let mut store = RecordingStore::recovering(reads);
        assert!(matches!(
            recover_observation(&fixture, &mut store, 16 * 1024),
            Err(OperationRecoveryError::RecordBoundExceeded)
        ));
        assert_eq!(store.index_queries.len(), role + 1);
        assert_eq!((store.commits, store.read_backs), (0, 0));
    }
}

#[test]
fn recovery_rejects_same_identity_with_changed_handle_contents_or_reference() {
    let original = OperationFixture::exact();
    let changed = OperationFixture::with_handle_identity("1", "operation-1", 121);
    let mut changed_reference = OperationFixture::exact();
    changed_reference.handle_ref = record_ref("operation/other-reference");
    for fixture in [&changed, &changed_reference] {
        let mut store = RecordingStore::recovering(observation_recovery_reads(&original));
        assert!(matches!(
            recover_observation(fixture, &mut store, 16 * 1024),
            Err(OperationRecoveryError::HandleConflict)
        ));
        assert_eq!(store.index_queries.len(), 1);
        assert_eq!((store.commits, store.read_backs), (0, 0));
    }
}

#[test]
fn physical_recovery_does_not_pretend_to_admit_observation_or_checkpoint_contents() {
    let fixture = OperationFixture::exact();
    let mut reads = observation_recovery_reads(&fixture);
    for read in &mut reads[1..] {
        let original = read.record().expect("present");
        let opaque = ControlRecord::new(
            original.coordinate().clone(),
            original.schema_id().clone(),
            original.schema_sha256().clone(),
            pale_ports::ControlRecordBytes::new(b"{}".to_vec()).expect("opaque bytes"),
        );
        *read = pale_ports::OperationCustodyIndexObservation::exact(read.query().clone(), opaque)
            .expect("schema role only");
    }
    let mut store = RecordingStore::recovering(reads);
    let evidence = recover_observation(&fixture, &mut store, 16 * 1024)
        .expect("physical lookup is not semantic admission")
        .expect("retained evidence");
    assert_eq!(
        evidence.observation().canonical_record_jcs().as_slice(),
        b"{}"
    );
    assert_eq!(
        evidence.checkpoint().canonical_record_jcs().as_slice(),
        b"{}"
    );
    assert_eq!((store.commits, store.read_backs), (0, 0));
}

pub(in crate::resident) struct RecordingAnchor {
    advance_result: Option<Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError>>,
    observation_result: Option<Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError>>,
    follow_up_observation_result:
        Option<Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError>>,
    advances: usize,
    observations: usize,
}

impl RecordingAnchor {
    fn predecessor() -> Self {
        Self {
            advance_result: None,
            observation_result: Some(Ok(AntiRollbackAnchorObservation::ExactPredecessor)),
            follow_up_observation_result: None,
            advances: 0,
            observations: 0,
        }
    }

    fn advancing() -> Self {
        Self {
            advance_result: Some(Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)),
            observation_result: Some(Ok(AntiRollbackAnchorObservation::ExactPredecessor)),
            follow_up_observation_result: None,
            advances: 0,
            observations: 0,
        }
    }

    pub(in crate::resident) fn unavailable_after_preflight() -> Self {
        Self {
            advance_result: Some(Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ))),
            observation_result: Some(Ok(AntiRollbackAnchorObservation::ExactPredecessor)),
            follow_up_observation_result: None,
            advances: 0,
            observations: 0,
        }
    }

    pub(in crate::resident) fn unavailable_initial_after_preflight() -> Self {
        let mut anchor = Self::unavailable_after_preflight();
        anchor.observation_result = Some(Ok(AntiRollbackAnchorObservation::Missing));
        anchor
    }

    pub(in crate::resident) fn conflicting_initial_after_preflight() -> Self {
        Self {
            advance_result: Some(Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(
                AntiRollbackAnchorObservation::Ahead {
                    observed: operation_anchor(3, "operation/checkpoint-unlike", '7'),
                },
            ))),
            observation_result: Some(Ok(AntiRollbackAnchorObservation::Missing)),
            follow_up_observation_result: None,
            advances: 0,
            observations: 0,
        }
    }

    pub(in crate::resident) fn ambiguous_ahead_after_preflight() -> Self {
        Self {
            advance_result: Some(Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::OutcomeUnknown,
            ))),
            observation_result: Some(Ok(AntiRollbackAnchorObservation::Missing)),
            follow_up_observation_result: Some(Ok(AntiRollbackAnchorObservation::Ahead {
                observed: operation_anchor(3, "operation/checkpoint-unlike", '7'),
            })),
            advances: 0,
            observations: 0,
        }
    }

    pub(in crate::resident) fn ambiguous_observation_failure_after_preflight() -> Self {
        Self {
            advance_result: Some(Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::OutcomeUnknown,
            ))),
            observation_result: Some(Ok(AntiRollbackAnchorObservation::Missing)),
            follow_up_observation_result: Some(Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ))),
            advances: 0,
            observations: 0,
        }
    }

    pub(in crate::resident) const fn access_counts(&self) -> (usize, usize) {
        (self.advances, self.observations)
    }

    pub(in crate::resident) fn allow_next_advance(&mut self) {
        assert!(self
            .advance_result
            .replace(Ok(AntiRollbackAnchorAdvanceDisposition::Advanced))
            .is_none());
    }

    /// Admit one later failed observation without granting another advance.
    pub(in crate::resident) fn allow_unavailable_observation(&mut self) {
        assert!(self
            .observation_result
            .replace(Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            )))
            .is_none());
    }

    /// Admit a later exact-successor read after an earlier predecessor preflight.
    pub(in crate::resident) fn allow_exact_successor_observation(&mut self) {
        assert!(self
            .follow_up_observation_result
            .replace(Ok(AntiRollbackAnchorObservation::ExactSuccessor))
            .is_none());
    }

    pub(in crate::resident) fn allow_initial_preflight(&mut self) {
        assert!(self
            .observation_result
            .replace(Ok(AntiRollbackAnchorObservation::Missing))
            .is_none());
    }

    /// Admit the retained anchor as the exact predecessor of one successor.
    pub(in crate::resident) fn allow_successor_preflight(&mut self) {
        assert!(self
            .observation_result
            .replace(Ok(AntiRollbackAnchorObservation::ExactPredecessor))
            .is_none());
    }
}

fn commit_observation(
    prepared: PreparedOperationObservationCustody,
    store: &mut dyn OperationCustodyPort,
) -> Result<
    DatabaseCommittedOperationObservationAnchorPending,
    OperationObservationCustodyCommitFailure,
> {
    let mut anchor = RecordingAnchor::predecessor();
    prepared.commit(store, &mut anchor)
}

fn operation_anchor(sequence: u64, reference: &str, digest_byte: char) -> AntiRollbackAnchor {
    AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
        ControlStoreIdV1::parse("control-store").expect("store"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation"),
        ControlCheckpointDomainIdV1::parse("operation:principal-1").expect("checkpoint domain"),
        ControlDomainEpochV1::new(1).expect("epoch"),
        ControlDomainSequenceV1::new(sequence).expect("sequence"),
        ControlRecordCoordinate::new(
            record_ref(reference),
            pale_ports::ControlRecordDigest::parse(sha(digest_byte)).expect("digest"),
        ),
    ))
}

impl AntiRollbackAnchorPort for RecordingAnchor {
    fn read(
        &mut self,
        key: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, AntiRollbackAnchorError> {
        let _ = key;
        panic!("observation transition test must not read a recovery anchor")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.advances += 1;
        self.advance_result.take().expect("one advance result")
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations += 1;
        self.observation_result
            .take()
            .or_else(|| self.follow_up_observation_result.take())
            .expect("one anchor observation")
    }
}

struct ResolveStore;

impl OperationObservationAuthorityStoreResolver for ResolveStore {
    fn resolve(
        &self,
        observation: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        assert_eq!(
            observation.deployment_runtime_authority_ref(),
            &record_ref("runtime-authority")
        );
        assert_eq!(
            observation.deployment_runtime_authority_sha256(),
            &artifact_digest('1')
        );
        Ok(store_coordinate())
    }
}

fn prepare_exact<'a>(
    fixture: &'a OperationFixture,
    checkpoints: &'a CheckpointPair,
    verifier: &'a dyn OperationObservationCheckpointConstructionVerifier,
) -> PreparedOperationObservationCustody {
    prepare_operation_observation_append(fixture.transition(), &ResolveStore)
        .and_then(|append| {
            append.attach_checkpoint(
                checkpoints.transition(verifier),
                checkpoints.physical_batch(),
            )
        })
        .expect("typed observation custody prepares")
}

struct AcceptCheckpoint;

#[test]
fn checkpoint_attachment_rejects_physical_root_or_profile_substitution_before_verifier() {
    #[derive(Debug, Clone, Copy)]
    enum Substitution {
        PriorState,
        NextState,
        PriorUnresolved,
        NextUnresolved,
        ProfileReference,
        ProfileDigest,
        PriorTransaction,
        NextTransaction,
    }
    let fixture = OperationFixture::exact();
    // Leave a real store-transaction gap so the prior physical stamp can be
    // changed independently while both the core pair and port binding admit.
    let mut prior = checkpoint_preimage(1, None);
    prior.committed_transaction_sequence = ControlTransactionSequenceV1::new(2).unwrap();
    let predecessor = admit_initial_checkpoint(&prior);
    let predecessor_ref = record_ref("operation/checkpoint-1");
    let mut next = checkpoint_preimage(
        2,
        Some((
            predecessor_ref.clone(),
            predecessor.canonical_control_checkpoint_sha256().clone(),
        )),
    );
    next.committed_transaction_sequence = ControlTransactionSequenceV1::new(4).unwrap();
    let checkpoints = CheckpointPair {
        successor: admit_successor_checkpoint(&next, &predecessor_ref, &predecessor),
        predecessor,
        predecessor_ref,
        successor_ref: record_ref("operation/checkpoint-2"),
    };
    for substitution in [
        Substitution::PriorState,
        Substitution::NextState,
        Substitution::PriorUnresolved,
        Substitution::NextUnresolved,
        Substitution::ProfileReference,
        Substitution::ProfileDigest,
        Substitution::PriorTransaction,
        Substitution::NextTransaction,
    ] {
        let original = checkpoints.physical_batch();
        let binding = original.binding();
        let mut profile_ref = binding.profile_ref().clone();
        let mut profile_sha256 = binding.profile_sha256().clone();
        let mut prior_state = original.state().predecessor_root().clone();
        let mut next_state = original.state().successor_root().clone();
        let mut prior_unresolved = original.unresolved().predecessor_root().clone();
        let mut next_unresolved = original.unresolved().successor_root().clone();
        let mut phase = binding.phase().clone();
        let mut successor = binding.successor().clone();
        let mut transaction = binding.transaction().clone();
        let with_transaction = |head: &pale_ports::ControlCheckpointHead, sequence| {
            pale_ports::ControlCheckpointHead::new(
                head.checkpoint_domain_id().clone(),
                head.domain_epoch(),
                head.domain_sequence(),
                ControlTransactionSequenceV1::new(sequence).unwrap(),
                head.coordinate().clone(),
            )
        };
        match substitution {
            Substitution::PriorState => prior_state = artifact_digest('0'),
            Substitution::NextState => next_state = artifact_digest('0'),
            Substitution::PriorUnresolved => prior_unresolved = artifact_digest('0'),
            Substitution::NextUnresolved => next_unresolved = artifact_digest('0'),
            Substitution::ProfileReference => profile_ref = record_ref("another-profile"),
            Substitution::ProfileDigest => {
                profile_sha256 = CheckpointProfileDigest::parse(sha('0')).expect("digest");
            }
            Substitution::PriorTransaction => {
                let pale_ports::AccumulatorCheckpointPhase::Successor(prior) = &phase else {
                    panic!("successor fixture")
                };
                phase =
                    pale_ports::AccumulatorCheckpointPhase::Successor(with_transaction(prior, 1));
            }
            Substitution::NextTransaction => {
                successor = with_transaction(&successor, 5);
                transaction = pale_ports::ControlTransactionPreparation::new(
                    binding.store().clone(),
                    ControlTransactionSequenceV1::new(4).unwrap(),
                )
                .unwrap();
            }
        }
        let batch = crate::operation_accumulator_fixture::physical_batch(
            pale_ports::AccumulatorCheckpointBinding::new(
                binding.store().clone(),
                profile_ref,
                profile_sha256,
                phase,
                successor,
                transaction,
            )
            .expect("same physical checkpoint heads"),
            prior_state,
            next_state,
            prior_unresolved,
            next_unresolved,
        );
        let calls = Cell::new(0);
        let verifier = |_: &OperationObservationCheckpointConstruction<'_>| {
            calls.set(calls.get() + 1);
            Ok(())
        };
        let error = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
            .expect("unchanged admitted observation")
            .attach_checkpoint(checkpoints.transition(&verifier), batch)
            .expect_err("matching checkpoint heads do not establish matching roots or profile");
        assert!(
            matches!(
                error,
                OperationObservationCustodyProjectionError::Semantic(_)
            ),
            "{substitution:?}: {error:?}"
        );
        assert_eq!(calls.get(), 0, "{substitution:?}");
    }
}

impl OperationObservationCheckpointConstructionVerifier for AcceptCheckpoint {
    fn verify(
        &self,
        _: &OperationObservationCheckpointConstruction<'_>,
    ) -> Result<(), OperationObservationCheckpointConstructionRejection> {
        Ok(())
    }
}

struct RejectCheckpoint;

impl OperationObservationCheckpointConstructionVerifier for RejectCheckpoint {
    fn verify(
        &self,
        _: &OperationObservationCheckpointConstruction<'_>,
    ) -> Result<(), OperationObservationCheckpointConstructionRejection> {
        Err(OperationObservationCheckpointConstructionRejection)
    }
}

struct RecordingCheckpoint<'a> {
    fixture: &'a OperationFixture,
    proof_seen: &'a Cell<bool>,
    key_seen: &'a RefCell<Option<String>>,
}

impl OperationObservationCheckpointConstructionVerifier for RecordingCheckpoint<'_> {
    fn verify(
        &self,
        construction: &OperationObservationCheckpointConstruction<'_>,
    ) -> Result<(), OperationObservationCheckpointConstructionRejection> {
        assert_eq!(construction.store(), &store_coordinate());
        assert_eq!(
            construction.handle(),
            (&self.fixture.handle_ref, &self.fixture.handle)
        );
        assert_eq!(
            construction.predecessor(),
            (&self.fixture.predecessor_ref, &self.fixture.predecessor)
        );
        assert_eq!(
            construction.successor(),
            (&self.fixture.successor_ref, &self.fixture.successor)
        );
        assert_eq!(
            construction.predecessor_record().coordinate().record_ref(),
            &self.fixture.predecessor_ref
        );
        assert_eq!(
            construction.successor_record().coordinate().record_ref(),
            &self.fixture.successor_ref
        );
        assert_eq!(
            construction
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref(),
            &record_ref("operation/checkpoint-2")
        );
        let OperationCustodyIndexMutation::LatestObservation { key, .. } =
            construction.latest_observation_mutation()
        else {
            return Err(OperationObservationCheckpointConstructionRejection);
        };
        self.key_seen.replace(Some(key.as_str().to_owned()));
        self.proof_seen.set(true);
        Ok(())
    }
}

#[test]
fn exact_admitted_append_projects_once_and_completes_only_after_anchor() {
    let fixture = OperationFixture::exact();
    let resolutions = Cell::new(0);
    let resolver = |observation: &OperationObservationV1| {
        resolutions.set(resolutions.get() + 1);
        ResolveStore.resolve(observation)
    };
    let append = prepare_operation_observation_append(fixture.transition(), &resolver)
        .expect("prepare without a checkpoint");
    assert_eq!(resolutions.get(), 1);
    assert_eq!(append.store(), &store_coordinate());
    assert_eq!(append.handle(), (&fixture.handle_ref, &fixture.handle));
    assert_eq!(
        append.predecessor(),
        (&fixture.predecessor_ref, &fixture.predecessor)
    );
    assert_eq!(
        append.successor(),
        (&fixture.successor_ref, &fixture.successor)
    );
    assert_eq!(
        append
            .predecessor_record()
            .canonical_record_jcs()
            .as_slice(),
        canon::to_canonical_string(&fixture.predecessor)
            .expect("predecessor bytes")
            .as_bytes()
    );
    assert_eq!(
        append.successor_record().canonical_record_jcs().as_slice(),
        canon::to_canonical_string(&fixture.successor)
            .expect("successor bytes")
            .as_bytes()
    );
    let successor_bytes = append
        .successor_record()
        .canonical_record_jcs()
        .as_slice()
        .as_ptr();
    let primary_mutation = append.latest_observation_mutation().clone();
    assert_eq!(
        format!("{append:?}"),
        "PreparedOperationObservationAppend { .. }"
    );
    // Only now is a checkpoint pair supplied. The fixture verifier exercises
    // the attachment boundary, not an implemented accumulator producer.
    let checkpoints = CheckpointPair::exact();
    let proof_seen = Cell::new(false);
    let key_seen = RefCell::new(None);
    let verifier = RecordingCheckpoint {
        fixture: &fixture,
        proof_seen: &proof_seen,
        key_seen: &key_seen,
    };
    let prepared = append
        .attach_checkpoint(
            checkpoints.transition(&verifier),
            checkpoints.physical_batch(),
        )
        .expect("attach the exact construction proof");
    assert_eq!(
        resolutions.get(),
        1,
        "attachment uses the retained store resolution"
    );
    assert!(proof_seen.get());
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let mut anchor = RecordingAnchor::advancing();
    let pending = prepared
        .commit(&mut store, &mut anchor)
        .expect("database custody");
    assert_eq!(
        pending.candidate().records()[0]
            .record()
            .canonical_record_jcs()
            .as_slice()
            .as_ptr(),
        successor_bytes,
        "the prepared successor buffer moves into the custody candidate"
    );
    assert_eq!(&pending.candidate().indexes()[0], &primary_mutation);
    assert_eq!(
        pending.database_evidence(),
        OperationCustodyDatabaseEvidence::New
    );
    assert_eq!(
        pending.candidate().kind(),
        OperationCustodyTransitionKind::Observe
    );
    let candidate = store.candidate.as_ref().expect("candidate observed once");
    assert_eq!(candidate.kind(), OperationCustodyTransitionKind::Observe);
    assert_eq!(candidate.records().len(), 1);
    assert_eq!(candidate.indexes().len(), 2);
    assert!(candidate.indexes().iter().any(|index| matches!(index,
        OperationCustodyIndexMutation::ObservationCheckpoint { successor, .. }
            if successor == candidate.checkpoint().successor_record().coordinate()
    )));
    match &candidate.indexes()[0] {
        OperationCustodyIndexMutation::LatestObservation {
            key,
            expected: OperationCustodyIndexPredecessor::Exact(predecessor),
            successor,
        } => {
            assert_eq!(Some(key.as_str()), key_seen.borrow().as_deref());
            assert_eq!(
                predecessor.coordinate().record_ref(),
                &fixture.predecessor_ref
            );
            assert_eq!(successor.record_ref(), &fixture.successor_ref);
        }
        _ => panic!("one exact latest-observation compare-and-set"),
    }
    let completed = pending.advance_anchor(&mut anchor).expect("hard anchor");
    assert_eq!(
        completed.anchor_evidence(),
        OperationCustodyAnchorEvidence::Advanced
    );
    assert_eq!(
        completed.candidate().records()[0]
            .record()
            .coordinate()
            .record_ref(),
        &fixture.successor_ref
    );
    assert_eq!(store.commits, 1);
    assert_eq!(store.read_backs, 1);
    assert_eq!(anchor.observations, 1);
    assert_eq!(anchor.advances, 1);
}

#[test]
fn restored_database_cannot_append_when_the_independent_anchor_is_already_ahead() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let mut anchor = RecordingAnchor {
        advance_result: None,
        observation_result: Some(Ok(AntiRollbackAnchorObservation::ExactSuccessor)),
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let failure = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut anchor)
        .expect_err("an independently retained successor rejects restored database state");

    assert!(matches!(
        failure.reason(),
        OperationObservationCustodyCommitFailureReason::AnchorPrecondition(
            OperationCustodyAnchorFailureReason::NotAdvanced(observation)
        ) if matches!(observation.as_ref(), AntiRollbackAnchorObservation::ExactSuccessor)
    ));
    assert_eq!(store.commits, 0);
    assert_eq!(store.read_backs, 1);
    assert_eq!(anchor.observations, 1);
    let _failure = failure
        .try_into_retryable()
        .expect_err("an ahead anchor is not append retry authority");
}

#[test]
fn conflicting_or_partial_database_state_fails_before_anchor_or_commit() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore {
        store: store_coordinate(),
        preparation: None,
        commit_result: Some(Ok(OperationCustodyCommitOutcome::Committed)),
        follow_up_commit_results: VecDeque::new(),
        read_back_results: VecDeque::from([Ok(
            OperationCustodyReadBackOutcome::ConflictingOrPartial,
        )]),
        candidate: None,
        commits: 0,
        read_backs: 0,
        index_reads: VecDeque::new(),
        index_queries: Vec::new(),
        index_read_counter: None,
        index_error: None,
        unlisted_indexes_are_absent: false,
        checkpoint_reads: VecDeque::new(),
        checkpoint_queries: Vec::new(),
        lease_retentions: VecDeque::new(),
        retained_lease: None,
        session_basis_override: None,
        recover_indexes_from_committed_candidate: false,
    };
    let mut anchor = RecordingAnchor::predecessor();
    let failure = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut anchor)
        .expect_err("partial physical state requires recovery");

    assert!(matches!(
        failure.reason(),
        OperationObservationCustodyCommitFailureReason::Database(
            OperationCustodyDatabaseFailureReason::Store(error)
        ) if error.kind() == ControlStoreErrorKind::RecoveryRequired
    ));
    assert_eq!(store.commits, 0);
    assert_eq!(store.read_backs, 1);
    assert_eq!(anchor.observations, 0);
}

#[test]
fn unavailable_observation_anchor_preflight_retains_a_retryable_append_without_writes() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let mut anchor = RecordingAnchor {
        advance_result: None,
        observation_result: Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))),
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let failure = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut anchor)
        .expect_err("unavailable anchor cannot permit an observation write");

    assert!(matches!(
        failure.reason(),
        OperationObservationCustodyCommitFailureReason::AnchorPrecondition(
            OperationCustodyAnchorFailureReason::Port(error)
        ) if error.kind() == AntiRollbackAnchorErrorKind::Unavailable
    ));
    let _prepared = failure
        .try_into_retryable()
        .expect("read-only preflight failure retains the original append");
    assert_eq!(store.commits, 0);
    assert_eq!(store.read_backs, 1);
    assert_eq!(anchor.observations, 1);
    assert_eq!(anchor.advances, 0);
}

#[test]
fn anchor_change_after_preflight_cannot_complete_a_new_observation_commit() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let mut anchor = RecordingAnchor {
        advance_result: Some(Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay)),
        observation_result: Some(Ok(AntiRollbackAnchorObservation::ExactPredecessor)),
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut anchor)
        .expect("predecessor permits one observation commit");
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("a concurrent anchor successor is not this call's completion evidence");

    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::NotAdvanced(observation)
            if matches!(observation.as_ref(), AntiRollbackAnchorObservation::ExactSuccessor)
    ));
    assert_eq!(store.commits, 1);
    assert_eq!(store.read_backs, 1);
    assert_eq!(anchor.observations, 1);
    assert_eq!(anchor.advances, 1);
}

#[test]
fn conflicting_handle_content_keeps_the_same_observation_index_slot() {
    let original = OperationFixture::exact();
    let conflicting = OperationFixture::with_handle_identity("other-ref", "operation-1", 121);
    assert_eq!(
        original.handle.operation_handle_id(),
        conflicting.handle.operation_handle_id()
    );
    assert_ne!(original.handle_ref, conflicting.handle_ref);
    assert_ne!(
        original.handle.canonical_operation_handle_sha256(),
        conflicting.handle.canonical_operation_handle_sha256()
    );

    let project_mutation = |fixture: &OperationFixture| {
        let checkpoints = CheckpointPair::exact();
        let proof_seen = Cell::new(false);
        let key_seen = RefCell::new(None);
        let verifier = RecordingCheckpoint {
            fixture,
            proof_seen: &proof_seen,
            key_seen: &key_seen,
        };
        let prepared = prepare_exact(fixture, &checkpoints, &verifier);
        let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
        let _pending = commit_observation(prepared, &mut store).expect("record physical candidate");
        store
            .candidate
            .expect("candidate captured")
            .indexes()
            .iter()
            .find_map(|index| {
                if let OperationCustodyIndexMutation::LatestObservation { key, expected, .. } =
                    index
                {
                    Some((key.clone(), expected.clone()))
                } else {
                    None
                }
            })
            .expect("latest observation mutation")
    };
    let (original_key, original_predecessor) = project_mutation(&original);
    let (conflicting_key, conflicting_predecessor) = project_mutation(&conflicting);
    assert_eq!(
        original_key, conflicting_key,
        "changed content cannot evade the existing slot"
    );
    assert_ne!(
        original_predecessor, conflicting_predecessor,
        "the exact predecessor still detects conflicting content"
    );
    assert_ne!(
        original_key,
        project_mutation(&OperationFixture::with_handle_identity(
            "1",
            "operation-2",
            120
        ))
        .0,
        "intentionally distinct handle identities retain distinct slots"
    );
}

#[test]
fn unlike_handle_lineage_is_rejected_before_checkpoint_proof_or_storage() {
    let fixture = OperationFixture::exact();
    let another = OperationFixture::named("2");
    let checkpoints = CheckpointPair::exact();
    let proof_seen = Cell::new(false);
    let key_seen = RefCell::new(None);
    let verifier = RecordingCheckpoint {
        fixture: &fixture,
        proof_seen: &proof_seen,
        key_seen: &key_seen,
    };
    let transition = OperationObservationTransition::new(
        &another.handle_ref,
        &another.handle,
        &fixture.predecessor_ref,
        &fixture.predecessor,
        &fixture.successor_ref,
        &fixture.successor,
    );
    let error = prepare_operation_observation_append(transition, &ResolveStore)
        .and_then(|append| {
            append.attach_checkpoint(
                checkpoints.transition(&verifier),
                checkpoints.physical_batch(),
            )
        })
        .expect_err("unlike handle is not one observation lineage");
    assert!(matches!(
        error,
        OperationObservationCustodyProjectionError::Semantic(_)
            | OperationObservationCustodyProjectionError::Lineage
    ));
    assert!(!proof_seen.get());
}

#[test]
fn reconciled_observation_custody_retains_unknown_predecessor_and_exact_operation() {
    let mut fixture = OperationFixture::exact();
    let authority = signing_authority();
    fixture.predecessor = issue_observation(
        &authority,
        &fixture.handle_ref,
        &fixture.handle,
        fixture.predecessor.schema_sha256().clone(),
        OperationObservationRevisionV1::new(1).expect("revision"),
        None,
        OperationObservationStateV1::OutcomeUnknown {
            diagnostic: evidence("diagnostic/uncertain", 'a'),
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/frontier"),
                record_sha256: ReconciliationFrontierDigest::parse(sha('b')).expect("frontier"),
            },
            effect_receipt: None,
        },
        81,
    );
    let original = canon::to_canonical_string(&fixture.predecessor).expect("predecessor bytes");
    fixture.successor = issue_observation(
        &authority,
        &fixture.handle_ref,
        &fixture.handle,
        fixture.successor.schema_sha256().clone(),
        OperationObservationRevisionV1::new(2).expect("revision"),
        Some((&fixture.predecessor_ref, &fixture.predecessor)),
        OperationObservationStateV1::Succeeded {
            result: CapabilityResultReferenceV1 {
                record_ref: record_ref("capability/result"),
                record_sha256: CapabilityResultDigest::parse(sha('c')).expect("result"),
            },
            effect_receipt: None,
        },
        82,
    );
    // issue_observation uses synthetic evidence admission. This proves physical
    // custody of an admitted append, not a live result/reconciliation join.
    let checkpoints = CheckpointPair::exact();
    let proof_seen = Cell::new(false);
    let key_seen = RefCell::new(None);
    let verifier = RecordingCheckpoint {
        fixture: &fixture,
        proof_seen: &proof_seen,
        key_seen: &key_seen,
    };
    let prepared = prepare_exact(&fixture, &checkpoints, &verifier);
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let pending = commit_observation(prepared, &mut store).expect("append commits");
    let mut anchor = RecordingAnchor::advancing();
    let completed = pending
        .advance_anchor(&mut anchor)
        .expect("exact checkpoint anchor");
    let candidate = completed.candidate();
    assert_eq!(candidate.records().len(), 1);
    assert_eq!(candidate.kind(), OperationCustodyTransitionKind::Observe);
    let OperationCustodyIndexMutation::LatestObservation {
        expected: OperationCustodyIndexPredecessor::Exact(predecessor),
        successor,
        ..
    } = &candidate.indexes()[0]
    else {
        panic!("exact latest-observation mutation");
    };
    assert_eq!(
        predecessor.canonical_record_jcs().as_slice(),
        original.as_bytes()
    );
    assert_eq!(successor.record_ref(), &fixture.successor_ref);
    assert_eq!(
        fixture.successor.operation_handle_id(),
        fixture.predecessor.operation_handle_id()
    );
    assert_eq!(
        fixture.successor.operation_handle_sha256(),
        fixture.predecessor.operation_handle_sha256()
    );
    assert_eq!(
        fixture.successor.predecessor_sha256().as_ref(),
        Some(fixture.predecessor.canonical_operation_observation_sha256())
    );
    assert_eq!(
        candidate.records()[0]
            .record()
            .canonical_record_jcs()
            .as_slice(),
        canon::to_canonical_string(&fixture.successor)
            .expect("successor bytes")
            .as_bytes()
    );
    assert_eq!(
        canon::to_canonical_string(&fixture.predecessor).expect("unchanged predecessor"),
        original
    );
    assert_eq!(store.commits, 1);
    assert_eq!(anchor.advances, 1);
}

#[test]
fn reused_observation_reference_is_reported_as_invalid_lineage() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let proof_seen = Cell::new(false);
    let store_seen = Cell::new(false);
    let verifier = |_: &OperationObservationCheckpointConstruction<'_>| {
        proof_seen.set(true);
        Ok(())
    };
    let resolver = |_: &OperationObservationV1| {
        store_seen.set(true);
        Ok(store_coordinate())
    };
    let transition = OperationObservationTransition::new(
        &fixture.handle_ref,
        &fixture.handle,
        &fixture.predecessor_ref,
        &fixture.predecessor,
        &fixture.predecessor_ref,
        &fixture.successor,
    );
    let error = prepare_operation_observation_append(transition, &resolver)
        .and_then(|append| {
            append.attach_checkpoint(
                checkpoints.transition(&verifier),
                checkpoints.physical_batch(),
            )
        })
        .expect_err("one immutable reference cannot identify both observations");

    assert!(matches!(
        error,
        OperationObservationCustodyProjectionError::Lineage
    ));
    assert!(!store_seen.get());
    assert!(!proof_seen.get());
}

#[test]
fn checkpoint_owner_rejection_precedes_storage() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let error = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
        .and_then(|append| {
            append.attach_checkpoint(
                checkpoints.transition(&RejectCheckpoint),
                checkpoints.physical_batch(),
            )
        })
        .expect_err("checkpoint owner rejects unlike accumulator");
    assert!(matches!(
        error,
        OperationObservationCustodyProjectionError::Checkpoint(_)
    ));
}

#[test]
fn checkpoint_attachment_rejects_unlike_pair_before_construction_verification() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let wrong_predecessor_ref = record_ref("operation/unlike-checkpoint");
    let proof_seen = Cell::new(false);
    let verifier = |_: &OperationObservationCheckpointConstruction<'_>| {
        proof_seen.set(true);
        Ok(())
    };
    let append = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
        .expect("observation prepares independently");
    let error = append
        .attach_checkpoint(
            OperationObservationCheckpointTransition::new(
                &wrong_predecessor_ref,
                &checkpoints.predecessor,
                &checkpoints.successor_ref,
                &checkpoints.successor,
                &verifier,
            ),
            checkpoints.physical_batch(),
        )
        .expect_err("the supplied pair must have exact predecessor linkage");
    assert!(matches!(
        error,
        OperationObservationCustodyProjectionError::Semantic(_)
    ));
    assert!(!proof_seen.get());

    let wrong_incarnation = |_: &OperationObservationV1| {
        Ok(AuthorityStoreCoordinate::new(
            store_coordinate().store_id().clone(),
            ControlIncarnationIdV1::parse("incarnation-2").expect("another incarnation"),
        ))
    };
    let append = prepare_operation_observation_append(fixture.transition(), &wrong_incarnation)
        .expect("the test resolver selects a distinct incarnation");
    assert!(matches!(
        append.attach_checkpoint(
            checkpoints.transition(&verifier),
            checkpoints.physical_batch()
        ),
        Err(OperationObservationCustodyProjectionError::StoreScope)
    ));
    assert!(!proof_seen.get());
}

#[test]
fn checkpoint_attachment_presents_the_retained_append_not_another_observation() {
    let expected = OperationFixture::exact();
    let other = OperationFixture::named("2");
    let expected_append =
        prepare_operation_observation_append(expected.transition(), &ResolveStore)
            .expect("expected append");
    let other_append = prepare_operation_observation_append(other.transition(), &ResolveStore)
        .expect("different admitted append");
    let checkpoints = CheckpointPair::exact();
    let verifier = |construction: &OperationObservationCheckpointConstruction<'_>| {
        if construction.successor_record() == expected_append.successor_record()
            && construction.latest_observation_mutation()
                == expected_append.latest_observation_mutation()
        {
            Ok(())
        } else {
            Err(OperationObservationCheckpointConstructionRejection)
        }
    };
    assert!(matches!(
        other_append.attach_checkpoint(
            checkpoints.transition(&verifier),
            checkpoints.physical_batch()
        ),
        Err(OperationObservationCustodyProjectionError::Checkpoint(_))
    ));
    // Dropping the unused preparation neither stores an observation nor
    // establishes a checkpoint. No storage/anchor port enters either phase.
    drop(expected_append);
}

#[test]
fn unresolved_runtime_authority_cannot_select_a_store() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let reject = |_: &OperationObservationV1| Err(OperationObservationAuthorityStoreRejection);
    let error = prepare_operation_observation_append(fixture.transition(), &reject)
        .and_then(|append| {
            append.attach_checkpoint(
                checkpoints.transition(&AcceptCheckpoint),
                checkpoints.physical_batch(),
            )
        })
        .expect_err("runtime authority must resolve before custody projection");
    assert!(matches!(
        error,
        OperationObservationCustodyProjectionError::RuntimeAuthority(_)
    ));
}

#[test]
fn authority_derived_store_must_match_both_checkpoints() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let wrong_store = |_: &OperationObservationV1| {
        Ok(AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("another-store").expect("store"),
            ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation"),
        ))
    };
    let error = prepare_operation_observation_append(fixture.transition(), &wrong_store)
        .and_then(|append| {
            append.attach_checkpoint(
                checkpoints.transition(&AcceptCheckpoint),
                checkpoints.physical_batch(),
            )
        })
        .expect_err("checkpoint custody must match the authority-owned store");
    assert!(matches!(
        error,
        OperationObservationCustodyProjectionError::StoreScope
    ));
}

#[test]
fn ambiguous_commit_performs_one_exact_read_back_and_never_retries() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore {
        store: store_coordinate(),
        preparation: None,
        commit_result: Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))),
        follow_up_commit_results: VecDeque::new(),
        read_back_results: VecDeque::from([
            Ok(OperationCustodyReadBackOutcome::Predecessor),
            Ok(OperationCustodyReadBackOutcome::ExactSuccessor),
        ]),
        candidate: None,
        commits: 0,
        read_backs: 0,
        index_reads: VecDeque::new(),
        index_queries: Vec::new(),
        index_read_counter: None,
        index_error: None,
        unlisted_indexes_are_absent: false,
        checkpoint_reads: VecDeque::new(),
        checkpoint_queries: Vec::new(),
        lease_retentions: VecDeque::new(),
        retained_lease: None,
        session_basis_override: None,
        recover_indexes_from_committed_candidate: false,
    };
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("one exact read-back proves the append");
    assert_eq!(
        pending.database_evidence(),
        OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit
    );
    assert_eq!(store.commits, 1);
    assert_eq!(store.read_backs, 2);
}

#[test]
fn candidate_conflict_retains_the_prepared_append_without_replay_claim() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore {
        store: store_coordinate(),
        preparation: None,
        commit_result: Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::CandidateConflict,
        ))),
        follow_up_commit_results: VecDeque::new(),
        read_back_results: VecDeque::from([Ok(OperationCustodyReadBackOutcome::Predecessor)]),
        candidate: None,
        commits: 0,
        read_backs: 0,
        index_reads: VecDeque::new(),
        index_queries: Vec::new(),
        index_read_counter: None,
        index_error: None,
        unlisted_indexes_are_absent: false,
        checkpoint_reads: VecDeque::new(),
        checkpoint_queries: Vec::new(),
        lease_retentions: VecDeque::new(),
        retained_lease: None,
        session_basis_override: None,
        recover_indexes_from_committed_candidate: false,
    };
    let failure = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect_err("a later successor needs explicit recovery");
    assert!(matches!(
        failure.reason(),
        OperationObservationCustodyCommitFailureReason::Database(
            OperationCustodyDatabaseFailureReason::Store(error)
        )
            if error.kind() == ControlStoreErrorKind::CandidateConflict
    ));
    let _failure = failure
        .try_into_retryable()
        .expect_err("a competing candidate is not retry proof");
    assert_eq!(store.commits, 1);
    assert_eq!(store.read_backs, 1);
}

#[test]
fn failed_ambiguity_read_back_cannot_recover_commit_capability() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore {
        store: store_coordinate(),
        preparation: None,
        commit_result: Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))),
        follow_up_commit_results: VecDeque::new(),
        read_back_results: VecDeque::from([
            Ok(OperationCustodyReadBackOutcome::Predecessor),
            Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
        ]),
        candidate: None,
        commits: 0,
        read_backs: 0,
        index_reads: VecDeque::new(),
        index_queries: Vec::new(),
        index_read_counter: None,
        index_error: None,
        unlisted_indexes_are_absent: false,
        checkpoint_reads: VecDeque::new(),
        checkpoint_queries: Vec::new(),
        lease_retentions: VecDeque::new(),
        retained_lease: None,
        session_basis_override: None,
        recover_indexes_from_committed_candidate: false,
    };
    let failure = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect_err("failed ambiguity read-back remains uncertain");
    assert!(matches!(
        failure.reason(),
        OperationObservationCustodyCommitFailureReason::Database(
            OperationCustodyDatabaseFailureReason::ReadBackFailed { .. }
        )
    ));
    let _failure = failure
        .try_into_retryable()
        .expect_err("uncertainty cannot become commit capability");
    assert_eq!(store.commits, 1);
    assert_eq!(store.read_backs, 2);
}

#[test]
fn exact_database_replay_observes_the_anchor_before_any_advance() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::ExactReplay);
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("exact database replay");
    let mut anchor = RecordingAnchor {
        advance_result: None,
        observation_result: Some(Ok(AntiRollbackAnchorObservation::ExactSuccessor)),
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let completed = pending
        .advance_anchor(&mut anchor)
        .expect("exact anchor replay");
    assert_eq!(
        completed.anchor_evidence(),
        OperationCustodyAnchorEvidence::ExactReplay
    );
    assert_eq!(anchor.observations, 1);
    assert_eq!(anchor.advances, 0);
    assert_eq!(store.commits, 0);
}

#[test]
fn exact_database_replay_rejects_a_later_anchor_without_ancestry_proof() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::ExactReplay);
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("exact database replay");
    let mut anchor = RecordingAnchor {
        advance_result: None,
        observation_result: Some(Ok(AntiRollbackAnchorObservation::Ahead {
            observed: operation_anchor(3, "operation/checkpoint-3", '3'),
        })),
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("ordering alone cannot prove checkpoint ancestry");
    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::NotAdvanced(observation)
            if matches!(observation.as_ref(), AntiRollbackAnchorObservation::Ahead { .. })
    ));
    let _failure = failure
        .try_into_retryable()
        .expect_err("an unproved later anchor is not retry proof");
    assert_eq!(anchor.observations, 1);
    assert_eq!(anchor.advances, 0);
    assert_eq!(store.commits, 0);
}

#[test]
fn failed_anchor_retains_the_database_committed_phase() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("database custody");
    let mut anchor = RecordingAnchor {
        advance_result: Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))),
        observation_result: None,
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("unavailable anchor cannot complete custody");
    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::Port(_)
    ));
    assert_eq!(
        failure
            .try_into_retryable()
            .expect("unavailable anchor did not cross an ambiguous boundary")
            .database_evidence(),
        OperationCustodyDatabaseEvidence::New
    );
}

#[test]
fn later_anchor_without_ancestry_proof_cannot_complete_custody() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("database custody");
    let mut anchor = RecordingAnchor {
        advance_result: Some(Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(
            AntiRollbackAnchorObservation::Ahead {
                observed: operation_anchor(3, "operation/checkpoint-3", '3'),
            },
        ))),
        observation_result: None,
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("ordering alone cannot prove checkpoint ancestry");
    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::NotAdvanced(observation)
            if matches!(observation.as_ref(), AntiRollbackAnchorObservation::Ahead { .. })
    ));
    let _failure = failure
        .try_into_retryable()
        .expect_err("an unproved later anchor is not retry proof");
}

#[test]
fn same_position_different_anchor_coordinate_is_conflicting() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("database custody");
    let mut anchor = RecordingAnchor {
        advance_result: Some(Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(
            AntiRollbackAnchorObservation::Ahead {
                observed: operation_anchor(2, "operation/checkpoint-other", '7'),
            },
        ))),
        observation_result: None,
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("same position with unlike identity is contradictory");
    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::NotAdvanced(observation)
            if matches!(observation.as_ref(), AntiRollbackAnchorObservation::Conflicting { .. })
    ));
    let _failure = failure
        .try_into_retryable()
        .expect_err("contradictory anchor is not retry proof");
}

#[test]
fn ambiguous_anchor_advance_recovers_only_from_exact_observation() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("database custody");
    let mut anchor = RecordingAnchor {
        advance_result: Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::OutcomeUnknown,
        ))),
        observation_result: Some(Ok(AntiRollbackAnchorObservation::ExactSuccessor)),
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let completed = pending
        .advance_anchor(&mut anchor)
        .expect("exact read-back resolves ambiguous anchor advance");
    assert_eq!(
        completed.anchor_evidence(),
        OperationCustodyAnchorEvidence::RecoveredExactAfterAmbiguousAdvance
    );
    assert_eq!(anchor.advances, 1);
    assert_eq!(anchor.observations, 1);
}

#[test]
fn ambiguous_anchor_advance_rejects_a_later_value_without_ancestry_proof() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("database custody");
    let mut anchor = RecordingAnchor {
        advance_result: Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::OutcomeUnknown,
        ))),
        observation_result: Some(Ok(AntiRollbackAnchorObservation::Ahead {
            observed: operation_anchor(3, "operation/checkpoint-3", '3'),
        })),
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("ordering cannot resolve ambiguous checkpoint ancestry");
    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::AmbiguousNotExact { observation, .. }
            if matches!(observation.as_ref(), AntiRollbackAnchorObservation::Ahead { .. })
    ));
    let _failure = failure
        .try_into_retryable()
        .expect_err("ambiguous advancement cannot become retry authority");
}

#[test]
fn failed_observation_after_ambiguous_anchor_cannot_retry_the_advance() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("database custody");
    let mut anchor = RecordingAnchor {
        advance_result: Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::OutcomeUnknown,
        ))),
        observation_result: Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))),
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("failed ambiguity observation remains uncertain");
    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::AmbiguousObservationFailed { .. }
    ));
    let _failure = failure
        .try_into_retryable()
        .expect_err("anchor uncertainty cannot become advance capability");
    assert_eq!(anchor.advances, 1);
    assert_eq!(anchor.observations, 1);
}

#[test]
fn later_anchor_claim_from_another_store_cannot_complete_custody() {
    let fixture = OperationFixture::exact();
    let checkpoints = CheckpointPair::exact();
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let pending = prepare_exact(&fixture, &checkpoints, &AcceptCheckpoint)
        .commit(&mut store, &mut RecordingAnchor::predecessor())
        .expect("database custody");
    let conflicting = AntiRollbackAnchorObservation::Ahead {
        observed: AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
            ControlStoreIdV1::parse("another-store").expect("store"),
            ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation"),
            ControlCheckpointDomainIdV1::parse("operation:principal-1").expect("checkpoint domain"),
            ControlDomainEpochV1::new(1).expect("epoch"),
            ControlDomainSequenceV1::new(3).expect("sequence"),
            ControlRecordCoordinate::new(
                record_ref("operation/checkpoint-3"),
                pale_ports::ControlRecordDigest::parse(sha('3')).expect("digest"),
            ),
        )),
    };
    let mut anchor = RecordingAnchor {
        advance_result: Some(Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(
            conflicting,
        ))),
        observation_result: None,
        follow_up_observation_result: None,
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("wrong-lineage later anchor is contradictory");
    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::NotAdvanced(observation)
            if matches!(observation.as_ref(), AntiRollbackAnchorObservation::Conflicting { .. })
    ));
}
