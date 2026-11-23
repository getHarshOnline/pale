//! Complete one retained cancellation request whose operation never crossed dispatch.

use super::*;
use crate::resident::operation_control::OwnedLocalOperationCancellationCompletion;
use crate::resident::operation_dispatch::{
    commit_prepared_checkpoint, observation_checkpoint_preimage,
    verify_recovered_operation_head_custody, OperationAppendFailure, OperationAttemptError,
    OperationCheckpointInput,
};
use crate::resident::process_host::restart::ResidentProcessRecoveredLocalCancellation;
use crate::resident::{
    CompletedOperationObservationCustody, LocalOperationCancellationCompletionInput,
    OperationCancellationCompletionError, OperationCancellationRecoveryError,
    OperationObservationAuthorityStoreResolver,
};
use pale_core::records::{
    CheckpointProfileDigest, CheckpointSigningAuthorityDigest, ControlCheckpointDomainIdV1,
    ControlIncarnationIdV1, ControlStateCheckpointV1, ControlStoreIdV1, OperationActivePhaseV1,
    OperationObservationStateV1, OperationObservationV1, ResidentKeyIdV1, ResidentPrincipalIdV1,
    SigningKeyEpochV1,
};
use pale_ports::{AccumulatorPreparationLimits, AuthorityStoreCoordinate};

/// Fresh terminal coordinates and current authority for one retained local request.
pub(super) struct ResidentProcessLocalCancellationCompletionInput<'a> {
    pub(super) current: &'a super::super::AmbientSessionAdmissionContext<'a>,
    pub(super) maximum_request_bytes: usize,
    pub(super) maximum_record_bytes: usize,
    pub(super) diagnostic_ref: ResidentRecordRefV1,
    pub(super) observation_ref: ResidentRecordRefV1,
    pub(super) established_at: UnixTimeV1,
    pub(super) checkpoint_issued_at: UnixTimeV1,
    pub(super) signing: pale_core::records::SignedResidentAdmissionContextV1<'a>,
    pub(super) maximum_diagnostic_bytes: usize,
    pub(super) resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    pub(super) checkpoint: OperationCheckpointInput,
    pub(super) checkpoint_schema: &'a pale_core::records::SchemaDigest,
    pub(super) checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(super) accumulator_limits: AccumulatorPreparationLimits,
}

/// Completed database and independent-anchor custody for local cancellation.
pub(super) struct CompletedResidentProcessLocalCancellation {
    pub(super) custody: CompletedOperationObservationCustody,
    pub(super) checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    pub(super) observation: Box<(ResidentRecordRefV1, OperationObservationV1)>,
}

/// Exact checkpoint signer coordinate retained after terminal observation signing.
struct OwnedLocalCancellationCheckpointAuthority {
    authority_ref: ResidentRecordRefV1,
    authority_sha256: CheckpointSigningAuthorityDigest,
    principal_id: ResidentPrincipalIdV1,
    control_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
    checkpoint_domain_id: ControlCheckpointDomainIdV1,
    checkpoint_profile_ref: ResidentRecordRefV1,
    checkpoint_profile_sha256: CheckpointProfileDigest,
    subject_key_id: ResidentKeyIdV1,
    subject_key_epoch: SigningKeyEpochV1,
}

impl OwnedLocalCancellationCheckpointAuthority {
    fn retain(authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>) -> Self {
        Self {
            authority_ref: authority.authority_ref.clone(),
            authority_sha256: authority.authority_sha256.clone(),
            principal_id: authority.principal_id.clone(),
            control_store_id: authority.control_store_id.clone(),
            control_incarnation_id: authority.control_incarnation_id.clone(),
            checkpoint_domain_id: authority.checkpoint_domain_id.clone(),
            checkpoint_profile_ref: authority.checkpoint_profile_ref.clone(),
            checkpoint_profile_sha256: authority.checkpoint_profile_sha256.clone(),
            subject_key_id: authority.subject_key_id.clone(),
            subject_key_epoch: authority.subject_key_epoch,
        }
    }

    fn matches(&self, authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>) -> bool {
        authority.authority_ref == &self.authority_ref
            && authority.authority_sha256 == &self.authority_sha256
            && authority.principal_id == &self.principal_id
            && authority.control_store_id == &self.control_store_id
            && authority.control_incarnation_id == &self.control_incarnation_id
            && authority.checkpoint_domain_id == &self.checkpoint_domain_id
            && authority.checkpoint_profile_ref == &self.checkpoint_profile_ref
            && authority.checkpoint_profile_sha256 == &self.checkpoint_profile_sha256
            && authority.subject_key_id == &self.subject_key_id
            && authority.subject_key_epoch == self.subject_key_epoch
    }
}

/// Immutable basis shared by each post-sign cancellation phase.
struct ResidentProcessLocalCancellationBasis {
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    terminal: OwnedLocalOperationCancellationCompletion,
    checkpoint: OperationCheckpointInput,
    checkpoint_issued_at: UnixTimeV1,
    checkpoint_schema: pale_core::records::SchemaDigest,
    checkpoint_authority: OwnedLocalCancellationCheckpointAuthority,
    accumulator_limits: AccumulatorPreparationLimits,
}

impl ResidentProcessLocalCancellationBasis {
    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.terminal.observation()
    }
}

/// Signed terminal cancellation whose checkpoint is not yet signed.
#[must_use = "issue and attach the fixed checkpoint without re-signing the terminal observation"]
pub(super) struct ResidentProcessPreparedLocalCancellation {
    basis: Box<ResidentProcessLocalCancellationBasis>,
    failure: Option<OperationAppendFailure>,
}

impl std::fmt::Debug for ResidentProcessPreparedLocalCancellation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessPreparedLocalCancellation")
            .finish_non_exhaustive()
    }
}

/// Exact signed checkpoint awaiting attachment to the same terminal candidate.
#[must_use = "attach this exact checkpoint without calling a signer"]
pub(super) struct ResidentProcessIssuedLocalCancellationCheckpoint {
    basis: Box<ResidentProcessLocalCancellationBasis>,
    checkpoint: Box<ControlStateCheckpointV1>,
    failure: OperationAppendFailure,
}

impl std::fmt::Debug for ResidentProcessIssuedLocalCancellationCheckpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessIssuedLocalCancellationCheckpoint")
            .finish_non_exhaustive()
    }
}

/// Database or anchor custody pending for one issued terminal append.
#[must_use = "continue only the retained physical custody phase"]
pub(super) struct ResidentProcessLocalCancellationCustodyPending {
    basis: Box<ResidentProcessLocalCancellationBasis>,
    failure: OperationAppendFailure,
}

impl std::fmt::Debug for ResidentProcessLocalCancellationCustodyPending {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessLocalCancellationCustodyPending")
            .finish_non_exhaustive()
    }
}

/// Exact phase retained after terminal observation signing.
#[derive(Debug)]
pub(super) enum ResidentProcessPendingLocalCancellation {
    /// Checkpoint roots or signing did not complete.
    Prepared(ResidentProcessPreparedLocalCancellation),
    /// Checkpoint signing completed, but attachment did not.
    Issued(ResidentProcessIssuedLocalCancellationCheckpoint),
    /// The exact physical candidate stopped at database or anchor custody.
    Custody(ResidentProcessLocalCancellationCustodyPending),
}

impl ResidentProcessPendingLocalCancellation {
    /// Borrow the exact signed terminal observation retained across every phase.
    #[must_use]
    pub(super) fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        match self {
            Self::Prepared(pending) => pending.basis.observation(),
            Self::Issued(pending) => pending.basis.observation(),
            Self::Custody(pending) => pending.basis.observation(),
        }
    }

    /// Borrow the fixed checkpoint reference and identity.
    #[must_use]
    pub(super) fn checkpoint(&self) -> &OperationCheckpointInput {
        match self {
            Self::Prepared(pending) => &pending.basis.checkpoint,
            Self::Issued(pending) => &pending.basis.checkpoint,
            Self::Custody(pending) => &pending.basis.checkpoint,
        }
    }
}

/// Why a retained local cancellation did not reach terminal custody.
pub(super) enum ResidentProcessLocalCancellationCompletionFailure {
    /// Deployment, store, operation, authority, or signer selection differed.
    Selection {
        recovered: Box<ResidentProcessRecoveredLocalCancellation>,
        error: ResidentProcessHostSelectionError,
    },
    /// The retained signed cancellation request did not re-admit exactly.
    Recovery {
        recovered: Box<ResidentProcessRecoveredLocalCancellation>,
        error: OperationCancellationRecoveryError,
    },
    /// Terminal cancellation preparation rejected the exact recovered request.
    Completion {
        recovered: Box<ResidentProcessRecoveredLocalCancellation>,
        error: OperationCancellationCompletionError,
    },
    /// The final head, checkpoint, or independent anchor could not be re-proved.
    Admission {
        recovered: Box<ResidentProcessRecoveredLocalCancellation>,
        error: OperationAttemptError,
    },
    /// Observation signing completed; the exact candidate must continue without re-signing.
    Pending(ResidentProcessPendingLocalCancellation),
}

impl std::fmt::Debug for ResidentProcessLocalCancellationCompletionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Selection { .. } => "Selection",
            Self::Recovery { .. } => "Recovery",
            Self::Completion { .. } => "Completion",
            Self::Admission { .. } => "Admission",
            Self::Pending(_) => "Pending",
        };
        formatter
            .debug_struct("ResidentProcessLocalCancellationCompletionFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Why a signed cancellation token refused one continuation attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentProcessLocalCancellationContinuationRefusal {
    /// The caller selected another deployment authority.
    Selector,
    /// The caller supplied custody ports for another store incarnation.
    AccessStore,
    /// The fresh checkpoint authority differs from the retained coordinate.
    CheckpointAuthority,
}

/// A stopped prepared-phase continuation always returns an exact phase token.
#[derive(Debug)]
pub(super) enum ResidentProcessPreparedLocalCancellationFailure {
    /// Selection failed before projection or physical custody was touched.
    Refused {
        pending: ResidentProcessPreparedLocalCancellation,
        reason: ResidentProcessLocalCancellationContinuationRefusal,
    },
    /// Root projection, checkpoint construction, or signing is still pending.
    Prepared(ResidentProcessPreparedLocalCancellation),
    /// Checkpoint signing completed, but attachment is still pending.
    Issued(ResidentProcessIssuedLocalCancellationCheckpoint),
    /// Database or anchor custody is still pending.
    Custody(ResidentProcessLocalCancellationCustodyPending),
}

/// A stopped issued-phase continuation returns the issued checkpoint unchanged.
#[derive(Debug)]
pub(super) enum ResidentProcessIssuedLocalCancellationFailure {
    /// Selection failed before projection or physical custody was touched.
    Refused {
        pending: ResidentProcessIssuedLocalCancellationCheckpoint,
        reason: ResidentProcessLocalCancellationContinuationRefusal,
    },
    /// Rebuilt roots still cannot attach the already issued checkpoint.
    Issued(ResidentProcessIssuedLocalCancellationCheckpoint),
    /// Database or anchor custody is still pending.
    Custody(ResidentProcessLocalCancellationCustodyPending),
}

/// A stopped physical continuation retains only the existing custody failure.
#[derive(Debug)]
pub(super) enum ResidentProcessLocalCancellationCustodyFailure {
    /// Selection failed before physical custody was touched.
    Refused {
        pending: ResidentProcessLocalCancellationCustodyPending,
        reason: ResidentProcessLocalCancellationContinuationRefusal,
    },
    /// Database or anchor custody is still pending.
    Custody(ResidentProcessLocalCancellationCustodyPending),
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Complete one retained pre-dispatch cancellation without a carrier call.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn complete_recovered_local_cancellation<
        AdmitRequest,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &self,
        recovered: ResidentProcessRecoveredLocalCancellation,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessLocalCancellationCompletionInput<'_>,
        admit_request: AdmitRequest,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        CompletedResidentProcessLocalCancellation,
        ResidentProcessLocalCancellationCompletionFailure,
    >
    where
        AdmitRequest: FnOnce(
            &ResidentRecordRefV1,
            pale_core::records::CommonRequestEnvelopeWireV1,
        ) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let selector = recovered.start.selector.clone();
        let index = match self.deployment_authority_index(&selector) {
            Ok(index) => index,
            Err(error) => {
                return Err(
                    ResidentProcessLocalCancellationCompletionFailure::Selection {
                        recovered: Box::new(recovered),
                        error,
                    },
                );
            }
        };
        let application = self.deployments[index].application.as_ref();
        let selected_store = application.store();
        if access.store != selected_store {
            return Err(
                ResidentProcessLocalCancellationCompletionFailure::Selection {
                    recovered: Box::new(recovered),
                    error: ResidentProcessHostSelectionError::AccessStoreAssociation,
                },
            );
        }
        if application
            .generation(&recovered.start.generation_digest)
            .is_none()
        {
            return Err(
                ResidentProcessLocalCancellationCompletionFailure::Selection {
                    recovered: Box::new(recovered),
                    error: ResidentProcessHostSelectionError::LocalCancellationGeneration,
                },
            );
        }
        if let Err(error) =
            validate_local_cancellation_associations(&selector, selected_store, &recovered, &input)
        {
            return Err(
                ResidentProcessLocalCancellationCompletionFailure::Selection {
                    recovered: Box::new(recovered),
                    error,
                },
            );
        }

        let ResidentDeploymentOperationAccess {
            custody,
            sessions,
            anchor,
            ..
        } = access;
        let cancellation = match recovered.start.control.recover_local_cancellation_request(
            custody,
            input.maximum_request_bytes,
            admit_request,
        ) {
            Ok(cancellation) => cancellation,
            Err(error) => {
                return Err(
                    ResidentProcessLocalCancellationCompletionFailure::Recovery {
                        recovered: Box::new(recovered),
                        error,
                    },
                );
            }
        };
        if !selector.matches_signing_authority(
            &cancellation
                .request()
                .to_preimage()
                .signing_authority_binding,
        ) {
            return Err(
                ResidentProcessLocalCancellationCompletionFailure::Selection {
                    recovered: Box::new(recovered),
                    error: ResidentProcessHostSelectionError::LocalCancellationSigningAuthority,
                },
            );
        }
        if let Err(error) = verify_recovered_operation_head_custody(
            recovered.start.control.operation().operation(),
            recovered.start.control.observation(),
            custody,
            anchor,
            input.maximum_record_bytes,
        ) {
            return Err(
                ResidentProcessLocalCancellationCompletionFailure::Admission {
                    recovered: Box::new(recovered),
                    error,
                },
            );
        }
        let prepared = match cancellation.prepare_completion(
            LocalOperationCancellationCompletionInput {
                diagnostic_ref: input.diagnostic_ref.clone(),
                observation_ref: input.observation_ref.clone(),
                established_at: input.established_at,
                signing: input.signing,
                maximum_diagnostic_bytes: input.maximum_diagnostic_bytes,
                resolve_store: input.resolve_store,
            },
            admit_rotation,
            sign_observation,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                return Err(
                    ResidentProcessLocalCancellationCompletionFailure::Completion {
                        recovered: Box::new(recovered),
                        error,
                    },
                );
            }
        };
        let pending = ResidentProcessPreparedLocalCancellation {
            basis: Box::new(ResidentProcessLocalCancellationBasis {
                selector: selector.clone(),
                store: selected_store.clone(),
                terminal: prepared.into_owned(),
                checkpoint: input.checkpoint,
                checkpoint_issued_at: input.checkpoint_issued_at,
                checkpoint_schema: input.checkpoint_schema.clone(),
                checkpoint_authority: OwnedLocalCancellationCheckpointAuthority::retain(
                    input.checkpoint_authority,
                ),
                accumulator_limits: input.accumulator_limits,
            }),
            failure: None,
        };
        self.continue_prepared_local_cancellation(
            pending,
            &selector,
            ResidentDeploymentOperationAccess {
                store: selected_store,
                custody,
                sessions,
                anchor,
            },
            input.resolve_store,
            input.checkpoint_authority,
            sign_checkpoint,
        )
        .map_err(|failure| match failure {
            ResidentProcessPreparedLocalCancellationFailure::Refused { pending, .. }
            | ResidentProcessPreparedLocalCancellationFailure::Prepared(pending) => {
                ResidentProcessLocalCancellationCompletionFailure::Pending(
                    ResidentProcessPendingLocalCancellation::Prepared(pending),
                )
            }
            ResidentProcessPreparedLocalCancellationFailure::Issued(pending) => {
                ResidentProcessLocalCancellationCompletionFailure::Pending(
                    ResidentProcessPendingLocalCancellation::Issued(pending),
                )
            }
            ResidentProcessPreparedLocalCancellationFailure::Custody(pending) => {
                ResidentProcessLocalCancellationCompletionFailure::Pending(
                    ResidentProcessPendingLocalCancellation::Custody(pending),
                )
            }
        })
    }

    /// Issue one checkpoint for an already signed terminal cancellation.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn continue_prepared_local_cancellation<CheckpointSign>(
        &self,
        pending: ResidentProcessPreparedLocalCancellation,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        CompletedResidentProcessLocalCancellation,
        ResidentProcessPreparedLocalCancellationFailure,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_prepared_local_cancellation_with_attachment(
            pending,
            selector,
            access,
            resolve_store,
            checkpoint_authority,
            sign_checkpoint,
            |prepared, reference, checkpoint| {
                prepared.attach_issued_checkpoint(reference, checkpoint)
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn continue_prepared_local_cancellation_with_attachment<CheckpointSign, Attach>(
        &self,
        pending: ResidentProcessPreparedLocalCancellation,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign_checkpoint: CheckpointSign,
        attach: Attach,
    ) -> Result<
        CompletedResidentProcessLocalCancellation,
        ResidentProcessPreparedLocalCancellationFailure,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Attach: for<'prepared> FnOnce(
            crate::resident::PreparedOperationObservationCheckpoint<'prepared>,
            &ResidentRecordRefV1,
            &ControlStateCheckpointV1,
        ) -> Result<
            crate::resident::PreparedOperationObservationCustody,
            crate::resident::OperationObservationCustodyProjectionError,
        >,
    {
        if let Err(reason) = self.validate_local_cancellation_continuation(
            selector,
            access.store,
            &pending.basis.selector,
            &pending.basis.store,
        ) {
            return Err(ResidentProcessPreparedLocalCancellationFailure::Refused {
                pending,
                reason,
            });
        }
        if !pending
            .basis
            .checkpoint_authority
            .matches(checkpoint_authority)
        {
            return Err(ResidentProcessPreparedLocalCancellationFailure::Refused {
                pending,
                reason: ResidentProcessLocalCancellationContinuationRefusal::CheckpointAuthority,
            });
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        issue_prepared_local_cancellation_with_attachment(
            pending,
            resolve_store,
            custody,
            anchor,
            checkpoint_authority,
            sign_checkpoint,
            attach,
        )
    }

    /// Attach one already issued checkpoint and continue its physical custody.
    pub(super) fn continue_issued_local_cancellation(
        &self,
        pending: ResidentProcessIssuedLocalCancellationCheckpoint,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<
        CompletedResidentProcessLocalCancellation,
        ResidentProcessIssuedLocalCancellationFailure,
    > {
        if let Err(reason) = self.validate_local_cancellation_continuation(
            selector,
            access.store,
            &pending.basis.selector,
            &pending.basis.store,
        ) {
            return Err(ResidentProcessIssuedLocalCancellationFailure::Refused { pending, reason });
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        continue_issued_local_cancellation(pending, resolve_store, custody, anchor)
    }

    /// Continue only database or independent-anchor custody for one exact append.
    pub(super) fn continue_local_cancellation_custody(
        &self,
        pending: ResidentProcessLocalCancellationCustodyPending,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<
        CompletedResidentProcessLocalCancellation,
        ResidentProcessLocalCancellationCustodyFailure,
    > {
        if let Err(reason) = self.validate_local_cancellation_continuation(
            selector,
            access.store,
            &pending.basis.selector,
            &pending.basis.store,
        ) {
            return Err(ResidentProcessLocalCancellationCustodyFailure::Refused {
                pending,
                reason,
            });
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let ResidentProcessLocalCancellationCustodyPending { basis, failure } = pending;
        match failure.continue_custody(custody, anchor) {
            Ok((completed, checkpoint)) => {
                Ok(completed_local_cancellation(basis, completed, checkpoint))
            }
            Err(failure) => Err(ResidentProcessLocalCancellationCustodyFailure::Custody(
                ResidentProcessLocalCancellationCustodyPending { basis, failure },
            )),
        }
    }

    fn validate_local_cancellation_continuation(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access_store: &AuthorityStoreCoordinate,
        expected_selector: &ResidentDeploymentAuthoritySelector,
        expected_store: &AuthorityStoreCoordinate,
    ) -> Result<(), ResidentProcessLocalCancellationContinuationRefusal> {
        let index = self
            .deployment_authority_index(selector)
            .map_err(|_| ResidentProcessLocalCancellationContinuationRefusal::Selector)?;
        if selector != expected_selector
            || self.deployments[index].application.store() != expected_store
        {
            return Err(ResidentProcessLocalCancellationContinuationRefusal::Selector);
        }
        if access_store != expected_store {
            return Err(ResidentProcessLocalCancellationContinuationRefusal::AccessStore);
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn issue_prepared_local_cancellation_with_attachment<CheckpointSign, Attach>(
    mut pending: ResidentProcessPreparedLocalCancellation,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    sign_checkpoint: CheckpointSign,
    attach: Attach,
) -> Result<
    CompletedResidentProcessLocalCancellation,
    ResidentProcessPreparedLocalCancellationFailure,
>
where
    CheckpointSign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    Attach: for<'prepared> FnOnce(
        crate::resident::PreparedOperationObservationCheckpoint<'prepared>,
        &ResidentRecordRefV1,
        &ControlStateCheckpointV1,
    ) -> Result<
        crate::resident::PreparedOperationObservationCustody,
        crate::resident::OperationObservationCustodyProjectionError,
    >,
{
    let prepared = match pending.basis.terminal.prepare_checkpoint(
        pending.basis.accumulator_limits,
        resolve_store,
        custody,
    ) {
        Ok(prepared) => prepared,
        Err(error) => {
            pending.failure = Some(OperationAppendFailure::Projection(error));
            return Err(ResidentProcessPreparedLocalCancellationFailure::Prepared(
                pending,
            ));
        }
    };
    let preimage = match observation_checkpoint_preimage(
        &prepared,
        pending.basis.terminal.predecessor_checkpoint(),
        &pending.basis.checkpoint,
        pending.basis.checkpoint_issued_at,
        &pending.basis.checkpoint_schema,
        checkpoint_authority,
    ) {
        Ok(preimage) => preimage,
        Err(error) => {
            pending.failure = Some(OperationAppendFailure::Operation(error));
            return Err(ResidentProcessPreparedLocalCancellationFailure::Prepared(
                pending,
            ));
        }
    };
    let checkpoint = match prepared.issue_checkpoint_value(
        preimage,
        &pending.basis.checkpoint_schema,
        checkpoint_authority,
        sign_checkpoint,
    ) {
        Ok(checkpoint) => checkpoint,
        Err(error) => {
            pending.failure = Some(OperationAppendFailure::Projection(error));
            return Err(ResidentProcessPreparedLocalCancellationFailure::Prepared(
                pending,
            ));
        }
    };
    let physical = match attach(prepared, &pending.basis.checkpoint.reference, &checkpoint) {
        Ok(physical) => physical,
        Err(error) => {
            return Err(ResidentProcessPreparedLocalCancellationFailure::Issued(
                ResidentProcessIssuedLocalCancellationCheckpoint {
                    basis: pending.basis,
                    checkpoint: Box::new(checkpoint),
                    failure: OperationAppendFailure::Projection(error),
                },
            ));
        }
    };
    finish_local_cancellation_physical(pending.basis, checkpoint, physical, custody, anchor)
        .map_err(ResidentProcessPreparedLocalCancellationFailure::Custody)
}

fn continue_issued_local_cancellation(
    mut pending: ResidentProcessIssuedLocalCancellationCheckpoint,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) -> Result<CompletedResidentProcessLocalCancellation, ResidentProcessIssuedLocalCancellationFailure>
{
    let prepared = match pending.basis.terminal.prepare_checkpoint(
        pending.basis.accumulator_limits,
        resolve_store,
        custody,
    ) {
        Ok(prepared) => prepared,
        Err(error) => {
            pending.failure = OperationAppendFailure::Projection(error);
            return Err(ResidentProcessIssuedLocalCancellationFailure::Issued(
                pending,
            ));
        }
    };
    let physical = match prepared
        .attach_issued_checkpoint(&pending.basis.checkpoint.reference, &pending.checkpoint)
    {
        Ok(physical) => physical,
        Err(error) => {
            pending.failure = OperationAppendFailure::Projection(error);
            return Err(ResidentProcessIssuedLocalCancellationFailure::Issued(
                pending,
            ));
        }
    };
    finish_local_cancellation_physical(
        pending.basis,
        *pending.checkpoint,
        physical,
        custody,
        anchor,
    )
    .map_err(ResidentProcessIssuedLocalCancellationFailure::Custody)
}

fn finish_local_cancellation_physical(
    basis: Box<ResidentProcessLocalCancellationBasis>,
    checkpoint: ControlStateCheckpointV1,
    physical: crate::resident::PreparedOperationObservationCustody,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) -> Result<CompletedResidentProcessLocalCancellation, ResidentProcessLocalCancellationCustodyPending>
{
    let checkpoint = (basis.checkpoint.reference.clone(), checkpoint);
    match commit_prepared_checkpoint(physical, checkpoint, custody, anchor) {
        Ok((completed, checkpoint)) => {
            Ok(completed_local_cancellation(basis, completed, checkpoint))
        }
        Err(failure) => Err(ResidentProcessLocalCancellationCustodyPending { basis, failure }),
    }
}

fn completed_local_cancellation(
    basis: Box<ResidentProcessLocalCancellationBasis>,
    custody: CompletedOperationObservationCustody,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
) -> CompletedResidentProcessLocalCancellation {
    let observation = (
        basis.terminal.observation().0.clone(),
        basis.terminal.observation().1.clone(),
    );
    CompletedResidentProcessLocalCancellation {
        custody,
        checkpoint: Box::new(checkpoint),
        observation: Box::new(observation),
    }
}

fn validate_local_cancellation_associations(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &pale_ports::AuthorityStoreCoordinate,
    recovered: &ResidentProcessRecoveredLocalCancellation,
    input: &ResidentProcessLocalCancellationCompletionInput<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let control = &recovered.start.control;
    let operation = control.operation();
    let issued = operation.operation();
    let (handle_ref, handle) = issued.handle();
    if operation.store() != store
        || control.observation().store() != store
        || !recovered.start.lineage.admits(handle_ref, handle)
        || handle.compiled_generation_sha256() != &recovered.start.generation_digest
        || !selector.matches_operation(issued)
    {
        return Err(ResidentProcessHostSelectionError::LocalCancellationOperationAuthority);
    }
    if !selector.matches_current_session(input.current)
        || !selector.matches_signing_authority(input.current.generation.signed.signing_authority)
        || input
            .current
            .generation
            .compiled_generation
            .compiled_generation_digest()
            != &recovered.start.generation_digest
        || input
            .current
            .generation
            .active_head
            .compiled_generation_digest()
            != &recovered.start.generation_digest
        || input
            .current
            .generation
            .activation_fence
            .activation_fence_digest()
            != handle.activation_fence_sha256()
    {
        return Err(ResidentProcessHostSelectionError::LocalCancellationCurrentAuthority);
    }
    let historical_session = issued.historical_session_lease();
    if issued.historical_operation_lease().operation_lease_digest()
        != handle.operation_lease_sha256()
        || historical_session.compiled_generation_digest() != handle.compiled_generation_sha256()
        || historical_session.activation_fence_digest() != handle.activation_fence_sha256()
        || historical_session.instance_digest() != handle.instance_sha256()
        || historical_session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical_session
            .deployment_runtime_authority_sha256()
            .as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical_session.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::LocalCancellationHistoricalAuthority);
    }
    let original_request = operation.request().to_preimage();
    let observation = control.observation();
    let head = observation.selected_head();
    let predecessor = observation
        .selected_predecessor()
        .ok_or(ResidentProcessHostSelectionError::LocalCancellationHistory)?;
    if !matches!(
        head.1.state(),
        OperationObservationStateV1::CancellationRequested { request }
            if request == &recovered.request
    ) || !matches!(
        predecessor.1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving
                | OperationActivePhaseV1::ContextBound { .. }
                | OperationActivePhaseV1::ProviderResolved { .. }
                | OperationActivePhaseV1::DispatchPrepared { .. }
        }
    ) || head.1.predecessor_ref().as_ref() != Some(predecessor.0)
        || head.1.predecessor_sha256().as_ref()
            != Some(predecessor.1.canonical_operation_observation_sha256())
    {
        return Err(ResidentProcessHostSelectionError::LocalCancellationHistory);
    }
    if !selector.matches_signing_authority(&original_request.signing_authority_binding)
        || !operation_observation_matches(selector, head.1)
        || !operation_observation_matches(selector, predecessor.1)
        || !selector.matches_signing_authority(input.signing.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::LocalCancellationSigningAuthority);
    }
    let checkpoint = observation.current_checkpoint().1;
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
        || checkpoint.issuer_principal_id() != &selector.principal_id
        || !checkpoint_authority_matches(selector, store, input.checkpoint_authority)
    {
        return Err(ResidentProcessHostSelectionError::LocalCancellationCheckpointAuthority);
    }
    if input.maximum_request_bytes == 0
        || input.maximum_record_bytes == 0
        || input.maximum_diagnostic_bytes == 0
    {
        return Err(ResidentProcessHostSelectionError::LocalCancellationBounds);
    }
    Ok(())
}

fn operation_observation_matches(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/cancellation_recovery_support.rs"]
mod test_support;
