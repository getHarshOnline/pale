//! Behavioral laws for ambient session and child-operation admission.

use super::*;
use crate::principal_trust_test_support as principal_trust_bootstrap;
use pale_core::records::*;

use crate::session_admission_fixture::*;

impl Fixture {
    fn with_subject(
        binding_validity: (u64, u64),
        runtime_validity: (u64, u64),
        grant_validity: (u64, u64),
        subject_key: &str,
        public_key_character: &str,
        change_routes: impl FnOnce(&mut Vec<SessionRouteConstraintV1>),
    ) -> (
        Self,
        PrincipalTrustCheckpointPreimageV1,
        DeploymentAcceptanceV1,
    ) {
        Self::with_subject_contract_set(
            binding_validity,
            runtime_validity,
            grant_validity,
            subject_key,
            public_key_character,
            change_routes,
            SessionContractSetConstraintV1 {
                contract_set_ref: record_ref("contract-set-1"),
                contract_set_sha256: ContractSetDigest::parse(sha('c'))
                    .expect("contract set digest"),
            },
        )
    }

    fn with_mutation_inputs() -> (
        Self,
        PrincipalTrustCheckpointPreimageV1,
        DeploymentAcceptanceV1,
    ) {
        Self::with_subject((30, 180), (40, 150), (50, 220), "caller-key-1", "Q", |_| {})
    }

    fn new() -> Self {
        Self::with_validity((30, 180), (40, 150), (50, 220))
    }

    fn with_validity(
        binding_validity: (u64, u64),
        runtime_validity: (u64, u64),
        grant_validity: (u64, u64),
    ) -> Self {
        Self::with_grant_routes(binding_validity, runtime_validity, grant_validity, |_| {})
    }

    fn with_grant_routes(
        binding_validity: (u64, u64),
        runtime_validity: (u64, u64),
        grant_validity: (u64, u64),
        change_routes: impl FnOnce(&mut Vec<SessionRouteConstraintV1>),
    ) -> Self {
        Self::with_subject(
            binding_validity,
            runtime_validity,
            grant_validity,
            "caller-key-1",
            "Q",
            change_routes,
        )
        .0
    }
}

struct RejectingAuthority(AmbientSessionAuthorityError);

impl RejectingAuthority {
    fn prove(
        &self,
        frontier: AmbientSessionAuthorityError,
    ) -> Result<(), AmbientSessionAuthorityError> {
        if self.0 == frontier {
            Err(frontier)
        } else {
            Ok(())
        }
    }
}

impl AmbientSessionAuthorityPort for RejectingAuthority {
    fn verify_session_admission_lease_signature(
        &mut self,
        _: &[u8],
        _: &SessionAdmissionLeasePreimageV1,
        _: &SessionAdmissionLeaseDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }

    fn prove_current_principal_trust(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &PrincipalTrustCheckpointV1,
        _: &ResidentRecordRefV1,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.prove(AmbientSessionAuthorityError::PrincipalTrustCheckpoint)
    }

    fn prove_capability_revocation_frontier(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &CapabilityRevocationCheckpointV1,
        _: &ResidentRecordRefV1,
        _: &CapabilitySessionGrantV1,
        _: Option<&ResidentRecordRefV1>,
        _: Option<&CapabilitySessionGrantRevocationV1>,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.prove(AmbientSessionAuthorityError::CapabilityRevocationFrontier)
    }

    fn prove_deployment_revocation_frontier(
        &mut self,
        _: &DeploymentBindingV1,
        _: DeploymentAcceptanceRevocationEpochV1,
        _: Option<&ResidentRecordRefV1>,
        _: Option<&DeploymentAcceptanceRevocationV1>,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.prove(AmbientSessionAuthorityError::DeploymentRevocationFrontier)
    }

    fn prove_runtime_authority_head(
        &mut self,
        _: &ResidentRecordRefV1,
        _: &DeploymentRuntimeAuthorityV1,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.prove(AmbientSessionAuthorityError::RuntimeAuthorityHead)
    }

    fn prove_session_revocation_frontier(
        &mut self,
        _: &ResidentPrincipalIdV1,
        _: PrincipalRevocationEpochV1,
        _: Option<&ResidentRecordRefV1>,
        _: Option<&ArtifactDigest>,
        _: &PrincipalTrustCheckpointV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.prove(AmbientSessionAuthorityError::SessionRevocationFrontier)
    }

    fn prove_data_classification_ceiling(
        &mut self,
        _: &DataClassificationV1,
        _: &DataClassificationV1,
    ) -> Result<(), AmbientSessionAuthorityError> {
        self.prove(AmbientSessionAuthorityError::DataClassificationCeiling)
    }
}

fn admit(
    fixture: &Fixture,
    view: &AdmissionView<'_>,
) -> Result<AdmittedAmbientSession, AmbientSessionAdmissionError> {
    admit_ambient_session(
        fixture.session_wire_with(|_| {}),
        &view.context(),
        &mut ProvingAuthority,
    )
}

fn operation_request() -> OperationLeaseRequestV1 {
    OperationLeaseRequestV1 {
        operation_lease_id: OperationLeaseIdV1::parse("operation-lease-1")
            .expect("operation lease id"),
        operation: PackageResourceRefV1 {
            package_id: PackageIdV1::parse("operations").expect("package id"),
            resource_id: PackageResourceIdV1::parse("review").expect("operation id"),
        },
        capabilities: vec![CapabilityIdV1::parse("read").expect("capability")],
        effects: vec![EffectClass::Read],
        read_set: vec![record_ref("read-a")],
        evidence_obligations: vec![record_ref("evidence-a")],
        budget_units: LeaseBudgetUnitsV1::new(5).expect("operation budget"),
        timeout_millis: LeaseTimeoutMillisV1::new(500).expect("operation timeout"),
        expires_at: UnixTimeV1::new(90).expect("operation expiry"),
        invocation_correlation: InvocationCorrelationV1::Unobserved {
            reason: ResidentReasonV1::parse("native invocation identity unavailable")
                .expect("unobserved reason"),
        },
    }
}

fn trust_successor_with_floor(
    fixture: &Fixture,
    mut preimage: PrincipalTrustCheckpointPreimageV1,
    session_floor: u64,
    issued_at: u64,
) -> (ResidentRecordRefV1, PrincipalTrustCheckpointV1) {
    preimage.trust_checkpoint_id =
        PrincipalTrustCheckpointIdV1::parse("trust-checkpoint-successor")
            .expect("successor checkpoint id");
    preimage.checkpoint_epoch = fixture
        .trust
        .checkpoint_epoch()
        .checked_successor()
        .expect("successor checkpoint epoch");
    preimage.previous_trust_checkpoint_ref = Some(fixture.trust_ref.clone());
    preimage.previous_trust_checkpoint_sha256 =
        Some(fixture.trust.canonical_trust_checkpoint_sha256().clone());
    preimage.session_floor_epoch =
        PrincipalSessionFloorEpochV1::new(session_floor).expect("session floor");
    preimage.issued_at = UnixTimeV1::new(issued_at).expect("successor issue time");
    let digest =
        PrincipalTrustCheckpointDigest::hash_canonical(&preimage).expect("checkpoint digest");
    let admitted = PrincipalTrustCheckpointV1::admit_successor_wire(
        principal_trust_bootstrap::signed_wire(
            &preimage,
            "canonical_trust_checkpoint_sha256",
            &digest,
        ),
        &fixture.trust_ref,
        &fixture.trust,
        &preimage.schema_sha256,
        |_, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("trust successor admits");
    (record_ref("trust-checkpoint-successor"), admitted)
}

fn grant_revocation_at(
    fixture: &Fixture,
    effective_at: u64,
) -> (
    ResidentRecordRefV1,
    CapabilitySessionGrantRevocationV1,
    ResidentRecordRefV1,
    CapabilityRevocationCheckpointV1,
) {
    let signing_authority = signing_authority(
        &fixture.binding_ref,
        &fixture.binding,
        &fixture.runtime_ref,
        &fixture.runtime,
        &fixture.trust_ref,
        &fixture.trust,
    );
    let revocation_preimage = CapabilitySessionGrantRevocationPreimageV1 {
        schema_id: SchemaId::parse(CAPABILITY_SESSION_GRANT_REVOCATION_V1_SCHEMA)
            .expect("grant revocation schema"),
        schema_sha256: schema_digest('e'),
        revocation_id: CapabilityGrantRevocationIdV1::parse("grant-revocation-1")
            .expect("grant revocation id"),
        grant_ref: fixture.grant_ref.clone(),
        grant_sha256: fixture.grant.canonical_grant_sha256().clone(),
        grant_id: fixture.grant.session_grant_id().clone(),
        revocation_epoch: CapabilityGrantRevocationEpochV1::new(1).expect("grant revocation epoch"),
        previous_revocation_ref: None,
        previous_revocation_sha256: None,
        previous_revocation_epoch: None,
        effective_at: UnixTimeV1::new(effective_at).expect("revocation effective time"),
        reason_code: ResidentReasonV1::parse("operator revoked grant").expect("revocation reason"),
        issuer_principal_id: fixture.grant.issuer_principal_id().clone(),
        issuer_key_id: fixture.runtime.subject_key_id().clone(),
        issuer_key_epoch: *fixture.runtime.subject_key_epoch(),
    };
    let revocation_digest =
        CapabilitySessionGrantRevocationDigest::hash_canonical(&revocation_preimage)
            .expect("grant revocation digest");
    let revocation = CapabilitySessionGrantRevocationV1::admit_initial_wire(
        principal_trust_bootstrap::signed_wire(
            &revocation_preimage,
            "canonical_revocation_sha256",
            &revocation_digest,
        ),
        &fixture.grant_ref,
        &fixture.grant,
        &SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &revocation_preimage.schema_sha256,
            signing_authority: &signing_authority,
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("grant revocation admits");

    let checkpoint_preimage = CapabilityRevocationCheckpointPreimageV1 {
        schema_id: SchemaId::parse(CAPABILITY_REVOCATION_CHECKPOINT_V1_SCHEMA)
            .expect("capability checkpoint schema"),
        schema_sha256: fixture.checkpoint.schema_sha256().clone(),
        checkpoint_id: CapabilityRevocationCheckpointIdV1::parse("capability-checkpoint-2")
            .expect("capability checkpoint id"),
        checkpoint_epoch: fixture
            .checkpoint
            .checkpoint_epoch()
            .checked_successor()
            .expect("capability checkpoint successor"),
        previous_checkpoint_ref: Some(fixture.checkpoint_ref.clone()),
        previous_checkpoint_sha256: Some(fixture.checkpoint.canonical_checkpoint_sha256().clone()),
        authority_principal_id: fixture.grant.issuer_principal_id().clone(),
        authority_heads_root_sha256: artifact('f'),
        covered_grant_count: CoveredGrantCountV1::new(1).expect("covered grant count"),
        generated_at: UnixTimeV1::new(59).expect("checkpoint time"),
        issuer_key_id: fixture.runtime.subject_key_id().clone(),
        issuer_key_epoch: *fixture.runtime.subject_key_epoch(),
    };
    let checkpoint_digest =
        CapabilityRevocationCheckpointDigest::hash_canonical(&checkpoint_preimage)
            .expect("checkpoint digest");
    let checkpoint = CapabilityRevocationCheckpointV1::admit_successor_wire(
        principal_trust_bootstrap::signed_wire(
            &checkpoint_preimage,
            "canonical_checkpoint_sha256",
            &checkpoint_digest,
        ),
        &fixture.checkpoint_ref,
        &fixture.checkpoint,
        &SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &checkpoint_preimage.schema_sha256,
            signing_authority: &signing_authority,
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("capability checkpoint successor admits");
    (
        record_ref("grant-revocation-1"),
        revocation,
        record_ref("capability-checkpoint-2"),
        checkpoint,
    )
}

#[test]
fn session_authority_validity_includes_issue_time_and_excludes_expiry() {
    #[derive(Clone, Copy, Debug)]
    enum Authority {
        Grant,
        Deployment,
        Runtime,
    }

    for (authority, original_issue, original_expiry) in [
        (Authority::Grant, 50, 220),
        (Authority::Deployment, 30, 180),
        (Authority::Runtime, 40, 150),
    ] {
        for (issue, expiry, retained_at, valid) in [
            (59, original_expiry, 59, true),
            (60, original_expiry, 60, true),
            (61, original_expiry, 61, false),
            (original_issue, 59, 58, false),
            (original_issue, 60, 59, false),
            (original_issue, 61, 60, true),
        ] {
            // Rebuild each dependent digest through the existing core admission
            // path. Other validity windows include the supplied time of 60.
            let (fixture, expected_error) = match authority {
                Authority::Grant => (
                    Fixture::with_validity((30, 180), (40, 150), (issue, expiry)),
                    AmbientSessionAdmissionError::GrantOutsideValidity,
                ),
                Authority::Deployment => (
                    Fixture::with_validity((issue, expiry), (40, 150), (50, 220)),
                    AmbientSessionAdmissionError::DeploymentOutsideValidity,
                ),
                Authority::Runtime => (
                    Fixture::with_validity((30, 180), (issue, expiry), (50, 220)),
                    AmbientSessionAdmissionError::RuntimeAuthorityInactive,
                ),
            };
            let mut initial = fixture.view();
            initial.now = UnixTimeV1::new(retained_at).expect("valid initial admission time");
            let retained = admit(&fixture, &initial).expect("initial session admits");
            let resolved = initial.context();
            retained
                .revalidate(&resolved, &mut ProvingAuthority)
                .expect("full revalidation succeeds before the clock changes");
            let current = fixture.view();
            let mut request = operation_request();
            request.expires_at = UnixTimeV1::new(61).expect("bounded child expiry");
            for (entry, result) in [
                ("ambient", admit(&fixture, &current).map(|_| ())),
                (
                    "revalidation",
                    retained.revalidate(&current.context(), &mut ProvingAuthority),
                ),
                (
                    "final time",
                    retained.revalidate_time(&resolved.at_time(current.now)),
                ),
                (
                    "operation",
                    retained
                        .admit_operation(
                            schema_digest('f'),
                            request,
                            &current.context(),
                            &mut ProvingAuthority,
                        )
                        .map(|_| ()),
                ),
            ] {
                if valid {
                    assert!(
                        result.is_ok(),
                        "{authority:?} {issue}..{expiry}, {entry}: {result:?}"
                    );
                } else {
                    let error = result.expect_err("time outside selected authority interval");
                    assert_eq!(
                        std::mem::discriminant(&error),
                        std::mem::discriminant(&expected_error),
                        "{authority:?} {issue}..{expiry}, {entry}: {error:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn scheduled_grant_revocation_blocks_at_its_effective_time() {
    let fixture = Fixture::new();
    let (revocation_ref, revocation, checkpoint_ref, checkpoint) =
        grant_revocation_at(&fixture, 60);
    // The checkpoint is already issued at 59, before revocation is effective.
    // Its separate lower-bound check must not mask this revocation boundary.
    let mut before = fixture.view();
    before.now = UnixTimeV1::new(59).expect("time before revocation");
    before.capability_revocations = Some(CapabilityRevocationFrontier {
        checkpoint_ref: &checkpoint_ref,
        checkpoint: &checkpoint,
        grant_head: Some(CapabilityGrantRevocationHead {
            revocation_ref: &revocation_ref,
            revocation: &revocation,
        }),
    });
    let retained = admit(&fixture, &before).expect("scheduled revocation is not effective yet");
    let resolved = before.context();
    retained
        .revalidate(&resolved, &mut ProvingAuthority)
        .expect("full revalidation precedes the scheduled grant revocation");
    for now in [59, 60, 61] {
        let mut current = fixture.view();
        current.now = UnixTimeV1::new(now).expect("revocation boundary time");
        current.capability_revocations = before.capability_revocations;
        let mut request = operation_request();
        request.expires_at = UnixTimeV1::new(60).expect("child expiry at revocation boundary");
        for result in [
            admit(&fixture, &current).map(|_| ()),
            retained.revalidate(&current.context(), &mut ProvingAuthority),
            retained.revalidate_time(&resolved.at_time(current.now)),
            retained
                .admit_operation(
                    schema_digest('f'),
                    request,
                    &current.context(),
                    &mut ProvingAuthority,
                )
                .map(|_| ()),
        ] {
            if now == 59 {
                assert!(result.is_ok(), "before effective time: {result:?}");
            } else {
                assert!(
                    matches!(result, Err(AmbientSessionAdmissionError::GrantRevoked)),
                    "at or after effective time {now}: {result:?}"
                );
            }
        }
    }
}

#[test]
fn scheduled_deployment_revocation_blocks_at_its_effective_time() {
    let (fixture, _, acceptance) = Fixture::with_mutation_inputs();
    let preimage = DeploymentAcceptanceRevocationPreimageV1 {
        schema_id: SchemaId::parse(DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SCHEMA)
            .expect("deployment revocation schema"),
        schema_sha256: schema_digest('e'),
        deployment_acceptance_revocation_id: DeploymentAcceptanceRevocationIdV1::parse(
            "deployment-revocation-1",
        )
        .expect("deployment revocation id"),
        deployment_acceptance_ref: fixture.binding.deployment_acceptance_ref().clone(),
        deployment_acceptance_sha256: acceptance.canonical_acceptance_sha256().clone(),
        deployment_terms_sha256: acceptance.deployment_terms_sha256().clone(),
        revocation_epoch: DeploymentAcceptanceRevocationEpochV1::new(1)
            .expect("deployment revocation epoch"),
        previous_revocation_ref: None,
        previous_revocation_sha256: None,
        previous_revocation_epoch: None,
        effective_at: UnixTimeV1::new(60).expect("deployment revocation effective time"),
        reason_code: ResidentReasonV1::parse("owner revoked acceptance")
            .expect("deployment revocation reason"),
        issuer_principal_id: acceptance.issuer_principal_id().clone(),
        issuer_key_id: acceptance.issuer_key_id().clone(),
        issuer_key_epoch: *acceptance.issuer_key_epoch(),
    };
    let digest = DeploymentAcceptanceRevocationDigest::hash_canonical(&preimage)
        .expect("deployment revocation digest");
    let revocation = DeploymentAcceptanceRevocationV1::admit_contextual_wire(
        principal_trust_bootstrap::signed_wire(
            &preimage,
            "canonical_deployment_acceptance_revocation_sha256",
            &digest,
        ),
        &DeploymentAcceptanceRevocationAdmissionContextV1 {
            expected_schema_sha256: &preimage.schema_sha256,
            deployment_acceptance_ref: fixture.binding.deployment_acceptance_ref(),
            deployment_acceptance: &acceptance,
            current_head: None,
        },
        |_, _, _, _, _| Ok(()),
    )
    .expect("exact epoch-one deployment revocation admits");
    let revocation_ref = record_ref("deployment-revocation-1");
    let mut before = fixture.view();
    before.now = UnixTimeV1::new(59).expect("time before revocation");
    before.deployment_revocations.binding_head = Some(DeploymentAcceptanceRevocationHead {
        revocation_ref: &revocation_ref,
        revocation: &revocation,
    });
    let retained = admit(&fixture, &before).expect("scheduled revocation is not effective yet");
    let resolved = before.context();
    retained
        .revalidate(&resolved, &mut ProvingAuthority)
        .expect("full revalidation precedes the scheduled deployment revocation");
    for now in [59, 60, 61] {
        let mut current = fixture.view();
        current.now = UnixTimeV1::new(now).expect("revocation boundary time");
        current.deployment_revocations = before.deployment_revocations;
        let mut request = operation_request();
        request.expires_at = UnixTimeV1::new(60).expect("child expiry at revocation boundary");
        for result in [
            admit(&fixture, &current).map(|_| ()),
            retained.revalidate(&current.context(), &mut ProvingAuthority),
            retained.revalidate_time(&resolved.at_time(current.now)),
            retained
                .admit_operation(
                    schema_digest('f'),
                    request,
                    &current.context(),
                    &mut ProvingAuthority,
                )
                .map(|_| ()),
        ] {
            if now == 59 {
                assert!(result.is_ok(), "before effective time: {result:?}");
            } else {
                assert!(
                    matches!(result, Err(AmbientSessionAdmissionError::DeploymentRevoked)),
                    "at or after effective time {now}: {result:?}"
                );
            }
        }
    }
}

#[test]
fn ambient_session_admits_genesis_frontier_and_preserves_unobserved_invocation() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let admitted = admit(&fixture, &view).expect("ambient session admits");

    assert_eq!(admitted.grant_ref(), &fixture.grant_ref);
    assert_eq!(
        admitted.capability_checkpoint_ref(),
        &fixture.checkpoint_ref
    );
    assert_eq!(admitted.runtime_authority_ref(), &fixture.runtime_ref);
    let operation = admitted
        .admit_operation(
            schema_digest('f'),
            operation_request(),
            &view.context(),
            &mut ProvingAuthority,
        )
        .expect("narrow operation admits");
    assert!(matches!(
        operation.invocation_correlation(),
        InvocationCorrelationV1::Unobserved { .. }
    ));

    let mut observed_request = operation_request();
    observed_request.operation_lease_id =
        OperationLeaseIdV1::parse("operation-lease-2").expect("operation lease id");
    observed_request.invocation_correlation = InvocationCorrelationV1::Observed {
        invocation_correlation_id: InvocationCorrelationIdV1::parse("invocation-1")
            .expect("invocation correlation id"),
        native_invocation_id: NativeInvocationIdV1::parse("native-invocation-1")
            .expect("native invocation id"),
    };
    let observed = admitted
        .admit_operation(
            schema_digest('f'),
            observed_request,
            &view.context(),
            &mut ProvingAuthority,
        )
        .expect("observed operation admits");
    assert!(matches!(
        observed.invocation_correlation(),
        InvocationCorrelationV1::Observed {
            invocation_correlation_id,
            native_invocation_id,
        } if invocation_correlation_id.as_str() == "invocation-1"
            && native_invocation_id.as_str() == "native-invocation-1"
    ));
}

#[test]
fn ambient_session_distinguishes_absent_expired_revoked_and_stale_floor() {
    let (fixture, trust_preimage, _) = Fixture::with_mutation_inputs();

    let mut absent = fixture.view();
    absent.grant = None;
    assert!(matches!(
        admit(&fixture, &absent),
        Err(AmbientSessionAdmissionError::GrantAbsent)
    ));

    let mut expired = fixture.view();
    expired.now = UnixTimeV1::new(220).expect("grant expiry");
    assert!(matches!(
        admit(&fixture, &expired),
        Err(AmbientSessionAdmissionError::GrantOutsideValidity)
    ));

    let (revocation_ref, revocation, checkpoint_ref, checkpoint) =
        grant_revocation_at(&fixture, 59);
    let mut revoked = fixture.view();
    revoked.capability_revocations = Some(CapabilityRevocationFrontier {
        checkpoint_ref: &checkpoint_ref,
        checkpoint: &checkpoint,
        grant_head: Some(CapabilityGrantRevocationHead {
            revocation_ref: &revocation_ref,
            revocation: &revocation,
        }),
    });
    assert!(matches!(
        admit(&fixture, &revoked),
        Err(AmbientSessionAdmissionError::GrantRevoked)
    ));

    let (trust_ref, trust) = trust_successor_with_floor(&fixture, trust_preimage, 2, 55);
    let mut stale_floor = fixture.view();
    stale_floor.current_trust = Some(CurrentPrincipalTrustHead {
        checkpoint_ref: &trust_ref,
        checkpoint: &trust,
    });
    assert!(matches!(
        admit(&fixture, &stale_floor),
        Err(AmbientSessionAdmissionError::StaleSessionFloor)
    ));
}

#[test]
fn current_trust_checkpoint_must_be_issued_by_the_supplied_time() {
    let (fixture, trust_preimage, _) = Fixture::with_mutation_inputs();
    let initial_view = fixture.view();
    let admitted = admit(&fixture, &initial_view).expect("initial ambient session admits");
    let (trust_ref, trust) = trust_successor_with_floor(&fixture, trust_preimage, 1, 61);

    let mut before = fixture.view();
    before.now = UnixTimeV1::new(60).expect("time before current trust checkpoint");
    before.current_trust = Some(CurrentPrincipalTrustHead {
        checkpoint_ref: &trust_ref,
        checkpoint: &trust,
    });
    assert!(matches!(
        admit(&fixture, &before),
        Err(AmbientSessionAdmissionError::PrincipalTrustCheckpointNotYetIssued)
    ));
    assert!(matches!(
        admitted.revalidate(&before.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::PrincipalTrustCheckpointNotYetIssued)
    ));
    assert!(matches!(
        admitted.admit_operation(
            schema_digest('f'),
            operation_request(),
            &before.context(),
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::PrincipalTrustCheckpointNotYetIssued)
    ));

    for now in [61, 62] {
        let mut current = fixture.view();
        current.now = UnixTimeV1::new(now).expect("current trust-checkpoint time");
        current.current_trust = Some(CurrentPrincipalTrustHead {
            checkpoint_ref: &trust_ref,
            checkpoint: &trust,
        });
        admit(&fixture, &current).expect("ambient session admits at or after checkpoint issue");
        admitted
            .revalidate(&current.context(), &mut ProvingAuthority)
            .expect("revalidation accepts the current trust checkpoint after issue");
        admitted
            .admit_operation(
                schema_digest('f'),
                operation_request(),
                &current.context(),
                &mut ProvingAuthority,
            )
            .expect("operation admits at or after checkpoint issue");
    }
}

#[test]
fn ambient_session_distinguishes_subject_generation_activation_and_workspace_scope() {
    let fixture = Fixture::new();
    let other_subject = CapabilitySessionSubjectV1 {
        identity: RequestCallerIdV1::parse("caller-2").expect("caller id"),
        ..fixture.subject.clone()
    };
    let mut wrong_subject = fixture.view();
    wrong_subject.subject = &other_subject;
    assert!(matches!(
        admit(&fixture, &wrong_subject),
        Err(AmbientSessionAdmissionError::WrongSubject)
    ));

    let view = fixture.view();
    let stale_generation = fixture.session_wire_with(|preimage| {
        preimage.compiled_generation_digest =
            CompiledGenerationDigest::parse(sha('9')).expect("stale generation");
    });
    assert!(matches!(
        admit_ambient_session(stale_generation, &view.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::StaleGeneration)
    ));

    let stale_activation = fixture.session_wire_with(|preimage| {
        preimage.activation_fence_digest =
            ActivationFenceDigest::parse(sha('9')).expect("stale fence");
    });
    assert!(matches!(
        admit_ambient_session(stale_activation, &view.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::StaleActivation)
    ));

    for wire in [
        fixture.session_wire_with(|preimage| {
            preimage.project_id = LogicalProjectIdV1::parse("project-2").expect("project id");
        }),
        fixture.session_wire_with(|preimage| {
            preimage.execution_workspace_id =
                LogicalExecutionWorkspaceIdV1::parse("execution-workspace-2")
                    .expect("execution workspace id");
        }),
    ] {
        assert!(matches!(
            admit_ambient_session(wire, &view.context(), &mut ProvingAuthority),
            Err(AmbientSessionAdmissionError::WrongScope)
        ));
    }
}

#[test]
fn genesis_checkpoint_requires_the_exact_empty_frontier() {
    let fixture = Fixture::new();
    let (revocation_ref, revocation, _, _) = grant_revocation_at(&fixture, 59);
    let mut view = fixture.view();
    view.capability_revocations = Some(CapabilityRevocationFrontier {
        checkpoint_ref: &fixture.checkpoint_ref,
        checkpoint: &fixture.checkpoint,
        grant_head: Some(CapabilityGrantRevocationHead {
            revocation_ref: &revocation_ref,
            revocation: &revocation,
        }),
    });
    assert!(matches!(
        admit(&fixture, &view),
        Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::CapabilityRevocationFrontier
        ))
    ));
}

#[test]
fn ambient_session_preserves_absent_authorities_and_each_ceiling_failure() {
    let fixture = Fixture::new();

    let mut missing_trust = fixture.view();
    missing_trust.current_trust = None;
    assert!(matches!(
        admit(&fixture, &missing_trust),
        Err(AmbientSessionAdmissionError::PrincipalTrustCheckpointAbsent)
    ));

    let mut missing_checkpoint = fixture.view();
    missing_checkpoint.capability_revocations = None;
    assert!(matches!(
        admit(&fixture, &missing_checkpoint),
        Err(AmbientSessionAdmissionError::CapabilityRevocationCheckpointAbsent)
    ));

    let mut missing_runtime = fixture.view();
    missing_runtime.runtime_authority = None;
    assert!(matches!(
        admit(&fixture, &missing_runtime),
        Err(AmbientSessionAdmissionError::RuntimeAuthorityAbsent)
    ));

    let view = fixture.view();
    let capability_widening = fixture.session_wire_with(|preimage| {
        preimage.capability_ceiling = vec![CapabilityIdV1::parse("publish").expect("capability")];
    });
    assert!(matches!(
        admit_ambient_session(capability_widening, &view.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::CapabilityOutsideGrant)
    ));

    let scope_widening = fixture.session_wire_with(|preimage| {
        preimage.controller_id = ResidentControllerIdV1::parse("controller-2").expect("controller");
    });
    assert!(matches!(
        admit_ambient_session(scope_widening, &view.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::WrongScope)
    ));

    let mut route_widening = fixture.view();
    route_widening.route_use.transport_class = LocalTransportClassV1::UnixSocket;
    assert!(matches!(
        admit(&fixture, &route_widening),
        Err(AmbientSessionAdmissionError::RouteOutsideGrant)
    ));

    let unlisted_contract = SessionContractSetConstraintV1 {
        contract_set_ref: record_ref("contract-set-2"),
        contract_set_sha256: ContractSetDigest::parse(sha('e')).expect("contract set digest"),
    };
    let mut contract_widening = fixture.view();
    contract_widening.route_use.contract_set = &unlisted_contract;
    assert!(matches!(
        admit(&fixture, &contract_widening),
        Err(AmbientSessionAdmissionError::RouteOutsideGrant)
    ));
}

#[test]
fn ambient_session_distinguishes_deployment_runtime_and_session_lifetimes() {
    let fixture = Fixture::new();

    let mut deployment_expired = fixture.view();
    deployment_expired.now = UnixTimeV1::new(180).expect("binding expiry");
    assert!(matches!(
        admit(&fixture, &deployment_expired),
        Err(AmbientSessionAdmissionError::DeploymentOutsideValidity)
    ));

    let mut runtime_expired = fixture.view();
    runtime_expired.now = UnixTimeV1::new(150).expect("runtime expiry");
    assert!(matches!(
        admit(&fixture, &runtime_expired),
        Err(AmbientSessionAdmissionError::RuntimeAuthorityInactive)
    ));

    let mut session_expired = fixture.view();
    session_expired.now = UnixTimeV1::new(100).expect("session expiry");
    assert!(matches!(
        admit(&fixture, &session_expired),
        Err(AmbientSessionAdmissionError::SessionOutsideValidity)
    ));

    let mut deployment_revoked = fixture.view();
    deployment_revoked.deployment_revocations.epoch =
        DeploymentAcceptanceRevocationEpochV1::new(2).expect("new deployment revocation epoch");
    assert!(matches!(
        admit(&fixture, &deployment_revoked),
        Err(AmbientSessionAdmissionError::DeploymentRevoked)
    ));

    let wrong_binding = fixture.session_wire_with(|preimage| {
        preimage.deployment_binding_ref = record_ref("deployment-binding-2");
    });
    let view = fixture.view();
    assert!(matches!(
        admit_ambient_session(wrong_binding, &view.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::WrongDeployment)
    ));
}

#[test]
fn ambient_session_requires_the_exact_session_revocation_frontier() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let stale_frontier = fixture.session_wire_with(|preimage| {
        preimage.revocation_epoch =
            PrincipalRevocationEpochV1::new(2).expect("stale revocation epoch");
    });

    assert!(matches!(
        admit_ambient_session(stale_frontier, &view.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::SessionRevocationFrontier
        ))
    ));
}

#[test]
fn ambient_session_runs_every_external_current_authority_proof() {
    let fixture = Fixture::new();
    let view = fixture.view();
    for rejected in [
        AmbientSessionAuthorityError::PrincipalTrustCheckpoint,
        AmbientSessionAuthorityError::CapabilityRevocationFrontier,
        AmbientSessionAuthorityError::DeploymentRevocationFrontier,
        AmbientSessionAuthorityError::RuntimeAuthorityHead,
        AmbientSessionAuthorityError::SessionRevocationFrontier,
    ] {
        assert!(matches!(
            admit_ambient_session(
                fixture.session_wire_with(|_| {}),
                &view.context(),
                &mut RejectingAuthority(rejected),
            ),
            Err(AmbientSessionAdmissionError::Authority(actual)) if actual == rejected
        ));
    }
    assert!(matches!(
        admit_ambient_session(
            fixture.session_wire_with(|_| {}),
            &view.context(),
            &mut RejectingAuthority(AmbientSessionAuthorityError::DataClassificationCeiling,),
        ),
        Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::DataClassificationCeiling
        ))
    ));
}

#[test]
fn session_and_operation_revalidation_preserve_generation_and_activation_errors() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let admitted = admit(&fixture, &view).expect("ambient session admits");

    let other_generation = compiled_generation_with_instance(&fixture.profile, '7');
    assert_eq!(
        other_generation.profile_digest(),
        fixture.generation.profile_digest()
    );
    assert_eq!(
        other_generation.workspace_semantic_ir_digest(),
        fixture.generation.workspace_semantic_ir_digest()
    );
    assert_ne!(
        other_generation.instance_digest(),
        fixture.generation.instance_digest()
    );
    let (other_fence, other_active) = active_generation(&other_generation);
    let mut stale_generation = view.context();
    stale_generation.generation = SessionAdmissionLeaseContextV1 {
        active_head: &other_active,
        compiled_generation: &other_generation,
        effective_profile: &fixture.profile,
        activation_fence: &other_fence,
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &fixture.session_schema_sha256,
            signing_authority: &fixture.signing_authority,
        },
    };
    assert!(matches!(
        admitted.revalidate(&stale_generation, &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::StaleGeneration)
    ));
    assert!(matches!(
        admitted.admit_operation(
            schema_digest('f'),
            operation_request(),
            &stale_generation,
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::StaleGeneration)
    ));

    let (other_fence, other_active) =
        active_generation_with_fence(&fixture.generation, "activation-fence-2");
    let mut stale_activation = view.context();
    stale_activation.generation = SessionAdmissionLeaseContextV1 {
        active_head: &other_active,
        compiled_generation: &fixture.generation,
        effective_profile: &fixture.profile,
        activation_fence: &other_fence,
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &fixture.session_schema_sha256,
            signing_authority: &fixture.signing_authority,
        },
    };
    assert!(matches!(
        admitted.revalidate(&stale_activation, &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::StaleActivation)
    ));
    assert!(matches!(
        admitted.admit_operation(
            schema_digest('f'),
            operation_request(),
            &stale_activation,
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::StaleActivation)
    ));

    let other_profile = effective_profile_with_run_policy("A different admitted run policy.");
    let mut stale_profile = view.context();
    stale_profile.generation = SessionAdmissionLeaseContextV1 {
        active_head: &fixture.active,
        compiled_generation: &fixture.generation,
        effective_profile: &other_profile,
        activation_fence: &fixture.fence,
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &fixture.session_schema_sha256,
            signing_authority: &fixture.signing_authority,
        },
    };
    assert!(matches!(
        admitted.revalidate(&stale_profile, &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::Record(
            InvalidResidentRecordV1::ContextualAdmission {
                record: "session admission lease",
                invariant: "compiled effective profile and project/execution-workspace coordinates",
            }
        ))
    ));
    assert!(matches!(
        admitted.admit_operation(
            schema_digest('f'),
            operation_request(),
            &stale_profile,
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::Record(
            InvalidResidentRecordV1::ContextualAdmission {
                record: "session admission lease",
                invariant: "compiled effective profile and project/execution-workspace coordinates",
            }
        ))
    ));
}

#[test]
fn session_and_operation_revalidation_preserve_exact_authority_references() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let admitted = admit(&fixture, &view).expect("ambient session admits");

    let replacement_grant_ref = record_ref("grant-republished");
    let mut changed_grant = fixture.view();
    changed_grant.grant = Some(CapabilityGrantAuthority {
        grant_ref: &replacement_grant_ref,
        grant: &fixture.grant,
        issued_trust_checkpoint_ref: &fixture.trust_ref,
        issued_trust_checkpoint: &fixture.trust,
    });
    assert!(matches!(
        admitted.revalidate(&changed_grant.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
    ));
    assert!(matches!(
        admitted.admit_operation(
            schema_digest('f'),
            operation_request(),
            &changed_grant.context(),
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
    ));

    let replacement_checkpoint_ref = record_ref("capability-checkpoint-republished");
    let mut changed_checkpoint = fixture.view();
    changed_checkpoint.capability_revocations = Some(CapabilityRevocationFrontier {
        checkpoint_ref: &replacement_checkpoint_ref,
        checkpoint: &fixture.checkpoint,
        grant_head: None,
    });
    assert!(matches!(
        admitted.revalidate(&changed_checkpoint.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
    ));
    assert!(matches!(
        admitted.admit_operation(
            schema_digest('f'),
            operation_request(),
            &changed_checkpoint.context(),
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
    ));

    let replacement_runtime_ref = record_ref("runtime-authority-republished");
    let mut changed_runtime = fixture.view();
    changed_runtime.runtime_authority = Some(DeploymentRuntimeAuthorityHead {
        authority_ref: &replacement_runtime_ref,
        authority: &fixture.runtime,
    });
    assert!(matches!(
        admitted.revalidate(&changed_runtime.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
    ));
    assert!(matches!(
        admitted.admit_operation(
            schema_digest('f'),
            operation_request(),
            &changed_runtime.context(),
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
    ));
}

#[test]
fn session_revalidation_preserves_the_lease_without_requesting_a_child() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let admitted = admit(&fixture, &view).expect("ambient session admits");
    let resolved = view.context();
    admitted
        .revalidate(&resolved, &mut ProvingAuthority)
        .expect("full revalidation succeeds at the initial clock reading");
    let lease_before = serde_json::to_value(SessionAdmissionLeaseWireV1::from(admitted.lease()))
        .expect("lease wire serializes");
    let authority_before = (
        admitted.grant_ref().clone(),
        admitted.grant_sha256().clone(),
        admitted.capability_checkpoint_ref().clone(),
        admitted.capability_checkpoint_sha256().clone(),
        admitted.runtime_authority_ref().clone(),
        admitted.runtime_authority_sha256().clone(),
    );

    for now in [60, 61, 99] {
        let mut current = fixture.view();
        current.now = UnixTimeV1::new(now).expect("time inside session validity");
        // These fixtures prove the pure record join, not transport authentication.
        admitted
            .revalidate(&current.context(), &mut ProvingAuthority)
            .expect("the same session remains current");
        admitted
            .revalidate_time(&resolved.at_time(current.now))
            .expect("the same resolved session remains inside its time limits");
    }
    assert_eq!(
        serde_json::to_value(SessionAdmissionLeaseWireV1::from(admitted.lease()))
            .expect("lease wire serializes"),
        lease_before
    );
    assert_eq!(
        (
            admitted.grant_ref().clone(),
            admitted.grant_sha256().clone(),
            admitted.capability_checkpoint_ref().clone(),
            admitted.capability_checkpoint_sha256().clone(),
            admitted.runtime_authority_ref().clone(),
            admitted.runtime_authority_sha256().clone(),
        ),
        authority_before
    );

    let mut expired = fixture.view();
    expired.now = UnixTimeV1::new(100).expect("exact session expiry");
    assert!(matches!(
        admitted.revalidate(&expired.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::SessionOutsideValidity)
    ));
    assert!(matches!(
        admitted.revalidate_time(&resolved.at_time(expired.now)),
        Err(AmbientSessionAdmissionError::SessionOutsideValidity)
    ));
}

#[test]
fn session_revalidation_rejects_a_foreign_subject_route_and_missing_grant() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let admitted = admit(&fixture, &view).expect("ambient session admits");
    let foreign_subject = CapabilitySessionSubjectV1 {
        subject_key_id: ResidentKeyIdV1::parse("caller-key-2").expect("foreign key"),
        ..fixture.subject.clone()
    };
    let mut foreign = fixture.view();
    foreign.subject = &foreign_subject;
    assert!(matches!(
        admitted.revalidate(&foreign.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::WrongSubject)
    ));
    let foreign_route = SessionRouteConstraintV1 {
        route_generation: RouteGenerationV1::new(2).expect("different route generation"),
        ..fixture.route.clone()
    };
    let mut changed_route = fixture.view();
    changed_route.route_use.route = &foreign_route;
    assert!(matches!(
        admitted.revalidate(&changed_route.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::RouteOutsideGrant)
    ));
    let mut absent = fixture.view();
    absent.grant = None;
    assert!(matches!(
        admitted.revalidate(&absent.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::GrantAbsent)
    ));
}

#[test]
fn session_revalidation_cannot_switch_between_grant_permitted_routes() {
    let fixture = Fixture::with_grant_routes((30, 180), (40, 150), (50, 220), |routes| {
        let mut other = routes[0].clone();
        other.route_binding_ref = record_ref("route-binding-2");
        other.route_binding_sha256 = artifact('e');
        other.route_generation = RouteGenerationV1::new(2).expect("second route generation");
        routes.push(other);
    });
    let first_view = fixture.view();
    let first = admit(&fixture, &first_view).expect("first permitted route admits");
    let mut second_view = fixture.view();
    second_view.route_use.route = &fixture.grant.allowed_route_constraints().routes[1];
    let second = admit(&fixture, &second_view).expect("second permitted route admits afresh");
    assert_eq!(first.lease(), second.lease());
    assert_eq!(first.grant_sha256(), second.grant_sha256());
    let retained_first = RetainedAmbientSession::capture(&first);
    let retained_second = RetainedAmbientSession::capture(&second);
    assert_eq!(retained_first.lease(), retained_second.lease());
    assert_ne!(
        retained_first.basis(),
        retained_second.basis(),
        "a lease digest is not the complete selected admission basis"
    );
    let resumed = retained_first
        .revalidate(&first_view.context(), &mut ProvingAuthority)
        .expect("same-basis current revalidation returns admission");
    resumed
        .admit_operation(
            schema_digest('f'),
            operation_request(),
            &first_view.context(),
            &mut ProvingAuthority,
        )
        .expect("returned admission has a real existing consumer");
    assert!(matches!(
        retained_first.revalidate(&second_view.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
    ));
    first
        .revalidate(&first_view.context(), &mut ProvingAuthority)
        .expect("first session keeps its selected route");
    second
        .revalidate(&second_view.context(), &mut ProvingAuthority)
        .expect("second session keeps its selected route");
    assert!(matches!(
        first.revalidate(&second_view.context(), &mut ProvingAuthority),
        Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
    ));
    assert!(matches!(
        first.admit_operation(
            schema_digest('f'),
            operation_request(),
            &second_view.context(),
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::SessionAuthorityChanged)
    ));
}

#[test]
fn session_revalidation_rechecks_each_current_authority_proof_before_child_checks() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let admitted = admit(&fixture, &view).expect("ambient session admits");
    for rejected in [
        AmbientSessionAuthorityError::PrincipalTrustCheckpoint,
        AmbientSessionAuthorityError::CapabilityRevocationFrontier,
        AmbientSessionAuthorityError::DeploymentRevocationFrontier,
        AmbientSessionAuthorityError::RuntimeAuthorityHead,
        AmbientSessionAuthorityError::SessionRevocationFrontier,
        AmbientSessionAuthorityError::DataClassificationCeiling,
    ] {
        assert!(matches!(
            admitted.revalidate(&view.context(), &mut RejectingAuthority(rejected)),
            Err(AmbientSessionAdmissionError::Authority(actual)) if actual == rejected
        ));
        let mut expired_child = operation_request();
        expired_child.expires_at = view.now;
        assert!(matches!(
            admitted.admit_operation(
                schema_digest('f'),
                expired_child,
                &view.context(),
                &mut RejectingAuthority(rejected),
            ),
            Err(AmbientSessionAdmissionError::Authority(actual)) if actual == rejected
        ));
    }
}

#[test]
fn operation_lease_rejects_every_widening_axis_and_expired_child() {
    let fixture = Fixture::new();
    let view = fixture.view();
    let admitted = admit(&fixture, &view).expect("ambient session admits");
    let assert_widening = |request| {
        assert!(matches!(
            admitted.admit_operation(
                schema_digest('f'),
                request,
                &view.context(),
                &mut ProvingAuthority,
            ),
            Err(AmbientSessionAdmissionError::OperationWidening)
        ));
    };

    let mut request = operation_request();
    request
        .capabilities
        .push(CapabilityIdV1::parse("write").expect("capability"));
    assert_widening(request);
    let mut request = operation_request();
    request.effects.push(EffectClass::Write);
    assert_widening(request);
    let mut request = operation_request();
    request.read_set.push(record_ref("read-b"));
    assert_widening(request);
    let mut request = operation_request();
    request.evidence_obligations.clear();
    assert_widening(request);
    let mut request = operation_request();
    request.budget_units = LeaseBudgetUnitsV1::new(11).expect("wider budget");
    assert_widening(request);
    let mut request = operation_request();
    request.timeout_millis = LeaseTimeoutMillisV1::new(1_001).expect("wider timeout");
    assert_widening(request);
    let mut request = operation_request();
    request.expires_at = UnixTimeV1::new(101).expect("later expiry");
    assert_widening(request);

    let mut expired = operation_request();
    expired.expires_at = view.now;
    assert!(matches!(
        admitted.admit_operation(
            schema_digest('f'),
            expired,
            &view.context(),
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::OperationExpired)
    ));
}
