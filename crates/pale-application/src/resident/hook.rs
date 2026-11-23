//! Typed admission for repository-local native events entering the resident plane.
//!
//! A repository hook carries only a stable [`RepositoryHookRoutingV1`] and one
//! bounded native event. Admission validates their event agreement before any
//! active generation, execution path, policy, context, or session lookup. Those
//! changing coordinates remain resident-owned and are intentionally absent from
//! this request.

use crate::hooks::{
    host_event_name, parse_stdin_event, RepositoryHookRoutingV1, StdinEvent, StdinEventFields,
    WriteBoundary,
};
use crate::host_tool::{matches_pre_tool, protocol_tool_kind, ProtocolToolKind};
use crate::terminal_adapters::TerminalAdapterDescriptor;
use pale_core::records::{
    ArtifactDigest, EffectClass, ExecutionPathSelectionIdV1, HarnessActivityIdentity,
    HookContextV1, InvalidHostIdentity, LifecycleEvent, WorkspaceId,
};
use pale_ports::MachineFilesystemPort;

use super::application::NativeActivitySessionAdmissionError;
use super::{
    AdmittedAmbientSession, AmbientSessionAdmissionContext, AmbientSessionAuthorityPort,
    GenerationLifecycleRecoveryResolver, ResidentApplication,
};

mod caller;
mod child_context;
mod codex_completion;
pub(in crate::resident) mod frame;
mod native_start;
mod operation_registry;
pub(in crate::resident) mod response_policy;
pub use response_policy::{
    NativeHookPermissionRequest, NativeHookResponseAuthorityError, NativeHookResponseAuthorityPort,
};
pub(super) mod prepared;
pub use caller::{CallerBoundResidentHookRequest, ResidentHookCallerBinding};
pub use codex_completion::{
    CodexCommandObservation, CodexCommandObservationError, CodexCommandTerminalStatus,
};
pub use frame::{AuthenticatedResidentHookRequest, ResidentHookFrameAdmissionError};
pub use native_start::{ResidentNativeToolStart, ResidentNativeToolStartError};
pub(in crate::resident) use operation_registry::Occurrence;
pub use operation_registry::{
    CorrelatedResidentHookOperation, ResidentHookNativeToolBinding,
    ResidentHookNativeToolBindingError, ResidentHookOperationRegistry,
    ResidentHookOperationRegistryError, ResidentHookOperationWritePolicyError,
};

/// Maximum native event size admitted at the resident hook boundary.
///
/// The caller may read one byte beyond this ceiling to distinguish an exactly
/// full event from an oversized stream. The admitted request retains parsed
/// fields rather than the source bytes.
pub const RESIDENT_HOOK_EVENT_BYTE_LIMIT: usize = 1 << 20;

/// One structurally admitted repository-hook request awaiting resident lookup.
///
/// This value proves bounded JSON shape and agreement between the static
/// command event and the native payload's event name. It proves no active
/// generation, session admission, execution-path selection, policy decision,
/// context availability, provider readiness, or native effect.
#[must_use = "an admitted native event must be resolved by the resident or rejected"]
pub struct ResidentHookIngressRequest {
    routing: RepositoryHookRoutingV1,
    event: Box<StdinEventFields>,
    native_event_sha256: ArtifactDigest,
}

impl ResidentHookIngressRequest {
    /// Admit one bounded native payload against its static routing coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentHookIngressAdmissionError`] when stdin is absent,
    /// malformed, oversized, or names a different native event. No error text
    /// contains prompt, command, tool input, native identity, or source bytes.
    pub fn admit(
        routing: RepositoryHookRoutingV1,
        bytes: &[u8],
    ) -> Result<Self, ResidentHookIngressAdmissionError> {
        let event = match parse_stdin_event(bytes, RESIDENT_HOOK_EVENT_BYTE_LIMIT) {
            StdinEvent::Absent => return Err(ResidentHookIngressAdmissionError::Absent),
            StdinEvent::Malformed(reason) => {
                return Err(ResidentHookIngressAdmissionError::Malformed(reason));
            }
            StdinEvent::Fields(event) => event,
        };
        let expected = host_event_name(routing.event());
        if event.hook_event_name() != Some(expected) {
            return Err(ResidentHookIngressAdmissionError::EventMismatch { expected });
        }
        Ok(Self {
            routing,
            event,
            native_event_sha256: ArtifactDigest::hash_bytes(bytes),
        })
    }

    /// Borrow the stable repository-local routing coordinate.
    #[must_use]
    pub const fn routing(&self) -> &RepositoryHookRoutingV1 {
        &self.routing
    }

    /// Borrow structurally admitted native fields for resident policy lookup.
    ///
    /// These fields remain observations. Their session, tool, path, and effect
    /// claims acquire no authority through parsing.
    #[must_use]
    pub const fn event(&self) -> &StdinEventFields {
        &self.event
    }

    /// Original bounded payload identity, without retaining its private bytes.
    /// This is provenance only, not authenticated native-source evidence.
    #[must_use]
    pub const fn native_event_sha256(&self) -> &ArtifactDigest {
        &self.native_event_sha256
    }

    /// Recover and currently re-admit this request's unique ambient session.
    ///
    /// The native identity is hashed before lookup. Absence, malformed identity,
    /// ambiguous correlation, stale authority, and incomplete private custody all
    /// fail closed. This step does not create an operation or evaluate a tool.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentHookSessionAdmissionError`] when the event cannot name
    /// one exact live session or current authority no longer admits that session.
    fn admit_native_session<P: AmbientSessionAuthorityPort>(
        &self,
        application: &ResidentApplication<'_>,
        historical: &dyn GenerationLifecycleRecoveryResolver,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<AdmittedResidentHookSession<'_>, ResidentHookSessionAdmissionError> {
        let activity =
            HarnessActivityIdentity::try_from_host(self.event.session_id(), self.event.turn_id())
                .map_err(ResidentHookSessionAdmissionErrorKind::InvalidNativeIdentity)?
                .ok_or(ResidentHookSessionAdmissionErrorKind::NativeIdentityAbsent)?;
        let session = application
            .admit_native_activity_session(&activity, historical, context, authority)
            .map_err(ResidentHookSessionAdmissionErrorKind::Session)
            .map_err(ResidentHookSessionAdmissionError)?;
        let workspace_id = &context.generation.effective_profile.workspace_id;
        if !self.matches_compiled_route(application, workspace_id, &session) {
            return Err(ResidentHookSessionAdmissionError(
                ResidentHookSessionAdmissionErrorKind::RouteSubstitution,
            ));
        }
        Ok(AdmittedResidentHookSession {
            request: self,
            session,
            workspace_id: workspace_id.clone(),
        })
    }

    fn select_hook_policy<'a>(
        &self,
        application: &'a ResidentApplication<'_>,
        workspace_id: &WorkspaceId,
        session: &AdmittedAmbientSession,
    ) -> Result<SelectedHookPolicy<'a>, ResidentHookPolicyAdmissionError> {
        if !self.matches_compiled_route(application, workspace_id, session) {
            return Err(ResidentHookPolicyAdmissionError::RouteSubstitution);
        }

        let lease = session.lease();
        let activity =
            HarnessActivityIdentity::try_from_host(self.event.session_id(), self.event.turn_id())
                .map_err(|_| ResidentHookPolicyAdmissionError::NativeSessionSubstitution)?
                .ok_or(ResidentHookPolicyAdmissionError::NativeSessionSubstitution)?;
        if !matches!(
            lease.native_session_correlation(),
            pale_core::records::NativeSessionCorrelationV1::Observed {
                native_session_identity,
            } if native_session_identity == &activity
        ) {
            return Err(ResidentHookPolicyAdmissionError::NativeSessionSubstitution);
        }
        let generation = application
            .generation(lease.compiled_generation_digest())
            .ok_or(ResidentHookPolicyAdmissionError::MissingGeneration)?;
        let path = generation
            .hook_path(lease.repository_id(), lease.execution_path().id())
            .ok_or(ResidentHookPolicyAdmissionError::MissingExecutionPath)?;
        if !path.events.contains(&self.routing.event()) {
            return Err(ResidentHookPolicyAdmissionError::UnselectedEvent);
        }
        let policy = path
            .policy
            .as_ref()
            .ok_or(ResidentHookPolicyAdmissionError::MissingHookContext)?;
        let hook_context = &policy.context;
        if hook_context.profile_digest != *lease.profile_digest()
            || hook_context.instance_digest != *lease.instance_digest()
        {
            return Err(ResidentHookPolicyAdmissionError::HookContextSubstitution);
        }
        let injection = policy
            .injection_surface
            .map(|index| {
                generation
                    .repository(lease.repository_id())
                    .and_then(|repository| repository.render().surfaces.get(index))
                    .map(crate::render::GeneratedSurface::content)
                    .ok_or(ResidentHookPolicyAdmissionError::InvalidInjection)
            })
            .transpose()?;

        Ok(SelectedHookPolicy {
            hook_context,
            adapter: policy.adapter,
            injection,
            session_admits_write: lease.effect_ceiling().contains(&EffectClass::Write),
        })
    }

    /// Compare the static hook route with the exact generation-bound path proof.
    fn matches_compiled_route(
        &self,
        application: &ResidentApplication<'_>,
        workspace_id: &WorkspaceId,
        session: &AdmittedAmbientSession,
    ) -> bool {
        let route = self.routing.identity();
        let lease = session.lease();
        if route.workspace_id() != workspace_id || route.repository_id() != lease.repository_id() {
            return false;
        }
        let Some(generation) = application.generation(lease.compiled_generation_digest()) else {
            return false;
        };
        generation
            .hook_path(lease.repository_id(), lease.execution_path().id())
            .is_some_and(|path| {
                &path.selection == lease.execution_path()
                    && path.provider.as_ref() == Some(route.provider_binding())
            })
    }
}

/// One native hook request joined to its unique, currently admitted session.
///
/// The request and session cannot be recombined by the caller. This prevents a
/// same-route session with a wider effect ceiling from evaluating another
/// native session's event.
#[must_use = "an admitted resident hook session must be evaluated or deliberately discarded"]
pub struct AdmittedResidentHookSession<'a> {
    request: &'a ResidentHookIngressRequest,
    session: AdmittedAmbientSession,
    workspace_id: WorkspaceId,
}

impl std::fmt::Debug for AdmittedResidentHookSession<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedResidentHookSession")
            .field("request", &"<admitted>")
            .field("session", &"<admitted>")
            .field("workspace", &"<admitted>")
            .finish()
    }
}

impl AdmittedResidentHookSession<'_> {
    /// Borrow the exact recovered and currently re-admitted ambient session.
    #[must_use]
    pub const fn session(&self) -> &AdmittedAmbientSession {
        &self.session
    }

    /// Select the prepared policy for this admitted pre-effect event.
    ///
    /// The result can evaluate the sealed write boundary. It does not authorize,
    /// perform, or deny a native write, and it creates no durable evidence. The
    /// policy evaluator selects no operation. Callers must establish exact
    /// operation correlation and current operation authority separately.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentHookPolicyAdmissionError`] when the event is not a
    /// selected pre-tool or permission-request event, or its current session and
    /// route do not match the prepared generation policy.
    pub fn admit_write_policy<'a>(
        &'a self,
        application: &'a ResidentApplication<'_>,
    ) -> Result<AdmittedResidentHookWritePolicy<'a>, ResidentHookPolicyAdmissionError> {
        if !matches!(
            self.request.routing.event(),
            LifecycleEvent::PreToolUse | LifecycleEvent::PermissionRequest
        ) {
            return Err(ResidentHookPolicyAdmissionError::WrongEvent);
        }
        let policy =
            self.request
                .select_hook_policy(application, &self.workspace_id, &self.session)?;
        Ok(AdmittedResidentHookWritePolicy {
            event: self.request.event(),
            hook_context: policy.hook_context,
            adapter: policy.adapter,
            admits_write: policy.session_admits_write,
        })
    }
}

struct SelectedHookPolicy<'a> {
    hook_context: &'a HookContextV1,
    injection: Option<&'a str>,
    adapter: TerminalAdapterDescriptor,
    session_admits_write: bool,
}

struct HookContextSurfaceCandidate<'a> {
    rel_path: &'a str,
    kind: &'a str,
    consumers: &'a [ExecutionPathSelectionIdV1],
    content: &'a str,
}

impl<'a> From<&'a crate::render::GeneratedSurface> for HookContextSurfaceCandidate<'a> {
    fn from(surface: &'a crate::render::GeneratedSurface) -> Self {
        Self {
            rel_path: &surface.rel_path,
            kind: &surface.kind,
            consumers: &surface.execution_path_consumers,
            content: surface.content(),
        }
    }
}

fn select_hook_context_surface<'a>(
    candidates: impl Iterator<Item = HookContextSurfaceCandidate<'a>>,
    selection_id: &ExecutionPathSelectionIdV1,
) -> Result<&'a str, ResidentHookPolicyAdmissionError> {
    let mut matching = candidates.filter(|candidate| {
        candidate.kind == "hook-context" && candidate.consumers.contains(selection_id)
    });
    let candidate = matching
        .next()
        .ok_or(ResidentHookPolicyAdmissionError::MissingHookContext)?;
    if matching.next().is_some() {
        return Err(ResidentHookPolicyAdmissionError::RepeatedHookContext);
    }
    let expected_path = format!(".pale/execution-paths/{selection_id}/hook-context.json");
    if candidate.rel_path != expected_path
        || candidate.consumers != std::slice::from_ref(selection_id)
    {
        return Err(ResidentHookPolicyAdmissionError::HookContextSurfaceSubstitution);
    }
    Ok(candidate.content)
}

/// One exact compiled write policy selected for an admitted resident session.
///
/// This value can evaluate policy only. It is not a native enforcement token,
/// operation lease, effect authorization, host response, or evidence record.
#[must_use = "an admitted write policy must be evaluated or deliberately discarded"]
#[derive(Debug)]
pub struct AdmittedResidentHookWritePolicy<'a> {
    event: &'a StdinEventFields,
    hook_context: &'a HookContextV1,
    adapter: TerminalAdapterDescriptor,
    admits_write: bool,
}

impl AdmittedResidentHookWritePolicy<'_> {
    /// Evaluate the compiled write boundary without performing a host write.
    ///
    /// A [`ResidentHookWritePolicyEvaluation::NoViolation`] result means only
    /// that this policy found no reason to deny. It grants no effect authority.
    #[must_use]
    pub fn evaluate_write_boundary(
        &self,
        filesystem: &dyn MachineFilesystemPort,
    ) -> ResidentHookWritePolicyEvaluation {
        let Some(tool_name) = self.event.tool_name() else {
            return ResidentHookWritePolicyEvaluation::Deny {
                reason: "Pale write policy: the event has no determinable tool.".to_owned(),
            };
        };
        let tool_kind = protocol_tool_kind(self.adapter, tool_name);
        let has_sealed_binding = self.hook_context.host_tool_effects.iter().any(|binding| {
            matches_pre_tool(
                &binding.tool,
                tool_name,
                self.event.tool_input(),
                self.event.command(),
            )
        });
        if matches!(
            tool_kind,
            ProtocolToolKind::Command | ProtocolToolKind::Unknown
        ) && !has_sealed_binding
        {
            return ResidentHookWritePolicyEvaluation::Deny {
                reason: format!(
                    "Pale write policy: tool `{tool_name}` has no matching sealed effect binding."
                ),
            };
        }
        let facts = self
            .event
            .pre_tool_use_facts(self.adapter, &self.hook_context.host_tool_effects);
        if let Some(reason) = WriteBoundary::new(
            self.adapter,
            &self.hook_context.read_only_dirs,
            &self.hook_context.protected_runtime_dirs,
            &self.hook_context.host_tool_effects,
        )
        .violation(filesystem, facts)
        {
            return ResidentHookWritePolicyEvaluation::Deny { reason };
        }

        let is_proven_write = matches!(tool_kind, ProtocolToolKind::StructuredWrite)
            || self.hook_context.host_tool_effects.iter().any(|binding| {
                binding.effect_class == EffectClass::Write
                    && matches_pre_tool(
                        &binding.tool,
                        tool_name,
                        self.event.tool_input(),
                        self.event.command(),
                    )
            });
        if is_proven_write && !self.admits_write {
            return ResidentHookWritePolicyEvaluation::Deny {
                reason:
                    "Pale write policy: the admitted authority does not include this write effect."
                        .to_owned(),
            };
        }
        ResidentHookWritePolicyEvaluation::NoViolation
    }
}

/// Policy-only result for one admitted pre-effect write-boundary evaluation.
///
/// The caller still owns native response encoding and enforcement. Neither
/// disposition proves a host effect or records durable denial evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidentHookWritePolicyEvaluation {
    /// This policy found no reason to deny; it does not authorize a write.
    NoViolation,
    /// The event must not proceed under this compiled policy.
    Deny {
        /// Host-independent reason suitable for the existing response owner.
        reason: String,
    },
}

/// Failure to select an exact compiled policy for an admitted hook event.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResidentHookPolicyAdmissionError {
    /// The selected injection is absent, repeated, empty or differs from its seal.
    #[error("admitted child context has no unique matching sealed injection")]
    InvalidInjection,
    /// The event does not match the requested policy operation.
    #[error("resident hook event does not match the requested policy operation")]
    WrongEvent,
    /// The admitted execution path did not select this lifecycle event.
    #[error("admitted execution path does not select the hook event")]
    UnselectedEvent,
    /// Static routing differs from the admitted generation-bound session path.
    #[error("repository hook route does not match the admitted session path")]
    RouteSubstitution,
    /// Event identity differs from the admitted session correlation.
    #[error("native hook event does not match the admitted session")]
    NativeSessionSubstitution,
    /// The session's immutable generation is no longer retained.
    #[error("admitted hook generation is not retained")]
    MissingGeneration,
    /// The session's repository is absent from its retained generation.
    #[error("admitted hook repository is not retained")]
    MissingRepository,
    /// The session path is absent from compiler support evidence.
    #[error("admitted hook execution path is absent")]
    MissingExecutionPath,
    /// More than one compiler support row claims the session path.
    #[error("admitted hook execution path is repeated")]
    RepeatedExecutionPath,
    /// No retained generated context belongs to the session path.
    #[error("admitted hook context is absent")]
    MissingHookContext,
    /// More than one retained generated context claims the session path.
    #[error("admitted hook context is repeated")]
    RepeatedHookContext,
    /// The generated context path or consumer set differs from its selected path.
    #[error("admitted hook context surface is substituted")]
    HookContextSurfaceSubstitution,
    /// The retained generated context is not strict JSON of the expected shape.
    #[error("admitted hook context is malformed")]
    MalformedHookContext,
    /// The retained context violates its public contract.
    #[error("admitted hook context violates its contract")]
    InvalidHookContext,
    /// The context disagrees with the admitted generation, session, route, or proof.
    #[error("admitted hook context coordinates are substituted")]
    HookContextSubstitution,
    /// The route's provider body is absent from the retained generation.
    #[error("admitted hook provider binding is absent")]
    MissingProviderBinding,
    /// The provider body or terminal-adapter coordinate is invalid.
    #[error("admitted hook provider binding is invalid")]
    InvalidProviderBinding,
}

/// A structurally admitted hook request did not recover current ambient authority.
#[derive(Debug)]
pub struct ResidentHookSessionAdmissionError(ResidentHookSessionAdmissionErrorKind);

impl From<ResidentHookSessionAdmissionErrorKind> for ResidentHookSessionAdmissionError {
    fn from(source: ResidentHookSessionAdmissionErrorKind) -> Self {
        Self(source)
    }
}

#[derive(Debug, thiserror::Error)]
enum ResidentHookSessionAdmissionErrorKind {
    /// The selected connection binding has no observed native activity.
    #[error("hook caller binding requires an observed native activity")]
    UnobservedCallerActivity,
    /// No authenticated connection key matches the selected binding.
    #[error("authenticated hook caller does not match its selected binding")]
    CallerSubstitution,
    /// The event claims another native activity under the selected key.
    #[error("native hook activity does not match its caller binding")]
    CallerActivitySubstitution,
    /// Current recovery selected another lease or grant for the same native activity.
    #[error("recovered hook session does not match its caller binding")]
    CallerSessionSubstitution,
    /// The host exposed neither a session nor a turn identity.
    #[error("native hook event has no session or turn identity")]
    NativeIdentityAbsent,
    /// The selected native identity violated its bounded grammar.
    #[error(transparent)]
    InvalidNativeIdentity(#[from] InvalidHostIdentity),
    /// Live session selection, custody, historical recovery, or current admission failed.
    #[error(transparent)]
    Session(#[from] NativeActivitySessionAdmissionError),
    /// Static hook routing disagrees with the admitted generation and path proof.
    #[error("repository hook route does not match the admitted session path")]
    RouteSubstitution,
}

impl std::fmt::Display for ResidentHookSessionAdmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for ResidentHookSessionAdmissionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

/// Failure before a repository-hook request can enter resident lookup.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResidentHookIngressAdmissionError {
    /// The native harness supplied no event document.
    #[error("native hook stdin is absent")]
    Absent,
    /// The native event violated the bounded structural grammar.
    #[error("native hook stdin is malformed: {0}")]
    Malformed(&'static str),
    /// The payload's native event disagrees with the static command coordinate.
    #[error("native hook stdin does not name the expected `{expected}` event")]
    EventMismatch {
        /// Native event spelling expected by the selected harness protocol.
        expected: &'static str,
    },
}

/// Explicit bootstrap disposition while no resident hook carrier is selected.
///
/// This type prevents the CLI from treating local one-shot evaluation or an
/// invented socket as resident admission. It is removed from the execution path
/// only when an exact resident carrier is implemented and verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("resident hook transport is unavailable in this build")]
pub struct ResidentHookTransportUnavailable;

#[cfg(test)]
#[path = "../../tests/internal/resident/hook.rs"]
mod tests;
