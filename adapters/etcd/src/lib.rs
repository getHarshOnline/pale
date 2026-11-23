//! Authenticated etcd v3 custody for Pale's existing anti-rollback anchor port.
//!
//! This adapter implements `AntiRollbackAnchorPort` from `pale-ports` on top of
//! `pale-core`. No Pale crate composes it into the resident yet. Transition law
//! stays with the port: the adapter reads one exact key linearizably and submits
//! one compare-and-advance transaction without retry or failover. The caller
//! supplies an admitted cluster identity, namespace and native secrets.
//! Software qualification cannot establish independent administration, backup
//! or restore domains for the actual deployment.

use std::{num::NonZeroU64, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition as Advance, AntiRollbackAnchorError,
    AntiRollbackAnchorErrorKind as ErrorKind, AntiRollbackAnchorKey,
    AntiRollbackAnchorObservation as Observation, AntiRollbackAnchorPort,
    PreparedAntiRollbackAnchorTransition,
};
use reqwest::{blocking::Client, header::HeaderValue, Certificate, Identity, Url};
use serde::Deserialize;
use serde_json::{json, Value};

mod codec;
mod gateway;

/// A local constructor input was invalid; diagnostics disclose no native secrets.
#[derive(Debug, thiserror::Error)]
#[error("invalid etcd anchor configuration")]
pub struct EtcdConfigurationError;

#[derive(Debug, thiserror::Error)]
enum StoreFailure {
    #[error("etcd transport did not complete")]
    Transport,
    #[error("etcd response did not satisfy the anchor contract")]
    Protocol,
    #[error("etcd returned HTTP status {0}")]
    Status(u16),
}

/// One selected HTTPS cluster member and exact anchor namespace.
///
/// The bearer token must be issued by the independent etcd administrator with
/// the intended namespace privileges. Optional mTLS is additional transport
/// protection: the JSON gateway does not turn a certificate CN into etcd auth.
/// With account authentication enabled it rejects a nonempty client CN; use
/// a separately issued transport client certificate without a CN for this route.
/// This type provides no token creation, refresh, user management, key deletion,
/// lease, snapshot restore, automatic retry or membership discovery.
pub struct EtcdAnchorStore {
    endpoint: Url,
    client: Client,
    token: HeaderValue,
    cluster_id: NonZeroU64,
    prefix: Vec<u8>,
}

impl EtcdAnchorStore {
    /// Bind owner-supplied endpoint, trust root, credentials and request ceiling.
    ///
    /// The endpoint must be a plain HTTPS origin without userinfo, query,
    /// fragment or path. The namespace must begin and end with `/`, have at
    /// least one interior byte and be no more than 256 bytes. No network access
    /// occurs here. The client trusts only the supplied CA and does not use
    /// ambient proxies, redirects or automatic request retries.
    /// The token is bounded to 16 KiB, each PEM input to 64 KiB and each request
    /// timeout to a positive duration no greater than 30 seconds. This synchronous
    /// port must run in the host's blocking context, outside an async executor.
    ///
    /// # Errors
    ///
    /// Refuses invalid endpoint, namespace, credential, PEM or timeout inputs,
    /// or failure to construct the pinned native HTTPS client.
    pub fn new(
        endpoint: &str,
        cluster_id: NonZeroU64,
        prefix: &[u8],
        root_ca_pem: &[u8],
        token: &str,
        client_identity_pem: Option<&[u8]>,
        timeout: Duration,
    ) -> Result<Self, EtcdConfigurationError> {
        let endpoint = Url::parse(endpoint).map_err(|_| EtcdConfigurationError)?;
        if endpoint.scheme() != "https"
            || endpoint.host_str().is_none()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.path() != "/"
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || prefix.len() < 3
            || prefix.len() > 256
            || !prefix.starts_with(b"/")
            || !prefix.ends_with(b"/")
            || prefix.iter().any(|byte| byte.is_ascii_control())
            || token.is_empty()
            || token.len() > 16 * 1024
            || root_ca_pem.len() > 64 * 1024
            || client_identity_pem.is_some_and(|pem| pem.len() > 64 * 1024)
            || timeout.is_zero()
            || timeout > Duration::from_secs(30)
        {
            return Err(EtcdConfigurationError);
        }
        let mut token = HeaderValue::from_str(token).map_err(|_| EtcdConfigurationError)?;
        token.set_sensitive(true);
        let root = Certificate::from_pem(root_ca_pem).map_err(|_| EtcdConfigurationError)?;
        let mut builder = Client::builder()
            .https_only(true)
            .tls_certs_only([root])
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(timeout)
            .connect_timeout(timeout);
        if let Some(pem) = client_identity_pem {
            builder =
                builder.identity(Identity::from_pem(pem).map_err(|_| EtcdConfigurationError)?);
        }
        let client = builder.build().map_err(|_| EtcdConfigurationError)?;
        Ok(Self {
            endpoint,
            client,
            token,
            cluster_id,
            prefix: prefix.to_vec(),
        })
    }

    fn advance(
        &self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<Advance, AntiRollbackAnchorError> {
        let observed = self
            .range(transition.key())
            .map_err(|e| error(ErrorKind::Unavailable, e))?;
        let classification = transition.classify_observed(observed.as_ref().map(|o| &o.anchor));
        match classification {
            Observation::ExactSuccessor => return Ok(Advance::ExactReplay),
            Observation::Missing if transition.expected().is_none() => {}
            Observation::ExactPredecessor => {}
            other => return Ok(Advance::NotAdvanced(other)),
        }
        let raw_key = codec::physical_key(&self.prefix, transition.key())
            .map_err(|e| error(ErrorKind::Unavailable, e))?;
        let key = STANDARD.encode(&raw_key);
        let next =
            codec::encode(transition.next()).map_err(|e| error(ErrorKind::Unavailable, e))?;
        let compare = match &observed {
            None => vec![json!({"key":key,"target":"VERSION","result":"EQUAL","version":"0"})],
            Some(observed) => vec![
                json!({"key":key,"target":"VALUE","result":"EQUAL","value":STANDARD.encode(&observed.bytes)}),
                json!({"key":key,"target":"MOD","result":"EQUAL","mod_revision":observed.revision}),
            ],
        };
        let request = json!({"compare":compare,
            "success":[{"request_put":{"key":key,"value":STANDARD.encode(&next)}}, {"request_range":gateway::range_request(&raw_key)}],
            "failure":[{"request_range":gateway::range_request(&raw_key)}]});
        let response = self
            .post("/v3/kv/txn", request)
            .map_err(|e| error(ErrorKind::OutcomeUnknown, e))?;
        self.transaction_result(transition, &raw_key, response)
            .map_err(|e| error(ErrorKind::OutcomeUnknown, e))
    }

    fn transaction_result(
        &self,
        transition: &PreparedAntiRollbackAnchorTransition,
        raw_key: &[u8],
        response: Value,
    ) -> Result<Advance, StoreFailure> {
        #[derive(Deserialize)]
        struct Transaction {
            header: gateway::Header,
            #[serde(default)]
            succeeded: bool,
            responses: Vec<Value>,
        }
        let transaction: Transaction =
            serde_json::from_value(response).map_err(|_| StoreFailure::Protocol)?;
        self.verify_header(&transaction.header)?;
        let expected_count = if transaction.succeeded { 2 } else { 1 };
        if transaction.responses.len() != expected_count {
            return Err(StoreFailure::Protocol);
        }
        if transaction.succeeded
            && transaction.responses[0]
                .get("response_put")
                .and_then(Value::as_object)
                .is_none()
        {
            return Err(StoreFailure::Protocol);
        }
        let value = transaction.responses[expected_count - 1]
            .get("response_range")
            .ok_or(StoreFailure::Protocol)?
            .clone();
        let range: gateway::Range =
            serde_json::from_value(value).map_err(|_| StoreFailure::Protocol)?;
        if range.header.as_ref().map(|h| h.revision.as_str())
            != Some(transaction.header.revision.as_str())
        {
            return Err(StoreFailure::Protocol);
        }
        let observed = gateway::decode_range(range, raw_key, transition.key())?;
        let classification = transition.classify_observed(observed.as_ref().map(|o| &o.anchor));
        if transaction.succeeded {
            if observed.as_ref().map(|o| o.revision.as_str())
                != Some(transaction.header.revision.as_str())
            {
                return Err(StoreFailure::Protocol);
            }
            if classification != Observation::ExactSuccessor {
                return Err(StoreFailure::Protocol);
            }
            Ok(Advance::Advanced)
        } else if classification == Observation::ExactSuccessor {
            Ok(Advance::ExactReplay)
        } else {
            Ok(Advance::NotAdvanced(classification))
        }
    }
}

impl AntiRollbackAnchorPort for EtcdAnchorStore {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.range(key)
            .map(|o| o.map(|o| o.anchor))
            .map_err(|e| error(ErrorKind::Unavailable, e))
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<Advance, AntiRollbackAnchorError> {
        self.advance(transition)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<Observation, AntiRollbackAnchorError> {
        self.read(transition.key())
            .map(|o| transition.classify_observed(o.as_ref()))
    }
}

fn error(kind: ErrorKind, source: StoreFailure) -> AntiRollbackAnchorError {
    AntiRollbackAnchorError::with_source(kind, source)
}

#[cfg(test)]
#[path = "../tests/internal/custody.rs"]
mod tests;
