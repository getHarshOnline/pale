//! Carry SDK requests over explicitly configured, mutually authenticated UDS.

use pale_adapter_process::OsCryptographicRandom;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{num::NonZeroU32, path::Path};

use crate::{
    AuthenticatedResidentCarrier, AuthenticatedResidentCarrierFailure,
    AuthenticatedResidentCarrierSource, AuthenticatedResidentCarrierSourceFailure,
};
use pale_adapter_uds::{
    AuthenticatedLocalClient, FileCallerCredential, LocalEndpointConfiguration,
    LocalRequestDelivery, UnixSocketCarrier,
};
use pale_ports::{CryptographicRandomSourcePort, LocalConnectionSignerPort, LocalIpcFrameLimits};

/// A configured native connection source for either agent host's SDK attachment.
///
/// Each application request uses one fresh authenticated socket. The retained
/// endpoint directory is rechecked before connecting. Credentials, peer pinning
/// and response-record checks remain explicit choices of the composing host.
/// This source discovers no endpoint, creates no keys and retries no payload.
pub struct LocalAgentCarrierSource {
    connection: Arc<Connection>,
}

struct Connection {
    endpoint: LocalEndpointConfiguration,
    limits: LocalIpcFrameLimits,
    signer: Mutex<Box<dyn LocalConnectionSignerPort + Send>>,
    resident_key: [u8; 32],
    random: Arc<dyn CryptographicRandomSourcePort>,
}

/// The selected protected connection files or public-key pins were refused.
#[derive(Debug, thiserror::Error)]
#[error("the selected local agent connection is unavailable or invalid")]
pub struct LocalAgentCarrierSourceError;

impl LocalAgentCarrierSource {
    /// Load explicit protected endpoint and transport-credential files.
    ///
    /// Paths are UTF-8 and must pass the native owner-private, no-follow checks.
    /// The credential is a raw 32-byte Ed25519 seed matching the caller pin.
    /// The connection uses operating-system randomness and a 1 MiB frame bound.
    /// This selects no response authority and opens no socket. Custom carriers
    /// use [`crate::AuthenticatedResidentCarrierSource`] directly.
    ///
    /// # Errors
    /// Refuses unsafe files, invalid endpoint configuration or mismatched credentials.
    pub fn load(
        endpoint_configuration: &str,
        caller_credential: &str,
        caller_public_key: [u8; 32],
        resident_public_key: [u8; 32],
    ) -> Result<Self, LocalAgentCarrierSourceError> {
        let endpoint = LocalEndpointConfiguration::load(Path::new(endpoint_configuration))
            .map_err(|_| LocalAgentCarrierSourceError)?;
        let signer = FileCallerCredential::load(
            Path::new(caller_credential),
            caller_public_key,
            resident_public_key,
        )
        .map_err(|_| LocalAgentCarrierSourceError)?;
        let limits = LocalIpcFrameLimits::new(
            NonZeroU32::new(1024 * 1024).ok_or(LocalAgentCarrierSourceError)?,
        );
        Ok(Self::new(
            endpoint,
            limits,
            Box::new(signer),
            resident_public_key,
            Arc::new(OsCryptographicRandom),
        ))
    }

    /// Retain the caller's selected configuration and transport credentials.
    ///
    /// No connection opens here. Supply this source to the SDK's
    /// [`crate::ResidentAttachment::provision_operations`] together with
    /// independent signed-response admission and a bounded call timeout.
    pub(crate) fn new(
        endpoint: LocalEndpointConfiguration,
        limits: LocalIpcFrameLimits,
        signer: Box<dyn LocalConnectionSignerPort + Send>,
        resident_key: [u8; 32],
        random: Arc<dyn CryptographicRandomSourcePort>,
    ) -> Self {
        Self {
            connection: Arc::new(Connection {
                endpoint,
                limits,
                signer: Mutex::new(signer),
                resident_key,
                random,
            }),
        }
    }
}

impl Connection {
    fn authenticate(
        &self,
        deadline: Instant,
    ) -> Result<
        AuthenticatedLocalClient<UnixSocketCarrier>,
        AuthenticatedResidentCarrierSourceFailure,
    > {
        // Never let a shared or poisoned signer lock renew the SDK's deadline.
        let key = self
            .signer
            .try_lock()
            .map_err(|_| AuthenticatedResidentCarrierSourceFailure)?
            .public_key();
        let carrier = self
            .endpoint
            .connect(self.limits, deadline)
            .map_err(|_| AuthenticatedResidentCarrierSourceFailure)?;
        AuthenticatedLocalClient::authenticate(
            carrier,
            key,
            self.resident_key,
            self.random.as_ref(),
            deadline,
        )
        .map_err(|_| AuthenticatedResidentCarrierSourceFailure)
    }
}

impl AuthenticatedResidentCarrierSource for LocalAgentCarrierSource {
    fn open(
        &mut self,
        deadline: Instant,
    ) -> Result<Box<dyn AuthenticatedResidentCarrier>, AuthenticatedResidentCarrierSourceFailure>
    {
        let authenticated = self.connection.authenticate(deadline)?;
        Ok(Box::new(Carrier {
            connection: self.connection.clone(),
            authenticated: Some(authenticated),
        }))
    }
}

struct Carrier {
    connection: Arc<Connection>,
    authenticated: Option<AuthenticatedLocalClient<UnixSocketCarrier>>,
}

impl AuthenticatedResidentCarrier for Carrier {
    fn exchange(
        &mut self,
        request: &[u8],
        deadline: Instant,
    ) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure> {
        let authenticated = match self.authenticated.take() {
            Some(value) => value,
            None => self
                .connection
                .authenticate(deadline)
                .map_err(|_| AuthenticatedResidentCarrierFailure::ConnectionUnavailable)?,
        };
        let mut signer = self
            .connection
            .signer
            .try_lock()
            .map_err(|_| AuthenticatedResidentCarrierFailure::NotSent)?;
        authenticated
            .exchange(signer.as_mut(), request, deadline)
            .map_err(|error| match error.request_delivery() {
                LocalRequestDelivery::NotSent => AuthenticatedResidentCarrierFailure::NotSent,
                LocalRequestDelivery::Uncertain => {
                    AuthenticatedResidentCarrierFailure::OutcomeUnknown
                }
            })
    }
}
