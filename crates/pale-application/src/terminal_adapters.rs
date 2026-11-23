//! Find which coding tool, such as Claude Code or Codex, a provider binding names,
//! and the configuration, events and generated content that tool uses.
//!
//! `compile`, `hooks`, `host_tool`, `operation_admission`, `render` and `resident`
//! build on it; it builds on no other module in this crate.
//!
//! Descriptors select harness configuration, events and generated repository
//! content. They are not filesystem, process or control-store implementations.

use pale_core::records::{
    ExecutionPathSelectionV1, ExecutionPathWitnessV1, ExperienceCompositionPlanV1,
    ProviderBindingRefV1, ProviderBindingV1,
};

/// Opaque handle to one application-owned terminal adapter descriptor.
///
/// Product layout, codec, event, configuration, and probe facts remain behind
/// this registry handle; neutral records carry only exact provider references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalAdapterDescriptor {
    registry_id: &'static str,
    kind: AdapterKind,
}

/// Explicit canonical key for internal sorted maps of exact provider references.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ProviderBindingKey {
    provider_binding_id: String,
    provider_binding_sha256: String,
}

impl From<&ProviderBindingRefV1> for ProviderBindingKey {
    fn from(reference: &ProviderBindingRefV1) -> Self {
        Self {
            provider_binding_id: reference.provider_binding_id.clone(),
            provider_binding_sha256: reference
                .provider_binding_identity
                .provider_binding_sha256
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum AdapterKind {
    ClaudeCode,
    Codex,
}

/// Descriptor for the Claude Code adapter implementation.
const CLAUDE_CODE: TerminalAdapterDescriptor = TerminalAdapterDescriptor {
    registry_id: "pale.adapter.claude-code.v1",
    kind: AdapterKind::ClaudeCode,
};
/// Descriptor for the Codex adapter implementation.
const CODEX: TerminalAdapterDescriptor = TerminalAdapterDescriptor {
    registry_id: "pale.adapter.codex.v1",
    kind: AdapterKind::Codex,
};
/// Complete deterministic terminal-adapter registry.
const DESCRIPTORS: [TerminalAdapterDescriptor; 2] = [CLAUDE_CODE, CODEX];

/// Product-native filesystem layout owned by one terminal adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionLayout {
    front_door: &'static str,
    skills_root: &'static str,
    agents_root: &'static str,
    agent_suffix: &'static str,
}

impl ProjectionLayout {
    /// Repository entrypoint maintained by the adapter.
    pub(crate) const fn front_door(self) -> &'static str {
        self.front_door
    }

    /// Adapter-owned skill directory.
    pub(crate) const fn skills_root(self) -> &'static str {
        self.skills_root
    }

    /// Adapter-owned specialist-agent directory.
    pub(crate) const fn agents_root(self) -> &'static str {
        self.agents_root
    }

    /// Native suffix for one specialist-agent declaration.
    pub(crate) const fn agent_suffix(self) -> &'static str {
        self.agent_suffix
    }
}

impl TerminalAdapterDescriptor {
    /// Registry descriptor for the Claude Code adapter boundary.
    #[must_use]
    pub(crate) const fn claude_code() -> Self {
        CLAUDE_CODE
    }

    /// Registry descriptor for the Codex adapter boundary.
    #[must_use]
    pub(crate) const fn codex() -> Self {
        CODEX
    }

    /// Stable registry identity carried by provider-binding declarations.
    #[must_use]
    pub(crate) const fn registry_id(self) -> &'static str {
        self.registry_id
    }

    /// Stable short label used only inside product-native projections.
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self.kind {
            AdapterKind::ClaudeCode => "claude-code",
            AdapterKind::Codex => "codex",
        }
    }

    /// Whether this descriptor owns the Claude Code adapter contract.
    #[must_use]
    pub(crate) const fn is_claude_code(self) -> bool {
        matches!(self.kind, AdapterKind::ClaudeCode)
    }

    /// Whether this descriptor owns the Codex adapter contract.
    #[must_use]
    pub(crate) const fn is_codex(self) -> bool {
        matches!(self.kind, AdapterKind::Codex)
    }

    /// Filesystem layout owned by this adapter implementation.
    pub(crate) const fn projection_layout(self) -> ProjectionLayout {
        match self.kind {
            AdapterKind::ClaudeCode => ProjectionLayout {
                front_door: "AGENTS.md",
                skills_root: ".claude/skills",
                agents_root: ".claude/agents",
                agent_suffix: ".md",
            },
            AdapterKind::Codex => ProjectionLayout {
                front_door: "AGENTS.md",
                skills_root: ".codex/skills",
                agents_root: ".codex/agents",
                agent_suffix: ".toml",
            },
        }
    }
}

impl std::fmt::Display for TerminalAdapterDescriptor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Iterate the complete sorted application-owned adapter registry.
pub(crate) fn descriptors() -> impl ExactSizeIterator<Item = TerminalAdapterDescriptor> {
    DESCRIPTORS.into_iter()
}

/// Resolve an opaque adapter-plugin identity to its application descriptor.
pub(crate) fn by_plugin_id(
    plugin_id: &str,
) -> Result<TerminalAdapterDescriptor, TerminalAdapterResolutionError> {
    descriptors()
        .find(|descriptor| descriptor.registry_id() == plugin_id)
        .ok_or_else(|| TerminalAdapterResolutionError::UnknownAdapterPlugin(plugin_id.to_owned()))
}

/// Resolve one exact provider reference through the admitted composition plan.
///
/// # Errors
///
/// Returns [`TerminalAdapterResolutionError`] when the binding is absent or repeated, its
/// exact identity differs, or its terminal adapter is not registered.
pub fn for_provider(
    reference: &ProviderBindingRefV1,
    plan: &ExperienceCompositionPlanV1,
) -> Result<TerminalAdapterDescriptor, TerminalAdapterResolutionError> {
    let mut candidates = plan
        .declarations
        .provider_bindings
        .iter()
        .filter(|binding| binding.id == reference.provider_binding_id);
    let binding = candidates.next().ok_or_else(|| {
        TerminalAdapterResolutionError::UnknownProviderBinding(
            reference.provider_binding_id.clone(),
        )
    })?;
    if candidates.next().is_some() {
        return Err(TerminalAdapterResolutionError::RepeatedProviderBinding(
            reference.provider_binding_id.clone(),
        ));
    }
    for_exact_binding(reference, binding)
}

/// Resolve an exact provider binding through the application adapter registry.
///
/// The reference must match both the stable locator and the domain-separated
/// body identity before the adapter-plugin coordinate is inspected.
///
/// # Errors
///
/// Returns [`TerminalAdapterResolutionError::ProviderBindingIdentityMismatch`] when the
/// reference and body differ, or [`TerminalAdapterResolutionError::UnknownAdapterPlugin`]
/// when no terminal implementation owns the exact plugin identity.
pub fn for_exact_binding(
    reference: &ProviderBindingRefV1,
    binding: &ProviderBindingV1,
) -> Result<TerminalAdapterDescriptor, TerminalAdapterResolutionError> {
    if binding.id != reference.provider_binding_id
        || binding.binding_identity != reference.provider_binding_identity
    {
        return Err(
            TerminalAdapterResolutionError::ProviderBindingIdentityMismatch(
                reference.provider_binding_id.clone(),
            ),
        );
    }
    by_plugin_id(&binding.adapter_plugin.adapter_plugin_id)
}

/// Resolve the terminal adapter selected by one neutral execution path.
pub(crate) fn for_selection(
    selection: &ExecutionPathSelectionV1,
    plan: &ExperienceCompositionPlanV1,
) -> Result<(ProviderBindingRefV1, TerminalAdapterDescriptor), TerminalAdapterResolutionError> {
    let witness =
        ExecutionPathWitnessV1::from_compiled_plan(selection, plan).map_err(|source| {
            TerminalAdapterResolutionError::InvalidExecutionPathWitness(Box::new(source))
        })?;
    let reference = witness
        .terminal_provider_binding
        .ok_or(TerminalAdapterResolutionError::MissingTerminalProviderBinding)?;
    let descriptor = for_provider(&reference, plan)?;
    Ok((reference, descriptor))
}

/// Failure to resolve an exact terminal provider through the adapter registry.
#[derive(Debug, thiserror::Error)]
pub enum TerminalAdapterResolutionError {
    /// The neutral execution path does not terminate in a provider-bound leaf.
    #[error("selected execution path has no terminal provider binding")]
    MissingTerminalProviderBinding,
    /// The provider binding ID is absent from the admitted plan.
    #[error("provider binding `{0}` is absent from the admitted composition plan")]
    UnknownProviderBinding(String),
    /// The provider binding ID is not unique in the admitted plan.
    #[error("provider binding `{0}` is repeated in the admitted composition plan")]
    RepeatedProviderBinding(String),
    /// The exact provider-binding identity differs from the admitted body.
    #[error("provider binding `{0}` has a mismatched exact identity")]
    ProviderBindingIdentityMismatch(String),
    /// The provider binding selects no registered terminal adapter.
    #[error("adapter plugin `{0}` has no registered terminal adapter descriptor")]
    UnknownAdapterPlugin(String),
    /// The execution-path witness could not be derived from the admitted plan.
    #[error("execution-path witness is invalid: {0}")]
    InvalidExecutionPathWitness(Box<pale_core::records::InvalidExecutionPathWitnessV1>),
}
