//! Retain expiry of one recovered Invoke or Delegate before dispatch.
//!
//! This process boundary selects one deployment and verifies the recovered head
//! before expiry can use the custody ports. It never resumes the operation and
//! never calls an action, provider, A2A route, or task implementation.

use super::*;
use crate::resident::operation_dispatch::{
    verify_recovered_operation_head_custody, OperationAppendFailure, OperationCheckpointInput,
};
use crate::resident::operation_expiry::{
    prepare_pre_dispatch_operation_expiry, CompletedPreDispatchOperationExpiry,
    IssuedPreDispatchOperationExpiryCheckpoint, PreDispatchOperationExpiryAnchorPending,
    PreDispatchOperationExpiryCommitFailure, PreDispatchOperationExpiryCustodyFailure,
    PreDispatchOperationExpiryError, PreDispatchOperationExpiryInput,
    PreparedPreDispatchOperationExpiry, PreparedPreDispatchOperationExpiryCheckpoint,
};
use crate::resident::process_host::restart::ResidentProcessRecoveredOperationStart;
use crate::resident::OperationObservationAuthorityStoreResolver;
use pale_ports::AccumulatorPreparationLimits;

/// Fresh identities, signer, clock, and finite bounds for one expiry fact.
pub(super) struct ResidentProcessPreDispatchExpiryInput<'a> {
    pub(super) clock: &'a dyn pale_ports::WallClockSourcePort,
    pub(super) diagnostic_ref: ResidentRecordRefV1,
    pub(super) observation_ref: ResidentRecordRefV1,
    pub(super) signing: pale_core::records::SignedResidentAdmissionContextV1<'a>,
    pub(super) maximum_diagnostic_bytes: usize,
    pub(super) maximum_record_bytes: usize,
}

/// Why expiry preparation stopped before a signed successor existed.
#[derive(Debug)]
pub(super) enum ResidentProcessPreDispatchExpiryFailure {
    Selection(ResidentProcessHostSelectionError),
    Admission(crate::resident::operation_dispatch::OperationAttemptError),
    Expiry(PreDispatchOperationExpiryError),
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Sign one expiry successor for an authenticated recovered head.
    pub(super) fn prepare_recovered_pre_dispatch_expiry<'operation, Rotation, Sign>(
        &self,
        start: &'operation ResidentProcessRecoveredOperationStart,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessPreDispatchExpiryInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<
        PreparedPreDispatchOperationExpiry<'operation>,
        ResidentProcessPreDispatchExpiryFailure,
    >
    where
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
        let selector = &start.selector;
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessPreDispatchExpiryFailure::Selection)?;
        let application = self.deployments[index].application.as_ref();
        let store = application.store();
        if access.store != store {
            return Err(ResidentProcessPreDispatchExpiryFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        validate_expiry_operation(selector, store, start, input.signing)
            .map_err(ResidentProcessPreDispatchExpiryFailure::Selection)?;
        if application.generation(&start.generation_digest).is_none() {
            return Err(ResidentProcessPreDispatchExpiryFailure::Selection(
                ResidentProcessHostSelectionError::PreDispatchExpiryGeneration,
            ));
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        verify_recovered_operation_head_custody(
            start.control.operation().operation(),
            start.control.observation(),
            custody,
            anchor,
            input.maximum_record_bytes,
        )
        .map_err(ResidentProcessPreDispatchExpiryFailure::Admission)?;
        let prepared = prepare_pre_dispatch_operation_expiry(
            &start.control,
            start.lineage.has_parent_operation(),
            PreDispatchOperationExpiryInput {
                clock: input.clock,
                diagnostic_ref: input.diagnostic_ref,
                observation_ref: input.observation_ref,
                signing: input.signing,
                maximum_diagnostic_bytes: input.maximum_diagnostic_bytes,
            },
            admit_rotation,
            sign,
        )
        .map_err(ResidentProcessPreDispatchExpiryFailure::Expiry)?;
        let _ = prepared.diagnostic();
        Ok(prepared)
    }

    /// Compute the exact diagnostic and observation roots for one expiry.
    pub(super) fn prepare_pre_dispatch_expiry_checkpoint<'expiry>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        expiry: &'expiry PreparedPreDispatchOperationExpiry<'expiry>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        limits: AccumulatorPreparationLimits,
    ) -> Result<
        Result<PreparedPreDispatchOperationExpiryCheckpoint<'expiry>, OperationAppendFailure>,
        ResidentProcessHostSelectionError,
    > {
        self.validate_expiry_phase(selector, &access, expiry)?;
        Ok(expiry.prepare_checkpoint(resolve_store, limits, access.custody))
    }

    /// Sign one exact expiry checkpoint. Later phases accept no signer.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn issue_pre_dispatch_expiry_checkpoint<'expiry, Sign>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        prepared: PreparedPreDispatchOperationExpiryCheckpoint<'expiry>,
        identity: OperationCheckpointInput,
        schema: &pale_core::records::SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<
        Result<IssuedPreDispatchOperationExpiryCheckpoint<'expiry>, OperationAppendFailure>,
        ResidentProcessHostSelectionError,
    >
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let expiry = prepared.expiry();
        let store = self.validate_expiry_phase(selector, &access, expiry)?;
        validate_expiry_checkpoint(selector, store, expiry, &identity, authority)?;
        Ok(prepared.issue(identity, schema, authority, sign))
    }

    /// Attach, commit, and anchor one already-signed expiry checkpoint.
    pub(super) fn retain_pre_dispatch_expiry<'expiry>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        issued: IssuedPreDispatchOperationExpiryCheckpoint<'expiry>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        limits: AccumulatorPreparationLimits,
    ) -> Result<
        Result<
            CompletedPreDispatchOperationExpiry<'expiry>,
            PreDispatchOperationExpiryCustodyFailure<'expiry>,
        >,
        ResidentProcessHostSelectionError,
    > {
        self.validate_expiry_phase(selector, &access, issued.expiry())?;
        let _ = issued.checkpoint();
        let retained =
            issued.attach_and_commit(resolve_store, limits, access.custody, access.anchor);
        Ok(match retained {
            Ok(completed) => {
                let _ = (
                    completed.expiry(),
                    completed.checkpoint(),
                    completed.custody(),
                );
                Ok(completed)
            }
            Err(PreDispatchOperationExpiryCustodyFailure::Issued {
                continuation,
                failure,
            }) => Err(PreDispatchOperationExpiryCustodyFailure::Issued {
                continuation,
                failure,
            }),
            Err(PreDispatchOperationExpiryCustodyFailure::Commit(failure)) => {
                Err(PreDispatchOperationExpiryCustodyFailure::Commit(failure))
            }
            Err(PreDispatchOperationExpiryCustodyFailure::Anchor(pending)) => {
                Err(PreDispatchOperationExpiryCustodyFailure::Anchor(pending))
            }
        })
    }

    /// Resolve only the retained database ambiguity for this expiry.
    pub(super) fn reconcile_pre_dispatch_expiry<'expiry>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        failure: PreDispatchOperationExpiryCommitFailure<'expiry>,
    ) -> Result<
        Result<
            PreDispatchOperationExpiryAnchorPending<'expiry>,
            Box<PreDispatchOperationExpiryCommitFailure<'expiry>>,
        >,
        ResidentProcessHostSelectionError,
    > {
        self.validate_expiry_phase(selector, &access, failure.expiry())?;
        Ok(failure.reconcile_exact(access.custody))
    }

    /// Continue only the independent-anchor transition retained for this expiry.
    pub(super) fn continue_pre_dispatch_expiry_anchor<'expiry>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        pending: PreDispatchOperationExpiryAnchorPending<'expiry>,
    ) -> Result<
        Result<
            CompletedPreDispatchOperationExpiry<'expiry>,
            Box<PreDispatchOperationExpiryAnchorPending<'expiry>>,
        >,
        ResidentProcessHostSelectionError,
    > {
        self.validate_expiry_phase(selector, &access, pending.expiry())?;
        Ok(pending.continue_anchor(access.anchor))
    }

    fn validate_expiry_phase<'a>(
        &'a self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: &ResidentDeploymentOperationAccess<'_>,
        expiry: &PreparedPreDispatchOperationExpiry<'_>,
    ) -> Result<&'a AuthorityStoreCoordinate, ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        let store = self.deployments[index].store();
        let operation = expiry.operation();
        if access.store != store
            || !operation.checkpoint_selection().matches_store(store)
            || !selector.matches_operation(operation)
            || !operation_observation_matches(selector, expiry.observation().1)
        {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        Ok(store)
    }
}

fn validate_expiry_operation(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    start: &ResidentProcessRecoveredOperationStart,
    signing: pale_core::records::SignedResidentAdmissionContextV1<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let control = &start.control;
    let operation = control.operation().operation();
    let (handle_ref, handle) = operation.handle();
    let selected = control.observation().selected();
    let head = control.observation().selected_head();
    if control.operation().store() != store
        || control.observation().store() != store
        || !start.lineage.admits(handle_ref, handle)
        || handle.compiled_generation_sha256() != &start.generation_digest
        || !selector.matches_operation(operation)
        || selected.0 != head.0
        || selected.1.canonical_operation_observation_sha256()
            != head.1.canonical_operation_observation_sha256()
        || !matches!(
            head.1.state(),
            pale_core::records::OperationObservationStateV1::Active {
                phase: pale_core::records::OperationActivePhaseV1::Resolving
                    | pale_core::records::OperationActivePhaseV1::ContextBound { .. }
                    | pale_core::records::OperationActivePhaseV1::ProviderResolved { .. }
                    | pale_core::records::OperationActivePhaseV1::DispatchPrepared { .. }
            }
        )
    {
        return Err(ResidentProcessHostSelectionError::PreDispatchExpiryOperationAuthority);
    }
    let historical = operation.historical_session_lease();
    if operation
        .historical_operation_lease()
        .operation_lease_digest()
        != handle.operation_lease_sha256()
        || historical.compiled_generation_digest() != handle.compiled_generation_sha256()
        || historical.activation_fence_digest() != handle.activation_fence_sha256()
        || historical.instance_digest() != handle.instance_sha256()
        || historical.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical.deployment_runtime_authority_sha256().as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::PreDispatchExpiryHistoricalAuthority);
    }
    let request = control.operation().request().to_preimage();
    if !selector.matches_signing_authority(&request.signing_authority_binding)
        || !operation_observation_matches(selector, head.1)
        || !selector.matches_signing_authority(signing.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::PreDispatchExpirySigningAuthority);
    }
    let checkpoint = control.observation().current_checkpoint().1;
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
        || checkpoint.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::PreDispatchExpiryCheckpointAuthority);
    }
    Ok(())
}

fn validate_expiry_checkpoint(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    expiry: &PreparedPreDispatchOperationExpiry<'_>,
    identity: &OperationCheckpointInput,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let operation = expiry.operation();
    let (handle_ref, handle) = operation.handle();
    let (observation_ref, _) = expiry.observation();
    let (prior_ref, prior) = expiry.predecessor_checkpoint();
    if !checkpoint_authority_matches(selector, store, authority)
        || identity.reference == *observation_ref
        || identity.reference == *expiry.diagnostic_ref()
        || identity.reference == *prior_ref
        || identity.reference == *handle_ref
        || identity.reference == *handle.operation_lease_ref()
        || identity.reference == *operation.request_coordinate().record_ref()
        || identity.id == *prior.control_checkpoint_id()
    {
        return Err(ResidentProcessHostSelectionError::PreDispatchExpiryCheckpointAuthority);
    }
    Ok(())
}

fn operation_observation_matches(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &pale_core::records::OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}
