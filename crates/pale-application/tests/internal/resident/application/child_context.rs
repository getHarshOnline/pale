//! Native child events use the recovered session's sealed policy and payload.

use super::*;
use crate::hooks::HookOutcome;

#[test]
fn recovered_child_start_withholds_context_until_the_sealed_selector_matches() -> TestResult {
    let port = MemoryAuthorityStore::default();
    let mut resident = empty_application(&port, RingCursorV1::new(0));
    let (compilation, materializations, profile) = local::source_generation_inputs_with(
        |_| {},
        |private| {
            private.run_policies[0].session_inject = true;
            private.run_policies[0].child_type_selectors = vec!["research-*".into()];
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
    let (session, _) = resident.admit_session(
        wire_for_activity(&fixture, activity),
        &context,
        &mut ProvingAuthority,
    )?;
    let resolver = resolver_for_active(&active);
    let repository = resident
        .generation(session.lease().compiled_generation_digest())
        .unwrap()
        .repository(session.lease().repository_id())
        .unwrap();
    let support = repository
        .render()
        .support
        .iter()
        .find(|support| {
            &support.execution_path_witness.selection == session.lease().execution_path()
        })
        .unwrap();
    let provider = support
        .execution_path_witness
        .terminal_provider_binding
        .clone()
        .unwrap();
    let injection = repository
        .render()
        .surfaces
        .iter()
        .find(|surface| surface.rel_path == harness_injection_path(&provider))
        .unwrap()
        .content()
        .to_owned();
    assert!(!injection.is_empty());

    let caller_binding = ResidentHookCallerBinding::new(HOOK_CALLER_KEY, &session)?;
    let caller = HookCaller(Some(HOOK_CALLER_KEY));
    for (child, admitted) in [
        (None, false),
        (Some(" "), false),
        (Some("reviewer"), false),
        (Some("research-reader"), true),
    ] {
        let mut payload = serde_json::json!({
            "hook_event_name": "SubagentStart", "session_id": "native-session"
        });
        if let Some(child) = child {
            payload["agent_type"] = serde_json::json!(child);
        }
        let request = ResidentHookIngressRequest::admit(
            RepositoryHookRoutingV1::new(
                LifecycleEvent::SubagentStart,
                fixture.profile.workspace_id.clone(),
                session.lease().repository_id().clone(),
                provider.clone(),
            )?,
            &serde_json::to_vec(&payload)?,
        )?;
        let recovered = caller_binding
            .admit_request(&caller, &request)?
            .admit_native_session(&resident, &resolver, &context, &mut ProvingAuthority)?;
        let HookOutcome::Context(output) = recovered.evaluate_child_context(&resident)? else {
            panic!("child propagation produces context, not tool authorization")
        };
        if admitted {
            assert_eq!(output, injection);
        } else {
            assert!(!output.contains(&injection));
            if child.is_none() || child == Some(" ") {
                assert!(output.contains("omitted the agent identity"));
            } else {
                assert!(output.is_empty());
            }
        }
        assert_eq!(
            recovered.admit_write_policy(&resident).unwrap_err(),
            ResidentHookPolicyAdmissionError::WrongEvent
        );
    }
    Ok(())
}
