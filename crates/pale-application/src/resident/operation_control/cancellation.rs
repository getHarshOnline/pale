//! Durable local custody for operation cancellation requests.

use super::{
    AuthorizedOperationCancellation, OperationCancellationDisposition, RecoveredOperationControl,
    OPERATION_CANCEL_CAPABILITY_ID,
};
use crate::diagnostic::{AdmittedDiagnostic, DiagnosticAdmissionError, DiagnosticProjection};
use crate::resident::{
    operation_dispatch::signing, prepare_operation_observation_append, record_projection,
    CompletedOperationObservationCustody, OperationObservationAuthorityStoreResolver,
    OperationObservationCustodyProjectionError, OperationObservationTransition,
    PreparedOperationObservationCheckpoint,
};
use pale_core::{canon, records::*};
use pale_ports::{
    AccumulatorPreparationLimits, ControlRecord, ControlRecordCoordinate, ControlRecordDigest,
    ControlRecordError, ControlStoreError, OperationCustodyPort, OperationCustodyRecord,
    OperationCustodyRecordKind,
};

/// Fresh identities and signer for one local cancellation request.
pub struct LocalOperationCancellationRequestInput<'a> {
    /// Immutable record reference for the admitted caller request.
    pub request_ref: ResidentRecordRefV1,
    /// Immutable record reference for the successor observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Current admitted runtime signer for the successor observation.
    pub signing: SignedResidentAdmissionContextV1<'a>,
}

/// Fresh identities and signer for one post-dispatch cancellation request.
///
/// These inputs create only durable local request custody. They do not identify
/// an owner-native execution, call its carrier, or prove cancellation.
pub struct DispatchedOperationCancellationRequestInput<'a> {
    /// Immutable record reference for the admitted caller request.
    pub request_ref: ResidentRecordRefV1,
    /// Immutable record reference for the successor observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Current admitted runtime signer for the successor observation.
    pub signing: SignedResidentAdmissionContextV1<'a>,
}

struct PreparedOperationCancellationRequest<'a> {
    operation: &'a crate::resident::IssuedLocalOperation,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    request_record: pale_ports::ControlRecord,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

/// Signed cancellation request awaiting checkpoint preparation and custody.
///
/// This phase is available only before dispatch. Committing its exact append
/// makes a concurrent dispatch from the same predecessor lose the storage race.
/// It does not yet prove terminal cancellation or any remote consequence.
#[must_use = "retain and anchor the cancellation request before reporting it"]
pub struct PreparedLocalOperationCancellationRequest<'a> {
    request: PreparedOperationCancellationRequest<'a>,
}

/// Signed post-dispatch cancellation request awaiting durable custody.
///
/// Committing this value is the required local step before an owner-specific
/// cancellation carrier may run. This type deliberately has no terminal
/// completion method: acknowledgement cannot prove cancellation, and only the
/// selected owner's exact read-back may establish the next state.
#[must_use = "retain and anchor the request before any owner-specific cancellation call"]
pub struct PreparedDispatchedOperationCancellationRequest<'a> {
    request: PreparedOperationCancellationRequest<'a>,
}

/// Fresh output identities and signer for terminal local cancellation.
pub struct LocalOperationCancellationCompletionInput<'a> {
    /// Fresh immutable reference for the cancellation diagnostic.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the terminal observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Time at which the local pre-dispatch consequence was established.
    pub established_at: UnixTimeV1,
    /// Current admitted runtime signer for the terminal observation.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Maximum canonical diagnostic bytes.
    pub maximum_diagnostic_bytes: usize,
    /// Existing owner of the operation's storage coordinate.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
}

/// Anchored cancellation request recovered from durable local operation history.
///
/// This phase exists only when the current operation head is the exact
/// `CancellationRequested` observation and its referenced signed request has
/// been read, bounded, contextually admitted, and matched to the operation.
/// It grants no new cancellation authority and cannot replay the request. A
/// request whose predecessor was dispatched remains owner-routed and cannot use
/// the local pre-dispatch completion path.
#[must_use = "route or complete the recovered cancellation request without replaying it"]
pub struct RecoveredLocalOperationCancellationRequest<'a> {
    control: &'a RecoveredOperationControl,
    request_record: ControlRecord,
    request: CommonRequestEnvelopeV1,
}

/// Terminal local cancellation awaiting checkpoint preparation and custody.
///
/// This value proves only that the retained request won before dispatch. It
/// does not report a provider response, remote task state, or external effect.
#[must_use = "retain and anchor the terminal cancellation before reporting it"]
pub struct PreparedLocalOperationCancellationCompletion<'a> {
    operation: &'a crate::resident::IssuedLocalOperation,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    diagnostic: AdmittedDiagnostic,
    diagnostic_record: pale_ports::ControlRecord,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

/// Owned terminal cancellation retained after observation signing.
///
/// This private phase keeps the exact signed observation and diagnostic while
/// checkpoint projection or physical custody is retried. It cannot sign another
/// terminal observation or change the retained operation coordinate.
#[must_use = "continue the exact signed cancellation candidate to durable custody"]
pub(in crate::resident) struct OwnedLocalOperationCancellationCompletion {
    checkpoint_selection: crate::resident::checkpoint_profile::OperationCheckpointSelection,
    handle_ref: ResidentRecordRefV1,
    handle: OperationHandleV1,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    diagnostic_record: pale_ports::ControlRecord,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

impl AuthorizedOperationCancellation<'_> {
    /// Prepare a durable cancellation request for an operation before dispatch.
    ///
    /// The fresh control request was already contextually admitted by
    /// [`crate::resident::authorize_operation_cancellation`]. This method binds
    /// its exact bytes to a predecessor-linked `CancellationRequested`
    /// observation. It signs nothing and performs no write until all identities,
    /// routing state, and runtime-authority succession have been admitted.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation has crossed dispatch, is terminal or
    /// uncertain, a record reference is reused, projection fails, or observation
    /// signing is rejected. No provider or native handler is contacted.
    pub fn prepare_local_request<Rotation, Sign>(
        &self,
        input: LocalOperationCancellationRequestInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedLocalOperationCancellationRequest<'_>, OperationCancellationRequestError>
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
        if !matches!(
            self.disposition(),
            OperationCancellationDisposition::BeforeDispatch
        ) {
            return Err(OperationCancellationRequestError::Routing);
        }
        Ok(PreparedLocalOperationCancellationRequest {
            request: self.prepare_request(
                input.request_ref,
                input.observation_ref,
                input.signing,
                admit_rotation,
                sign,
            )?,
        })
    }

    /// Prepare durable custody before an owner-specific post-dispatch cancel call.
    ///
    /// The current observation must name one dispatched attempt. The resulting
    /// `CancellationRequested` observation is nonterminal and survives restart.
    /// This method neither contacts the selected owner nor assumes that the Pale
    /// attempt identity is an owner-native cancellation key.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is not currently dispatched, a
    /// reference is reused, projection fails, or observation signing is rejected.
    pub fn prepare_dispatched_request<Rotation, Sign>(
        &self,
        input: DispatchedOperationCancellationRequestInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedDispatchedOperationCancellationRequest<'_>, OperationCancellationRequestError>
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
        if !matches!(
            self.disposition(),
            OperationCancellationDisposition::Dispatched { .. }
        ) {
            return Err(OperationCancellationRequestError::Routing);
        }
        Ok(PreparedDispatchedOperationCancellationRequest {
            request: self.prepare_request(
                input.request_ref,
                input.observation_ref,
                input.signing,
                admit_rotation,
                sign,
            )?,
        })
    }

    fn prepare_request<Rotation, Sign>(
        &self,
        request_ref: ResidentRecordRefV1,
        observation_ref: ResidentRecordRefV1,
        signing_context: SignedResidentAdmissionContextV1<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedOperationCancellationRequest<'_>, OperationCancellationRequestError>
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
        let operation = self.recovered_operation.operation();
        let predecessor = self.recovered_observation.selected_head();
        let handle = operation.handle();
        let request_record = self.project_request_record(&request_ref, &observation_ref)?;
        let request_digest =
            ArtifactDigest::parse(self.request.canonical_request_sha256().as_str())
                .map_err(|_| OperationCancellationRequestError::Association)?;
        let observation = signing::issue_observation(
            &OperationObservationAdmissionContextV1 {
                signed: signing_context,
                handle_ref: handle.0,
                handle: handle.1,
                predecessor: Some(predecessor),
            },
            OperationObservationStateV1::CancellationRequested {
                request: ImmutableRecordReferenceV1 {
                    record_ref: request_ref,
                    record_sha256: request_digest,
                },
            },
            self.authorized_at,
            admit_rotation,
            sign,
        )?;
        let checkpoint = self.recovered_observation.current_checkpoint();
        Ok(PreparedOperationCancellationRequest {
            operation,
            checkpoint: (checkpoint.0.clone(), checkpoint.1.clone()),
            predecessor: (predecessor.0.clone(), predecessor.1.clone()),
            request_record,
            observation_ref,
            observation,
        })
    }

    /// Validate fresh cancellation identities and project the signed request.
    ///
    /// Consultation custody uses this same check before it atomically commits
    /// the request with the stopped private roster. Keeping the projection here
    /// prevents the consultation path from creating a weaker cancellation
    /// boundary than ordinary local or dispatched cancellation.
    pub(in crate::resident) fn project_request_record(
        &self,
        request_ref: &ResidentRecordRefV1,
        observation_ref: &ResidentRecordRefV1,
    ) -> Result<ControlRecord, OperationCancellationRequestError> {
        let operation = self.recovered_operation.operation();
        let predecessor = self.recovered_observation.selected_head();
        let handle = operation.handle();
        if request_ref == observation_ref
            || request_ref == handle.0
            || observation_ref == handle.0
            || request_ref == handle.1.operation_lease_ref()
            || observation_ref == handle.1.operation_lease_ref()
            || request_ref == operation.request_coordinate().record_ref()
            || observation_ref == operation.request_coordinate().record_ref()
            || request_ref == predecessor.0
            || observation_ref == predecessor.0
            || Some(request_ref) == predecessor.1.predecessor_ref().as_ref()
            || Some(observation_ref) == predecessor.1.predecessor_ref().as_ref()
            || request_ref == self.recovered_observation.current_checkpoint().0
            || observation_ref == self.recovered_observation.current_checkpoint().0
        {
            return Err(OperationCancellationRequestError::Association);
        }
        crate::resident::record_projection::request_record(request_ref, self.request)
            .map_err(OperationObservationCustodyProjectionError::from)
            .map_err(Into::into)
    }
}

impl PreparedLocalOperationCancellationRequest<'_> {
    /// Borrow the exact caller request retained with the cancellation observation.
    #[must_use]
    pub const fn request_record(&self) -> &pale_ports::ControlRecord {
        &self.request.request_record
    }

    /// Borrow the signed cancellation-request observation before custody.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.request.observation_ref, &self.request.observation)
    }

    /// Borrow the authenticated checkpoint used as this append's predecessor.
    #[must_use]
    pub const fn predecessor_checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.request.checkpoint.0, &self.request.checkpoint.1)
    }

    /// Prepare the exact request and observation for one atomic checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error when the store, observation lineage, selected profile,
    /// checkpoint roots, resource bounds, or physical preparation differs.
    pub fn prepare_checkpoint<'a>(
        &'a self,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        self.request
            .request_append(resolve_store)?
            .prepare_checkpoint(
                self.request.operation.checkpoint_selection(),
                &self.request.checkpoint.0,
                &self.request.checkpoint.1,
                limits,
                custody,
            )
    }

    /// Prepare terminal cancellation after this exact request reached custody.
    ///
    /// This is the same semantic transition used after restart. The completed
    /// custody must contain this request and observation and the supplied
    /// checkpoint must be its exact signed successor. No provider, handler, or
    /// carrier is contacted.
    ///
    /// # Errors
    ///
    /// Returns an error when custody, the signed checkpoint, current history,
    /// fresh identities, time, bounds, or signing authority differs.
    pub fn prepare_completion_after_custody<Rotation, Sign>(
        &self,
        completed: &CompletedOperationObservationCustody,
        checkpoint: &(ResidentRecordRefV1, ControlStateCheckpointV1),
        input: LocalOperationCancellationCompletionInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<
        PreparedLocalOperationCancellationCompletion<'_>,
        OperationCancellationCompletionError,
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
        let expected_observation = record_projection::record(
            record_projection::coordinate(
                &self.request.observation_ref,
                self.request
                    .observation
                    .canonical_operation_observation_sha256()
                    .as_str(),
            )
            .map_err(OperationObservationCustodyProjectionError::from)?,
            &self.request.observation,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let expected_checkpoint = record_projection::record(
            record_projection::coordinate(
                &checkpoint.0,
                checkpoint.1.canonical_control_checkpoint_sha256().as_str(),
            )
            .map_err(OperationObservationCustodyProjectionError::from)?,
            &checkpoint.1,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let candidate = completed.candidate();
        if candidate.record(OperationCustodyRecordKind::CommonRequest)
            != Some(&self.request.request_record)
            || candidate.record(OperationCustodyRecordKind::OperationObservation)
                != Some(&expected_observation)
            || candidate.checkpoint().successor_record() != &expected_checkpoint
        {
            return Err(OperationCancellationCompletionError::Association);
        }
        prepare_local_cancellation_completion(
            self.request.operation,
            (&checkpoint.0, &checkpoint.1),
            (&self.request.observation_ref, &self.request.observation),
            (&self.request.predecessor.0, &self.request.predecessor.1),
            &self.request.request_record,
            input,
            admit_rotation,
            sign,
        )
    }
}

impl PreparedDispatchedOperationCancellationRequest<'_> {
    /// Borrow the exact caller request retained with the cancellation observation.
    #[must_use]
    pub const fn request_record(&self) -> &pale_ports::ControlRecord {
        &self.request.request_record
    }

    /// Borrow the nonterminal cancellation-request observation before custody.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.request.observation_ref, &self.request.observation)
    }

    /// Borrow the authenticated checkpoint used as this append's predecessor.
    #[must_use]
    pub const fn predecessor_checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.request.checkpoint.0, &self.request.checkpoint.1)
    }

    /// Prepare the exact request and observation for one atomic checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error when the store, lineage, profile, checkpoint roots,
    /// resource bounds, or physical preparation differs.
    pub fn prepare_checkpoint<'a>(
        &'a self,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        self.request
            .request_append(resolve_store)?
            .prepare_checkpoint(
                self.request.operation.checkpoint_selection(),
                &self.request.checkpoint.0,
                &self.request.checkpoint.1,
                limits,
                custody,
            )
    }
}

impl PreparedOperationCancellationRequest<'_> {
    fn request_append<'a>(
        &'a self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<
        crate::resident::PreparedOperationObservationAppend<'a>,
        OperationObservationCustodyProjectionError,
    > {
        Ok(prepare_operation_observation_append(
            OperationObservationTransition::new(
                self.operation.handle().0,
                self.operation.handle().1,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_associated_record(OperationCustodyRecord::CommonRequest(
            self.request_record.clone(),
        )))
    }
}

impl RecoveredOperationControl {
    /// Recover the exact durable cancellation request at the current local head.
    ///
    /// The read is bounded and the caller must re-admit the historical signed
    /// request through its existing authority owner. The recovered request is
    /// matched to this operation, current observation, generation, fence, and
    /// original workspace. Success does not renew the request or permit replay.
    /// A dispatched predecessor is retained for owner-specific continuation; it
    /// does not permit local terminal completion or prove an owner outcome.
    ///
    /// # Errors
    ///
    /// Returns an error when the current head is not a cancellation request,
    /// evidence is missing or oversized, schema or bytes differ, historical
    /// request admission fails, or the request names another operation.
    pub fn recover_local_cancellation_request<Admit>(
        &self,
        custody: &mut dyn OperationCustodyPort,
        maximum_request_bytes: usize,
        admit_request: Admit,
    ) -> Result<RecoveredLocalOperationCancellationRequest<'_>, OperationCancellationRecoveryError>
    where
        Admit: FnOnce(
            &ResidentRecordRefV1,
            CommonRequestEnvelopeWireV1,
        ) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1>,
    {
        if maximum_request_bytes == 0 {
            return Err(OperationCancellationRecoveryError::Bounds);
        }
        let operation = self.operation().operation();
        let (head_ref, head) = self.observation().selected_head();
        let OperationObservationStateV1::CancellationRequested { request } = head.state() else {
            return Err(OperationCancellationRecoveryError::NotRequested);
        };
        let selected = self.observation().selected();
        let predecessor = if selected.0 != head_ref {
            selected
        } else {
            self.observation()
                .selected_predecessor()
                .ok_or(OperationCancellationRecoveryError::Association)?
        };
        let (predecessor_ref, predecessor) = predecessor;
        if head.predecessor_ref().as_ref() != Some(predecessor_ref)
            || head.predecessor_sha256().as_ref()
                != Some(predecessor.canonical_operation_observation_sha256())
            || !matches!(
                predecessor.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving
                        | OperationActivePhaseV1::ContextBound { .. }
                        | OperationActivePhaseV1::ProviderResolved { .. }
                        | OperationActivePhaseV1::DispatchPrepared { .. }
                        | OperationActivePhaseV1::Dispatched { .. }
                }
            )
        {
            return Err(OperationCancellationRecoveryError::Routing);
        }
        let coordinate = ControlRecordCoordinate::new(
            request.record_ref.clone(),
            ControlRecordDigest::parse(request.record_sha256.as_str())?,
        );
        let request_record = custody
            .read_operation_record(self.operation().store(), &coordinate, maximum_request_bytes)?
            .ok_or(OperationCancellationRecoveryError::Missing)?;
        if request_record.coordinate() != &coordinate
            || request_record.canonical_record_jcs().as_slice().len() > maximum_request_bytes
        {
            return Err(OperationCancellationRecoveryError::Bounds);
        }
        let schema = pale_contracts::record_schema_identity(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID)?;
        if request_record.schema_id() != &schema.schema
            || request_record.schema_sha256() != &schema.schema_sha256
        {
            return Err(OperationCancellationRecoveryError::Association);
        }
        let wire: CommonRequestEnvelopeWireV1 = serde_json::from_value(canon::parse_strict(
            request_record.canonical_record_jcs().as_slice(),
        )?)?;
        let admitted = admit_request(request_record.coordinate().record_ref(), wire)?;
        admitted.verify_operation_handle_body(operation.handle().1)?;
        if admitted.capability_id().as_str() != OPERATION_CANCEL_CAPABILITY_ID
            || admitted.workspace_id() != self.operation().request().workspace_id()
            || admitted.generation() != Some(operation.handle().1.compiled_generation_sha256())
            || admitted.fence_token() != Some(operation.handle().1.activation_fence_sha256())
            || admitted.issued_at() < *operation.handle().1.created_at()
            || admitted.issued_at() > *head.observed_at()
            || admitted.expires_at() < *head.observed_at()
            || request.record_ref != *request_record.coordinate().record_ref()
            || request.record_sha256.as_str() != admitted.canonical_request_sha256().as_str()
            || request_record.coordinate().record_sha256().as_str()
                != admitted.canonical_request_sha256().as_str()
            || canon::to_canonical_string(&admitted)?.as_bytes()
                != request_record.canonical_record_jcs().as_slice()
            || head.operation_handle_ref() != operation.handle().0
            || head.operation_handle_sha256()
                != operation.handle().1.canonical_operation_handle_sha256()
            || head_ref == operation.handle().0
        {
            return Err(OperationCancellationRecoveryError::Association);
        }
        Ok(RecoveredLocalOperationCancellationRequest {
            control: self,
            request_record,
            request: admitted,
        })
    }
}

impl RecoveredLocalOperationCancellationRequest<'_> {
    /// Borrow the historically admitted cancellation request.
    #[must_use]
    pub const fn request(&self) -> &CommonRequestEnvelopeV1 {
        &self.request
    }

    /// Prepare the terminal local consequence of the recovered request.
    ///
    /// The semantic predecessor remains the exact `CancellationRequested`
    /// observation. The physical append predecessor is the freshly recovered
    /// current checkpoint, so unrelated domain commits cannot strand the
    /// operation. This method does not replay the request or contact a handler.
    ///
    /// # Errors
    ///
    /// Returns an error for changed current history, reused references, invalid
    /// time, oversized diagnostics, or rejected observation signing.
    pub fn prepare_completion<Rotation, Sign>(
        &self,
        input: LocalOperationCancellationCompletionInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<
        PreparedLocalOperationCancellationCompletion<'_>,
        OperationCancellationCompletionError,
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
        let operation = self.control.operation().operation();
        let predecessor = self.control.observation().selected_head();
        if !matches!(
            predecessor.1.state(),
            OperationObservationStateV1::CancellationRequested { request }
                if request.record_ref == *self.request_record.coordinate().record_ref()
                    && request.record_sha256.as_str()
                        == self.request_record.coordinate().record_sha256().as_str()
        ) {
            return Err(OperationCancellationCompletionError::Association);
        }
        let Some((_, request_predecessor)) = self.control.observation().selected_predecessor()
        else {
            return Err(OperationCancellationCompletionError::Association);
        };
        if !matches!(
            request_predecessor.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving
                    | OperationActivePhaseV1::ContextBound { .. }
                    | OperationActivePhaseV1::ProviderResolved { .. }
                    | OperationActivePhaseV1::DispatchPrepared { .. }
            }
        ) {
            return Err(OperationCancellationCompletionError::Association);
        }
        prepare_local_cancellation_completion(
            operation,
            self.control.observation().current_checkpoint(),
            predecessor,
            self.control
                .observation()
                .selected_predecessor()
                .ok_or(OperationCancellationCompletionError::Association)?,
            &self.request_record,
            input,
            admit_rotation,
            sign,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_local_cancellation_completion<'a, Rotation, Sign>(
    operation: &'a crate::resident::IssuedLocalOperation,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    request_predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    request_record: &ControlRecord,
    input: LocalOperationCancellationCompletionInput<'_>,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedLocalOperationCancellationCompletion<'a>, OperationCancellationCompletionError>
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
    let (handle_ref, handle) = operation.handle();
    if !matches!(
        predecessor.1.state(),
        OperationObservationStateV1::CancellationRequested { request }
            if request.record_ref == *request_record.coordinate().record_ref()
                && request.record_sha256.as_str()
                    == request_record.coordinate().record_sha256().as_str()
    ) || !matches!(
        request_predecessor.1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving
                | OperationActivePhaseV1::ContextBound { .. }
                | OperationActivePhaseV1::ProviderResolved { .. }
                | OperationActivePhaseV1::DispatchPrepared { .. }
        }
    ) || predecessor.1.predecessor_ref().as_ref() != Some(request_predecessor.0)
        || predecessor.1.predecessor_sha256().as_ref()
            != Some(
                request_predecessor
                    .1
                    .canonical_operation_observation_sha256(),
            )
    {
        return Err(OperationCancellationCompletionError::Association);
    }
    for reference in [&input.diagnostic_ref, &input.observation_ref] {
        if reference == handle_ref
            || reference == handle.operation_lease_ref()
            || reference == operation.request_coordinate().record_ref()
            || reference == request_record.coordinate().record_ref()
            || reference == predecessor.0
            || reference == request_predecessor.0
            || reference == checkpoint.0
        {
            return Err(OperationCancellationCompletionError::Association);
        }
    }
    if input.diagnostic_ref == input.observation_ref
        || input.established_at < *predecessor.1.observed_at()
        || input.established_at < *checkpoint.1.issued_at()
        || input.established_at > *handle.retain_until()
    {
        return Err(OperationCancellationCompletionError::Association);
    }
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .map_err(DiagnosticAdmissionError::Registry)?;
    let diagnostic = DiagnosticProjection {
        code: "operation_cancelled_before_dispatch".to_string(),
        stage: DiagnosticStageV1::OperationAdmission,
        disposition: DiagnosticDispositionV1::Blocked,
        summary: "operation was cancelled before dispatch".to_string(),
        subject: Some((
            "operation".to_string(),
            handle.operation_handle_id().as_str().to_string(),
        )),
        source: None,
        schema_reference: None,
        causes: Vec::new(),
        notes: Vec::new(),
        help: Vec::new(),
        facts: Vec::new(),
        retry: DiagnosticRetryV1::NotIndicated,
        effect: DiagnosticEffectCertaintyV1::NotStarted,
        request: Some((
            "operation_cancel".to_string(),
            request_record
                .coordinate()
                .record_sha256()
                .as_str()
                .to_string(),
        )),
        generation: None,
        receipt: None,
        omitted_causes: 0,
    }
    .admit(&identity)?;
    if diagnostic.canonical_bytes().len() > input.maximum_diagnostic_bytes {
        return Err(OperationCancellationCompletionError::BoundExceeded);
    }
    let (diagnostic_digest, diagnostic_record) =
        crate::resident::record_projection::diagnostic_record(&input.diagnostic_ref, &diagnostic)
            .map_err(OperationObservationCustodyProjectionError::from)?;
    let observation = signing::issue_observation(
        &OperationObservationAdmissionContextV1 {
            signed: input.signing,
            handle_ref,
            handle,
            predecessor: Some(predecessor),
        },
        OperationObservationStateV1::Cancelled {
            diagnostic: ImmutableRecordReferenceV1 {
                record_ref: input.diagnostic_ref,
                record_sha256: diagnostic_digest,
            },
        },
        input.established_at,
        admit_rotation,
        sign,
    )?;
    Ok(PreparedLocalOperationCancellationCompletion {
        operation,
        checkpoint: (checkpoint.0.clone(), checkpoint.1.clone()),
        predecessor: (predecessor.0.clone(), predecessor.1.clone()),
        diagnostic,
        diagnostic_record,
        observation_ref: input.observation_ref,
        observation,
    })
}

impl PreparedLocalOperationCancellationCompletion<'_> {
    /// Borrow the admitted terminal diagnostic before custody.
    #[must_use]
    pub const fn diagnostic(&self) -> &AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the signed terminal observation before custody.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Borrow the freshly recovered checkpoint used as the append predecessor.
    #[must_use]
    pub const fn predecessor_checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Retain the signed candidate without retaining a borrowed recovery graph.
    ///
    /// This moves the exact terminal records and copies only the immutable
    /// operation coordinate needed by checkpoint projection. It performs no
    /// signing, storage access, identity allocation, or carrier call.
    pub(in crate::resident) fn into_owned(self) -> OwnedLocalOperationCancellationCompletion {
        OwnedLocalOperationCancellationCompletion {
            checkpoint_selection: self.operation.checkpoint_selection(),
            handle_ref: self.operation.handle().0.clone(),
            handle: self.operation.handle().1.clone(),
            checkpoint: self.checkpoint,
            predecessor: self.predecessor,
            diagnostic_record: self.diagnostic_record,
            observation_ref: self.observation_ref,
            observation: self.observation,
        }
    }

    /// Prepare the terminal observation and diagnostic for one atomic checkpoint.
    ///
    /// # Errors
    ///
    /// Returns an error when storage selection, lineage, bounds, or physical
    /// preparation differs. This method performs no write or anchor update.
    pub fn prepare_checkpoint<'a>(
        &'a self,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        prepare_operation_observation_append(
            OperationObservationTransition::new(
                self.operation.handle().0,
                self.operation.handle().1,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_associated_record(OperationCustodyRecord::Diagnostic(
            self.diagnostic_record.clone(),
        ))
        .prepare_checkpoint(
            self.operation.checkpoint_selection(),
            &self.checkpoint.0,
            &self.checkpoint.1,
            limits,
            custody,
        )
    }
}

impl OwnedLocalOperationCancellationCompletion {
    /// Borrow the exact signed terminal observation retained for continuation.
    #[must_use]
    pub(in crate::resident) const fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Borrow the checkpoint that was current when the terminal state was signed.
    #[must_use]
    pub(in crate::resident) const fn predecessor_checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Project the retained candidate into one physical checkpoint append.
    ///
    /// A retry uses the same signed terminal observation. This method cannot
    /// allocate a record identity, re-sign the observation, or contact a
    /// provider, native handler, A2A carrier, or agent.
    pub(in crate::resident) fn prepare_checkpoint<'a>(
        &'a self,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        prepare_operation_observation_append(
            OperationObservationTransition::new(
                &self.handle_ref,
                &self.handle,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_associated_record(OperationCustodyRecord::Diagnostic(
            self.diagnostic_record.clone(),
        ))
        .prepare_checkpoint(
            self.checkpoint_selection.clone(),
            &self.checkpoint.0,
            &self.checkpoint.1,
            limits,
            custody,
        )
    }
}

/// Failure before a local cancellation request is ready for checkpoint custody.
#[derive(Debug, thiserror::Error)]
pub enum OperationCancellationRequestError {
    /// Current state requires provider, consultation, established, or reconciliation routing.
    #[error("operation cancellation is not a local pre-dispatch request")]
    Routing,
    /// A fresh reference or recovered association differs.
    #[error("operation cancellation request inputs do not match the recovered operation")]
    Association,
    /// Record or observation projection failed.
    #[error(transparent)]
    Projection(#[from] OperationObservationCustodyProjectionError),
    /// The shared operation owner rejected succession, authority, encoding, or signing.
    #[error(transparent)]
    Operation(#[from] crate::resident::OperationAttemptError),
}

/// Failure before a terminal local cancellation is ready for checkpoint custody.
#[derive(Debug, thiserror::Error)]
pub enum OperationCancellationCompletionError {
    /// Custody, checkpoint, reference, or time evidence differs.
    #[error("operation cancellation completion does not match the retained request")]
    Association,
    /// The diagnostic owner rejected the terminal explanation.
    #[error(transparent)]
    Diagnostic(#[from] DiagnosticAdmissionError),
    /// The canonical diagnostic exceeds the caller's admitted bound.
    #[error("operation cancellation diagnostic exceeds its admitted byte limit")]
    BoundExceeded,
    /// Record or observation projection failed.
    #[error(transparent)]
    Projection(#[from] OperationObservationCustodyProjectionError),
    /// The shared operation owner rejected succession, authority, encoding, or signing.
    #[error(transparent)]
    Operation(#[from] crate::resident::OperationAttemptError),
}

/// Failure before a durable cancellation request can resume after recovery.
#[derive(Debug, thiserror::Error)]
pub enum OperationCancellationRecoveryError {
    /// The current operation head is not a local cancellation request.
    #[error("current operation history has no resumable local cancellation request")]
    NotRequested,
    /// The request followed consultation or another unsupported owner-specific phase.
    #[error("durable cancellation request requires owner-specific recovery")]
    Routing,
    /// The signed request referenced by the current observation is absent.
    #[error("durable cancellation request evidence is missing")]
    Missing,
    /// A zero or exceeded request-body ceiling prevents bounded recovery.
    #[error("durable cancellation request exceeds its recovery bound")]
    Bounds,
    /// Schema, bytes, identity, operation, generation, or timing differs.
    #[error("durable cancellation request does not match the current operation")]
    Association,
    /// Physical immutable request lookup failed.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// The exact request coordinate could not be formed.
    #[error(transparent)]
    Coordinate(#[from] ControlRecordError),
    /// The common-request schema identity is unavailable.
    #[error(transparent)]
    Schema(#[from] pale_contracts::SchemaError),
    /// Strict JSON parsing or canonical encoding failed.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// Retained bytes do not decode as a common request.
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
    /// The historical signed-request owner rejected the retained request.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
}
