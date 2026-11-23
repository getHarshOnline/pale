//! Retain authenticated native starts across the real durable-start continuation.

use std::num::NonZeroUsize;

use pale_core::records::{
    CommonRequestEnvelopeV1, InvocationCorrelationV1, OperationHandleDigest, UnixTimeV1,
    NATIVE_TOOL_REQUEST_V1_SCHEMA_ID,
};
use pale_ports::ResidentOperationFrameFailure as Failure;

use crate::resident::{
    hook::Occurrence, AdmittedAmbientSession, AmbientSessionAdmissionContext,
    AmbientSessionAuthorityPort, AuthenticatedResidentHookRequest,
    GenerationLifecycleRecoveryResolver, IssuedLocalOperation, ResidentApplication,
    ResidentHookCallerBinding, ResidentHookOperationRegistry, ResidentNativeToolStart,
};

/// One current admission, borrowed only during this bounded frame turn.
pub(in crate::resident) struct NativeHookStartContext<'a> {
    pub(in crate::resident) application: &'a ResidentApplication<'a>,
    pub(in crate::resident) session: &'a AdmittedAmbientSession,
    pub(in crate::resident) historical: &'a dyn GenerationLifecycleRecoveryResolver,
    pub(in crate::resident) current: &'a AmbientSessionAdmissionContext<'a>,
}

/// Process-local pending starts and the already-issued one-shot hook registry.
/// Both stores use the process's configured aggregate capacity as their bound.
/// Pending entries retain the original caller key through interrupted custody;
/// a later connection cannot replace it when fetching the durable start result.
pub(in crate::resident) struct NativeHookStarts {
    limit: NonZeroUsize,
    entries: Vec<Entry>,
    registry: ResidentHookOperationRegistry,
}

struct Entry {
    request: CommonRequestEnvelopeV1,
    occurrence: Occurrence,
    native: AuthenticatedResidentHookRequest,
    binding: ResidentHookCallerBinding,
    expires_at: UnixTimeV1,
    registered: Option<OperationHandleDigest>,
    pending: Option<IssuedLocalOperation>,
    response:
        Option<crate::resident::operation_dispatch::hook_response::NativeHookResponseContinuation>,
}

impl NativeHookStarts {
    pub(in crate::resident) const fn new(limit: NonZeroUsize) -> Self {
        Self {
            limit,
            entries: Vec::new(),
            registry: ResidentHookOperationRegistry::new(limit),
        }
    }

    /// Reserve correlation before allocation, without granting native permission.
    pub(in crate::resident) fn reserve<P: AmbientSessionAuthorityPort>(
        &mut self,
        request: &CommonRequestEnvelopeV1,
        candidate: ResidentNativeToolStart,
        caller_key: [u8; 32],
        context: NativeHookStartContext<'_>,
        authority: &mut P,
    ) -> Result<(), Failure> {
        if candidate.body() != request.body() {
            return Err(Failure::InvalidRequest);
        }
        self.entries.retain(|entry| {
            entry.expires_at > context.current.now
                || entry.pending.as_ref().is_some_and(|operation| {
                    operation.handle().1.retain_until() > &context.current.now
                })
        });
        let occurrence =
            Occurrence::from_request(candidate.request()).map_err(|_| Failure::InvalidRequest)?;
        let native = candidate.into_authenticated(caller_key);
        let binding = ResidentHookCallerBinding::new(caller_key, context.session)
            .map_err(|_| Failure::AdmissionDenied)?;
        let _admitted = native
            .bind_to(&binding)
            .and_then(|bound| {
                bound.admit_native_session(
                    context.application,
                    context.historical,
                    context.current,
                    authority,
                )
            })
            .map_err(|_| Failure::AdmissionDenied)?;
        if let Some(entry) = self.entries.iter().find(|entry| {
            entry.request.canonical_request_sha256() == request.canonical_request_sha256()
                || entry.occurrence == occurrence
        }) {
            if entry.request.canonical_request_sha256() != request.canonical_request_sha256()
                || entry.native.caller_key() != caller_key
            {
                return Err(Failure::ReplayConflict);
            }
            // The retained binding also rejects replacement leases and grants.
            let _admitted = entry
                .native
                .bind_to(&entry.binding)
                .and_then(|bound| {
                    bound.admit_native_session(
                        context.application,
                        context.historical,
                        context.current,
                        authority,
                    )
                })
                .map_err(|_| Failure::AdmissionDenied)?;
            return Ok(());
        }
        if self.entries.len() >= self.limit.get() {
            return Err(Failure::ResidentUnavailable);
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| Failure::ResidentUnavailable)?;
        self.entries.push(Entry {
            request: request.clone(),
            occurrence,
            native,
            binding,
            expires_at: *context.session.lease().expires_at(),
            registered: None,
            pending: None,
            response: None,
        });
        Ok(())
    }

    /// Register only the issued result of the matching durable start journey.
    /// Exact start replay does not replace an entry or clear its consumed tombstone.
    pub(in crate::resident) fn issued<P: AmbientSessionAuthorityPort>(
        &mut self,
        request: &CommonRequestEnvelopeV1,
        operation: &IssuedLocalOperation,
        context: NativeHookStartContext<'_>,
        authority: &mut P,
    ) -> Result<(), Failure> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| {
                entry.request.canonical_request_sha256() == request.canonical_request_sha256()
            })
            .ok_or(Failure::AdmissionDenied)?;
        let handle = operation.handle().1;
        if handle.request_binding().canonical_request_sha256()
            != entry.request.canonical_request_sha256()
        {
            return Err(Failure::ReplayConflict);
        }
        if let Some(registered) = &entry.registered {
            return if registered == handle.canonical_operation_handle_sha256() {
                Ok(())
            } else {
                Err(Failure::ReplayConflict)
            };
        }
        let admitted = entry
            .native
            .bind_to(&entry.binding)
            .and_then(|bound| {
                bound.admit_native_session(
                    context.application,
                    context.historical,
                    context.current,
                    authority,
                )
            })
            .map_err(|_| Failure::AdmissionDenied)?;
        self.registry
            .register(
                &entry.native,
                &admitted,
                operation,
                context.current,
                authority,
            )
            .map_err(|_| Failure::AdmissionDenied)?;
        entry.registered = Some(handle.canonical_operation_handle_sha256().clone());
        Ok(())
    }

    fn matching_entry(&self, request: &AuthenticatedResidentHookRequest) -> Result<usize, Failure> {
        let occurrence =
            Occurrence::from_request(request.request()).map_err(|_| Failure::InvalidRequest)?;
        self.entries
            .iter()
            .position(|entry| {
                entry.native.caller_key() == request.caller_key()
                    && entry.occurrence == occurrence
                    && entry.native.request().routing() == request.request().routing()
                    && entry.native.request().native_event_sha256()
                        == request.request().native_event_sha256()
            })
            .ok_or(Failure::AdmissionDenied)
    }

    /// Locate the original signed claim without treating it as current authority.
    pub(in crate::resident) fn original_request(
        &self,
        request: &AuthenticatedResidentHookRequest,
    ) -> Result<&CommonRequestEnvelopeV1, Failure> {
        Ok(&self.entries[self.matching_entry(request)?].request)
    }

    /// Borrow only this exact caller's retained response, including its returned tombstone.
    pub(super) fn response_mut(
        &mut self,
        request: &AuthenticatedResidentHookRequest,
    ) -> Result<
        &mut Option<
            crate::resident::operation_dispatch::hook_response::NativeHookResponseContinuation,
        >,
        Failure,
    > {
        let index = self.matching_entry(request)?;
        Ok(&mut self.entries[index].response)
    }

    /// Move consumed identity into retained state before any fallible head read.
    /// Explicit retries re-admit the same pending occurrence, never register or
    /// consume a second operation. Current head checks remain with recovery.
    pub(in crate::resident) fn continue_hook<'request, P: AmbientSessionAuthorityPort>(
        &mut self,
        request: &'request AuthenticatedResidentHookRequest,
        context: NativeHookStartContext<'_>,
        authority: &mut P,
    ) -> Result<crate::resident::CorrelatedResidentHookOperation<'request>, Failure> {
        let index = self.matching_entry(request)?;
        if self.entries[index].pending.is_none() {
            let correlated = self
                .registry
                .consume(
                    request,
                    context.application,
                    context.historical,
                    context.current,
                    authority,
                )
                .map_err(|_| Failure::AdmissionDenied)?;
            self.entries[index].pending = Some(correlated.into_pending_operation());
        }
        let pending = self.entries[index]
            .pending
            .as_ref()
            .ok_or(Failure::ResidentUnavailable)?;
        self.registry
            .resume_pending(
                request,
                pending,
                context.application,
                context.historical,
                context.current,
                authority,
            )
            .map_err(|_| Failure::AdmissionDenied)
    }
}

/// Native invocation identity must come from the signed occurrence, never a
/// session-wide policy default. Policy still owns the Pale correlation ID.
pub(super) fn admit_invocation_policy(
    request: &CommonRequestEnvelopeV1,
    lease: &super::ResidentProcessOperationLeaseBasis,
) -> Result<(), super::ResidentProcessHostSelectionError> {
    if request.body().schema_id.as_str() != NATIVE_TOOL_REQUEST_V1_SCHEMA_ID {
        return Ok(());
    }
    let native = ResidentNativeToolStart::admit(request.body())
        .map_err(|_| super::ResidentProcessHostSelectionError::StartRequestAssociation)?;
    match &lease.invocation_correlation {
        InvocationCorrelationV1::Observed {
            native_invocation_id,
            ..
        } if native_invocation_id == native.invocation() => Ok(()),
        _ => Err(super::ResidentProcessHostSelectionError::StartRequestAssociation),
    }
}
