//! Host-native provider invocation across real `SQLite` close and reopen.
//!
//! Signing, authority, provider evidence, and the independent anchor remain
//! explicit test models. These tests qualify custody composition only; they do
//! not select production authority or make the provider publicly callable.

use super::*;
use crate::resident_fixture::local_operation_start_fixture::{
    artifact_digest, sha, with_start_context,
};
use crate::resident_fixture::session_admission_fixture::{
    local_start_contract_set, LocalStartContext,
};
use pale_application::provider_invocation::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Success,
    ContextBoundSuccess,
    SafeFailure,
    RecordingRetry,
    TerminalCheckpointSigningRetry,
    PreparationTransactionDrift,
    CommitTransactionDrift,
    CancellationAfterDispatch,
    SafeFailureAfterCancellation,
    ColdRestart,
    ColdRestartAfterCancellation,
    ColdRestartAfterCancellationObservationSubstitution,
    ColdRestartAfterCancellationRequestSubstitution,
    SubstitutedReference,
    StaleAuthority,
    PreReleaseResultTime,
}

struct Store(AuthorityStoreCoordinate);

impl OperationObservationAuthorityStoreResolver for Store {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(self.0.clone())
    }
}

struct SequenceClock {
    times: Vec<u64>,
    next: AtomicUsize,
}

impl WallClockSourcePort for SequenceClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        Ok(UnixTimeV1::new(self.times[index]).unwrap())
    }
}

#[derive(Clone)]
struct SharedAnchor(Rc<RefCell<IndependentAnchor>>);

impl Default for SharedAnchor {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(IndependentAnchor::default())))
    }
}

impl SharedAnchor {
    fn advance_count(&self) -> usize {
        self.0.borrow().advance_count()
    }
}

impl AntiRollbackAnchorPort for SharedAnchor {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.0.borrow_mut().read(key)
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.0.borrow_mut().compare_and_advance(transition)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.0.borrow_mut().observe(transition)
    }
}

struct CancellationRace {
    prepared: PreparedOperationObservationCustody,
    file: PathBuf,
    coordinate: AuthorityStoreCoordinate,
    anchor: SharedAnchor,
}

struct ResultPort {
    observation: ProviderInvocationObservation,
    calls: Cell<usize>,
    cancellation: RefCell<Option<CancellationRace>>,
}

impl ProviderInvocationPort for ResultPort {
    fn invoke(&self, _: ProviderInvocationRequest<'_>) -> ProviderInvocationObservation {
        self.calls.set(self.calls.get() + 1);
        if let Some(mut race) = self.cancellation.borrow_mut().take() {
            let mut connection = connection(&race.file);
            let mut custody = SqliteOperationCustody {
                connection: &mut connection,
                coordinate: race.coordinate,
            };
            race.prepared
                .commit(&mut custody, &mut race.anchor)
                .unwrap()
                .advance_anchor(&mut race.anchor)
                .unwrap();
        }
        self.observation.clone()
    }
}

struct TransactionDriftCustody<'port, 'connection> {
    inner: &'port mut SqliteOperationCustody<'connection>,
    file: PathBuf,
    coordinate: AuthorityStoreCoordinate,
    lease: OperationLeaseV1,
    inject: Option<TransactionDriftPoint>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransactionDriftPoint {
    Preparation,
    Commit,
}

impl TransactionDriftCustody<'_, '_> {
    fn inject_at(&mut self, point: TransactionDriftPoint) {
        if self.inject != Some(point) {
            return;
        }
        self.inject = None;
        let mut connection = connection(&self.file);
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: self.coordinate.clone(),
        };
        retain_operation_lease_body(
            &mut custody,
            &self.coordinate,
            &record_ref("provider-service/unrelated-lease"),
            &self.lease,
            1 << 20,
        )
        .unwrap();
    }
}

impl ControlTransactionPort for TransactionDriftCustody<'_, '_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        let prepared = self.inner.prepare_transaction(store)?;
        self.inject_at(TransactionDriftPoint::Preparation);
        Ok(prepared)
    }
}

impl OperationCustodyPort for TransactionDriftCustody<'_, '_> {
    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &OperationA2aBindingScope,
        cursor: Option<&OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationA2aBindings, ControlStoreError> {
        self.inner.read_a2a_task_bindings(
            store,
            scope,
            cursor,
            maximum_bindings,
            maximum_body_bytes,
        )
    }

    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationPublicationSources, OperationPublicationReadError> {
        self.inner
            .read_publication_sources(store, cursor, maximum_sources, maximum_body_bytes)
    }

    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &ControlCheckpointDomainIdV1,
        maximum_record_bytes: usize,
    ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        self.inner
            .read_checkpoint_head(store, domain, maximum_record_bytes)
    }

    fn verify_accumulator_membership(
        &mut self,
        request: &OperationAccumulatorMembership,
    ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
        self.inner.verify_accumulator_membership(request)
    }

    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        self.inner.prepare_accumulator(request)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        store: &AuthorityStoreCoordinate,
        record: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.inner.compare_and_retain_operation_lease(store, record)
    }

    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        self.inner
            .read_operation_record(store, coordinate, maximum_body_bytes)
    }

    fn compare_and_commit(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.inject_at(TransactionDriftPoint::Commit);
        self.inner.compare_and_commit(candidate)
    }

    fn observe_exact(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        self.inner.observe_exact(candidate)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        self.inner.read_index(store, query, maximum_record_bytes)
    }
}

struct Evidence;

impl ProviderInvocationEvidenceAdmission for Evidence {
    fn admit_evidence_frontier(
        &self,
        frontier: &ProviderInvocationEvidenceFrontier,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(
            frontier.evidence_ref(),
            &record_ref("provider-service/evidence")
        );
        Ok(())
    }

    fn resolve_immutable(&self, _: &ResidentRecordRefV1) -> Option<Vec<u8>> {
        None
    }

    fn admit_payload(
        &self,
        schema: &SchemaId,
        digest: &SchemaDigest,
        bytes: &[u8],
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(schema.as_str(), "example/result.v1");
        assert_eq!(digest.as_str(), sha('2'));
        assert_eq!(bytes, br#"{"accepted":true}"#);
        Ok(())
    }

    fn admit_failure_class(
        &self,
        class: AdapterFailureClassV1,
        retry_at: Option<UnixTimeV1>,
        _: &RequestBindingV1,
        _: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(class, AdapterFailureClassV1::Unavailable);
        assert_eq!(retry_at, None);
        Ok(())
    }

    fn admit_absence_evidence(
        &self,
        _: &ResidentRecordRefV1,
        _: &ArtifactDigest,
        _: &[u8],
        _: &RequestBindingV1,
        _: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("host-native compute does not assert external-effect absence")
    }

    fn verify_result_signature(
        &self,
        domain: &[u8],
        preimage: &CapabilityResultPreimageV1,
        digest: &CapabilityResultDigest,
        actual: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
        assert_eq!(
            digest,
            &CapabilityResultDigest::hash_canonical(preimage).unwrap()
        );
        assert_eq!(actual, &signature());
        Ok(())
    }

    fn verify_failure_signature(
        &self,
        domain: &[u8],
        preimage: &AdapterFailurePreimageV1,
        digest: &AdapterFailureDigest,
        actual: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(domain, ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN);
        assert_eq!(
            digest,
            &AdapterFailureDigest::hash_canonical(preimage).unwrap()
        );
        assert_eq!(actual, &signature());
        Ok(())
    }
}

fn signed_wire<P, D, W>(preimage: &P, digest_field: &str, digest: &D) -> W
where
    P: serde::Serialize,
    D: serde::Serialize,
    W: serde::de::DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert(digest_field.into(), serde_json::to_value(digest).unwrap());
    value.as_object_mut().unwrap().insert(
        "signature".into(),
        serde_json::to_value(signature()).unwrap(),
    );
    serde_json::from_value(value).unwrap()
}

fn provider_admission<'a>(
    result: CapabilityResultAdmissionContextV1<'a>,
    contracts: &'a ContractSetV1,
    failure_schema: &'a SchemaDigest,
) -> ProviderInvocationAdmissionContext<'a> {
    ProviderInvocationAdmissionContext::new(
        CapabilitySuccessAdmissionContextV1 {
            contract_set_ref: result.expected_request_binding.contract_set_ref(),
            result,
            contract_set: contracts,
        },
        AdapterFailureAdmissionContextV1 {
            expected_schema_sha256: failure_schema,
            expected_request_binding: result.expected_request_binding,
            expected_attempt_id: result.expected_attempt_id,
            expected_implementation_kind: result.expected_implementation_kind,
            expected_implementation_version: result.expected_implementation_version,
            signing_authority: result.signing_authority,
        },
        ProviderInvocationSafeFailureProof::BeforeDispatch,
    )
}

fn provider_coordinate(
    invocation: ProviderInvocationRequest<'_>,
) -> ProviderInvocationObservationCoordinate {
    let frontier = ProviderInvocationEvidenceFrontier::new(
        ProviderInvocationEvidenceSubject::from_request(invocation),
        record_ref("provider-service/evidence"),
        artifact_digest('5'),
        UnixTimeV1::new(65).unwrap(),
        UnixTimeV1::new(75).unwrap(),
    )
    .unwrap();
    ProviderInvocationObservationCoordinate::capture(invocation, frontier).unwrap()
}

#[allow(clippy::too_many_arguments)]
fn success_wire(
    invocation: ProviderInvocationRequest<'_>,
    request: &CommonRequestEnvelopeV1,
    signing: &SigningAuthorityBindingV1,
    attempt: &CapabilityAttemptIdV1,
    kind: &BoundedResidentTextV1,
    version: &BoundedResidentTextV1,
    contracts: &ContractSetV1,
    started_at: UnixTimeV1,
) -> ProviderInvocationSuccessWire {
    let binding = request.local_request_binding().unwrap();
    let schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
    let context = CapabilitySuccessAdmissionContextV1 {
        result: CapabilityResultAdmissionContextV1 {
            expected_schema_sha256: &schema,
            expected_request_binding: &binding,
            expected_attempt_id: attempt,
            expected_implementation_kind: kind,
            expected_implementation_version: version,
            signing_authority: signing,
        },
        contract_set_ref: binding.contract_set_ref(),
        contract_set: contracts,
    };
    let preimage = CapabilityResultPreimageV1 {
        schema_id: SchemaId::parse(CAPABILITY_RESULT_V1_SCHEMA).unwrap(),
        schema_sha256: schema.clone(),
        request_binding: binding.clone(),
        attempt_id: attempt.clone(),
        status: CapabilityResultStatusV1::Succeeded,
        success: Some(
            CapabilitySuccessV1::inline(
                SchemaId::parse("example/result.v1").unwrap(),
                SchemaDigest::parse(sha('2')).unwrap(),
                serde_json::json!({"accepted": true}),
            )
            .unwrap(),
        ),
        failure: None,
        implementation_kind: kind.clone(),
        implementation_version: version.clone(),
        started_at,
        completed_at: UnixTimeV1::new(70).unwrap(),
        signing_authority_binding: signing.clone(),
        issuer_principal_id: signing.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(signing.signer_key_id()).unwrap(),
        issuer_key_epoch: *signing.signer_key_epoch(),
    };
    let result = CapabilityResultV1::issue_success_contextual(
        preimage,
        &context,
        |_| None,
        |selected_schema, digest, bytes| Evidence.admit_payload(selected_schema, digest, bytes),
        |_, _, _| Ok(signature()),
    )
    .unwrap();
    let wire = serde_json::from_value(serde_json::to_value(result).unwrap()).unwrap();
    ProviderInvocationSuccessWire::new(provider_coordinate(invocation), wire)
}

fn failure_wire(
    invocation: ProviderInvocationRequest<'_>,
    request: &CommonRequestEnvelopeV1,
    signing: &SigningAuthorityBindingV1,
    attempt: &CapabilityAttemptIdV1,
    kind: &BoundedResidentTextV1,
    version: &BoundedResidentTextV1,
) -> ProviderInvocationFailureWire {
    let binding = request.local_request_binding().unwrap();
    let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
    let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
    let failure_preimage = AdapterFailurePreimageV1 {
        schema_id: SchemaId::parse(ADAPTER_FAILURE_V1_SCHEMA).unwrap(),
        schema_sha256: failure_schema,
        request_binding: binding.clone(),
        failure_id: AdapterFailureIdV1::parse("provider-safe-failure").unwrap(),
        attempt_id: attempt.clone(),
        failure_class: AdapterFailureClassV1::Unavailable,
        stable_code: ResidentReasonV1::parse("provider_unavailable").unwrap(),
        bounded_message: None,
        provider_status: None,
        retry_at: None,
        effect_state: AdapterFailureEffectStateV1::NotStarted,
        absence_evidence_ref: None,
        absence_evidence_sha256: None,
        implementation_kind: kind.clone(),
        implementation_version: version.clone(),
        recorded_at: UnixTimeV1::new(70).unwrap(),
        signing_authority_binding: signing.clone(),
        issuer_principal_id: signing.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(signing.signer_key_id()).unwrap(),
        issuer_key_epoch: *signing.signer_key_epoch(),
    };
    let failure_digest = AdapterFailureDigest::hash_canonical(&failure_preimage).unwrap();
    let failure = signed_wire(
        &failure_preimage,
        "canonical_failure_sha256",
        &failure_digest,
    );
    let failure_ref = record_ref("provider-service/adapter-failure");
    let result_preimage = CapabilityResultPreimageV1 {
        schema_id: SchemaId::parse(CAPABILITY_RESULT_V1_SCHEMA).unwrap(),
        schema_sha256: result_schema,
        request_binding: binding,
        attempt_id: attempt.clone(),
        status: CapabilityResultStatusV1::Failed,
        success: None,
        failure: Some(
            serde_json::from_value(serde_json::json!({
                "failure_ref": failure_ref,
                "failure_sha256": failure_digest,
            }))
            .unwrap(),
        ),
        implementation_kind: kind.clone(),
        implementation_version: version.clone(),
        started_at: UnixTimeV1::new(67).unwrap(),
        completed_at: UnixTimeV1::new(70).unwrap(),
        signing_authority_binding: signing.clone(),
        issuer_principal_id: signing.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(signing.signer_key_id()).unwrap(),
        issuer_key_epoch: *signing.signer_key_epoch(),
    };
    let result_digest = CapabilityResultDigest::hash_canonical(&result_preimage).unwrap();
    let result = signed_wire(&result_preimage, "canonical_result_sha256", &result_digest);
    ProviderInvocationFailureWire::new(
        provider_coordinate(invocation),
        failure_ref,
        failure,
        result,
    )
}

fn run_case(case: Case) {
    with_host_generation("bounded_compute", |generation, effective| {
        let start_context = if case == Case::ContextBoundSuccess {
            LocalStartContext::EvidenceOnly
        } else {
            LocalStartContext::SelfContained
        };
        with_start_context(
            generation,
            effective,
            &format!("provider-sqlite-{case:?}"),
            |_| {},
            StartAdmission::AcceptedOwnerModel,
            start_context,
            |prepared, request, owner_calls, record_signatures, parents| {
                assert_eq!((owner_calls, record_signatures), (1, 2));
                let directory = crate::test_support::private_tempdir();
                let binding = parents.current.deployment_binding;
                let scope = ControlStoreScope::Deployment(DeploymentStoreScope::new(
                    ResidentWorkspaceIdV1::parse(
                        binding.deployment_terms().workspace_id().as_str(),
                    )
                    .unwrap(),
                    parents.current.deployment_binding_ref.clone(),
                    ArtifactDigest::parse(binding.canonical_binding_sha256().as_str()).unwrap(),
                ));
                let location = ControlStorageRoot::retain(directory.path())
                    .unwrap()
                    .prepare_store_location(scope.clone())
                    .unwrap();
                let coordinate = store_coordinate();
                let initialized = initialize_unadmitted_store(
                    &location,
                    ControlStoreIdentity::new(
                        scope,
                        coordinate.store_id().clone(),
                        coordinate.incarnation_id().clone(),
                    ),
                )
                .unwrap();
                let file = initialized.path().to_path_buf();
                let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                    &location,
                    coordinate.clone(),
                )
                .unwrap();
                let session = parents.session.lease();
                let session_body = SessionLeaseBody::new(
                    coordinate.clone(),
                    session.session_admission_lease_digest().clone(),
                    session.schema_id().clone(),
                    session.schema_sha256().clone(),
                    ControlRecordBytes::new(
                        pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(
                            session,
                        ))
                        .unwrap()
                        .into_bytes(),
                    )
                    .unwrap(),
                );
                assert_eq!(
                    sessions.compare_and_retain(&session_body).unwrap(),
                    SessionLeaseBodyRetention::Retained
                );
                let mut sql = connection(&file);
                let mut custody = SqliteOperationCustody {
                    connection: &mut sql,
                    coordinate: coordinate.clone(),
                };
                retain_operation_lease_body(
                    &mut custody,
                    &coordinate,
                    &record_ref("start-lease"),
                    parents.lease,
                    1 << 20,
                )
                .unwrap();
                let prepared = prepared
                    .unwrap()
                    .prepare_checkpoint(
                        AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4_096, 100_000)
                            .unwrap(),
                        &mut custody,
                    )
                    .unwrap();
                let mut checkpoint_fields = checkpoint_preimage(1, None);
                checkpoint_fields.checkpoint_domain_id =
                    SupportedOperationCheckpointProfile::conflict_domain(request).unwrap();
                checkpoint_fields.checkpoint_profile_sha256 =
                    SupportedOperationCheckpointProfile::content_digest().unwrap();
                checkpoint_fields.committed_transaction_sequence = prepared.transaction_sequence();
                checkpoint_fields.state_root_sha256 = prepared.state_root().clone();
                checkpoint_fields.unresolved_custody_root_sha256 =
                    prepared.unresolved_root().clone();
                checkpoint_fields.changed_leaf_keys = prepared.changed_keys().to_vec();
                checkpoint_fields.leaf_count = prepared.leaf_count();
                checkpoint_fields.issued_at = UnixTimeV1::new(61).unwrap();
                let (physical, pending) = prepared
                    .issue_checkpoint(
                        &record_ref("provider-service/checkpoint-1"),
                        checkpoint_fields.clone(),
                        &checkpoint_fields.schema_sha256,
                        &checkpoint_authority(&checkpoint_fields),
                        |_, _, _| Ok(signature()),
                    )
                    .unwrap();
                let mut anchor = SharedAnchor::default();
                let retained = physical
                    .commit(&mut custody, &mut anchor, 1 << 20)
                    .unwrap()
                    .advance_anchor(&mut anchor)
                    .unwrap();
                let initial_candidate = retained.candidate().clone();
                let initial_checkpoint = read_checkpoint(
                    initial_candidate.checkpoint().successor_record(),
                    &checkpoint_fields,
                );
                let operation = pending
                    .finish(retained)
                    .unwrap_or_else(|_| panic!("exact provider operation custody"));
                let (handle_ref, handle) = operation.handle();
                let first_record = initial_candidate
                    .record(OperationCustodyRecordKind::OperationObservation)
                    .unwrap();
                let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                let first = issue_observation(
                    parents.signing,
                    handle_ref,
                    handle,
                    observation_schema.clone(),
                    OperationObservationRevisionV1::new(1).unwrap(),
                    None,
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Resolving,
                    },
                    60,
                );
                assert_eq!(
                    first_record.canonical_record_jcs().as_slice(),
                    pale_core::canon::to_canonical_string(&first)
                        .unwrap()
                        .as_bytes()
                );
                let initial_observation = OperationObservationAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: parents.signing,
                    },
                    handle_ref,
                    handle,
                    predecessor: Some((first_record.coordinate().record_ref(), &first)),
                };
                let resolver = Store(coordinate.clone());
                let context_binding = (case == Case::ContextBoundSuccess).then(|| {
                    let evidence = ImmutableRecordReferenceV1 {
                        record_ref: record_ref("provider-service/context-projection"),
                        record_sha256: ArtifactDigest::hash_bytes(
                            b"admitted provider context projection",
                        ),
                    };
                    let admissions = AtomicUsize::new(0);
                    let prepared = prepare_operation_context_binding(
                        OperationContextBindingInput {
                            operation: &operation,
                            request,
                            session: parents.session,
                            current: parents.current,
                            lease: operation.historical_operation_lease(),
                            evidence: evidence.clone(),
                            observation_ref: record_ref("provider-service/context-bound"),
                            observation: &initial_observation,
                        },
                        &mut ProvingAuthority,
                        |actual, obligations| {
                            admissions.fetch_add(1, Ordering::SeqCst);
                            assert_eq!(actual, &evidence);
                            assert_eq!(obligations, &[record_ref("evidence-a")]);
                            Ok(())
                        },
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                    )
                    .unwrap();
                    assert_eq!(admissions.load(Ordering::SeqCst), 1);
                    let (physical, fields) = super::prepare_append(
                        prepared.prepare_append(&resolver).unwrap(),
                        &operation,
                        (
                            initial_candidate
                                .checkpoint()
                                .successor()
                                .coordinate()
                                .record_ref(),
                            &initial_checkpoint,
                        ),
                        &mut custody,
                        62,
                    );
                    let completed = physical
                        .commit(&mut custody, &mut anchor)
                        .unwrap()
                        .advance_anchor(&mut anchor)
                        .unwrap();
                    assert_eq!(
                        custody.observe_exact(completed.candidate()).unwrap(),
                        OperationCustodyReadBackOutcome::ExactSuccessor
                    );
                    let retained_context: bool = custody
                        .connection
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record WHERE record_ref=?1)",
                            ["provider-service/context-projection"],
                            |row| row.get(0),
                        )
                        .unwrap();
                    assert!(
                        !retained_context,
                        "context ownership remains outside Pale custody"
                    );
                    let checkpoint_ref = completed
                        .candidate()
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_ref()
                        .clone();
                    let checkpoint = read_checkpoint(
                        completed.candidate().checkpoint().successor_record(),
                        &fields,
                    );
                    let binding = prepared
                        .finish(&completed, &resolver)
                        .expect("the exact committed context observation completed custody");
                    assert!(matches!(
                        binding.observation().1.state(),
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::ContextBound {
                                evidence: actual
                            }
                        } if actual == &evidence
                    ));
                    (binding, checkpoint_ref, checkpoint)
                });
                let observation = OperationObservationAdmissionContextV1 {
                    signed: initial_observation.signed,
                    handle_ref,
                    handle,
                    predecessor: Some(context_binding.as_ref().map_or(
                        (first_record.coordinate().record_ref(), &first),
                        |(binding, _, _)| binding.observation(),
                    )),
                };
                let checkpoint = context_binding.as_ref().map_or(
                    (
                        initial_candidate
                            .checkpoint()
                            .successor()
                            .coordinate()
                            .record_ref(),
                        &initial_checkpoint,
                    ),
                    |(_, reference, checkpoint)| (reference, checkpoint),
                );
                let request_binding = request.local_request_binding().unwrap();
                let attempt = CapabilityAttemptIdV1::parse("provider-attempt-1").unwrap();
                let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
                let version = BoundedResidentTextV1::parse("1").unwrap();
                let contracts = local_start_contract_set();
                let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
                let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
                let admission = provider_admission(
                    CapabilityResultAdmissionContextV1 {
                        expected_schema_sha256: &result_schema,
                        expected_request_binding: &request_binding,
                        expected_attempt_id: &attempt,
                        expected_implementation_kind: &kind,
                        expected_implementation_version: &version,
                        signing_authority: parents.signing,
                    },
                    &contracts,
                    &failure_schema,
                );
                let provider = generation
                    .provider_binding("provider-binding-claude-code")
                    .unwrap();
                let invocation = ProviderInvocationRequest::new(
                    ProviderInvocationCoordinate::new(
                        generation.record().compiled_generation_digest(),
                        generation.record().instance_digest(),
                        operation.historical_operation_lease().operation(),
                        &request_binding,
                        &attempt,
                    ),
                    provider,
                    &kind,
                    &version,
                )
                .unwrap();
                let clock = SequenceClock {
                    times: if matches!(
                        case,
                        Case::ColdRestartAfterCancellation
                            | Case::ColdRestartAfterCancellationObservationSubstitution
                            | Case::ColdRestartAfterCancellationRequestSubstitution
                    ) {
                        vec![62, 63, 64, 71, 72, 73, 74, 75, 76, 77]
                    } else {
                        vec![62, 63, 64, 65, 66, 67, 71, 72, 73, 74]
                    },
                    next: AtomicUsize::new(0),
                };
                let checkpoint_input = |name: &str| OperationCheckpointInput {
                    reference: record_ref(&format!("provider-service/{name}-checkpoint")),
                    id: ControlCheckpointIdV1::parse(format!("provider-service-{name}-checkpoint"))
                        .unwrap(),
                };
                let retained_dispatch = retain_provider_operation_dispatch(
                    ProviderOperationDispatchInput {
                        operation: ProviderOperationInput {
                            context: OperationAttemptContext {
                                operation: &operation,
                                request,
                                session: parents.session,
                                current: parents.current,
                                lease: operation.historical_operation_lease(),
                                context_binding: context_binding
                                    .as_ref()
                                    .map(|(binding, _, _)| binding),
                                generation,
                                selection_ref: record_ref("provider-service/selection"),
                                maximum_selection_bytes: 65_536,
                                observation_ref: record_ref("provider-service/resolution"),
                                observation,
                            },
                            invocation,
                            result_admission: admission,
                            resolution_ref: record_ref("provider-service/resolved"),
                        },
                        checkpoint,
                        checkpoint_schema: &checkpoint_fields.schema_sha256,
                        checkpoint_authority: &checkpoint_authority(&checkpoint_fields),
                        signing: observation.signed,
                        resolution_checkpoint: checkpoint_input("resolved"),
                        preparation_checkpoint: checkpoint_input("prepared"),
                        dispatch_checkpoint: checkpoint_input("dispatched"),
                        dispatch_ref: record_ref("provider-service/dispatched"),
                        clock: &clock,
                        resolve_store: &resolver,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 10_000, 10_000, 4_096, 100_000,
                        )
                        .unwrap(),
                    },
                    &mut ProvingAuthority,
                    &mut custody,
                    &mut anchor,
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap();
                let dispatch_candidate = retained_dispatch.custody().candidate().clone();
                let terminal_anchor_advances = anchor.advance_count();
                let dispatched_ref = retained_dispatch.observation().0.clone();
                let dispatched_digest = retained_dispatch
                    .observation()
                    .1
                    .canonical_operation_observation_sha256()
                    .clone();
                let dispatched = (
                    retained_dispatch.observation().0.clone(),
                    retained_dispatch.observation().1.clone(),
                );
                let selection_coordinate = match retained_dispatch.observation().1.state() {
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Dispatched { evidence, .. },
                    } => ControlRecordCoordinate::new(
                        evidence.record_ref.clone(),
                        ControlRecordDigest::parse(evidence.record_sha256.as_str()).unwrap(),
                    ),
                    _ => unreachable!("retained provider dispatch has dispatched state"),
                };
                let mut interruption = if matches!(
                    case,
                    Case::CancellationAfterDispatch
                        | Case::SafeFailureAfterCancellation
                        | Case::ColdRestartAfterCancellation
                        | Case::ColdRestartAfterCancellationObservationSubstitution
                        | Case::ColdRestartAfterCancellationRequestSubstitution
                ) {
                    let recovered = super::control_cancellation::recover_control(
                        &operation,
                        request,
                        &parents,
                        &checkpoint_fields,
                        &sessions,
                        &mut custody,
                        &mut anchor,
                        4,
                    );
                    assert!(matches!(
                        recovered.observation().selected_head().1.state(),
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Dispatched { .. }
                        }
                    ));
                    let cancel_request = super::control_cancellation::control_request(
                        request,
                        handle,
                        OPERATION_CANCEL_CAPABILITY_ID,
                        "provider-dispatched",
                        68,
                    );
                    let authorized = recovered
                        .authorize_cancellation(
                            &cancel_request,
                            UnixTimeV1::new(69).unwrap(),
                            |_| Ok(()),
                        )
                        .unwrap();
                    let cancellation = authorized
                        .prepare_dispatched_request(
                            DispatchedOperationCancellationRequestInput {
                                request_ref: record_ref(
                                    "provider-service/dispatched-cancel-request",
                                ),
                                observation_ref: record_ref(
                                    "provider-service/dispatched-cancellation-requested",
                                ),
                                signing: observation.signed,
                            },
                            |_, _, _, _| Ok(()),
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap();
                    let predecessor = cancellation.predecessor_checkpoint();
                    let prepared = cancellation
                        .prepare_checkpoint(
                            AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4_096, 100_000)
                                .unwrap(),
                            &resolver,
                            &mut custody,
                        )
                        .unwrap();
                    let mut fields = checkpoint_preimage(
                        predecessor.1.domain_sequence().get() + 1,
                        Some((
                            predecessor.0.clone(),
                            predecessor.1.canonical_control_checkpoint_sha256().clone(),
                        )),
                    );
                    fields.checkpoint_domain_id = predecessor.1.checkpoint_domain_id().clone();
                    fields.checkpoint_profile_ref = predecessor.1.checkpoint_profile_ref().clone();
                    fields.checkpoint_profile_sha256 =
                        predecessor.1.checkpoint_profile_sha256().clone();
                    fields.committed_transaction_sequence = prepared.transaction_sequence();
                    fields.state_root_sha256 = prepared.state_root().clone();
                    fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
                    fields.changed_leaf_keys = prepared.changed_keys().to_vec();
                    fields.leaf_count = prepared.leaf_count();
                    fields.issued_at = UnixTimeV1::new(70).unwrap();
                    let (prepared, _) = prepared
                        .issue_checkpoint(
                            &record_ref("provider-service/dispatched-cancel-checkpoint"),
                            fields.clone(),
                            &fields.schema_sha256,
                            &checkpoint_authority(&fields),
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap();
                    Some(CancellationRace {
                        prepared,
                        file: file.clone(),
                        coordinate: coordinate.clone(),
                        anchor: anchor.clone(),
                    })
                } else {
                    None
                };
                if matches!(
                    case,
                    Case::ColdRestartAfterCancellation
                        | Case::ColdRestartAfterCancellationObservationSubstitution
                        | Case::ColdRestartAfterCancellationRequestSubstitution
                ) {
                    let mut cancellation = interruption
                        .take()
                        .expect("cold restart fixture retains the cancellation transition");
                    cancellation
                        .prepared
                        .commit(&mut custody, &mut cancellation.anchor)
                        .unwrap()
                        .advance_anchor(&mut cancellation.anchor)
                        .unwrap();
                }
                drop(custody);
                drop(sql);

                let provider_observation = match case {
                    Case::SafeFailure | Case::SafeFailureAfterCancellation => {
                        ProviderInvocationObservation::Failed(Box::new(failure_wire(
                            invocation,
                            request,
                            parents.signing,
                            &attempt,
                            &kind,
                            &version,
                        )))
                    }
                    _ => ProviderInvocationObservation::Succeeded(Box::new(success_wire(
                        invocation,
                        request,
                        parents.signing,
                        &attempt,
                        &kind,
                        &version,
                        &contracts,
                        UnixTimeV1::new(if case == Case::PreReleaseResultTime {
                            66
                        } else {
                            67
                        })
                        .unwrap(),
                    ))),
                };
                let provider_port = ResultPort {
                    observation: provider_observation,
                    calls: Cell::new(0),
                    cancellation: RefCell::new(interruption),
                };
                let revoked_session = SessionRevocationFrontier {
                    epoch: PrincipalRevocationEpochV1::new(
                        parents.current.session_revocations.epoch.get() + 1,
                    )
                    .unwrap(),
                    revocation_ref: parents.current.session_revocations.revocation_ref,
                    revocation_sha256: parents.current.session_revocations.revocation_sha256,
                };
                let current = AmbientSessionAdmissionContext {
                    now: parents.current.now,
                    subject: parents.current.subject,
                    grant: parents.current.grant,
                    current_principal_trust: parents.current.current_principal_trust,
                    capability_revocations: parents.current.capability_revocations,
                    deployment_binding_ref: parents.current.deployment_binding_ref,
                    deployment_binding: parents.current.deployment_binding,
                    deployment_revocations: parents.current.deployment_revocations,
                    runtime_authority: parents.current.runtime_authority,
                    session_revocations: if case == Case::StaleAuthority {
                        &revoked_session
                    } else {
                        parents.current.session_revocations
                    },
                    route_use: parents.current.route_use,
                    generation: SessionAdmissionLeaseContextV1 {
                        active_head: parents.current.generation.active_head,
                        compiled_generation: parents.current.generation.compiled_generation,
                        effective_profile: parents.current.generation.effective_profile,
                        activation_fence: parents.current.generation.activation_fence,
                        signed: parents.current.generation.signed,
                    },
                };
                let terminal_ref = match case {
                    Case::SubstitutedReference => record_ref("provider-service/dispatched"),
                    Case::ColdRestartAfterCancellationObservationSubstitution => {
                        record_ref("provider-service/dispatched-cancellation-requested")
                    }
                    Case::ColdRestartAfterCancellationRequestSubstitution => {
                        record_ref("provider-service/dispatched-cancel-request")
                    }
                    _ => record_ref("provider-service/terminal"),
                };
                let invoke_input = ProviderOperationInvokeInput {
                    current: &current,
                    signing: observation.signed,
                    terminal_checkpoint: checkpoint_input("terminal"),
                    result_ref: record_ref("provider-service/result"),
                    diagnostic_ref: record_ref("provider-service/diagnostic"),
                    reconciliation: ReconciliationFrontierReferenceV1 {
                        record_ref: record_ref("provider-service/reconciliation"),
                        record_sha256: ReconciliationFrontierDigest::parse(sha('d')).unwrap(),
                    },
                    terminal_ref,
                    checkpoint_schema: &checkpoint_fields.schema_sha256,
                    checkpoint_authority: &checkpoint_authority(&checkpoint_fields),
                    clock: &clock,
                    resolve_store: &resolver,
                    maximum_record_bytes: 65_536,
                    maximum_diagnostic_bytes: 65_536,
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 10_000, 10_000, 4_096, 100_000,
                    )
                    .unwrap(),
                };
                let mut reopened = connection(&file);
                let mut reopened_custody = SqliteOperationCustody {
                    connection: &mut reopened,
                    coordinate: coordinate.clone(),
                };
                let mut invocation_custody = TransactionDriftCustody {
                    inner: &mut reopened_custody,
                    file: file.clone(),
                    coordinate: coordinate.clone(),
                    lease: parents.lease.clone(),
                    inject: match case {
                        Case::PreparationTransactionDrift => {
                            Some(TransactionDriftPoint::Preparation)
                        }
                        Case::CommitTransactionDrift => Some(TransactionDriftPoint::Commit),
                        _ => None,
                    },
                };
                if matches!(
                    case,
                    Case::ColdRestart
                        | Case::ColdRestartAfterCancellation
                        | Case::ColdRestartAfterCancellationObservationSubstitution
                        | Case::ColdRestartAfterCancellationRequestSubstitution
                ) {
                    drop(retained_dispatch);
                    let recovered = if case == Case::ColdRestart {
                        super::control_cancellation::recover_control(
                            &operation,
                            request,
                            &parents,
                            &checkpoint_fields,
                            &sessions,
                            &mut *invocation_custody.inner,
                            &mut anchor,
                            4,
                        )
                    } else {
                        super::control_cancellation::recover_control_selected(
                            &operation,
                            request,
                            &parents,
                            &checkpoint_fields,
                            &sessions,
                            &mut *invocation_custody.inner,
                            &mut anchor,
                            5,
                            OperationObservationSelection::HistoryMember {
                                reference: &dispatched_ref,
                                digest: &dispatched_digest,
                            },
                        )
                    };
                    assert_eq!(
                        recovered.observation().selected(),
                        (&dispatched.0, &dispatched.1)
                    );
                    assert!(match case {
                        Case::ColdRestart =>
                            recovered.observation().selected()
                                == recovered.observation().selected_head(),
                        Case::ColdRestartAfterCancellation
                        | Case::ColdRestartAfterCancellationObservationSubstitution
                        | Case::ColdRestartAfterCancellationRequestSubstitution => matches!(
                            recovered.observation().selected_head().1.state(),
                            OperationObservationStateV1::CancellationRequested { .. }
                        ),
                        _ => unreachable!(),
                    });
                    let selection = invocation_custody
                        .inner
                        .read_operation_record(&coordinate, &selection_coordinate, 65_536)
                        .unwrap()
                        .expect("restart reads the exact retained private selection");
                    assert_eq!(selection.coordinate(), &selection_coordinate);
                    assert!(!selection.canonical_record_jcs().as_slice().is_empty());
                    assert_eq!(provider_port.calls.get(), 0);
                    return;
                }
                let observation_signs = Cell::new(0);
                let checkpoint_signs = Cell::new(0);
                let result = invoke_provider_operation(
                    retained_dispatch,
                    invoke_input,
                    &provider_port,
                    &Evidence,
                    &mut ProvingAuthority,
                    &mut invocation_custody,
                    &mut anchor,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        observation_signs.set(observation_signs.get() + 1);
                        if case == Case::RecordingRetry && observation_signs.get() == 1 {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "provider terminal observation",
                                invariant: "test retry",
                            });
                        }
                        Ok(signature())
                    },
                    |_, _, _| {
                        checkpoint_signs.set(checkpoint_signs.get() + 1);
                        if case == Case::TerminalCheckpointSigningRetry
                            && checkpoint_signs.get() == 1
                        {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "provider terminal checkpoint",
                                invariant: "test retry",
                            });
                        }
                        Ok(signature())
                    },
                );

                if case == Case::SubstitutedReference {
                    assert!(matches!(
                        result,
                        Err(ProviderOperationInvokeFailure::Admission { .. })
                    ));
                    assert_eq!(provider_port.calls.get(), 0);
                    return;
                }
                if case == Case::StaleAuthority {
                    assert!(matches!(
                        result,
                        Err(ProviderOperationInvokeFailure::Release { reason, .. })
                            if matches!(
                                *reason,
                                OperationAttemptError::Session(
                                    AmbientSessionAdmissionError::Authority(
                                        AmbientSessionAuthorityError::SessionRevocationFrontier
                                    )
                                )
                            )
                    ));
                    assert_eq!(provider_port.calls.get(), 0);
                    return;
                }

                let result = if matches!(
                    case,
                    Case::CancellationAfterDispatch | Case::SafeFailureAfterCancellation
                ) {
                    let conflict = match result {
                        Err(ProviderOperationInvokeFailure::Recording(
                            ProviderOperationRecordingFailure::ConcurrentHead { conflict },
                        )) => conflict,
                        Err(ProviderOperationInvokeFailure::Recording(
                            ProviderOperationRecordingFailure::Preparation { reason, .. },
                        )) => panic!("cancellation race failed during preparation: {reason:?}"),
                        Err(ProviderOperationInvokeFailure::Recording(
                            ProviderOperationRecordingFailure::Custody { custody },
                        )) => panic!("cancellation race produced ambiguous custody: {custody:?}"),
                        Err(other) => panic!(
                            "the committed cancellation must be a concurrent head, got {other:?}"
                        ),
                        Ok(_) => {
                            panic!("the committed cancellation must prevent terminal commitment")
                        }
                    };
                    assert!(conflict.outcome().is_some());
                    let recovered = super::control_cancellation::recover_control_selected(
                        &operation,
                        request,
                        &parents,
                        &checkpoint_fields,
                        &sessions,
                        &mut *invocation_custody.inner,
                        &mut anchor,
                        5,
                        OperationObservationSelection::HistoryMember {
                            reference: &dispatched_ref,
                            digest: &dispatched_digest,
                        },
                    );
                    assert!(matches!(
                        recovered.observation().selected_head().1.state(),
                        OperationObservationStateV1::CancellationRequested { .. }
                    ));
                    assert!(matches!(
                        recovered.observation().selected().1.state(),
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Dispatched { .. }
                        }
                    ));
                    let continuation = conflict
                        .recover(recovered.observation())
                        .expect("protected cancellation history retains the consumed result");
                    continue_consumed_provider_recording(
                        continuation,
                        &ProviderOperationInvokeInput {
                            current: &current,
                            signing: observation.signed,
                            terminal_checkpoint: checkpoint_input("terminal"),
                            result_ref: record_ref("provider-service/result"),
                            diagnostic_ref: record_ref("provider-service/diagnostic"),
                            reconciliation: ReconciliationFrontierReferenceV1 {
                                record_ref: record_ref("provider-service/reconciliation"),
                                record_sha256: ReconciliationFrontierDigest::parse(sha('d'))
                                    .unwrap(),
                            },
                            terminal_ref: record_ref("provider-service/terminal"),
                            checkpoint_schema: &checkpoint_fields.schema_sha256,
                            checkpoint_authority: &checkpoint_authority(&checkpoint_fields),
                            clock: &clock,
                            resolve_store: &resolver,
                            maximum_record_bytes: 65_536,
                            maximum_diagnostic_bytes: 65_536,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                1, 10_000, 10_000, 4_096, 100_000,
                            )
                            .unwrap(),
                        },
                        &mut invocation_custody,
                        &mut anchor,
                        |_| Ok(()),
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                        |_, _, _| Ok(signature()),
                    )
                    .unwrap()
                } else if matches!(
                    case,
                    Case::RecordingRetry
                        | Case::TerminalCheckpointSigningRetry
                        | Case::PreparationTransactionDrift
                        | Case::CommitTransactionDrift
                ) {
                    let Err(ProviderOperationInvokeFailure::Recording(
                        ProviderOperationRecordingFailure::Preparation {
                            continuation,
                            reason,
                        },
                    )) = result
                    else {
                        panic!("pre-commit refusal must retain the consumed result")
                    };
                    assert!(continuation.outcome().is_some());
                    if case == Case::PreparationTransactionDrift {
                        assert!(matches!(
                            *reason,
                            ProviderOperationRecordingPreparationError::Projection(
                                OperationObservationCustodyProjectionError::AccumulatorPreparation(
                                    pale_ports::OperationAccumulatorPreparationError::Store(
                                        ref error
                                    )
                                )
                            ) if error.kind() == ControlStoreErrorKind::CandidateConflict
                        ));
                    } else if case == Case::CommitTransactionDrift {
                        assert!(matches!(
                            *reason,
                            ProviderOperationRecordingPreparationError::StoreTransactionDrift
                        ));
                    } else if case == Case::TerminalCheckpointSigningRetry {
                        assert!(matches!(
                            *reason,
                            ProviderOperationRecordingPreparationError::Projection(
                                OperationObservationCustodyProjectionError::Semantic(
                                    InvalidResidentRecordV1::ContextualAdmission {
                                        record: "provider terminal checkpoint",
                                        invariant: "test retry",
                                    }
                                )
                            )
                        ));
                        assert_eq!(provider_port.calls.get(), 1);
                        assert_eq!(checkpoint_signs.get(), 1);
                        assert_eq!(anchor.advance_count(), terminal_anchor_advances);
                        assert_eq!(
                            invocation_custody
                                .inner
                                .observe_exact(&dispatch_candidate)
                                .unwrap(),
                            OperationCustodyReadBackOutcome::ExactSuccessor
                        );
                        assert_eq!(
                            invocation_custody
                                .inner
                                .read_checkpoint_head(
                                    &coordinate,
                                    dispatch_candidate
                                        .checkpoint()
                                        .successor()
                                        .checkpoint_domain_id(),
                                    65_536,
                                )
                                .unwrap(),
                            Some((
                                dispatch_candidate.checkpoint().successor().clone(),
                                dispatch_candidate.checkpoint().successor_record().clone(),
                            ))
                        );
                    }
                    continue_consumed_provider_recording(
                        *continuation,
                        &ProviderOperationInvokeInput {
                            current: &current,
                            signing: observation.signed,
                            terminal_checkpoint: checkpoint_input("terminal"),
                            result_ref: record_ref("provider-service/result"),
                            diagnostic_ref: record_ref("provider-service/diagnostic"),
                            reconciliation: ReconciliationFrontierReferenceV1 {
                                record_ref: record_ref("provider-service/reconciliation"),
                                record_sha256: ReconciliationFrontierDigest::parse(sha('d'))
                                    .unwrap(),
                            },
                            terminal_ref: record_ref("provider-service/terminal"),
                            checkpoint_schema: &checkpoint_fields.schema_sha256,
                            checkpoint_authority: &checkpoint_authority(&checkpoint_fields),
                            clock: &clock,
                            resolve_store: &resolver,
                            maximum_record_bytes: 65_536,
                            maximum_diagnostic_bytes: 65_536,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                1, 10_000, 10_000, 4_096, 100_000,
                            )
                            .unwrap(),
                        },
                        &mut invocation_custody,
                        &mut anchor,
                        |_| Ok(()),
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                        |_, _, _| {
                            checkpoint_signs.set(checkpoint_signs.get() + 1);
                            Ok(signature())
                        },
                    )
                    .unwrap()
                } else {
                    result.unwrap()
                };
                assert_eq!(provider_port.calls.get(), 1);
                let candidate = if case == Case::PreReleaseResultTime {
                    let ProviderOperationInvokeResult::OutcomeUnknown(uncertain) = result else {
                        panic!("pre-release result time must be retained as outcome unknown")
                    };
                    assert!(matches!(
                        uncertain.uncertainty(),
                        ProviderOperationUncertainty::ResponseAssociation { reason, .. }
                            if matches!(reason.as_ref(), OperationAttemptError::TimeOrder)
                    ));
                    uncertain.custody().candidate().clone()
                } else {
                    let ProviderOperationInvokeResult::Completed(completed) = result else {
                        panic!("fixture provides conclusive provider evidence")
                    };
                    if matches!(
                        case,
                        Case::CancellationAfterDispatch | Case::SafeFailureAfterCancellation
                    ) {
                        assert_eq!(
                            completed.observation().1.predecessor_ref(),
                            &Some(record_ref(
                                "provider-service/dispatched-cancellation-requested"
                            )),
                            "late conclusive evidence must follow the durable cancellation request"
                        );
                        assert!(match case {
                            Case::CancellationAfterDispatch => matches!(
                                completed.observation().1.state(),
                                OperationObservationStateV1::Succeeded { .. }
                            ),
                            Case::SafeFailureAfterCancellation => matches!(
                                completed.observation().1.state(),
                                OperationObservationStateV1::Failed { .. }
                            ),
                            _ => unreachable!(),
                        });
                    }
                    completed.custody().candidate().clone()
                };
                if case == Case::TerminalCheckpointSigningRetry {
                    assert_eq!(checkpoint_signs.get(), 2);
                    assert_eq!(anchor.advance_count(), terminal_anchor_advances + 1);
                    assert!(candidate
                        .checkpoint()
                        .successor()
                        .is_exact_successor_of(dispatch_candidate.checkpoint().successor()));
                }
                if case == Case::PreReleaseResultTime {
                    assert!(candidate
                        .record(OperationCustodyRecordKind::CapabilityResult)
                        .is_none());
                    assert!(candidate
                        .record(OperationCustodyRecordKind::Diagnostic)
                        .is_some());
                } else {
                    assert!(candidate
                        .record(OperationCustodyRecordKind::CapabilityResult)
                        .is_some());
                }
                if matches!(case, Case::SafeFailure | Case::SafeFailureAfterCancellation) {
                    assert!(candidate
                        .record(OperationCustodyRecordKind::AdapterFailure)
                        .is_some());
                    assert!(candidate
                        .record(OperationCustodyRecordKind::Diagnostic)
                        .is_some());
                } else if case != Case::PreReleaseResultTime {
                    assert!(candidate
                        .record(OperationCustodyRecordKind::Diagnostic)
                        .is_none());
                }
                drop(invocation_custody);
                drop(reopened_custody);
                drop(reopened);
                let mut recovered = connection(&file);
                let mut recovered_custody = SqliteOperationCustody {
                    connection: &mut recovered,
                    coordinate: coordinate.clone(),
                };
                assert_eq!(
                    recovered_custody.observe_exact(&candidate).unwrap(),
                    OperationCustodyReadBackOutcome::ExactSuccessor
                );
                if case == Case::TerminalCheckpointSigningRetry {
                    let head = candidate.checkpoint().successor();
                    assert_eq!(
                        recovered_custody
                            .read_checkpoint_head(&coordinate, head.checkpoint_domain_id(), 65_536,)
                            .unwrap(),
                        Some((
                            head.clone(),
                            candidate.checkpoint().successor_record().clone()
                        ))
                    );
                    let expected_anchor =
                        AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
                            coordinate.store_id().clone(),
                            coordinate.incarnation_id().clone(),
                            head.checkpoint_domain_id().clone(),
                            head.domain_epoch(),
                            head.domain_sequence(),
                            head.coordinate().clone(),
                        ));
                    assert_eq!(
                        anchor.read(&expected_anchor.key()).unwrap(),
                        Some(expected_anchor)
                    );
                }
                let sources = recovered_custody
                    .read_publication_sources(&coordinate, None, 16, 1 << 20)
                    .unwrap();
                assert!(sources.sources.iter().any(|source| {
                    source.observation().coordinate()
                        == candidate
                            .record(OperationCustodyRecordKind::OperationObservation)
                            .unwrap()
                            .coordinate()
                }));
            },
        )
    });
}

#[test]
fn provider_terminal_checkpoint_signing_failure_retries_without_provider_replay() {
    run_case(Case::TerminalCheckpointSigningRetry);
}

#[test]
fn provider_invoke_uses_real_reopened_custody_and_preserves_one_shot_release() {
    for case in [
        Case::Success,
        Case::ContextBoundSuccess,
        Case::SafeFailure,
        Case::RecordingRetry,
        Case::PreparationTransactionDrift,
        Case::CommitTransactionDrift,
        Case::CancellationAfterDispatch,
        Case::SafeFailureAfterCancellation,
        Case::ColdRestart,
        Case::ColdRestartAfterCancellation,
        Case::ColdRestartAfterCancellationObservationSubstitution,
        Case::ColdRestartAfterCancellationRequestSubstitution,
        Case::SubstitutedReference,
        Case::StaleAuthority,
        Case::PreReleaseResultTime,
    ] {
        run_case(case);
    }
}
