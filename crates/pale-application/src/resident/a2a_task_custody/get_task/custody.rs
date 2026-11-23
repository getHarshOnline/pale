//! Private physical custody before one authorized `GetTask` carrier release.

use super::super::{binding_key, RecoveredA2aTaskHistory};
use super::service::{
    authorize_a2a_get_task_readback, A2aGetTaskReadbackAuthorizationContext,
    A2aGetTaskReadbackError, A2aGetTaskReadbackInput, InFlightA2aGetTaskReadback,
    PreparedA2aGetTaskReadback,
};
use crate::resident::{
    checkpoint_profile::{
        a2a_evidence_leaf, a2a_record_leaf, A2aCustodyLeaf, OperationLeafProjectionError,
    },
    operation_custody::{
        prepare_operation_observation_candidate,
        DatabaseCommittedOperationObservationAnchorPending,
        OperationObservationCandidatePreparationError,
    },
    record_projection, CompletedOperationObservationCustody, OperationCheckpointInput,
    OperationCheckpointSelection, OperationObservationCustodyAnchorFailure,
    OperationObservationCustodyCommitFailure, OperationObservationCustodyProjectionError,
    PreparedOperationObservationCustody,
};
use pale_core::records::*;
use pale_ports::*;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{num::NonZeroUsize, time::Duration};

/// Fresh physical identities and bounds for one durable read preparation.
pub(in crate::resident) struct A2aGetTaskPreparationInput<'a> {
    pub readback: A2aGetTaskReadbackInput<'a>,
    pub preparation_ref: ResidentRecordRefV1,
    pub current_predecessor: OperationCustodyIndexPredecessor,
    pub maximum_record_bytes: usize,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct A2aGetTaskPreparation {
    operation_handle_ref: ResidentRecordRefV1,
    operation_handle_sha256: OperationHandleDigest,
    dispatch_ref: ResidentRecordRefV1,
    dispatch_sha256: OperationObservationDigest,
    attempt_selection_ref: ResidentRecordRefV1,
    attempt_selection_sha256: ArtifactDigest,
    attempt_id: CapabilityAttemptIdV1,
    route_admission_ref: ResidentRecordRefV1,
    route_admission_sha256: A2aRouteAdmissionDigest,
    task_binding_ref: ResidentRecordRefV1,
    task_binding_sha256: A2aTaskBindingDigest,
    task_binding_id: A2aTaskBindingIdV1,
    local_head_ref: ResidentRecordRefV1,
    local_head_sha256: OperationObservationDigest,
    remote_head_ref: ResidentRecordRefV1,
    remote_head_sha256: A2aTaskObservationDigest,
    authorized_at: UnixTimeV1,
    json_rpc_request_id: A2aJsonRpcRequestIdV1,
    request_evidence: ImmutableRecordReferenceV1,
    reserved_response_ref: ResidentRecordRefV1,
    request_sha256: ArtifactDigest,
    tls_trust_input_sha256: ArtifactDigest,
    limits: A2aGetTaskFiniteLimits,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct A2aGetTaskFiniteLimits {
    request_bytes: u64,
    response_bytes: u64,
    evidence_bytes: u64,
    line_bytes: u64,
    event_bytes: u64,
    events: u64,
    queued_events: u64,
    connect_timeout_nanos: u64,
    read_timeout_nanos: u64,
    total_timeout_nanos: u64,
}

#[derive(Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(in crate::resident) enum A2aGetTaskLocalDispositionKind {
    ReadStopped,
    ClockFailure,
    ExchangeFailure,
    EvidenceFailure,
    ResponseFailure,
    AssociationFailure,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct A2aGetTaskLocalDisposition {
    preparation_ref: ResidentRecordRefV1,
    preparation_sha256: ArtifactDigest,
    task_binding_ref: ResidentRecordRefV1,
    task_binding_sha256: A2aTaskBindingDigest,
    task_binding_id: A2aTaskBindingIdV1,
    disposition: A2aGetTaskLocalDispositionKind,
    recorded_at: UnixTimeV1,
}

/// Owned facts required to retain a same-process local terminal disposition.
#[derive(Clone)]
pub(in crate::resident) struct A2aGetTaskLocalDispositionBasis {
    selection: OperationCheckpointSelection,
    issuer_principal_id: ResidentPrincipalIdV1,
    preparation: ControlRecord,
    binding_ref: ResidentRecordRefV1,
    binding_sha256: A2aTaskBindingDigest,
    binding_id: A2aTaskBindingIdV1,
    current_key: OperationCustodyIndexKey,
    store: AuthorityStoreCoordinate,
    earliest_recorded_at: UnixTimeV1,
    retain_until: UnixTimeV1,
    maximum_record_bytes: usize,
}

pub(in crate::resident) enum RecoveredA2aGetTaskCustody {
    Absent,
    Preparation(RecoveredA2aGetTaskPrivateRecord),
    LocalDisposition(RecoveredA2aGetTaskPrivateRecord),
    PublicObservation(ControlRecord),
}

pub(in crate::resident) struct RecoveredA2aGetTaskPrivateRecord {
    record: ControlRecord,
    checkpoint: ControlRecord,
}

impl RecoveredA2aGetTaskPrivateRecord {
    /// Return the retained private record and its admitted checkpoint.
    pub(in crate::resident) fn coordinates(
        &self,
    ) -> (&ControlRecordCoordinate, &ControlRecordCoordinate) {
        (self.record.coordinate(), self.checkpoint.coordinate())
    }
}

#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum A2aGetTaskCustodyRecoveryError {
    #[error("A2A GetTask custody record is missing")]
    Missing,
    #[error("A2A GetTask custody association differs")]
    Association,
    #[error(transparent)]
    Store(#[from] ControlStoreError),
}

#[must_use = "retain and anchor the local GetTask disposition"]
pub(in crate::resident) struct PreparedA2aGetTaskLocalDisposition {
    basis: A2aGetTaskLocalDispositionBasis,
    disposition: ControlRecord,
}

/// Prepared bytes and exact physical candidate inputs. No carrier has started.
#[must_use = "retain and anchor the GetTask preparation before carrier release"]
pub(in crate::resident) struct PreparedA2aGetTaskPreparation<'a> {
    history: &'a RecoveredA2aTaskHistory,
    operation: &'a crate::resident::IssuedLocalOperation,
    prepared: PreparedA2aGetTaskReadback,
    preparation: ControlRecord,
    current_key: OperationCustodyIndexKey,
    current_predecessor: OperationCustodyIndexPredecessor,
    store: AuthorityStoreCoordinate,
    records: Vec<OperationCustodyRecord>,
    dependencies: OperationA2aDependencies,
    maximum_record_bytes: usize,
}

/// Signed checkpoint candidate whose database and anchor custody remain pending.
#[must_use = "commit and anchor the GetTask preparation before carrier release"]
pub(in crate::resident) struct PreparedA2aGetTaskCustody<'a> {
    physical: PreparedOperationObservationCustody,
    release: A2aGetTaskRelease<'a>,
}

/// Database-committed preparation awaiting independent anchoring.
#[must_use = "advance the independent anchor before carrier release"]
pub(in crate::resident) struct DatabaseCommittedA2aGetTaskAnchorPending<'a> {
    physical: DatabaseCommittedOperationObservationAnchorPending,
    release: A2aGetTaskRelease<'a>,
}

/// Same-process retained preparation. Cold recovery cannot construct this type.
#[must_use = "run fresh preflight and start the prepared exchange at most once"]
pub(in crate::resident) struct RetainedA2aGetTask<'a> {
    custody: CompletedOperationObservationCustody,
    release: A2aGetTaskRelease<'a>,
}

/// Started carrier read paired with the request custody that authorized it.
pub(in crate::resident) struct StartedRetainedA2aGetTask {
    in_flight: InFlightA2aGetTaskReadback,
    custody: CompletedOperationObservationCustody,
    disposition_basis: A2aGetTaskLocalDispositionBasis,
}

impl StartedRetainedA2aGetTask {
    pub(in crate::resident) fn into_parts(
        self,
    ) -> (
        InFlightA2aGetTaskReadback,
        CompletedOperationObservationCustody,
        A2aGetTaskLocalDispositionBasis,
    ) {
        (self.in_flight, self.custody, self.disposition_basis)
    }
}

/// A retained preflight failure or a closed carrier-start failure.
pub(in crate::resident) enum A2aGetTaskStartFailure<'a> {
    Preflight {
        retained: Box<RetainedA2aGetTask<'a>>,
        reason: A2aGetTaskReadbackError,
    },
    Started {
        reason: Box<A2aGetTaskReadbackError>,
        custody: Box<CompletedOperationObservationCustody>,
        disposition_basis: Box<A2aGetTaskLocalDispositionBasis>,
    },
}

pub(in crate::resident) struct A2aGetTaskRelease<'a> {
    history: &'a RecoveredA2aTaskHistory,
    operation: &'a crate::resident::IssuedLocalOperation,
    prepared: PreparedA2aGetTaskReadback,
    preparation: ControlRecordCoordinate,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    maximum_record_bytes: usize,
}

enum A2aGetTaskCustodyFailurePhase {
    Commit(OperationObservationCustodyCommitFailure),
    Anchor(OperationObservationCustodyAnchorFailure),
    RecoveryOnly(CompletedOperationObservationCustody),
}

/// Exact custody continuation with the unopened same-process release retained.
pub(in crate::resident) struct A2aGetTaskCustodyFailure<'a> {
    release: Box<A2aGetTaskRelease<'a>>,
    phase: A2aGetTaskCustodyFailurePhase,
}

/// Build exact request bytes and their private preparation record without I/O.
pub(in crate::resident) fn prepare_a2a_get_task_custody<'a, Authorize>(
    input: A2aGetTaskPreparationInput<'a>,
    port: &dyn A2aExchangePort,
    clock: &dyn WallClockSourcePort,
    authorize: Authorize,
) -> Result<PreparedA2aGetTaskPreparation<'a>, A2aGetTaskReadbackError>
where
    Authorize:
        FnOnce(A2aGetTaskReadbackAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
{
    if input.maximum_record_bytes == 0
        || input.preparation_ref == input.readback.request_record_ref
        || input.preparation_ref == input.readback.response_record_ref
    {
        return Err(A2aGetTaskReadbackError::Association);
    }
    let authorized_at = clock.observe_unix_time()?;
    let authorized = authorize_a2a_get_task_readback(input.readback, authorized_at, authorize)?;
    let operation = authorized.attempt.operation();
    let attempt_id = authorized.attempt.attempt_id().clone();
    let history = authorized.history;
    let prepared = authorized.prepare(port)?;
    let coordinate = prepared.prepared.coordinate();
    let limits = finite_limits(coordinate.limits())?;
    let request_evidence = ImmutableRecordReferenceV1 {
        record_ref: prepared.request_evidence.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::parse(
            prepared
                .request_evidence
                .coordinate()
                .record_sha256()
                .as_str(),
        )
        .map_err(|_| A2aGetTaskReadbackError::Association)?,
    };
    let basis = &prepared.basis;
    let binding = history.binding();
    let preparation = A2aGetTaskPreparation {
        operation_handle_ref: basis.operation.0.clone(),
        operation_handle_sha256: basis.operation.1.clone(),
        dispatch_ref: basis.dispatch.0.clone(),
        dispatch_sha256: basis.dispatch.1.clone(),
        attempt_selection_ref: basis.selection.record_ref().clone(),
        attempt_selection_sha256: ArtifactDigest::parse(basis.selection.record_sha256().as_str())
            .map_err(|_| A2aGetTaskReadbackError::Association)?,
        attempt_id,
        route_admission_ref: basis.route.0.clone(),
        route_admission_sha256: basis.route.1.clone(),
        task_binding_ref: basis.binding.0.clone(),
        task_binding_sha256: basis.binding.1.clone(),
        task_binding_id: binding.1.task_binding_id().clone(),
        local_head_ref: basis.local_head.0.clone(),
        local_head_sha256: basis.local_head.1.clone(),
        remote_head_ref: basis.remote_head.0.clone(),
        remote_head_sha256: basis.remote_head.1.clone(),
        authorized_at,
        json_rpc_request_id: coordinate.context().request_id().clone(),
        request_evidence,
        reserved_response_ref: prepared.response_record_ref.clone(),
        request_sha256: coordinate.request_sha256().clone(),
        tls_trust_input_sha256: coordinate.tls_trust_input_sha256().clone(),
        limits,
    }
    .record(input.preparation_ref, input.maximum_record_bytes)?;
    validate_current_predecessor(&input.current_predecessor, history, &preparation)?;
    let current_key = binding_key(operation.handle().1, binding.1.task_binding_id())?;
    let retained_records = retained_parents(history, operation, input.maximum_record_bytes)?;
    let retained_request_evidence = prepared_request_evidence(&prepared)?;
    Ok(PreparedA2aGetTaskPreparation {
        history,
        operation,
        prepared,
        preparation: preparation.clone(),
        current_key,
        current_predecessor: input.current_predecessor,
        store: history.local_observation().store().clone(),
        records: vec![OperationCustodyRecord::A2aGetTaskPreparation(preparation)],
        dependencies: OperationA2aDependencies {
            new_evidence: vec![retained_request_evidence],
            retained_evidence: Vec::new(),
            retained_records,
            associated_route: None,
        },
        maximum_record_bytes: input.maximum_record_bytes,
    })
}

impl A2aGetTaskLocalDispositionBasis {
    pub(in crate::resident) fn prepare(
        self,
        disposition_ref: ResidentRecordRefV1,
        disposition: A2aGetTaskLocalDispositionKind,
        recorded_at: UnixTimeV1,
    ) -> Result<PreparedA2aGetTaskLocalDisposition, A2aGetTaskReadbackError> {
        if disposition_ref == *self.preparation.coordinate().record_ref()
            || recorded_at < self.earliest_recorded_at
            || recorded_at > self.retain_until
        {
            return Err(A2aGetTaskReadbackError::Association);
        }
        let value = A2aGetTaskLocalDisposition {
            preparation_ref: self.preparation.coordinate().record_ref().clone(),
            preparation_sha256: ArtifactDigest::parse(
                self.preparation.coordinate().record_sha256().as_str(),
            )
            .map_err(|_| A2aGetTaskReadbackError::Association)?,
            task_binding_ref: self.binding_ref.clone(),
            task_binding_sha256: self.binding_sha256.clone(),
            task_binding_id: self.binding_id.clone(),
            disposition,
            recorded_at,
        };
        let record = private_record(
            &value,
            disposition_ref,
            PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA,
            self.maximum_record_bytes,
        )?;
        Ok(PreparedA2aGetTaskLocalDisposition {
            basis: self,
            disposition: record,
        })
    }
}

pub(in crate::resident) fn recover_a2a_get_task_custody(
    history: &RecoveredA2aTaskHistory,
    operation: &crate::resident::IssuedLocalOperation,
    maximum_record_bytes: usize,
    custody: &mut dyn OperationCustodyPort,
) -> Result<RecoveredA2aGetTaskCustody, A2aGetTaskCustodyRecoveryError> {
    if maximum_record_bytes == 0 {
        return Err(A2aGetTaskCustodyRecoveryError::Association);
    }
    let binding = history.binding();
    let key = binding_key(operation.handle().1, binding.1.task_binding_id())
        .map_err(|_| A2aGetTaskCustodyRecoveryError::Association)?;
    let observation = custody.read_index(
        history.local_observation().store(),
        &OperationCustodyIndexQuery::CurrentA2aGetTask(key),
        maximum_record_bytes,
    )?;
    let Some(record) = observation.into_record() else {
        return Ok(RecoveredA2aGetTaskCustody::Absent);
    };
    match record.schema_id().as_str() {
        A2A_TASK_OBSERVATION_V1_SCHEMA => {
            let current = history.observation();
            if record.coordinate().record_ref() != current.0
                || record.coordinate().record_sha256().as_str()
                    != current.1.canonical_a2a_task_observation_sha256().as_str()
            {
                return Err(A2aGetTaskCustodyRecoveryError::Association);
            }
            Ok(RecoveredA2aGetTaskCustody::PublicObservation(record))
        }
        PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA => {
            let preparation: A2aGetTaskPreparation = admit_private_record(
                &record,
                PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
            )?;
            validate_recovered_preparation(&preparation, &record, history, operation)?;
            Ok(RecoveredA2aGetTaskCustody::Preparation(
                recover_private_checkpoint(record, maximum_record_bytes, custody, history)?,
            ))
        }
        PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA => {
            let disposition: A2aGetTaskLocalDisposition = admit_private_record(
                &record,
                PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA,
            )?;
            validate_recovered_disposition(&disposition, &record, history, operation)?;
            Ok(RecoveredA2aGetTaskCustody::LocalDisposition(
                recover_private_checkpoint(record, maximum_record_bytes, custody, history)?,
            ))
        }
        _ => Err(A2aGetTaskCustodyRecoveryError::Association),
    }
}

fn recover_private_checkpoint(
    record: ControlRecord,
    maximum_record_bytes: usize,
    custody: &mut dyn OperationCustodyPort,
    history: &RecoveredA2aTaskHistory,
) -> Result<RecoveredA2aGetTaskPrivateRecord, A2aGetTaskCustodyRecoveryError> {
    let key = OperationCustodyIndexKey::for_a2a_get_task_checkpoint(record.coordinate());
    let checkpoint = custody
        .read_index(
            history.local_observation().store(),
            &OperationCustodyIndexQuery::A2aGetTaskCheckpoint(key),
            maximum_record_bytes,
        )?
        .into_record()
        .ok_or(A2aGetTaskCustodyRecoveryError::Missing)?;
    Ok(RecoveredA2aGetTaskPrivateRecord { record, checkpoint })
}

fn admit_private_record<T: DeserializeOwned + Serialize + JsonSchema>(
    record: &ControlRecord,
    schema: &str,
) -> Result<T, A2aGetTaskCustodyRecoveryError> {
    if record.schema_id().as_str() != schema
        || record.schema_sha256()
            != &private_schema_digest::<T>()
                .map_err(|_| A2aGetTaskCustodyRecoveryError::Association)?
        || pale_core::sha256_hex(record.canonical_record_jcs().as_slice())
            != record.coordinate().record_sha256().as_str()
    {
        return Err(A2aGetTaskCustodyRecoveryError::Association);
    }
    let value: T = serde_json::from_slice(record.canonical_record_jcs().as_slice())
        .map_err(|_| A2aGetTaskCustodyRecoveryError::Association)?;
    if pale_core::canon::to_canonical_string(&value)
        .map_err(|_| A2aGetTaskCustodyRecoveryError::Association)?
        .as_bytes()
        != record.canonical_record_jcs().as_slice()
    {
        return Err(A2aGetTaskCustodyRecoveryError::Association);
    }
    Ok(value)
}

fn validate_recovered_preparation(
    preparation: &A2aGetTaskPreparation,
    record: &ControlRecord,
    history: &RecoveredA2aTaskHistory,
    operation: &crate::resident::IssuedLocalOperation,
) -> Result<(), A2aGetTaskCustodyRecoveryError> {
    let handle = operation.handle();
    let dispatch = history.dispatch();
    let local = history.local_observation().selected_head();
    let route = history.route();
    let binding = history.binding();
    let remote = history.observation();
    let (attempt_id, selection) = match dispatch.1.state() {
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::Dispatched {
                    attempt_id,
                    evidence,
                },
        } => (attempt_id, evidence),
        _ => return Err(A2aGetTaskCustodyRecoveryError::Association),
    };
    let finite = &preparation.limits;
    let valid = preparation.operation_handle_ref == *handle.0
        && preparation.operation_handle_sha256 == *handle.1.canonical_operation_handle_sha256()
        && preparation.dispatch_ref == *dispatch.0
        && preparation.dispatch_sha256 == *dispatch.1.canonical_operation_observation_sha256()
        && preparation.attempt_selection_ref == selection.record_ref
        && preparation.attempt_selection_sha256 == selection.record_sha256
        && preparation.attempt_id == *attempt_id
        && preparation.route_admission_ref == *route.0
        && preparation.route_admission_sha256 == *route.1.canonical_a2a_route_admission_sha256()
        && preparation.task_binding_ref == *binding.0
        && preparation.task_binding_sha256 == *binding.1.canonical_a2a_task_binding_sha256()
        && preparation.task_binding_id == *binding.1.task_binding_id()
        && preparation.local_head_ref == *local.0
        && preparation.local_head_sha256 == *local.1.canonical_operation_observation_sha256()
        && preparation.remote_head_ref == *remote.0
        && preparation.remote_head_sha256 == *remote.1.canonical_a2a_task_observation_sha256()
        && preparation.request_evidence.record_sha256 == preparation.request_sha256
        && preparation.authorized_at >= *remote.1.observed_at()
        && preparation.authorized_at <= *handle.1.retain_until()
        && preparation.request_evidence.record_ref != preparation.reserved_response_ref
        && preparation.request_evidence.record_ref != *record.coordinate().record_ref()
        && preparation.reserved_response_ref != *record.coordinate().record_ref()
        && [
            finite.request_bytes,
            finite.response_bytes,
            finite.evidence_bytes,
            finite.line_bytes,
            finite.event_bytes,
            finite.events,
            finite.queued_events,
            finite.connect_timeout_nanos,
            finite.read_timeout_nanos,
            finite.total_timeout_nanos,
        ]
        .into_iter()
        .all(|value| value > 0);
    valid
        .then_some(())
        .ok_or(A2aGetTaskCustodyRecoveryError::Association)
}

fn validate_recovered_disposition(
    disposition: &A2aGetTaskLocalDisposition,
    record: &ControlRecord,
    history: &RecoveredA2aTaskHistory,
    operation: &crate::resident::IssuedLocalOperation,
) -> Result<(), A2aGetTaskCustodyRecoveryError> {
    let binding = history.binding();
    let valid = disposition.preparation_ref != *record.coordinate().record_ref()
        && disposition.task_binding_ref == *binding.0
        && disposition.task_binding_sha256 == *binding.1.canonical_a2a_task_binding_sha256()
        && disposition.task_binding_id == *binding.1.task_binding_id()
        && disposition.recorded_at >= *history.observation().1.observed_at()
        && disposition.recorded_at <= *operation.handle().1.retain_until();
    valid
        .then_some(())
        .ok_or(A2aGetTaskCustodyRecoveryError::Association)
}

impl<'a> PreparedA2aGetTaskPreparation<'a> {
    /// Build, sign, and close the exact physical checkpoint candidate.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_custody<Sign>(
        self,
        identity: OperationCheckpointInput,
        issued_at: UnixTimeV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        custody: &mut dyn OperationCustodyPort,
        sign: Sign,
    ) -> Result<PreparedA2aGetTaskCustody<'a>, A2aGetTaskReadbackError>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let selection = self.operation.checkpoint_selection();
        let (predecessor_ref, predecessor) = self.history.local_observation().current_checkpoint();
        selection
            .verify(&self.store, self.operation.handle().1, predecessor)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let prepared = self.prepare_checkpoint(
            selection,
            predecessor_ref,
            predecessor,
            limits,
            membership_limits,
            custody,
        )?;
        prepared.issue(identity, issued_at, schema, authority, sign)
    }

    fn prepare_checkpoint(
        self,
        selection: OperationCheckpointSelection,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &ControlStateCheckpointV1,
        limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedGetTaskCheckpoint<'a>, A2aGetTaskReadbackError> {
        let prior = OperationAccumulatorCheckpoint::new(
            self.store.clone(),
            predecessor.checkpoint_profile_ref().clone(),
            predecessor.checkpoint_profile_sha256().clone(),
            checkpoint_head(predecessor_ref, predecessor)?,
            predecessor.state_root_sha256().clone(),
            predecessor.unresolved_custody_root_sha256().clone(),
        );
        let mut changes = Vec::new();
        let mut changed_keys = Vec::new();
        let preparation_leaf = a2a_record_leaf(
            &self.store,
            predecessor,
            "a2a-get-task-preparation",
            self.preparation.coordinate().record_ref().as_str(),
            self.preparation.coordinate(),
        )
        .map_err(leaf_error)?;
        push_change(preparation_leaf, None, &mut changes, &mut changed_keys)?;
        let current_leaf = a2a_record_leaf(
            &self.store,
            predecessor,
            "a2a-get-task-current",
            self.current_key.as_str(),
            self.preparation.coordinate(),
        )
        .map_err(leaf_error)?;
        let old = match &self.current_predecessor {
            OperationCustodyIndexPredecessor::Absent => None,
            OperationCustodyIndexPredecessor::Exact(record) => Some(
                a2a_record_leaf(
                    &self.store,
                    predecessor,
                    "a2a-get-task-current",
                    self.current_key.as_str(),
                    record.coordinate(),
                )
                .map_err(leaf_error)?
                .value,
            ),
        };
        push_change(current_leaf, old, &mut changes, &mut changed_keys)?;
        let evidence_leaf =
            a2a_evidence_leaf(&self.store, predecessor, &self.prepared.request_evidence)
                .map_err(leaf_error)?;
        push_change(evidence_leaf, None, &mut changes, &mut changed_keys)?;
        let mut retained = retained_membership(&self, predecessor)?;
        retained.sort_by(|left, right| left.key().cmp(right.key()));
        let membership = OperationAccumulatorMembership::new(
            prior.clone(),
            retained,
            membership_limits,
            selection.implementation(),
        )?;
        let verified = custody
            .verify_accumulator_membership(&membership)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        if verified.request() != &membership {
            return Err(A2aGetTaskReadbackError::Association);
        }
        changes.sort_by(|left, right| left.key().cmp(right.key()));
        changed_keys.sort();
        let inserted = 2 + u64::from(matches!(
            self.current_predecessor,
            OperationCustodyIndexPredecessor::Absent
        ));
        let leaf_count = ControlLeafCountV1::new(
            predecessor
                .leaf_count()
                .get()
                .checked_add(inserted)
                .ok_or(A2aGetTaskReadbackError::Association)?,
        )?;
        let state = StateLeafChanges::new(changes, limits.maximum_leaf_changes())
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let unresolved =
            UnresolvedCustodyLeafChanges::new(Vec::new(), limits.maximum_leaf_changes())
                .map_err(OperationObservationCustodyProjectionError::from)?;
        let transaction = custody
            .prepare_transaction(&self.store)
            .map_err(OperationAccumulatorPreparationError::from)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let request = OperationAccumulatorPreparation::new(
            AccumulatorPreparationBasis::Successor(prior),
            state,
            unresolved,
            limits,
            selection.implementation(),
            transaction,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let accumulator = custody
            .prepare_accumulator(&request)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        if accumulator.request() != &request {
            return Err(A2aGetTaskReadbackError::Association);
        }
        Ok(PreparedGetTaskCheckpoint {
            append: self,
            selection,
            predecessor_ref: predecessor_ref.clone(),
            predecessor: predecessor.clone(),
            changed_keys,
            leaf_count,
            accumulator,
        })
    }
}

impl PreparedA2aGetTaskLocalDisposition {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_custody<Sign>(
        self,
        request_checkpoint: &(ResidentRecordRefV1, ControlStateCheckpointV1),
        request_custody: &CompletedOperationObservationCustody,
        identity: OperationCheckpointInput,
        issued_at: UnixTimeV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn OperationCustodyPort,
        sign: Sign,
    ) -> Result<PreparedOperationObservationCustody, A2aGetTaskReadbackError>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let (predecessor_ref, predecessor) = request_checkpoint;
        let preparation = request_custody
            .candidate()
            .record(OperationCustodyRecordKind::A2aGetTaskPreparation)
            .ok_or(A2aGetTaskReadbackError::Association)?;
        if request_custody.candidate().store() != &self.basis.store
            || preparation.coordinate() != self.basis.preparation.coordinate()
            || request_custody
                .candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref()
                != predecessor_ref
            || request_custody
                .candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_sha256()
                .as_str()
                != predecessor.canonical_control_checkpoint_sha256().as_str()
        {
            return Err(A2aGetTaskReadbackError::Association);
        }
        let selection = self.basis.selection;
        selection
            .matches_store(&self.basis.store)
            .then_some(())
            .ok_or(A2aGetTaskReadbackError::Association)?;
        let prior = OperationAccumulatorCheckpoint::new(
            self.basis.store.clone(),
            predecessor.checkpoint_profile_ref().clone(),
            predecessor.checkpoint_profile_sha256().clone(),
            checkpoint_head(predecessor_ref, predecessor)?,
            predecessor.state_root_sha256().clone(),
            predecessor.unresolved_custody_root_sha256().clone(),
        );
        let mut changes = Vec::new();
        let mut changed_keys = Vec::new();
        push_change(
            a2a_record_leaf(
                &self.basis.store,
                predecessor,
                "a2a-get-task-local-disposition",
                self.disposition.coordinate().record_ref().as_str(),
                self.disposition.coordinate(),
            )
            .map_err(leaf_error)?,
            None,
            &mut changes,
            &mut changed_keys,
        )?;
        push_change(
            a2a_record_leaf(
                &self.basis.store,
                predecessor,
                "a2a-get-task-current",
                self.basis.current_key.as_str(),
                self.disposition.coordinate(),
            )
            .map_err(leaf_error)?,
            Some(
                a2a_record_leaf(
                    &self.basis.store,
                    predecessor,
                    "a2a-get-task-current",
                    self.basis.current_key.as_str(),
                    self.basis.preparation.coordinate(),
                )
                .map_err(leaf_error)?
                .value,
            ),
            &mut changes,
            &mut changed_keys,
        )?;
        changes.sort_by(|left, right| left.key().cmp(right.key()));
        changed_keys.sort();
        let leaf_count = ControlLeafCountV1::new(
            predecessor
                .leaf_count()
                .get()
                .checked_add(1)
                .ok_or(A2aGetTaskReadbackError::Association)?,
        )?;
        let state = StateLeafChanges::new(changes, limits.maximum_leaf_changes())
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let unresolved =
            UnresolvedCustodyLeafChanges::new(Vec::new(), limits.maximum_leaf_changes())
                .map_err(OperationObservationCustodyProjectionError::from)?;
        let transaction = custody
            .prepare_transaction(&self.basis.store)
            .map_err(OperationAccumulatorPreparationError::from)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let request = OperationAccumulatorPreparation::new(
            AccumulatorPreparationBasis::Successor(prior),
            state,
            unresolved,
            limits,
            selection.implementation(),
            transaction,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let accumulator = custody
            .prepare_accumulator(&request)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        if accumulator.request() != &request {
            return Err(A2aGetTaskReadbackError::Association);
        }
        let sequence = predecessor
            .domain_sequence()
            .get()
            .checked_add(1)
            .ok_or(A2aGetTaskReadbackError::Association)?;
        let preimage = ControlStateCheckpointPreimageV1 {
            schema_id: predecessor.schema_id().clone(),
            schema_sha256: schema.clone(),
            control_checkpoint_id: identity.id.clone(),
            control_store_id: authority.control_store_id.clone(),
            control_incarnation_id: authority.control_incarnation_id.clone(),
            checkpoint_domain_id: authority.checkpoint_domain_id.clone(),
            domain_epoch: *predecessor.domain_epoch(),
            domain_sequence: ControlDomainSequenceV1::new(sequence)?,
            previous_control_checkpoint_ref: Some(predecessor_ref.clone()),
            previous_control_checkpoint_sha256: Some(
                predecessor.canonical_control_checkpoint_sha256().clone(),
            ),
            checkpoint_profile_ref: authority.checkpoint_profile_ref.clone(),
            checkpoint_profile_sha256: authority.checkpoint_profile_sha256.clone(),
            control_schema_version: *predecessor.control_schema_version(),
            committed_transaction_sequence: accumulator.request().transaction().successor(),
            state_root_sha256: accumulator.state_root().clone(),
            leaf_count,
            changed_leaf_keys: changed_keys,
            unresolved_custody_root_sha256: accumulator.unresolved_root().clone(),
            issued_at,
            checkpoint_signing_authority_ref: authority.authority_ref.clone(),
            checkpoint_signing_authority_sha256: authority.authority_sha256.clone(),
            issuer_principal_id: authority.principal_id.clone(),
            issuer_key_id: authority.subject_key_id.clone(),
            issuer_key_epoch: authority.subject_key_epoch,
        };
        if !selection.matches_authority(authority)
            || authority.principal_id != &self.basis.issuer_principal_id
        {
            return Err(A2aGetTaskReadbackError::Association);
        }
        let successor = ControlStateCheckpointV1::issue_successor_contextual(
            preimage,
            predecessor_ref,
            predecessor,
            schema,
            authority,
            sign,
        )?;
        let successor_head = checkpoint_head(&identity.reference, &successor)?;
        let checkpoint = OperationCustodyCheckpoint::new(
            OperationCheckpointPhase::Successor {
                predecessor: checkpoint_head(predecessor_ref, predecessor)?,
                predecessor_record: Box::new(checkpoint_record(
                    predecessor_ref,
                    predecessor,
                    self.basis.maximum_record_bytes,
                )?),
            },
            successor_head.clone(),
            checkpoint_record(
                &identity.reference,
                &successor,
                self.basis.maximum_record_bytes,
            )?,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let batch = accumulator
            .bind_successor(successor_head)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let candidate = OperationCustodyCandidate::new_a2a_custody(
            self.basis.store,
            OperationCustodyTransitionKind::DisposeA2aGetTask,
            vec![OperationCustodyRecord::A2aGetTaskLocalDisposition(
                self.disposition.clone(),
            )],
            vec![OperationCustodyIndexMutation::CurrentA2aGetTask {
                key: self.basis.current_key,
                expected: OperationCustodyIndexPredecessor::Exact(self.basis.preparation),
                successor: self.disposition.coordinate().clone(),
            }],
            checkpoint,
            batch,
            OperationA2aDependencies {
                new_evidence: Vec::new(),
                retained_evidence: Vec::new(),
                retained_records: Vec::new(),
                associated_route: None,
            },
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        prepare_operation_observation_candidate(candidate).map_err(|error| match error {
            OperationObservationCandidatePreparationError::NotObservation => {
                A2aGetTaskReadbackError::Association
            }
            OperationObservationCandidatePreparationError::Anchor(error) => {
                OperationObservationCustodyProjectionError::from(error).into()
            }
        })
    }
}

struct PreparedGetTaskCheckpoint<'a> {
    append: PreparedA2aGetTaskPreparation<'a>,
    selection: OperationCheckpointSelection,
    predecessor_ref: ResidentRecordRefV1,
    predecessor: ControlStateCheckpointV1,
    changed_keys: Vec<ChangedControlLeafKeyV1>,
    leaf_count: ControlLeafCountV1,
    accumulator: PreparedOperationAccumulator,
}

impl<'a> PreparedGetTaskCheckpoint<'a> {
    fn issue<Sign>(
        self,
        identity: OperationCheckpointInput,
        issued_at: UnixTimeV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<PreparedA2aGetTaskCustody<'a>, A2aGetTaskReadbackError>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let sequence = self
            .predecessor
            .domain_sequence()
            .get()
            .checked_add(1)
            .ok_or(A2aGetTaskReadbackError::Association)?;
        let preimage = ControlStateCheckpointPreimageV1 {
            schema_id: self.predecessor.schema_id().clone(),
            schema_sha256: schema.clone(),
            control_checkpoint_id: identity.id.clone(),
            control_store_id: authority.control_store_id.clone(),
            control_incarnation_id: authority.control_incarnation_id.clone(),
            checkpoint_domain_id: authority.checkpoint_domain_id.clone(),
            domain_epoch: *self.predecessor.domain_epoch(),
            domain_sequence: ControlDomainSequenceV1::new(sequence)?,
            previous_control_checkpoint_ref: Some(self.predecessor_ref.clone()),
            previous_control_checkpoint_sha256: Some(
                self.predecessor
                    .canonical_control_checkpoint_sha256()
                    .clone(),
            ),
            checkpoint_profile_ref: authority.checkpoint_profile_ref.clone(),
            checkpoint_profile_sha256: authority.checkpoint_profile_sha256.clone(),
            control_schema_version: *self.predecessor.control_schema_version(),
            committed_transaction_sequence: self.accumulator.request().transaction().successor(),
            state_root_sha256: self.accumulator.state_root().clone(),
            unresolved_custody_root_sha256: self.accumulator.unresolved_root().clone(),
            changed_leaf_keys: self.changed_keys.clone(),
            leaf_count: self.leaf_count,
            issued_at,
            checkpoint_signing_authority_ref: authority.authority_ref.clone(),
            checkpoint_signing_authority_sha256: authority.authority_sha256.clone(),
            issuer_principal_id: authority.principal_id.clone(),
            issuer_key_id: authority.subject_key_id.clone(),
            issuer_key_epoch: authority.subject_key_epoch,
        };
        if !self.selection.matches_authority(authority)
            || authority.principal_id != self.append.operation.handle().1.issuer_principal_id()
        {
            return Err(A2aGetTaskReadbackError::Association);
        }
        let successor = ControlStateCheckpointV1::issue_successor_contextual(
            preimage,
            &self.predecessor_ref,
            &self.predecessor,
            schema,
            authority,
            sign,
        )?;
        let successor_head = checkpoint_head(&identity.reference, &successor)?;
        let checkpoint = OperationCustodyCheckpoint::new(
            OperationCheckpointPhase::Successor {
                predecessor: checkpoint_head(&self.predecessor_ref, &self.predecessor)?,
                predecessor_record: Box::new(checkpoint_record(
                    &self.predecessor_ref,
                    &self.predecessor,
                    self.append.maximum_record_bytes,
                )?),
            },
            successor_head.clone(),
            checkpoint_record(
                &identity.reference,
                &successor,
                self.append.maximum_record_bytes,
            )?,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let batch = self
            .accumulator
            .bind_successor(successor_head)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let release = A2aGetTaskRelease {
            history: self.append.history,
            operation: self.append.operation,
            prepared: self.append.prepared,
            preparation: self.append.preparation.coordinate().clone(),
            checkpoint: (identity.reference, successor),
            maximum_record_bytes: self.append.maximum_record_bytes,
        };
        let candidate = OperationCustodyCandidate::new_a2a_custody(
            self.append.store,
            OperationCustodyTransitionKind::PrepareA2aGetTask,
            self.append.records,
            vec![OperationCustodyIndexMutation::CurrentA2aGetTask {
                key: self.append.current_key,
                expected: self.append.current_predecessor,
                successor: self.append.preparation.coordinate().clone(),
            }],
            checkpoint,
            batch,
            self.append.dependencies,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let physical =
            prepare_operation_observation_candidate(candidate).map_err(|error| match error {
                OperationObservationCandidatePreparationError::NotObservation => {
                    A2aGetTaskReadbackError::Association
                }
                OperationObservationCandidatePreparationError::Anchor(error) => {
                    A2aGetTaskReadbackError::from(OperationObservationCustodyProjectionError::from(
                        error,
                    ))
                }
            })?;
        Ok(PreparedA2aGetTaskCustody { physical, release })
    }
}

impl<'a> PreparedA2aGetTaskCustody<'a> {
    pub(in crate::resident) fn commit(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<DatabaseCommittedA2aGetTaskAnchorPending<'a>, Box<A2aGetTaskCustodyFailure<'a>>>
    {
        let Self { physical, release } = self;
        match physical.commit(custody, anchor) {
            Ok(physical) => Ok(DatabaseCommittedA2aGetTaskAnchorPending { physical, release }),
            Err(failure) => Err(Box::new(A2aGetTaskCustodyFailure {
                release: Box::new(release),
                phase: A2aGetTaskCustodyFailurePhase::Commit(failure),
            })),
        }
    }
}

impl<'a> DatabaseCommittedA2aGetTaskAnchorPending<'a> {
    pub(in crate::resident) fn advance_anchor(
        self,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<RetainedA2aGetTask<'a>, Box<A2aGetTaskCustodyFailure<'a>>> {
        let Self { physical, release } = self;
        let database_evidence = physical.database_evidence();
        match physical.advance_anchor(anchor) {
            Ok(custody)
                if database_evidence == crate::resident::OperationCustodyDatabaseEvidence::New =>
            {
                Ok(RetainedA2aGetTask { custody, release })
            }
            Ok(custody) => Err(Box::new(A2aGetTaskCustodyFailure {
                release: Box::new(release),
                phase: A2aGetTaskCustodyFailurePhase::RecoveryOnly(custody),
            })),
            Err(failure) => Err(Box::new(A2aGetTaskCustodyFailure {
                release: Box::new(release),
                phase: A2aGetTaskCustodyFailurePhase::Anchor(failure),
            })),
        }
    }
}

impl<'a> A2aGetTaskCustodyFailure<'a> {
    /// Continue only a retry which the physical custody owner proves safe.
    pub(in crate::resident) fn continue_exact(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<RetainedA2aGetTask<'a>, Box<Self>> {
        let Self { release, phase } = self;
        let pending = match phase {
            A2aGetTaskCustodyFailurePhase::Commit(failure) => {
                match failure.reconcile_exact(custody) {
                    Ok(pending) => pending,
                    Err(failure) => match failure.try_into_retryable() {
                        Ok(prepared) => match prepared.commit(custody, anchor) {
                            Ok(pending) => pending,
                            Err(failure) => {
                                return Err(Box::new(Self {
                                    release,
                                    phase: A2aGetTaskCustodyFailurePhase::Commit(failure),
                                }));
                            }
                        },
                        Err(failure) => {
                            return Err(Box::new(Self {
                                release,
                                phase: A2aGetTaskCustodyFailurePhase::Commit(failure),
                            }));
                        }
                    },
                }
            }
            A2aGetTaskCustodyFailurePhase::Anchor(failure) => match failure.try_into_retryable() {
                Ok(pending) => pending,
                Err(failure) => match failure.continue_after_failed_observation(anchor) {
                    Ok(completed)
                        if completed.database_evidence()
                            == crate::resident::OperationCustodyDatabaseEvidence::New =>
                    {
                        return Ok(RetainedA2aGetTask {
                            custody: completed,
                            release: *release,
                        });
                    }
                    Ok(completed) => {
                        return Err(Box::new(Self {
                            release,
                            phase: A2aGetTaskCustodyFailurePhase::RecoveryOnly(completed),
                        }));
                    }
                    Err(failure) => {
                        return Err(Box::new(Self {
                            release,
                            phase: A2aGetTaskCustodyFailurePhase::Anchor(failure),
                        }));
                    }
                },
            },
            A2aGetTaskCustodyFailurePhase::RecoveryOnly(completed) => {
                return Err(Box::new(Self {
                    release,
                    phase: A2aGetTaskCustodyFailurePhase::RecoveryOnly(completed),
                }));
            }
        };
        let database_evidence = pending.database_evidence();
        match pending.advance_anchor(anchor) {
            Ok(completed)
                if database_evidence == crate::resident::OperationCustodyDatabaseEvidence::New =>
            {
                Ok(RetainedA2aGetTask {
                    custody: completed,
                    release: *release,
                })
            }
            Ok(completed) => Err(Box::new(Self {
                release,
                phase: A2aGetTaskCustodyFailurePhase::RecoveryOnly(completed),
            })),
            Err(failure) => Err(Box::new(Self {
                release,
                phase: A2aGetTaskCustodyFailurePhase::Anchor(failure),
            })),
        }
    }

    /// Extract custody recovered after an ambiguous commit without restoring carrier authority.
    pub(in crate::resident) fn into_non_dispatchable_custody(
        self,
    ) -> Result<CompletedOperationObservationCustody, Box<Self>> {
        let Self { release, phase } = self;
        match phase {
            A2aGetTaskCustodyFailurePhase::RecoveryOnly(custody) => Ok(custody),
            phase => Err(Box::new(Self { release, phase })),
        }
    }
}

impl<'a> RetainedA2aGetTask<'a> {
    /// Borrow the checkpoint that made this request eligible for release.
    #[must_use]
    pub(in crate::resident) fn request_checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.release.checkpoint.0, &self.release.checkpoint.1)
    }

    /// Start once while preserving the physical request custody for its response.
    // The error retains the exact unopened or carrier-started phase. Boxing the
    // error would add another allocation without reducing any owned custody.
    #[allow(clippy::result_large_err)]
    pub(in crate::resident) fn start_retained<Authorize>(
        self,
        port: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> Result<StartedRetainedA2aGetTask, A2aGetTaskStartFailure<'a>>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let now = match clock.observe_unix_time() {
            Ok(now) => now,
            Err(error) => {
                return Err(A2aGetTaskStartFailure::Preflight {
                    retained: Box::new(self),
                    reason: error.into(),
                });
            }
        };
        let limits = self.release.prepared.prepared.coordinate().limits();
        let history = self.release.history;
        let operation = self.release.operation;
        let binding = history.binding();
        let local_head = history.local_observation().selected_head();
        if !self
            .release
            .prepared
            .basis
            .matches_without_local_head(operation, history)
            || self.release.prepared.basis.local_head.0 != *local_head.0
            || self.release.prepared.basis.local_head.1
                != *local_head.1.canonical_operation_observation_sha256()
            || !self
                .release
                .prepared
                .basis
                .admits_exchange_deadline(now, limits.total_timeout)
        {
            return Err(A2aGetTaskStartFailure::Preflight {
                retained: Box::new(self),
                reason: A2aGetTaskReadbackError::Association,
            });
        }
        let context = A2aGetTaskReadbackAuthorizationContext {
            operation: operation.handle(),
            dispatch: history.dispatch(),
            local_head,
            route: history.route(),
            binding: history.binding(),
            remote_head: history.observation(),
            authorized_at: now,
        };
        if let Err(error) = authorize(context) {
            return Err(A2aGetTaskStartFailure::Preflight {
                retained: Box::new(self),
                reason: error.into(),
            });
        }
        let current_key = match binding_key(operation.handle().1, binding.1.task_binding_id()) {
            Ok(key) => key,
            Err(reason) => {
                return Err(A2aGetTaskStartFailure::Preflight {
                    retained: Box::new(self),
                    reason: A2aGetTaskReadbackError::Custody(reason),
                });
            }
        };
        let Self { custody, release } = self;
        let preparation = custody
            .candidate()
            .record(OperationCustodyRecordKind::A2aGetTaskPreparation)
            .filter(|record| record.coordinate() == &release.preparation)
            .cloned();
        let Some(preparation) = preparation else {
            return Err(A2aGetTaskStartFailure::Preflight {
                retained: Box::new(RetainedA2aGetTask { custody, release }),
                reason: A2aGetTaskReadbackError::Association,
            });
        };
        let disposition_basis = A2aGetTaskLocalDispositionBasis {
            selection: release.operation.checkpoint_selection(),
            issuer_principal_id: release.operation.handle().1.issuer_principal_id().clone(),
            preparation,
            binding_ref: binding.0.clone(),
            binding_sha256: binding.1.canonical_a2a_task_binding_sha256().clone(),
            binding_id: binding.1.task_binding_id().clone(),
            current_key,
            store: release.history.local_observation().store().clone(),
            earliest_recorded_at: *release.history.observation().1.observed_at(),
            retain_until: *release.operation.handle().1.retain_until(),
            maximum_record_bytes: release.maximum_record_bytes,
        };
        match release.prepared.start(port) {
            Ok(in_flight) => Ok(StartedRetainedA2aGetTask {
                in_flight,
                custody,
                disposition_basis,
            }),
            Err(reason) => Err(A2aGetTaskStartFailure::Started {
                reason: Box::new(reason),
                custody: Box::new(custody),
                disposition_basis: Box::new(disposition_basis),
            }),
        }
    }

    /// Return the exact completed custody that authorizes this same-process release.
    pub(in crate::resident) const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }
}

fn finite_limits(
    limits: A2aExchangeLimits,
) -> Result<A2aGetTaskFiniteLimits, A2aGetTaskReadbackError> {
    fn usize_u64(value: NonZeroUsize) -> Result<u64, A2aGetTaskReadbackError> {
        u64::try_from(value.get()).map_err(|_| A2aGetTaskReadbackError::Association)
    }
    fn nanos(value: Duration) -> Result<u64, A2aGetTaskReadbackError> {
        u64::try_from(value.as_nanos()).map_err(|_| A2aGetTaskReadbackError::Association)
    }
    Ok(A2aGetTaskFiniteLimits {
        request_bytes: usize_u64(limits.request_bytes)?,
        response_bytes: usize_u64(limits.response_bytes)?,
        evidence_bytes: usize_u64(limits.evidence_bytes)?,
        line_bytes: usize_u64(limits.line_bytes)?,
        event_bytes: usize_u64(limits.event_bytes)?,
        events: usize_u64(limits.events)?,
        queued_events: usize_u64(limits.queued_events)?,
        connect_timeout_nanos: nanos(limits.connect_timeout)?,
        read_timeout_nanos: nanos(limits.read_timeout)?,
        total_timeout_nanos: nanos(limits.total_timeout)?,
    })
}

impl A2aGetTaskPreparation {
    fn record(
        &self,
        reference: ResidentRecordRefV1,
        maximum_bytes: usize,
    ) -> Result<ControlRecord, A2aGetTaskReadbackError> {
        let bytes = pale_core::canon::to_canonical_string(self)
            .map_err(OperationObservationCustodyProjectionError::from)?
            .into_bytes();
        if bytes.len() > maximum_bytes {
            return Err(A2aGetTaskReadbackError::Association);
        }
        Ok(ControlRecord::new(
            ControlRecordCoordinate::new(
                reference,
                ControlRecordDigest::parse(pale_core::sha256_hex(&bytes))
                    .map_err(OperationObservationCustodyProjectionError::from)?,
            ),
            SchemaId::parse(PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA)
                .map_err(|_| A2aGetTaskReadbackError::Association)?,
            private_schema_digest::<Self>()?,
            ControlRecordBytes::new(bytes)
                .map_err(OperationObservationCustodyProjectionError::from)?,
        ))
    }
}

fn private_schema_digest<T: JsonSchema>() -> Result<SchemaDigest, A2aGetTaskReadbackError> {
    let schema = schemars::schema_for!(T);
    let canonical = pale_core::canon::to_canonical_string(&schema)
        .map_err(OperationObservationCustodyProjectionError::from)?;
    SchemaDigest::parse(pale_core::sha256_hex(canonical.as_bytes()))
        .map_err(|_| A2aGetTaskReadbackError::Association)
}

fn private_record<T: Serialize + JsonSchema>(
    value: &T,
    reference: ResidentRecordRefV1,
    schema_id: &str,
    maximum_bytes: usize,
) -> Result<ControlRecord, A2aGetTaskReadbackError> {
    let bytes = pale_core::canon::to_canonical_string(value)
        .map_err(OperationObservationCustodyProjectionError::from)?
        .into_bytes();
    if bytes.len() > maximum_bytes {
        return Err(A2aGetTaskReadbackError::Association);
    }
    Ok(ControlRecord::new(
        ControlRecordCoordinate::new(
            reference,
            ControlRecordDigest::parse(pale_core::sha256_hex(&bytes))
                .map_err(OperationObservationCustodyProjectionError::from)?,
        ),
        SchemaId::parse(schema_id).map_err(|_| A2aGetTaskReadbackError::Association)?,
        private_schema_digest::<T>()?,
        ControlRecordBytes::new(bytes).map_err(OperationObservationCustodyProjectionError::from)?,
    ))
}

fn prepared_request_evidence(
    prepared: &PreparedA2aGetTaskReadback,
) -> Result<OperationA2aEvidence, A2aGetTaskReadbackError> {
    OperationA2aEvidence::new(
        ImmutableRecordReferenceV1 {
            record_ref: prepared.request_evidence.coordinate().record_ref().clone(),
            record_sha256: ArtifactDigest::parse(
                prepared
                    .request_evidence
                    .coordinate()
                    .record_sha256()
                    .as_str(),
            )
            .map_err(|_| A2aGetTaskReadbackError::Association)?,
        },
        prepared.request_evidence.bytes().to_vec(),
        prepared
            .request_evidence
            .sse()
            .map(|(id, kind)| (id.to_owned(), kind.to_owned())),
        prepared.maximum_evidence_bytes,
    )
    .map_err(Into::into)
}

fn retained_parents(
    history: &RecoveredA2aTaskHistory,
    operation: &crate::resident::IssuedLocalOperation,
    maximum: usize,
) -> Result<Vec<ControlRecord>, A2aGetTaskReadbackError> {
    let mut records = Vec::new();
    let handle = operation.handle();
    records.push(project(
        handle.0,
        handle.1.canonical_operation_handle_sha256().as_str(),
        handle.1,
        maximum,
    )?);
    let dispatch = history.dispatch();
    records.push(project(
        dispatch.0,
        dispatch.1.canonical_operation_observation_sha256().as_str(),
        dispatch.1,
        maximum,
    )?);
    let local = history.local_observation().selected_head();
    if local.0 != dispatch.0 {
        records.push(project(
            local.0,
            local.1.canonical_operation_observation_sha256().as_str(),
            local.1,
            maximum,
        )?);
    }
    let route = history.route();
    records.push(project(
        route.0,
        route.1.canonical_a2a_route_admission_sha256().as_str(),
        route.1,
        maximum,
    )?);
    let binding = history.binding();
    records.push(project(
        binding.0,
        binding.1.canonical_a2a_task_binding_sha256().as_str(),
        binding.1,
        maximum,
    )?);
    let remote = history.observation();
    records.push(project(
        remote.0,
        remote.1.canonical_a2a_task_observation_sha256().as_str(),
        remote.1,
        maximum,
    )?);
    Ok(records)
}

fn project<T: Serialize + record_projection::ResidentRecordSchema>(
    reference: &ResidentRecordRefV1,
    digest: &str,
    value: &T,
    maximum: usize,
) -> Result<ControlRecord, A2aGetTaskReadbackError> {
    let coordinate = record_projection::coordinate(reference, digest)
        .map_err(OperationObservationCustodyProjectionError::from)?;
    let record = record_projection::record(coordinate, value)
        .map_err(OperationObservationCustodyProjectionError::from)?;
    if record.canonical_record_jcs().as_slice().len() > maximum {
        return Err(A2aGetTaskReadbackError::Association);
    }
    Ok(record)
}

fn validate_current_predecessor(
    predecessor: &OperationCustodyIndexPredecessor,
    history: &RecoveredA2aTaskHistory,
    preparation: &ControlRecord,
) -> Result<(), A2aGetTaskReadbackError> {
    let valid = match predecessor {
        OperationCustodyIndexPredecessor::Absent => true,
        OperationCustodyIndexPredecessor::Exact(record)
            if record.schema_id().as_str() == A2A_TASK_OBSERVATION_V1_SCHEMA =>
        {
            let current = history.observation();
            record.coordinate().record_ref() == current.0
                && record.coordinate().record_sha256().as_str()
                    == current.1.canonical_a2a_task_observation_sha256().as_str()
        }
        OperationCustodyIndexPredecessor::Exact(record) => {
            record.schema_id().as_str()
                == PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA
                && record.coordinate().record_ref() != preparation.coordinate().record_ref()
        }
    };
    if valid {
        Ok(())
    } else {
        Err(A2aGetTaskReadbackError::Association)
    }
}

fn retained_membership(
    append: &PreparedA2aGetTaskPreparation<'_>,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<Vec<AccumulatorMembershipLeaf>, A2aGetTaskReadbackError> {
    let key = append.current_key.as_str();
    let mut leaves = Vec::new();
    for record in &append.dependencies.retained_records {
        let (class, logical) = match record.schema_id().as_str() {
            A2A_ROUTE_ADMISSION_V1_SCHEMA => (
                "a2a-route-admission",
                record.coordinate().record_ref().as_str(),
            ),
            A2A_TASK_BINDING_V1_SCHEMA => ("a2a-task-binding", key),
            A2A_TASK_OBSERVATION_V1_SCHEMA => ("a2a-task-observation", key),
            OPERATION_HANDLE_V1_SCHEMA | OPERATION_OBSERVATION_V1_SCHEMA => continue,
            _ => return Err(A2aGetTaskReadbackError::Association),
        };
        let leaf = a2a_record_leaf(
            &append.store,
            checkpoint,
            class,
            logical,
            record.coordinate(),
        )
        .map_err(leaf_error)?;
        leaves.push(
            AccumulatorMembershipLeaf::new(leaf.hashed_key, leaf.value)
                .map_err(super::super::A2aTaskCustodyError::from)?,
        );
    }
    Ok(leaves)
}

fn push_change(
    leaf: A2aCustodyLeaf,
    old: Option<ArtifactDigest>,
    changes: &mut Vec<AccumulatorLeafChange>,
    keys: &mut Vec<ChangedControlLeafKeyV1>,
) -> Result<(), A2aGetTaskReadbackError> {
    changes.push(
        AccumulatorLeafChange::new(leaf.hashed_key, old, Some(leaf.value))
            .map_err(OperationObservationCustodyProjectionError::from)?,
    );
    keys.push(leaf.key);
    Ok(())
}

fn leaf_error(error: OperationLeafProjectionError) -> A2aGetTaskReadbackError {
    let error = match error {
        OperationLeafProjectionError::Store => {
            OperationObservationCustodyProjectionError::StoreScope
        }
        OperationLeafProjectionError::Identity(error) => error.into(),
        OperationLeafProjectionError::Canonicalize(error) => error.into(),
    };
    error.into()
}

fn checkpoint_head(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlCheckpointHead, A2aGetTaskReadbackError> {
    Ok(ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)?,
    ))
}

fn checkpoint_record(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
    maximum: usize,
) -> Result<ControlRecord, A2aGetTaskReadbackError> {
    project(
        reference,
        checkpoint.canonical_control_checkpoint_sha256().as_str(),
        checkpoint,
        maximum,
    )
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/operation_observation_custody/a2a_exchange/get_task_custody.rs"]
mod tests;
