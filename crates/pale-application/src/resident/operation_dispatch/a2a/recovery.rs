//! Recover one committed pre-Task A2A send without granting replay.
//!
//! A process can stop after durable request custody but before it records the
//! carrier's first response. Recovery cannot infer whether the remote endpoint
//! accepted that request. This module authenticates the retained send and turns
//! it into an explicit outcome-unknown continuation. It never rebuilds a
//! release-capable exchange or calls a carrier.

use super::*;
use crate::resident::{
    checkpoint_profile::{a2a_evidence_leaf, a2a_record_leaf, OperationLeafProjectionError},
    operation_custody::read_index,
    operation_dispatch::provider::{
        recover_provider_dispatch_under_cancellation_request, recover_provider_operation_dispatch,
        RecoveredProviderOperationDispatch,
    },
    prepare_operation_observation_append, LocalOperationRecoveryError,
    OperationCheckpointAncestryLimits, OperationCheckpointSelection,
    OperationObservationCustodyProjectionError, OperationObservationTransition,
    OperationRecoveryError, PreparedOperationObservationCheckpoint,
};
use pale_core::canon;
use pale_ports::{
    AccumulatorMembershipLeaf, AccumulatorMembershipLimits, AccumulatorPreparationLimits,
    ControlRecord, ControlRecordCoordinate, ControlStoreError, OperationCustodyIndexKey,
    OperationCustodyIndexQuery, OperationCustodyPort, OperationCustodyRecord,
};

/// Independent ceilings for one pre-Task send recovery.
#[derive(Debug, Clone, Copy)]
pub struct A2aSendRecoveryLimits {
    maximum_record_bytes: usize,
    maximum_evidence_bytes: usize,
    checkpoint: OperationCheckpointAncestryLimits,
    membership: AccumulatorMembershipLimits,
}

impl A2aSendRecoveryLimits {
    /// Construct finite record, evidence, ancestry, and membership ceilings.
    ///
    /// # Errors
    /// Returns [`A2aSendRecoveryError::Bounds`] when either byte ceiling is zero.
    pub fn new(
        maximum_record_bytes: usize,
        maximum_evidence_bytes: usize,
        checkpoint: OperationCheckpointAncestryLimits,
        membership: AccumulatorMembershipLimits,
    ) -> Result<Self, A2aSendRecoveryError> {
        if maximum_record_bytes == 0 || maximum_evidence_bytes == 0 {
            return Err(A2aSendRecoveryError::Bounds);
        }
        Ok(Self {
            maximum_record_bytes,
            maximum_evidence_bytes,
            checkpoint,
            membership,
        })
    }
}

/// Exact historical dispatch and physical scope selected for send recovery.
pub struct A2aSendRecoveryInput<'a> {
    /// Historical provider-attempt inputs used by their existing recovery owner.
    pub history: OperationAttemptHistoryInput<'a>,
    /// Anchored local history that contains the exact dispatch for this recovery mode.
    pub recovered: &'a RecoveredOperationObservation,
    /// Explicit finite recovery ceilings.
    pub limits: A2aSendRecoveryLimits,
}

/// Historical owners required to re-admit the retained checkpoint and route.
pub struct A2aSendRecoveryAdmission<'a> {
    /// Admit an existing signed checkpoint from its exact retained bytes.
    pub admit_checkpoint:
        &'a mut dyn FnMut(
            &ResidentRecordRefV1,
            ControlStateCheckpointWireV1,
        ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    /// Re-admit the exact historical route. This grants no fresh route use.
    pub admit_route: &'a mut dyn FnMut(
        &ResidentRecordRefV1,
        A2aRouteAdmissionWireV1,
    ) -> Result<A2aRouteAdmissionV1, InvalidResidentRecordV1>,
}

/// A committed send whose remote acceptance cannot be recovered locally.
///
/// This value deliberately has no carrier or start method. Its only state-changing
/// continuation is an outcome-unknown observation with an existing reconciliation
/// frontier.
#[must_use = "record the recovered uncertainty before reconciliation"]
pub struct RecoveredA2aSendOutcomeUnknown<'a> {
    operation: &'a IssuedLocalOperation,
    recovered: &'a RecoveredOperationObservation,
    preparation_record: ControlRecord,
    preparation: A2aSendPreparation,
    route: (ResidentRecordRefV1, A2aRouteAdmissionV1),
    request_evidence: OperationA2aEvidence,
    diagnostic: crate::diagnostic::AdmittedDiagnostic,
}

/// Read-only proof of the private A2A preparation behind one provider dispatch.
///
/// This value proves the exact provider selection, route, request bytes,
/// checkpoint membership, and independent anchor. It cannot start a carrier or
/// establish that the remote system received the request.
pub(in crate::resident) struct RecoveredA2aSendPreparation<'a> {
    operation: &'a IssuedLocalOperation,
    recovered: &'a RecoveredOperationObservation,
    preparation_record: ControlRecord,
    preparation: A2aSendPreparation,
    route: (ResidentRecordRefV1, A2aRouteAdmissionV1),
    request_evidence: OperationA2aEvidence,
}

/// What the exact provider-dispatch index proves about a retained A2A send.
///
/// Only an exact absent index observation establishes the first state. A present
/// row must pass every preparation, checkpoint, route, request, membership, and
/// anchor check before it reaches the second state. All other outcomes remain
/// explicit evidence failures and cannot fall back to ordinary provider replay.
pub(in crate::resident) enum A2aSendCustodyProbe<'a> {
    NoRetainedA2aSend,
    ExactRetainedA2aSend(Box<RecoveredA2aSendPreparation<'a>>),
    UntrustedOrIncompleteA2aEvidence(A2aSendRecoveryError),
}

/// Return whether the exact provider dispatch indexes an A2A send preparation.
///
/// This bounded read needs no A2A route or checkpoint admission. An absent
/// index is therefore enough to keep a crossed Delegate with its generic
/// provider recovery owner. A present index proves only that A2A evidence must
/// be admitted; it does not establish that the evidence is valid.
pub(in crate::resident) fn has_indexed_a2a_send_preparation(
    history: &OperationAttemptHistoryInput<'_>,
    recovered: &RecoveredOperationObservation,
    custody: &mut dyn OperationCustodyPort,
    maximum_record_bytes: usize,
) -> Result<bool, A2aSendRecoveryError> {
    let dispatch = recover_provider_operation_dispatch(
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
    )?;
    read_indexed_a2a_send_preparation(&dispatch, custody, maximum_record_bytes)
        .map(|record| record.is_some())
}

fn read_indexed_a2a_send_preparation(
    dispatch: &RecoveredProviderOperationDispatch<'_>,
    custody: &mut dyn OperationCustodyPort,
    maximum_record_bytes: usize,
) -> Result<Option<ControlRecord>, A2aSendRecoveryError> {
    if maximum_record_bytes == 0 {
        return Err(A2aSendRecoveryError::Bounds);
    }
    if dispatch.operation().handle().1.operation_class() != &OperationClassV1::Delegate {
        return Err(A2aSendRecoveryError::Association);
    }
    let dispatch_coordinate = record_projection::coordinate(
        dispatch.observation().0,
        dispatch
            .observation()
            .1
            .canonical_operation_observation_sha256()
            .as_str(),
    )
    .map_err(|_| A2aSendRecoveryError::Association)?;
    let query = OperationCustodyIndexQuery::A2aSendPreparation(
        OperationCustodyIndexKey::for_a2a_send_preparation(&dispatch_coordinate),
    );
    read_index(custody, dispatch.store(), query, maximum_record_bytes).map_err(Into::into)
}

impl RecoveredA2aSendPreparation<'_> {
    /// Return whether one admitted task binding names this exact original send.
    #[must_use]
    pub(in crate::resident) fn matches_binding(&self, binding: &A2aTaskBindingV1) -> bool {
        binding.operation_handle_ref() == self.operation.handle().0
            && binding.operation_handle_sha256()
                == self
                    .operation
                    .handle()
                    .1
                    .canonical_operation_handle_sha256()
            && binding.operation_observation_ref() == &self.preparation.dispatch_ref
            && binding.operation_observation_sha256() == &self.preparation.dispatch_sha256
            && binding.attempt_id() == &self.preparation.attempt_id
            && binding.route_admission_ref() == &self.preparation.route_admission_ref
            && binding.route_admission_sha256() == &self.preparation.route_admission_sha256
            && binding.route_admission_id() == self.route.1.route_admission_id()
            && binding.json_rpc_request_id() == &self.preparation.json_rpc_request_id
            && binding.request_mode() == &self.preparation.request_mode
            && binding.request_evidence() == &self.preparation.request_evidence
            && self.request_evidence.coordinate().record_ref()
                == &self.preparation.request_evidence.record_ref
            && self.request_evidence.coordinate().record_sha256().as_str()
                == self.preparation.request_evidence.record_sha256.as_str()
    }
}

impl RecoveredA2aSendOutcomeUnknown<'_> {
    /// Borrow the explicit recovery diagnostic.
    #[must_use]
    pub const fn diagnostic(&self) -> &crate::diagnostic::AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the exact retained request bytes. They are not replay authority.
    #[must_use]
    pub const fn request_evidence(&self) -> &OperationA2aEvidence {
        &self.request_evidence
    }

    /// Borrow the historically admitted route. This grants no fresh route use.
    #[must_use]
    pub fn route(&self) -> (&ResidentRecordRefV1, &A2aRouteAdmissionV1) {
        (&self.route.0, &self.route.1)
    }

    /// Borrow the exact private preparation coordinate.
    #[must_use]
    pub const fn preparation(&self) -> &ControlRecordCoordinate {
        self.preparation_record.coordinate()
    }

    pub(in crate::resident) fn process_task_key(
        &self,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        Ok(crate::resident::task_owner::ResidentOperationTaskKey::new(
            self.recovered.store().clone(),
            ImmutableRecordReferenceV1 {
                record_ref: self.preparation.dispatch_ref.clone(),
                record_sha256: ArtifactDigest::parse(self.preparation.dispatch_sha256.as_str())
                    .map_err(|_| OperationAttemptError::Encoding)?,
            },
        ))
    }
}

/// Recovery failure. No variant grants transport retry or remote Task identity.
#[derive(Debug, thiserror::Error)]
pub enum A2aSendRecoveryError {
    /// A finite record, evidence, ancestry, or proof ceiling was exhausted.
    #[error("A2A send recovery bounds exceeded")]
    Bounds,
    /// Required indexed, immutable, or protocol evidence is absent.
    #[error("A2A send recovery evidence is missing")]
    Missing,
    /// The selected operation, dispatch, route, request, or checkpoint differs.
    #[error("A2A send recovery association differs")]
    Association,
    /// Existing provider-attempt recovery rejected the historical dispatch.
    #[error(transparent)]
    Attempt(#[from] OperationAttemptError),
    /// Existing checkpoint ancestry or membership proof failed.
    #[error(transparent)]
    Checkpoint(#[from] LocalOperationRecoveryError),
    /// Typed index lookup failed.
    #[error(transparent)]
    Lookup(#[from] OperationRecoveryError),
    /// Physical immutable record or evidence lookup failed.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// Existing historical owner rejected signed route or checkpoint meaning.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// A logical checkpoint leaf could not be projected exactly.
    #[error(transparent)]
    Projection(#[from] OperationObservationCustodyProjectionError),
    /// Canonical private-record encoding failed.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// Strict private or public wire decoding failed.
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
    /// The diagnostic contract could not represent the recovered uncertainty.
    #[error(transparent)]
    Diagnostic(#[from] crate::diagnostic::DiagnosticAdmissionError),
}

/// Recover durable pre-Task send custody as explicit unknown certainty.
///
/// The existing provider-dispatch owner first re-admits the selected attempt.
/// Recovery then proves the private preparation, route, request bytes, indexed
/// checkpoint, current accumulator membership, and independent anchor. It does
/// no write, signing, network access, or retry.
///
/// # Errors
/// Rejects missing, excessive, non-canonical, foreign, stale, or substituted
/// evidence. A failure never permits replaying the original request.
pub fn recover_a2a_send_outcome_unknown<'a>(
    input: A2aSendRecoveryInput<'a>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admission: A2aSendRecoveryAdmission<'_>,
) -> Result<RecoveredA2aSendOutcomeUnknown<'a>, A2aSendRecoveryError> {
    let recovered = recover_a2a_send_preparation(
        input,
        custody,
        anchor,
        admission,
        A2aSendRecoveryHead::Dispatched,
    )?;
    let diagnostic = recovered_uncertainty_diagnostic(
        recovered.operation,
        &recovered.preparation.json_rpc_request_id,
    )?;
    Ok(RecoveredA2aSendOutcomeUnknown {
        operation: recovered.operation,
        recovered: recovered.recovered,
        preparation_record: recovered.preparation_record,
        preparation: recovered.preparation,
        route: recovered.route,
        request_evidence: recovered.request_evidence,
        diagnostic,
    })
}

/// Recover the A2A send preparation beneath one direct cancellation-request head.
///
/// This private path admits the original provider selection, route, request
/// bytes, checkpoint membership, and independent anchor. It creates no
/// transport request and grants no replay or cancellation authority.
pub(in crate::resident) fn recover_a2a_send_preparation_under_cancellation_request<'a>(
    input: A2aSendRecoveryInput<'a>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admission: A2aSendRecoveryAdmission<'_>,
) -> Result<RecoveredA2aSendPreparation<'a>, A2aSendRecoveryError> {
    recover_a2a_send_preparation(
        input,
        custody,
        anchor,
        admission,
        A2aSendRecoveryHead::DirectCancellationRequest,
    )
}

/// Distinguish exact absence from a fully proved send and unusable A2A evidence.
///
/// The probe reads only the preparation index derived from the admitted provider
/// dispatch before it attempts the complete A2A proof. It never calls a provider
/// or carrier and never grants request replay.
pub(in crate::resident) fn probe_a2a_send_custody_for_known_task<'a>(
    input: A2aSendRecoveryInput<'a>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admission: A2aSendRecoveryAdmission<'_>,
) -> A2aSendCustodyProbe<'a> {
    let A2aSendRecoveryInput {
        history,
        recovered,
        limits,
    } = input;
    let dispatch = match recover_provider_operation_dispatch(history, recovered) {
        Ok(dispatch) => dispatch,
        Err(error) => return A2aSendCustodyProbe::UntrustedOrIncompleteA2aEvidence(error.into()),
    };
    let preparation_record =
        match read_indexed_a2a_send_preparation(&dispatch, custody, limits.maximum_record_bytes) {
            Ok(None) => return A2aSendCustodyProbe::NoRetainedA2aSend,
            Ok(Some(record)) => record,
            Err(error) => return A2aSendCustodyProbe::UntrustedOrIncompleteA2aEvidence(error),
        };
    match recover_indexed_a2a_send_preparation(
        dispatch,
        preparation_record,
        limits,
        custody,
        anchor,
        admission,
    ) {
        Ok(recovered) => A2aSendCustodyProbe::ExactRetainedA2aSend(Box::new(recovered)),
        Err(error) => A2aSendCustodyProbe::UntrustedOrIncompleteA2aEvidence(error),
    }
}

enum A2aSendRecoveryHead {
    Dispatched,
    DirectCancellationRequest,
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/operation_dispatch/a2a_recovery.rs"]
mod tests;

fn recover_a2a_send_preparation<'a>(
    input: A2aSendRecoveryInput<'a>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admission: A2aSendRecoveryAdmission<'_>,
    head: A2aSendRecoveryHead,
) -> Result<RecoveredA2aSendPreparation<'a>, A2aSendRecoveryError> {
    let A2aSendRecoveryInput {
        history,
        recovered,
        limits,
    } = input;
    let dispatch = match head {
        A2aSendRecoveryHead::Dispatched => recover_provider_operation_dispatch(history, recovered)?,
        A2aSendRecoveryHead::DirectCancellationRequest => {
            recover_provider_dispatch_under_cancellation_request(history, recovered)?
        }
    };
    let preparation_record =
        read_indexed_a2a_send_preparation(&dispatch, custody, limits.maximum_record_bytes)?
            .ok_or(A2aSendRecoveryError::Missing)?;
    recover_indexed_a2a_send_preparation(
        dispatch,
        preparation_record,
        limits,
        custody,
        anchor,
        admission,
    )
}

fn recover_indexed_a2a_send_preparation<'a>(
    dispatch: RecoveredProviderOperationDispatch<'a>,
    preparation_record: ControlRecord,
    limits: A2aSendRecoveryLimits,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    mut admission: A2aSendRecoveryAdmission<'_>,
) -> Result<RecoveredA2aSendPreparation<'a>, A2aSendRecoveryError> {
    let store = dispatch.store();
    let operation = dispatch.operation();
    let preparation = admit_preparation(&preparation_record, &dispatch)?;

    let checkpoint_query = OperationCustodyIndexQuery::A2aSendPreparationCheckpoint(
        OperationCustodyIndexKey::for_a2a_send_preparation_checkpoint(
            preparation_record.coordinate(),
        ),
    );
    let checkpoint_record = read_index(
        custody,
        store,
        checkpoint_query,
        limits.maximum_record_bytes,
    )?
    .ok_or(A2aSendRecoveryError::Missing)?;
    let checkpoint = dispatch.recovered_observation().current.verify_ancestry(
        &checkpoint_record,
        limits.maximum_record_bytes,
        limits.checkpoint,
        custody,
        &mut admission.admit_checkpoint,
    )?;

    let route_coordinate = record_projection::coordinate(
        &preparation.route_admission_ref,
        preparation.route_admission_sha256.as_str(),
    )
    .map_err(|_| A2aSendRecoveryError::Association)?;
    let route_record = custody
        .read_operation_record(store, &route_coordinate, limits.maximum_record_bytes)?
        .ok_or(A2aSendRecoveryError::Missing)?;
    if route_record.coordinate() != &route_coordinate
        || route_record.schema_id().as_str() != A2A_ROUTE_ADMISSION_V1_SCHEMA
    {
        return Err(A2aSendRecoveryError::Association);
    }
    let route_wire: A2aRouteAdmissionWireV1 =
        serde_json::from_slice(route_record.canonical_record_jcs().as_slice())?;
    let route = (admission.admit_route)(route_record.coordinate().record_ref(), route_wire)?;
    if route.canonical_a2a_route_admission_sha256() != &preparation.route_admission_sha256
        || canon::to_canonical_string(&route)?.as_bytes()
            != route_record.canonical_record_jcs().as_slice()
    {
        return Err(A2aSendRecoveryError::Association);
    }
    let request = dispatch.provider_request()?;
    let handle = operation.handle().1;
    if route.deployment_runtime_authority_ref() != handle.deployment_runtime_authority_ref()
        || route.deployment_runtime_authority_sha256().as_str()
            != handle.deployment_runtime_authority_sha256().as_str()
        || route.issuer_principal_id() != handle.issuer_principal_id()
        || route.provider_binding().provider_binding_id != request.provider_binding().id
        || route.provider_binding().provider_binding_identity
            != request.provider_binding().binding_identity
        || !route
            .route()
            .matches_request_binding(operation.handle().1.request_binding())
        || route.observed_at() > dispatch.observation().1.observed_at()
        || dispatch.observation().1.observed_at() > route.fresh_until()
    {
        return Err(A2aSendRecoveryError::Association);
    }

    let request_coordinate = record_projection::coordinate(
        &preparation.request_evidence.record_ref,
        preparation.request_evidence.record_sha256.as_str(),
    )
    .map_err(|_| A2aSendRecoveryError::Association)?;
    let request_evidence = custody
        .read_a2a_evidence(store, &request_coordinate, limits.maximum_evidence_bytes)?
        .ok_or(A2aSendRecoveryError::Missing)?;
    let inserted = custody.read_a2a_evidence_inserted_at(
        store,
        *checkpoint.committed_transaction_sequence(),
        1,
        limits.maximum_evidence_bytes,
    )?;
    if inserted.as_slice() != [request_evidence.clone()] {
        return Err(A2aSendRecoveryError::Association);
    }

    let current = dispatch.recovered_observation().current_checkpoint().1;
    let mut leaves = Vec::with_capacity(3);
    for (class, logical, coordinate) in [
        (
            "a2a-send-preparation",
            preparation_record.coordinate().record_ref().as_str(),
            preparation_record.coordinate(),
        ),
        (
            "a2a-route-admission",
            route_record.coordinate().record_ref().as_str(),
            route_record.coordinate(),
        ),
    ] {
        let leaf =
            a2a_record_leaf(store, current, class, logical, coordinate).map_err(map_leaf_error)?;
        leaves.push(
            AccumulatorMembershipLeaf::new(leaf.hashed_key, leaf.value)
                .map_err(LocalOperationRecoveryError::from)?,
        );
    }
    let evidence_leaf =
        a2a_evidence_leaf(store, current, &request_evidence).map_err(map_leaf_error)?;
    leaves.push(
        AccumulatorMembershipLeaf::new(evidence_leaf.hashed_key, evidence_leaf.value)
            .map_err(LocalOperationRecoveryError::from)?,
    );
    leaves.sort_by(|left, right| left.key().cmp(right.key()));
    dispatch.recovered_observation().current.verify_membership(
        &operation.checkpoint_selection(),
        leaves,
        limits.membership,
        custody,
        anchor,
    )?;

    Ok(RecoveredA2aSendPreparation {
        operation,
        recovered: dispatch.recovered_observation(),
        preparation_record,
        preparation,
        route: (route_coordinate.record_ref().clone(), route),
        request_evidence,
    })
}

fn admit_preparation(
    record: &ControlRecord,
    dispatch: &crate::resident::operation_dispatch::provider::RecoveredProviderOperationDispatch<
        '_,
    >,
) -> Result<A2aSendPreparation, A2aSendRecoveryError> {
    if record.schema_id().as_str() != PRIVATE_OPERATION_A2A_SEND_PREPARATION_V1_SCHEMA
        || record.schema_sha256()
            != &A2aSendPreparation::schema_digest()
                .map_err(|_| A2aSendRecoveryError::Association)?
        || pale_core::sha256_hex(record.canonical_record_jcs().as_slice())
            != record.coordinate().record_sha256().as_str()
    {
        return Err(A2aSendRecoveryError::Association);
    }
    let preparation: A2aSendPreparation =
        serde_json::from_slice(record.canonical_record_jcs().as_slice())?;
    if canon::to_canonical_string(&preparation)?.as_bytes()
        != record.canonical_record_jcs().as_slice()
    {
        return Err(A2aSendRecoveryError::Association);
    }
    let (handle_ref, handle) = dispatch.operation().handle();
    let (dispatch_ref, observation) = dispatch.observation();
    let (attempt_id, selection) =
        dispatched_attempt(observation).map_err(|_| A2aSendRecoveryError::Association)?;
    let distinct = [
        handle_ref,
        dispatch_ref,
        &selection.record_ref,
        &preparation.route_admission_ref,
        &preparation.request_evidence.record_ref,
        record.coordinate().record_ref(),
    ];
    if preparation.operation_handle_ref != *handle_ref
        || preparation.operation_handle_sha256 != *handle.canonical_operation_handle_sha256()
        || preparation.dispatch_ref != *dispatch_ref
        || preparation.dispatch_sha256 != *observation.canonical_operation_observation_sha256()
        || preparation.attempt_selection != *selection
        || preparation.attempt_id != *attempt_id
        || preparation.request_mode != A2aTaskRequestModeV1::SendMessage
        || preparation.native_message_id.is_empty()
        || distinct
            .iter()
            .enumerate()
            .any(|(index, value)| distinct[..index].contains(value))
    {
        return Err(A2aSendRecoveryError::Association);
    }
    Ok(preparation)
}

fn recovered_uncertainty_diagnostic(
    operation: &IssuedLocalOperation,
    request_id: &A2aJsonRpcRequestIdV1,
) -> Result<crate::diagnostic::AdmittedDiagnostic, crate::diagnostic::DiagnosticAdmissionError> {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .map_err(crate::diagnostic::DiagnosticAdmissionError::Registry)?;
    crate::diagnostic::DiagnosticProjection {
        code: "a2a_send_outcome_unknown_after_recovery".to_string(),
        stage: DiagnosticStageV1::ResidentRecovery,
        disposition: DiagnosticDispositionV1::EffectOutcomeUnknown,
        summary: "the committed A2A request has no retained first response after recovery"
            .to_string(),
        subject: Some((
            "operation".to_string(),
            operation
                .handle()
                .1
                .operation_handle_id()
                .as_str()
                .to_string(),
        )),
        source: None,
        schema_reference: None,
        causes: Vec::new(),
        notes: vec!["The original SendMessage request must not be replayed.".to_string()],
        help: vec!["Reconcile owner-native state before selecting another action.".to_string()],
        facts: Vec::new(),
        retry: DiagnosticRetryV1::ReconcileFirst,
        effect: DiagnosticEffectCertaintyV1::OutcomeUnknown,
        request: Some(("a2a_json_rpc".to_string(), request_id.as_str().to_string())),
        generation: None,
        receipt: None,
        omitted_causes: 0,
    }
    .admit(&identity)
}

/// A recovered uncertainty whose signed observation could not be prepared.
#[derive(thiserror::Error)]
#[error("recovered A2A send uncertainty observation could not be prepared")]
pub struct RecoveredA2aSendObservationFailure<'a> {
    outcome: Box<RecoveredA2aSendOutcomeUnknown<'a>>,
    #[source]
    reason: OperationAttemptError,
}

impl std::fmt::Debug for RecoveredA2aSendObservationFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecoveredA2aSendObservationFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl<'a> RecoveredA2aSendObservationFailure<'a> {
    /// Recover the non-replayable phase after correcting rejected inputs.
    pub fn into_outcome(self) -> RecoveredA2aSendOutcomeUnknown<'a> {
        *self.outcome
    }

    /// Borrow the exact rejected observation reason.
    #[must_use]
    pub const fn reason(&self) -> &OperationAttemptError {
        &self.reason
    }
}

/// Signed outcome-unknown observation awaiting its exact checkpoint.
#[must_use = "retain the observation through checkpoint, database, and anchor custody"]
pub struct PreparedRecoveredA2aSendObservation<'a> {
    operation: &'a IssuedLocalOperation,
    recovered: &'a RecoveredOperationObservation,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    diagnostic: crate::diagnostic::AdmittedDiagnostic,
    diagnostic_record: ControlRecord,
}

impl<'a> RecoveredA2aSendOutcomeUnknown<'a> {
    /// Sign an outcome-unknown observation without reconstructing a carrier.
    ///
    /// # Errors
    /// Rejects aliased references, backward time, elapsed retention, an unknown
    /// reconciliation frontier, excessive diagnostics, or signing failure.
    pub fn prepare_observation<Reconciliation, Rotation, Sign>(
        self,
        input: A2aOutcomeUnknownObservationInput<'_>,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedRecoveredA2aSendObservation<'a>, RecoveredA2aSendObservationFailure<'a>>
    where
        Reconciliation:
            FnOnce(&ReconciliationFrontierReferenceV1) -> Result<(), InvalidResidentRecordV1>,
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
        let prepared = (|| {
            let predecessor = self.recovered.selected();
            let checkpoint = self.recovered.current_checkpoint();
            let floor = std::cmp::max(*predecessor.1.observed_at(), *checkpoint.1.issued_at());
            let observed_at =
                crate::resident::operation_dispatch::custody::sample_time(input.clock, floor)?;
            if observed_at > *self.operation.handle().1.retain_until() {
                return Err(OperationAttemptError::RetentionElapsed);
            }
            let aliases = [
                self.operation.handle().0,
                self.operation.handle().1.operation_lease_ref(),
                self.operation.request_coordinate().record_ref(),
                predecessor.0,
                &self.preparation.attempt_selection.record_ref,
                &self.preparation.route_admission_ref,
                &self.preparation.request_evidence.record_ref,
                self.preparation_record.coordinate().record_ref(),
                checkpoint.0,
            ];
            if input.diagnostic_ref == input.observation_ref
                || aliases.iter().any(|reference| {
                    *reference == &input.diagnostic_ref || *reference == &input.observation_ref
                })
                || aliases.contains(&&input.reconciliation.record_ref)
                || input.reconciliation.record_ref == input.diagnostic_ref
                || input.reconciliation.record_ref == input.observation_ref
            {
                return Err(OperationAttemptError::Association);
            }
            admit_reconciliation(&input.reconciliation).map_err(OperationAttemptError::from)?;
            if self.diagnostic.canonical_bytes().len() > input.maximum_diagnostic_bytes {
                return Err(OperationAttemptError::BoundExceeded);
            }
            let (diagnostic_sha256, diagnostic_record) =
                record_projection::diagnostic_record(&input.diagnostic_ref, &self.diagnostic)
                    .map_err(OperationObservationCustodyProjectionError::from)?;
            let observation = signing::issue_observation(
                &OperationObservationAdmissionContextV1 {
                    signed: input.signing,
                    handle_ref: self.operation.handle().0,
                    handle: self.operation.handle().1,
                    predecessor: Some(predecessor),
                },
                OperationObservationStateV1::OutcomeUnknown {
                    diagnostic: ImmutableRecordReferenceV1 {
                        record_ref: input.diagnostic_ref,
                        record_sha256: diagnostic_sha256,
                    },
                    reconciliation: input.reconciliation,
                    effect_receipt: None,
                },
                observed_at,
                admit_rotation,
                sign,
            )?;
            Ok((input.observation_ref, observation, diagnostic_record))
        })();
        match prepared {
            Ok((observation_ref, observation, diagnostic_record)) => {
                Ok(PreparedRecoveredA2aSendObservation {
                    operation: self.operation,
                    recovered: self.recovered,
                    observation_ref,
                    observation,
                    diagnostic: self.diagnostic,
                    diagnostic_record,
                })
            }
            Err(reason) => Err(RecoveredA2aSendObservationFailure {
                outcome: Box::new(self),
                reason,
            }),
        }
    }
}

impl PreparedRecoveredA2aSendObservation<'_> {
    /// Borrow the admitted recovery diagnostic before physical retention.
    #[must_use]
    pub const fn diagnostic(&self) -> &crate::diagnostic::AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the signed operation observation before physical retention.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Borrow the authenticated checkpoint that retained the recovered request.
    #[must_use]
    pub fn predecessor_checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        self.recovered.current_checkpoint()
    }

    pub(super) fn checkpoint_selection(&self) -> OperationCheckpointSelection {
        self.operation.checkpoint_selection()
    }

    pub(super) fn retain_until(&self) -> UnixTimeV1 {
        *self.operation.handle().1.retain_until()
    }

    /// Prepare the exact current-checkpoint successor for this observation.
    ///
    /// # Errors
    /// Preserves lineage, authority-owned storage, and accumulator failures.
    pub fn prepare_checkpoint<'a>(
        &'a self,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<
        PreparedOperationObservationCheckpoint<'a>,
        OperationObservationCustodyProjectionError,
    > {
        let predecessor = self.recovered.selected();
        let append = prepare_operation_observation_append(
            OperationObservationTransition::new(
                self.operation.handle().0,
                self.operation.handle().1,
                predecessor.0,
                predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?
        .with_associated_record(OperationCustodyRecord::Diagnostic(
            self.diagnostic_record.clone(),
        ));
        let checkpoint = self.recovered.current_checkpoint();
        append.prepare_checkpoint(
            self.operation.checkpoint_selection(),
            checkpoint.0,
            checkpoint.1,
            limits,
            custody,
        )
    }
}

fn map_leaf_error(error: OperationLeafProjectionError) -> A2aSendRecoveryError {
    let projection = match error {
        OperationLeafProjectionError::Store => {
            OperationObservationCustodyProjectionError::StoreScope
        }
        OperationLeafProjectionError::Identity(error) => error.into(),
        OperationLeafProjectionError::Canonicalize(error) => error.into(),
    };
    projection.into()
}
