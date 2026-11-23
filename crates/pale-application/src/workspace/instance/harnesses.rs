//! Exact native-harness executable and protocol admission.

use pale_core::records::{NativeHarnessBinding, ProviderBindingRefV1, ResolvedProjectInstanceV1};
use pale_ports::{IdentityProbeRequest, ProbeLimits, ProcessProbePort};

use crate::workspace::WorkspaceError;

/// Verify every selected machine-bound native harness.
///
/// The version probe is intentionally exact and bounded. Protocol behavior is
/// represented by Pale's closed projection revision and independently proven
/// by host conformance tests; a generic version parser cannot establish it.
pub(crate) fn verify_native_harnesses(
    probes: &dyn ProcessProbePort,
    instance: &ResolvedProjectInstanceV1,
) -> Result<Vec<ProviderBindingRefV1>, WorkspaceError> {
    let mut verified = Vec::with_capacity(instance.native_harnesses.len());
    for binding in &instance.native_harnesses {
        verify_native_harness(probes, binding)?;
        verified.push(binding.provider_binding.clone());
    }
    Ok(verified)
}

/// Verify one native harness against its retained executable and protocol projection.
///
/// The executable descriptor and current pathname are compared immediately
/// before and after the bounded version probe. The host process API still
/// consumes a pathname, so this does not claim descriptor-based execution.
///
/// # Errors
///
/// Returns an identity error when the executable cannot be retained, its bytes
/// or filesystem identity change around the probe, its reported version differs,
/// or the selected harness protocol is not the implemented projection revision.
pub fn verify_native_harness(
    probes: &dyn ProcessProbePort,
    binding: &NativeHarnessBinding,
) -> Result<(), WorkspaceError> {
    let provider = &binding.provider_binding.provider_binding_id;
    let output = probes
        .probe(IdentityProbeRequest {
            executable: &binding.exec,
            expected_digest: &binding.digest,
            argument: "--version",
            limits: ProbeLimits::workspace_identity(),
        })
        .map_err(|source| WorkspaceError::NativeHarnessProbe {
            harness: provider.clone(),
            source: Box::new(source),
        })?
        .stdout;
    let reported = std::str::from_utf8(&output)
        .map(str::trim)
        .map_err(|source| WorkspaceError::NativeHarnessUtf8 {
            harness: provider.clone(),
            source,
        })?;
    if reported != binding.version {
        return Err(WorkspaceError::NativeHarnessVersionMismatch {
            harness: provider.clone(),
            actual: reported.to_string(),
            expected: binding.version.clone(),
        });
    }
    Ok(())
}
