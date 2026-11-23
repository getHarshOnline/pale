//! Native correlation uses a real admitted session and custody-issued operation.
//! Authority, signatures and physical stores remain fixture models.

use super::*;
use crate::resident::operation_observation_custody::tests::local_start as custody;
use crate::resident::{
    AuthenticatedResidentHookRequest, IssuedLocalOperation, ResidentHookOperationRegistry,
    ResidentHookOperationRegistryError as Error,
};
use std::{cell::Cell, num::NonZeroUsize};

#[derive(Clone, Copy)]
enum NativeMapping {
    AmbiguousRead,
    Read,
    OtherCapability,
    Write { effect: EffectClass },
}

fn event(tool_id: &str, input: &str) -> String {
    serde_json::json!({
        "hook_event_name": "PreToolUse", "session_id": "native-session",
        "turn_id": "turn-1", "tool_use_id": tool_id, "tool_name": "Read",
        "tool_input": { "file_path": input }
    })
    .to_string()
}

fn with_operation(
    tool_id: &str,
    run: impl FnOnce(
        &ResidentApplication<'_>,
        &RecoveryResolver,
        &Fixture,
        &AmbientSessionAdmissionContext<'_>,
        &AuthenticatedResidentHookRequest,
        &crate::resident::AdmittedResidentHookSession<'_>,
        &IssuedLocalOperation,
        &RepositoryHookRoutingV1,
    ) -> TestResult,
) -> TestResult {
    with_operations(
        &[tool_id],
        |app, history, fixture, context, request, admitted, operations, route| {
            run(
                app,
                history,
                fixture,
                context,
                request,
                admitted,
                &operations[0],
                route,
            )
        },
    )
}

fn with_operations(
    tool_ids: &[&str],
    run: impl FnOnce(
        &ResidentApplication<'_>,
        &RecoveryResolver,
        &Fixture,
        &AmbientSessionAdmissionContext<'_>,
        &AuthenticatedResidentHookRequest,
        &crate::resident::AdmittedResidentHookSession<'_>,
        &[IssuedLocalOperation],
        &RepositoryHookRoutingV1,
    ) -> TestResult,
) -> TestResult {
    with_tool_mapping(NativeMapping::AmbiguousRead, tool_ids, run)
}

fn with_tool_mapping(
    mapping: NativeMapping,
    tool_ids: &[&str],
    run: impl FnOnce(
        &ResidentApplication<'_>,
        &RecoveryResolver,
        &Fixture,
        &AmbientSessionAdmissionContext<'_>,
        &AuthenticatedResidentHookRequest,
        &crate::resident::AdmittedResidentHookSession<'_>,
        &[IssuedLocalOperation],
        &RepositoryHookRoutingV1,
    ) -> TestResult,
) -> TestResult {
    with_tool_mapping_and_requests(
        mapping,
        tool_ids,
        |app, history, fixture, context, request, admitted, operations, route, _requests| {
            run(
                app, history, fixture, context, request, admitted, operations, route,
            )
        },
    )
}

fn with_tool_mapping_and_requests(
    mapping: NativeMapping,
    tool_ids: &[&str],
    run: impl FnOnce(
        &ResidentApplication<'_>,
        &RecoveryResolver,
        &Fixture,
        &AmbientSessionAdmissionContext<'_>,
        &AuthenticatedResidentHookRequest,
        &crate::resident::AdmittedResidentHookSession<'_>,
        &[IssuedLocalOperation],
        &RepositoryHookRoutingV1,
        &[CommonRequestEnvelopeV1],
    ) -> TestResult,
) -> TestResult {
    with_response_custody(
        mapping,
        tool_ids,
        |app, history, fixture, context, request, admitted, operations, route, requests, _| {
            run(
                app, history, fixture, context, request, admitted, operations, route, requests,
            )
        },
    )
}

fn with_response_custody(
    mapping: NativeMapping,
    tool_ids: &[&str],
    run: impl FnOnce(
        &ResidentApplication<'_>,
        &RecoveryResolver,
        &Fixture,
        &AmbientSessionAdmissionContext<'_>,
        &AuthenticatedResidentHookRequest,
        &crate::resident::AdmittedResidentHookSession<'_>,
        &[IssuedLocalOperation],
        &RepositoryHookRoutingV1,
        &[CommonRequestEnvelopeV1],
        &[(
            pale_ports::OperationCustodyCandidate,
            ControlStateCheckpointV1,
        )],
    ) -> TestResult,
) -> TestResult {
    let unique = !matches!(mapping, NativeMapping::AmbiguousRead);
    let sibling = if matches!(mapping, NativeMapping::OtherCapability) {
        "write"
    } else {
        "sibling"
    };
    let is_write = matches!(mapping, NativeMapping::Write { .. });
    let effect = match mapping {
        NativeMapping::Write { effect } => effect,
        _ => EffectClass::Compute,
    };
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let source =
        crate::composition_source_fixtures::workspace_composition_source(&["direct-claude-code"]);
    let provider = &source.declarations.provider_bindings[0];
    let provider_ref = ProviderBindingRefV1 {
        provider_binding_id: provider.id.clone(),
        provider_binding_identity: provider.binding_identity.clone(),
    };
    let (compilation, materializations, profile) = local::source_generation_inputs_with(
        |operation| {
            if unique {
                operation.capability_requirements[1].capability =
                    CapabilityIdV1::parse(sibling).unwrap();
            }
            operation.capability_requirements[0].effect_class = effect;
            if is_write {
                operation.capability_requirements[1].effect_class = EffectClass::Write;
            }
        },
        |private| {
            let read = CapabilityIdV1::parse("read").unwrap();
            if unique {
                private.capability_bindings.insert(
                    CapabilityIdV1::parse(sibling).unwrap(),
                    private.capability_bindings[&read].clone(),
                );
            }
            if is_write {
                let sibling = private
                    .capability_bindings
                    .get_mut(&CapabilityIdV1::parse(sibling).unwrap())
                    .unwrap();
                sibling.effect_class = EffectClass::Write;
                sibling.implementation = CapabilityProviderBinding::HostNative {
                    provider_tools: vec![ProviderToolBindingV1 {
                        provider_binding: provider_ref.clone(),
                        tools: vec![HostToolBinding {
                            name: "OtherWrite".into(),
                            model_name: None,
                            observability: ToolObservability::PreToolObservable,
                            commands: vec![],
                            command_source: None,
                            input_equals: Default::default(),
                        }],
                    }],
                };
            }
            private
                .capability_bindings
                .get_mut(&read)
                .unwrap()
                .effect_class = effect;
            private
                .capability_bindings
                .get_mut(&read)
                .unwrap()
                .implementation = CapabilityProviderBinding::HostNative {
                provider_tools: vec![ProviderToolBindingV1 {
                    provider_binding: provider_ref,
                    tools: vec![HostToolBinding {
                        name: if is_write { "Write" } else { "Read" }.into(),
                        model_name: Some("read_model_alias".into()),
                        observability: ToolObservability::PreToolObservable,
                        commands: vec![],
                        command_source: None,
                        input_equals: Default::default(),
                    }],
                }],
            };
            if matches!(mapping, NativeMapping::OtherCapability) {
                private.capability_bindings.insert(
                    CapabilityIdV1::parse(sibling).unwrap(),
                    private.capability_bindings[&read].clone(),
                );
                let CapabilityProviderBinding::HostNative { provider_tools } = &mut private
                    .capability_bindings
                    .get_mut(&read)
                    .unwrap()
                    .implementation
                else {
                    unreachable!()
                };
                provider_tools[0].tools[0].name = "OriginalRequestTool".into();
            }
        },
    );
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
    let activity = HarnessActivityIdentity::try_from_host(Some("native-session"), None)?.unwrap();
    let wire = fixture.session_wire_with(|value| {
        configure_wire(&fixture, value);
        if matches!(mapping, NativeMapping::OtherCapability) {
            value
                .capability_ceiling
                .push(CapabilityIdV1::parse(sibling).unwrap());
        }
        if is_write {
            value.effect_ceiling.push(EffectClass::Write);
        }
        value.native_session_correlation = NativeSessionCorrelationV1::Observed {
            native_session_identity: activity,
        };
    });
    let (session, _) = resident.admit_session(wire, &context, &mut ProvingAuthority)?;
    let resolver = resolver_for_active(&active);
    let generation = resident.generation(&digest).unwrap();
    let provider = generation
        .repository(session.lease().repository_id())
        .unwrap()
        .render()
        .support
        .iter()
        .find(|row| &row.execution_path_witness.selection == session.lease().execution_path())
        .unwrap()
        .execution_path_witness
        .terminal_provider_binding
        .clone()
        .unwrap();
    let route = RepositoryHookRoutingV1::new(
        LifecycleEvent::PreToolUse,
        fixture.profile.workspace_id.clone(),
        session.lease().repository_id().clone(),
        provider,
    )?;
    let mut payload: serde_json::Value = serde_json::from_str(&event(tool_ids[0], "first"))?;
    let write_directory = is_write.then(tempfile::tempdir).transpose()?;
    if let Some(directory) = &write_directory {
        payload["tool_name"] = "Write".into();
        payload["tool_input"]["file_path"] =
            directory.path().join("output").to_str().unwrap().into();
    }
    let captured = captured_hook(&route, &payload.to_string(), HOOK_CALLER_KEY);
    let binding = ResidentHookCallerBinding::new(HOOK_CALLER_KEY, &session)?;
    let admitted = captured.bind_to(&binding)?.admit_native_session(
        &resident,
        &resolver,
        &context,
        &mut ProvingAuthority,
    )?;
    let mut operations = Vec::new();
    let mut signed_requests = Vec::new();
    let mut custody_states = Vec::new();
    for &tool_id in tool_ids {
        let lease = session.admit_operation(
            local::schema_digest(OPERATION_LEASE_V1_SCHEMA_ID),
            OperationLeaseRequestV1 {
                operation_lease_id: OperationLeaseIdV1::parse(format!(
                    "native-operation-{tool_id}"
                ))?,
                operation: local::operation_id(),
                capabilities: if matches!(mapping, NativeMapping::OtherCapability) {
                    vec![
                        CapabilityIdV1::parse("read")?,
                        CapabilityIdV1::parse(sibling)?,
                    ]
                } else {
                    vec![CapabilityIdV1::parse("read")?]
                },
                effects: vec![effect],
                read_set: vec![],
                evidence_obligations: vec![],
                budget_units: LeaseBudgetUnitsV1::new(5)?,
                timeout_millis: LeaseTimeoutMillisV1::new(500)?,
                expires_at: UnixTimeV1::new(90)?,
                invocation_correlation: InvocationCorrelationV1::Observed {
                    invocation_correlation_id: InvocationCorrelationIdV1::parse(
                        "native-correlation",
                    )?,
                    native_invocation_id: NativeInvocationIdV1::parse(tool_id)?,
                },
            },
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
        let mut native_payload = payload.clone();
        native_payload["tool_use_id"] = tool_id.into();
        let native = crate::resident::ResidentNativeToolStart::prepare(
            route.clone(),
            native_payload.to_string().as_bytes(),
        )?;
        let issued: Result<IssuedLocalOperation, Box<dyn std::error::Error>> =
            local::with_admitted_start_session(
                generation,
                &active.profile,
                tool_id,
                |request| request.body = native.body().clone(),
                StartAdmission::AcceptedOwnerModel,
                local::RecoveryParents {
                    session: &session,
                    current: &context,
                    lease: &lease,
                    signing: &signing,
                },
                |prepared, request, _, _, _| {
                    signed_requests.push(request.clone());
                    let (physical, pending, checkpoint) = custody::issue_initial(
                        custody::prepared_initial(prepared?),
                        request,
                        |_| {},
                        &Cell::new(0),
                    )?;
                    let mut store = custody::absent_start_store(request);
                    let mut anchor = custody::initial_anchor();
                    let completed = physical
                        .commit(&mut store, &mut anchor, 1 << 20)
                        .unwrap()
                        .advance_anchor(&mut anchor)
                        .unwrap();
                    custody_states.push((completed.candidate().clone(), checkpoint));
                    let issued = pending
                        .finish(completed)
                        .unwrap_or_else(|_| panic!("issued operation"));
                    Ok(issued)
                },
            );
        operations.push(issued?);
    }
    run(
        &resident,
        &resolver,
        &fixture,
        &context,
        &captured,
        &admitted,
        &operations,
        &route,
        &signed_requests,
        &custody_states,
    )
}

#[test]
fn exact_occurrence_is_consumed_once_and_cannot_be_registered_again() -> TestResult {
    with_operation(
        "call-1",
        |application, historical, _, context, request, admitted, operation, route| {
            let mut registry = ResidentHookOperationRegistry::new(NonZeroUsize::new(1).unwrap());
            registry.register(request, admitted, operation, context, &mut ProvingAuthority)?;
            let result = registry.consume(
                request,
                application,
                historical,
                context,
                &mut ProvingAuthority,
            )?;
            assert_eq!(result.operation().handle(), operation.handle());
            assert_eq!(
                result.admit_native_tool(application).unwrap_err(),
                crate::resident::ResidentHookNativeToolBindingError::Ambiguous
            );
            assert!(matches!(
                result.admit_write_policy(application),
                Err(
                    crate::resident::ResidentHookOperationWritePolicyError::NativeTool(
                        crate::resident::ResidentHookNativeToolBindingError::Ambiguous
                    )
                )
            ));
            assert_eq!(
                result.session().session().lease(),
                admitted.session().lease()
            );
            assert!(!format!("{result:?}").contains("call-1"));
            assert_eq!(
                registry
                    .consume(
                        request,
                        application,
                        historical,
                        context,
                        &mut ProvingAuthority
                    )
                    .unwrap_err(),
                Error::Consumed
            );
            assert_eq!(
                registry.register(request, admitted, operation, context, &mut ProvingAuthority),
                Err(Error::Duplicate)
            );
            // Another captured frame cannot be paired with this admission.
            let substitute = captured_hook(route, &event("call-1", "first"), [9; 32]);
            assert_eq!(
                registry.register(
                    &substitute,
                    admitted,
                    operation,
                    context,
                    &mut ProvingAuthority
                ),
                Err(Error::RequestMismatch)
            );
            Ok(())
        },
    )
}

#[test]
fn substitutions_do_not_consume_the_original_occurrence() -> TestResult {
    with_operation(
        "call-1",
        |application, historical, _, context, request, admitted, operation, route| {
            let mut registry = ResidentHookOperationRegistry::new(NonZeroUsize::new(2).unwrap());
            registry.register(request, admitted, operation, context, &mut ProvingAuthority)?;
            for (payload, key, expected) in [
                (
                    event("call-1", "changed"),
                    HOOK_CALLER_KEY,
                    Error::PayloadMismatch,
                ),
                (
                    event("call-2", "first"),
                    HOOK_CALLER_KEY,
                    Error::NotRegistered,
                ),
                (
                    event("call-1", "first").replace("turn-1", "turn-2"),
                    HOOK_CALLER_KEY,
                    Error::NotRegistered,
                ),
                (event("call-1", "first"), [8; 32], Error::NotRegistered),
            ] {
                let substituted = captured_hook(route, &payload, key);
                assert_eq!(
                    registry
                        .consume(
                            &substituted,
                            application,
                            historical,
                            context,
                            &mut ProvingAuthority
                        )
                        .unwrap_err(),
                    expected
                );
            }
            let _ = registry.consume(
                request,
                application,
                historical,
                context,
                &mut ProvingAuthority,
            )?;
            Ok(())
        },
    )
}

#[test]
fn current_revocation_and_operation_expiry_are_rechecked() -> TestResult {
    with_operation(
        "call-1",
        |application, historical, fixture, context, request, admitted, operation, _| {
            let mut registry = ResidentHookOperationRegistry::new(NonZeroUsize::new(1).unwrap());
            registry.register(request, admitted, operation, context, &mut ProvingAuthority)?;
            let expired = context.at_time(UnixTimeV1::new(90)?);
            assert_eq!(
                registry
                    .consume(
                        request,
                        application,
                        historical,
                        &expired,
                        &mut ProvingAuthority
                    )
                    .unwrap_err(),
                Error::Expired
            );
            let mut changed = fixture.view();
            changed.session_revocations.epoch = PrincipalRevocationEpochV1::new(2)?;
            assert_eq!(
                registry
                    .consume(
                        request,
                        application,
                        historical,
                        &changed.context(),
                        &mut ProvingAuthority
                    )
                    .unwrap_err(),
                Error::SessionUnavailable
            );
            // Rejected attempts never take the retained operation.
            let _ = registry.consume(
                request,
                application,
                historical,
                context,
                &mut ProvingAuthority,
            )?;
            Ok(())
        },
    )
}

#[test]
fn parallel_occurrences_are_independent_and_capacity_never_evicts_them() -> TestResult {
    let mut one = ResidentHookOperationRegistry::new(NonZeroUsize::new(1).unwrap());
    let mut two = ResidentHookOperationRegistry::new(NonZeroUsize::new(2).unwrap());
    with_operations(
        &["call-1", "call-2"],
        |app, history, _, context, request, admitted, operations, route| {
            let operation = &operations[0];
            one.register(request, admitted, operation, context, &mut ProvingAuthority)?;
            two.register(request, admitted, operation, context, &mut ProvingAuthority)?;
            let second = captured_hook(route, &event("call-2", "first"), HOOK_CALLER_KEY);
            let binding = ResidentHookCallerBinding::new(HOOK_CALLER_KEY, admitted.session())?;
            let second_admitted = second.bind_to(&binding)?.admit_native_session(
                app,
                history,
                context,
                &mut ProvingAuthority,
            )?;
            {
                let admitted = &second_admitted;
                let request = &second;
                let operation = &operations[1];
                assert_eq!(
                    one.register(request, admitted, operation, context, &mut ProvingAuthority),
                    Err(Error::Full)
                );
                two.register(request, admitted, operation, context, &mut ProvingAuthority)?;
                let consumed =
                    two.consume(&second, app, history, context, &mut ProvingAuthority)?;
                assert_eq!(consumed.operation().handle(), operation.handle());
            }
            for registry in [&mut one, &mut two] {
                let consumed =
                    registry.consume(request, app, history, context, &mut ProvingAuthority)?;
                assert_eq!(consumed.operation().handle(), operation.handle());
            }
            Ok(())
        },
    )
}

#[test]
fn registration_rejects_a_different_operation_and_missing_tool_identity() -> TestResult {
    with_operation(
        "call-1",
        |app, history, _, context, request, admitted, operation, route| {
            let binding = ResidentHookCallerBinding::new(HOOK_CALLER_KEY, admitted.session())?;
            let mut registry = ResidentHookOperationRegistry::new(NonZeroUsize::new(1).unwrap());
            for (payload, expected) in [
                (event("call-2", "first"), Error::OperationMismatch),
                (event("", "first"), Error::InvalidOccurrence),
            ] {
                let request = captured_hook(route, &payload, HOOK_CALLER_KEY);
                let session = request.bind_to(&binding)?.admit_native_session(
                    app,
                    history,
                    context,
                    &mut ProvingAuthority,
                )?;
                assert_eq!(
                    registry.register(
                        &request,
                        &session,
                        operation,
                        context,
                        &mut ProvingAuthority
                    ),
                    Err(expected)
                );
            }
            registry.register(request, admitted, operation, context, &mut ProvingAuthority)?;
            Ok(())
        },
    )
}

#[test]
fn matching_native_identity_does_not_admit_an_unmapped_tool() -> TestResult {
    with_operation(
        "call-1",
        |app, history, _, context, _, admitted, operation, route| {
            let binding = ResidentHookCallerBinding::new(HOOK_CALLER_KEY, admitted.session())?;
            for tool in ["Bash", "exec_command", "unknown.native", "read_model_alias"] {
                let mut payload: serde_json::Value =
                    serde_json::from_str(&event("call-1", "first"))?;
                payload["tool_name"] = tool.into();
                let request = captured_hook(route, &payload.to_string(), HOOK_CALLER_KEY);
                let session = request.bind_to(&binding)?.admit_native_session(
                    app,
                    history,
                    context,
                    &mut ProvingAuthority,
                )?;
                let mut registry =
                    ResidentHookOperationRegistry::new(NonZeroUsize::new(1).unwrap());
                registry.register(
                    &request,
                    &session,
                    operation,
                    context,
                    &mut ProvingAuthority,
                )?;
                let correlated =
                    registry.consume(&request, app, history, context, &mut ProvingAuthority)?;
                assert_eq!(
                    correlated.admit_native_tool(app).unwrap_err(),
                    crate::resident::ResidentHookNativeToolBindingError::Unavailable
                );
                assert!(matches!(
                    correlated.admit_write_policy(app),
                    Err(
                        crate::resident::ResidentHookOperationWritePolicyError::NativeTool(
                            crate::resident::ResidentHookNativeToolBindingError::Unavailable
                        )
                    )
                ));
            }
            Ok(())
        },
    )
}

#[test]
fn one_compiled_native_capability_is_selected_without_an_effect_grant() -> TestResult {
    with_tool_mapping(
        NativeMapping::Read,
        &["call-1"],
        |app, history, _, context, request, admitted, operations, _| {
            let mut registry = ResidentHookOperationRegistry::new(NonZeroUsize::new(1).unwrap());
            registry.register(
                request,
                admitted,
                &operations[0],
                context,
                &mut ProvingAuthority,
            )?;
            let correlated =
                registry.consume(request, app, history, context, &mut ProvingAuthority)?;
            let binding = correlated.admit_native_tool(app)?;
            assert_eq!(binding.requirement_id().as_str(), "first");
            assert_eq!(binding.capability_id().as_str(), "read");
            assert_eq!(binding.effect_class(), EffectClass::Compute);
            assert_eq!(
                correlated
                    .admit_write_policy(app)?
                    .evaluate_write_boundary(&pale_adapter_fs::LocalMachineFilesystem),
                ResidentHookWritePolicyEvaluation::NoViolation,
            );
            Ok(())
        },
    )
}

#[test]
fn operation_write_policy_cannot_borrow_the_sessions_broader_effects() -> TestResult {
    for effect in [EffectClass::Compute, EffectClass::Write] {
        with_tool_mapping(
            NativeMapping::Write { effect },
            &["call-1"],
            |app, history, _, context, request, admitted, operations, _| {
                assert!(admitted
                    .session()
                    .lease()
                    .effect_ceiling()
                    .contains(&EffectClass::Write));
                assert_eq!(
                    operations[0].historical_operation_lease().effects(),
                    &[effect]
                );
                let filesystem = pale_adapter_fs::LocalMachineFilesystem;
                assert_eq!(
                    admitted
                        .admit_write_policy(app)?
                        .evaluate_write_boundary(&filesystem),
                    ResidentHookWritePolicyEvaluation::NoViolation,
                );
                let mut registry =
                    ResidentHookOperationRegistry::new(NonZeroUsize::new(1).unwrap());
                registry.register(
                    request,
                    admitted,
                    &operations[0],
                    context,
                    &mut ProvingAuthority,
                )?;
                let correlated =
                    registry.consume(request, app, history, context, &mut ProvingAuthority)?;
                let result = correlated
                    .admit_write_policy(app)?
                    .evaluate_write_boundary(&filesystem);
                match effect {
                    EffectClass::Write => {
                        assert_eq!(result, ResidentHookWritePolicyEvaluation::NoViolation)
                    }
                    _ => assert!(matches!(
                        result,
                        ResidentHookWritePolicyEvaluation::Deny { .. }
                    )),
                }
                Ok(())
            },
        )?;
    }
    Ok(())
}

#[test]
fn a_broader_operation_lease_cannot_replace_the_original_requests_capability() -> TestResult {
    with_tool_mapping(
        NativeMapping::OtherCapability,
        &["call-1"],
        |app, history, _, context, request, admitted, operations, _| {
            let operation = &operations[0];
            assert_eq!(
                operation
                    .handle()
                    .1
                    .request_binding()
                    .capability_id()
                    .as_str(),
                "read"
            );
            assert_eq!(
                operation.historical_operation_lease().capabilities().len(),
                2
            );
            let mut registry = ResidentHookOperationRegistry::new(NonZeroUsize::new(1).unwrap());
            registry.register(request, admitted, operation, context, &mut ProvingAuthority)?;
            let correlated =
                registry.consume(request, app, history, context, &mut ProvingAuthority)?;
            assert_eq!(
                correlated.admit_native_tool(app).unwrap_err(),
                crate::resident::ResidentHookNativeToolBindingError::Unavailable,
            );
            assert!(matches!(
                correlated.admit_write_policy(app),
                Err(
                    crate::resident::ResidentHookOperationWritePolicyError::NativeTool(
                        crate::resident::ResidentHookNativeToolBindingError::Unavailable
                    )
                )
            ));
            Ok(())
        },
    )
}

#[path = "hook_native_start.rs"]
mod native_start;

#[path = "hook_response.rs"]
mod response;
