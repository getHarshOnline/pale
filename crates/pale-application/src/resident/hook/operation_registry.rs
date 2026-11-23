//! Bounded one-shot correlation of authenticated hooks with issued operations.

use std::num::NonZeroUsize;

use pale_core::records::{
    ArtifactDigest, HarnessActivityIdentity, InvocationCorrelationV1, LifecycleEvent,
    OperationHandleDigest, TurnIdentityDigest, UnixTimeV1,
};

use crate::operation_admission::ToolUseIdentityDigest;

use super::{
    AdmittedResidentHookSession, AmbientSessionAdmissionContext, AmbientSessionAuthorityPort,
    AuthenticatedResidentHookRequest, GenerationLifecycleRecoveryResolver, ResidentApplication,
    ResidentHookCallerBinding, ResidentHookIngressRequest,
};
use crate::resident::IssuedLocalOperation;

mod tool_binding;
pub use tool_binding::{
    ResidentHookNativeToolBinding, ResidentHookNativeToolBindingError,
    ResidentHookOperationWritePolicyError,
};

/// A finite registry of exact native occurrences and already-issued operations.
///
/// Trusted composition registers an operation after its durable start. A hook
/// cannot register itself or select a different operation. Consumption proves
/// correlation only: current operation progress, dispatch permission, policy,
/// disclosure and native response encoding remain with their existing owners.
/// Entries are process-local and must be registered again from protected
/// recovery after restart; this registry is not durable replay evidence.
pub struct ResidentHookOperationRegistry {
    limit: NonZeroUsize,
    entries: Vec<Entry>,
}

struct Entry {
    caller_key: [u8; 32],
    identity: Occurrence,
    binding: ResidentHookCallerBinding,
    payload: ArtifactDigest,
    handle: OperationHandleDigest,
    expires_at: UnixTimeV1,
    operation: Option<IssuedLocalOperation>,
}

#[derive(PartialEq, Eq)]
pub(in crate::resident) struct Occurrence {
    activity: HarnessActivityIdentity,
    turn: Option<TurnIdentityDigest>,
    tool: ToolUseIdentityDigest,
}

impl Occurrence {
    pub(in crate::resident) fn from_request(
        request: &ResidentHookIngressRequest,
    ) -> Result<Self, Error> {
        let event = request.event();
        Ok(Self {
            activity: HarnessActivityIdentity::try_from_host(event.session_id(), event.turn_id())
                .map_err(|_| Error::InvalidOccurrence)?
                .ok_or(Error::InvalidOccurrence)?,
            turn: event
                .turn_id()
                .map(TurnIdentityDigest::try_from_host_identity)
                .transpose()
                .map_err(|_| Error::InvalidOccurrence)?,
            tool: ToolUseIdentityDigest::try_from_host(
                event.tool_use_id().ok_or(Error::InvalidOccurrence)?,
            )
            .map_err(|_| Error::InvalidOccurrence)?,
        })
    }
}

impl ResidentHookOperationRegistry {
    /// Set the maximum number of live entries, including consumed occurrences.
    #[must_use]
    pub const fn new(limit: NonZeroUsize) -> Self {
        Self {
            limit,
            entries: Vec::new(),
        }
    }

    /// Register one admitted pre-effect event with its exact durable operation.
    ///
    /// The credential comes from the exact frame whose session was admitted.
    /// Neither a separately supplied caller key, parsed ID nor a core lease
    /// can substitute for the issued application operation. The original event
    /// digest is retained, so another payload cannot consume this entry.
    ///
    /// # Errors
    /// Rejects missing native correlation, another session or operation, stale
    /// authority, expiry, duplicate occurrences and a full registry. A duplicate
    /// never replaces an entry, including one already consumed.
    pub fn register<P: AmbientSessionAuthorityPort>(
        &mut self,
        request: &AuthenticatedResidentHookRequest,
        admitted: &AdmittedResidentHookSession<'_>,
        operation: &IssuedLocalOperation,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<(), Error> {
        if !std::ptr::eq(&request.request, admitted.request) {
            return Err(Error::RequestMismatch);
        }
        let caller_key = request.caller_key;
        if !matches!(
            admitted.request.routing().event(),
            LifecycleEvent::PreToolUse | LifecycleEvent::PermissionRequest
        ) {
            return Err(Error::WrongEvent);
        }
        let identity = Occurrence::from_request(admitted.request)?;
        validate_operation(operation, admitted, &identity, context, authority)?;
        let binding = ResidentHookCallerBinding::new(caller_key, admitted.session())
            .map_err(|_| Error::SessionUnavailable)?;
        self.prune(context.now);
        if self.entries.iter().any(|entry| {
            (entry.caller_key == caller_key && entry.identity == identity)
                || &entry.handle == operation.handle().1.canonical_operation_handle_sha256()
        }) {
            return Err(Error::Duplicate);
        }
        if self.entries.len() >= self.limit.get() {
            return Err(Error::Full);
        }
        self.entries.try_reserve(1).map_err(|_| Error::Full)?;
        self.entries.push(Entry {
            caller_key,
            identity,
            binding,
            payload: admitted.request.native_event_sha256().clone(),
            handle: operation
                .handle()
                .1
                .canonical_operation_handle_sha256()
                .clone(),
            expires_at: *admitted.session().lease().expires_at(),
            operation: Some(operation.clone_for_recovery_continuation()),
        });
        Ok(())
    }

    /// Consume the exact occurrence after current session and lease checks.
    ///
    /// Caller and native identity are matched before reading session custody.
    /// Failed admission leaves the entry intact. Successful consumption leaves
    /// a tombstone until session expiry, preventing a second registration of
    /// the same occurrence while that session is live.
    ///
    /// # Errors
    /// Rejects unmatched callers or occurrences, changed payloads, replay,
    /// expired operations and any failure to recover the exact current session.
    pub fn consume<'request, P: AmbientSessionAuthorityPort>(
        &mut self,
        request: &'request AuthenticatedResidentHookRequest,
        application: &ResidentApplication<'_>,
        historical: &dyn GenerationLifecycleRecoveryResolver,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<CorrelatedResidentHookOperation<'request>, Error> {
        let identity = Occurrence::from_request(&request.request)?;
        self.prune(context.now);
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.caller_key == request.caller_key && entry.identity == identity)
            .ok_or(Error::NotRegistered)?;
        if entry.payload != *request.request.native_event_sha256() {
            return Err(Error::PayloadMismatch);
        }
        let operation = entry.operation.as_ref().ok_or(Error::Consumed)?;
        let admitted = request
            .bind_to(&entry.binding)
            .and_then(|bound| {
                bound.admit_native_session(application, historical, context, authority)
            })
            .map_err(|_| Error::SessionUnavailable)?;
        validate_operation(operation, &admitted, &identity, context, authority)?;
        let operation = entry.operation.take().ok_or(Error::Consumed)?;
        Ok(CorrelatedResidentHookOperation {
            admitted,
            operation,
        })
    }

    fn prune(&mut self, now: UnixTimeV1) {
        self.entries.retain(|entry| entry.expires_at > now);
    }

    /// Re-admit an already consumed occurrence retained by the process owner.
    /// The tombstone, exact handle and original event must still agree. This
    /// renews no operation state and grants no permission to dispatch.
    pub(in crate::resident) fn resume_pending<'request, P: AmbientSessionAuthorityPort>(
        &mut self,
        request: &'request AuthenticatedResidentHookRequest,
        operation: &IssuedLocalOperation,
        application: &ResidentApplication<'_>,
        historical: &dyn GenerationLifecycleRecoveryResolver,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<CorrelatedResidentHookOperation<'request>, Error> {
        let identity = Occurrence::from_request(&request.request)?;
        self.prune(context.now);
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.caller_key == request.caller_key && entry.identity == identity)
            .ok_or(Error::NotRegistered)?;
        if entry.operation.is_some()
            || &entry.handle != operation.handle().1.canonical_operation_handle_sha256()
        {
            return Err(Error::OperationMismatch);
        }
        if entry.payload != *request.request.native_event_sha256() {
            return Err(Error::PayloadMismatch);
        }
        let admitted = request
            .bind_to(&entry.binding)
            .and_then(|bound| {
                bound.admit_native_session(application, historical, context, authority)
            })
            .map_err(|_| Error::SessionUnavailable)?;
        validate_operation(operation, &admitted, &identity, context, authority)?;
        Ok(CorrelatedResidentHookOperation {
            admitted,
            operation: operation.clone_for_recovery_continuation(),
        })
    }
}

fn validate_operation<P: AmbientSessionAuthorityPort>(
    operation: &IssuedLocalOperation,
    admitted: &AdmittedResidentHookSession<'_>,
    identity: &Occurrence,
    context: &AmbientSessionAdmissionContext<'_>,
    authority: &mut P,
) -> Result<(), Error> {
    if !operation.same_session(admitted.session()) {
        return Err(Error::OperationMismatch);
    }
    let current = operation
        .revalidate_session(context, authority)
        .map_err(|_| Error::SessionUnavailable)?;
    let lease = operation.historical_operation_lease();
    lease
        .verify_against_parent(current.lease())
        .map_err(|_| Error::OperationMismatch)?;
    if lease.expires_at() <= context.now {
        return Err(Error::Expired);
    }
    let InvocationCorrelationV1::Observed {
        native_invocation_id,
        ..
    } = lease.invocation_correlation()
    else {
        return Err(Error::OperationMismatch);
    };
    if ToolUseIdentityDigest::try_from_host(native_invocation_id.as_str())
        .map_err(|_| Error::InvalidOccurrence)?
        != identity.tool
    {
        return Err(Error::OperationMismatch);
    }
    Ok(())
}

/// An exact one-shot association, with current session admission but no effect grant.
#[must_use = "correlation must reach the operation owner or be deliberately discarded"]
pub struct CorrelatedResidentHookOperation<'request> {
    admitted: AdmittedResidentHookSession<'request>,
    operation: IssuedLocalOperation,
}

impl CorrelatedResidentHookOperation<'_> {
    /// Transfer the issued identity to the process's pending continuation before
    /// its next fallible step. Later use requires fresh session and head checks.
    pub(in crate::resident) fn into_pending_operation(self) -> IssuedLocalOperation {
        self.operation
    }

    /// Borrow the currently re-admitted session and its exact hook event.
    pub const fn session(&self) -> &AdmittedResidentHookSession<'_> {
        &self.admitted
    }

    /// Borrow the issued operation for its existing current-progress checks.
    #[must_use]
    pub const fn operation(&self) -> &IssuedLocalOperation {
        &self.operation
    }
}

impl std::fmt::Debug for CorrelatedResidentHookOperation<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CorrelatedResidentHookOperation(<private occurrence>)")
    }
}

/// A native occurrence could not be registered or consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResidentHookOperationRegistryError {
    /// Session admission belongs to another captured frame.
    #[error("resident hook session was admitted from another frame")]
    RequestMismatch,
    /// Only pre-tool and permission-request events enter this registry.
    #[error("resident hook is not a pre-effect event")]
    WrongEvent,
    /// Native activity, turn or tool-use identity is absent or malformed.
    #[error("resident hook has no valid native occurrence")]
    InvalidOccurrence,
    /// The operation does not retain this session and native invocation.
    #[error("resident hook does not match the issued operation")]
    OperationMismatch,
    /// Current authority or exact session recovery failed.
    #[error("resident hook session is unavailable")]
    SessionUnavailable,
    /// The operation lease has reached its hard expiry.
    #[error("resident hook operation has expired")]
    Expired,
    /// This occurrence already has a registration or replay tombstone.
    #[error("resident hook occurrence is already registered")]
    Duplicate,
    /// Live entries exhausted the configured bound or memory allocation failed.
    #[error("resident hook registry is full")]
    Full,
    /// No exact caller and occurrence have been registered.
    #[error("resident hook occurrence is not registered")]
    NotRegistered,
    /// The matching occurrence supplied different event bytes.
    #[error("resident hook payload differs from its registration")]
    PayloadMismatch,
    /// This occurrence was already consumed once.
    #[error("resident hook occurrence was already consumed")]
    Consumed,
}

use ResidentHookOperationRegistryError as Error;
