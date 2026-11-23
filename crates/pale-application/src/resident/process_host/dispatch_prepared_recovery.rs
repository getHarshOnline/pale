//! Finish one retained `DispatchPrepared` operation after cold restart.

use super::*;
use crate::actions::NativeActionRegistry;
use crate::resident::operation_dispatch::{
    recover_native_operation_preparation, recover_provider_operation_preparation,
    CompletedRecoveredDispatchCustody, IssuedRecoveredDispatchCheckpoint, OperationAppendFailure,
    OperationCheckpointInput, OperationPreparationHistoryInput,
    PreparedRecoveredDispatchCheckpoint, PreparedRecoveredOperationDispatch,
    PreparedRecoveredProviderDispatch, RecoveredDispatchAnchorPending,
    RecoveredDispatchCommitFailure, RecoveredDispatchCustodyFailure,
    RecoveredProviderDispatchRelease,
};
use crate::resident::process_host::restart::ResidentProcessRecoveredPreparedAttempt;
use pale_core::records::{ContractSetV1, SignedResidentAdmissionContextV1};
use std::sync::Arc;

/// Current authority and fresh dispatch identity for one retained preparation.
pub(super) struct ResidentProcessPreparedDispatchInput<'a> {
    pub(super) current: &'a super::super::AmbientSessionAdmissionContext<'a>,
    pub(super) signing: SignedResidentAdmissionContextV1<'a>,
    pub(super) observation_ref: ResidentRecordRefV1,
    pub(super) maximum_selection_bytes: usize,
    pub(super) maximum_record_bytes: usize,
}

/// Why semantic recovery stopped before a signed dispatch existed.
#[derive(Debug)]
pub(super) enum ResidentProcessPreparedDispatchFailure {
    Selection(ResidentProcessHostSelectionError),
    Admission(crate::resident::operation_dispatch::OperationAttemptError),
}

/// A completed recovered dispatch did not match its process-host route.
pub(super) struct ResidentProcessRecoveredProviderReleaseFailure {
    pub(super) dispatch: Box<PreparedRecoveredProviderDispatch>,
    pub(super) custody: Box<CompletedRecoveredDispatchCustody>,
    pub(super) reason: ResidentProcessHostSelectionError,
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Rebuild one exact native action and sign only its dispatch successor.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_native_prepared_dispatch<'operation, Authority, Rotation, Sign>(
        &'operation self,
        attempt: &'operation ResidentProcessRecoveredPreparedAttempt,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessPreparedDispatchInput<'_>,
        registry: &NativeActionRegistry,
        contract_set: &'operation ContractSetV1,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<Arc<PreparedRecoveredOperationDispatch>, ResidentProcessPreparedDispatchFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let (application, _) = self
            .select_prepared_dispatch(attempt, &access, &input)
            .map_err(ResidentProcessPreparedDispatchFailure::Selection)?;
        let generation = application
            .generation(&attempt.start.generation_digest)
            .ok_or(ResidentProcessPreparedDispatchFailure::Admission(
                crate::resident::operation_dispatch::OperationAttemptError::Association,
            ))?;
        let control = &attempt.start.control;
        let operation = control.operation();
        let history = OperationPreparationHistoryInput {
            operation: operation.operation(),
            request: operation.request(),
            session: operation.operation().historical_session_lease(),
            lease: operation.operation().historical_operation_lease(),
            generation,
            prepared: control.observation().selected().1,
            selection: &attempt.selection,
            maximum_selection_bytes: input.maximum_selection_bytes,
        };
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        recover_native_operation_preparation(
            history,
            control.observation(),
            registry,
            contract_set,
            input.current,
            input.signing,
            input.observation_ref,
            authority,
            custody,
            anchor,
            input.maximum_record_bytes,
            admit_rotation,
            sign,
        )
        .map_err(ResidentProcessPreparedDispatchFailure::Admission)
    }

    /// Rebuild one exact provider request and sign only its dispatch successor.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_provider_prepared_dispatch<'operation, Authority, Rotation, Sign>(
        &'operation self,
        attempt: &'operation ResidentProcessRecoveredPreparedAttempt,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessPreparedDispatchInput<'_>,
        admission: ProviderInvocationAdmissionContext<'operation>,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedRecoveredProviderDispatch, ResidentProcessPreparedDispatchFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let (application, _) = self
            .select_prepared_dispatch(attempt, &access, &input)
            .map_err(ResidentProcessPreparedDispatchFailure::Selection)?;
        let generation = application
            .generation(&attempt.start.generation_digest)
            .ok_or(ResidentProcessPreparedDispatchFailure::Admission(
                crate::resident::operation_dispatch::OperationAttemptError::Association,
            ))?;
        let control = &attempt.start.control;
        let operation = control.operation();
        let history = OperationPreparationHistoryInput {
            operation: operation.operation(),
            request: operation.request(),
            session: operation.operation().historical_session_lease(),
            lease: operation.operation().historical_operation_lease(),
            generation,
            prepared: control.observation().selected().1,
            selection: &attempt.selection,
            maximum_selection_bytes: input.maximum_selection_bytes,
        };
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        recover_provider_operation_preparation(
            history,
            control.observation(),
            admission,
            input.current,
            input.signing,
            input.observation_ref,
            authority,
            custody,
            anchor,
            input.maximum_record_bytes,
            admit_rotation,
            sign,
        )
        .map_err(ResidentProcessPreparedDispatchFailure::Admission)
    }

    /// Join one newly completed recovered dispatch to its one-time provider release.
    pub(super) fn complete_recovered_provider_release(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        dispatch: PreparedRecoveredProviderDispatch,
        custody: CompletedRecoveredDispatchCustody,
    ) -> Result<RecoveredProviderDispatchRelease, ResidentProcessRecoveredProviderReleaseFailure>
    {
        let selected_store = match self.deployment_authority_index(selector) {
            Ok(index) => self.deployments[index].store(),
            Err(reason) => {
                return Err(ResidentProcessRecoveredProviderReleaseFailure {
                    dispatch: Box::new(dispatch),
                    custody: Box::new(custody),
                    reason,
                });
            }
        };
        if selected_store != access_store
            || custody.custody().candidate().store() != selected_store
            || !selector.matches_operation(dispatch.operation())
        {
            return Err(ResidentProcessRecoveredProviderReleaseFailure {
                dispatch: Box::new(dispatch),
                custody: Box::new(custody),
                reason: ResidentProcessHostSelectionError::ProviderOperationAuthority,
            });
        }
        RecoveredProviderDispatchRelease::new(dispatch, custody).map_err(|parts| {
            let (dispatch, custody) = *parts;
            ResidentProcessRecoveredProviderReleaseFailure {
                dispatch: Box::new(dispatch),
                custody: Box::new(custody),
                reason: ResidentProcessHostSelectionError::ProviderTaskAssociation,
            }
        })
    }

    /// Compute one dispatch checkpoint under the selected process store.
    pub(super) fn prepare_recovered_dispatch_checkpoint<'dispatch>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        dispatch: &'dispatch Arc<PreparedRecoveredOperationDispatch>,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        limits: pale_ports::AccumulatorPreparationLimits,
    ) -> Result<
        Result<PreparedRecoveredDispatchCheckpoint<'dispatch>, OperationAppendFailure>,
        ResidentProcessHostSelectionError,
    > {
        self.validate_recovered_dispatch_phase(selector, &access, dispatch)?;
        Ok(dispatch.prepare_checkpoint(resolve_store, limits, access.custody))
    }

    /// Sign one exact prepared checkpoint. Attachment accepts no signer later.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn issue_recovered_dispatch_checkpoint<'dispatch, Sign>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        prepared: PreparedRecoveredDispatchCheckpoint<'dispatch>,
        identity: OperationCheckpointInput,
        now: UnixTimeV1,
        schema: &pale_core::records::SchemaDigest,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<
        Result<IssuedRecoveredDispatchCheckpoint, OperationAppendFailure>,
        ResidentProcessHostSelectionError,
    >
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let dispatch = prepared.dispatch();
        let store = self.validate_recovered_dispatch_phase(selector, &access, dispatch)?;
        validate_recovered_checkpoint_input(
            selector,
            store,
            dispatch,
            &identity,
            now,
            checkpoint_authority,
        )?;
        Ok(prepared.issue(identity, now, schema, checkpoint_authority, sign))
    }

    /// Attach the issued checkpoint, commit once and advance its anchor once.
    pub(super) fn retain_recovered_dispatch(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        issued: IssuedRecoveredDispatchCheckpoint,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        limits: pale_ports::AccumulatorPreparationLimits,
    ) -> Result<
        Result<CompletedRecoveredDispatchCustody, RecoveredDispatchCustodyFailure>,
        (
            Box<IssuedRecoveredDispatchCheckpoint>,
            ResidentProcessHostSelectionError,
        ),
    > {
        if let Err(reason) =
            self.validate_recovered_dispatch_phase(selector, &access, issued.dispatch())
        {
            return Err((Box::new(issued), reason));
        }
        let _issued_checkpoint = issued.checkpoint();
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let retained = issued.attach_and_commit(resolve_store, limits, custody, anchor);
        Ok(match retained {
            Ok(completed) => {
                let _ = (
                    completed.observation(),
                    completed.checkpoint(),
                    completed.custody(),
                );
                Ok(completed)
            }
            Err(RecoveredDispatchCustodyFailure::Issued {
                continuation,
                failure,
            }) => Err(RecoveredDispatchCustodyFailure::Issued {
                continuation,
                failure,
            }),
            Err(RecoveredDispatchCustodyFailure::Commit(failure)) => {
                Err(RecoveredDispatchCustodyFailure::Commit(failure))
            }
            Err(RecoveredDispatchCustodyFailure::Anchor(pending)) => {
                Err(RecoveredDispatchCustodyFailure::Anchor(pending))
            }
        })
    }

    /// Resolve only the exact retained database ambiguity by read-back.
    pub(super) fn reconcile_recovered_dispatch(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        failure: RecoveredDispatchCommitFailure,
    ) -> Result<
        Result<RecoveredDispatchAnchorPending, Box<RecoveredDispatchCommitFailure>>,
        (
            Box<RecoveredDispatchCommitFailure>,
            ResidentProcessHostSelectionError,
        ),
    > {
        if let Err(reason) =
            self.validate_recovered_dispatch_phase(selector, &access, failure.dispatch())
        {
            return Err((Box::new(failure), reason));
        }
        Ok(failure.reconcile_exact(access.custody))
    }

    /// Continue only the exact retained independent-anchor transition.
    pub(super) fn continue_recovered_dispatch_anchor(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        pending: RecoveredDispatchAnchorPending,
    ) -> Result<
        Result<CompletedRecoveredDispatchCustody, Box<RecoveredDispatchAnchorPending>>,
        (
            Box<RecoveredDispatchAnchorPending>,
            ResidentProcessHostSelectionError,
        ),
    > {
        if let Err(reason) =
            self.validate_recovered_dispatch_phase(selector, &access, pending.dispatch())
        {
            return Err((Box::new(pending), reason));
        }
        Ok(pending.continue_anchor(access.anchor))
    }

    fn select_prepared_dispatch<'a>(
        &'a self,
        attempt: &ResidentProcessRecoveredPreparedAttempt,
        access: &ResidentDeploymentOperationAccess<'_>,
        input: &ResidentProcessPreparedDispatchInput<'_>,
    ) -> Result<
        (&'a ResidentApplication<'a>, &'a AuthorityStoreCoordinate),
        ResidentProcessHostSelectionError,
    > {
        let selector = &attempt.start.selector;
        let index = self.deployment_authority_index(selector)?;
        let application = self.deployments[index].application.as_ref();
        let store = application.store();
        if access.store != store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        let control = &attempt.start.control;
        let operation = control.operation().operation();
        let (handle_ref, handle) = operation.handle();
        if control.operation().store() != store
            || control.observation().store() != store
            || !attempt.start.lineage.admits(handle_ref, handle)
            || !selector.matches_operation(operation)
        {
            return Err(ResidentProcessHostSelectionError::NativeOperationAuthority);
        }
        if !selector.matches_current_session(input.current)
            || !selector
                .matches_signing_authority(input.current.generation.signed.signing_authority)
        {
            return Err(ResidentProcessHostSelectionError::NativeCurrentAuthority);
        }
        let historical = operation.historical_session_lease();
        if historical.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
            || historical.deployment_runtime_authority_sha256().as_str()
                != selector.runtime_authority_sha256.as_str()
            || historical.issuer_principal_id() != &selector.principal_id
            || operation
                .historical_operation_lease()
                .operation_lease_digest()
                != handle.operation_lease_sha256()
        {
            return Err(ResidentProcessHostSelectionError::HistoricalNativeAuthority);
        }
        let request = control.operation().request().to_preimage();
        let prepared = control.observation().selected();
        let checkpoint = control.observation().current_checkpoint().1;
        if !selector.matches_signing_authority(&request.signing_authority_binding)
            || !observation_matches_selector(selector, prepared.1)
            || !selector.matches_signing_authority(input.signing.signing_authority)
        {
            return Err(ResidentProcessHostSelectionError::NativeSigningAuthority);
        }
        if checkpoint.control_store_id() != store.store_id()
            || checkpoint.control_incarnation_id() != store.incarnation_id()
            || checkpoint.issuer_principal_id() != &selector.principal_id
        {
            return Err(ResidentProcessHostSelectionError::NativeCheckpointAuthority);
        }
        Ok((application, store))
    }

    fn validate_recovered_dispatch_phase<'a>(
        &'a self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: &ResidentDeploymentOperationAccess<'_>,
        dispatch: &PreparedRecoveredOperationDispatch,
    ) -> Result<&'a AuthorityStoreCoordinate, ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        let store = self.deployments[index].store();
        if access.store != store
            || !dispatch
                .operation()
                .checkpoint_selection()
                .matches_store(store)
            || !selector.matches_operation(dispatch.operation())
            || !observation_matches_selector(selector, dispatch.observation().1)
        {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        Ok(store)
    }
}

impl<'host, 'store> ResidentProcessProviderRecoveryDriver<'host, 'store> {
    /// Rebuild one provider dispatch while retaining independent task ownership.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_prepared_dispatch<'operation, Authority, Rotation, Sign>(
        &self,
        attempt: &'operation ResidentProcessRecoveredPreparedAttempt,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessPreparedDispatchInput<'_>,
        admission: ProviderInvocationAdmissionContext<'operation>,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedRecoveredProviderDispatch, ResidentProcessPreparedDispatchFailure>
    where
        'host: 'operation,
        Authority: super::super::AmbientSessionAuthorityPort,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if access.store != &self.store
            || attempt.start.selector != self.selector
            || !self
                .selector
                .matches_operation(attempt.start.control.operation().operation())
        {
            return Err(ResidentProcessPreparedDispatchFailure::Selection(
                ResidentProcessHostSelectionError::ProviderOperationAuthority,
            ));
        }
        let generation = self
            .application
            .generation(&attempt.start.generation_digest)
            .ok_or(ResidentProcessPreparedDispatchFailure::Admission(
                crate::resident::operation_dispatch::OperationAttemptError::Association,
            ))?;
        let control = &attempt.start.control;
        let operation = control.operation();
        let history = OperationPreparationHistoryInput {
            operation: operation.operation(),
            request: operation.request(),
            session: operation.operation().historical_session_lease(),
            lease: operation.operation().historical_operation_lease(),
            generation,
            prepared: control.observation().selected().1,
            selection: &attempt.selection,
            maximum_selection_bytes: input.maximum_selection_bytes,
        };
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        recover_provider_operation_preparation(
            history,
            control.observation(),
            admission,
            input.current,
            input.signing,
            input.observation_ref,
            authority,
            custody,
            anchor,
            input.maximum_record_bytes,
            admit_rotation,
            sign,
        )
        .map_err(ResidentProcessPreparedDispatchFailure::Admission)
    }

    /// Join a new recovered dispatch commit to this driver's one-time release.
    pub(super) fn complete_release(
        &self,
        dispatch: PreparedRecoveredProviderDispatch,
        custody: CompletedRecoveredDispatchCustody,
    ) -> Result<RecoveredProviderDispatchRelease, ResidentProcessRecoveredProviderReleaseFailure>
    {
        if custody.custody().candidate().store() != &self.store
            || !self.selector.matches_operation(dispatch.operation())
        {
            return Err(ResidentProcessRecoveredProviderReleaseFailure {
                dispatch: Box::new(dispatch),
                custody: Box::new(custody),
                reason: ResidentProcessHostSelectionError::ProviderOperationAuthority,
            });
        }
        RecoveredProviderDispatchRelease::new(dispatch, custody).map_err(|parts| {
            let (dispatch, custody) = *parts;
            ResidentProcessRecoveredProviderReleaseFailure {
                dispatch: Box::new(dispatch),
                custody: Box::new(custody),
                reason: ResidentProcessHostSelectionError::ProviderTaskAssociation,
            }
        })
    }
}

fn validate_recovered_checkpoint_input(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    dispatch: &PreparedRecoveredOperationDispatch,
    identity: &OperationCheckpointInput,
    now: UnixTimeV1,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let operation = dispatch.operation();
    let (_, handle) = operation.handle();
    let (observation_ref, observation) = dispatch.observation();
    let (prior_ref, prior) = dispatch.checkpoint();
    if !checkpoint_authority_matches(selector, store, authority)
        || identity.reference == *observation_ref
        || identity.reference == *prior_ref
        || identity.reference == *dispatch.selection_ref()
        || identity.reference == *operation.handle().0
        || identity.reference == *handle.operation_lease_ref()
        || identity.reference == *operation.request_coordinate().record_ref()
        || identity.id == *prior.control_checkpoint_id()
        || now < *observation.observed_at()
        || now < *prior.issued_at()
        || now >= *handle.expires_at()
        || now >= dispatch.request_expires_at()
        || now >= operation.historical_operation_lease().expires_at()
    {
        return Err(ResidentProcessHostSelectionError::NativeCheckpointAuthority);
    }
    Ok(())
}

fn observation_matches_selector(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &pale_core::records::OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}
