//! Generated content and its exact ownership boundary.

use std::sync::Arc;

use pale_core::records::{
    ArtifactDigest, ExecutionPathSelectionIdV1, SurfaceDigest, SurfaceOwnership,
};

use super::RenderError;

/// Renderer-owned categories written into generated manifests and plans.
///
/// The public projection keeps the stable string spelling for its wire-facing
/// consumers; render code selects only this closed vocabulary.
#[derive(Debug, Clone, Copy)]
pub(in crate::render) enum SurfaceKind {
    FrontDoor,
    PolicySkill,
    PolicyInvocationPolicy,
    OperationSkill,
    OperationInvocationPolicy,
    DoctrineSkill,
    StackSkill,
    RunModeSkill,
    SpecialistAgent,
    /// Inert explanation of why a repository-hook contribution is not emitted.
    HookActivationIntent,
    /// Exact Pale-owned fragment for a later structural native-config edit.
    RepositoryHookContribution,
    HarnessProjectionManifest,
    OperationContract,
    OperationCatalog,
    RunModeContext,
    Injection,
    LifecycleGuidance,
    OperationRouteBinding,
    HookContext,
    SemanticIndex,
    Capabilities,
    CapabilityBindings,
    NativeActionCatalog,
    NativeActionInputSchema,
    /// Inert Pale-owned input for a later structural tool-server attachment.
    NativeActionToolServerContribution,
}

impl SurfaceKind {
    /// Return the stable manifest spelling for this closed renderer-owned kind.
    fn as_str(self) -> &'static str {
        match self {
            Self::FrontDoor => "front-door",
            Self::PolicySkill => "policy-skill",
            Self::PolicyInvocationPolicy => "policy-invocation-policy",
            Self::OperationSkill => "operation-skill",
            Self::OperationInvocationPolicy => "operation-invocation-policy",
            Self::DoctrineSkill => "doctrine-skill",
            Self::StackSkill => "stack-skill",
            Self::RunModeSkill => "run-mode-skill",
            Self::SpecialistAgent => "specialist-agent",
            Self::HookActivationIntent => "hook-activation-intent",
            Self::RepositoryHookContribution => "repository-hook-contribution",
            Self::HarnessProjectionManifest => "harness-projection-manifest",
            Self::OperationContract => "operation-contract",
            Self::OperationCatalog => "operation-catalog",
            Self::RunModeContext => "run-mode-context",
            Self::Injection => "injection",
            Self::LifecycleGuidance => "lifecycle-guidance",
            Self::OperationRouteBinding => "operation-route-binding",
            Self::HookContext => "hook-context",
            Self::SemanticIndex => "semantic-index",
            Self::Capabilities => "capabilities",
            Self::CapabilityBindings => "capability-bindings",
            Self::NativeActionCatalog => "native-action-catalog",
            Self::NativeActionInputSchema => "native-action-input-schema",
            Self::NativeActionToolServerContribution => "native-action-tool-server-contribution",
        }
    }
}

/// One generated surface (file or logical artifact).
///
/// This is compiler output, not effect authority. Materialization revalidates
/// targets and binds exact owned bytes into the manifest and admitted plan.
#[derive(Clone)]
pub struct GeneratedSurface {
    /// Repo-relative path, or a logical target for artifact-only hosts.
    pub rel_path: String,
    /// Stable renderer-owned category used by manifests and materialization.
    pub kind: String,
    /// Exact filesystem region owned by this surface.
    pub ownership: SurfaceOwnership,
    /// Exact rendered bytes before whole-file or managed-block ownership is applied.
    content: Arc<String>,
    /// Exact neutral execution-path selections that consume this surface.
    ///
    /// The full path remains single-sourced in the effective profile. Carrying
    /// only its stable identity here prevents a generated artifact from
    /// reconstructing or relabelling the admitted delegation graph.
    pub execution_path_consumers: Vec<ExecutionPathSelectionIdV1>,
    /// Normalized policy ids this surface renders (traceability).
    pub referenced_policy_ids: Vec<String>,
    /// False when the surface embeds resolved absolute paths or is a global artifact.
    pub committable: bool,
    /// True when placement is an authorization-gated effect (hook wiring, the Gas
    /// Town global block): rendered but never written by `apply`; recorded as a
    /// deferred effect in the plan.
    pub deferred: bool,
    /// For artifact-only / deferred surfaces: the target + deferred-effect note.
    pub target_note: Option<String>,
}

impl std::fmt::Debug for GeneratedSurface {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GeneratedSurface")
            .field("rel_path", &self.rel_path)
            .field("kind", &self.kind)
            .field("ownership", &self.ownership)
            .field("content_bytes", &self.content.len())
            .field("execution_path_consumers", &self.execution_path_consumers)
            .field("referenced_policy_ids", &self.referenced_policy_ids)
            .field("committable", &self.committable)
            .field("deferred", &self.deferred)
            .field(
                "target_note",
                &self.target_note.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

impl GeneratedSurface {
    /// Borrow the exact rendered content of this surface.
    ///
    /// The returned text is the input to ownership-aware hashing and
    /// materialization. Its shared-storage strategy remains an implementation
    /// detail so callers cannot depend on allocation identity.
    #[must_use]
    pub fn content(&self) -> &str {
        self.content.as_str()
    }

    /// Share the immutable rendered allocation with application-owned staging.
    pub(crate) fn shared_content(&self) -> Arc<String> {
        Arc::clone(&self.content)
    }

    /// Construct a whole-file surface that is eligible for materialization.
    pub(in crate::render) fn whole_file(
        rel_path: impl Into<String>,
        kind: SurfaceKind,
        content: impl Into<String>,
        execution_path_consumers: Vec<ExecutionPathSelectionIdV1>,
        referenced_policy_ids: Vec<String>,
        committable: bool,
    ) -> Self {
        Self {
            rel_path: rel_path.into(),
            kind: kind.as_str().to_string(),
            ownership: SurfaceOwnership::WholeFile,
            content: Arc::new(content.into()),
            execution_path_consumers,
            referenced_policy_ids,
            committable,
            deferred: false,
            target_note: None,
        }
    }

    /// A marker-delimited region inside a user-owned file.
    pub(in crate::render) fn managed_block(
        rel_path: impl Into<String>,
        kind: SurfaceKind,
        content: impl Into<String>,
        execution_path_consumers: Vec<ExecutionPathSelectionIdV1>,
        referenced_policy_ids: Vec<String>,
        committable: bool,
    ) -> Result<Self, RenderError> {
        let surface = Self {
            rel_path: rel_path.into(),
            kind: kind.as_str().to_string(),
            ownership: SurfaceOwnership::ManagedBlock,
            content: Arc::new(content.into()),
            execution_path_consumers,
            referenced_policy_ids,
            committable,
            deferred: false,
            target_note: None,
        };
        surface.owned_sha256()?;
        Ok(surface)
    }

    /// Attach placement guidance without changing materialization eligibility.
    pub(in crate::render) fn with_note(mut self, note: impl Into<String>) -> Self {
        self.target_note = Some(note.into());
        self
    }

    /// The canonical digest record shared with generated-surface manifests.
    pub(crate) fn digest_record(&self) -> Result<SurfaceDigest, RenderError> {
        Ok(SurfaceDigest {
            rel_path: self.rel_path.clone(),
            kind: self.kind.clone(),
            ownership: self.ownership,
            execution_path_consumers: self.execution_path_consumers.clone(),
            committable: self.committable,
            sha256: self.owned_sha256()?,
        })
    }

    /// Hash only the bytes Pale owns under the declared ownership mode.
    ///
    /// Managed-block wrappers remain destination-owned and therefore cannot
    /// perturb the digest used for reconciliation.
    fn owned_sha256(&self) -> Result<ArtifactDigest, RenderError> {
        let owned = match self.ownership {
            SurfaceOwnership::WholeFile => self.content.as_str(),
            SurfaceOwnership::ManagedBlock => {
                pale_core::managed::split_managed_block(&self.content)
                    .map_err(|error| {
                        RenderError::projection_error(
                            format!("managed surface digest for `{}`", self.rel_path),
                            error,
                        )
                    })?
                    .block()
            }
        };
        Ok(ArtifactDigest::hash_bytes(owned.as_bytes()))
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/render/surface.rs"]
mod tests;
