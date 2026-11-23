//! Host-faithful evidence for repository-declared verification commands.

use pale_core::records::{
    CapabilityIdV1, CapabilityProvider, EffectiveProjectProfileV1, LogicalRepositoryIdV1,
    ProviderBindingRefV1, RepositoryVerifierBinding,
};

/// Whether an activated route must observe a configured verifier invocation.
///
/// This is a hook-wiring decision, not a provider-availability or evidence
/// claim. [`classify_repository_verifier`] separately classifies what the host
/// later observed, and even a proven command exit does not decide whether its
/// evidence was sufficient for the caller's conclusion.
pub fn profile_requires_repository_verifier_observation<'a>(
    profile: &EffectiveProjectProfileV1,
    repository_id: &LogicalRepositoryIdV1,
    provider_binding: &ProviderBindingRefV1,
    harness_observed: bool,
    verifier_capabilities: impl IntoIterator<Item = &'a CapabilityIdV1>,
) -> bool {
    harness_observed
        && profile_has_repository_verifier_binding(
            profile,
            repository_id,
            provider_binding,
            verifier_capabilities,
        )
}

/// Whether private configuration contains an exact verifier binding.
///
/// This structural predicate validates that a later observation has something
/// exact to match. It is not reachability, invocation, or evidence proof.
pub(crate) fn profile_has_repository_verifier_binding<'a>(
    profile: &EffectiveProjectProfileV1,
    repository_id: &LogicalRepositoryIdV1,
    provider_binding: &ProviderBindingRefV1,
    verifier_capabilities: impl IntoIterator<Item = &'a CapabilityIdV1>,
) -> bool {
    let Some(repository) = profile
        .repositories
        .iter()
        .find(|repository| &repository.id == repository_id)
    else {
        return false;
    };
    verifier_capabilities.into_iter().any(|capability_id| {
        repository
            .capability_commands
            .get(capability_id)
            .is_some_and(|commands| !commands.is_empty())
            && profile
                .capability_bindings
                .get(capability_id)
                .is_some_and(|binding| {
                    binding.provider() == CapabilityProvider::HostNative
                        && binding.provider_tools().is_some_and(|rows| {
                            rows.iter().any(|row| {
                                row.provider_binding == *provider_binding && !row.tools.is_empty()
                            })
                        })
                })
    })
}

/// Whether any activation root requires verifier observation on this harness.
pub(crate) fn profile_requires_any_activation_repository_verifier_observation(
    profile: &EffectiveProjectProfileV1,
    provider_binding: &ProviderBindingRefV1,
    harness_observed: bool,
    verifier_capabilities: &[CapabilityIdV1],
) -> bool {
    profile.session.as_ref().is_some_and(|session| {
        session.activation_roots.iter().any(|repository_id| {
            profile_requires_repository_verifier_observation(
                profile,
                repository_id,
                provider_binding,
                harness_observed,
                verifier_capabilities,
            )
        })
    })
}

/// What the native hook protocol actually proves about an exact verifier.
#[must_use = "verification evidence must update recency or preserve uncertainty"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationEvidence {
    /// The observed tool invocation is not an exact declared verifier.
    Irrelevant,
    /// The host protocol supplied structured evidence of a successful exit.
    ProvenSuccess,
    /// The invocation matched, but the host supplied no trustworthy exit status.
    CompletedWithoutStatus,
}

/// Classify an exact verifier without parsing forgeable command output.
pub fn classify_repository_verifier(
    host: super::TerminalAdapterDescriptor,
    bindings: &[RepositoryVerifierBinding],
    tool_name: Option<&str>,
    command: Option<&str>,
    tool_response: Option<&serde_json::Value>,
) -> VerificationEvidence {
    let (Some(tool_name), Some(command)) = (tool_name, command) else {
        return VerificationEvidence::Irrelevant;
    };
    let matched = bindings.iter().any(|binding| {
        binding.tool_name == tool_name && crate::host_command::matches(&binding.command, command)
    });
    if !matched {
        return VerificationEvidence::Irrelevant;
    }
    if host.is_claude_code() && claude_bash_success(tool_response) {
        // Claude routes non-zero Bash exits to PostToolUseFailure. Require its
        // structured, non-interrupted Bash response before accepting success.
        VerificationEvidence::ProvenSuccess
    } else {
        // Current Codex exposes command output but omits process exit status.
        // Output text cannot be promoted into a trustworthy status signal.
        VerificationEvidence::CompletedWithoutStatus
    }
}

fn claude_bash_success(response: Option<&serde_json::Value>) -> bool {
    let Some(response) = response.and_then(serde_json::Value::as_object) else {
        return false;
    };
    response
        .get("stdout")
        .is_some_and(serde_json::Value::is_string)
        && response
            .get("stderr")
            .is_some_and(serde_json::Value::is_string)
        && response
            .get("interrupted")
            .and_then(serde_json::Value::as_bool)
            == Some(false)
        && response.get("isImage").and_then(serde_json::Value::as_bool) == Some(false)
}

#[cfg(test)]
#[path = "../../tests/internal/hooks/verification.rs"]
mod tests;
