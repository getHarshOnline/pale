//! Behavioral laws for retained operation-lease bodies.

use std::sync::Mutex;

use pale_core::records::{
    CapabilityIdV1, ControlIncarnationIdV1, ControlStoreIdV1, EffectClass, InvalidResidentRecordV1,
    InvocationCorrelationV1, LeaseBudgetUnitsV1, LeaseTimeoutMillisV1, OperationLeaseDigest,
    OperationLeaseIdV1, OperationLeaseRequestV1, OperationLeaseV1, PackageIdV1,
    PackageResourceIdV1, PackageResourceRefV1, ResidentReasonV1, ResidentRecordRefV1, SchemaId,
    SessionAdmissionLeaseV1, SessionAdmissionLeaseWireV1, UnixTimeV1, OPERATION_LEASE_V1_SCHEMA_ID,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlStoreError, ControlStoreErrorKind, OperationCustodyCandidate,
    OperationCustodyCommitOutcome, OperationCustodyIndexObservation, OperationCustodyIndexQuery,
    OperationCustodyPort, OperationCustodyReadBackOutcome, SessionLeaseBody, SessionLeaseBodyRead,
    SessionLeaseBodyRetention, SessionLeaseCustodyPort,
};

use super::*;
use crate::resident::AdmittedCompiledGeneration;
use crate::resident_fixture::{
    active, advertised, applied_evidence, compile_live, fence, live_workspace, published,
    resolver_for_active, session_lease_for_profile, synthetic_set_policy, ActiveResidentFixture,
    RecoveryResolver,
};

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("deployment-store-1").expect("store identity"),
        ControlIncarnationIdV1::parse("deployment-incarnation-1").expect("store incarnation"),
    )
}

fn other_store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("deployment-store-1").expect("store identity"),
        ControlIncarnationIdV1::parse("deployment-incarnation-2").expect("store incarnation"),
    )
}

fn operation_lease_ref() -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse("operation/lease-1").expect("operation lease reference")
}

struct Fixture {
    parent: SessionAdmissionLeaseV1,
    substituted_parent: SessionAdmissionLeaseV1,
    lease: OperationLeaseV1,
    resolver: RecoveryResolver,
}

fn fixture() -> Fixture {
    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    let profile = compilation.profile().clone();
    let materialization = applied_evidence(&compilation).into_materialization();
    let admitted = AdmittedCompiledGeneration::admit(
        compilation,
        vec![materialization],
        synthetic_set_policy(),
    )
    .expect("generation admits");
    let generation = admitted.record().clone();
    let published = published(&generation, 1, None);
    let advertised = advertised(&published, 2, None);
    let fence = fence(&generation, 1, None);
    let active = active(&advertised, &fence, 3, None);
    let active_fixture = ActiveResidentFixture {
        generation,
        profile,
        published,
        advertised,
        fence,
        active,
    };
    let parent = session_lease_for_profile(
        &active_fixture.generation,
        &active_fixture.active,
        &active_fixture.fence,
        &active_fixture.profile,
        "session-operation-lease",
        10,
    );
    let substituted_parent = session_lease_for_profile(
        &active_fixture.generation,
        &active_fixture.active,
        &active_fixture.fence,
        &active_fixture.profile,
        "session-operation-lease-substituted",
        10,
    );
    let schema = pale_contracts::record_schema_identity(OPERATION_LEASE_V1_SCHEMA_ID)
        .expect("operation lease schema");
    let lease = OperationLeaseV1::seal(
        schema.schema_sha256,
        &parent,
        OperationLeaseRequestV1 {
            operation_lease_id: OperationLeaseIdV1::parse("operation-lease-1")
                .expect("operation lease identity"),
            operation: PackageResourceRefV1 {
                package_id: PackageIdV1::parse("operations").expect("package identity"),
                resource_id: PackageResourceIdV1::parse("observe").expect("operation identity"),
            },
            capabilities: vec![CapabilityIdV1::parse("read").expect("capability")],
            effects: vec![EffectClass::Read],
            read_set: vec![ResidentRecordRefV1::parse("read-a").expect("read reference")],
            evidence_obligations: vec![
                ResidentRecordRefV1::parse("evidence-a").expect("evidence reference")
            ],
            budget_units: LeaseBudgetUnitsV1::new(5).expect("budget"),
            timeout_millis: LeaseTimeoutMillisV1::new(500).expect("timeout"),
            expires_at: UnixTimeV1::new(90).expect("expiry"),
            invocation_correlation: InvocationCorrelationV1::Unobserved {
                reason: ResidentReasonV1::parse("native correlation unavailable").expect("reason"),
            },
        },
    )
    .expect("operation lease admits");
    Fixture {
        parent,
        substituted_parent,
        lease,
        resolver: resolver_for_active(&active_fixture),
    }
}

fn session_body(parent: &SessionAdmissionLeaseV1) -> SessionLeaseBody {
    let canonical =
        pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(parent))
            .expect("session wire canonicalizes");
    SessionLeaseBody::new(
        store(),
        parent.session_admission_lease_digest().clone(),
        parent.schema_id().clone(),
        parent.schema_sha256().clone(),
        ControlRecordBytes::new(canonical.into_bytes()).expect("session body"),
    )
}

struct MemorySessionStore {
    body: Mutex<Option<SessionLeaseBody>>,
}

impl MemorySessionStore {
    fn exact(parent: &SessionAdmissionLeaseV1) -> Self {
        Self {
            body: Mutex::new(Some(session_body(parent))),
        }
    }
}

impl SessionLeaseCustodyPort for MemorySessionStore {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("operation recovery does not retain a parent session")
    }

    fn read(
        &self,
        _: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        Ok(self.body.lock().expect("session body").clone())
    }
}

struct MemoryOperationStore {
    expected_store: AuthorityStoreCoordinate,
    record: Option<ControlRecord>,
    retain_error: Option<ControlStoreErrorKind>,
    retain_before_error: bool,
    read_override: Option<Option<ControlRecord>>,
    retains: usize,
    reads: usize,
    maximum_read: Option<usize>,
}

impl Default for MemoryOperationStore {
    fn default() -> Self {
        Self {
            expected_store: store(),
            record: None,
            retain_error: None,
            retain_before_error: false,
            read_override: None,
            retains: 0,
            reads: 0,
            maximum_read: None,
        }
    }
}

impl pale_ports::ControlTransactionPort for MemoryOperationStore {
    fn prepare_transaction(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        panic!("this physical-custody/readback fixture does not prepare a new checkpoint")
    }
}

impl OperationCustodyPort for MemoryOperationStore {
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
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_core::records::ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<
        Option<(pale_ports::ControlCheckpointHead, pale_ports::ControlRecord)>,
        pale_ports::ControlStoreError,
    > {
        panic!("this owning phase test does not perform locator recovery")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("this owning phase test does not perform membership recovery")
    }

    fn prepare_accumulator(
        &mut self,
        _: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("unexpected accumulator preparation in custody-only fixture")
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        supplied_store: &AuthorityStoreCoordinate,
        record: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.retains += 1;
        if supplied_store != &self.expected_store {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        if let Some(kind) = self.retain_error.take() {
            if self.retain_before_error {
                self.record = Some(record.clone());
            }
            return Err(ControlStoreError::new(kind));
        }
        match &self.record {
            Some(existing) if existing == record => Ok(OperationCustodyCommitOutcome::ExactReplay),
            Some(_) => Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            )),
            None => {
                self.record = Some(record.clone());
                Ok(OperationCustodyCommitOutcome::Committed)
            }
        }
    }

    fn read_operation_record(
        &mut self,
        supplied_store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        self.reads += 1;
        self.maximum_read = Some(maximum_body_bytes);
        if supplied_store != &self.expected_store {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        if let Some(observed) = self.read_override.take() {
            return Ok(observed);
        }
        match &self.record {
            Some(record) if record.coordinate() == coordinate => Ok(Some(record.clone())),
            Some(record) if record.coordinate().record_ref() == coordinate.record_ref() => Err(
                ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired),
            ),
            _ => Ok(None),
        }
    }

    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("lease-body fixture does not commit an operation start")
    }

    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        panic!("lease-body fixture does not observe an operation start")
    }

    fn read_index(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &OperationCustodyIndexQuery,
        _maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        panic!("lease-body fixture does not read operation indexes")
    }
}

fn retained_record(fixture: &Fixture) -> ControlRecord {
    let mut operation_store = MemoryOperationStore::default();
    retain_operation_lease_body(
        &mut operation_store,
        &store(),
        &operation_lease_ref(),
        &fixture.lease,
        16 * 1024,
    )
    .expect("lease body retains");
    operation_store.record.expect("retained lease record")
}

#[test]
fn retention_closes_ambiguity_without_creating_operation_authority() {
    let fixture = fixture();
    let mut operation_store = MemoryOperationStore::default();
    retain_operation_lease_body(
        &mut operation_store,
        &store(),
        &operation_lease_ref(),
        &fixture.lease,
        16 * 1024,
    )
    .expect("new lease body retains");
    assert_eq!(operation_store.retains, 1);
    assert_eq!(operation_store.reads, 0);
    let retained = operation_store.record.clone().expect("retained record");
    assert_eq!(retained.coordinate().record_ref(), &operation_lease_ref());
    assert_eq!(
        retained.coordinate().record_sha256().as_str(),
        fixture.lease.operation_lease_digest().as_str()
    );

    retain_operation_lease_body(
        &mut operation_store,
        &store(),
        &operation_lease_ref(),
        &fixture.lease,
        16 * 1024,
    )
    .expect("exact replay is harmless");
    assert_eq!(operation_store.retains, 2);
    assert_eq!(operation_store.reads, 0);

    let mut recovered_ambiguous = MemoryOperationStore {
        retain_error: Some(ControlStoreErrorKind::OutcomeUnknown),
        retain_before_error: true,
        ..MemoryOperationStore::default()
    };
    retain_operation_lease_body(
        &mut recovered_ambiguous,
        &store(),
        &operation_lease_ref(),
        &fixture.lease,
        16 * 1024,
    )
    .expect("one exact read closes an ambiguous commit");
    assert_eq!(recovered_ambiguous.reads, 1);

    let mut excluded_ambiguous = MemoryOperationStore {
        retain_error: Some(ControlStoreErrorKind::OutcomeUnknown),
        ..MemoryOperationStore::default()
    };
    assert!(matches!(
        retain_operation_lease_body(
            &mut excluded_ambiguous,
            &store(),
            &operation_lease_ref(),
            &fixture.lease,
            16 * 1024,
        ),
        Err(OperationCustodyDatabaseFailureReason::CommitExcludedByReadBack { .. })
    ));
    assert_eq!(excluded_ambiguous.reads, 1);

    let conflicting = ControlRecord::new(
        retained.coordinate().clone(),
        retained.schema_id().clone(),
        retained.schema_sha256().clone(),
        ControlRecordBytes::new(b"{}".to_vec()).expect("conflicting body"),
    );
    let mut conflicting_ambiguous = MemoryOperationStore {
        retain_error: Some(ControlStoreErrorKind::OutcomeUnknown),
        read_override: Some(Some(conflicting)),
        ..MemoryOperationStore::default()
    };
    assert!(matches!(
        retain_operation_lease_body(
            &mut conflicting_ambiguous,
            &store(),
            &operation_lease_ref(),
            &fixture.lease,
            16 * 1024,
        ),
        Err(OperationCustodyDatabaseFailureReason::RecoveryRequired { .. })
    ));

    let before_retains = operation_store.retains;
    let body_len = retained.canonical_record_jcs().as_slice().len();
    assert!(matches!(
        retain_operation_lease_body(
            &mut operation_store,
            &store(),
            &operation_lease_ref(),
            &fixture.lease,
            body_len - 1,
        ),
        Err(OperationCustodyDatabaseFailureReason::Store(error))
            if error.kind() == ControlStoreErrorKind::RecoveryRequired
    ));
    assert_eq!(operation_store.retains, before_retains);
}

#[test]
fn historical_recovery_rejects_substitution_and_requires_the_parent_session() {
    let fixture = fixture();
    let retained = retained_record(&fixture);
    let coordinate = retained.coordinate().clone();
    let session_store = MemorySessionStore::exact(&fixture.parent);
    let body_len = retained.canonical_record_jcs().as_slice().len();
    let mut operation_store = MemoryOperationStore {
        record: Some(retained.clone()),
        ..MemoryOperationStore::default()
    };
    let recovered = recover_operation_lease_body(
        &mut operation_store,
        &session_store,
        &store(),
        &coordinate,
        body_len,
        16 * 1024,
        &fixture.resolver,
    )
    .expect("exact historical body re-admits");
    assert_eq!(recovered, fixture.lease);
    assert_eq!(operation_store.maximum_read, Some(body_len));

    let mut missing = MemoryOperationStore::default();
    assert!(matches!(
        recover_operation_lease_body(
            &mut missing,
            &session_store,
            &store(),
            &coordinate,
            body_len,
            16 * 1024,
            &fixture.resolver,
        ),
        Err(OperationLeaseBodyRecoveryError::Missing)
    ));

    let mut wrong_store = MemoryOperationStore {
        record: Some(retained.clone()),
        ..MemoryOperationStore::default()
    };
    assert!(matches!(
        recover_operation_lease_body(
            &mut wrong_store,
            &session_store,
            &other_store(),
            &coordinate,
            body_len,
            16 * 1024,
            &fixture.resolver,
        ),
        Err(OperationLeaseBodyRecoveryError::Store(error))
            if error.kind() == ControlStoreErrorKind::RecoveryRequired
    ));

    let wrong_coordinate = ControlRecord::new(
        ControlRecordCoordinate::new(
            ResidentRecordRefV1::parse("operation/lease-replaced").expect("replacement ref"),
            coordinate.record_sha256().clone(),
        ),
        retained.schema_id().clone(),
        retained.schema_sha256().clone(),
        retained.canonical_record_jcs().clone(),
    );
    assert_hostile_record(
        &fixture,
        &session_store,
        &coordinate,
        wrong_coordinate,
        body_len,
        |error| {
            matches!(
                error,
                OperationLeaseBodyRecoveryError::ContextSubstitution(_)
            )
        },
    );

    let wrong_schema = ControlRecord::new(
        coordinate.clone(),
        SchemaId::parse("pale/not-operation-lease.v1").expect("wrong schema"),
        retained.schema_sha256().clone(),
        retained.canonical_record_jcs().clone(),
    );
    assert_hostile_record(
        &fixture,
        &session_store,
        &coordinate,
        wrong_schema,
        body_len,
        |error| {
            matches!(
                error,
                OperationLeaseBodyRecoveryError::ContextSubstitution(_)
            )
        },
    );

    let noncanonical = ControlRecord::new(
        coordinate.clone(),
        retained.schema_id().clone(),
        retained.schema_sha256().clone(),
        ControlRecordBytes::new(
            [
                b"{ ".as_slice(),
                &retained.canonical_record_jcs().as_slice()[1..],
            ]
            .concat(),
        )
        .expect("noncanonical body"),
    );
    assert_hostile_record(
        &fixture,
        &session_store,
        &coordinate,
        noncanonical,
        body_len + 1,
        |error| matches!(error, OperationLeaseBodyRecoveryError::NonCanonical),
    );

    let duplicate = ControlRecord::new(
        coordinate.clone(),
        retained.schema_id().clone(),
        retained.schema_sha256().clone(),
        ControlRecordBytes::new(br#"{"schema":"a","schema":"b"}"#.to_vec())
            .expect("duplicate-key body"),
    );
    assert_hostile_record(
        &fixture,
        &session_store,
        &coordinate,
        duplicate,
        body_len,
        |error| matches!(error, OperationLeaseBodyRecoveryError::Canonical(_)),
    );

    let mut changed_digest: serde_json::Value =
        pale_core::canon::parse_strict(retained.canonical_record_jcs().as_slice())
            .expect("retained operation lease parses");
    changed_digest["operation_lease_digest"] = serde_json::json!(digest('0'));
    let changed_digest = pale_core::canon::to_canonical_string(&changed_digest)
        .expect("changed lease remains canonical");
    let self_mismatch = ControlRecord::new(
        coordinate.clone(),
        retained.schema_id().clone(),
        retained.schema_sha256().clone(),
        ControlRecordBytes::new(changed_digest.into_bytes()).expect("self-mismatched body"),
    );
    assert_hostile_record(
        &fixture,
        &session_store,
        &coordinate,
        self_mismatch,
        body_len,
        |error| matches!(error, OperationLeaseBodyRecoveryError::Admission(_)),
    );

    let mut oversized = MemoryOperationStore {
        read_override: Some(Some(retained.clone())),
        ..MemoryOperationStore::default()
    };
    assert!(matches!(
        recover_operation_lease_body(
            &mut oversized,
            &session_store,
            &store(),
            &coordinate,
            body_len - 1,
            16 * 1024,
            &fixture.resolver,
        ),
        Err(OperationLeaseBodyRecoveryError::BoundExceeded)
    ));

    let no_historical_context = RecoveryResolver::default();
    let mut operation_store = MemoryOperationStore {
        record: Some(retained.clone()),
        ..MemoryOperationStore::default()
    };
    assert!(matches!(
        recover_operation_lease_body(
            &mut operation_store,
            &session_store,
            &store(),
            &coordinate,
            body_len,
            16 * 1024,
            &no_historical_context,
        ),
        Err(OperationLeaseBodyRecoveryError::ParentSession(
            GenerationLifecycleError::MissingRecoveryContext(_)
        ))
    ));

    let missing_parent = MemorySessionStore {
        body: Mutex::new(None),
    };
    let mut operation_store = MemoryOperationStore {
        record: Some(retained.clone()),
        ..MemoryOperationStore::default()
    };
    assert!(matches!(
        recover_operation_lease_body(
            &mut operation_store,
            &missing_parent,
            &store(),
            &coordinate,
            body_len,
            16 * 1024,
            &fixture.resolver,
        ),
        Err(OperationLeaseBodyRecoveryError::ParentSession(
            GenerationLifecycleError::MissingRecoveryContext("retained session lease body")
        ))
    ));

    let substituted_parent = MemorySessionStore {
        body: Mutex::new(Some(session_body(&fixture.substituted_parent))),
    };
    let mut operation_store = MemoryOperationStore {
        record: Some(retained.clone()),
        ..MemoryOperationStore::default()
    };
    assert!(matches!(
        recover_operation_lease_body(
            &mut operation_store,
            &substituted_parent,
            &store(),
            &coordinate,
            body_len,
            16 * 1024,
            &fixture.resolver,
        ),
        Err(OperationLeaseBodyRecoveryError::ParentSession(
            GenerationLifecycleError::RecoveryContextSubstitution("retained session lease body")
        ))
    ));

    let mut widened: serde_json::Value =
        pale_core::canon::parse_strict(retained.canonical_record_jcs().as_slice())
            .expect("retained operation lease parses");
    {
        let object = widened
            .as_object_mut()
            .expect("operation lease is an object");
        object.remove("operation_lease_digest");
        object.insert("capabilities".into(), serde_json::json!(["read", "write"]));
    }
    let widened_digest =
        OperationLeaseDigest::hash_canonical(&widened).expect("widened preimage digest");
    widened
        .as_object_mut()
        .expect("operation lease is an object")
        .insert(
            "operation_lease_digest".into(),
            serde_json::to_value(widened_digest).expect("digest serializes"),
        );
    let widened = pale_core::canon::to_canonical_string(&widened)
        .expect("widened operation lease canonicalizes");
    let widened = ControlRecord::new(
        coordinate.clone(),
        retained.schema_id().clone(),
        retained.schema_sha256().clone(),
        ControlRecordBytes::new(widened.into_bytes()).expect("widened lease body"),
    );
    assert_hostile_record(
        &fixture,
        &session_store,
        &coordinate,
        widened,
        16 * 1024,
        |error| {
            matches!(
                error,
                OperationLeaseBodyRecoveryError::Admission(InvalidResidentRecordV1::LeaseWidening)
            )
        },
    );
}

fn assert_hostile_record(
    fixture: &Fixture,
    session_store: &MemorySessionStore,
    requested: &ControlRecordCoordinate,
    hostile: ControlRecord,
    maximum_body_bytes: usize,
    expected: impl FnOnce(&OperationLeaseBodyRecoveryError) -> bool,
) {
    let mut operation_store = MemoryOperationStore {
        read_override: Some(Some(hostile)),
        ..MemoryOperationStore::default()
    };
    let error = recover_operation_lease_body(
        &mut operation_store,
        session_store,
        &store(),
        requested,
        maximum_body_bytes,
        16 * 1024,
        &fixture.resolver,
    )
    .expect_err("hostile retained body must fail closed");
    assert!(expected(&error), "unexpected recovery error: {error}");
}
