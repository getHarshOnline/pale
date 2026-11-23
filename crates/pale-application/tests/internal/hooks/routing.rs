//! Typed repository-hook routing admission laws.

use pale_core::records::{
    ArtifactDigest, LifecycleEvent, LogicalRepositoryIdV1, ProviderBindingIdentityV1,
    ProviderBindingRefV1, WorkspaceId,
};

use super::{RepositoryHookRoutingError, RepositoryHookRoutingV1};

fn provider() -> ProviderBindingRefV1 {
    ProviderBindingRefV1 {
        provider_binding_id: "terminal-provider".into(),
        provider_binding_identity: ProviderBindingIdentityV1 {
            provider_binding_sha256: ArtifactDigest::hash_bytes(b"terminal-provider"),
        },
    }
}

fn route(event: LifecycleEvent) -> RepositoryHookRoutingV1 {
    RepositoryHookRoutingV1::new(
        event,
        WorkspaceId::parse("ws.main").unwrap(),
        LogicalRepositoryIdV1::parse("repo.main").unwrap(),
        provider(),
    )
    .unwrap()
}

#[test]
fn canonical_arguments_round_trip_without_freezing_resident_state() {
    let expected = route(LifecycleEvent::PreToolUse);
    let arguments = expected.arguments();

    assert_eq!(
        RepositoryHookRoutingV1::parse(&arguments).unwrap(),
        expected
    );
    assert_eq!(
        arguments,
        [
            "hook",
            "pre-tool-use",
            "--workspace",
            "ws.main",
            "--repository",
            "repo.main",
            "--provider-binding",
            "terminal-provider",
            "--provider-binding-identity",
            provider()
                .provider_binding_identity
                .provider_binding_sha256
                .as_str(),
        ]
        .map(str::to_owned)
    );
}

#[test]
fn parser_rejects_aliases_and_host_event_mismatches() {
    let mut arguments = route(LifecycleEvent::PreToolUse).arguments();
    arguments[2] = "--workspace-id".to_owned();
    assert!(matches!(
        RepositoryHookRoutingV1::parse(&arguments),
        Err(RepositoryHookRoutingError::InvalidGrammar)
    ));

    assert!(RepositoryHookRoutingV1::new(
        LifecycleEvent::UserPromptExpansion,
        WorkspaceId::parse("ws.main").unwrap(),
        LogicalRepositoryIdV1::parse("repo.main").unwrap(),
        provider(),
    )
    .is_ok());
}
