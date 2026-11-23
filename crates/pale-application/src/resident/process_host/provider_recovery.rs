//! Retain provider uncertainty after a protected cold restart.

use super::*;
use crate::resident::local_operation_coordinator::ResidentLocalOperationCoordinator;
use crate::resident::operation_dispatch::{
    continue_issued_recovered_provider_unknown, continue_prepared_recovered_provider_unknown,
    IssuedRecoveredProviderUnknownCheckpoint, OperationAppendFailure, OperationAttemptError,
    OperationAttemptHistoryInput, PreparedRecoveredProviderUnknown, ProviderOperationInvokeInput,
    ProviderOperationInvokeResult, ProviderOperationRecordingCustody,
    ProviderOperationRecordingFinalization, RecoveredProviderUnknownRecordingFailure,
};
use crate::resident::process_host::restart::{
    ResidentProcessAttemptObservation, ResidentProcessRecoveredAttempt,
};
use pale_core::records::{OperationActivePhaseV1, OperationObservationStateV1};

/// Existing provider dispatch and fresh recording inputs for one cold restart.
pub(super) struct ResidentProcessProviderRecoveryInput<'operation, 'call> {
    pub(super) attempt: &'operation ResidentProcessRecoveredAttempt,
    pub(super) recording: ProviderOperationInvokeInput<'call>,
    pub(super) maximum_selection_bytes: usize,
}

/// Cold provider recovery stopped before exact uncertainty custody completed.
#[derive(Debug)]
pub(super) enum ResidentProcessProviderRecoveryFailure {
    /// Process deployment or authority selection differed before port use.
    Selection(ResidentProcessHostSelectionError),
    /// No physical candidate exists. Protected recovery may be repeated.
    Preparation(Box<OperationAppendFailure>),
    /// Observation signing completed; the exact checkpoint preparation is retained.
    Prepared(Box<ResidentProcessProviderRecoveryPrepared>),
    /// Checkpoint signing completed; the exact value awaits attachment.
    Issued(Box<ResidentProcessProviderRecoveryIssued>),
    /// One exact physical candidate owns all further custody work.
    Custody(Box<ResidentProcessProviderRecoveryCustody>),
    /// Physical custody completed, but semantic association remains unproved.
    Finalization(Box<ResidentProcessProviderRecoveryFinalizationFailure>),
}

/// Selector-bound signed uncertainty whose checkpoint is not yet signed.
#[must_use = "continue only this exact provider uncertainty phase"]
pub(super) struct ResidentProcessProviderRecoveryPrepared {
    selector: ResidentDeploymentAuthoritySelector,
    store: pale_ports::AuthorityStoreCoordinate,
    pending: PreparedRecoveredProviderUnknown,
    selection_failure: Option<ResidentProcessHostSelectionError>,
}

impl std::fmt::Debug for ResidentProcessProviderRecoveryPrepared {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessProviderRecoveryPrepared")
            .field("selector", &self.selector)
            .field("selection_failure", &self.selection_failure)
            .field("recording_failure", &self.pending.failure())
            .finish_non_exhaustive()
    }
}

/// Selector-bound signed checkpoint awaiting exact attachment.
#[must_use = "attach only this exact provider uncertainty checkpoint"]
pub(super) struct ResidentProcessProviderRecoveryIssued {
    selector: ResidentDeploymentAuthoritySelector,
    store: pale_ports::AuthorityStoreCoordinate,
    pending: IssuedRecoveredProviderUnknownCheckpoint,
    selection_failure: Option<ResidentProcessHostSelectionError>,
}

impl std::fmt::Debug for ResidentProcessProviderRecoveryIssued {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessProviderRecoveryIssued")
            .field("selector", &self.selector)
            .field("selection_failure", &self.selection_failure)
            .field("recording_failure", &self.pending.failure())
            .finish_non_exhaustive()
    }
}

/// Selector-bound custody for one already-issued uncertainty candidate.
#[derive(Debug)]
#[must_use = "continue only this exact provider uncertainty candidate"]
pub(super) struct ResidentProcessProviderRecoveryCustody {
    selector: ResidentDeploymentAuthoritySelector,
    store: pale_ports::AuthorityStoreCoordinate,
    custody: ProviderOperationRecordingCustody,
}

/// Why selector-bound provider uncertainty custody remains unfinished.
#[derive(Debug)]
pub(super) enum ResidentProcessProviderRecoveryCustodyReason {
    /// The selected deployment or physical store changed.
    Selection(ResidentProcessHostSelectionError),
    /// Exact read-back or independent anchoring remains unresolved.
    Custody,
}

/// Unchanged custody plus the reason it could not advance.
#[derive(Debug)]
#[must_use = "retry only the returned exact provider uncertainty custody"]
pub(super) struct ResidentProcessProviderRecoveryCustodyFailure {
    pub(super) pending: Box<ResidentProcessProviderRecoveryCustody>,
    pub(super) reason: ResidentProcessProviderRecoveryCustodyReason,
}

/// Selector-bound semantic completion for one physically retained candidate.
#[must_use = "finish only through the selected deployment and store"]
pub(super) struct ResidentProcessProviderRecoveryFinalization {
    selector: ResidentDeploymentAuthoritySelector,
    store: pale_ports::AuthorityStoreCoordinate,
    finalization: ProviderOperationRecordingFinalization,
}

impl std::fmt::Debug for ResidentProcessProviderRecoveryFinalization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessProviderRecoveryFinalization")
            .field("selector", &self.selector)
            .finish_non_exhaustive()
    }
}

/// Why selector-bound provider uncertainty finalization remains unfinished.
#[derive(Debug)]
pub(super) enum ResidentProcessProviderRecoveryFinalizationReason {
    /// The selected deployment or physical store changed.
    Selection(ResidentProcessHostSelectionError),
    /// Completed physical custody did not match its retained semantic candidate.
    Association(OperationAttemptError),
}

/// Unchanged semantic finalization plus its exact refusal.
#[derive(Debug)]
#[must_use = "retry only the returned exact provider uncertainty finalization"]
pub(super) struct ResidentProcessProviderRecoveryFinalizationFailure {
    pub(super) pending: Box<ResidentProcessProviderRecoveryFinalization>,
    pub(super) reason: ResidentProcessProviderRecoveryFinalizationReason,
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Record one protected provider dispatch as outcome unknown.
    ///
    /// Every process-owned association is checked before custody, anchor,
    /// clock, reconciliation, rotation, or signing work. No provider port,
    /// executor, route reservation, or A2A task is available on this path.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_provider<
        'operation,
        'call,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentProcessProviderRecoveryInput<'operation, 'call>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ProviderOperationInvokeResult, ResidentProcessProviderRecoveryFailure>
    where
        Authority: pale_ports::AmbientSessionAuthorityPort,
        Reconciliation: FnOnce(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
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
        let attempt = input.attempt;
        let selector = &attempt.start.selector;
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessProviderRecoveryFailure::Selection)?;
        let application = self.deployments[index].application.as_ref();
        let selected_store = application.store();
        if access.store != selected_store {
            return Err(ResidentProcessProviderRecoveryFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        let generation = application
            .generation(&attempt.start.generation_digest)
            .ok_or(ResidentProcessProviderRecoveryFailure::Selection(
                ResidentProcessHostSelectionError::HistoricalProviderAuthority,
            ))?;
        validate_provider_recovery_associations(
            selector,
            selected_store,
            generation,
            attempt,
            &input.recording,
            input.maximum_selection_bytes,
        )
        .map_err(ResidentProcessProviderRecoveryFailure::Selection)?;

        let recovered = &attempt.start.control;
        let operation = recovered.operation();
        let history = OperationAttemptHistoryInput {
            operation: operation.operation(),
            request: operation.request(),
            session: operation.operation().historical_session_lease(),
            lease: operation.operation().historical_operation_lease(),
            generation,
            dispatched: recovered.observation().selected().1,
            selection: &attempt.selection,
            maximum_selection_bytes: input.maximum_selection_bytes,
        };
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let result = ResidentLocalOperationCoordinator::new(authority, custody, anchor)
            .recover_dispatched(
                history,
                recovered.observation(),
                &input.recording,
                admit_reconciliation,
                admit_rotation,
                sign_observation,
                sign_checkpoint,
            );
        result.map_err(|failure| match failure {
            RecoveredProviderUnknownRecordingFailure::Preparation(failure) => {
                ResidentProcessProviderRecoveryFailure::Preparation(failure)
            }
            RecoveredProviderUnknownRecordingFailure::Prepared(pending) => {
                ResidentProcessProviderRecoveryFailure::Prepared(Box::new(
                    ResidentProcessProviderRecoveryPrepared {
                        selector: selector.clone(),
                        store: selected_store.clone(),
                        pending: *pending,
                        selection_failure: None,
                    },
                ))
            }
            RecoveredProviderUnknownRecordingFailure::Issued(pending) => {
                ResidentProcessProviderRecoveryFailure::Issued(Box::new(
                    ResidentProcessProviderRecoveryIssued {
                        selector: selector.clone(),
                        store: selected_store.clone(),
                        pending: *pending,
                        selection_failure: None,
                    },
                ))
            }
            RecoveredProviderUnknownRecordingFailure::Custody(custody) => {
                ResidentProcessProviderRecoveryFailure::Custody(Box::new(
                    ResidentProcessProviderRecoveryCustody {
                        selector: selector.clone(),
                        store: selected_store.clone(),
                        custody: *custody,
                    },
                ))
            }
            RecoveredProviderUnknownRecordingFailure::Finalization(failure) => {
                let (finalization, reason) = failure.into_parts();
                ResidentProcessProviderRecoveryFailure::Finalization(Box::new(
                    ResidentProcessProviderRecoveryFinalizationFailure {
                        pending: Box::new(ResidentProcessProviderRecoveryFinalization {
                            selector: selector.clone(),
                            store: selected_store.clone(),
                            finalization,
                        }),
                        reason: ResidentProcessProviderRecoveryFinalizationReason::Association(
                            reason,
                        ),
                    },
                ))
            }
        })
    }

    /// Continue checkpoint issuance for one already signed uncertainty observation.
    pub(super) fn continue_prepared_provider_recovery<CheckpointSign>(
        &self,
        pending: ResidentProcessProviderRecoveryPrepared,
        access: ResidentDeploymentOperationAccess<'_>,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ProviderOperationInvokeResult, ResidentProcessProviderRecoveryFailure>
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentProcessProviderRecoveryPrepared {
            selector,
            store,
            pending: phase,
            selection_failure: _,
        } = pending;
        if let Err(reason) =
            self.validate_provider_recovery_continuation(&selector, &store, access.store)
        {
            return Err(ResidentProcessProviderRecoveryFailure::Prepared(Box::new(
                ResidentProcessProviderRecoveryPrepared {
                    selector,
                    store,
                    pending: phase,
                    selection_failure: Some(reason),
                },
            )));
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        map_provider_recovery_continuation(
            selector,
            store,
            continue_prepared_recovered_provider_unknown(
                phase,
                resolve_store,
                custody,
                anchor,
                checkpoint_authority,
                sign_checkpoint,
            ),
        )
    }

    /// Attach one already signed checkpoint without invoking either signer again.
    pub(super) fn continue_issued_provider_recovery(
        &self,
        pending: ResidentProcessProviderRecoveryIssued,
        access: ResidentDeploymentOperationAccess<'_>,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
    ) -> Result<ProviderOperationInvokeResult, ResidentProcessProviderRecoveryFailure> {
        let ResidentProcessProviderRecoveryIssued {
            selector,
            store,
            pending: phase,
            selection_failure: _,
        } = pending;
        if let Err(reason) =
            self.validate_provider_recovery_continuation(&selector, &store, access.store)
        {
            return Err(ResidentProcessProviderRecoveryFailure::Issued(Box::new(
                ResidentProcessProviderRecoveryIssued {
                    selector,
                    store,
                    pending: phase,
                    selection_failure: Some(reason),
                },
            )));
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        map_provider_recovery_continuation(
            selector,
            store,
            continue_issued_recovered_provider_unknown(phase, resolve_store, custody, anchor),
        )
    }

    /// Continue physical custody of one already-issued uncertainty candidate.
    pub(super) fn continue_provider_recovery_custody<Authority>(
        &self,
        pending: ResidentProcessProviderRecoveryCustody,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> Result<
        ResidentProcessProviderRecoveryFinalization,
        ResidentProcessProviderRecoveryCustodyFailure,
    > {
        let index = match self.deployment_authority_index(&pending.selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessProviderRecoveryCustodyFailure {
                    pending: Box::new(pending),
                    reason: ResidentProcessProviderRecoveryCustodyReason::Selection(reason),
                });
            }
        };
        if self.deployments[index].store() != &pending.store || &pending.store != access.store {
            return Err(ResidentProcessProviderRecoveryCustodyFailure {
                pending: Box::new(pending),
                reason: ResidentProcessProviderRecoveryCustodyReason::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            });
        }
        let ResidentProcessProviderRecoveryCustody {
            selector,
            store,
            custody,
        } = pending;
        let ResidentDeploymentOperationAccess {
            custody: operation_custody,
            anchor,
            ..
        } = access;
        match ResidentLocalOperationCoordinator::new(authority, operation_custody, anchor)
            .continue_provider_recording_custody(custody)
        {
            Ok(finalization) => Ok(ResidentProcessProviderRecoveryFinalization {
                selector,
                store,
                finalization,
            }),
            Err(custody) => Err(ResidentProcessProviderRecoveryCustodyFailure {
                pending: Box::new(ResidentProcessProviderRecoveryCustody {
                    selector,
                    store,
                    custody,
                }),
                reason: ResidentProcessProviderRecoveryCustodyReason::Custody,
            }),
        }
    }

    /// Finish one physically retained candidate through its selected deployment.
    pub(super) fn continue_provider_recovery_finalization(
        &self,
        pending: ResidentProcessProviderRecoveryFinalization,
        store: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<ProviderOperationInvokeResult, ResidentProcessProviderRecoveryFinalizationFailure>
    {
        let index = match self.deployment_authority_index(&pending.selector) {
            Ok(index) => index,
            Err(reason) => {
                return Err(ResidentProcessProviderRecoveryFinalizationFailure {
                    pending: Box::new(pending),
                    reason: ResidentProcessProviderRecoveryFinalizationReason::Selection(reason),
                });
            }
        };
        if self.deployments[index].store() != &pending.store || &pending.store != store {
            return Err(ResidentProcessProviderRecoveryFinalizationFailure {
                pending: Box::new(pending),
                reason: ResidentProcessProviderRecoveryFinalizationReason::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            });
        }
        let ResidentProcessProviderRecoveryFinalization {
            selector,
            store,
            finalization,
        } = pending;
        match finalization.finish() {
            Ok(result) => Ok(result),
            Err(failure) => {
                let (finalization, reason) = failure.into_parts();
                Err(ResidentProcessProviderRecoveryFinalizationFailure {
                    pending: Box::new(ResidentProcessProviderRecoveryFinalization {
                        selector,
                        store,
                        finalization,
                    }),
                    reason: ResidentProcessProviderRecoveryFinalizationReason::Association(reason),
                })
            }
        }
    }

    fn validate_provider_recovery_continuation(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        store: &pale_ports::AuthorityStoreCoordinate,
        access_store: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        if self.deployments[index].store() != store || store != access_store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        Ok(())
    }
}

fn map_provider_recovery_continuation(
    selector: ResidentDeploymentAuthoritySelector,
    store: pale_ports::AuthorityStoreCoordinate,
    result: Result<ProviderOperationInvokeResult, RecoveredProviderUnknownRecordingFailure>,
) -> Result<ProviderOperationInvokeResult, ResidentProcessProviderRecoveryFailure> {
    result.map_err(|failure| match failure {
        RecoveredProviderUnknownRecordingFailure::Preparation(failure) => {
            ResidentProcessProviderRecoveryFailure::Preparation(failure)
        }
        RecoveredProviderUnknownRecordingFailure::Prepared(pending) => {
            ResidentProcessProviderRecoveryFailure::Prepared(Box::new(
                ResidentProcessProviderRecoveryPrepared {
                    selector,
                    store,
                    pending: *pending,
                    selection_failure: None,
                },
            ))
        }
        RecoveredProviderUnknownRecordingFailure::Issued(pending) => {
            ResidentProcessProviderRecoveryFailure::Issued(Box::new(
                ResidentProcessProviderRecoveryIssued {
                    selector,
                    store,
                    pending: *pending,
                    selection_failure: None,
                },
            ))
        }
        RecoveredProviderUnknownRecordingFailure::Custody(custody) => {
            ResidentProcessProviderRecoveryFailure::Custody(Box::new(
                ResidentProcessProviderRecoveryCustody {
                    selector,
                    store,
                    custody: *custody,
                },
            ))
        }
        RecoveredProviderUnknownRecordingFailure::Finalization(failure) => {
            let (finalization, reason) = failure.into_parts();
            ResidentProcessProviderRecoveryFailure::Finalization(Box::new(
                ResidentProcessProviderRecoveryFinalizationFailure {
                    pending: Box::new(ResidentProcessProviderRecoveryFinalization {
                        selector,
                        store,
                        finalization,
                    }),
                    reason: ResidentProcessProviderRecoveryFinalizationReason::Association(reason),
                },
            ))
        }
    })
}

pub(super) fn validate_provider_recovery_associations(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &pale_ports::AuthorityStoreCoordinate,
    generation: &crate::resident::AdmittedCompiledGeneration,
    attempt: &ResidentProcessRecoveredAttempt,
    recording: &ProviderOperationInvokeInput<'_>,
    maximum_selection_bytes: usize,
) -> Result<(), ResidentProcessHostSelectionError> {
    let control = &attempt.start.control;
    let operation = control.operation();
    let issued = operation.operation();
    let (_, handle) = issued.handle();
    let recovered = control.observation();
    if operation.store() != store
        || recovered.store() != store
        || !attempt.start.lineage.admits(issued.handle().0, handle)
        || !selector.matches_operation(issued)
    {
        return Err(ResidentProcessHostSelectionError::ProviderOperationAuthority);
    }
    if !selector.matches_current_session(recording.current)
        || !selector
            .matches_signing_authority(recording.current.generation.signed.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::ProviderCurrentAuthority);
    }
    let historical_session = issued.historical_session_lease();
    if issued.historical_operation_lease().operation_lease_digest()
        != handle.operation_lease_sha256()
        || historical_session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical_session
            .deployment_runtime_authority_sha256()
            .as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical_session.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::HistoricalProviderAuthority);
    }

    let selected = recovered.selected();
    let predecessor = recovered
        .selected_predecessor()
        .ok_or(ResidentProcessHostSelectionError::HistoricalProviderAuthority)?;
    let head = recovered.selected_head();
    validate_provider_recovery_shape(attempt.dispatch, issued, selected, predecessor, head)?;
    let provider_binding = OperationAttemptHistoryInput {
        operation: issued,
        request: operation.request(),
        session: issued.historical_session_lease(),
        lease: issued.historical_operation_lease(),
        generation,
        dispatched: selected.1,
        selection: &attempt.selection,
        maximum_selection_bytes,
    }
    .provider_binding()
    .map_err(|_| ResidentProcessHostSelectionError::HistoricalProviderAuthority)?;
    if !attempt
        .start
        .lineage
        .admits_provider(provider_binding.as_ref())
    {
        return Err(ResidentProcessHostSelectionError::HistoricalProviderAuthority);
    }
    let request = operation.request().to_preimage();
    if !selector.matches_signing_authority(&request.signing_authority_binding)
        || !provider_observation_matches(selector, selected.1)
        || !provider_observation_matches(selector, predecessor.1)
        || !provider_observation_matches(selector, head.1)
        || !selector.matches_signing_authority(recording.signing.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::ProviderObservationAuthority);
    }
    let checkpoint = recovered.current_checkpoint().1;
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
        || checkpoint.issuer_principal_id() != &selector.principal_id
        || !checkpoint_authority_matches(selector, store, recording.checkpoint_authority)
    {
        return Err(ResidentProcessHostSelectionError::ProviderCheckpointAuthority);
    }
    Ok(())
}

fn validate_provider_recovery_shape(
    dispatch: ResidentProcessAttemptObservation,
    operation: &super::super::IssuedLocalOperation,
    selected: (
        &ResidentRecordRefV1,
        &pale_core::records::OperationObservationV1,
    ),
    predecessor: (
        &ResidentRecordRefV1,
        &pale_core::records::OperationObservationV1,
    ),
    head: (
        &ResidentRecordRefV1,
        &pale_core::records::OperationObservationV1,
    ),
) -> Result<(), ResidentProcessHostSelectionError> {
    let matching_dispatch = matches!(
        (selected.1.state(), predecessor.1.state()),
        (
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { attempt_id, evidence },
            },
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::DispatchPrepared {
                    attempt_id: prepared_attempt,
                    evidence: prepared_evidence,
                },
            },
        ) if attempt_id == prepared_attempt && evidence == prepared_evidence
    );
    if !matching_dispatch {
        return Err(ResidentProcessHostSelectionError::HistoricalProviderAuthority);
    }
    let valid_head = match dispatch {
        ResidentProcessAttemptObservation::CurrentHead => selected == head,
        ResidentProcessAttemptObservation::CancellationPredecessor => {
            selected != head
                && matches!(
                    head.1.state(),
                    OperationObservationStateV1::CancellationRequested { .. }
                )
                && head
                    .1
                    .verify_admitted_successor_of(
                        operation.handle().0,
                        operation.handle().1,
                        selected.0,
                        selected.1,
                    )
                    .is_ok()
        }
    };
    if valid_head {
        Ok(())
    } else {
        Err(ResidentProcessHostSelectionError::HistoricalProviderAuthority)
    }
}

fn provider_observation_matches(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &pale_core::records::OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/provider_recovery_support.rs"]
mod test_support;
