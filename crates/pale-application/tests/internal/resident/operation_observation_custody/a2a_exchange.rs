//! Correlation laws through contextual record owners and public port constructors.
//!
//! Existing fixture callbacks stand in for signature and native evidence checks.

//! These tests do not establish TLS readiness, dispatch authority or durability.

use std::{
    num::NonZeroUsize,
    task::{Context, Poll},
    time::Duration,
};

use pale_ports::{
    A2aExchange, A2aExchangeCommand, A2aExchangeCoordinate, A2aExchangeError, A2aExchangeEvidence,
    A2aExchangeLimits, A2aExchangeParents, A2aExchangeRequest, A2aPreparedExchange,
    ProviderInvocationCoordinate, ProviderInvocationRequest,
};

use super::*;

#[path = "a2a_exchange/transport.rs"]
mod transport;

fn bound(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

pub(in crate::resident) fn limits() -> A2aExchangeLimits {
    A2aExchangeLimits {
        request_bytes: bound(1024),
        response_bytes: bound(2048),
        evidence_bytes: bound(4096),
        line_bytes: bound(512),
        event_bytes: bound(1024),
        events: bound(4),
        queued_events: bound(2),
        connect_timeout: Duration::from_secs(1),
        read_timeout: Duration::from_secs(2),
        total_timeout: Duration::from_secs(3),
    }
}

pub(in crate::resident) fn route(
    provider: &ProviderBindingV1,
    operation: &OperationFixture,
) -> A2aRouteAdmissionV1 {
    route_for_handle(
        provider,
        &operation.handle,
        "https://agent.example.test/a2a",
        false,
    )
}

fn route_at(
    provider: &ProviderBindingV1,
    operation: &OperationFixture,
    endpoint: &str,
    streaming: bool,
) -> A2aRouteAdmissionV1 {
    route_for_handle(provider, &operation.handle, endpoint, streaming)
}

pub(in crate::resident) fn route_for_handle(
    provider: &ProviderBindingV1,
    handle: &OperationHandleV1,
    endpoint: &str,
    streaming: bool,
) -> A2aRouteAdmissionV1 {
    route_for_handle_at(provider, handle, endpoint, streaming, 10, 50, 60)
}

pub(in crate::resident) fn route_for_handle_at(
    provider: &ProviderBindingV1,
    handle: &OperationHandleV1,
    endpoint: &str,
    streaming: bool,
    observed_at: u64,
    fresh_until: u64,
    expires_at: u64,
) -> A2aRouteAdmissionV1 {
    route_for_handle_at_with_authority(
        provider,
        handle,
        endpoint,
        streaming,
        observed_at,
        fresh_until,
        expires_at,
        &signing_authority(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn route_for_handle_at_with_authority(
    provider: &ProviderBindingV1,
    handle: &OperationHandleV1,
    endpoint: &str,
    streaming: bool,
    observed_at: u64,
    fresh_until: u64,
    expires_at: u64,
    authority: &SigningAuthorityBindingV1,
) -> A2aRouteAdmissionV1 {
    let binding = handle.request_binding();
    let provider_ref = ProviderBindingRefV1 {
        provider_binding_id: provider.id.clone(),
        provider_binding_identity: provider.binding_identity.clone(),
    };
    let route = CommonRequestRouteV1 {
        origin_principal_id: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        origin_node_id: ResidentNodeIdV1::parse("node-1").unwrap(),
        destination_principal_id: ResidentPrincipalIdV1::parse("principal-2").unwrap(),
        destination_node_id: Some(ResidentNodeIdV1::parse("node-2").unwrap()),
        route_id: ResidentRouteIdV1::parse("route-1").unwrap(),
        route_generation: *binding.route_generation(),
        transport: BoundedResidentTextV1::parse("a2a-json-rpc2-https").unwrap(),
        route_binding_schema_id: binding.route_binding_schema_id().clone(),
        route_binding_schema_sha256: binding.route_binding_schema_sha256().clone(),
        route_binding_ref: binding.route_binding_ref().clone(),
        route_binding_sha256: binding.route_binding_sha256().clone(),
        contract_set_ref: binding.contract_set_ref().clone(),
        contract_set_sha256: binding.contract_set_sha256().clone(),
    };
    let schema = schema_digest(A2A_ROUTE_ADMISSION_V1_SCHEMA_ID);
    let runtime = authority
        .integrated_runtime_authority()
        .expect("A2A route fixture requires integrated runtime authority");
    let card = evidence("agent-card/one", 'a');
    let security = evidence("route/security", 'b');
    A2aRouteAdmissionV1::issue_contextual(
        A2aRouteAdmissionPreimageV1 {
            schema_id: SchemaId::parse(A2A_ROUTE_ADMISSION_V1_SCHEMA).unwrap(),
            schema_sha256: schema.clone(),
            route_admission_id: A2aRouteAdmissionIdV1::parse("route-admission-1").unwrap(),
            provider_binding: provider_ref.clone(),
            route: route.clone(),
            endpoint: A2aHttpsEndpointV1::parse(endpoint).unwrap(),
            protocol_version: A2aProtocolVersionV1::V1_0,
            request_binding: A2aRequestBindingV1::JsonRpc2OverVerifiedHttps,
            agent_card: card.clone(),
            streaming: if streaming {
                A2aStreamingAdmissionV1::Advertised {
                    evidence: card.clone(),
                }
            } else {
                A2aStreamingAdmissionV1::NotAdvertised
            },
            security: A2aSecurityAdmissionV1 {
                selected_scheme_ids: vec![],
                evidence: security.clone(),
            },
            observed_at: UnixTimeV1::new(observed_at).unwrap(),
            fresh_until: UnixTimeV1::new(fresh_until).unwrap(),
            expires_at: UnixTimeV1::new(expires_at).unwrap(),
            deployment_runtime_authority_ref: runtime.deployment_runtime_authority_ref.clone(),
            deployment_runtime_authority_sha256: runtime
                .deployment_runtime_authority_sha256
                .clone(),
            issuer_principal_id: authority.principal_id().clone(),
            issuer_key_id: ResidentKeyIdV1::parse(authority.signer_key_id()).unwrap(),
            issuer_key_epoch: *authority.signer_key_epoch(),
        },
        &A2aRouteAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: authority,
            },
            provider_binding: &provider_ref,
            route: &route,
        },
        |preimage| {
            assert_eq!(preimage.agent_card, card);
            assert_eq!(preimage.security.evidence, security);
            Ok(())
        },
        |_, _, _| Ok(signature()),
    )
    .expect("contextual route fixture")
}

pub(in crate::resident) fn prepared(
    operation: &OperationFixture,
    provider: &ProviderBindingV1,
    route: &A2aRouteAdmissionV1,
    kind: &str,
    version: &str,
    limits: A2aExchangeLimits,
) -> A2aPreparedExchange {
    // Physical correlation fixture; the transport tests call the real encoder.
    A2aPreparedExchange::from_encoded(
        request(
            operation,
            provider,
            route,
            kind,
            version,
            limits,
            A2aTaskRequestModeV1::SendMessage,
        ),
        b"same physical body".to_vec(),
        ArtifactDigest::hash_bytes(b"trust input"),
    )
    .unwrap()
}

fn request(
    operation: &OperationFixture,
    provider: &ProviderBindingV1,
    route: &A2aRouteAdmissionV1,
    kind: &str,
    version: &str,
    limits: A2aExchangeLimits,
    mode: A2aTaskRequestModeV1,
) -> A2aExchangeRequest {
    let kind = BoundedResidentTextV1::parse(kind).unwrap();
    let version = BoundedResidentTextV1::parse(version).unwrap();
    let attempt = CapabilityAttemptIdV1::parse("attempt-1").unwrap();
    let semantic_operation = PackageResourceRefV1 {
        package_id: PackageIdV1::parse("operations").unwrap(),
        resource_id: PackageResourceIdV1::parse("observe").unwrap(),
    };
    let invocation = ProviderInvocationRequest::new(
        ProviderInvocationCoordinate::new(
            operation.handle.compiled_generation_sha256(),
            operation.handle.instance_sha256(),
            &semantic_operation,
            operation.handle.request_binding(),
            &attempt,
        ),
        provider,
        &kind,
        &version,
    )
    .unwrap();
    A2aExchangeRequest::new(
        invocation,
        A2aExchangeParents {
            operation: (&operation.handle_ref, &operation.handle),
            observation: (&operation.predecessor_ref, &operation.predecessor),
            route: (&record_ref("route/admitted"), route),
        },
        &A2aJsonRpcRequestIdV1::parse("rpc-1").unwrap(),
        A2aExchangeCommand::SendText {
            mode,
            message_id: "message-1",
            text: "inspect configuration",
        },
        limits,
    )
    .unwrap()
}

struct RecordedExchange(A2aExchangeCoordinate);

impl A2aExchange for RecordedExchange {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.0
    }
    fn poll_next(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        Poll::Ready(None)
    }
    fn cancel(&mut self) {}
}

#[test]
fn implementation_kind_and_version_remain_independent_of_binding_and_body() {
    let operation = OperationFixture::exact();
    let provider = crate::provider_binding_fixtures::provider_binding(
        crate::composition_source_fixtures::participant("a2a-participant"),
    );
    let route = route(&provider, &operation);
    let baseline = prepared(&operation, &provider, &route, "client", "0.0.1", limits());
    let echoed = RecordedExchange(baseline.coordinate().clone());
    let repeated = prepared(&operation, &provider, &route, "client", "0.0.1", limits());
    assert!(echoed.coordinate() == repeated.coordinate());
    for (kind, version) in [("other-client", "0.0.1"), ("client", "0.0.2")] {
        let substituted = prepared(&operation, &provider, &route, kind, version, limits());
        assert_eq!(baseline.bytes(), substituted.bytes());
        assert_eq!(
            baseline.coordinate().request_sha256(),
            substituted.coordinate().request_sha256()
        );
        assert!(echoed.coordinate() != substituted.coordinate());
    }
}

#[test]
fn every_raw_limit_participates_in_the_exact_exchange_coordinate() {
    let operation = OperationFixture::exact();
    let provider = crate::provider_binding_fixtures::provider_binding(
        crate::composition_source_fixtures::participant("a2a-participant"),
    );
    let route = route(&provider, &operation);
    let baseline = prepared(&operation, &provider, &route, "client", "0.0.1", limits());
    let echoed = RecordedExchange(baseline.coordinate().clone());
    let changes: [fn(&mut A2aExchangeLimits); 10] = [
        |v| v.request_bytes = bound(1025),
        |v| v.response_bytes = bound(2049),
        |v| v.evidence_bytes = bound(4097),
        |v| v.line_bytes = bound(513),
        |v| v.event_bytes = bound(1025),
        |v| v.events = bound(5),
        |v| v.queued_events = bound(3),
        |v| v.connect_timeout = Duration::from_millis(1001),
        |v| v.read_timeout = Duration::from_millis(2001),
        |v| v.total_timeout = Duration::from_millis(3001),
    ];
    for change in changes {
        let mut changed = limits();
        change(&mut changed);
        let substituted = prepared(&operation, &provider, &route, "client", "0.0.1", changed);
        assert!(baseline.request().coordinate() == substituted.request().coordinate());
        assert_eq!(
            baseline.coordinate().request_sha256(),
            substituted.coordinate().request_sha256()
        );
        assert!(echoed.coordinate() != substituted.coordinate());
    }
}
