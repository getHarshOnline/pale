//! Explicit native hook connection selection; no operation authority is inferred.

use std::{
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use pale_adapter_fs::{capture_private_configuration, ReadLimit};
use pale_adapter_process::OsCryptographicRandom;
use pale_adapter_uds::{FileCallerCredential, LocalEndpointConfiguration};
use pale_ports::LocalIpcFrameLimits;
use serde::Deserialize;

use crate::{LocalAgentCarrierSource, ResidentAttachment};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    schema_version: u32,
    endpoint_configuration: PathBuf,
    caller_credential: PathBuf,
    caller_public_key: [u8; 32],
    resident_public_key: [u8; 32],
    timeout_ms: u64,
}

/// The explicitly selected hook connection configuration was refused.
///
/// Errors omit paths, configuration bytes and credential contents.
#[derive(Debug, thiserror::Error)]
#[error("the selected resident hook connection configuration is unavailable or invalid")]
pub struct ResidentHookConfigurationError;

impl ResidentAttachment {
    /// Load an explicitly selected, owner-private JSON hook configuration.
    ///
    /// The closed document has `schema_version: 1`, absolute
    /// `endpoint_configuration` and `caller_credential` paths, two 32-byte arrays
    /// named `caller_public_key` and `resident_public_key`, and `timeout_ms` in
    /// `1..=30000`. The endpoint file uses the native adapter's endpoint format;
    /// the credential is a separately provisioned raw Ed25519 seed matching the
    /// caller pin. Files must pass native owner-private, no-follow checks.
    ///
    /// No connection opens here. Nothing is searched, generated, installed or
    /// repaired. File custody does not protect against other same-UID processes
    /// or backups. This attachment permits only blocking hook replies; it has
    /// no operation or observation authority and provides no host enforcement.
    ///
    /// # Errors
    /// Refuses unsafe files, invalid documents, mismatched credentials or timeouts.
    pub fn load_hook_configuration(path: &str) -> Result<Self, ResidentHookConfigurationError> {
        let bytes = capture_private_configuration(Path::new(path), ReadLimit::new(16 * 1024))
            .map_err(|_| ResidentHookConfigurationError)?;
        let configuration: Configuration =
            serde_json::from_slice(&bytes).map_err(|_| ResidentHookConfigurationError)?;
        if configuration.schema_version != 1 || !(1..=30_000).contains(&configuration.timeout_ms) {
            return Err(ResidentHookConfigurationError);
        }
        let endpoint = LocalEndpointConfiguration::load(&configuration.endpoint_configuration)
            .map_err(|_| ResidentHookConfigurationError)?;
        let signer = FileCallerCredential::load(
            &configuration.caller_credential,
            configuration.caller_public_key,
            configuration.resident_public_key,
        )
        .map_err(|_| ResidentHookConfigurationError)?;
        let limits = LocalIpcFrameLimits::new(
            NonZeroU32::new(1024 * 1024).ok_or(ResidentHookConfigurationError)?,
        );
        Self::provision_hooks(
            Box::new(LocalAgentCarrierSource::new(
                endpoint,
                limits,
                Box::new(signer),
                configuration.resident_public_key,
                Arc::new(OsCryptographicRandom),
            )),
            Duration::from_millis(configuration.timeout_ms),
        )
        .map_err(|_| ResidentHookConfigurationError)
    }
}
