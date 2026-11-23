//! Current ambient admission joined to the actual resident pin and Invoke owner.
//! Authority/signatures are fixture models; lifecycle, admission and custody are real owners.

use super::*;
use crate::hooks::RepositoryHookRoutingV1;
use crate::local_operation_start_fixture as local;
use crate::local_operation_start_fixture::StartAdmission;
use crate::resident::session_admission::SessionAdmissionBasisCustodyError;
use crate::resident::session_lease_custody::RetainedSessionRecoveryError;
use crate::resident::{
    AmbientSessionAuthorityError, LocalOperationStartError, ResidentHookCallerBinding,
    ResidentHookIngressRequest, ResidentHookPolicyAdmissionError,
    ResidentHookWritePolicyEvaluation,
};
use crate::session_admission_fixture::{self as ambient, Fixture, ProvingAuthority};
use pale_core::records::*;
use pale_ports::{
    LocalIpcCarrierError, LocalIpcFramePort, LocalIpcPeerCredentials, SessionLeaseBodyRead,
    SessionLeaseCustodyPort,
};

const HOOK_CALLER_KEY: [u8; 32] = [7; 32];

struct HookCaller(Option<[u8; 32]>);

impl LocalIpcFramePort for HookCaller {
    fn authenticated_caller_key(&self) -> Option<[u8; 32]> {
        self.0
    }
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        panic!("caller binding must not infer authority from kernel credentials")
    }
    fn send_frame(&mut self, _: &[u8], _: std::time::Instant) -> Result<(), LocalIpcCarrierError> {
        panic!("caller binding performs no carrier write")
    }
    fn receive_frame(&mut self, _: std::time::Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        panic!("caller binding performs no second carrier read")
    }
}

fn captured_hook(
    route: &RepositoryHookRoutingV1,
    event: &str,
    key: [u8; 32],
) -> crate::resident::AuthenticatedResidentHookRequest {
    struct Frame {
        bytes: Option<Box<[u8]>>,
        key: [u8; 32],
    }
    impl LocalIpcFramePort for Frame {
        fn authenticated_caller_key(&self) -> Option<[u8; 32]> {
            self.bytes.is_none().then_some(self.key)
        }
        fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
            panic!("kernel identity cannot prove the hook caller")
        }
        fn send_frame(
            &mut self,
            _: &[u8],
            _: std::time::Instant,
        ) -> Result<(), LocalIpcCarrierError> {
            panic!("frame admission emits no response")
        }
        fn receive_frame(
            &mut self,
            _: std::time::Instant,
        ) -> Result<Box<[u8]>, LocalIpcCarrierError> {
            Ok(self.bytes.take().expect("read exactly one frame"))
        }
    }
    let wire = pale_ports::ResidentHookFrameRequest::new(route.arguments(), event.into());
    let bytes = pale_core::canon::to_canonical_string(&wire)
        .unwrap()
        .into_bytes()
        .into_boxed_slice();
    crate::resident::AuthenticatedResidentHookRequest::receive(
        &mut Frame {
            bytes: Some(bytes),
            key,
        },
        std::time::Instant::now(),
    )
    .unwrap()
}

#[path = "child_context.rs"]
mod child_context;

#[path = "hook_registry.rs"]
mod hook_registry;

fn fixture(active: &ActiveResidentFixture, store_id: &str, incarnation: &str) -> Fixture {
    let contracts = ambient::local_start_contract_set();
    let (mut fixture, _, _) = Fixture::with_subject_contract_set(
        (30, 180),
        (40, 150),
        (50, 220),
        "runtime-key-1",
        "M",
        |_| {},
        SessionContractSetConstraintV1 {
            contract_set_ref: local::record_ref("contract-set-1"),
            contract_set_sha256: contracts.canonical_contract_set_sha256().clone(),
        },
    );
    fixture.profile = active.profile.clone();
    fixture.generation = active.generation.clone();
    fixture.active = active.active.clone();
    fixture.fence = active.fence.clone();
    fixture.session_schema_sha256 = local::schema_digest(SESSION_ADMISSION_LEASE_V1_SCHEMA_ID);
    fixture.runtime = ambient::runtime_authority_in_store(
        &fixture.binding_ref,
        &fixture.binding,
        (40, 150),
        &[
            CAPABILITY_RESULT_V1_SCHEMA,
            COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
            OPERATION_HANDLE_V1_SCHEMA,
            OPERATION_OBSERVATION_V1_SCHEMA,
            SESSION_ADMISSION_LEASE_V1_SCHEMA,
        ],
        store_id,
        incarnation,
    );
    fixture.signing_authority = ambient::signing_authority(
        &fixture.binding_ref,
        &fixture.binding,
        &fixture.runtime_ref,
        &fixture.runtime,
        &fixture.trust_ref,
        &fixture.trust,
    );
    fixture
}

fn configure_wire(fixture: &Fixture, value: &mut SessionAdmissionLeasePreimageV1) {
    value.repository_id = fixture.generation.repository_source_frontiers()[0]
        .repository_id
        .clone();
    value.execution_path = fixture.profile.execution_paths[0].clone();
    value.effect_ceiling = vec![EffectClass::Compute];
    value.read_ceiling.clear();
    value.evidence_obligations.clear();
}

fn wire(fixture: &Fixture) -> SessionAdmissionLeaseWireV1 {
    fixture.session_wire_with(|value| configure_wire(fixture, value))
}

fn wire_for_activity(
    fixture: &Fixture,
    activity: HarnessActivityIdentity,
) -> SessionAdmissionLeaseWireV1 {
    fixture.session_wire_with(|value| {
        configure_wire(fixture, value);
        value.native_session_correlation = NativeSessionCorrelationV1::Observed {
            native_session_identity: activity,
        };
    })
}

#[test]
fn native_activity_recovers_private_basis_and_revalidates_current_authority() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let mut generation_inputs = local::host_source_generation_input_copies("Write", 2);
    let (reconstruction, reconstruction_materializations, _) =
        generation_inputs.pop().expect("reconstruction input");
    let (compilation, materializations, profile) =
        generation_inputs.pop().expect("admission input");
    let digest =
        resident.admit_generation(compilation, materializations, synthetic_set_policy())?;
    let generation = resident.generation(&digest).unwrap().record().clone();
    let published = published(&generation, 1, None);
    let _ = resident.publish(published.clone())?;
    let advertised = advertised(&published, 2, None);
    let _ = resident.advertise(advertised.clone())?;
    let fence = fence(&generation, 1, None);
    let active_head = active(&advertised, &fence, 3, None);
    let _ = resident.activate(active_head.clone(), fence.clone())?;
    let active = ActiveResidentFixture {
        generation,
        profile,
        published,
        advertised,
        fence,
        active: active_head,
    };
    let fixture = fixture(&active, "control-store", "incarnation-1");
    let view = fixture.view();
    let context = view.context();
    let activity = HarnessActivityIdentity::try_from_host(Some("native-session"), None)?
        .expect("native session is present");
    let (session, _) = resident.admit_session(
        wire_for_activity(&fixture, activity.clone()),
        &context,
        &mut ProvingAuthority,
    )?;
    let retained = port
        .read(&SessionLeaseBodyRead::new(
            store(),
            session.lease().session_admission_lease_digest().clone(),
            1 << 20,
        ))?
        .expect("admitted session body is retained");
    assert!(retained.private_admission_basis_sha256().is_some());
    assert!(retained.canonical_private_admission_basis_jcs().is_some());

    let resolver = resolver_for_active(&active);
    let generation = resident
        .generation(session.lease().compiled_generation_digest())
        .expect("admitted generation remains retained");
    let repository = generation
        .repository(session.lease().repository_id())
        .expect("admitted repository remains retained");
    let support = repository
        .render()
        .support
        .iter()
        .find(|support| {
            &support.execution_path_witness.selection == session.lease().execution_path()
        })
        .expect("selected execution path remains retained");
    let provider = support
        .execution_path_witness
        .terminal_provider_binding
        .clone()
        .expect("fixture path has one terminal provider");
    let caller_binding = ResidentHookCallerBinding::new(HOOK_CALLER_KEY, &session)?;
    let caller = HookCaller(Some(HOOK_CALLER_KEY));
    let request = ResidentHookIngressRequest::admit(
        RepositoryHookRoutingV1::new(
            LifecycleEvent::PreToolUse,
            fixture.profile.workspace_id.clone(),
            session.lease().repository_id().clone(),
            provider.clone(),
        )?,
        br#"{"hook_event_name":"PreToolUse","session_id":"native-session","tool_name":"Read"}"#,
    )?;
    for absent_or_other in [HookCaller(None), HookCaller(Some([8; 32]))] {
        assert_eq!(
            caller_binding
                .admit_request(&absent_or_other, &request)
                .unwrap_err()
                .to_string(),
            "authenticated hook caller does not match its selected binding"
        );
    }
    let other_activity = ResidentHookIngressRequest::admit(
        request.routing().clone(),
        br#"{"hook_event_name":"PreToolUse","session_id":"another-session","tool_name":"Read"}"#,
    )?;
    assert_eq!(
        caller_binding
            .admit_request(&caller, &other_activity)
            .unwrap_err()
            .to_string(),
        "native hook activity does not match its caller binding"
    );
    let unobserved =
        crate::resident::admit_ambient_session(wire(&fixture), &context, &mut ProvingAuthority)?;
    assert!(ResidentHookCallerBinding::new(HOOK_CALLER_KEY, &unobserved).is_err());
    let another_lease = crate::resident::admit_ambient_session(
        fixture.session_wire_with(|value| {
            configure_wire(&fixture, value);
            value.session_admission_id =
                SessionAdmissionIdV1::parse("another-admitted-session").unwrap();
            value.native_session_correlation = NativeSessionCorrelationV1::Observed {
                native_session_identity: activity.clone(),
            };
        }),
        &context,
        &mut ProvingAuthority,
    )?;
    let another_binding = ResidentHookCallerBinding::new(HOOK_CALLER_KEY, &another_lease)?;
    assert_eq!(
        another_binding
            .admit_request(&caller, &request)?
            .admit_native_session(&resident, &resolver, &context, &mut ProvingAuthority)
            .unwrap_err()
            .to_string(),
        "recovered hook session does not match its caller binding"
    );
    let captured = captured_hook(
        request.routing(),
        r#"{"hook_event_name":"PreToolUse","session_id":"native-session","tool_name":"Read"}"#,
        HOOK_CALLER_KEY,
    );
    for substituted in [
        captured_hook(
            request.routing(),
            r#"{"hook_event_name":"PreToolUse","session_id":"another-session","tool_name":"Read"}"#,
            HOOK_CALLER_KEY,
        ),
        captured_hook(
            request.routing(),
            r#"{"hook_event_name":"PreToolUse","session_id":"native-session","tool_name":"Read"}"#,
            [8; 32],
        ),
    ] {
        assert!(substituted.bind_to(&caller_binding).is_err());
    }
    let recovered = captured.bind_to(&caller_binding)?.admit_native_session(
        &resident,
        &resolver,
        &context,
        &mut ProvingAuthority,
    )?;
    assert_eq!(recovered.session().lease(), session.lease());
    assert_eq!(recovered.session().grant_sha256(), session.grant_sha256());
    assert_eq!(
        recovered
            .admit_write_policy(&resident)?
            .evaluate_write_boundary(&pale_adapter_fs::LocalMachineFilesystem),
        ResidentHookWritePolicyEvaluation::NoViolation,
    );

    for (tool, allowed) in [("Read", true), ("Bash", false)] {
        let permission_request = ResidentHookIngressRequest::admit(
            RepositoryHookRoutingV1::new(
                LifecycleEvent::PermissionRequest,
                fixture.profile.workspace_id.clone(),
                session.lease().repository_id().clone(),
                provider.clone(),
            )?,
            &serde_json::to_vec(&serde_json::json!({
                "hook_event_name": "PermissionRequest",
                "session_id": "native-session",
                "tool_name": tool,
                "tool_input": {"command": "unbound-command"}
            }))?,
        )?;
        let permission_session = caller_binding
            .admit_request(&caller, &permission_request)?
            .admit_native_session(&resident, &resolver, &context, &mut ProvingAuthority)?;
        let result = permission_session
            .admit_write_policy(&resident)?
            .evaluate_write_boundary(&pale_adapter_fs::LocalMachineFilesystem);
        if allowed {
            assert_eq!(result, ResidentHookWritePolicyEvaluation::NoViolation);
        } else {
            assert!(matches!(
                result,
                ResidentHookWritePolicyEvaluation::Deny { .. }
            ));
        }
    }
    let wrong_event = ResidentHookIngressRequest::admit(
        RepositoryHookRoutingV1::new(
            LifecycleEvent::PostToolUse,
            fixture.profile.workspace_id.clone(),
            session.lease().repository_id().clone(),
            provider.clone(),
        )?,
        br#"{"hook_event_name":"PostToolUse","session_id":"native-session","tool_name":"Read"}"#,
    )?;
    let wrong_event = caller_binding
        .admit_request(&caller, &wrong_event)?
        .admit_native_session(&resident, &resolver, &context, &mut ProvingAuthority)?;
    assert_eq!(
        wrong_event.admit_write_policy(&resident).unwrap_err(),
        ResidentHookPolicyAdmissionError::WrongEvent,
    );

    let admitted_provider = support
        .execution_path_witness
        .terminal_provider_binding
        .clone()
        .expect("fixture path has one terminal provider");
    for substituted_routing in [
        RepositoryHookRoutingV1::new(
            LifecycleEvent::PreToolUse,
            WorkspaceId::parse("ws.other")?,
            session.lease().repository_id().clone(),
            admitted_provider.clone(),
        ),
        RepositoryHookRoutingV1::new(
            LifecycleEvent::PreToolUse,
            fixture.profile.workspace_id.clone(),
            LogicalRepositoryIdV1::parse("repo.other")?,
            admitted_provider.clone(),
        ),
        RepositoryHookRoutingV1::new(
            LifecycleEvent::PreToolUse,
            fixture.profile.workspace_id.clone(),
            session.lease().repository_id().clone(),
            ProviderBindingRefV1 {
                provider_binding_id: "other-provider".into(),
                provider_binding_identity: admitted_provider.provider_binding_identity.clone(),
            },
        ),
    ] {
        let substituted_request = captured_hook(
            &substituted_routing?,
            r#"{"hook_event_name":"PreToolUse","session_id":"native-session","tool_name":"Read"}"#,
            HOOK_CALLER_KEY,
        );
        assert_eq!(
            substituted_request
                .bind_to(&caller_binding)?
                .admit_native_session(&resident, &resolver, &context, &mut ProvingAuthority)
                .unwrap_err()
                .to_string(),
            "repository hook route does not match the admitted session path"
        );
    }

    let missing = HarnessActivityIdentity::try_from_host(Some("missing-session"), None)?
        .expect("missing session identity is present");
    assert!(matches!(
        resident.admit_native_activity_session(
            &missing,
            &resolver,
            &context,
            &mut ProvingAuthority,
        ),
        Err(NativeActivitySessionAdmissionError::Missing)
    ));

    let mut changed_view = fixture.view();
    changed_view.session_revocations.epoch = PrincipalRevocationEpochV1::new(2)?;
    assert!(matches!(
        resident.admit_native_activity_session(
            &activity,
            &resolver,
            &changed_view.context(),
            &mut ProvingAuthority,
        ),
        Err(NativeActivitySessionAdmissionError::Admission(
            AmbientSessionAdmissionError::Authority(
                AmbientSessionAuthorityError::SessionRevocationFrontier,
            ),
        ))
    ));

    let input = ResidentGenerationReconstructionInput::new(
        active.generation.compiled_generation_digest().clone(),
        reconstruction,
        reconstruction_materializations,
    );
    drop(resident);
    let mut source = FixedInputSource {
        requested: None,
        calls: 0,
        inputs: vec![input],
    };
    let opened = ResidentApplication::cold_open(
        &port,
        &port,
        &mut UnavailableEventCustody,
        limits().maximum_entry_bytes(),
        ResidentColdOpenInput::new(
            store(),
            limits(),
            limits().maximum_entry_bytes(),
            RecoveryReferenceKnowledge::Known(BTreeSet::new()),
            &resolver,
            &mut source,
            parameters(2),
        ),
    )?;
    let ResidentColdOpenOutcome::Ready(reopened) = opened else {
        panic!("exact retained session history must recover");
    };
    let recovered_after_restart = caller_binding
        .admit_request(&caller, &request)?
        .admit_native_session(&reopened, &resolver, &context, &mut ProvingAuthority)?;
    assert_eq!(recovered_after_restart.session().lease(), session.lease());
    assert_eq!(
        recovered_after_restart
            .admit_write_policy(&reopened)?
            .evaluate_write_boundary(&pale_adapter_fs::LocalMachineFilesystem),
        ResidentHookWritePolicyEvaluation::NoViolation,
    );
    assert_eq!(
        recovered_after_restart.session().grant_sha256(),
        session.grant_sha256()
    );
    Ok(())
}

#[test]
fn pre_tool_policy_requires_the_compiled_path_to_select_that_event() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let active = admit_and_activate(&mut resident, &workspace, &probe)?;
    let fixture = fixture(&active, "control-store", "incarnation-1");
    let view = fixture.view();
    let context = view.context();
    let activity = HarnessActivityIdentity::try_from_host(Some("native-session"), None)?
        .expect("native session is present");
    let (session, _) = resident.admit_session(
        wire_for_activity(&fixture, activity),
        &context,
        &mut ProvingAuthority,
    )?;
    let generation = resident
        .generation(session.lease().compiled_generation_digest())
        .expect("session generation remains retained");
    let repository = generation
        .repository(session.lease().repository_id())
        .expect("session repository remains retained");
    let support = repository
        .render()
        .support
        .iter()
        .find(|support| {
            &support.execution_path_witness.selection == session.lease().execution_path()
        })
        .expect("session path remains retained");
    assert!(!support
        .hook_requirements
        .events()
        .contains(&crate::hooks::HookEvent::PreToolUse));
    let provider = support
        .execution_path_witness
        .terminal_provider_binding
        .clone()
        .expect("fixture path has one terminal provider");
    let caller_binding = ResidentHookCallerBinding::new(HOOK_CALLER_KEY, &session)?;
    let caller = HookCaller(Some(HOOK_CALLER_KEY));
    let request = ResidentHookIngressRequest::admit(
        RepositoryHookRoutingV1::new(
            LifecycleEvent::PreToolUse,
            fixture.profile.workspace_id.clone(),
            session.lease().repository_id().clone(),
            provider,
        )?,
        br#"{"hook_event_name":"PreToolUse","session_id":"native-session","tool_name":"Read"}"#,
    )?;
    let resolver = resolver_for_active(&active);
    let admitted = caller_binding
        .admit_request(&caller, &request)?
        .admit_native_session(&resident, &resolver, &context, &mut ProvingAuthority)?;
    assert_eq!(
        admitted.admit_write_policy(&resident).unwrap_err(),
        ResidentHookPolicyAdmissionError::UnselectedEvent,
    );
    Ok(())
}

#[test]
fn lease_only_pin_cannot_substitute_for_recoverable_ambient_admission() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let active = admit_and_activate(&mut resident, &workspace, &probe)?;
    let fixture = fixture(&active, "control-store", "incarnation-1");
    let view = fixture.view();
    let context = view.context();
    let activity = HarnessActivityIdentity::try_from_host(Some("native-session"), None)?
        .expect("native session is present");
    let session = admit_ambient_session(
        wire_for_activity(&fixture, activity.clone()),
        &context,
        &mut ProvingAuthority,
    )?;
    let _receipt = resident.pin_session(session.lease())?;

    let resolver = resolver_for_active(&active);
    assert!(matches!(
        resident.admit_native_activity_session(
            &activity,
            &resolver,
            &context,
            &mut ProvingAuthority,
        ),
        Err(NativeActivitySessionAdmissionError::Recovery(
            RetainedSessionRecoveryError::Basis(SessionAdmissionBasisCustodyError::Missing),
        ))
    ));
    Ok(())
}

#[test]
fn admitted_retained_session_is_consumed_by_the_existing_local_invoke_start() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (compilation, materializations, profile) =
        local::source_generation_inputs_with(|_| {}, |_| {});
    let digest =
        resident.admit_generation(compilation, materializations, synthetic_set_policy())?;
    let generation = resident.generation(&digest).unwrap().record().clone();
    let published = published(&generation, 1, None);
    let _ = resident.publish(published.clone())?;
    let advertised = advertised(&published, 2, None);
    let _ = resident.advertise(advertised.clone())?;
    let fence = fence(&generation, 1, None);
    let active = active(&advertised, &fence, 3, None);
    let _ = resident.activate(active.clone(), fence.clone())?;
    let active = ActiveResidentFixture {
        generation,
        profile,
        published,
        advertised,
        fence,
        active,
    };
    let fixture = fixture(&active, "control-store", "incarnation-1");
    let view = fixture.view();
    let context = view.context();
    let before_wake = resident.wake_cursor();
    let (session, receipt) =
        resident.admit_session(wire(&fixture), &context, &mut ProvingAuthority)?;
    assert_eq!(receipt.journal_sequence().get(), 4);
    assert_eq!(resident.journal_head().map(JournalSequenceV1::get), Some(4));
    assert_ne!(resident.wake_cursor(), before_wake);
    assert_eq!(port.session_body_count(), 1);
    assert_eq!(port.append_calls(), 4);
    assert_eq!(resident.session_pins()?, resident.lifecycle().session_pins);
    assert_eq!(resident.session_pins()?.len(), 1);

    let operation = OperationLeaseRequestV1 {
        operation_lease_id: OperationLeaseIdV1::parse("start-lease")?,
        operation: local::operation_id(),
        capabilities: vec![CapabilityIdV1::parse("read")?],
        effects: vec![EffectClass::Compute],
        read_set: vec![],
        evidence_obligations: vec![],
        budget_units: LeaseBudgetUnitsV1::new(5)?,
        timeout_millis: LeaseTimeoutMillisV1::new(500)?,
        expires_at: UnixTimeV1::new(90)?,
        invocation_correlation: InvocationCorrelationV1::Unobserved {
            reason: ResidentReasonV1::parse("native invocation remains unobserved")?,
        },
    };
    let lease = session.admit_operation(
        local::schema_digest(OPERATION_LEASE_V1_SCHEMA_ID),
        operation.clone(),
        &context,
        &mut ProvingAuthority,
    )?;
    let signing = ambient::signing_authority(
        &fixture.binding_ref,
        &fixture.binding,
        &fixture.runtime_ref,
        &fixture.runtime,
        &fixture.trust_ref,
        &fixture.trust,
    );
    let registry = crate::actions::NativeActionRegistry::new().unwrap();
    local::with_admitted_start_session(
        resident.generation(&digest).unwrap(),
        &active.profile,
        "resident-admitted-session",
        local::native_request_body,
        StartAdmission::NativeInput {
            registry: &registry,
            reject_owner: false,
        },
        local::RecoveryParents {
            session: &session,
            current: &context,
            lease: &lease,
            signing: &signing,
        },
        |prepared, _, owners, signatures, parents| {
            assert_eq!(parents.session.lease(), session.lease());
            assert_eq!(prepared.unwrap().records().len(), 3);
            assert_eq!((owners, signatures), (1, 2));
        },
    );
    assert_eq!(
        port.append_calls(),
        4,
        "Invoke preparation has not committed a handle"
    );
    assert_eq!(port.session_body_count(), 1);

    let pinned = resident.lifecycle();
    let wake = resident.wake_cursor();
    let mut changed_view = fixture.view();
    changed_view.session_revocations.epoch = PrincipalRevocationEpochV1::new(2)?;
    let changed_context = changed_view.context();
    assert!(matches!(
        session.admit_operation(
            local::schema_digest(OPERATION_LEASE_V1_SCHEMA_ID),
            operation,
            &changed_context,
            &mut ProvingAuthority,
        ),
        Err(AmbientSessionAdmissionError::Authority(
            AmbientSessionAuthorityError::SessionRevocationFrontier,
        ))
    ));
    local::with_admitted_start_session(
        resident.generation(&digest).unwrap(),
        &active.profile,
        "resident-revoked-session",
        local::native_request_body,
        StartAdmission::NativeInput {
            registry: &registry,
            reject_owner: false,
        },
        local::RecoveryParents {
            session: &session,
            current: &changed_context,
            lease: &lease,
            signing: &signing,
        },
        |prepared, _, owners, signatures, _| {
            assert!(matches!(
                prepared,
                Err(LocalOperationStartError::Session(
                    AmbientSessionAdmissionError::Authority(
                        AmbientSessionAuthorityError::SessionRevocationFrontier,
                    ),
                ))
            ));
            assert_eq!((owners, signatures), (0, 0));
        },
    );
    assert_eq!(resident.lifecycle(), pinned);
    assert_eq!(resident.wake_cursor(), wake);
    assert_eq!(port.append_calls(), 4);
    assert_eq!(port.session_body_count(), 1);
    assert!(matches!(
        resident.admit_session(wire(&fixture), &context, &mut ProvingAuthority),
        Err(ResidentSessionAdmissionError::Retention(error))
            if error.class() == ResidentApplicationErrorClass::Lifecycle
    ));
    assert_eq!(resident.lifecycle(), pinned);
    assert_eq!(resident.wake_cursor(), wake);
    assert_eq!(port.append_calls(), 4);
    assert_eq!(port.session_body_count(), 1);
    Ok(())
}

#[test]
fn committed_session_admission_returns_replay_obligation_when_wake_publication_fails() -> TestResult
{
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(u64::MAX));
    let (workspace, probe) = live_workspace();
    let active = admit_and_activate(&mut resident, &workspace, &probe)?;
    let fixture = fixture(&active, "control-store", "incarnation-1");
    let view = fixture.view();
    let context = view.context();
    let wake = resident.wake_cursor();

    let (session, receipt) =
        resident.admit_session(wire(&fixture), &context, &mut ProvingAuthority)?;

    assert!(matches!(
        receipt.wake(),
        ResidentWakeDisposition::DurableReplayRequired { journal_sequence, .. }
            if journal_sequence.get() == 4
    ));
    assert_eq!(resident.journal_head().map(JournalSequenceV1::get), Some(4));
    assert_eq!(resident.wake_cursor(), wake);
    assert_eq!(resident.session_pins()?, resident.lifecycle().session_pins);
    assert!(resident
        .session_pins()?
        .contains_key(session.lease().session_admission_id()));
    assert_eq!(port.session_body_count(), 1);
    assert_eq!(port.append_calls(), 4);
    Ok(())
}

#[test]
fn session_admission_denial_and_foreign_store_leave_all_retention_state_unchanged() -> TestResult {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Fault {
        Grant,
        Subject,
        Expiry,
        Revocation,
        Store,
        Incarnation,
        ResidentHead,
    }
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (workspace, probe) = live_workspace();
    let active = admit_and_activate(&mut resident, &workspace, &probe)?;
    let before = resident.lifecycle();
    let before_wake = resident.wake_cursor();
    for fault in [
        Fault::Grant,
        Fault::Subject,
        Fault::Expiry,
        Fault::Revocation,
        Fault::Store,
        Fault::Incarnation,
        Fault::ResidentHead,
    ] {
        let mut fixture = fixture(
            &active,
            if fault == Fault::Store {
                "other-store"
            } else {
                "control-store"
            },
            if fault == Fault::Incarnation {
                "other-incarnation"
            } else {
                "incarnation-1"
            },
        );
        if fault == Fault::ResidentHead {
            (fixture.fence, fixture.active) = ambient::active_generation(&fixture.generation);
        }
        let mut wrong_subject = fixture.subject.clone();
        wrong_subject.identity = RequestCallerIdV1::parse("other-caller")?;
        let mut view = fixture.view();
        match fault {
            Fault::Grant => view.grant = None,
            Fault::Subject => view.subject = &wrong_subject,
            Fault::Expiry => view.now = UnixTimeV1::new(101)?,
            Fault::Revocation => {
                view.session_revocations.epoch = PrincipalRevocationEpochV1::new(2)?
            }
            Fault::Store | Fault::Incarnation | Fault::ResidentHead => {}
        }
        let context = view.context();
        let error = resident
            .admit_session(wire(&fixture), &context, &mut ProvingAuthority)
            .expect_err(
                "invalid authority or resident join cannot return an admitted retained session",
            );
        match (fault, error) {
            (
                Fault::Grant,
                ResidentSessionAdmissionError::Admission(AmbientSessionAdmissionError::GrantAbsent),
            )
            | (
                Fault::Subject,
                ResidentSessionAdmissionError::Admission(
                    AmbientSessionAdmissionError::WrongSubject,
                ),
            )
            | (
                Fault::Expiry,
                ResidentSessionAdmissionError::Admission(
                    AmbientSessionAdmissionError::SessionOutsideValidity,
                ),
            )
            | (
                Fault::Revocation,
                ResidentSessionAdmissionError::Admission(AmbientSessionAdmissionError::Authority(
                    AmbientSessionAuthorityError::SessionRevocationFrontier,
                )),
            )
            | (Fault::Store | Fault::Incarnation, ResidentSessionAdmissionError::StoreMismatch) => {
            }
            (Fault::ResidentHead, ResidentSessionAdmissionError::Retention(error)) => {
                assert_eq!(error.class(), ResidentApplicationErrorClass::Lifecycle);
            }
            (_, error) => panic!("{fault:?}: unexpected rejection {error:?}"),
        }
        assert_eq!(resident.lifecycle(), before, "{fault:?}");
        assert_eq!(resident.session_pins()?, before.session_pins, "{fault:?}");
        assert_eq!(resident.wake_cursor(), before_wake, "{fault:?}");
        assert_eq!(
            resident.journal_head().map(JournalSequenceV1::get),
            Some(3),
            "{fault:?}"
        );
        assert_eq!(port.append_calls(), 3, "{fault:?}");
        assert_eq!(port.session_body_count(), 0, "{fault:?}");
    }
    Ok(())
}

#[test]
fn session_admission_preserves_definitive_retention_failure_and_retry() -> TestResult {
    for body_failure in [true, false] {
        let port = MemoryAuthorityStore::default();
        let mut resident = empty_application(&port, RingCursorV1::new(0));
        let (workspace, probe) = live_workspace();
        let active = admit_and_activate(&mut resident, &workspace, &probe)?;
        let fixture = fixture(&active, "control-store", "incarnation-1");
        let view = fixture.view();
        let context = view.context();
        let before = resident.lifecycle();
        let before_wake = resident.wake_cursor();
        if body_failure {
            port.enqueue_session_body(Err(pale_ports::ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            )));
        } else {
            port.enqueue(Err(DurableAuthorityStoreErrorKind::Unavailable.into()));
        }
        let error = resident
            .admit_session(wire(&fixture), &context, &mut ProvingAuthority)
            .expect_err("failed retention cannot return the admitted session");
        assert!(
            matches!(error, ResidentSessionAdmissionError::Retention(error)
            if error.class() == ResidentApplicationErrorClass::DurabilityNoMutation)
        );
        assert_eq!(resident.lifecycle(), before);
        assert_eq!(resident.session_pins()?, before.session_pins);
        assert_eq!(resident.wake_cursor(), before_wake);
        assert_eq!(resident.journal_head().map(JournalSequenceV1::get), Some(3));
        assert_eq!(port.session_body_count(), usize::from(!body_failure));
        assert_eq!(port.append_calls(), 3 + usize::from(!body_failure));
        let (_, receipt) =
            resident.admit_session(wire(&fixture), &context, &mut ProvingAuthority)?;
        assert_eq!(receipt.journal_sequence().get(), 4);
        assert_eq!(port.session_body_count(), 1);
        assert_eq!(port.append_calls(), 4 + usize::from(!body_failure));
    }
    Ok(())
}

#[test]
fn ambiguous_admission_pin_returns_no_session_and_recovers_only_durable_retention() -> TestResult {
    for committed in [false, true] {
        let port = MemoryAuthorityStore::default();
        let mut resident = empty_application(&port, RingCursorV1::new(0));
        let (workspace, probe) = live_workspace();
        let active = admit_and_activate(&mut resident, &workspace, &probe)?;
        let fixture = fixture(&active, "control-store", "incarnation-1");
        let view = fixture.view();
        let context = view.context();
        let before = resident.lifecycle();
        let before_wake = resident.wake_cursor();
        if committed {
            port.enqueue_commit_then_ambiguous();
        } else {
            port.enqueue(Err(DurableAuthorityStoreErrorKind::AmbiguousCommit.into()));
        }
        assert!(matches!(
            resident.admit_session(wire(&fixture), &context, &mut ProvingAuthority),
            Err(ResidentSessionAdmissionError::Retention(error))
                if error.class() == ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        ));
        assert_eq!(resident.lifecycle(), before);
        assert_eq!(resident.wake_cursor(), before_wake);
        assert_eq!(port.session_body_count(), 1);
        assert_eq!(port.append_calls(), 4);
        assert!(matches!(
            resident.admit_session(wire(&fixture), &context, &mut ProvingAuthority),
            Err(ResidentSessionAdmissionError::Retention(error))
                if error.class() == ResidentApplicationErrorClass::DurabilityOutcomeUnknown
        ));
        assert_eq!(
            port.append_calls(),
            4,
            "uncertainty cannot blindly append again"
        );
        let input = reconstruction_input(&workspace, &probe, &active);
        let resolver = resolver_for_active(&active);
        drop(resident);
        let mut source = FixedInputSource {
            requested: None,
            calls: 0,
            inputs: vec![input],
        };
        let opened = ResidentApplication::cold_open(
            &port,
            &port,
            &mut UnavailableEventCustody,
            limits().maximum_entry_bytes(),
            ResidentColdOpenInput::new(
                store(),
                limits(),
                limits().maximum_entry_bytes(),
                RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                &resolver,
                &mut source,
                parameters(3),
            ),
        )?;
        let ResidentColdOpenOutcome::Ready(recovered) = opened else {
            panic!("the exact retained history must recover");
        };
        assert_eq!(recovered.session_pins()?.len(), usize::from(committed));
        assert_eq!(
            recovered.journal_head().map(JournalSequenceV1::get),
            Some(if committed { 4 } else { 3 })
        );
        assert_eq!(
            recovered.session_pins()?,
            recovered.lifecycle().session_pins
        );
        assert_eq!(
            port.append_calls(),
            4,
            "recovery does not re-admit or append a session"
        );
    }
    Ok(())
}
