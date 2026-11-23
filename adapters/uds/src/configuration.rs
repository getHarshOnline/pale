//! Explicit local endpoint selection from bounded, owner-private configuration.

use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use pale_adapter_fs::{
    capture_private_configuration, OwnerPrivateDirectory, PrivateConfigurationError, ReadLimit,
};
use pale_ports::{LocalIpcCarrierError, LocalIpcFrameLimits};
use serde::Deserialize;

use crate::UnixSocketCarrier;

/// Maximum bytes accepted from the selected local configuration file.
const CONFIGURATION_BYTE_LIMIT: usize = 16 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigurationDocument {
    schema_version: u32,
    endpoint: PathBuf,
}

/// Local configuration or its native filesystem protection was refused.
///
/// Diagnostics intentionally include neither configuration bytes nor pathnames.
#[derive(Debug, thiserror::Error)]
pub enum EndpointConfigurationError {
    /// A path is not canonical absolute syntax or a directory was replaced.
    #[error("local endpoint configuration has an invalid or changed path")]
    Path,
    /// Native I/O, no-follow traversal or metadata inspection failed.
    #[error("local endpoint configuration could not be read")]
    Io,
    /// The file or its parent fails the effective-UID/POSIX-mode rule.
    #[error("local endpoint configuration is not owner-private")]
    Protection,
    /// The document is oversized, has an unsupported version or invalid fields.
    #[error("local endpoint configuration has an invalid document")]
    Document,
    /// The selected socket connection failed after configuration admission.
    #[error(transparent)]
    Carrier(#[from] LocalIpcCarrierError),
}

/// One captured explicit endpoint and its retained owner-private directory.
///
/// Loading requires a regular, single-link `0600` file under an effective-UID
/// directory excluding all group/other mode bits. Every path component is
/// opened without following links. The endpoint directory has the same owner
/// and mode rule and its identity is rechecked before a connection.
///
/// These are POSIX filesystem checks, not ACL inspection, protection against
/// another process with the same UID, server authentication, session admission
/// or a live configuration lease. Deployment qualification must cover its ACLs
/// and administrative boundary. No credentials or authority come from this file.
pub struct LocalEndpointConfiguration {
    endpoint: PathBuf,
    parent: OwnerPrivateDirectory,
}

impl LocalEndpointConfiguration {
    /// Capture the endpoint from one explicitly selected local JSON file.
    ///
    /// The closed document is `{"schema_version":1,"endpoint":"/absolute/path"}`.
    /// No search, environment fallback, directory creation or permission repair
    /// occurs. The endpoint itself may not exist yet; its private parent must.
    ///
    /// # Errors
    ///
    /// Returns a redacted error on unsafe path traversal, insufficient POSIX
    /// protection, a non-regular/hard-linked file or invalid bounded document.
    pub fn load(path: &Path) -> Result<Self, EndpointConfigurationError> {
        let bytes = capture_private_configuration(path, ReadLimit::new(CONFIGURATION_BYTE_LIMIT))?;
        let document: ConfigurationDocument =
            serde_json::from_slice(&bytes).map_err(|_| EndpointConfigurationError::Document)?;
        if document.schema_version != 1 {
            return Err(EndpointConfigurationError::Document);
        }
        let (parent, _) = OwnerPrivateDirectory::parent_of(&document.endpoint)?;
        Ok(Self {
            endpoint: document.endpoint,
            parent,
        })
    }

    /// Borrow the exact selected endpoint after rechecking its parent identity.
    ///
    /// This check supplies no server authentication. A deployment must still
    /// authenticate its connected peer and admit signed requests and responses.
    ///
    /// # Errors
    ///
    /// Refuses a missing, newly unsafe or substituted endpoint parent directory.
    pub fn verified_path(&self) -> Result<&Path, EndpointConfigurationError> {
        self.parent.verify_parent_of(&self.endpoint)?;
        Ok(&self.endpoint)
    }

    /// Connect using only the captured endpoint and existing carrier mechanics.
    ///
    /// # Errors
    ///
    /// Returns the configuration/path refusal or the bounded native connect
    /// failure. Success proves transport reachability, not authenticated Pale.
    pub fn connect(
        &self,
        limits: LocalIpcFrameLimits,
        deadline: Instant,
    ) -> Result<UnixSocketCarrier, EndpointConfigurationError> {
        UnixSocketCarrier::connect(self.verified_path()?, limits, deadline).map_err(Into::into)
    }
}

impl From<PrivateConfigurationError> for EndpointConfigurationError {
    fn from(error: PrivateConfigurationError) -> Self {
        match error {
            PrivateConfigurationError::Path => Self::Path,
            PrivateConfigurationError::Io => Self::Io,
            PrivateConfigurationError::Protection => Self::Protection,
            PrivateConfigurationError::Content => Self::Document,
        }
    }
}
