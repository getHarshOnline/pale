//! MCP negotiation for initialized clients and self-contained stdio requests.

use std::{borrow::Cow, future::Future};

use rmcp::model::{
    ClientNotification, ClientRequest, ErrorData, ProtocolVersion, RequestMetaObject, ServerInfo,
    ServerResult,
};
use rmcp::service::{MaybeSendFuture, NotificationContext, RequestContext, RoleServer, Service};

/// Revisions supported by the transport, newest first.
static SUPPORTED_PROTOCOL_VERSIONS: [ProtocolVersion; 3] = [
    ProtocolVersion::V_2026_07_28,
    ProtocolVersion::V_2025_11_25,
    ProtocolVersion::V_2025_06_18,
];

/// Freshness advertised for the immutable, process-local tool catalog.
///
/// The selected catalog cannot change while the process is alive. A bounded
/// private hint nevertheless prevents a client from carrying built-in
/// descriptors indefinitely after the process that advertised them exits.
pub(super) const TOOL_LIST_CACHE_TTL_MS: u64 = 5 * 60 * 1_000;

/// Return the preferred revision for discovery and initialization fallback.
pub(super) fn protocol_version() -> ProtocolVersion {
    SUPPORTED_PROTOCOL_VERSIONS[0].clone()
}

/// Return the complete protocol-version set advertised by this leaf.
pub(super) fn supported_protocol_versions() -> Cow<'static, [ProtocolVersion]> {
    Cow::Borrowed(&SUPPORTED_PROTOCOL_VERSIONS)
}

/// Accept initialization or require complete protocol metadata on each request.
///
/// The SDK keeps negotiated client information on the connection and selects
/// the response format. Application calls still carry their own inputs and
/// retain no compiler or review session here.
pub(super) struct ProtocolService<S> {
    inner: S,
}

impl<S> ProtocolService<S> {
    /// Wrap the application service with the supported connection modes.
    pub(super) fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Service<RoleServer> for ProtocolService<S>
where
    S: Service<RoleServer>,
{
    async fn handle_request(
        &self,
        request: ClientRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<ServerResult, ErrorData> {
        if let ClientRequest::InitializeRequest(initialize) = &request {
            if context.peer.peer_info().is_some() {
                return Err(ErrorData::invalid_request(
                    "this connection is already initialized",
                    None,
                ));
            }
            let mut client = initialize.params.clone();
            let peer = context.peer.clone();
            let result = self.inner.handle_request(request, context).await?;
            if let ServerResult::InitializeResult(info) = &result {
                // An unsupported proposal can negotiate the server's default.
                // Later responses must use the agreed version, not the proposal.
                client.protocol_version = info.protocol_version.clone();
                peer.set_peer_info(client);
            }
            return Ok(result);
        }

        let declares_protocol = RequestMetaObject::DRAFT_REQUIRED_KEYS
            .iter()
            .any(|key| context.meta.contains_key(*key));
        if !declares_protocol
            && (context.peer.peer_info().is_some()
                || matches!(&request, ClientRequest::PingRequest(_)))
        {
            return self.inner.handle_request(request, context).await;
        }

        let missing = context
            .meta
            .missing_required_keys(&ProtocolVersion::V_2026_07_28);
        if !missing.is_empty() {
            return Err(ErrorData::invalid_params(
                format!(
                    "request _meta is missing or has malformed required fields: {}",
                    missing.join(", ")
                ),
                None,
            ));
        }

        let requested = context
            .meta
            .protocol_version()
            .expect("required protocol metadata was checked above");
        if requested != ProtocolVersion::V_2026_07_28 {
            return Err(ErrorData::unsupported_protocol_version(
                requested,
                &[ProtocolVersion::V_2026_07_28],
            ));
        }

        self.inner.handle_request(request, context).await
    }

    fn handle_notification(
        &self,
        notification: ClientNotification,
        context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = Result<(), ErrorData>> + MaybeSendFuture + '_ {
        self.inner.handle_notification(notification, context)
    }

    fn get_info(&self) -> ServerInfo {
        self.inner.get_info()
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        supported_protocol_versions()
    }
}
