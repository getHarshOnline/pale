//! Compiled interface identity and current-executable binding verification.

use pale_application::ApplicationFailureDisposition;
use pale_core::records::{
    DiagnosticStageV1, SurfaceExecutableBinding, SurfaceExecutableIdentityV1,
};

use super::{local_host_failure, LocalOneShotCompilation};
use crate::{ApiError, PaleApi};

/// Embedded schema selected for the compiled interface identity projection.
const EXECUTABLE_IDENTITY_SCHEMA: &str = "surface-executable-identity.v1";
/// Initial-v1 major carried by [`CompiledInterfaceIdentity`].
const EXECUTABLE_IDENTITY_SCHEMA_VERSION: u32 = 1;

/// Durable-publication mechanism carried by [`CompiledInterfaceIdentity`].
///
/// The value reports the implementation compiled for this target. It is not a
/// promise about storage hardware, power-loss behavior, installation, or
/// resident readiness.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DurablePublicationMechanism {
    /// Apple `F_FULLFSYNC` for file data followed by parent-directory `fsync`.
    AppleFullFsync,
    /// POSIX file and parent-directory `fsync`.
    UnixFsync,
    /// Rust [`std::fs::File::sync_all`] for file and parent directory.
    StandardSyncAll,
}

/// Version, protocol, target, and publication claims compiled into this interface.
///
/// [`Self::canonical_bytes`] remains the admitted initial-v1
/// [`SurfaceExecutableIdentityV1`] record used for an executable handshake. The
/// record is only a compiled claim until
/// [`PaleApi::verify_current_executable_binding`] binds it to the executable
/// selected by an admitted local workspace. In Python and Node hosts, “current
/// executable” means the interpreter or Node executable, not extension-module
/// or native-addon bytes. Neither this record nor successful binding
/// verification proves a complete deployment, resident state, service readiness, or
/// process health.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledInterfaceIdentity {
    canonical_bytes: Vec<u8>,
    version: String,
    protocol: String,
    os: String,
    arch: String,
    durable_publication: DurablePublicationMechanism,
}

impl CompiledInterfaceIdentity {
    /// Borrow the canonical executable-identity record.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// SDK package version compiled into this process.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Interface protocol implemented by this SDK.
    #[must_use]
    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    /// Operating-system identity compiled for this process.
    #[must_use]
    pub fn os(&self) -> &str {
        &self.os
    }

    /// Architecture identity compiled for this process.
    #[must_use]
    pub fn arch(&self) -> &str {
        &self.arch
    }

    /// Durable-publication implementation compiled for this process.
    #[must_use]
    pub const fn durable_publication(&self) -> DurablePublicationMechanism {
        self.durable_publication
    }
}

impl PaleApi {
    /// Report the version, interface protocol and target compiled into this build.
    ///
    /// This operation reads no executable or workspace state. The returned
    /// record proves only compiled version, protocol, target, and publication-
    /// mechanism claims. Use [`Self::verify_current_executable_binding`] when
    /// an admitted local workspace must bind those claims to its configured
    /// executable bytes. Neither operation establishes a complete deployment, resident
    /// state, service readiness, or process health.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] if the embedded executable schema is incoherent or the
    /// API-owned record cannot satisfy or canonically encode that schema.
    pub fn compiled_interface_identity(&self) -> Result<CompiledInterfaceIdentity, ApiError> {
        let schema_identity = pale_contracts::record_schema_identity(EXECUTABLE_IDENTITY_SCHEMA)
            .map_err(|error| {
                compiled_interface_identity_failure(&self.diagnostic_identity, error)
            })?;
        let durable_publication = pale_core::platform::durable_publication();
        let record = SurfaceExecutableIdentityV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_version: EXECUTABLE_IDENTITY_SCHEMA_VERSION,
            version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol: pale_core::SURFACE_INTERFACE_PROTOCOL.to_owned(),
            os: pale_core::platform::target_os().to_owned(),
            arch: pale_core::platform::target_arch().to_owned(),
            durable_publication,
        };
        pale_contracts::validate_record(&record, EXECUTABLE_IDENTITY_SCHEMA).map_err(|error| {
            compiled_interface_identity_failure(&self.diagnostic_identity, error)
        })?;
        let canonical_bytes = pale_core::canon::to_canonical_string(&record)
            .map_err(|error| compiled_interface_identity_failure(&self.diagnostic_identity, error))?
            .into_bytes();
        Ok(CompiledInterfaceIdentity {
            canonical_bytes,
            version: record.version,
            protocol: record.protocol,
            os: record.os,
            arch: record.arch,
            durable_publication: project_durable_publication(durable_publication),
        })
    }

    /// Verify this process against the executable bound by one local compilation.
    ///
    /// The captured host context supplies the expected binding. The process
    /// adapter verifies configured executable bytes and canonical identity
    /// before application policy compares the running path, version, and
    /// protocol. In Python and Node hosts, the current executable is the
    /// interpreter or Node process, not extension-module or native-addon bytes.
    /// Success returns the same API-owned compiled identity used for handshakes.
    /// It neither starts nor activates a resident, observes resident state, nor
    /// establishes a complete deployment, service readiness, or process health.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when no executable is bound, the current executable
    /// cannot be resolved, configured bytes or identity differ, or compiled
    /// interface identity construction fails.
    pub fn verify_current_executable_binding(
        &self,
        compilation: &LocalOneShotCompilation,
    ) -> Result<CompiledInterfaceIdentity, ApiError> {
        self.verify_bound_current_executable(compilation.host.surface_executable())
            .map_err(|error| current_executable_binding_failure(&self.diagnostic_identity, error))
    }

    pub(super) fn verify_bound_current_executable(
        &self,
        binding: Option<&SurfaceExecutableBinding>,
    ) -> Result<CompiledInterfaceIdentity, CurrentExecutableBindingVerificationError> {
        let binding = binding.ok_or(CurrentExecutableBindingVerificationError::Workspace(
            pale_application::workspace::WorkspaceError::SurfaceExecutableMissing,
        ))?;
        let running = std::env::current_exe()
            .map_err(CurrentExecutableBindingVerificationError::CurrentExecutable)?;
        let running = running
            .canonicalize()
            .map_err(CurrentExecutableBindingVerificationError::CurrentExecutable)?;
        pale_application::workspace::verify_process_executable_binding(
            &pale_adapter_process::LocalProcessProbe,
            binding,
            &running,
            env!("CARGO_PKG_VERSION"),
            pale_core::SURFACE_INTERFACE_PROTOCOL,
        )
        .map_err(CurrentExecutableBindingVerificationError::Workspace)?;
        self.compiled_interface_identity()
            .map_err(CurrentExecutableBindingVerificationError::Identity)
    }
}

fn project_durable_publication(
    value: pale_core::platform::DurablePublication,
) -> DurablePublicationMechanism {
    match value {
        pale_core::platform::DurablePublication::AppleFullFsync => {
            DurablePublicationMechanism::AppleFullFsync
        }
        pale_core::platform::DurablePublication::UnixFsync => {
            DurablePublicationMechanism::UnixFsync
        }
        pale_core::platform::DurablePublication::StandardSyncAll => {
            DurablePublicationMechanism::StandardSyncAll
        }
    }
}

fn compiled_interface_identity_failure<E>(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: E,
) -> ApiError
where
    E: std::error::Error + Send + Sync + 'static,
{
    local_host_failure(
        identity,
        error,
        ApplicationFailureDisposition::Internal,
        DiagnosticStageV1::Internal,
        "executable_identity_incoherent",
        "the compiled executable identity is incoherent",
    )
}

#[derive(Debug, thiserror::Error)]
pub(super) enum CurrentExecutableBindingVerificationError {
    /// Current process identity could not be observed.
    #[error("current executable identity could not be observed")]
    CurrentExecutable(#[source] std::io::Error),
    /// Application-owned executable binding or verification failed.
    #[error(transparent)]
    Workspace(#[from] pale_application::workspace::WorkspaceError),
    /// The API's compiled interface identity could not be constructed.
    #[error("compiled interface identity could not be constructed")]
    Identity(#[source] ApiError),
}

fn current_executable_binding_failure(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: CurrentExecutableBindingVerificationError,
) -> ApiError {
    let disposition = match &error {
        CurrentExecutableBindingVerificationError::CurrentExecutable(_) => {
            ApplicationFailureDisposition::Operational
        }
        CurrentExecutableBindingVerificationError::Workspace(source) => {
            source.failure_disposition()
        }
        CurrentExecutableBindingVerificationError::Identity(_) => {
            ApplicationFailureDisposition::Internal
        }
    };
    local_host_failure(
        identity,
        error,
        disposition,
        DiagnosticStageV1::HostObservation,
        "current_executable_binding_verification_failed",
        "the current process does not satisfy the selected executable binding",
    )
}
