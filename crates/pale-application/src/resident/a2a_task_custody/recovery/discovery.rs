//! Discover candidates; selected recovery remains with the existing history owner.

use super::*;

/// A bounded candidate associated with one admitted local operation.
/// Construction validates claims, not historical signatures or remote Task meaning.
/// Only [`Self::recover`] releases an admitted remote history.
pub struct DiscoveredA2aTaskBinding {
    store: AuthorityStoreCoordinate,
    scope: OperationA2aBindingScope,
    task_binding_id: A2aTaskBindingIdV1,
    coordinate: ControlRecordCoordinate,
}

/// A physical-prefix page, not an authenticated or complete remote Task set.
pub struct DiscoveredA2aTaskBindings {
    /// Candidates, each independently selectable for protected recovery.
    pub bindings: Vec<DiscoveredA2aTaskBinding>,
    /// Continue the same physical prefix; absence does not establish no Task.
    pub continuation: Option<OperationA2aBindingCursor>,
}

/// Discover retained candidates without knowing binding IDs or private record refs.
///
/// No history is automatically re-admitted for a page. A missing candidate never
/// proves a send did not happen, permits resend, or resolves unknown outcome.
/// Inspection authorization remains external, as for existing history recovery.
/// # Errors
/// Rejects unlike operation/store, physical bounds, malformed claims or unlike
/// application-owned keys. It never fixes, writes or retries retained state.
pub fn discover_a2a_task_bindings(
    store: &AuthorityStoreCoordinate,
    operation: &IssuedLocalOperation,
    cursor: Option<&OperationA2aBindingCursor>,
    maximum_bindings: usize,
    maximum_body_bytes: usize,
    port: &mut dyn OperationCustodyPort,
) -> Result<DiscoveredA2aTaskBindings, A2aTaskRecoveryError> {
    if maximum_bindings == 0
        || maximum_bindings > MAX_OPERATION_A2A_BINDINGS
        || maximum_body_bytes == 0
        || maximum_body_bytes > MAX_OPERATION_A2A_BINDING_BYTES
    {
        return Err(A2aTaskRecoveryError::Bounds);
    }
    let scope = operation_scope(store, operation)?;
    if cursor.is_some_and(|c| c.store() != store || c.scope() != &scope) {
        return Err(A2aTaskRecoveryError::Association);
    }
    let page =
        port.read_a2a_task_bindings(store, &scope, cursor, maximum_bindings, maximum_body_bytes)?;
    if page.bindings.len() > maximum_bindings {
        return Err(A2aTaskRecoveryError::Bounds);
    }
    let mut remaining = maximum_body_bytes;
    let (_, handle) = operation.handle();
    let mut bindings = Vec::with_capacity(page.bindings.len());
    let mut prior = cursor.map(|c| (c.after_transaction().get(), c.after_reference().as_str()));
    for source in &page.bindings {
        remaining = remaining
            .checked_sub(source.binding().canonical_record_jcs().as_slice().len())
            .ok_or(A2aTaskRecoveryError::Bounds)?;
        let position = (
            source.transaction().get(),
            source.binding().coordinate().record_ref().as_str(),
        );
        if prior.is_some_and(|p| p >= position)
            || cursor.is_some_and(|c| source.transaction().get() > c.through().get())
        {
            return Err(A2aTaskRecoveryError::Association);
        }
        prior = Some(position);
        let wire: A2aTaskBindingWireV1 =
            read::decode(source.binding(), A2A_TASK_BINDING_V1_SCHEMA_ID)?;
        if wire.operation_handle_id() != handle.operation_handle_id()
            || wire.operation_handle_ref() != scope.handle().record_ref()
            || wire.operation_handle_sha256().as_str() != scope.handle().record_sha256().as_str()
            || &binding_key(handle, wire.task_binding_id())? != source.binding_key()
        {
            return Err(A2aTaskRecoveryError::Association);
        }
        bindings.push(DiscoveredA2aTaskBinding {
            store: store.clone(),
            scope: scope.clone(),
            task_binding_id: wire.task_binding_id().clone(),
            coordinate: source.binding().coordinate().clone(),
        });
    }
    if let Some(next) = &page.continuation {
        let last = page
            .bindings
            .last()
            .ok_or(A2aTaskRecoveryError::Association)?;
        if next.store() != store
            || next.scope() != &scope
            || next.after_transaction() != last.transaction()
            || next.after_reference() != last.binding().coordinate().record_ref()
            || cursor.is_some_and(|c| c.through() != next.through())
        {
            return Err(A2aTaskRecoveryError::Association);
        }
    }
    Ok(DiscoveredA2aTaskBindings {
        bindings,
        continuation: page.continuation,
    })
}

fn operation_scope(
    store: &AuthorityStoreCoordinate,
    operation: &IssuedLocalOperation,
) -> Result<OperationA2aBindingScope, A2aTaskRecoveryError> {
    if !operation.checkpoint_selection().matches_store(store) {
        return Err(A2aTaskRecoveryError::Association);
    }
    let (reference, handle) = operation.handle();
    let key = crate::resident::operation_observation_custody::operation_handle_key(
        handle.operation_handle_id(),
    )
    .map_err(OperationObservationCustodyProjectionError::from)
    .map_err(A2aTaskCustodyError::from)?;
    Ok(OperationA2aBindingScope::new(
        key,
        read::coordinate(
            reference,
            handle.canonical_operation_handle_sha256().as_str(),
        )?,
    ))
}

impl DiscoveredA2aTaskBinding {
    /// Claimed binding identity for the existing protected recovery input.
    #[must_use]
    pub const fn task_binding_id(&self) -> &A2aTaskBindingIdV1 {
        &self.task_binding_id
    }
    /// Exact selected immutable binding coordinate, not admitted contents.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Reuse the complete protected history owner for this one selected candidate.
    /// All existing historical evidence/signature and current anchor checks remain
    /// mandatory. No new send, current route authorization or full-page walk occurs.
    /// # Errors
    /// Rejects substituted input/coordinate or any existing history recovery error.
    pub fn recover<Signer, State, Rotation, Signature, Checkpoint, CheckpointSignature>(
        &self,
        input: A2aTaskRecoveryInput<'_>,
        port: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        local_admissions: OperationObservationRecoveryAdmission<
            Signer,
            State,
            Rotation,
            Signature,
            Checkpoint,
        >,
        verify_checkpoint_signature: CheckpointSignature,
        remote: A2aTaskRecoveryAdmission<'_>,
    ) -> Result<RecoveredA2aTaskHistory, A2aTaskRecoveryError>
    where
        Checkpoint: FnMut(
            &ResidentRecordRefV1,
            ControlStateCheckpointWireV1,
        ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
        Signer: FnMut(
            &ResidentRecordRefV1,
            &OperationObservationWireV1,
        ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
        State: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        CheckpointSignature: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        if input.store != &self.store
            || input.task_binding_id != &self.task_binding_id
            || operation_scope(input.store, input.operation)? != self.scope
        {
            return Err(A2aTaskRecoveryError::Association);
        }
        let recovered = recover_a2a_task_history(
            input,
            port,
            anchor,
            local_admissions,
            verify_checkpoint_signature,
            remote,
        )?;
        let (reference, binding) = recovered.binding();
        if reference != self.coordinate.record_ref()
            || binding.canonical_a2a_task_binding_sha256().as_str()
                != self.coordinate.record_sha256().as_str()
        {
            return Err(A2aTaskRecoveryError::Association);
        }
        Ok(recovered)
    }
}
