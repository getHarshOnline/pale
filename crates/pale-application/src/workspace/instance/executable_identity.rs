//! Verify configured executable bytes and interface claims, not process health.

use std::path::Path;

use pale_core::canon;
use pale_core::records::{
    ExecutableIdentityDigest, ResolvedProjectInstanceV1, SurfaceExecutableBinding,
    SurfaceExecutableIdentityV1,
};
use pale_ports::{
    ExecutableIdentityError, ExecutableIdentityPort, ExecutableIdentityRequest,
    IdentityProbeRequest, ProbeLimits, ProcessProbePort,
};
use serde::de::IntoDeserializer;

use crate::workspace::WorkspaceError;

/// Verify the configured executable path, bytes, and reported interface identity.
///
/// The executable must be absolute, regular, non-symlink, and executable. Its
/// SHA-256 must match the machine binding before the bounded identity probe is
/// accepted.
pub(crate) fn verify_instance_executable_binding(
    probes: &dyn ProcessProbePort,
    instance: &ResolvedProjectInstanceV1,
) -> Result<ExecutableIdentityDigest, WorkspaceError> {
    let binding = instance
        .surface_executable
        .as_ref()
        .ok_or(WorkspaceError::SurfaceExecutableMissing)?;
    verify_bound_interface_identity(probes, binding)
}

fn executable_identity_failure(
    binding: &SurfaceExecutableBinding,
    error: ExecutableIdentityError,
) -> WorkspaceError {
    WorkspaceError::SurfaceExecutableIdentity {
        exec: binding.exec.clone(),
        source: Box::new(error),
    }
}

fn verify_bound_interface_identity(
    probes: &dyn ProcessProbePort,
    binding: &SurfaceExecutableBinding,
) -> Result<ExecutableIdentityDigest, WorkspaceError> {
    let output = probes
        .probe(IdentityProbeRequest {
            executable: &binding.exec,
            expected_digest: &binding.digest,
            argument: "compiled-identity",
            limits: ProbeLimits::workspace_identity(),
        })
        .map_err(|source| WorkspaceError::SurfaceExecutableProbe {
            exec: binding.exec.clone(),
            source: Box::new(source),
        })?
        .stdout;
    let value = canon::parse_strict(&output).map_err(|source| {
        WorkspaceError::SurfaceExecutableIdentityJson {
            exec: binding.exec.clone(),
            source: Box::new(source),
        }
    })?;
    pale_contracts::validate_value(&value, "surface-executable-identity.v1").map_err(|source| {
        WorkspaceError::SurfaceExecutableIdentitySchema {
            exec: binding.exec.clone(),
            source: Box::new(source),
        }
    })?;
    let identity: SurfaceExecutableIdentityV1 =
        serde_path_to_error::deserialize(value.into_deserializer()).map_err(|source| {
            let value_path = source.path().to_string();
            WorkspaceError::SurfaceExecutableIdentityDecode {
                exec: binding.exec.clone(),
                value_path,
                source: Box::new(source),
            }
        })?;
    if identity.version != binding.version {
        return Err(WorkspaceError::SurfaceExecutableVersionMismatch {
            running: identity.version,
            bound: binding.version.clone(),
        });
    }
    if identity.protocol != binding.protocol {
        return Err(WorkspaceError::SurfaceExecutableProtocolMismatch {
            running: identity.protocol,
            bound: binding.protocol.clone(),
        });
    }
    if identity.os != pale_core::platform::target_os()
        || identity.arch != pale_core::platform::target_arch()
        || identity.durable_publication != pale_core::platform::durable_publication()
    {
        return Err(WorkspaceError::SurfaceExecutablePlatformMismatch {
            exec: binding.exec.clone(),
            running_os: identity.os,
            running_arch: identity.arch,
            running_publication: identity.durable_publication,
            expected_os: pale_core::platform::target_os(),
            expected_arch: pale_core::platform::target_arch(),
            expected_publication: pale_core::platform::durable_publication(),
        });
    }
    Ok(ExecutableIdentityDigest::hash_canonical(&identity)?)
}

/// Verify that this process reports the selected executable binding and that its
/// installed pathname still resolves to the retained expected executable.
///
/// This is fail-closed self-attestation against stale or replaced installation
/// state. It does not turn the child process into an independent witness of the
/// bytes already loaded by a hostile same-user launcher.
///
/// # Errors
///
/// Returns an identity error when the bound or running path is not an absolute,
/// regular, executable non-symlink file, their canonical identities differ, the
/// installed bytes do not match the sealed digest, or the process-reported
/// version and protocol differ from the binding.
pub fn verify_process_executable_binding(
    identities: &dyn ExecutableIdentityPort,
    expected: &SurfaceExecutableBinding,
    running_executable: &Path,
    running_version: &str,
    running_protocol: &str,
) -> Result<(), WorkspaceError> {
    let installed = identities
        .verify_executable(ExecutableIdentityRequest {
            executable: &expected.exec,
            expected_digest: &expected.digest,
        })
        .map_err(|error| executable_identity_failure(expected, error))?;
    if running_executable != installed.canonical_path {
        return Err(WorkspaceError::SurfaceExecutablePathMismatch {
            running: running_executable.display().to_string(),
            bound: installed.canonical_path.display().to_string(),
        });
    }
    if running_version != expected.version {
        return Err(WorkspaceError::SurfaceExecutableVersionMismatch {
            running: running_version.to_string(),
            bound: expected.version.clone(),
        });
    }
    if running_protocol != expected.protocol {
        return Err(WorkspaceError::SurfaceExecutableProtocolMismatch {
            running: running_protocol.to_string(),
            bound: expected.protocol.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/internal/workspace/instance/executable_identity.rs"]
mod tests;
