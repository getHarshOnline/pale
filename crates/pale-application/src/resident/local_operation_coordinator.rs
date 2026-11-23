//! Private coordination of local resident operation paths.
//!
//! The records and custody phases keep their existing owners. This coordinator
//! only sequences those phases over one authority, control-store, and anchor
//! session. It cannot create authority, select a provider, reopen a consumed
//! call, or turn an uncertain outcome into success.

use super::local_operation_start::{
    prepare_local_child_start_retained, prepare_local_consult_start_retained,
    prepare_local_consult_start_with_intent_builder_retained, prepare_local_invoke_start_retained,
    HandleIssuedLocalOperationStartFailure, IssuedLocalOperationCheckpointFailure,
    PreparedLocalOperationCheckpointFailure, PreparedLocalOperationStartFailure,
    RetainedLocalOperationStartFailure, SignedLocalOperationStartFailure,
};
use super::operation_dispatch::{
    continue_consumed_provider_recording, invoke_provider_operation,
    prepare_pairwise_delegate_provider_call, prepare_provider_operation_call,
    prepare_recovered_provider_completion, prepare_recovered_provider_operation_call,
    record_provider_operation_completion, record_recovered_provider_operation_completion,
    record_recovered_provider_operation_unknown,
    recover_provider_dispatch_under_cancellation_request, retain_provider_operation_dispatch,
    ConsumedProviderRecordingContinuation, OwnedProviderInvocationRequest,
    PreparedProviderOperationCall, PreparedRecoveredProviderOperationCall,
    ProviderOperationCallBasis, ProviderOperationContinuationCoordinate,
    ProviderOperationDispatchInput, ProviderOperationInvokeFailure, ProviderOperationInvokeInput,
    ProviderOperationInvokeResult, ProviderOperationRecordingConflict,
    ProviderOperationRecordingFailure, ProviderOperationRecordingRecoveryFailure,
    ProviderOperationRetentionFailure, RecoveredProviderCallPreparationFailure,
    RecoveredProviderDispatchRelease, RecoveredProviderOperationDispatch,
    RecoveredProviderUnknownRecordingFailure, RetainedProviderOperationDispatch,
};
use super::{
    AmbientSessionAuthorityPort, CompletedOperationStartCustody, IssuedLocalOperation,
    LocalOperationStartError, LocalOperationStartInput, OperationAttemptError,
    OperationAttemptHistoryInput, OperationStartAnchorFailure, OperationStartCommitFailure,
    PendingLocalOperationStart, PreparedLocalChildParent, PreparedLocalOperationCheckpoint,
    PreparedLocalOperationStart, PreparedOperationStart, RecoveredOperationObservation,
};
use crate::provider_invocation::ProviderInvocationEvidenceAdmission;
use pale_core::records::*;
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, AuthorityStoreCoordinate,
    OperationCustodyPort, ProviderInvocationObservation, ProviderInvocationPort,
};

/// Inputs for preparing and durably starting one local operation.
pub(in crate::resident) struct ResidentLocalOperationStartInput<'a> {
    /// Exact contextually admitted semantic start input.
    pub semantic: LocalOperationStartInput<'a>,
    /// Native accumulator limits applied before the checkpoint can be signed.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Exact checkpoint identity, schema, and signer witness.
    pub checkpoint: ResidentLocalOperationCheckpointInput<'a>,
    /// Maximum encoded bytes accepted by physical custody.
    pub maximum_record_bytes: usize,
}

/// Inputs for issuing the initial operation checkpoint.
pub(in crate::resident) struct ResidentLocalOperationCheckpointInput<'a> {
    /// Stable checkpoint identity allocated for this exact start.
    pub id: ControlCheckpointIdV1,
    /// Immutable record reference allocated for the checkpoint.
    pub reference: ResidentRecordRefV1,
    /// Selected checkpoint schema digest.
    pub schema: &'a SchemaDigest,
    /// Already resolved checkpoint signing authority.
    pub authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
}

/// One private resident composition over the selected authority and custody owners.
pub(in crate::resident) struct ResidentLocalOperationCoordinator<'ports, Authority> {
    authority: &'ports mut Authority,
    custody: &'ports mut dyn OperationCustodyPort,
    anchor: &'ports mut dyn AntiRollbackAnchorPort,
}

impl<'ports, Authority> ResidentLocalOperationCoordinator<'ports, Authority> {
    /// Bind one coordinator to the already selected owners.
    pub(in crate::resident) fn new(
        authority: &'ports mut Authority,
        custody: &'ports mut dyn OperationCustodyPort,
        anchor: &'ports mut dyn AntiRollbackAnchorPort,
    ) -> Self {
        Self {
            authority,
            custody,
            anchor,
        }
    }

    /// Prepare and durably start one local Invoke through the existing owners.
    ///
    /// This method does not construct request, handle, publication, signer, or
    /// accumulator meaning. It sequences their admitted inputs so that no
    /// operation handle escapes before semantic admission, native root
    /// preparation, checkpoint issuance, atomic commit, and independent anchor.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_invoke<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentLocalOperationStartInput {
            semantic,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
        } = input;
        let prepared = prepare_local_invoke_start_retained(
            semantic,
            self.authority,
            admit_compiled_authority,
            sign_handle,
            sign_observation,
        )
        .map_err(ResidentLocalOperationStartFailure::from_semantic)?;
        self.finish_semantic_start(
            prepared,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    /// Prepare and durably start one root `Consult` through the same custody owner.
    ///
    /// This establishes only the parent operation. Roster selection, child
    /// starts, collection custody, and dispatch remain separate owners.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_consult<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        intent: Option<&super::consultation::ConsultationStartIntent>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentLocalOperationStartInput {
            semantic,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
        } = input;
        let prepared = prepare_local_consult_start_retained(
            semantic,
            intent.cloned(),
            self.authority,
            admit_compiled_authority,
            sign_handle,
            sign_observation,
        )
        .map_err(ResidentLocalOperationStartFailure::from_semantic)?;
        self.finish_semantic_start(
            prepared,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_consult_with_intent_builder<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        Intent,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        build_intent: Intent,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Intent: FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
        ) -> Result<
            super::consultation::ConsultationStartIntent,
            LocalOperationStartError,
        >,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentLocalOperationStartInput {
            semantic,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
        } = input;
        let prepared = prepare_local_consult_start_with_intent_builder_retained(
            semantic,
            self.authority,
            admit_compiled_authority,
            sign_handle,
            sign_observation,
            build_intent,
        )
        .map_err(ResidentLocalOperationStartFailure::from_semantic)?;
        self.finish_semantic_start(
            prepared,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    /// Durably start one already-protected `Invoke` or nested `Consult` child.
    ///
    /// The parent owner has already selected its exact current transaction.
    /// This method admits the narrower child and completes only that child's
    /// checkpoint and custody. It does not add the child to a consultation roster.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_consultation_child<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        parent: PreparedLocalChildParent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentLocalOperationStartInput {
            semantic,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
        } = input;
        let child_class = semantic.handle.operation_class;
        if !matches!(
            child_class,
            OperationClassV1::Invoke | OperationClassV1::Consult
        ) {
            return Err(ResidentLocalOperationStartFailure::Preparation(
                LocalOperationStartError::Coordinate,
            ));
        }
        let prepared = prepare_local_child_start_retained(
            semantic,
            parent,
            child_class,
            self.authority,
            admit_compiled_authority,
            sign_handle,
            sign_observation,
        )
        .map_err(ResidentLocalOperationStartFailure::from_semantic)?;
        self.finish_semantic_start(
            prepared,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    /// Durably start one `Delegate` child under a protected active parent.
    ///
    /// This path does not create Consult membership. The semantic owner checks
    /// the exact parent coordinate, generation, lease, and narrowed authority.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn start_delegate_child<
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        input: ResidentLocalOperationStartInput<'_>,
        parent: PreparedLocalChildParent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::AdmittedAmbientSession,
            &super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentLocalOperationStartInput {
            semantic,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
        } = input;
        let prepared = prepare_local_child_start_retained(
            semantic,
            parent,
            OperationClassV1::Delegate,
            self.authority,
            admit_compiled_authority,
            sign_handle,
            sign_observation,
        )
        .map_err(ResidentLocalOperationStartFailure::from_semantic)?;
        self.finish_semantic_start(
            prepared,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    pub(in crate::resident) fn finish_semantic_start<CheckpointPreimage, CheckpointSign>(
        &mut self,
        prepared: PreparedLocalOperationStart,
        accumulator_limits: AccumulatorPreparationLimits,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        maximum_record_bytes: usize,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let prepared = prepared
            .prepare_checkpoint(accumulator_limits, self.custody)
            .map_err(|failure| {
                ResidentLocalOperationStartFailure::SignedStart(Box::new(failure))
            })?;
        self.finish_prepared_checkpoint(
            prepared,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    /// Issue and attach the checkpoint for one already prepared native root.
    ///
    /// The handle, observation, semantic projection, and native accumulator are
    /// already complete. This phase accepts only the remaining checkpoint
    /// preimage owner and checkpoint signer.
    fn finish_prepared_checkpoint<CheckpointPreimage, CheckpointSign>(
        &mut self,
        prepared: PreparedLocalOperationCheckpoint,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        maximum_record_bytes: usize,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let preimage = match prepare_checkpoint_preimage(&prepared) {
            Ok(preimage) => preimage,
            Err(failure) => {
                return Err(ResidentLocalOperationStartFailure::PreparedCheckpoint {
                    continuation: Box::new(prepared),
                    failure: Box::new(failure),
                });
            }
        };
        if preimage.control_checkpoint_id != checkpoint.id {
            return Err(ResidentLocalOperationStartFailure::PreparedCheckpoint {
                continuation: Box::new(prepared),
                failure: Box::new(LocalOperationStartError::Coordinate),
            });
        }
        let retained_preimage = preimage.clone();
        let issued = prepared
            .issue_checkpoint_retained(
                &checkpoint.reference,
                preimage,
                checkpoint.schema,
                checkpoint.authority,
                sign_checkpoint,
            )
            .map_err(
                |failure| ResidentLocalOperationStartFailure::CheckpointIssuance {
                    continuation: Box::new(failure),
                    preimage: Box::new(retained_preimage),
                },
            )?;
        let (physical, pending) = issued.attach_retained().map_err(|failure| {
            ResidentLocalOperationStartFailure::CheckpointAttachment(Box::new(failure))
        })?;
        self.complete_start(physical, pending, maximum_record_bytes)
            .map_err(ResidentLocalOperationStartFailure::Custody)
    }

    /// Continue native checkpoint preparation for one exact signed start.
    ///
    /// This phase accepts neither start signer nor semantic admission. The
    /// retained signed handle and observation remain the only values used.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_signed_start<CheckpointPreimage, CheckpointSign>(
        &mut self,
        failure: PreparedLocalOperationStartFailure,
        accumulator_limits: AccumulatorPreparationLimits,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        maximum_record_bytes: usize,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let prepared = failure
            .continue_checkpoint(accumulator_limits, self.custody)
            .map_err(|failure| {
                ResidentLocalOperationStartFailure::SignedStart(Box::new(failure))
            })?;
        self.finish_prepared_checkpoint(
            prepared,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    /// Continue checkpoint issuance from one already prepared native root.
    ///
    /// Native preparation and both start signatures are retained. Only the
    /// checkpoint preimage owner and checkpoint signer can run.
    pub(in crate::resident) fn continue_prepared_checkpoint<CheckpointPreimage, CheckpointSign>(
        &mut self,
        prepared: PreparedLocalOperationCheckpoint,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        maximum_record_bytes: usize,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.finish_prepared_checkpoint(
            prepared,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    /// Retry checkpoint validation or signing for one prepared native root.
    ///
    /// The retained preimage must still name the allocated checkpoint. No start
    /// signer, semantic admission, preimage builder, or native accumulator
    /// preparation is exposed.
    pub(in crate::resident) fn continue_checkpoint_issuance<CheckpointSign>(
        &mut self,
        failure: PreparedLocalOperationCheckpointFailure,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        preimage: Box<ControlStateCheckpointPreimageV1>,
        maximum_record_bytes: usize,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if preimage.control_checkpoint_id != checkpoint.id {
            return Err(ResidentLocalOperationStartFailure::CheckpointIssuance {
                continuation: Box::new(failure),
                preimage,
            });
        }
        let retained_preimage = preimage.clone();
        let issued = failure
            .continue_issuance(
                &checkpoint.reference,
                *preimage,
                checkpoint.schema,
                checkpoint.authority,
                sign_checkpoint,
            )
            .map_err(
                |failure| ResidentLocalOperationStartFailure::CheckpointIssuance {
                    continuation: Box::new(failure),
                    preimage: retained_preimage,
                },
            )?;
        let (physical, pending) = issued.attach_retained().map_err(|failure| {
            ResidentLocalOperationStartFailure::CheckpointAttachment(Box::new(failure))
        })?;
        self.complete_start(physical, pending, maximum_record_bytes)
            .map_err(ResidentLocalOperationStartFailure::Custody)
    }

    /// Continue deterministic preparation around the same signed handle and observation.
    ///
    /// Neither start signer is accepted by this phase. Only the consultation
    /// intent, checkpoint preparation, and checkpoint signer that have not yet
    /// completed are supplied.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_signed_semantic_start<
        Intent,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        failure: SignedLocalOperationStartFailure,
        build_intent: Intent,
        accumulator_limits: AccumulatorPreparationLimits,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        maximum_record_bytes: usize,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        Intent: FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
        ) -> Result<
            Option<super::consultation::ConsultationStartIntent>,
            LocalOperationStartError,
        >,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let prepared = failure
            .continue_preparation(build_intent)
            .map_err(|failure| {
                ResidentLocalOperationStartFailure::SignedSemanticStart(Box::new(failure))
            })?;
        self.finish_semantic_start(
            prepared,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    /// Continue observation issuance around one already signed handle.
    ///
    /// This phase accepts no handle signer. Once the observation is signed, the
    /// existing signed-semantic and checkpoint path completes the same start.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_handle_issued_semantic_start<
        ObservationSign,
        Intent,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        failure: HandleIssuedLocalOperationStartFailure,
        sign_observation: ObservationSign,
        build_intent: Intent,
        accumulator_limits: AccumulatorPreparationLimits,
        checkpoint: ResidentLocalOperationCheckpointInput<'_>,
        maximum_record_bytes: usize,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure>
    where
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Intent: FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
        ) -> Result<
            Option<super::consultation::ConsultationStartIntent>,
            LocalOperationStartError,
        >,
        CheckpointPreimage:
            FnOnce(
                &PreparedLocalOperationCheckpoint,
            )
                -> Result<ControlStateCheckpointPreimageV1, LocalOperationStartError>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let signed = failure
            .continue_observation(sign_observation)
            .map_err(|failure| {
                ResidentLocalOperationStartFailure::HandleIssued(Box::new(failure))
            })?;
        let prepared = signed.complete(build_intent).map_err(|failure| {
            ResidentLocalOperationStartFailure::SignedSemanticStart(Box::new(failure))
        })?;
        self.finish_semantic_start(
            prepared,
            accumulator_limits,
            checkpoint,
            maximum_record_bytes,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        )
    }

    /// Commit, anchor, and expose one already prepared local operation start.
    ///
    /// The semantic start and signed checkpoint are prepared by their existing
    /// owners before this method. A handle is returned only after both physical
    /// custody steps match the pending application locator.
    pub(in crate::resident) fn complete_start(
        &mut self,
        prepared: PreparedOperationStart,
        pending: PendingLocalOperationStart,
        maximum_record_bytes: usize,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationCustodyFailure> {
        let database = match prepared.commit(self.custody, self.anchor, maximum_record_bytes) {
            Ok(database) => database,
            Err(failure) => {
                return Err(ResidentLocalOperationCustodyFailure::Commit {
                    pending: Box::new(pending),
                    failure: Box::new(failure),
                });
            }
        };
        let completed = match database.advance_anchor(self.anchor) {
            Ok(completed) => completed,
            Err(failure) => {
                return Err(ResidentLocalOperationCustodyFailure::Anchor {
                    pending: Box::new(pending),
                    failure: Box::new(failure),
                });
            }
        };
        pending.finish(completed).map_err(|owners| {
            ResidentLocalOperationCustodyFailure::Association {
                pending: Box::new(owners.0),
                completed: Box::new(owners.1),
            }
        })
    }

    /// Continue attachment of one already signed initial checkpoint.
    ///
    /// This path accepts no signer. If native preparation must be repeated after
    /// an attachment failure, the retained signed checkpoint remains the only
    /// checkpoint value that can enter custody.
    pub(in crate::resident) fn continue_checkpoint_attachment(
        &mut self,
        failure: IssuedLocalOperationCheckpointFailure,
        maximum_record_bytes: usize,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationStartFailure> {
        let (physical, pending) = failure
            .continue_attachment(self.custody)
            .map_err(|failure| {
                ResidentLocalOperationStartFailure::CheckpointAttachment(Box::new(failure))
            })?;
        self.complete_start(physical, pending, maximum_record_bytes)
            .map_err(ResidentLocalOperationStartFailure::Custody)
    }

    /// Continue one exact durable-start custody phase without rebuilding records.
    ///
    /// Database mutation is retried only when the custody owner proves that
    /// another commit is safe. Anchor mutation is retried only when the anchor
    /// owner proves that no unlike successor was retained. The final
    /// association check is pure and preserves both owners on another mismatch.
    pub(in crate::resident) fn continue_start_custody(
        &mut self,
        failure: ResidentLocalOperationCustodyFailure,
        maximum_record_bytes: usize,
    ) -> Result<IssuedLocalOperation, ResidentLocalOperationCustodyFailure> {
        match failure {
            ResidentLocalOperationCustodyFailure::Commit { pending, failure } => {
                let database = match failure.try_into_retryable() {
                    Ok(prepared) => {
                        return self.complete_start(prepared, *pending, maximum_record_bytes);
                    }
                    Err(failure) => match failure.reconcile_exact(self.custody) {
                        Ok(database) => database,
                        Err(failure) => {
                            return Err(ResidentLocalOperationCustodyFailure::Commit {
                                pending,
                                failure: Box::new(failure),
                            });
                        }
                    },
                };
                let completed = match database.advance_anchor(self.anchor) {
                    Ok(completed) => completed,
                    Err(failure) => {
                        return Err(ResidentLocalOperationCustodyFailure::Anchor {
                            pending,
                            failure: Box::new(failure),
                        });
                    }
                };
                pending.finish(completed).map_err(|owners| {
                    ResidentLocalOperationCustodyFailure::Association {
                        pending: Box::new(owners.0),
                        completed: Box::new(owners.1),
                    }
                })
            }
            ResidentLocalOperationCustodyFailure::Anchor { pending, failure } => {
                let completed = match failure.try_into_retryable() {
                    Ok(database) => match database.advance_anchor(self.anchor) {
                        Ok(completed) => completed,
                        Err(failure) => {
                            return Err(ResidentLocalOperationCustodyFailure::Anchor {
                                pending,
                                failure: Box::new(failure),
                            });
                        }
                    },
                    Err(failure) => match failure.continue_after_failed_observation(self.anchor) {
                        Ok(completed) => completed,
                        Err(failure) => {
                            return Err(ResidentLocalOperationCustodyFailure::Anchor {
                                pending,
                                failure: Box::new(failure),
                            });
                        }
                    },
                };
                pending.finish(completed).map_err(|owners| {
                    ResidentLocalOperationCustodyFailure::Association {
                        pending: Box::new(owners.0),
                        completed: Box::new(owners.1),
                    }
                })
            }
            ResidentLocalOperationCustodyFailure::Association { pending, completed } => pending
                .finish(*completed)
                .map_err(|owners| ResidentLocalOperationCustodyFailure::Association {
                    pending: Box::new(owners.0),
                    completed: Box::new(owners.1),
                }),
        }
    }

    /// Retain one provider dispatch and consume its provider call at most once.
    ///
    /// Retention completes before release. Preflight failures preserve the
    /// unopened dispatch. Once the provider is called, only terminal recording,
    /// reconciliation custody, or protected recording continuation can follow.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn invoke_provider<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Provider,
        Reconciliation,
    >(
        &mut self,
        dispatch: ProviderOperationDispatchInput<'operation, '_>,
        invocation: ProviderOperationInvokeInput<'_>,
        provider: &Provider,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        mut admit_reconciliation: Reconciliation,
        mut admit_rotation: Rotation,
        mut sign_observation: ObservationSign,
        mut sign_checkpoint: CheckpointSign,
    ) -> Result<ProviderOperationInvokeResult, ResidentProviderInvokeFailure<'operation>>
    where
        Authority: AmbientSessionAuthorityPort,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Provider: ProviderInvocationPort,
        Reconciliation:
            FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
    {
        let retained = self
            .retain_provider_dispatch(
                dispatch,
                &mut admit_rotation,
                &mut sign_observation,
                &mut sign_checkpoint,
            )
            .map_err(ResidentProviderInvokeFailure::Retention)?;
        self.invoke_retained_provider(
            retained,
            invocation,
            provider,
            evidence,
            &mut admit_reconciliation,
            &mut admit_rotation,
            &mut sign_observation,
            &mut sign_checkpoint,
        )
        .map_err(ResidentProviderInvokeFailure::Invocation)
    }

    /// Retain provider resolution, private selection, and dispatch before release.
    ///
    /// This split is used by the resident event loop, which can observe a
    /// cancellation between durable dispatch and provider release. The returned
    /// phase still has no provider-call method of its own.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_provider_dispatch<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        dispatch: ProviderOperationDispatchInput<'operation, '_>,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        RetainedProviderOperationDispatch<'operation>,
        ProviderOperationRetentionFailure<'operation>,
    >
    where
        Authority: AmbientSessionAuthorityPort,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        retain_provider_operation_dispatch(
            dispatch,
            self.authority,
            self.custody,
            self.anchor,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Complete preflight for one newly retained recovered dispatch.
    ///
    /// A historical dispatch cannot construct the required linear release.
    pub(in crate::resident) fn prepare_recovered_provider_call(
        &mut self,
        release: RecoveredProviderDispatchRelease,
        input: &ProviderOperationInvokeInput<'_>,
    ) -> Result<PreparedRecoveredProviderOperationCall, RecoveredProviderCallPreparationFailure>
    where
        Authority: AmbientSessionAuthorityPort,
    {
        prepare_recovered_provider_operation_call(
            release,
            input,
            self.authority,
            self.custody,
            self.anchor,
        )
    }

    /// Consume one already retained provider dispatch exactly once.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn invoke_retained_provider<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Provider,
        Reconciliation,
    >(
        &mut self,
        retained: RetainedProviderOperationDispatch<'operation>,
        invocation: ProviderOperationInvokeInput<'_>,
        provider: &Provider,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ProviderOperationInvokeResult, ProviderOperationInvokeFailure<'operation>>
    where
        Authority: AmbientSessionAuthorityPort,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Provider: ProviderInvocationPort,
        Reconciliation:
            FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
    {
        invoke_provider_operation(
            retained,
            invocation,
            provider,
            evidence,
            self.authority,
            self.custody,
            self.anchor,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Perform fresh release preflight without calling the provider.
    pub(in crate::resident) fn prepare_retained_provider_call<'operation>(
        &mut self,
        retained: RetainedProviderOperationDispatch<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
    ) -> Result<PreparedProviderOperationCall<'operation>, ProviderOperationInvokeFailure<'operation>>
    where
        Authority: AmbientSessionAuthorityPort,
    {
        prepare_provider_operation_call(retained, input, self.authority, self.custody, self.anchor)
    }

    /// Perform fresh release preflight for one lineage-proved pairwise Delegate.
    pub(in crate::resident) fn prepare_pairwise_delegate_provider_call<'operation>(
        &mut self,
        retained: RetainedProviderOperationDispatch<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
    ) -> Result<PreparedProviderOperationCall<'operation>, ProviderOperationInvokeFailure<'operation>>
    where
        Authority: AmbientSessionAuthorityPort,
    {
        prepare_pairwise_delegate_provider_call(
            retained,
            input,
            self.authority,
            self.custody,
            self.anchor,
        )
    }

    /// Admit and record one task-owner provider completion without another call.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn record_provider_completion<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        basis: ProviderOperationCallBasis<'operation>,
        request: &OwnedProviderInvocationRequest,
        observation: Result<
            ProviderInvocationObservation,
            crate::provider_invocation::ProviderInvocationAdmissionError,
        >,
        input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ProviderOperationInvokeResult, ProviderOperationRecordingFailure<'operation>>
    where
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Reconciliation:
            FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
    {
        record_provider_operation_completion(
            basis,
            request,
            observation,
            input,
            evidence,
            self.custody,
            self.anchor,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Rejoin an accepted provider completion to protected durable history.
    pub(in crate::resident) fn prepare_recovered_provider_completion<'operation>(
        &mut self,
        continuation: &ProviderOperationContinuationCoordinate,
        request: &OwnedProviderInvocationRequest,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        admission: crate::provider_invocation::ProviderInvocationAdmissionContext<'operation>,
        maximum_record_bytes: usize,
    ) -> Result<RecoveredProviderOperationDispatch<'operation>, OperationAttemptError> {
        prepare_recovered_provider_completion(
            continuation,
            request,
            history,
            recovered,
            admission,
            self.custody,
            self.anchor,
            maximum_record_bytes,
        )
    }

    /// Record an already accepted call after its protected-history rejoin.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn record_recovered_provider_completion<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        continuation: ProviderOperationContinuationCoordinate,
        request: &OwnedProviderInvocationRequest,
        observation: Result<
            ProviderInvocationObservation,
            crate::provider_invocation::ProviderInvocationAdmissionError,
        >,
        recovered: RecoveredProviderOperationDispatch<'operation>,
        admission: crate::provider_invocation::ProviderInvocationAdmissionContext<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        evidence: &dyn ProviderInvocationEvidenceAdmission,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ProviderOperationInvokeResult, ProviderOperationRecordingFailure<'static>>
    where
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Reconciliation:
            FnMut(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
    {
        record_recovered_provider_operation_completion(
            continuation,
            request,
            observation,
            recovered,
            admission,
            input,
            evidence,
            self.custody,
            self.anchor,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Continue recording one consumed call without reopening its provider.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_recording<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        continuation: ConsumedProviderRecordingContinuation<'operation>,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ProviderOperationInvokeResult, ProviderOperationRecordingFailure<'operation>>
    where
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
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
        continue_consumed_provider_recording(
            continuation,
            input,
            self.custody,
            self.anchor,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Rejoin a consumed call to one protected cancellation-aware head.
    ///
    /// Only the original dispatch or its direct `CancellationRequested`
    /// successor can become the recording predecessor. Late provider evidence
    /// remains attributable; cancellation does not discard it.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn continue_after_concurrent_head<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        conflict: ProviderOperationRecordingConflict<'operation>,
        recovered: &RecoveredOperationObservation,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ProviderOperationInvokeResult, ResidentProviderRecordingResumeFailure<'operation>>
    where
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
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
        let continuation = conflict
            .recover(recovered)
            .map_err(ResidentProviderRecordingResumeFailure::Recovery)?;
        self.continue_recording(
            continuation,
            input,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
        .map_err(ResidentProviderRecordingResumeFailure::Recording)
    }

    /// Record a cold-restart dispatch as outcome unknown without a provider port.
    ///
    /// Protected recovery must prove the original private selection, current
    /// head, checkpoint, and anchor. The API deliberately has no way to replay
    /// the provider call.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn recover_dispatched<
        'operation,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Reconciliation,
    >(
        &mut self,
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
        input: &ProviderOperationInvokeInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ProviderOperationInvokeResult, RecoveredProviderUnknownRecordingFailure>
    where
        Authority: AmbientSessionAuthorityPort,
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
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
        let recovered_dispatch = recover_provider_dispatch_under_cancellation_request(
            OperationAttemptHistoryInput {
                operation: history.operation,
                request: history.request,
                session: history.session,
                lease: history.lease,
                generation: history.generation,
                dispatched: history.dispatched,
                selection: history.selection,
                maximum_selection_bytes: history.maximum_selection_bytes,
            },
            recovered,
        )
        .map_err(|failure| {
            RecoveredProviderUnknownRecordingFailure::Preparation(Box::new(failure.into()))
        })?;
        recovered_dispatch.provider_request().map_err(|failure| {
            RecoveredProviderUnknownRecordingFailure::Preparation(Box::new(failure.into()))
        })?;
        recovered_dispatch
            .validate_current_at(input.current, self.authority)
            .map_err(|failure| {
                RecoveredProviderUnknownRecordingFailure::Preparation(Box::new(failure.into()))
            })?;
        record_recovered_provider_operation_unknown(
            history,
            recovered,
            input,
            self.custody,
            self.anchor,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    /// Continue one already-issued provider terminal candidate without
    /// rebuilding, re-signing, or reopening provider work.
    pub(in crate::resident) fn continue_provider_recording_custody(
        &mut self,
        custody: super::operation_dispatch::ProviderOperationRecordingCustody,
    ) -> Result<
        super::operation_dispatch::ProviderOperationRecordingFinalization,
        super::operation_dispatch::ProviderOperationRecordingCustody,
    > {
        custody.continue_custody(self.custody, self.anchor)
    }
}

/// Where durable local-start custody stopped.
pub(in crate::resident) enum ResidentLocalOperationCustodyFailure {
    /// The database transaction did not reach exact custody.
    Commit {
        pending: Box<PendingLocalOperationStart>,
        failure: Box<OperationStartCommitFailure>,
    },
    /// The database committed, but independent anchoring did not complete.
    Anchor {
        pending: Box<PendingLocalOperationStart>,
        failure: Box<OperationStartAnchorFailure>,
    },
    /// Completed physical custody did not match the pending application locator.
    Association {
        pending: Box<PendingLocalOperationStart>,
        completed: Box<CompletedOperationStartCustody>,
    },
}

/// Where semantic preparation or durable operation-start custody stopped.
pub(in crate::resident) enum ResidentLocalOperationStartFailure {
    /// The operation was rejected before durable start custody.
    Preparation(LocalOperationStartError),
    /// The handle was signed; initial observation issuance stopped.
    HandleIssued(Box<HandleIssuedLocalOperationStartFailure>),
    /// Handle and observation are signed; deterministic semantic projection stopped.
    SignedSemanticStart(Box<SignedLocalOperationStartFailure>),
    /// Handle and initial observation are signed; native preparation stopped.
    SignedStart(Box<PreparedLocalOperationStartFailure>),
    /// Native roots are prepared, but the checkpoint preimage was rejected.
    PreparedCheckpoint {
        continuation: Box<PreparedLocalOperationCheckpoint>,
        failure: Box<LocalOperationStartError>,
    },
    /// Checkpoint validation or signing stopped; the exact preimage is retained.
    CheckpointIssuance {
        continuation: Box<PreparedLocalOperationCheckpointFailure>,
        preimage: Box<ControlStateCheckpointPreimageV1>,
    },
    /// A checkpoint was signed once, but physical attachment stopped.
    CheckpointAttachment(Box<IssuedLocalOperationCheckpointFailure>),
    /// Issued records did not complete database and independent-anchor custody.
    Custody(ResidentLocalOperationCustodyFailure),
}

impl ResidentLocalOperationStartFailure {
    fn from_semantic(failure: RetainedLocalOperationStartFailure) -> Self {
        match failure {
            RetainedLocalOperationStartFailure::BeforeSigned(error) => Self::Preparation(error),
            RetainedLocalOperationStartFailure::HandleIssued(failure) => {
                Self::HandleIssued(Box::new(failure))
            }
            RetainedLocalOperationStartFailure::Signed(failure) => {
                Self::SignedSemanticStart(Box::new(failure))
            }
        }
    }
}

impl std::fmt::Debug for ResidentLocalOperationStartFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Preparation(_) => "Preparation",
            Self::HandleIssued(_) => "HandleIssued",
            Self::SignedSemanticStart(_) => "SignedSemanticStart",
            Self::SignedStart(_) => "SignedStart",
            Self::PreparedCheckpoint { .. } => "PreparedCheckpoint",
            Self::CheckpointIssuance { .. } => "CheckpointIssuance",
            Self::CheckpointAttachment(_) => "CheckpointAttachment",
            Self::Custody(_) => "Custody",
        };
        formatter
            .debug_struct("ResidentLocalOperationStartFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResidentLocalOperationCustodyFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Commit { .. } => "Commit",
            Self::Anchor { .. } => "Anchor",
            Self::Association { .. } => "Association",
        };
        formatter
            .debug_struct("ResidentLocalOperationCustodyFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Where fresh provider dispatch or invocation stopped.
pub(in crate::resident) enum ResidentProviderInvokeFailure<'operation> {
    /// Provider selection never reached retained dispatch custody.
    Retention(ProviderOperationRetentionFailure<'operation>),
    /// Dispatch was retained and invocation stopped at its exact owned phase.
    Invocation(ProviderOperationInvokeFailure<'operation>),
}

impl std::fmt::Debug for ResidentProviderInvokeFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Retention(_) => "Retention",
            Self::Invocation(_) => "Invocation",
        };
        formatter
            .debug_struct("ResidentProviderInvokeFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Where cancellation-aware recording resumption stopped.
pub(in crate::resident) enum ResidentProviderRecordingResumeFailure<'operation> {
    /// Protected history did not admit a safe recording predecessor.
    Recovery(ProviderOperationRecordingRecoveryFailure<'operation>),
    /// The consumed call rejoined a safe head but did not finish recording.
    Recording(ProviderOperationRecordingFailure<'operation>),
}

impl std::fmt::Debug for ResidentProviderRecordingResumeFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Recovery(_) => "Recovery",
            Self::Recording(_) => "Recording",
        };
        formatter
            .debug_struct("ResidentProviderRecordingResumeFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}
