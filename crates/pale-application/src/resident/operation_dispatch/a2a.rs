//! Durable A2A request preparation after provider dispatch.
//!
//! This module retains the exact `SendMessage` bytes before any network start.
//! Its private record joins existing identities; it does not copy route bodies,
//! credentials, message text, remote DTOs, task identity, or Pale outcomes.

use super::{
    provider::{
        recover_provider_operation_dispatch, RecoveredProviderOperationDispatch,
        RetainedProviderOperationDispatch,
    },
    *,
};
use crate::resident::{
    a2a_task_custody::A2aRouteAssociation, record_projection, RecoveredOperationObservation,
};
use pale_ports::{
    A2aExchangeCommand, A2aExchangeError, A2aExchangeInput, A2aExchangeLimits, A2aExchangeParents,
    A2aExchangePort, A2aExchangeRequest, A2aPreparedExchange, A2aSendMessageEvidencePort,
    AntiRollbackAnchorPort, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlRecordDigest, OperationA2aDependencies, OperationA2aEvidence, OperationCustodyIndexKey,
    OperationCustodyIndexMutation, OperationCustodyIndexPredecessor, OperationCustodyPort,
    OperationCustodyRecord, ProviderInvocationRequest,
    PRIVATE_OPERATION_A2A_SEND_PREPARATION_V1_SCHEMA,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

mod checkpoint;
mod delegate;
mod outcome_retention;
mod recovery;
mod response;
pub use checkpoint::{
    A2aFailedSendObservationInput, A2aInitialResponseFacts, A2aInitialTaskCustodyInput,
    A2aNoTaskObservationFailure, A2aNotStartedObservationFailure,
    A2aOutcomeUnknownObservationFailure, A2aOutcomeUnknownObservationInput, A2aSendCheckpointError,
    A2aSendCustodyFailure, A2aSendDiagnosticFailure, A2aSendNotStarted, A2aSendOutcomeUnknown,
    A2aSendReleaseError, A2aSendResponseFailure, A2aSendResponsePoll, A2aSendStartFailure,
    AwaitingA2aSendResponse, CompletedA2aNoTaskResponse, CompletedA2aSendResponse,
    CompletedA2aTaskResponse, DatabaseCommittedA2aSendAnchorPending, InFlightA2aSend,
    PreparedA2aSendCheckpoint, PreparedA2aSendCustody, PreparedA2aSendObservation,
    ReceivedA2aSendResponse, RecoveredA2aSendDiagnostic, RetainedA2aSend,
};
pub use delegate::{
    launch_a2a_delegate, A2aDelegateLaunchFailure, A2aDelegateLaunchInput, AwaitingA2aDelegate,
};
pub use outcome_retention::{
    retain_a2a_outcome, A2aNoTaskOutcomeRetention, A2aNotStartedOutcomeRetention,
    A2aOutcomeRetentionFailure, A2aOutcomeRetentionInput, A2aRecoveredUnknownOutcomeRetention,
    A2aTaskOutcomeRetention, A2aUnknownOutcomeRetention, RetainedA2aOutcome,
};
pub(in crate::resident) use outcome_retention::{
    retain_a2a_outcome_continuable, A2aOutcomeRetentionKind, ContinuableA2aOutcomeRetentionFailure,
};
pub(in crate::resident) use recovery::{
    has_indexed_a2a_send_preparation, probe_a2a_send_custody_for_known_task,
    recover_a2a_send_preparation_under_cancellation_request, A2aSendCustodyProbe,
};
pub use recovery::{
    recover_a2a_send_outcome_unknown, A2aSendRecoveryAdmission, A2aSendRecoveryError,
    A2aSendRecoveryInput, A2aSendRecoveryLimits, PreparedRecoveredA2aSendObservation,
    RecoveredA2aSendObservationFailure, RecoveredA2aSendOutcomeUnknown,
};
pub use response::{
    commit_a2a_send_observation, A2aSendObservationCommitFailure, A2aSendObservationCommitInput,
    CompletedA2aSendObservationCustody,
};

/// Private request-preparation record, absent from the public contract bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct A2aSendPreparation {
    operation_handle_ref: ResidentRecordRefV1,
    operation_handle_sha256: OperationHandleDigest,
    dispatch_ref: ResidentRecordRefV1,
    dispatch_sha256: OperationObservationDigest,
    attempt_selection: ImmutableRecordReferenceV1,
    attempt_id: CapabilityAttemptIdV1,
    route_admission_ref: ResidentRecordRefV1,
    route_admission_sha256: A2aRouteAdmissionDigest,
    json_rpc_request_id: A2aJsonRpcRequestIdV1,
    request_mode: A2aTaskRequestModeV1,
    native_message_id: String,
    request_evidence: ImmutableRecordReferenceV1,
    tls_trust_input_sha256: ArtifactDigest,
}

/// Exact inputs for preparing one unary A2A request without network I/O.
pub struct A2aSendInput<'operation, 'call> {
    /// Fresh or protected-recovered provider dispatch for this A2A continuation.
    pub dispatch: A2aSendDispatch<'operation>,
    /// Current session authority. Preparation-time authority is not reused.
    pub current: &'call AmbientSessionAdmissionContext<'call>,
    /// Exact admitted A2A route and its immutable reference.
    pub route: (&'call ResidentRecordRefV1, &'call A2aRouteAdmissionV1),
    /// Physical state of the immutable route row.
    pub route_predecessor: OperationCustodyIndexPredecessor,
    /// Whether this checkpoint domain already authenticates the route leaf.
    pub route_association: A2aRouteAssociation,
    /// Fresh JSON-RPC request identity.
    pub request_id: &'call A2aJsonRpcRequestIdV1,
    /// Fresh owner-native message identity.
    pub message_id: &'call str,
    /// Already selected text, copied only after disclosure admission.
    pub text: &'call str,
    /// Fresh immutable reference for the exact request bytes.
    pub request_evidence_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the private preparation record.
    pub preparation_ref: ResidentRecordRefV1,
    /// Explicit finite carrier ceilings.
    pub exchange_limits: A2aExchangeLimits,
    /// Maximum aggregate request-evidence bytes.
    pub maximum_evidence_bytes: usize,
    /// Maximum canonical bytes for each retained record.
    pub maximum_record_bytes: usize,
    /// Bound used while rechecking the retained provider dispatch.
    pub maximum_dispatch_record_bytes: usize,
    /// Store-coordinate resolver used to recheck a fresh provider dispatch.
    /// Protected recovery already carries its admitted store coordinate.
    pub resolve_store: &'call dyn OperationObservationAuthorityStoreResolver,
}

/// Provider dispatch admitted specifically for A2A request preparation.
///
/// Fresh construction preserves new-commit custody. Protected recovery requires
/// an anchored current `Dispatched` head and can only continue into a new A2A
/// preparation. Neither form starts the carrier by construction.
pub struct A2aSendDispatch<'operation> {
    inner: A2aProviderDispatch<'operation>,
}

impl<'operation> A2aSendDispatch<'operation> {
    /// Consume one newly retained provider dispatch for A2A preparation.
    #[must_use]
    pub fn fresh(dispatch: RetainedProviderOperationDispatch<'operation>) -> Self {
        Self {
            inner: A2aProviderDispatch::Fresh(Box::new(dispatch)),
        }
    }

    /// Recover an anchored provider dispatch for A2A preparation only.
    ///
    /// # Errors
    /// Rejects a substituted parent, selection, dispatch, predecessor, store,
    /// or non-current operation head. Success grants no provider replay.
    pub fn recover(
        history: OperationAttemptHistoryInput<'operation>,
        recovered: &'operation RecoveredOperationObservation,
    ) -> Result<Self, OperationAttemptError> {
        Ok(Self {
            inner: A2aProviderDispatch::Recovered(Box::new(recover_provider_operation_dispatch(
                history, recovered,
            )?)),
        })
    }

    /// Derive the store-qualified process-local key before any carrier use.
    pub(in crate::resident) fn task_key(
        &self,
    ) -> Result<crate::resident::task_owner::ResidentOperationTaskKey, OperationAttemptError> {
        let (record_ref, observation) = self.inner.observation();
        let record_sha256 = ArtifactDigest::parse(
            observation
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(|_| OperationAttemptError::Encoding)?;
        Ok(crate::resident::task_owner::ResidentOperationTaskKey::new(
            self.inner.store().clone(),
            ImmutableRecordReferenceV1 {
                record_ref: record_ref.clone(),
                record_sha256,
            },
        ))
    }

    /// Borrow the issued operation that owns this dispatch.
    #[must_use]
    pub(in crate::resident) fn operation(&self) -> &'operation IssuedLocalOperation {
        self.inner.operation()
    }

    /// Borrow the exact physical store that retains this dispatch.
    #[must_use]
    pub(in crate::resident) fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        self.inner.store()
    }
}

impl<'operation> From<RetainedProviderOperationDispatch<'operation>>
    for A2aSendDispatch<'operation>
{
    fn from(dispatch: RetainedProviderOperationDispatch<'operation>) -> Self {
        Self::fresh(dispatch)
    }
}

enum A2aProviderDispatch<'operation> {
    Fresh(Box<RetainedProviderOperationDispatch<'operation>>),
    Recovered(Box<RecoveredProviderOperationDispatch<'operation>>),
}

impl<'operation> A2aProviderDispatch<'operation> {
    fn operation(&self) -> &'operation IssuedLocalOperation {
        match self {
            Self::Fresh(dispatch) => dispatch.operation(),
            Self::Recovered(dispatch) => dispatch.operation(),
        }
    }

    fn provider_request(&self) -> Result<ProviderInvocationRequest<'_>, OperationAttemptError> {
        match self {
            Self::Fresh(dispatch) => dispatch.provider_request(),
            Self::Recovered(dispatch) => dispatch.provider_request(),
        }
    }

    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        match self {
            Self::Fresh(dispatch) => dispatch.observation(),
            Self::Recovered(dispatch) => dispatch.observation(),
        }
    }

    fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        match self {
            Self::Fresh(dispatch) => dispatch.checkpoint(),
            Self::Recovered(dispatch) => dispatch.checkpoint(),
        }
    }

    fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        match self {
            Self::Fresh(dispatch) => dispatch.custody().candidate().store(),
            Self::Recovered(dispatch) => dispatch.store(),
        }
    }

    fn selection_ref(&self) -> &ResidentRecordRefV1 {
        match self {
            Self::Fresh(dispatch) => &dispatch.context().selection_ref,
            Self::Recovered(dispatch) => dispatch.selection_ref(),
        }
    }

    fn preparation_observation_ref(&self) -> &ResidentRecordRefV1 {
        match self {
            Self::Fresh(dispatch) => &dispatch.context().observation_ref,
            Self::Recovered(dispatch) => dispatch.preparation_observation_ref(),
        }
    }

    fn validate_current_at<Authority: AmbientSessionAuthorityPort>(
        &self,
        current: &AmbientSessionAdmissionContext<'_>,
        authority: &mut Authority,
    ) -> Result<(), OperationAttemptError> {
        match self {
            Self::Fresh(dispatch) => dispatch.context().validate_current_at(current, authority),
            Self::Recovered(dispatch) => dispatch.validate_current_at(current, authority),
        }
    }

    fn verify_preparation_custody(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        maximum_record_bytes: usize,
    ) -> Result<(), OperationAttemptError> {
        match self {
            Self::Fresh(dispatch) => dispatch.verify_release_custody(
                resolve_store,
                custody,
                anchor,
                maximum_record_bytes,
            ),
            Self::Recovered(dispatch) => {
                dispatch.verify_preparation_custody(custody, anchor, maximum_record_bytes)
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/operation_dispatch/a2a_support.rs"]
pub(in crate::resident) mod test_support;

struct A2aSendPreparationInput<'operation, 'call> {
    dispatch: A2aProviderDispatch<'operation>,
    current: &'call AmbientSessionAdmissionContext<'call>,
    route: (&'call ResidentRecordRefV1, &'call A2aRouteAdmissionV1),
    route_predecessor: OperationCustodyIndexPredecessor,
    route_association: A2aRouteAssociation,
    request_id: &'call A2aJsonRpcRequestIdV1,
    message_id: &'call str,
    text: &'call str,
    request_evidence_ref: ResidentRecordRefV1,
    preparation_ref: ResidentRecordRefV1,
    exchange_limits: A2aExchangeLimits,
    maximum_evidence_bytes: usize,
    maximum_record_bytes: usize,
}

/// Exact request bytes and closed physical inputs awaiting checkpoint custody.
#[must_use = "retain the prepared request before starting its A2A exchange"]
pub struct PreparedA2aSend<'operation, 'call> {
    dispatch: A2aProviderDispatch<'operation>,
    pub(super) route: (&'call ResidentRecordRefV1, &'call A2aRouteAdmissionV1),
    pub(super) preparation: ControlRecord,
    pub(super) request_evidence: OperationA2aEvidence,
    pub(super) exchange: A2aPreparedExchange,
    pub(super) store: pale_ports::AuthorityStoreCoordinate,
    pub(super) records: Vec<OperationCustodyRecord>,
    pub(super) indexes: Vec<OperationCustodyIndexMutation>,
    pub(super) dependencies: OperationA2aDependencies,
    pub(super) maximum_evidence_bytes: usize,
    pub(super) maximum_record_bytes: usize,
}

impl PreparedA2aSend<'_, '_> {
    /// Copy the exact checkpoint selected as this preparation's predecessor.
    ///
    /// A fresh dispatch uses its committing checkpoint. Protected recovery uses
    /// the authenticated current checkpoint that still retains `Dispatched` as
    /// this operation's head.
    #[must_use]
    pub fn predecessor_checkpoint(&self) -> (ResidentRecordRefV1, ControlStateCheckpointV1) {
        let (reference, checkpoint) = self.dispatch.checkpoint();
        (reference.clone(), checkpoint.clone())
    }
}

/// Request preparation failed before any A2A exchange started.
#[derive(Debug, thiserror::Error)]
pub enum A2aSendPreparationError {
    /// Current operation or provider-dispatch admission failed.
    #[error(transparent)]
    Operation(#[from] OperationAttemptError),
    /// Route, reference, or disclosure evidence did not describe this send.
    #[error("A2A send preparation does not match its admitted operation and route")]
    Association,
    /// The selected exchange owner rejected request shape or bounds.
    #[error(transparent)]
    Exchange(#[from] A2aExchangeError),
    /// Private record encoding or physical projection failed.
    #[error(transparent)]
    Projection(#[from] OperationObservationCustodyProjectionError),
    /// The private preparation exceeded its selected byte ceiling.
    #[error("A2A send preparation exceeds its record byte limit")]
    BoundExceeded,
}

impl From<crate::resident::record_projection::ControlRecordProjectionError>
    for A2aSendPreparationError
{
    fn from(value: crate::resident::record_projection::ControlRecordProjectionError) -> Self {
        Self::Projection(value.into())
    }
}

/// Prepare exact unary `SendMessage` bytes and their closed custody atom.
///
/// `admit_disclosure` owns the current decision to copy the selected text to
/// this route. Success does not authorize a network start. The exact bytes must
/// still cross checkpoint, database, and independent-anchor custody.
///
/// # Errors
///
/// Returns an error before network I/O when current authority, dispatch custody,
/// route/disclosure/transport admission, request shape, bounds, or projection
/// does not match the selected operation.
pub fn prepare_a2a_send<'operation, 'call, Authority, Disclosure, Transport>(
    input: A2aSendInput<'operation, 'call>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    exchange: &dyn A2aExchangePort,
    admit_disclosure: Disclosure,
    admit_transport: Transport,
) -> Result<PreparedA2aSend<'operation, 'call>, A2aSendPreparationError>
where
    Authority: AmbientSessionAuthorityPort,
    Disclosure: FnOnce(
        &ProviderInvocationRequest<'_>,
        &A2aRouteAdmissionV1,
        &str,
    ) -> Result<(), InvalidResidentRecordV1>,
    Transport: FnOnce(
        &ImmutableRecordReferenceV1,
        &ProviderInvocationRequest<'_>,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let A2aSendInput {
        dispatch,
        current,
        route,
        route_predecessor,
        route_association,
        request_id,
        message_id,
        text,
        request_evidence_ref,
        preparation_ref,
        exchange_limits,
        maximum_evidence_bytes,
        maximum_record_bytes,
        maximum_dispatch_record_bytes,
        resolve_store,
    } = input;
    let dispatch = dispatch.inner;
    dispatch.verify_preparation_custody(
        resolve_store,
        custody,
        anchor,
        maximum_dispatch_record_bytes,
    )?;
    prepare_a2a_send_from_dispatch(
        A2aSendPreparationInput {
            dispatch,
            current,
            route,
            route_predecessor,
            route_association,
            request_id,
            message_id,
            text,
            request_evidence_ref,
            preparation_ref,
            exchange_limits,
            maximum_evidence_bytes,
            maximum_record_bytes,
        },
        authority,
        exchange,
        admit_disclosure,
        admit_transport,
    )
}

fn prepare_a2a_send_from_dispatch<'operation, 'call, Authority, Disclosure, Transport>(
    input: A2aSendPreparationInput<'operation, 'call>,
    authority: &mut Authority,
    exchange: &dyn A2aExchangePort,
    admit_disclosure: Disclosure,
    admit_transport: Transport,
) -> Result<PreparedA2aSend<'operation, 'call>, A2aSendPreparationError>
where
    Authority: AmbientSessionAuthorityPort,
    Disclosure: FnOnce(
        &ProviderInvocationRequest<'_>,
        &A2aRouteAdmissionV1,
        &str,
    ) -> Result<(), InvalidResidentRecordV1>,
    Transport: FnOnce(
        &ImmutableRecordReferenceV1,
        &ProviderInvocationRequest<'_>,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    input
        .dispatch
        .validate_current_at(input.current, authority)?;
    let (handle_ref, handle) = input.dispatch.operation().handle();
    if handle.operation_class() != &OperationClassV1::Delegate {
        return Err(A2aSendPreparationError::Association);
    }
    let provider_request = input.dispatch.provider_request()?;
    let (dispatch_ref, dispatch) = input.dispatch.observation();
    let (attempt_id, attempt_selection) = dispatched_attempt(dispatch)?;
    let (route_ref, route) = input.route;
    route.verify().map_err(OperationAttemptError::from)?;
    validate_route(
        route,
        route_ref,
        handle,
        dispatch.observed_at(),
        &provider_request,
        input.current.now,
    )?;
    admit_disclosure(&provider_request, route, input.text).map_err(OperationAttemptError::from)?;
    validate_references(
        &input.preparation_ref,
        &input.request_evidence_ref,
        handle_ref,
        dispatch_ref,
        route_ref,
        attempt_selection,
        input.dispatch.operation().request_coordinate().record_ref(),
    )?;

    let request = A2aExchangeRequest::new(
        provider_request,
        A2aExchangeParents {
            operation: (handle_ref, handle),
            observation: (dispatch_ref, dispatch),
            route: (route_ref, route),
        },
        input.request_id,
        A2aExchangeCommand::SendText {
            mode: A2aTaskRequestModeV1::SendMessage,
            message_id: input.message_id,
            text: input.text,
        },
        input.exchange_limits,
    )?;
    let expected_context = request.coordinate().clone();
    let expected_endpoint = request.endpoint().clone();
    let expected_limits = request.limits();
    let prepared_exchange = exchange.prepare(request)?;
    if prepared_exchange.coordinate().context() != &expected_context
        || prepared_exchange.coordinate().limits() != expected_limits
        || prepared_exchange.request().coordinate() != &expected_context
        || prepared_exchange.request().endpoint() != &expected_endpoint
        || prepared_exchange.request().limits() != expected_limits
        || !matches!(
            prepared_exchange.request().input(),
            A2aExchangeInput::Text {
                mode: A2aTaskRequestModeV1::SendMessage,
                message_id,
                text,
            } if message_id == input.message_id && text == input.text
        )
    {
        return Err(A2aExchangeError::CoordinateMismatch.into());
    }
    admit_transport(
        &route.security().evidence,
        &provider_request,
        prepared_exchange.coordinate().tls_trust_input_sha256(),
    )
    .map_err(OperationAttemptError::from)?;
    let request_evidence_ref = ImmutableRecordReferenceV1 {
        record_ref: input.request_evidence_ref,
        record_sha256: prepared_exchange.coordinate().request_sha256().clone(),
    };
    let request_evidence = OperationA2aEvidence::new(
        request_evidence_ref.clone(),
        prepared_exchange.bytes().to_vec(),
        None,
        input.maximum_evidence_bytes,
    )
    .map_err(OperationObservationCustodyProjectionError::from)?;
    let preparation = A2aSendPreparation {
        operation_handle_ref: handle_ref.clone(),
        operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
        dispatch_ref: dispatch_ref.clone(),
        dispatch_sha256: dispatch.canonical_operation_observation_sha256().clone(),
        attempt_selection: attempt_selection.clone(),
        attempt_id: attempt_id.clone(),
        route_admission_ref: route_ref.clone(),
        route_admission_sha256: route.canonical_a2a_route_admission_sha256().clone(),
        json_rpc_request_id: input.request_id.clone(),
        request_mode: A2aTaskRequestModeV1::SendMessage,
        native_message_id: input.message_id.to_owned(),
        request_evidence: request_evidence_ref,
        tls_trust_input_sha256: prepared_exchange
            .coordinate()
            .tls_trust_input_sha256()
            .clone(),
    };
    let preparation = preparation.record(input.preparation_ref, input.maximum_record_bytes)?;
    let route_record = record_projection::record(
        record_projection::coordinate(
            route_ref,
            route.canonical_a2a_route_admission_sha256().as_str(),
        )?,
        route,
    )?;
    let handle_record = record_projection::record(
        record_projection::coordinate(
            handle_ref,
            handle.canonical_operation_handle_sha256().as_str(),
        )?,
        handle,
    )?;
    let dispatch_record = record_projection::record(
        record_projection::coordinate(
            dispatch_ref,
            dispatch.canonical_operation_observation_sha256().as_str(),
        )?,
        dispatch,
    )?;
    for record in [
        &preparation,
        &route_record,
        &handle_record,
        &dispatch_record,
    ] {
        if record.canonical_record_jcs().as_slice().len() > input.maximum_record_bytes {
            return Err(A2aSendPreparationError::BoundExceeded);
        }
    }
    let mut records = vec![OperationCustodyRecord::A2aSendPreparation(
        preparation.clone(),
    )];
    let mut retained_records = vec![handle_record, dispatch_record];
    let mut associated_route = None;
    match input.route_predecessor {
        OperationCustodyIndexPredecessor::Absent => {
            if input.route_association == A2aRouteAssociation::Retained {
                return Err(A2aSendPreparationError::Association);
            }
            records.push(OperationCustodyRecord::A2aRouteAdmission(route_record));
        }
        OperationCustodyIndexPredecessor::Exact(expected) if expected == route_record => {
            if input.route_association == A2aRouteAssociation::Insert {
                associated_route = Some(route_record);
            } else {
                retained_records.push(route_record);
            }
        }
        OperationCustodyIndexPredecessor::Exact(_) => {
            return Err(A2aSendPreparationError::Association);
        }
    }
    let dispatch_coordinate = record_projection::coordinate(
        dispatch_ref,
        dispatch.canonical_operation_observation_sha256().as_str(),
    )?;
    let indexes = vec![OperationCustodyIndexMutation::A2aSendPreparation {
        key: OperationCustodyIndexKey::for_a2a_send_preparation(&dispatch_coordinate),
        expected: OperationCustodyIndexPredecessor::Absent,
        successor: preparation.coordinate().clone(),
    }];
    let store = input.dispatch.store().clone();
    Ok(PreparedA2aSend {
        dispatch: input.dispatch,
        route: input.route,
        preparation,
        request_evidence: request_evidence.clone(),
        exchange: prepared_exchange,
        store,
        records,
        indexes,
        dependencies: OperationA2aDependencies {
            new_evidence: vec![request_evidence],
            retained_evidence: Vec::new(),
            retained_records,
            associated_route,
        },
        maximum_evidence_bytes: input.maximum_evidence_bytes,
        maximum_record_bytes: input.maximum_record_bytes,
    })
}

impl A2aSendPreparation {
    fn record(
        &self,
        reference: ResidentRecordRefV1,
        maximum_bytes: usize,
    ) -> Result<ControlRecord, A2aSendPreparationError> {
        let bytes = pale_core::canon::to_canonical_string(self)
            .map_err(OperationObservationCustodyProjectionError::from)?
            .into_bytes();
        if bytes.len() > maximum_bytes {
            return Err(A2aSendPreparationError::BoundExceeded);
        }
        Ok(ControlRecord::new(
            ControlRecordCoordinate::new(
                reference,
                ControlRecordDigest::parse(pale_core::sha256_hex(&bytes))
                    .map_err(OperationObservationCustodyProjectionError::from)?,
            ),
            SchemaId::parse(PRIVATE_OPERATION_A2A_SEND_PREPARATION_V1_SCHEMA)
                .map_err(|_| A2aSendPreparationError::Association)?,
            Self::schema_digest()?,
            ControlRecordBytes::new(bytes)
                .map_err(OperationObservationCustodyProjectionError::from)?,
        ))
    }

    fn schema_digest() -> Result<SchemaDigest, A2aSendPreparationError> {
        static DIGEST: OnceLock<Result<SchemaDigest, ()>> = OnceLock::new();
        DIGEST
            .get_or_init(|| {
                let schema = schemars::schema_for!(A2aSendPreparation);
                let canonical = pale_core::canon::to_canonical_string(&schema).map_err(|_| ())?;
                SchemaDigest::parse(pale_core::sha256_hex(canonical.as_bytes())).map_err(|_| ())
            })
            .as_ref()
            .cloned()
            .map_err(|_| A2aSendPreparationError::Association)
    }
}

fn dispatched_attempt(
    dispatch: &OperationObservationV1,
) -> Result<(&CapabilityAttemptIdV1, &ImmutableRecordReferenceV1), A2aSendPreparationError> {
    match dispatch.state() {
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::Dispatched {
                    attempt_id,
                    evidence,
                },
        } => Ok((attempt_id, evidence)),
        _ => Err(A2aSendPreparationError::Association),
    }
}

fn validate_route(
    route: &A2aRouteAdmissionV1,
    route_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    dispatch_observed_at: &UnixTimeV1,
    request: &ProviderInvocationRequest<'_>,
    now: UnixTimeV1,
) -> Result<(), A2aSendPreparationError> {
    if route_ref.as_str().is_empty()
        || route.provider_binding().provider_binding_id != request.provider_binding().id
        || route.provider_binding().provider_binding_identity
            != request.provider_binding().binding_identity
        || !route
            .route()
            .matches_request_binding(handle.request_binding())
        || route.observed_at() > dispatch_observed_at
        || dispatch_observed_at > route.fresh_until()
        || now < *dispatch_observed_at
        || now < *route.observed_at()
        || now > *route.fresh_until()
        || now >= *route.expires_at()
    {
        return Err(A2aSendPreparationError::Association);
    }
    Ok(())
}

fn validate_references(
    preparation: &ResidentRecordRefV1,
    evidence: &ResidentRecordRefV1,
    handle: &ResidentRecordRefV1,
    dispatch: &ResidentRecordRefV1,
    route: &ResidentRecordRefV1,
    selection: &ImmutableRecordReferenceV1,
    request: &ResidentRecordRefV1,
) -> Result<(), A2aSendPreparationError> {
    let values = [
        preparation,
        evidence,
        handle,
        dispatch,
        route,
        &selection.record_ref,
        request,
    ];
    if values
        .iter()
        .enumerate()
        .any(|(index, value)| values[..index].contains(value))
    {
        return Err(A2aSendPreparationError::Association);
    }
    Ok(())
}
