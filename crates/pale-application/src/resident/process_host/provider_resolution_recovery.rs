//! Retain the first attempt after a recovered provider resolution.

use super::*;
use crate::resident::operation_dispatch::{
    prepare_recovered_provider_attempt, retain_recovered_provider_attempt,
    CompletedOperationContextBinding, CompletedRecoveredProviderAttempt, OperationAttemptError,
    PreparedRecoveredProviderAttempt, RecoveredProviderAttemptAnchorPending,
    RecoveredProviderAttemptCommitFailure, RecoveredProviderAttemptCustodyFailure,
    RecoveredProviderAttemptInput, RecoveredProviderAttemptMeaning,
};
use crate::resident::process_host::restart::ResidentProcessRecoveredProviderResolution;
use pale_core::records::{
    CapabilityAttemptIdV1, ControlCheckpointIdV1, OperationActivePhaseV1,
    OperationObservationStateV1, OperationObservationV1, SchemaDigest,
    SignedResidentAdmissionContextV1,
};
use pale_ports::CryptographicRandomSourceError;
use std::collections::BTreeSet;

const RANDOM_BYTES: usize = 32;
const MAXIMUM_RANDOM_DRAWS: usize = 10;
const ATTEMPT_ID_DOMAIN: &[u8] = b"pale:provider-attempt:attempt-id:v1\0";
const SELECTION_REF_DOMAIN: &[u8] = b"pale:provider-attempt:selection-record-ref:v1\0";
const PREPARATION_REF_DOMAIN: &[u8] = b"pale:provider-attempt:preparation-record-ref:v1\0";
const CHECKPOINT_ID_DOMAIN: &[u8] = b"pale:provider-attempt:checkpoint-id:v1\0";
const CHECKPOINT_REF_DOMAIN: &[u8] = b"pale:provider-attempt:checkpoint-record-ref:v1\0";

/// Current authority and non-identity result meaning for one recovered attempt.
#[derive(Clone, Copy)]
pub(super) struct ResidentProcessProviderResolvedInput<'a> {
    pub(super) session: &'a super::super::AdmittedAmbientSession,
    pub(super) current: &'a super::super::AmbientSessionAdmissionContext<'a>,
    pub(super) meaning: RecoveredProviderAttemptMeaning<'a>,
    pub(super) signing: SignedResidentAdmissionContextV1<'a>,
    pub(super) checkpoint_schema: &'a SchemaDigest,
    pub(super) checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(super) resolve_store: &'a dyn super::super::OperationObservationAuthorityStoreResolver,
    pub(super) accumulator_limits: pale_ports::AccumulatorPreparationLimits,
    pub(super) maximum_selection_bytes: usize,
    pub(super) maximum_record_bytes: usize,
}

/// Fresh single-use identities for one first provider attempt.
#[must_use = "use these exact identities or preserve them for the same recovery"]
pub(super) struct ResidentProcessProviderAttemptAllocation {
    attempt_id: CapabilityAttemptIdV1,
    selection_ref: ResidentRecordRefV1,
    preparation_ref: ResidentRecordRefV1,
    checkpoint: crate::resident::operation_dispatch::OperationCheckpointInput,
}

/// One signed successor and the only checkpoint identity allocated for it.
#[must_use = "retain this exact preparation or preserve it for the same recovery"]
pub(super) struct ResidentProcessPreparedProviderResolution {
    attempt: PreparedRecoveredProviderAttempt,
    checkpoint: crate::resident::operation_dispatch::OperationCheckpointInput,
}

impl ResidentProcessPreparedProviderResolution {
    pub(super) fn into_parts(
        self,
    ) -> (
        PreparedRecoveredProviderAttempt,
        crate::resident::operation_dispatch::OperationCheckpointInput,
    ) {
        (self.attempt, self.checkpoint)
    }
}

/// Recovered preparation stopped before a signed successor existed.
#[derive(Debug)]
pub(super) enum ResidentProcessProviderResolvedPreparationError {
    Selection(ResidentProcessHostSelectionError),
    Random(CryptographicRandomSourceError),
    RandomDrawsExhausted,
    Identity(InvalidResidentRecordV1),
    Admission(OperationAttemptError),
}

/// Retention stopped after `DispatchPrepared` was signed exactly once.
pub(super) enum ResidentProcessProviderResolvedRetentionFailure {
    Selection {
        preparation: Box<ResidentProcessPreparedProviderResolution>,
        reason: ResidentProcessHostSelectionError,
    },
    Custody(RecoveredProviderAttemptCustodyFailure),
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Prove the current recovery joins, then allocate one first-attempt bundle.
    pub(super) fn allocate_provider_resolved_attempt<Authority>(
        &self,
        recovered: &ResidentProcessRecoveredProviderResolution,
        access: ResidentDeploymentOperationAccess<'_>,
        input: &ResidentProcessProviderResolvedInput<'_>,
        authority: &mut Authority,
        random: &dyn CryptographicRandomSourcePort,
    ) -> Result<
        ResidentProcessProviderAttemptAllocation,
        ResidentProcessProviderResolvedPreparationError,
    >
    where
        Authority: super::super::AmbientSessionAuthorityPort,
    {
        self.validate_provider_resolved(recovered, &access, input)
            .map_err(ResidentProcessProviderResolvedPreparationError::Selection)?;
        input
            .session
            .revalidate(input.current, authority)
            .map_err(|_| {
                ResidentProcessProviderResolvedPreparationError::Selection(
                    ResidentProcessHostSelectionError::ProviderCurrentAuthority,
                )
            })?;
        let operation = recovered.start.control.operation().operation();
        crate::resident::operation_dispatch::verify_recovered_operation_head_custody(
            operation,
            recovered.start.control.observation(),
            access.custody,
            access.anchor,
            input.maximum_record_bytes,
        )
        .map_err(|_| {
            ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderOperationAuthority,
            )
        })?;
        allocate_provider_attempt(random)
    }

    /// Rebuild and sign `DispatchPrepared` with a previously allocated bundle.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_provider_resolved_attempt<Authority, Rotation, Sign>(
        &self,
        recovered: &ResidentProcessRecoveredProviderResolution,
        input: &ResidentProcessProviderResolvedInput<'_>,
        allocation: ResidentProcessProviderAttemptAllocation,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<
        ResidentProcessPreparedProviderResolution,
        ResidentProcessProviderResolvedPreparationError,
    >
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
        let index = self
            .deployment_authority_index(&recovered.start.selector)
            .map_err(ResidentProcessProviderResolvedPreparationError::Selection)?;
        let application = self.deployments[index].application.as_ref();
        let generation = application
            .generation(&recovered.start.generation_digest)
            .ok_or(ResidentProcessProviderResolvedPreparationError::Selection(
                ResidentProcessHostSelectionError::ProviderOperationAuthority,
            ))?;
        let control = &recovered.start.control;
        let operation = control.operation();
        let ResidentProcessProviderAttemptAllocation {
            attempt_id,
            selection_ref,
            preparation_ref,
            checkpoint,
        } = allocation;
        let attempt = prepare_recovered_provider_attempt(
            RecoveredProviderAttemptInput {
                operation: operation.operation(),
                request: operation.request(),
                session: input.session,
                lease: operation.operation().historical_operation_lease(),
                generation,
                recovered: control.observation(),
                current: input.current,
                provider_binding: &recovered.provider_binding,
                attempt_id: &attempt_id,
                selection_ref,
                observation_ref: preparation_ref,
                maximum_selection_bytes: input.maximum_selection_bytes,
                signing: input.signing,
                meaning: input.meaning,
            },
            authority,
            admit_rotation,
            sign,
        )
        .map_err(ResidentProcessProviderResolvedPreparationError::Admission)?;
        Ok(ResidentProcessPreparedProviderResolution {
            attempt,
            checkpoint,
        })
    }

    /// Retain the signed attempt without calling a provider or carrier.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn retain_provider_resolved_attempt<Sign>(
        &self,
        recovered: &ResidentProcessRecoveredProviderResolution,
        access: ResidentDeploymentOperationAccess<'_>,
        input: &ResidentProcessProviderResolvedInput<'_>,
        preparation: ResidentProcessPreparedProviderResolution,
        sign_checkpoint: Sign,
    ) -> Result<CompletedRecoveredProviderAttempt, ResidentProcessProviderResolvedRetentionFailure>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if let Err(reason) = self.validate_provider_resolved(recovered, &access, input) {
            return Err(ResidentProcessProviderResolvedRetentionFailure::Selection {
                preparation: Box::new(preparation),
                reason,
            });
        }
        let ResidentProcessPreparedProviderResolution {
            attempt,
            checkpoint,
        } = preparation;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        retain_recovered_provider_attempt(
            attempt,
            crate::resident::operation_dispatch::OperationCheckpointInput {
                reference: checkpoint.reference,
                id: checkpoint.id,
            },
            input.current.now,
            input.checkpoint_schema,
            input.checkpoint_authority,
            input.resolve_store,
            input.accumulator_limits,
            custody,
            anchor,
            sign_checkpoint,
        )
        .map_err(ResidentProcessProviderResolvedRetentionFailure::Custody)
    }

    /// Retry custody for the exact signed provider attempt and checkpoint identity.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn continue_prepared_provider_resolved_attempt<Sign>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        attempt: PreparedRecoveredProviderAttempt,
        checkpoint: crate::resident::operation_dispatch::OperationCheckpointInput,
        now: UnixTimeV1,
        checkpoint_schema: &SchemaDigest,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        accumulator_limits: pale_ports::AccumulatorPreparationLimits,
        sign_checkpoint: Sign,
    ) -> Result<
        Result<CompletedRecoveredProviderAttempt, RecoveredProviderAttemptCustodyFailure>,
        (
            Box<PreparedRecoveredProviderAttempt>,
            crate::resident::operation_dispatch::OperationCheckpointInput,
            ResidentProcessHostSelectionError,
        ),
    >
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let index = match self.deployment_authority_index(selector) {
            Ok(index) => index,
            Err(reason) => return Err((Box::new(attempt), checkpoint, reason)),
        };
        if self.deployments[index].store() != access.store {
            return Err((
                Box::new(attempt),
                checkpoint,
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        Ok(retain_recovered_provider_attempt(
            attempt,
            checkpoint,
            now,
            checkpoint_schema,
            checkpoint_authority,
            resolve_store,
            accumulator_limits,
            custody,
            anchor,
            sign_checkpoint,
        ))
    }

    /// Resolve only the exact ambiguous provider-attempt database candidate.
    pub(super) fn reconcile_provider_resolved_attempt(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        failure: RecoveredProviderAttemptCommitFailure,
    ) -> Result<
        Result<RecoveredProviderAttemptAnchorPending, Box<RecoveredProviderAttemptCommitFailure>>,
        (
            Box<RecoveredProviderAttemptCommitFailure>,
            ResidentProcessHostSelectionError,
        ),
    > {
        let index = match self.deployment_authority_index(selector) {
            Ok(index) => index,
            Err(reason) => return Err((Box::new(failure), reason)),
        };
        if self.deployments[index].store() != access.store {
            return Err((
                Box::new(failure),
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        Ok(failure.reconcile_exact(access.custody))
    }

    /// Continue only the exact retained provider-attempt anchor transition.
    pub(super) fn continue_provider_resolved_attempt_anchor(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        pending: RecoveredProviderAttemptAnchorPending,
    ) -> Result<
        Result<CompletedRecoveredProviderAttempt, Box<RecoveredProviderAttemptAnchorPending>>,
        (
            Box<RecoveredProviderAttemptAnchorPending>,
            ResidentProcessHostSelectionError,
        ),
    > {
        let index = match self.deployment_authority_index(selector) {
            Ok(index) => index,
            Err(reason) => return Err((Box::new(pending), reason)),
        };
        if self.deployments[index].store() != access.store {
            return Err((
                Box::new(pending),
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        Ok(pending.continue_anchor(access.anchor))
    }

    fn validate_provider_resolved(
        &self,
        recovered: &ResidentProcessRecoveredProviderResolution,
        access: &ResidentDeploymentOperationAccess<'_>,
        input: &ResidentProcessProviderResolvedInput<'_>,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        let selector = &recovered.start.selector;
        let index = self.deployment_authority_index(selector)?;
        let application = self.deployments[index].application.as_ref();
        let store = application.store();
        if access.store != store
            || recovered.start.control.operation().store() != store
            || recovered.start.control.observation().store() != store
        {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        let operation = recovered.start.control.operation().operation();
        let (_, handle) = operation.handle();
        let historical_session = operation.historical_session_lease();
        let historical_lease = operation.historical_operation_lease();
        let head = recovered.start.control.observation().selected_head();
        let predecessor = recovered
            .start
            .control
            .observation()
            .selected_predecessor()
            .ok_or(ResidentProcessHostSelectionError::ProviderOperationAuthority)?;
        let checkpoint = recovered.start.control.observation().current_checkpoint().1;
        if !recovered.start.lineage.admits(operation.handle().0, handle)
            || !recovered
                .start
                .lineage
                .admits_provider(Some(&recovered.provider_binding))
            || !selector.matches_operation(operation)
            || handle.compiled_generation_sha256() != &recovered.start.generation_digest
            || historical_session != input.session.lease()
            || historical_session.deployment_runtime_authority_ref()
                != &selector.runtime_authority_ref
            || historical_session
                .deployment_runtime_authority_sha256()
                .as_str()
                != selector.runtime_authority_sha256.as_str()
            || historical_session.issuer_principal_id() != &selector.principal_id
            || historical_lease.operation_lease_digest() != handle.operation_lease_sha256()
            || !selector.matches_current_session(input.current)
            || !selector
                .matches_signing_authority(input.current.generation.signed.signing_authority)
            || !selector.matches_signing_authority(
                &recovered
                    .start
                    .control
                    .operation()
                    .request()
                    .to_preimage()
                    .signing_authority_binding,
            )
            || !selector.matches_signing_authority(input.signing.signing_authority)
            || !selector
                .matches_signing_authority(input.meaning.admission.result_signing_authority())
            || !selector
                .matches_signing_authority(input.meaning.admission.failure_signing_authority())
            || !checkpoint_authority_matches(selector, store, input.checkpoint_authority)
            || checkpoint.control_store_id() != store.store_id()
            || checkpoint.control_incarnation_id() != store.incarnation_id()
            || checkpoint.issuer_principal_id() != &selector.principal_id
            || input.checkpoint_authority.authority_ref
                != checkpoint.checkpoint_signing_authority_ref()
            || input.checkpoint_authority.authority_sha256
                != checkpoint.checkpoint_signing_authority_sha256()
            || input.checkpoint_authority.checkpoint_domain_id != checkpoint.checkpoint_domain_id()
            || input.checkpoint_authority.checkpoint_profile_ref
                != checkpoint.checkpoint_profile_ref()
            || input.checkpoint_authority.checkpoint_profile_sha256
                != checkpoint.checkpoint_profile_sha256()
            || input.checkpoint_authority.subject_key_id != checkpoint.issuer_key_id()
            || input.checkpoint_authority.subject_key_epoch != *checkpoint.issuer_key_epoch()
            || input.current.now < *head.1.observed_at()
            || input.current.now < *checkpoint.issued_at()
            || input.current.now >= *handle.expires_at()
            || input.current.now >= recovered.start.control.operation().request().expires_at()
            || input.current.now >= historical_lease.expires_at()
            || input.maximum_selection_bytes == 0
            || input.maximum_record_bytes == 0
        {
            return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
        }
        if !observation_matches_provider_selector(selector, head.1)
            || !observation_matches_provider_selector(selector, predecessor.1)
        {
            return Err(ResidentProcessHostSelectionError::ProviderObservationAuthority);
        }
        let OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::ProviderResolved { provider_binding },
        } = head.1.state()
        else {
            return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
        };
        if provider_binding != &recovered.provider_binding {
            return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
        }
        if historical_lease.evidence_obligations().is_empty() {
            if !matches!(
                predecessor.1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                }
            ) {
                return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
            }
        } else {
            let binding = CompletedOperationContextBinding::recover(
                operation,
                recovered.start.control.observation(),
            )
            .map_err(|_| ResidentProcessHostSelectionError::ProviderOperationAuthority)?;
            if predecessor != binding.observation() {
                return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
            }
        }
        Ok(())
    }
}

fn observation_matches_provider_selector(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}

fn allocate_provider_attempt(
    random: &dyn CryptographicRandomSourcePort,
) -> Result<ResidentProcessProviderAttemptAllocation, ResidentProcessProviderResolvedPreparationError>
{
    let mut nonces = ProviderAttemptNoncePool::default();
    Ok(ResidentProcessProviderAttemptAllocation {
        attempt_id: CapabilityAttemptIdV1::parse(nonces.allocate(
            random,
            ATTEMPT_ID_DOMAIN,
            "provider-attempt:v1",
        )?)
        .map_err(ResidentProcessProviderResolvedPreparationError::Identity)?,
        selection_ref: ResidentRecordRefV1::parse(nonces.allocate(
            random,
            SELECTION_REF_DOMAIN,
            "provider-attempt-selection-record:v1",
        )?)
        .map_err(ResidentProcessProviderResolvedPreparationError::Identity)?,
        preparation_ref: ResidentRecordRefV1::parse(nonces.allocate(
            random,
            PREPARATION_REF_DOMAIN,
            "operation-observation-record:v1",
        )?)
        .map_err(ResidentProcessProviderResolvedPreparationError::Identity)?,
        checkpoint: crate::resident::operation_dispatch::OperationCheckpointInput {
            id: ControlCheckpointIdV1::parse(nonces.allocate(
                random,
                CHECKPOINT_ID_DOMAIN,
                "operation-checkpoint:v1",
            )?)
            .map_err(ResidentProcessProviderResolvedPreparationError::Identity)?,
            reference: ResidentRecordRefV1::parse(nonces.allocate(
                random,
                CHECKPOINT_REF_DOMAIN,
                "operation-checkpoint-record:v1",
            )?)
            .map_err(ResidentProcessProviderResolvedPreparationError::Identity)?,
        },
    })
}

#[derive(Default)]
struct ProviderAttemptNoncePool {
    seen: BTreeSet<[u8; RANDOM_BYTES]>,
    draws: usize,
}

impl ProviderAttemptNoncePool {
    fn allocate(
        &mut self,
        random: &dyn CryptographicRandomSourcePort,
        domain: &[u8],
        prefix: &str,
    ) -> Result<String, ResidentProcessProviderResolvedPreparationError> {
        loop {
            if self.draws == MAXIMUM_RANDOM_DRAWS {
                return Err(ResidentProcessProviderResolvedPreparationError::RandomDrawsExhausted);
            }
            self.draws += 1;
            let mut nonce = [0_u8; RANDOM_BYTES];
            random
                .fill_bytes(&mut nonce)
                .map_err(ResidentProcessProviderResolvedPreparationError::Random)?;
            if !self.seen.insert(nonce) {
                continue;
            }
            let mut preimage = Vec::with_capacity(domain.len() + nonce.len());
            preimage.extend_from_slice(domain);
            preimage.extend_from_slice(&nonce);
            return Ok(format!("{prefix}:{}", pale_core::sha256_hex(&preimage)));
        }
    }
}
