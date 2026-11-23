//! Stable repository-hook routing shared by rendering, admission, and transports.

use pale_core::records::{
    ArtifactDigest, InvalidDigest, InvalidWorkspaceLanguageId, LifecycleEvent,
    LogicalRepositoryIdV1, ProviderBindingIdentityV1, ProviderBindingRefV1, WorkspaceId,
};

/// Identity of one repository-scoped Pale hook contribution.
///
/// The lifecycle event is intentionally absent: every event contributed for
/// this identity is replaced or removed as one repository-local unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryHookIdentityV1 {
    workspace_id: WorkspaceId,
    repository_id: LogicalRepositoryIdV1,
    provider_binding: ProviderBindingRefV1,
}

impl RepositoryHookIdentityV1 {
    /// Construct the exact key used for structural merge and removal.
    #[must_use]
    pub fn new(
        workspace_id: WorkspaceId,
        repository_id: LogicalRepositoryIdV1,
        provider_binding: ProviderBindingRefV1,
    ) -> Self {
        Self {
            workspace_id,
            repository_id,
            provider_binding,
        }
    }

    /// Principal-owned workspace whose active resident state is resolved.
    #[must_use]
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Canonical repository from which the native event originated.
    #[must_use]
    pub fn repository_id(&self) -> &LogicalRepositoryIdV1 {
        &self.repository_id
    }

    /// Exact terminal provider binding that owns the event route.
    #[must_use]
    pub const fn provider_binding(&self) -> &ProviderBindingRefV1 {
        &self.provider_binding
    }
}

/// Stable coordinate carried by a repository-local `pale hook` command.
///
/// This coordinate deliberately excludes compilation generations, policy
/// digests, session state, and filesystem paths. The resident boundary resolves
/// those changing facts when it admits the event. Construction proves only that
/// the selected native harness exposes the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryHookRoutingV1 {
    event: LifecycleEvent,
    identity: RepositoryHookIdentityV1,
}

impl RepositoryHookRoutingV1 {
    /// Validate and construct one repository-hook routing coordinate.
    ///
    /// # Errors
    ///
    /// Construction validates no native event support; the selected adapter
    /// descriptor remains the sole authority for that fact.
    pub fn new(
        event: LifecycleEvent,
        workspace_id: WorkspaceId,
        repository_id: LogicalRepositoryIdV1,
        provider_binding: ProviderBindingRefV1,
    ) -> Result<Self, RepositoryHookRoutingError> {
        Ok(Self {
            event,
            identity: RepositoryHookIdentityV1::new(workspace_id, repository_id, provider_binding),
        })
    }

    /// Parse the exact argument vector that follows the `pale` executable.
    ///
    /// # Errors
    ///
    /// Returns [`RepositoryHookRoutingError`] when the grammar, identity, or
    /// host/event pair is invalid. No permissive aliases are admitted.
    pub fn parse<T: AsRef<str>>(arguments: &[T]) -> Result<Self, RepositoryHookRoutingError> {
        if arguments.len() != 10
            || arguments[0].as_ref() != "hook"
            || arguments[2].as_ref() != "--workspace"
            || arguments[4].as_ref() != "--repository"
            || arguments[6].as_ref() != "--provider-binding"
            || arguments[8].as_ref() != "--provider-binding-identity"
        {
            return Err(RepositoryHookRoutingError::InvalidGrammar);
        }
        let event = LifecycleEvent::vocabulary()
            .find(|event| event.as_str() == arguments[1].as_ref())
            .ok_or(RepositoryHookRoutingError::UnknownEvent)?;
        let workspace_id = WorkspaceId::parse(arguments[3].as_ref())
            .map_err(RepositoryHookRoutingError::InvalidWorkspaceId)?;
        let repository_id = LogicalRepositoryIdV1::parse(arguments[5].as_ref())
            .map_err(RepositoryHookRoutingError::InvalidRepositoryId)?;
        let provider_binding_id = arguments[7].as_ref();
        if provider_binding_id.is_empty() {
            return Err(RepositoryHookRoutingError::InvalidProviderBindingId);
        }
        let provider_binding_identity = ProviderBindingIdentityV1 {
            provider_binding_sha256: ArtifactDigest::parse(arguments[9].as_ref())
                .map_err(RepositoryHookRoutingError::InvalidProviderBindingIdentity)?,
        };
        Self::new(
            event,
            workspace_id,
            repository_id,
            ProviderBindingRefV1 {
                provider_binding_id: provider_binding_id.to_owned(),
                provider_binding_identity,
            },
        )
    }

    /// Encode the canonical argument vector that follows the `pale` executable.
    #[must_use]
    pub fn arguments(&self) -> [String; 10] {
        [
            "hook".to_owned(),
            self.event.as_str().to_owned(),
            "--workspace".to_owned(),
            self.identity.workspace_id.to_string(),
            "--repository".to_owned(),
            self.identity.repository_id.to_string(),
            "--provider-binding".to_owned(),
            self.identity.provider_binding.provider_binding_id.clone(),
            "--provider-binding-identity".to_owned(),
            self.identity
                .provider_binding
                .provider_binding_identity
                .provider_binding_sha256
                .to_string(),
        ]
    }

    /// Native lifecycle event carried by this coordinate.
    #[must_use]
    pub const fn event(&self) -> LifecycleEvent {
        self.event
    }

    /// Repository-local merge and removal key carried by this coordinate.
    #[must_use]
    pub const fn identity(&self) -> &RepositoryHookIdentityV1 {
        &self.identity
    }
}

/// Failure to admit the closed repository-hook command grammar.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryHookRoutingError {
    /// Arguments do not match the single initial-v1 command shape.
    #[error("repository hook arguments do not match the initial-v1 grammar")]
    InvalidGrammar,
    /// The event token is outside the closed lifecycle vocabulary.
    #[error("repository hook event is unknown")]
    UnknownEvent,
    /// The principal-owned workspace identity is malformed.
    #[error("repository hook workspace identity is invalid: {0}")]
    InvalidWorkspaceId(#[source] InvalidWorkspaceLanguageId),
    /// The canonical repository identity is malformed.
    #[error("repository hook repository identity is invalid: {0}")]
    InvalidRepositoryId(#[source] InvalidWorkspaceLanguageId),
    /// The exact provider-binding locator is empty.
    #[error("repository hook provider-binding identity is empty")]
    InvalidProviderBindingId,
    /// The exact provider-binding body identity is not a canonical digest.
    #[error("repository hook provider-binding body identity is invalid: {0}")]
    InvalidProviderBindingIdentity(#[source] InvalidDigest),
}

#[cfg(test)]
#[path = "../../tests/internal/hooks/routing.rs"]
mod tests;
