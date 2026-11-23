//! Atomic physical custody for durable Pale operations.
//!
//! Application policy admits every semantic record and derives every private
//! index key before crossing this boundary. The port owns only durable start,
//! predecessor-linked observation, exact read-back, and point lookup. It owns
//! no routing, provider call, consultation join, cancellation, or settlement.
//! When an observation carries a result, diagnostic or private attempt selection, the
//! application has already checked their association; this boundary checks custody.

use std::collections::BTreeSet;

use pale_core::records::{
    ControlTransactionSequenceV1, ADAPTER_FAILURE_V1_SCHEMA, CAPABILITY_RESULT_V1_SCHEMA,
    COMMON_REQUEST_ENVELOPE_V1_SCHEMA, CONTROL_STATE_CHECKPOINT_V1_SCHEMA, DIAGNOSTIC_V1_SCHEMA,
    OPERATION_HANDLE_V1_SCHEMA, OPERATION_OBSERVATION_V1_SCHEMA,
};

use crate::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord, ControlRecordCoordinate,
    ControlRecordError, ControlStoreError, OperationAccumulatorBatch,
};

mod collection;
pub use collection::{
    OperationCollectionFrontierEvidence, OperationCollectionHeadObservation,
    OperationCollectionJournal, OperationCollectionJournalError, OperationCollectionSnapshot,
    PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA,
};

mod publication;
pub use publication::{
    OperationPublicationCursor, OperationPublicationReadError, OperationPublicationSource,
    OperationPublicationSourceKind, OperationPublicationSources,
    MAX_OPERATION_PUBLICATION_BODY_BYTES, MAX_OPERATION_PUBLICATION_SOURCES,
};

/// Private application attempt-selection bytes retained with an observation.
/// This physical schema tag is not a public contract or semantic admission.
pub const PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA: &str =
    "pale/private-operation-attempt-selection.v1";
/// Private exact A2A request preparation retained before one carrier release.
/// This physical schema tag is not a public contract or semantic admission.
pub const PRIVATE_OPERATION_A2A_SEND_PREPARATION_V1_SCHEMA: &str =
    "pale/private-operation-a2a-send-preparation.v1";
/// Private exact `GetTask` preparation retained before one carrier release.
pub const PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA: &str =
    "pale/private-operation-a2a-get-task-preparation.v1";
/// Private closed local outcome for one retained `GetTask` preparation.
pub const PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA: &str =
    "pale/private-operation-a2a-get-task-local-disposition.v1";
/// Private exact native outcome retained before terminal evidence is issued.
/// This physical schema tag is not a public contract or semantic admission.
pub const PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA: &str =
    "pale/private-operation-native-outcome.v1";

/// Private exact hook response retained before one native response release.
/// The application binds its permission and disclosure evidence to the operation.
pub const PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA: &str =
    "pale/private-operation-hook-response.v1";

mod a2a;
pub use a2a::{OperationA2aDependencies, OperationA2aEvidence};
mod a2a_discovery;
pub use a2a_discovery::{
    OperationA2aBindingCursor, OperationA2aBindingScope, OperationA2aBindingSource,
    OperationA2aBindings, MAX_OPERATION_A2A_BINDINGS, MAX_OPERATION_A2A_BINDING_BYTES,
};

mod session_basis;
pub use session_basis::{OperationSessionBasis, MAX_OPERATION_SESSION_BASIS_BYTES};
mod start_discovery;
pub use start_discovery::{
    OperationStartCursor, OperationStartReadError, OperationStartSource, OperationStartSources,
    MAX_OPERATION_START_SOURCES, MAX_OPERATION_START_SOURCE_BYTES,
};

/// Closed physical operation transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationCustodyTransitionKind {
    /// Commit one root request, handle, and initial observation.
    StartRoot,
    /// Commit one child request, handle, initial observation, and exact parent link.
    StartChild,
    /// Append an observation with one admitted single-record association or the
    /// exact failure-result-diagnostic atom for a safe provider failure.
    Observe,
    /// Retain one exact A2A request before its one-shot carrier release.
    PrepareA2aSend,
    /// Retain one exact `GetTask` request before its one-shot carrier release.
    PrepareA2aGetTask,
    /// Close a retained `GetTask` after a local stop or closed local failure.
    DisposeA2aGetTask,
    /// Retain an admitted remote binding and its initial owner-state observation.
    BindA2aTask,
    /// Advance remote task history without changing the Pale operation state.
    ObserveA2aTask,
}

/// Closed record role in one operation-custody transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationCustodyRecordKind {
    /// Private exact A2A request preparation.
    A2aSendPreparation,
    /// Private exact `GetTask` request preparation.
    A2aGetTaskPreparation,
    /// Private closed local `GetTask` disposition.
    A2aGetTaskLocalDisposition,
    /// Private exact native computation outcome.
    NativeOutcome,
    /// Private exact host response and its admitted permission evidence.
    HookResponse,
    /// Exact admitted route retained with a remote task binding.
    A2aRouteAdmission,
    /// Immutable remote task binding.
    A2aTaskBinding,
    /// Remote owner-state history, not a Pale result.
    A2aTaskObservation,
    /// Exact caller-signed common request.
    CommonRequest,
    /// Immutable durable operation handle.
    OperationHandle,
    /// One predecessor-linked operation observation.
    OperationObservation,
    /// Capability result retained with its successor observation.
    CapabilityResult,
    /// Adapter failure referenced by a failed capability result.
    AdapterFailure,
    /// Private attempt selection retained with its observation.
    AttemptSelection,
    /// Diagnostic retained with its failure observation.
    Diagnostic,
    /// Derived physical frontier inseparable from its authority-journal append.
    CollectionFrontier,
}

impl OperationCustodyRecordKind {
    const fn schema_id(self) -> &'static str {
        match self {
            Self::A2aSendPreparation => PRIVATE_OPERATION_A2A_SEND_PREPARATION_V1_SCHEMA,
            Self::A2aGetTaskPreparation => PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
            Self::A2aGetTaskLocalDisposition => {
                PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA
            }
            Self::NativeOutcome => PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA,
            Self::HookResponse => PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA,
            Self::A2aRouteAdmission => pale_core::records::A2A_ROUTE_ADMISSION_V1_SCHEMA,
            Self::A2aTaskBinding => pale_core::records::A2A_TASK_BINDING_V1_SCHEMA,
            Self::A2aTaskObservation => pale_core::records::A2A_TASK_OBSERVATION_V1_SCHEMA,
            Self::CommonRequest => COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
            Self::OperationHandle => OPERATION_HANDLE_V1_SCHEMA,
            Self::OperationObservation => OPERATION_OBSERVATION_V1_SCHEMA,
            Self::CapabilityResult => CAPABILITY_RESULT_V1_SCHEMA,
            Self::AdapterFailure => ADAPTER_FAILURE_V1_SCHEMA,
            Self::AttemptSelection => PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
            Self::Diagnostic => DIAGNOSTIC_V1_SCHEMA,
            Self::CollectionFrontier => PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA,
        }
    }
}

/// One already-admitted immutable record in an operation transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationCustodyRecord {
    /// Private exact A2A request preparation.
    A2aSendPreparation(ControlRecord),
    /// Private exact `GetTask` request preparation.
    A2aGetTaskPreparation(ControlRecord),
    /// Private closed local `GetTask` disposition.
    A2aGetTaskLocalDisposition(ControlRecord),
    /// Private exact native computation outcome.
    NativeOutcome(ControlRecord),
    /// Private exact host response and its admitted permission evidence.
    HookResponse(ControlRecord),
    /// Exact admitted historical route.
    A2aRouteAdmission(ControlRecord),
    /// Immutable binding to an actually observed remote task.
    A2aTaskBinding(ControlRecord),
    /// One admitted remote-task history revision.
    A2aTaskObservation(ControlRecord),
    /// Exact caller-signed common request.
    CommonRequest(ControlRecord),
    /// Immutable durable operation handle.
    OperationHandle(ControlRecord),
    /// One predecessor-linked operation observation.
    OperationObservation(ControlRecord),
    /// Capability result whose association was admitted by the application.
    CapabilityResult(ControlRecord),
    /// Adapter failure whose association was admitted by the application.
    AdapterFailure(ControlRecord),
    /// Private attempt selection whose meaning was admitted by the application.
    AttemptSelection(ControlRecord),
    /// Diagnostic whose failure association was admitted by the application.
    Diagnostic(ControlRecord),
    /// Derived physical binding; accepted only through the paired attachment owner.
    CollectionFrontier(ControlRecord),
}

impl OperationCustodyRecord {
    /// Return the record's closed role.
    #[must_use]
    pub const fn kind(&self) -> OperationCustodyRecordKind {
        match self {
            Self::A2aSendPreparation(_) => OperationCustodyRecordKind::A2aSendPreparation,
            Self::A2aGetTaskPreparation(_) => OperationCustodyRecordKind::A2aGetTaskPreparation,
            Self::A2aGetTaskLocalDisposition(_) => {
                OperationCustodyRecordKind::A2aGetTaskLocalDisposition
            }
            Self::NativeOutcome(_) => OperationCustodyRecordKind::NativeOutcome,
            Self::HookResponse(_) => OperationCustodyRecordKind::HookResponse,
            Self::A2aRouteAdmission(_) => OperationCustodyRecordKind::A2aRouteAdmission,
            Self::A2aTaskBinding(_) => OperationCustodyRecordKind::A2aTaskBinding,
            Self::A2aTaskObservation(_) => OperationCustodyRecordKind::A2aTaskObservation,
            Self::CommonRequest(_) => OperationCustodyRecordKind::CommonRequest,
            Self::OperationHandle(_) => OperationCustodyRecordKind::OperationHandle,
            Self::OperationObservation(_) => OperationCustodyRecordKind::OperationObservation,
            Self::CapabilityResult(_) => OperationCustodyRecordKind::CapabilityResult,
            Self::AdapterFailure(_) => OperationCustodyRecordKind::AdapterFailure,
            Self::AttemptSelection(_) => OperationCustodyRecordKind::AttemptSelection,
            Self::Diagnostic(_) => OperationCustodyRecordKind::Diagnostic,
            Self::CollectionFrontier(_) => OperationCustodyRecordKind::CollectionFrontier,
        }
    }

    /// Borrow the exact physical record.
    #[must_use]
    pub const fn record(&self) -> &ControlRecord {
        match self {
            Self::A2aSendPreparation(record)
            | Self::A2aGetTaskPreparation(record)
            | Self::A2aGetTaskLocalDisposition(record)
            | Self::NativeOutcome(record)
            | Self::HookResponse(record)
            | Self::A2aRouteAdmission(record)
            | Self::A2aTaskBinding(record)
            | Self::A2aTaskObservation(record)
            | Self::CommonRequest(record)
            | Self::OperationHandle(record)
            | Self::OperationObservation(record)
            | Self::CapabilityResult(record)
            | Self::AdapterFailure(record)
            | Self::AttemptSelection(record)
            | Self::Diagnostic(record)
            | Self::CollectionFrontier(record) => record,
        }
    }
}

/// Domain-separated canonical digest used as one private operation-index key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationCustodyIndexKey(String);

impl OperationCustodyIndexKey {
    /// Key one immutable A2A request preparation by its exact dispatch.
    #[must_use]
    pub fn for_a2a_send_preparation(coordinate: &ControlRecordCoordinate) -> Self {
        Self::for_coordinate(b"pale:a2a-send-preparation:v1\0", coordinate)
    }

    /// Key the checkpoint that retained one exact private A2A request preparation.
    #[must_use]
    pub fn for_a2a_send_preparation_checkpoint(coordinate: &ControlRecordCoordinate) -> Self {
        Self::for_coordinate(b"pale:a2a-send-preparation-checkpoint:v1\0", coordinate)
    }

    /// Key the checkpoint that retained one private `GetTask` record.
    #[must_use]
    pub fn for_a2a_get_task_checkpoint(coordinate: &ControlRecordCoordinate) -> Self {
        Self::for_coordinate(b"pale:a2a-get-task-checkpoint:v1\0", coordinate)
    }

    /// Key one immutable native outcome by its exact dispatched observation.
    #[must_use]
    pub fn for_native_outcome(coordinate: &ControlRecordCoordinate) -> Self {
        Self::for_coordinate(b"pale:native-outcome:v1\0", coordinate)
    }

    /// Key an immutable observation checkpoint by its exact reference and digest.
    ///
    /// Hashes a domain tag, big-endian reference-byte length, reference bytes,
    /// and the fixed-width digest. This private lookup identity is not the
    /// canonical record identity and grants no authority.
    #[must_use]
    pub fn for_observation(coordinate: &ControlRecordCoordinate) -> Self {
        Self::for_coordinate(b"pale:operation-observation-checkpoint:v1\0", coordinate)
    }

    /// Key the current private consultation collection by its exact operation handle.
    #[must_use]
    pub fn for_collection_head(coordinate: &ControlRecordCoordinate) -> Self {
        Self::for_coordinate(b"pale:consultation-collection-head:v1\0", coordinate)
    }

    fn for_coordinate(domain: &[u8], coordinate: &ControlRecordCoordinate) -> Self {
        let reference = coordinate.record_ref().as_str().as_bytes();
        let mut bytes = Vec::with_capacity(domain.len() + 8 + reference.len() + 64);
        bytes.extend_from_slice(domain);
        bytes.extend_from_slice(&(reference.len() as u64).to_be_bytes());
        bytes.extend_from_slice(reference);
        bytes.extend_from_slice(coordinate.record_sha256().as_str().as_bytes());
        Self(pale_core::sha256_hex(&bytes))
    }
    /// Admit one canonical lowercase SHA-256 spelling.
    ///
    /// # Errors
    /// Returns [`ControlRecordError::InvalidDigest`] for a malformed digest.
    pub fn parse(value: impl Into<String>) -> Result<Self, ControlRecordError> {
        let value = value.into();
        if pale_core::is_sha256_hex(&value) {
            Ok(Self(value))
        } else {
            Err(ControlRecordError::InvalidDigest)
        }
    }

    /// Borrow the canonical digest spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Exact predecessor asserted at one private index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationCustodyIndexPredecessor {
    /// The index must not exist.
    Absent,
    /// The index must contain this exact immutable record.
    Exact(ControlRecord),
}

/// Closed private-index mutation set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationCustodyIndexMutation {
    /// Bind one exact dispatched native attempt to its retained outcome.
    NativeOutcome {
        /// Key derived from the exact dispatched observation coordinate.
        key: OperationCustodyIndexKey,
        /// Outcome staging is immutable and must be absent on first retention.
        expected: OperationCustodyIndexPredecessor,
        /// Exact private outcome record.
        successor: ControlRecordCoordinate,
    },
    /// Bind one exact dispatched provider attempt to its retained A2A request.
    A2aSendPreparation {
        /// Key derived from the exact retained dispatch coordinate.
        key: OperationCustodyIndexKey,
        /// Preparation is immutable and must be absent on first retention.
        expected: OperationCustodyIndexPredecessor,
        /// Exact private preparation record.
        successor: ControlRecordCoordinate,
    },
    /// Bind one retained A2A request preparation to its committing checkpoint.
    A2aSendPreparationCheckpoint {
        /// Key derived from the exact preparation coordinate.
        key: OperationCustodyIndexKey,
        /// Signed checkpoint coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Advance the current physical `GetTask` phase for one binding.
    CurrentA2aGetTask {
        /// Existing binding-qualified key.
        key: OperationCustodyIndexKey,
        /// Absent for the first read, otherwise the exact terminal predecessor.
        expected: OperationCustodyIndexPredecessor,
        /// Preparation, public observation, or private local disposition.
        successor: ControlRecordCoordinate,
    },
    /// Bind one private `GetTask` record to its committing checkpoint.
    A2aGetTaskCheckpoint {
        /// Key derived from the exact private record coordinate.
        key: OperationCustodyIndexKey,
        /// Signed checkpoint coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Immutable admitted binding identity; absent only at initial insertion.
    A2aTaskBinding {
        /// Existing stable operation-handle key, derived by the application.
        operation_key: OperationCustodyIndexKey,
        /// Operation-and-binding identity key.
        key: OperationCustodyIndexKey,
        /// Absent for the initial atom; an exact retained binding for successors.
        expected: OperationCustodyIndexPredecessor,
        /// Immutable binding coordinate, unchanged for successor observations.
        successor: ControlRecordCoordinate,
    },
    /// Latest remote observation, independently of the local operation state.
    LatestA2aTaskObservation {
        /// The same operation-and-binding identity key.
        key: OperationCustodyIndexKey,
        /// Absent initially; otherwise the exact remote predecessor.
        expected: OperationCustodyIndexPredecessor,
        /// New admitted remote observation.
        successor: ControlRecordCoordinate,
    },
    /// Derived immutable revision-one remote observation.
    FirstA2aTaskObservation {
        /// The binding identity key.
        key: OperationCustodyIndexKey,
        /// Initial remote observation coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Derived checkpoint closing one exact remote history revision.
    A2aTaskObservationCheckpoint {
        /// Exact observation-coordinate key.
        key: OperationCustodyIndexKey,
        /// Signed checkpoint coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Retain the original request independently of the latest observation.
    RequestForHandle {
        /// The same handle-identity key used by the immutable handle index.
        key: OperationCustodyIndexKey,
        /// Immutable caller-signed common-request coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Retain the initial observation independently of its advancing latest head.
    StartObservation {
        /// The same request-identity key used by request evidence.
        key: OperationCustodyIndexKey,
        /// Immutable initial observation coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Retain the checkpoint that committed this exact observation.
    ObservationCheckpoint {
        /// Exact observation-coordinate key, scoped by this index role and store.
        key: OperationCustodyIndexKey,
        /// Immutable signed checkpoint coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Bind request identity to the exact caller-signed request record.
    RequestEvidence {
        /// Domain-separated request-identity key.
        key: OperationCustodyIndexKey,
        /// Exact required predecessor.
        expected: OperationCustodyIndexPredecessor,
        /// Immutable common-request record coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Bind the same request identity to its durable operation handle.
    RequestOperation {
        /// Domain-separated request-identity key.
        key: OperationCustodyIndexKey,
        /// Exact required predecessor.
        expected: OperationCustodyIndexPredecessor,
        /// Immutable operation-handle record coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Bind stable handle identity to its immutable handle record.
    OperationHandle {
        /// Domain-separated handle-identity key.
        key: OperationCustodyIndexKey,
        /// Exact required predecessor.
        expected: OperationCustodyIndexPredecessor,
        /// Immutable operation-handle record coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Bind one exact parent-plus-child tuple to the child handle.
    ParentChildMembership {
        /// Key derived from both exact parent and child handle coordinates.
        key: OperationCustodyIndexKey,
        /// Exact required predecessor.
        expected: OperationCustodyIndexPredecessor,
        /// Immutable child operation-handle record coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Advance the current private consultation collection for one handle.
    CollectionHead {
        /// Domain-separated operation-handle key.
        key: OperationCustodyIndexKey,
        /// Absent initially, then the exact preceding collection frontier.
        expected: OperationCustodyIndexPredecessor,
        /// New private collection-frontier coordinate.
        successor: ControlRecordCoordinate,
    },
    /// Advance the latest observation for one handle.
    LatestObservation {
        /// Domain-separated operation-handle key.
        key: OperationCustodyIndexKey,
        /// Absent at start, otherwise the exact predecessor observation.
        expected: OperationCustodyIndexPredecessor,
        /// Immutable successor-observation record coordinate.
        successor: ControlRecordCoordinate,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum IndexRole {
    NativeOutcome,
    A2aSendPreparation,
    A2aSendPreparationCheckpoint,
    CurrentA2aGetTask,
    A2aGetTaskCheckpoint,
    A2aTaskBinding,
    LatestA2aTaskObservation,
    FirstA2aTaskObservation,
    A2aTaskObservationCheckpoint,
    RequestForHandle,
    StartObservation,
    ObservationCheckpoint,
    RequestEvidence,
    RequestOperation,
    OperationHandle,
    ParentChildMembership,
    CollectionHead,
    LatestObservation,
}

impl OperationCustodyIndexMutation {
    fn role(&self) -> IndexRole {
        match self {
            Self::NativeOutcome { .. } => IndexRole::NativeOutcome,
            Self::A2aSendPreparation { .. } => IndexRole::A2aSendPreparation,
            Self::A2aSendPreparationCheckpoint { .. } => IndexRole::A2aSendPreparationCheckpoint,
            Self::CurrentA2aGetTask { .. } => IndexRole::CurrentA2aGetTask,
            Self::A2aGetTaskCheckpoint { .. } => IndexRole::A2aGetTaskCheckpoint,
            Self::A2aTaskBinding { .. } => IndexRole::A2aTaskBinding,
            Self::LatestA2aTaskObservation { .. } => IndexRole::LatestA2aTaskObservation,
            Self::FirstA2aTaskObservation { .. } => IndexRole::FirstA2aTaskObservation,
            Self::A2aTaskObservationCheckpoint { .. } => IndexRole::A2aTaskObservationCheckpoint,
            Self::RequestForHandle { .. } => IndexRole::RequestForHandle,
            Self::StartObservation { .. } => IndexRole::StartObservation,
            Self::ObservationCheckpoint { .. } => IndexRole::ObservationCheckpoint,
            Self::RequestEvidence { .. } => IndexRole::RequestEvidence,
            Self::RequestOperation { .. } => IndexRole::RequestOperation,
            Self::OperationHandle { .. } => IndexRole::OperationHandle,
            Self::ParentChildMembership { .. } => IndexRole::ParentChildMembership,
            Self::CollectionHead { .. } => IndexRole::CollectionHead,
            Self::LatestObservation { .. } => IndexRole::LatestObservation,
        }
    }

    fn key(&self) -> &OperationCustodyIndexKey {
        match self {
            Self::NativeOutcome { key, .. }
            | Self::A2aSendPreparation { key, .. }
            | Self::A2aSendPreparationCheckpoint { key, .. }
            | Self::CurrentA2aGetTask { key, .. }
            | Self::A2aGetTaskCheckpoint { key, .. }
            | Self::A2aTaskBinding { key, .. }
            | Self::LatestA2aTaskObservation { key, .. }
            | Self::FirstA2aTaskObservation { key, .. }
            | Self::A2aTaskObservationCheckpoint { key, .. }
            | Self::RequestForHandle { key, .. }
            | Self::StartObservation { key, .. }
            | Self::ObservationCheckpoint { key, .. }
            | Self::RequestEvidence { key, .. }
            | Self::RequestOperation { key, .. }
            | Self::OperationHandle { key, .. }
            | Self::ParentChildMembership { key, .. }
            | Self::CollectionHead { key, .. }
            | Self::LatestObservation { key, .. } => key,
        }
    }

    fn expected(&self) -> &OperationCustodyIndexPredecessor {
        match self {
            Self::A2aSendPreparationCheckpoint { .. }
            | Self::A2aGetTaskCheckpoint { .. }
            | Self::FirstA2aTaskObservation { .. }
            | Self::A2aTaskObservationCheckpoint { .. }
            | Self::RequestForHandle { .. }
            | Self::StartObservation { .. }
            | Self::ObservationCheckpoint { .. } => &OperationCustodyIndexPredecessor::Absent,
            Self::NativeOutcome { expected, .. }
            | Self::A2aSendPreparation { expected, .. }
            | Self::CurrentA2aGetTask { expected, .. }
            | Self::A2aTaskBinding { expected, .. }
            | Self::LatestA2aTaskObservation { expected, .. }
            | Self::RequestEvidence { expected, .. }
            | Self::RequestOperation { expected, .. }
            | Self::OperationHandle { expected, .. }
            | Self::ParentChildMembership { expected, .. }
            | Self::CollectionHead { expected, .. }
            | Self::LatestObservation { expected, .. } => expected,
        }
    }

    fn successor(&self) -> &ControlRecordCoordinate {
        match self {
            Self::NativeOutcome { successor, .. }
            | Self::A2aSendPreparation { successor, .. }
            | Self::A2aSendPreparationCheckpoint { successor, .. }
            | Self::CurrentA2aGetTask { successor, .. }
            | Self::A2aGetTaskCheckpoint { successor, .. }
            | Self::A2aTaskBinding { successor, .. }
            | Self::LatestA2aTaskObservation { successor, .. }
            | Self::FirstA2aTaskObservation { successor, .. }
            | Self::A2aTaskObservationCheckpoint { successor, .. }
            | Self::RequestForHandle { successor, .. }
            | Self::StartObservation { successor, .. }
            | Self::ObservationCheckpoint { successor, .. }
            | Self::RequestEvidence { successor, .. }
            | Self::RequestOperation { successor, .. }
            | Self::OperationHandle { successor, .. }
            | Self::ParentChildMembership { successor, .. }
            | Self::CollectionHead { successor, .. }
            | Self::LatestObservation { successor, .. } => successor,
        }
    }
}

/// Exact initial or predecessor/successor checkpoint evidence for one transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCustodyCheckpoint {
    phase: OperationCheckpointPhase,
    successor: ControlCheckpointHead,
    successor_record: ControlRecord,
}

/// Closed record-bearing checkpoint phase for one atomic operation mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationCheckpointPhase {
    /// No prior checkpoint exists. Native custody must prove whole-domain absence.
    Initial,
    /// Existing exact checkpoint evidence that the successor replaces.
    Successor {
        /// Exact physical predecessor head.
        predecessor: ControlCheckpointHead,
        /// Immutable bytes named by that head.
        predecessor_record: Box<ControlRecord>,
    },
}

impl OperationCustodyCheckpoint {
    /// Check record/head equality and the closed initial/successor relation.
    ///
    /// # Errors
    ///
    /// Returns [`OperationCustodyCandidateError`] when either checkpoint record
    /// does not exactly match its head or the successor does not advance the
    /// same hard-conflict domain by one sequence, or Initial is not sequence one.
    pub fn new(
        phase: OperationCheckpointPhase,
        successor: ControlCheckpointHead,
        successor_record: ControlRecord,
    ) -> Result<Self, OperationCustodyCandidateError> {
        validate_checkpoint_record(&successor, &successor_record)
            .map_err(|()| OperationCustodyCandidateError::SuccessorCheckpoint)?;
        let valid = match &phase {
            OperationCheckpointPhase::Initial => successor.domain_sequence().get() == 1,
            OperationCheckpointPhase::Successor {
                predecessor,
                predecessor_record,
            } => {
                validate_checkpoint_record(predecessor, predecessor_record)
                    .map_err(|()| OperationCustodyCandidateError::PredecessorCheckpoint)?;
                successor.is_exact_successor_of(predecessor)
            }
        };
        if !valid {
            return Err(OperationCustodyCandidateError::CheckpointSuccession);
        }
        Ok(Self {
            phase,
            successor,
            successor_record,
        })
    }

    /// Borrow the closed initial or successor record phase.
    #[must_use]
    pub const fn phase(&self) -> &OperationCheckpointPhase {
        &self.phase
    }

    /// Borrow the immutable predecessor checkpoint record.
    #[must_use]
    pub const fn predecessor_record(&self) -> Option<&ControlRecord> {
        match &self.phase {
            OperationCheckpointPhase::Initial => None,
            OperationCheckpointPhase::Successor {
                predecessor_record, ..
            } => Some(predecessor_record),
        }
    }

    /// Borrow the exact successor checkpoint head.
    #[must_use]
    pub const fn successor(&self) -> &ControlCheckpointHead {
        &self.successor
    }

    /// Borrow the immutable successor checkpoint record.
    #[must_use]
    pub const fn successor_record(&self) -> &ControlRecord {
        &self.successor_record
    }
}

/// Closed operation transition awaiting one atomic comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCustodyCandidate {
    store: AuthorityStoreCoordinate,
    kind: OperationCustodyTransitionKind,
    records: Box<[OperationCustodyRecord]>,
    indexes: Box<[OperationCustodyIndexMutation]>,
    checkpoint: Box<OperationCustodyCheckpoint>,
    accumulator: Box<OperationAccumulatorBatch>,
    collection_journal: Option<Box<OperationCollectionJournal>>,
    session_basis: Option<Box<OperationSessionBasis>>,
    a2a_evidence: Box<[OperationA2aEvidence]>,
    a2a_retained_evidence: Box<[OperationA2aEvidence]>,
    a2a_retained_records: Box<[ControlRecord]>,
    a2a_associated_route: Option<Box<ControlRecord>>,
}

enum LocalTransition {
    StartRoot(OperationSessionBasis),
    StartChild(OperationSessionBasis),
    Observe,
}

impl OperationCustodyCandidate {
    /// Prove an Observe-only physical transaction and derive its recovery index.
    ///
    /// Callers supply the latest-observation mutation. Its immutable
    /// [`OperationCustodyIndexMutation::ObservationCheckpoint`] entry is
    /// derived here, never supplied independently. Starts use [`Self::new_start`].
    ///
    /// # Errors
    ///
    /// Returns [`OperationCustodyCandidateError`] when records, indexes,
    /// predecessors, successor coordinates, or immutable record references do
    /// not form the selected closed transaction.
    pub fn new(
        store: AuthorityStoreCoordinate,
        kind: OperationCustodyTransitionKind,
        records: Vec<OperationCustodyRecord>,
        indexes: Vec<OperationCustodyIndexMutation>,
        checkpoint: OperationCustodyCheckpoint,
        accumulator: OperationAccumulatorBatch,
    ) -> Result<Self, OperationCustodyCandidateError> {
        if kind != OperationCustodyTransitionKind::Observe {
            return Err(OperationCustodyCandidateError::RecordShape);
        }
        Self::new_local(
            store,
            LocalTransition::Observe,
            records,
            indexes,
            checkpoint,
            accumulator,
        )
    }

    /// Construct a root or child start with its required immutable session basis.
    ///
    /// Derives immutable request-for-handle, start-observation and observation-
    /// checkpoint indexes from the exact supplied records and mutations.
    ///
    /// # Errors
    /// Rejects non-start transitions, missing physical association or malformed atoms.
    pub fn new_start(
        store: AuthorityStoreCoordinate,
        kind: OperationCustodyTransitionKind,
        records: Vec<OperationCustodyRecord>,
        indexes: Vec<OperationCustodyIndexMutation>,
        checkpoint: OperationCustodyCheckpoint,
        accumulator: OperationAccumulatorBatch,
        session_basis: OperationSessionBasis,
    ) -> Result<Self, OperationCustodyCandidateError> {
        let transition = match kind {
            OperationCustodyTransitionKind::StartRoot => LocalTransition::StartRoot(session_basis),
            OperationCustodyTransitionKind::StartChild => {
                LocalTransition::StartChild(session_basis)
            }
            _ => return Err(OperationCustodyCandidateError::RecordShape),
        };
        Self::new_local(store, transition, records, indexes, checkpoint, accumulator)
    }

    fn new_local(
        store: AuthorityStoreCoordinate,
        transition: LocalTransition,
        records: Vec<OperationCustodyRecord>,
        mut indexes: Vec<OperationCustodyIndexMutation>,
        checkpoint: OperationCustodyCheckpoint,
        accumulator: OperationAccumulatorBatch,
    ) -> Result<Self, OperationCustodyCandidateError> {
        let (kind, session_basis) = match transition {
            LocalTransition::StartRoot(basis) => {
                (OperationCustodyTransitionKind::StartRoot, Some(basis))
            }
            LocalTransition::StartChild(basis) => {
                (OperationCustodyTransitionKind::StartChild, Some(basis))
            }
            LocalTransition::Observe => (OperationCustodyTransitionKind::Observe, None),
        };
        let phase_matches = match (accumulator.binding().phase(), checkpoint.phase()) {
            (crate::AccumulatorCheckpointPhase::Initial, OperationCheckpointPhase::Initial) => true,
            (
                crate::AccumulatorCheckpointPhase::Successor(left),
                OperationCheckpointPhase::Successor {
                    predecessor: right, ..
                },
            ) => left == right,
            _ => false,
        };
        if accumulator.binding().store() != &store
            || !phase_matches
            || (matches!(checkpoint.phase(), OperationCheckpointPhase::Initial)
                && kind != OperationCustodyTransitionKind::StartRoot)
            || accumulator.binding().successor() != checkpoint.successor()
        {
            return Err(OperationCustodyCandidateError::AccumulatorBinding);
        }
        if records
            .iter()
            .any(|record| record.kind() == OperationCustodyRecordKind::CollectionFrontier)
        {
            return Err(OperationCustodyCandidateError::CollectionJournalBinding);
        }
        validate_records(kind, &records)?;
        validate_indexes(kind, &records, &indexes)?;
        validate_record_references(&records, &indexes, &checkpoint)?;
        if let Some(basis) = &session_basis {
            for (role, coordinate) in [
                (OperationCustodyRecordKind::OperationHandle, basis.handle()),
                (OperationCustodyRecordKind::CommonRequest, basis.request()),
            ] {
                if !records.iter().any(|record| {
                    record.kind() == role && record.record().coordinate() == coordinate
                }) {
                    return Err(OperationCustodyCandidateError::SessionBasis);
                }
            }
        }
        let observation = records
            .iter()
            .find(|record| record.kind() == OperationCustodyRecordKind::OperationObservation)
            .ok_or(OperationCustodyCandidateError::RecordShape)?
            .record()
            .coordinate();
        if let Some(key) = indexes.iter().find_map(|index| match index {
            OperationCustodyIndexMutation::OperationHandle { key, .. } => Some(key.clone()),
            _ => None,
        }) {
            let request = records
                .iter()
                .find(|record| record.kind() == OperationCustodyRecordKind::CommonRequest)
                .ok_or(OperationCustodyCandidateError::RecordShape)?;
            indexes.push(OperationCustodyIndexMutation::RequestForHandle {
                key,
                successor: request.record().coordinate().clone(),
            });
        }
        if let Some(key) = indexes.iter().find_map(|index| match index {
            OperationCustodyIndexMutation::RequestEvidence { key, .. } => Some(key.clone()),
            _ => None,
        }) {
            indexes.push(OperationCustodyIndexMutation::StartObservation {
                key,
                successor: observation.clone(),
            });
        }
        indexes.push(OperationCustodyIndexMutation::ObservationCheckpoint {
            key: OperationCustodyIndexKey::for_observation(observation),
            successor: checkpoint.successor_record().coordinate().clone(),
        });
        Ok(Self {
            store,
            kind,
            records: records.into_boxed_slice(),
            indexes: indexes.into_boxed_slice(),
            checkpoint: Box::new(checkpoint),
            accumulator: Box::new(accumulator),
            collection_journal: None,
            session_basis: session_basis.map(Box::new),
            a2a_evidence: Box::new([]),
            a2a_retained_evidence: Box::new([]),
            a2a_retained_records: Box::new([]),
            a2a_associated_route: None,
        })
    }

    /// Attach a derived collection frontier and its exact journal append.
    /// This does not admit the signed observation's semantic association.
    ///
    /// # Errors
    /// Rejects starts, an existing optional role, unlike physical transactions,
    /// or any immutable reference alias.
    pub fn with_collection_journal(
        self,
        journal: OperationCollectionJournal,
    ) -> Result<Self, OperationCustodyCandidateError> {
        self.with_collection_journal_after(journal, None)
    }

    /// Attach a derived collection frontier after an exact retained frontier.
    ///
    /// Initial collection uses `None`; every later collection state must supply
    /// the complete current frontier record. This is a private compare-and-swap
    /// index, not semantic consultation admission. A consultation cancellation
    /// may also retain its one signed request in the same transaction; no other
    /// optional-record combination is accepted here.
    ///
    /// # Errors
    /// Rejects an unlike transition, transaction, predecessor, or record set.
    pub fn with_collection_journal_after(
        mut self,
        journal: OperationCollectionJournal,
        predecessor: Option<ControlRecord>,
    ) -> Result<Self, OperationCustodyCandidateError> {
        let common_request_count = self
            .records
            .iter()
            .filter(|record| record.kind() == OperationCustodyRecordKind::CommonRequest)
            .count();
        let collection_record_shape =
            self.records.len() == 1 || (self.records.len() == 2 && common_request_count == 1);
        if self.kind != OperationCustodyTransitionKind::Observe
            || !collection_record_shape
            || self.collection_journal.is_some()
            || self.accumulator.binding().transaction() != journal.transaction()
            || self
                .indexes
                .iter()
                .any(|index| matches!(index, OperationCustodyIndexMutation::CollectionHead { .. }))
            || predecessor.as_ref().is_some_and(|record| {
                record.schema_id().as_str() != PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA
                    || record.coordinate() == journal.frontier().coordinate()
            })
        {
            return Err(OperationCustodyCandidateError::CollectionJournalBinding);
        }
        let mut records = self.records.into_vec();
        records.push(OperationCustodyRecord::CollectionFrontier(
            journal.frontier().clone(),
        ));
        let mut indexes = self.indexes.into_vec();
        indexes.push(OperationCustodyIndexMutation::CollectionHead {
            key: OperationCustodyIndexKey::for_collection_head(journal.operation_handle()),
            expected: predecessor.map_or(
                OperationCustodyIndexPredecessor::Absent,
                OperationCustodyIndexPredecessor::Exact,
            ),
            successor: journal.frontier().coordinate().clone(),
        });
        validate_records(self.kind, &records)?;
        validate_record_references(&records, &indexes, &self.checkpoint)?;
        self.records = records.into_boxed_slice();
        self.indexes = indexes.into_boxed_slice();
        self.collection_journal = Some(Box::new(journal));
        Ok(self)
    }

    /// Attach one or two exact A2A protocol items to an observation append.
    ///
    /// The application must separately authenticate the item's meaning and add
    /// its logical leaf to the same checkpoint. This method only closes the
    /// physical transaction: the bytes must be new, reference-distinct, and
    /// committed with the signed observation. It does not create an A2A Task
    /// binding or grant permission to replay a request.
    ///
    /// # Errors
    /// Rejects non-observation transitions, an empty or oversized collection,
    /// a second attachment, duplicate references, or a reference that aliases
    /// any signed record or checkpoint in this transaction.
    pub fn with_a2a_evidence_items(
        mut self,
        evidence: Vec<OperationA2aEvidence>,
    ) -> Result<Self, OperationCustodyCandidateError> {
        if self.kind != OperationCustodyTransitionKind::Observe
            || !self.a2a_evidence.is_empty()
            || !self.a2a_retained_evidence.is_empty()
            || !self.a2a_retained_records.is_empty()
            || self.a2a_associated_route.is_some()
            || !(1..=2).contains(&evidence.len())
        {
            return Err(OperationCustodyCandidateError::A2aEvidence);
        }
        for (index, item) in evidence.iter().enumerate() {
            let reference = item.coordinate().record_ref();
            let aliased = evidence[..index]
                .iter()
                .any(|prior| prior.coordinate().record_ref() == reference)
                || self
                    .records
                    .iter()
                    .any(|record| record.record().coordinate().record_ref() == reference)
                || self.indexes.iter().any(|index| {
                    index.successor().record_ref() == reference
                        || matches!(
                            index.expected(),
                            OperationCustodyIndexPredecessor::Exact(record)
                                if record.coordinate().record_ref() == reference
                        )
                })
                || self
                    .checkpoint
                    .predecessor_record()
                    .is_some_and(|record| record.coordinate().record_ref() == reference)
                || self.checkpoint.successor_record().coordinate().record_ref() == reference;
            if aliased {
                return Err(OperationCustodyCandidateError::DuplicateRecordReference);
            }
        }
        self.a2a_evidence = evidence.into_boxed_slice();
        Ok(self)
    }

    /// Borrow the mandatory pair when this candidate carries a collection frontier.
    #[must_use]
    pub fn collection_journal(&self) -> Option<&OperationCollectionJournal> {
        self.collection_journal.as_deref()
    }

    /// Required for starts and absent for every successor transition.
    #[must_use]
    pub fn session_basis(&self) -> Option<&OperationSessionBasis> {
        self.session_basis.as_deref()
    }

    /// Return whether this candidate carries protocol-specific or collection
    /// material beyond its closed record and index sets.
    ///
    /// Semantic finalizers use this to reject a physically valid candidate
    /// that extends the transition they originally prepared.
    #[must_use]
    pub fn has_auxiliary_evidence(&self) -> bool {
        self.collection_journal.is_some()
            || !self.a2a_evidence.is_empty()
            || !self.a2a_retained_evidence.is_empty()
            || !self.a2a_retained_records.is_empty()
            || self.a2a_associated_route.is_some()
    }

    /// Borrow the exact selected store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the closed physical transition shape.
    #[must_use]
    pub const fn kind(&self) -> OperationCustodyTransitionKind {
        self.kind
    }

    /// Borrow the complete immutable record set.
    #[must_use]
    pub const fn records(&self) -> &[OperationCustodyRecord] {
        &self.records
    }

    /// Borrow the complete typed index mutation set.
    #[must_use]
    pub const fn indexes(&self) -> &[OperationCustodyIndexMutation] {
        &self.indexes
    }

    /// Borrow the hard-domain checkpoint successor.
    #[must_use]
    pub const fn checkpoint(&self) -> &OperationCustodyCheckpoint {
        &self.checkpoint
    }

    /// Borrow the mandatory exact physical accumulator contribution.
    #[must_use]
    pub const fn accumulator(&self) -> &OperationAccumulatorBatch {
        &self.accumulator
    }

    /// Find the sole record occupying `kind` in this validated candidate.
    #[must_use]
    pub fn record(&self, kind: OperationCustodyRecordKind) -> Option<&ControlRecord> {
        self.records
            .iter()
            .find(|record| record.kind() == kind)
            .map(OperationCustodyRecord::record)
    }
}

fn validate_record_references(
    records: &[OperationCustodyRecord],
    indexes: &[OperationCustodyIndexMutation],
    checkpoint: &OperationCustodyCheckpoint,
) -> Result<(), OperationCustodyCandidateError> {
    let mut record_refs = BTreeSet::new();
    let references_are_unique = records
        .iter()
        .map(|record| record.record().coordinate().record_ref())
        .chain(indexes.iter().filter_map(|index| match index.expected() {
            OperationCustodyIndexPredecessor::Absent => None,
            OperationCustodyIndexPredecessor::Exact(record) => {
                Some(record.coordinate().record_ref())
            }
        }))
        .chain(
            checkpoint
                .predecessor_record()
                .map(|record| record.coordinate().record_ref()),
        )
        .chain([checkpoint.successor_record().coordinate().record_ref()])
        .all(|record_ref| record_refs.insert(record_ref));
    if !references_are_unique {
        return Err(OperationCustodyCandidateError::DuplicateRecordReference);
    }
    Ok(())
}

fn validate_records(
    kind: OperationCustodyTransitionKind,
    records: &[OperationCustodyRecord],
) -> Result<(), OperationCustodyCandidateError> {
    let observed = records
        .iter()
        .map(OperationCustodyRecord::kind)
        .collect::<BTreeSet<_>>();
    let mut expected = required_record_roles(kind)?;
    if kind == OperationCustodyTransitionKind::Observe {
        let optional = [
            OperationCustodyRecordKind::CapabilityResult,
            OperationCustodyRecordKind::AdapterFailure,
            OperationCustodyRecordKind::AttemptSelection,
            OperationCustodyRecordKind::CommonRequest,
            OperationCustodyRecordKind::Diagnostic,
            OperationCustodyRecordKind::CollectionFrontier,
            OperationCustodyRecordKind::NativeOutcome,
            OperationCustodyRecordKind::HookResponse,
        ];
        let selected = optional
            .into_iter()
            .filter(|role| observed.contains(role))
            .collect::<BTreeSet<_>>();
        let safe_provider_failure = selected
            == [
                OperationCustodyRecordKind::AdapterFailure,
                OperationCustodyRecordKind::CapabilityResult,
                OperationCustodyRecordKind::Diagnostic,
            ]
            .into_iter()
            .collect();
        let consultation_cancellation = selected
            == [
                OperationCustodyRecordKind::CollectionFrontier,
                OperationCustodyRecordKind::CommonRequest,
            ]
            .into_iter()
            .collect();
        if selected.len() == 1 || safe_provider_failure || consultation_cancellation {
            expected.extend(selected);
        } else if !selected.is_empty() {
            return Err(OperationCustodyCandidateError::RecordShape);
        }
    }
    if observed.len() != records.len() || observed != expected {
        return Err(OperationCustodyCandidateError::RecordShape);
    }
    if records
        .iter()
        .any(|record| record.record().schema_id().as_str() != record.kind().schema_id())
    {
        return Err(OperationCustodyCandidateError::RecordSchema);
    }
    Ok(())
}

fn validate_indexes(
    kind: OperationCustodyTransitionKind,
    records: &[OperationCustodyRecord],
    indexes: &[OperationCustodyIndexMutation],
) -> Result<(), OperationCustodyCandidateError> {
    let observed = indexes
        .iter()
        .map(OperationCustodyIndexMutation::role)
        .collect::<BTreeSet<_>>();
    let mut expected = required_index_roles(kind)?;
    if records
        .iter()
        .any(|record| record.kind() == OperationCustodyRecordKind::CollectionFrontier)
    {
        expected.insert(IndexRole::CollectionHead);
    }
    if records
        .iter()
        .any(|record| record.kind() == OperationCustodyRecordKind::NativeOutcome)
    {
        expected.insert(IndexRole::NativeOutcome);
    }
    if observed.len() != indexes.len() || observed != expected {
        return Err(OperationCustodyCandidateError::IndexShape);
    }
    let keyed_roles = indexes
        .iter()
        .map(|index| (index.role(), index.key()))
        .collect::<BTreeSet<_>>();
    if keyed_roles.len() != indexes.len() {
        return Err(OperationCustodyCandidateError::DuplicateIndex);
    }
    if matches!(
        kind,
        OperationCustodyTransitionKind::StartRoot | OperationCustodyTransitionKind::StartChild
    ) {
        let key_for = |role| {
            indexes
                .iter()
                .find(|index| index.role() == role)
                .expect("validated start shape contains every required index")
                .key()
        };
        if key_for(IndexRole::RequestEvidence) != key_for(IndexRole::RequestOperation)
            || key_for(IndexRole::OperationHandle) != key_for(IndexRole::LatestObservation)
        {
            return Err(OperationCustodyCandidateError::IndexKeyPair);
        }
    }
    for index in indexes {
        let predecessor_valid = match (kind, index.role(), index.expected()) {
            (
                OperationCustodyTransitionKind::Observe,
                IndexRole::NativeOutcome,
                OperationCustodyIndexPredecessor::Absent,
            ) => true,
            (
                OperationCustodyTransitionKind::Observe,
                IndexRole::LatestObservation,
                OperationCustodyIndexPredecessor::Exact(record),
            ) => {
                record.schema_id().as_str() == OPERATION_OBSERVATION_V1_SCHEMA
                    && record.coordinate() != index.successor()
            }
            (
                OperationCustodyTransitionKind::Observe,
                IndexRole::CollectionHead,
                OperationCustodyIndexPredecessor::Absent,
            ) => true,
            (
                OperationCustodyTransitionKind::Observe,
                IndexRole::CollectionHead,
                OperationCustodyIndexPredecessor::Exact(record),
            ) => {
                record.schema_id().as_str() == PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA
                    && record.coordinate() != index.successor()
            }
            (
                OperationCustodyTransitionKind::StartRoot
                | OperationCustodyTransitionKind::StartChild,
                _,
                OperationCustodyIndexPredecessor::Absent,
            ) => true,
            _ => false,
        };
        if !predecessor_valid {
            return Err(OperationCustodyCandidateError::IndexPredecessor);
        }
        let successor_kind = match index.role() {
            IndexRole::A2aSendPreparation
            | IndexRole::A2aSendPreparationCheckpoint
            | IndexRole::CurrentA2aGetTask
            | IndexRole::A2aGetTaskCheckpoint
            | IndexRole::A2aTaskBinding
            | IndexRole::FirstA2aTaskObservation
            | IndexRole::LatestA2aTaskObservation
            | IndexRole::A2aTaskObservationCheckpoint => {
                return Err(OperationCustodyCandidateError::IndexShape);
            }
            IndexRole::RequestForHandle
            | IndexRole::StartObservation
            | IndexRole::ObservationCheckpoint => {
                return Err(OperationCustodyCandidateError::IndexShape);
            }
            IndexRole::RequestEvidence => OperationCustodyRecordKind::CommonRequest,
            IndexRole::NativeOutcome => OperationCustodyRecordKind::NativeOutcome,
            IndexRole::RequestOperation
            | IndexRole::OperationHandle
            | IndexRole::ParentChildMembership => OperationCustodyRecordKind::OperationHandle,
            IndexRole::CollectionHead => OperationCustodyRecordKind::CollectionFrontier,
            IndexRole::LatestObservation => OperationCustodyRecordKind::OperationObservation,
        };
        if !records.iter().any(|record| {
            record.kind() == successor_kind && record.record().coordinate() == index.successor()
        }) {
            return Err(OperationCustodyCandidateError::IndexSuccessor);
        }
    }
    Ok(())
}

fn required_record_roles(
    kind: OperationCustodyTransitionKind,
) -> Result<BTreeSet<OperationCustodyRecordKind>, OperationCustodyCandidateError> {
    Ok(match kind {
        OperationCustodyTransitionKind::BindA2aTask
        | OperationCustodyTransitionKind::ObserveA2aTask
        | OperationCustodyTransitionKind::PrepareA2aSend
        | OperationCustodyTransitionKind::PrepareA2aGetTask
        | OperationCustodyTransitionKind::DisposeA2aGetTask => {
            return Err(OperationCustodyCandidateError::RecordShape);
        }
        OperationCustodyTransitionKind::StartRoot | OperationCustodyTransitionKind::StartChild => [
            OperationCustodyRecordKind::CommonRequest,
            OperationCustodyRecordKind::OperationHandle,
            OperationCustodyRecordKind::OperationObservation,
        ]
        .into_iter()
        .collect(),
        OperationCustodyTransitionKind::Observe => {
            [OperationCustodyRecordKind::OperationObservation]
                .into_iter()
                .collect()
        }
    })
}

fn required_index_roles(
    kind: OperationCustodyTransitionKind,
) -> Result<BTreeSet<IndexRole>, OperationCustodyCandidateError> {
    Ok(match kind {
        OperationCustodyTransitionKind::BindA2aTask
        | OperationCustodyTransitionKind::ObserveA2aTask
        | OperationCustodyTransitionKind::PrepareA2aSend
        | OperationCustodyTransitionKind::PrepareA2aGetTask
        | OperationCustodyTransitionKind::DisposeA2aGetTask => {
            return Err(OperationCustodyCandidateError::IndexShape);
        }
        OperationCustodyTransitionKind::StartRoot => [
            IndexRole::RequestEvidence,
            IndexRole::RequestOperation,
            IndexRole::OperationHandle,
            IndexRole::LatestObservation,
        ]
        .into_iter()
        .collect(),
        OperationCustodyTransitionKind::StartChild => [
            IndexRole::RequestEvidence,
            IndexRole::RequestOperation,
            IndexRole::OperationHandle,
            IndexRole::ParentChildMembership,
            IndexRole::LatestObservation,
        ]
        .into_iter()
        .collect(),
        OperationCustodyTransitionKind::Observe => {
            [IndexRole::LatestObservation].into_iter().collect()
        }
    })
}

/// Candidate construction failure before storage access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OperationCustodyCandidateError {
    /// The required start attachment is unbounded or associated with unlike records.
    #[error("operation session basis is unbounded or incorrectly associated")]
    SessionBasis,
    /// Exact raw evidence or its retained-parent closure is malformed.
    #[error("A2A custody evidence is unbounded, changed or incomplete")]
    A2aEvidence,
    /// The private frontier is not paired with this exact observation transaction.
    #[error("operation collection frontier and journal do not form the exact paired attachment")]
    CollectionJournalBinding,
    /// Accumulator and operation do not retain the same store/checkpoint pair.
    #[error("operation accumulator binding differs from custody")]
    AccumulatorBinding,
    /// Record roles do not exactly match the selected transaction.
    #[error("operation custody transition has the wrong record shape")]
    RecordShape,
    /// A record role carries the wrong canonical schema.
    #[error("operation custody record uses the wrong schema")]
    RecordSchema,
    /// Index roles do not exactly match the selected transaction.
    #[error("operation custody transition has the wrong index shape")]
    IndexShape,
    /// A typed index role appears more than once.
    #[error("operation custody transition repeats a typed index")]
    DuplicateIndex,
    /// Paired start indexes do not share their request or handle identity key.
    #[error("operation custody start indexes do not share their paired identity key")]
    IndexKeyPair,
    /// An index does not assert the required exact predecessor.
    #[error("operation custody index has the wrong predecessor")]
    IndexPredecessor,
    /// An index successor is absent from the transaction record set.
    #[error("operation custody index successor is not in the transaction")]
    IndexSuccessor,
    /// Candidate, exact-predecessor, or checkpoint records reuse an immutable reference.
    #[error("operation custody transition repeats an immutable record reference")]
    DuplicateRecordReference,
    /// Predecessor checkpoint head and record disagree.
    #[error("operation custody predecessor checkpoint record is not exact")]
    PredecessorCheckpoint,
    /// Successor checkpoint head and record disagree.
    #[error("operation custody successor checkpoint record is not exact")]
    SuccessorCheckpoint,
    /// Checkpoint domain, epoch, sequence, or coordinate does not advance exactly once.
    #[error("operation custody checkpoint is not the exact domain successor")]
    CheckpointSuccession,
}

/// Closed point-query family for restart-safe operation lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationCustodyIndexQuery {
    /// Resolve the staged outcome for one exact dispatched native attempt.
    NativeOutcome(OperationCustodyIndexKey),
    /// Resolve the request preparation for one exact dispatched attempt.
    A2aSendPreparation(OperationCustodyIndexKey),
    /// Resolve the checkpoint that retained one exact request preparation.
    A2aSendPreparationCheckpoint(OperationCustodyIndexKey),
    /// Resolve the current physical `GetTask` phase for one binding.
    CurrentA2aGetTask(OperationCustodyIndexKey),
    /// Resolve the checkpoint retaining one private `GetTask` record.
    A2aGetTaskCheckpoint(OperationCustodyIndexKey),
    /// Resolve one operation-scoped immutable task binding.
    A2aTaskBinding(OperationCustodyIndexKey),
    /// Resolve its immutable initial remote observation.
    FirstA2aTaskObservation(OperationCustodyIndexKey),
    /// Resolve its current remote history head.
    LatestA2aTaskObservation(OperationCustodyIndexKey),
    /// Resolve the checkpoint that retained one remote observation.
    A2aTaskObservationCheckpoint(OperationCustodyIndexKey),
    /// Resolve the original caller-signed request using the handle-identity key.
    RequestForHandle(OperationCustodyIndexKey),
    /// Resolve the immutable initial observation using the original request key.
    StartObservation(OperationCustodyIndexKey),
    /// Resolve the checkpoint that committed an exact observation.
    ObservationCheckpoint(OperationCustodyIndexKey),
    /// Resolve exact caller-signed request evidence by request identity.
    RequestEvidence(OperationCustodyIndexKey),
    /// Resolve the durable operation handle created for an exact request.
    RequestOperation(OperationCustodyIndexKey),
    /// Resolve one immutable operation handle by handle identity.
    OperationHandle(OperationCustodyIndexKey),
    /// Prove one exact parent-plus-child membership.
    ParentChildMembership(OperationCustodyIndexKey),
    /// Resolve the current private consultation collection for one handle.
    CollectionHead(OperationCustodyIndexKey),
    /// Resolve the latest predecessor-linked observation for one handle.
    LatestObservation(OperationCustodyIndexKey),
}

impl OperationCustodyIndexQuery {
    /// Borrow the domain-separated point-query key.
    #[must_use]
    pub const fn key(&self) -> &OperationCustodyIndexKey {
        match self {
            Self::NativeOutcome(key)
            | Self::A2aSendPreparation(key)
            | Self::A2aSendPreparationCheckpoint(key)
            | Self::CurrentA2aGetTask(key)
            | Self::A2aGetTaskCheckpoint(key)
            | Self::A2aTaskBinding(key)
            | Self::FirstA2aTaskObservation(key)
            | Self::LatestA2aTaskObservation(key)
            | Self::A2aTaskObservationCheckpoint(key)
            | Self::RequestForHandle(key)
            | Self::StartObservation(key)
            | Self::ObservationCheckpoint(key)
            | Self::RequestEvidence(key)
            | Self::RequestOperation(key)
            | Self::OperationHandle(key)
            | Self::ParentChildMembership(key)
            | Self::CollectionHead(key)
            | Self::LatestObservation(key) => key,
        }
    }
}

/// Exact result of one typed private-index point read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCustodyIndexObservation {
    query: OperationCustodyIndexQuery,
    record: Option<ControlRecord>,
}

impl OperationCustodyIndexObservation {
    /// Construct an exact absent observation for `query`.
    #[must_use]
    pub const fn absent(query: OperationCustodyIndexQuery) -> Self {
        Self {
            query,
            record: None,
        }
    }

    /// Bind an exact stored record to the selected index domain.
    ///
    /// # Errors
    /// Returns [`OperationCustodyObservationError::RecordSchema`] when the
    /// record cannot inhabit the selected index.
    pub fn exact(
        query: OperationCustodyIndexQuery,
        record: ControlRecord,
    ) -> Result<Self, OperationCustodyObservationError> {
        query_accepts_schema(&query, record.schema_id().as_str())
            .then_some(Self {
                query,
                record: Some(record),
            })
            .ok_or(OperationCustodyObservationError::RecordSchema)
    }

    /// Borrow the exact point query answered by this observation.
    #[must_use]
    pub const fn query(&self) -> &OperationCustodyIndexQuery {
        &self.query
    }

    /// Borrow the exact record when present.
    #[must_use]
    pub const fn record(&self) -> Option<&ControlRecord> {
        self.record.as_ref()
    }

    /// Take the stored record without copying its retained bytes.
    #[must_use]
    pub fn into_record(self) -> Option<ControlRecord> {
        self.record
    }
}

/// Typed point-observation construction failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OperationCustodyObservationError {
    /// The returned record cannot inhabit the selected index domain.
    #[error("operation custody index returned a record from the wrong schema")]
    RecordSchema,
}

/// Successful physical operation-custody disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationCustodyCommitOutcome {
    /// The selected custody material committed during this call.
    Committed,
    /// The complete exact custody material already existed byte-for-byte.
    ExactReplay,
}

/// Exact read-back classification after an ambiguous commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationCustodyReadBackOutcome {
    /// The phase's exact predecessor or initial-domain absence remains, and no
    /// candidate rows exist. This is not an independent-anchor observation.
    Predecessor,
    /// Every candidate record, index, and successor exists exactly.
    ExactSuccessor,
    /// Storage is partial, contradictory, or owned by another transition.
    ConflictingOrPartial,
}

/// Replaceable atomic physical custody for admitted Pale operations.
pub trait OperationCustodyPort: crate::ControlTransactionPort {
    /// Recover the current private consultation snapshot by exact handle identity.
    ///
    /// This is a bounded point read. It does not scan Genealogy, admit roster
    /// meaning, or establish that the operation may continue. A present result
    /// reports the exact handle-scoped query and coordinate observed with the
    /// frontier in the same physical read. The application verifies that join;
    /// the observation is not an independent freshness proof.
    ///
    /// # Errors
    /// Rejects unsupported, excessive, partial, changed, or inconsistent custody.
    fn read_collection_snapshot(
        &mut self,
        _store: &AuthorityStoreCoordinate,
        _handle: &ControlRecordCoordinate,
        _maximum_frontier_bytes: usize,
        _maximum_journal_payload_bytes: usize,
    ) -> Result<Option<OperationCollectionSnapshot>, ControlStoreError> {
        Err(ControlStoreError::new(
            crate::ControlStoreErrorKind::RecoveryRequired,
        ))
    }

    /// Read a historical collection frontier and its journal by exact identity.
    ///
    /// The result retains the actual current handle-scoped head observed in the
    /// same read; it does not pretend the requested frontier is current. Callers
    /// must admit the historical observation and collection independently.
    ///
    /// # Errors
    /// Rejects unsupported, oversized, partial or inconsistent custody, including
    /// a requested frontier that does not belong to the supplied handle.
    fn read_collection_snapshot_at(
        &mut self,
        _store: &AuthorityStoreCoordinate,
        _handle: &ControlRecordCoordinate,
        _frontier: &ControlRecordCoordinate,
        _maximum_frontier_bytes: usize,
        _maximum_journal_payload_bytes: usize,
    ) -> Result<Option<OperationCollectionSnapshot>, ControlStoreError> {
        Err(ControlStoreError::new(
            crate::ControlStoreErrorKind::RecoveryRequired,
        ))
    }

    /// Read the exact bounded private attachment for an operation handle.
    /// Physical bytes do not grant session authority; callers must prove their
    /// checkpoint membership and admit their application-owned content.
    ///
    /// # Errors
    /// Rejects unsupported, oversized, changed or inconsistent custody.
    fn read_operation_session_basis(
        &mut self,
        _store: &AuthorityStoreCoordinate,
        _handle: &ControlRecordCoordinate,
        _maximum_bytes: usize,
    ) -> Result<Option<OperationSessionBasis>, ControlStoreError> {
        Err(ControlStoreError::new(
            crate::ControlStoreErrorKind::RecoveryRequired,
        ))
    }

    /// Discover committed operation starts in one frozen transaction prefix.
    ///
    /// The returned session-basis bytes still require application admission.
    /// Empty or exhausted pages do not prove that an operation is absent.
    ///
    /// # Errors
    /// Rejects invalid limits, changed cursors, and incomplete physical custody.
    fn read_operation_starts(
        &mut self,
        _store: &AuthorityStoreCoordinate,
        _cursor: Option<&OperationStartCursor>,
        _maximum_sources: usize,
        _maximum_session_basis_bytes: usize,
    ) -> Result<OperationStartSources, OperationStartReadError> {
        Err(ControlStoreError::new(crate::ControlStoreErrorKind::RecoveryRequired).into())
    }

    /// Read exact protocol bytes and bound metadata without JCS rewriting.
    ///
    /// The bound includes raw bytes and SSE ID/type. Absence is not evidence
    /// of a missing remote task. A caller must re-admit source meaning and
    /// checkpoint membership before use. Implementations without raw evidence
    /// custody fail closed rather than manufacturing an absent observation.
    ///
    /// # Errors
    /// Rejects unavailable, changed, oversized or inconsistent custody.
    fn read_a2a_evidence(
        &mut self,
        _store: &AuthorityStoreCoordinate,
        _coordinate: &ControlRecordCoordinate,
        _maximum_bytes: usize,
    ) -> Result<Option<OperationA2aEvidence>, ControlStoreError> {
        Err(ControlStoreError::new(
            crate::ControlStoreErrorKind::RecoveryRequired,
        ))
    }

    /// Read the bounded raw A2A evidence inserted by one exact transaction.
    ///
    /// This is a physical transaction association, not protocol admission. It
    /// lets an admitted checkpoint recover optional evidence without already
    /// knowing the evidence coordinate. Implementations must reject excess,
    /// partial, cross-namespace, or later-transaction material.
    ///
    /// # Errors
    /// Rejects unsupported reads, zero or exceeded bounds, and inconsistent custody.
    fn read_a2a_evidence_inserted_at(
        &mut self,
        _store: &AuthorityStoreCoordinate,
        _transaction: ControlTransactionSequenceV1,
        _maximum_items: usize,
        _maximum_bytes: usize,
    ) -> Result<Vec<OperationA2aEvidence>, ControlStoreError> {
        Err(ControlStoreError::new(
            crate::ControlStoreErrorKind::RecoveryRequired,
        ))
    }
    /// Discover physical binding candidates for one exact operation in a bounded
    /// retained transaction prefix. Neither empty nor exhausted proves remote
    /// absence. Signatures, history, checkpoint membership and inspection rights
    /// remain with their existing application owners.
    /// # Errors
    /// Invalid or exhausted limits return `BoundExceeded`; inconsistent scope,
    /// cursor, index or same-transaction custody fails closed.
    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &OperationA2aBindingScope,
        cursor: Option<&OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationA2aBindings, ControlStoreError>;
    /// Discover committed local and remote observation/checkpoint pairs in one snapshot.
    ///
    /// Start without a cursor to capture the current transaction frontier.
    /// Continuation uses an indexed seek after the last returned source and
    /// excludes later commits. Each of the two schema-indexed seeks examines at
    /// most `maximum_sources + 1` coordinates; their merge returns at most
    /// `maximum_sources` bodies with one aggregate `maximum_body_bytes` bound
    /// checked before allocation. Both limits must be positive and within the
    /// published ceilings. Remote observations remain remote history, not local
    /// operation completion or acceptance.
    /// A missing checkpoint association or unlike insertion transaction fails
    /// the whole page. No source is silently skipped. This does not admit the
    /// records, establish publication status or authorize a notification.
    ///
    /// # Errors
    /// Returns a distinct limit failure without a partial page. Storage failures
    /// retain their recovery classification. No call mutates source custody.
    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationPublicationSources, OperationPublicationReadError>;

    /// Read the current indexed domain head and its bounded opaque record.
    /// Whole-domain absence returns `None`; partial indexed state is recovery
    /// required. No signature, profile admission or independent anchor is proved.
    ///
    /// # Errors
    /// Fails on unavailable, malformed, incomplete or excessive stored evidence.
    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &pale_core::records::ControlCheckpointDomainIdV1,
        maximum_record_bytes: usize,
    ) -> Result<Option<(ControlCheckpointHead, crate::ControlRecord)>, ControlStoreError>;

    /// Verify selected state leaves against one exact current checkpoint in a
    /// read snapshot. A changed checkpoint is a conflict, not an automatic retry.
    /// The adapter must verify its actual bounded proof without tree updates.
    ///
    /// # Errors
    /// Fails on exhausted work/proof limits, unavailable or corrupt state, wrong
    /// membership, or a checkpoint that is no longer the selected head.
    fn verify_accumulator_membership(
        &mut self,
        request: &crate::OperationAccumulatorMembership,
    ) -> Result<
        crate::VerifiedOperationAccumulatorMembership,
        crate::OperationAccumulatorPreparationError,
    >;

    /// Compute finite tree changes against one exact read snapshot, without writes.
    ///
    /// The application owns profile admission and logical leaf classification.
    /// For Initial the adapter proves whole-domain indexed absence across all
    /// epochs/profiles; for Successor it verifies selected predecessor leaves
    /// and their proof against the supplied admitted roots. It then compares the proof-derived successor
    /// with the actual update. Both trees share one snapshot, which must end
    /// before return. No database lock, checkpoint signature, or authority is
    /// returned. The later commit repeats the exact predecessor comparison.
    ///
    /// # Errors
    /// Returns a distinct limit error for exhausted work/data ceilings. Missing,
    /// corrupt, or mismatched physical evidence retains its storage/recovery
    /// classification. Any failure discards preparation; it cannot authorize a
    /// write, retry of an uncertain effect, or independent anchor advancement.
    fn prepare_accumulator(
        &mut self,
        request: &crate::OperationAccumulatorPreparation,
    ) -> Result<crate::PreparedOperationAccumulator, crate::OperationAccumulatorPreparationError>;

    /// Reconstruct an already committed successor from one bounded read snapshot.
    ///
    /// The application supplies admitted checkpoints and the original logical
    /// changes. The adapter verifies the current successor, reverses those
    /// changes in memory, and requires the exact predecessor roots before
    /// returning the original forward batch. Initial checkpoints are excluded.
    /// Record admission, exact custody read-back, and independent anchoring
    /// remain separate requirements. This method performs no durable write.
    ///
    /// # Errors
    /// Rejects unsupported recovery, mismatched or corrupt evidence, changed
    /// heads or transactions, and exhausted preparation limits.
    fn recover_committed_accumulator(
        &mut self,
        _request: &crate::OperationAccumulatorPreparation,
        _successor: &crate::OperationAccumulatorCheckpoint,
    ) -> Result<crate::PreparedOperationAccumulator, crate::OperationAccumulatorPreparationError>
    {
        Err(ControlStoreError::new(crate::ControlStoreErrorKind::RecoveryRequired).into())
    }

    /// Retain one exact operation-lease record before a handle can rely on it.
    ///
    /// The record remains opaque at this boundary. A reused immutable
    /// reference with different digest, schema, or bytes must fail with
    /// [`crate::ControlStoreErrorKind::RecoveryRequired`]. An unindexed row
    /// left by a later start failure grants no operation authority.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when physical custody cannot prove an
    /// exact retained record or exact replay.
    fn compare_and_retain_operation_lease(
        &mut self,
        store: &AuthorityStoreCoordinate,
        record: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError>;

    /// Read one exact record needed to recover an operation, without allocating
    /// its body above `maximum_body_bytes`.
    ///
    /// Absence is not permission to recreate historical authority. A reused
    /// reference, malformed metadata, or oversized body must fail with
    /// [`crate::ControlStoreErrorKind::RecoveryRequired`].
    /// The caller must admit the returned schema and body through their owner;
    /// a physical read cannot establish lease, result or diagnostic meaning.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when bounded physical observation fails
    /// or retained custody is contradictory.
    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError>;

    /// Compare the exact predecessor and atomically commit the candidate.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when physical comparison or atomic commit
    /// fails. An outcome-unknown failure requires [`Self::observe_exact`].
    fn compare_and_commit(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError>;

    /// Read back the complete candidate without mutation. Unknown commit
    /// outcomes must use this method and must never be retried blindly.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when exact read-back cannot complete.
    fn observe_exact(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError>;

    /// Read one exact typed private index without scanning or mutation.
    ///
    /// Reject a present record before copying its body into Rust-owned memory
    /// if it exceeds `maximum_record_bytes`. Metadata must also be bounded
    /// before allocation. An absent index remains absent even with a zero limit.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when the selected physical index cannot
    /// be read reliably.
    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError>;
}

fn query_accepts_schema(query: &OperationCustodyIndexQuery, schema: &str) -> bool {
    match query {
        OperationCustodyIndexQuery::NativeOutcome(_) => {
            schema == PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA
        }
        OperationCustodyIndexQuery::A2aSendPreparation(_) => {
            schema == PRIVATE_OPERATION_A2A_SEND_PREPARATION_V1_SCHEMA
        }
        OperationCustodyIndexQuery::A2aSendPreparationCheckpoint(_) => {
            schema == CONTROL_STATE_CHECKPOINT_V1_SCHEMA
        }
        OperationCustodyIndexQuery::CurrentA2aGetTask(_) => {
            matches!(
                schema,
                PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA
                    | PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA
                    | pale_core::records::A2A_TASK_OBSERVATION_V1_SCHEMA
            )
        }
        OperationCustodyIndexQuery::A2aGetTaskCheckpoint(_) => {
            schema == CONTROL_STATE_CHECKPOINT_V1_SCHEMA
        }
        OperationCustodyIndexQuery::A2aTaskBinding(_) => {
            schema == pale_core::records::A2A_TASK_BINDING_V1_SCHEMA
        }
        OperationCustodyIndexQuery::FirstA2aTaskObservation(_)
        | OperationCustodyIndexQuery::LatestA2aTaskObservation(_) => {
            schema == pale_core::records::A2A_TASK_OBSERVATION_V1_SCHEMA
        }
        OperationCustodyIndexQuery::A2aTaskObservationCheckpoint(_) => {
            schema == CONTROL_STATE_CHECKPOINT_V1_SCHEMA
        }
        OperationCustodyIndexQuery::ObservationCheckpoint(_) => {
            schema == CONTROL_STATE_CHECKPOINT_V1_SCHEMA
        }
        OperationCustodyIndexQuery::RequestForHandle(_)
        | OperationCustodyIndexQuery::RequestEvidence(_) => {
            schema == COMMON_REQUEST_ENVELOPE_V1_SCHEMA
        }
        OperationCustodyIndexQuery::RequestOperation(_)
        | OperationCustodyIndexQuery::OperationHandle(_)
        | OperationCustodyIndexQuery::ParentChildMembership(_) => {
            schema == OPERATION_HANDLE_V1_SCHEMA
        }
        OperationCustodyIndexQuery::CollectionHead(_) => {
            schema == PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA
        }
        OperationCustodyIndexQuery::LatestObservation(_)
        | OperationCustodyIndexQuery::StartObservation(_) => {
            schema == OPERATION_OBSERVATION_V1_SCHEMA
        }
    }
}

fn validate_checkpoint_record(
    checkpoint: &ControlCheckpointHead,
    record: &ControlRecord,
) -> Result<(), ()> {
    (checkpoint.coordinate() == record.coordinate()
        && record.schema_id().as_str() == CONTROL_STATE_CHECKPOINT_V1_SCHEMA)
        .then_some(())
        .ok_or(())
}

#[cfg(test)]
#[path = "../tests/internal/operation.rs"]
mod tests;
