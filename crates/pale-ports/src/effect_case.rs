//! Atomic physical custody for admitted effect-case transitions.
//!
//! Application policy constructs and signs semantic records before this
//! boundary. This module proves only one of eight finite transaction shapes,
//! exact physical index mutation, and one hard-domain checkpoint successor. It
//! does not select identifiers, clocks, signers, routes, providers, policy, or
//! effects.

use std::collections::BTreeSet;

use pale_core::records::{
    ACTION_ENVELOPE_V1_SCHEMA, COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, EFFECT_ADMISSION_V1_SCHEMA,
    EFFECT_CONFLICT_CLAIM_V1_SCHEMA, EFFECT_DISPATCH_INTENT_V1_SCHEMA, EFFECT_RECEIPT_V1_SCHEMA,
    RECONCILIATION_CASE_V1_SCHEMA, RECONCILIATION_CUSTODY_V1_SCHEMA,
    RECONCILIATION_REQUEST_V1_SCHEMA, RECONCILIATION_RESULT_V1_SCHEMA,
};

use crate::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord, ControlRecordCoordinate,
    ControlRecordDigest, ControlRecordError, ControlStoreError,
};

/// One physical transaction shape in the initial-v1 effect-case lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectCaseTransitionKind {
    /// Admit a new logical action or bind another request to an existing one.
    AdmitAction(EffectCaseActionAdmissionKind),
    /// Persist the preallocated attempt and dispatch intent before transport.
    CommitDispatchIntent,
    /// Release the active claim after separately admitted pre-dispatch cancellation.
    CancelBeforeDispatch,
    /// Retain a terminal receipt and release its claim without reconciliation.
    SettleWithoutReconciliation,
    /// Retain an ambiguous receipt, open case/custody, and transfer its claim.
    OpenReconciliation,
    /// Append one result revision under an open or terminal custody subshape.
    AdvanceReconciliation(EffectCaseReconciliationAdvanceKind),
    /// Advance the exclusive custody head without rewriting case history.
    TransferCustody,
    /// Append a predecessor-bound case and new custody successor.
    ReopenReconciliation,
}

/// Closed idempotency branch for an action-admission transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectCaseActionAdmissionKind {
    /// Persist a new logical action, admission, claim, and current disposition.
    New,
    /// Bind a new request identity to one exact existing logical action.
    AliasExisting,
}

/// Closed custody consequence of one reconciliation-result revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectCaseReconciliationAdvanceKind {
    /// The result advances its case while existing custody remains active.
    Open,
    /// The terminal result and closed-custody successor commit together.
    Terminal,
}

/// One immutable record role admitted by an effect-case transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EffectCaseTransitionMemberKind {
    /// The exact caller-signed request envelope.
    CommonRequest,
    /// The admitted external-action body carried by the request.
    Action,
    /// A typed and scoped conflict claim or its successor.
    ConflictClaim,
    /// The independently signed effect-admission decision.
    Admission,
    /// The durable intent that precedes any native transport call.
    DispatchIntent,
    /// The immutable observed effect result.
    Receipt,
    /// An ambiguity case or predecessor-bound reopening.
    ReconciliationCase,
    /// The exact caller-signed read-only reconciliation request.
    ReconciliationRequest,
    /// Custody that fences one unresolved conflict resource.
    ReconciliationCustody,
    /// One immutable reconciliation observation.
    ReconciliationResult,
}

impl EffectCaseTransitionMemberKind {
    const fn schema_id(self) -> &'static str {
        match self {
            Self::CommonRequest => COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
            Self::Action => ACTION_ENVELOPE_V1_SCHEMA,
            Self::ConflictClaim => EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
            Self::Admission => EFFECT_ADMISSION_V1_SCHEMA,
            Self::DispatchIntent => EFFECT_DISPATCH_INTENT_V1_SCHEMA,
            Self::Receipt => EFFECT_RECEIPT_V1_SCHEMA,
            Self::ReconciliationCase => RECONCILIATION_CASE_V1_SCHEMA,
            Self::ReconciliationRequest => RECONCILIATION_REQUEST_V1_SCHEMA,
            Self::ReconciliationCustody => RECONCILIATION_CUSTODY_V1_SCHEMA,
            Self::ReconciliationResult => RECONCILIATION_RESULT_V1_SCHEMA,
        }
    }
}

/// One already-admitted immutable record in an effect-case transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectCaseTransitionMember {
    /// The exact caller-signed request envelope.
    CommonRequest(ControlRecord),
    /// The admitted external-action body carried by the request.
    Action(ControlRecord),
    /// A typed and scoped conflict claim or its successor.
    ConflictClaim(ControlRecord),
    /// The independently signed effect-admission decision.
    Admission(ControlRecord),
    /// The durable intent that precedes any native transport call.
    DispatchIntent(ControlRecord),
    /// The immutable observed effect result.
    Receipt(ControlRecord),
    /// An ambiguity case or predecessor-bound reopening.
    ReconciliationCase(ControlRecord),
    /// The exact caller-signed read-only reconciliation request.
    ReconciliationRequest(ControlRecord),
    /// Custody that fences one unresolved conflict resource.
    ReconciliationCustody(ControlRecord),
    /// One immutable reconciliation observation.
    ReconciliationResult(ControlRecord),
}

impl EffectCaseTransitionMember {
    /// Return the closed record role without inspecting bytes.
    #[must_use]
    pub const fn kind(&self) -> EffectCaseTransitionMemberKind {
        match self {
            Self::CommonRequest(_) => EffectCaseTransitionMemberKind::CommonRequest,
            Self::Action(_) => EffectCaseTransitionMemberKind::Action,
            Self::ConflictClaim(_) => EffectCaseTransitionMemberKind::ConflictClaim,
            Self::Admission(_) => EffectCaseTransitionMemberKind::Admission,
            Self::DispatchIntent(_) => EffectCaseTransitionMemberKind::DispatchIntent,
            Self::Receipt(_) => EffectCaseTransitionMemberKind::Receipt,
            Self::ReconciliationCase(_) => EffectCaseTransitionMemberKind::ReconciliationCase,
            Self::ReconciliationRequest(_) => EffectCaseTransitionMemberKind::ReconciliationRequest,
            Self::ReconciliationCustody(_) => EffectCaseTransitionMemberKind::ReconciliationCustody,
            Self::ReconciliationResult(_) => EffectCaseTransitionMemberKind::ReconciliationResult,
        }
    }

    /// Borrow the exact immutable record projected for custody.
    #[must_use]
    pub const fn record(&self) -> &ControlRecord {
        match self {
            Self::CommonRequest(record)
            | Self::Action(record)
            | Self::ConflictClaim(record)
            | Self::Admission(record)
            | Self::DispatchIntent(record)
            | Self::Receipt(record)
            | Self::ReconciliationCase(record)
            | Self::ReconciliationRequest(record)
            | Self::ReconciliationCustody(record)
            | Self::ReconciliationResult(record) => record,
        }
    }
}

/// Canonical digest of one application-owned physical index key.
///
/// The containing [`EffectCaseIndexMutation`] or [`EffectCaseIndexAssertion`]
/// variant preserves the key's semantic domain. This value is not a
/// record-content digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EffectCaseIndexKey(ControlRecordDigest);

impl EffectCaseIndexKey {
    /// Admit one exact lowercase SHA-256 index-key digest.
    ///
    /// # Errors
    ///
    /// Returns [`ControlRecordError::InvalidDigest`] for a non-canonical value.
    pub fn parse(value: impl Into<String>) -> Result<Self, ControlRecordError> {
        ControlRecordDigest::parse(value).map(Self)
    }

    /// Borrow the canonical lowercase hexadecimal spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Exact predecessor state expected at one physical index key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectCaseIndexPredecessor {
    /// The index key must not yet exist.
    Absent,
    /// The index key must point at this exact immutable record.
    Exact(ControlRecord),
}

/// Closed map/head mutation family for one effect-case transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectCaseIndexMutation {
    /// Bind caller request identity to its immutable request record.
    RequestIdentity {
        /// Digest of the canonical request-identity key.
        key: EffectCaseIndexKey,
        /// Exact predecessor expected for compare-and-set.
        expected: EffectCaseIndexPredecessor,
        /// Request record that becomes the index head.
        successor: ControlRecordCoordinate,
    },
    /// Bind scoped idempotency to the admitted logical-action disposition.
    LogicalAction {
        /// Digest of the complete scoped logical-action key.
        key: EffectCaseIndexKey,
        /// Exact predecessor expected for compare-and-set.
        expected: EffectCaseIndexPredecessor,
        /// Effect-admission record that becomes the disposition head.
        successor: ControlRecordCoordinate,
    },
    /// Advance the current durable disposition of one logical action.
    ActionDisposition {
        /// Digest of the same canonical logical-action key used by the immutable map.
        key: EffectCaseIndexKey,
        /// Exact predecessor disposition expected for compare-and-set.
        expected: EffectCaseIndexPredecessor,
        /// Exact current-disposition record that becomes the mutable head.
        successor: ControlRecordCoordinate,
    },
    /// Advance the active claim head for one typed/scoped resource.
    ActiveClaim {
        /// Digest of the canonical conflict-resource key.
        key: EffectCaseIndexKey,
        /// Exact predecessor claim expected for compare-and-set.
        expected: EffectCaseIndexPredecessor,
        /// Conflict-claim record that becomes the resource head.
        successor: ControlRecordCoordinate,
    },
    /// Advance exclusive reconciliation custody for one resource.
    ActiveCustody {
        /// Digest of the canonical custody resource key.
        key: EffectCaseIndexKey,
        /// Exact predecessor custody expected for compare-and-set.
        expected: EffectCaseIndexPredecessor,
        /// Custody record that becomes the resource head.
        successor: ControlRecordCoordinate,
    },
    /// Bind one logical action's sole initial-v1 dispatch slot to its intent.
    ActionIntent {
        /// Digest of the canonical logical-action attempt slot.
        key: EffectCaseIndexKey,
        /// Exact predecessor expected for compare-and-set.
        expected: EffectCaseIndexPredecessor,
        /// Dispatch-intent record that fills the action's dispatch slot.
        successor: ControlRecordCoordinate,
    },
    /// Advance a case head to an opening/reopening or result revision.
    CaseResult {
        /// Digest of the canonical reconciliation-case key.
        key: EffectCaseIndexKey,
        /// Exact predecessor case/result expected for compare-and-set.
        expected: EffectCaseIndexPredecessor,
        /// Case or result record that becomes the case head.
        successor: ControlRecordCoordinate,
    },
}

/// Closed physical index read set co-evaluated with one transition.
///
/// Assertions never advance their index. They close races against another
/// transaction that dispatches, settles, or acquires custody concurrently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectCaseIndexAssertion {
    /// Require one exact immutable logical-action admission.
    LogicalAction {
        /// Digest of the complete scoped logical-action key.
        key: EffectCaseIndexKey,
        /// Exact admission record required by an alias transaction.
        expected: EffectCaseIndexPredecessor,
    },
    /// Require one exact current logical-action disposition.
    ActionDisposition {
        /// Digest of the same canonical logical-action key used by the immutable map.
        key: EffectCaseIndexKey,
        /// Exact current disposition returned by an alias transaction.
        expected: EffectCaseIndexPredecessor,
    },
    /// Require one exact active-claim observation.
    ActiveClaim {
        /// Digest of the canonical conflict-resource key.
        key: EffectCaseIndexKey,
        /// Exact absent or immutable-record state required by the transition.
        expected: EffectCaseIndexPredecessor,
    },
    /// Require one exact predecessor case/result observation.
    CaseResult {
        /// Digest of the canonical predecessor reconciliation-case key.
        key: EffectCaseIndexKey,
        /// Exact terminal result required before reopening another case.
        expected: EffectCaseIndexPredecessor,
    },
    /// Require one exact active-custody observation.
    ActiveCustody {
        /// Digest of the canonical custody resource key.
        key: EffectCaseIndexKey,
        /// Exact absent or immutable-record state required by the transition.
        expected: EffectCaseIndexPredecessor,
    },
    /// Require one exact action-intent observation.
    ActionIntent {
        /// Digest of the canonical logical-action attempt slot.
        key: EffectCaseIndexKey,
        /// Exact absent or immutable-record state required by the transition.
        expected: EffectCaseIndexPredecessor,
    },
}

impl EffectCaseIndexAssertion {
    fn role(&self) -> EffectCaseAssertionRole {
        match self {
            Self::LogicalAction { .. } => EffectCaseAssertionRole::LogicalAction,
            Self::ActionDisposition { .. } => EffectCaseAssertionRole::ActionDisposition,
            Self::ActiveClaim { .. } => EffectCaseAssertionRole::ActiveClaim,
            Self::ActiveCustody { .. } => EffectCaseAssertionRole::ActiveCustody,
            Self::ActionIntent { .. } => EffectCaseAssertionRole::ActionIntent,
            Self::CaseResult { .. } => EffectCaseAssertionRole::CaseResult,
        }
    }

    fn key(&self) -> &EffectCaseIndexKey {
        match self {
            Self::LogicalAction { key, .. }
            | Self::ActionDisposition { key, .. }
            | Self::ActiveClaim { key, .. }
            | Self::ActiveCustody { key, .. }
            | Self::ActionIntent { key, .. }
            | Self::CaseResult { key, .. } => key,
        }
    }

    fn record_predecessor(&self) -> Option<&EffectCaseIndexPredecessor> {
        match self {
            Self::LogicalAction { expected, .. }
            | Self::ActionDisposition { expected, .. }
            | Self::ActiveClaim { expected, .. }
            | Self::ActiveCustody { expected, .. }
            | Self::ActionIntent { expected, .. }
            | Self::CaseResult { expected, .. } => Some(expected),
        }
    }
}

impl EffectCaseIndexMutation {
    fn role(&self) -> EffectCaseIndexRole {
        match self {
            Self::RequestIdentity { .. } => EffectCaseIndexRole::RequestIdentity,
            Self::LogicalAction { .. } => EffectCaseIndexRole::LogicalAction,
            Self::ActionDisposition { .. } => EffectCaseIndexRole::ActionDisposition,
            Self::ActiveClaim { .. } => EffectCaseIndexRole::ActiveClaim,
            Self::ActiveCustody { .. } => EffectCaseIndexRole::ActiveCustody,
            Self::ActionIntent { .. } => EffectCaseIndexRole::ActionIntent,
            Self::CaseResult { .. } => EffectCaseIndexRole::CaseResult,
        }
    }

    fn key(&self) -> &EffectCaseIndexKey {
        match self {
            Self::RequestIdentity { key, .. }
            | Self::LogicalAction { key, .. }
            | Self::ActionDisposition { key, .. }
            | Self::ActiveClaim { key, .. }
            | Self::ActiveCustody { key, .. }
            | Self::ActionIntent { key, .. }
            | Self::CaseResult { key, .. } => key,
        }
    }

    fn record_predecessor(&self) -> Option<&EffectCaseIndexPredecessor> {
        match self {
            Self::RequestIdentity { expected, .. }
            | Self::LogicalAction { expected, .. }
            | Self::ActionDisposition { expected, .. }
            | Self::ActiveClaim { expected, .. }
            | Self::ActiveCustody { expected, .. }
            | Self::ActionIntent { expected, .. }
            | Self::CaseResult { expected, .. } => Some(expected),
        }
    }

    fn record_successor(&self) -> Option<&ControlRecordCoordinate> {
        match self {
            Self::RequestIdentity { successor, .. }
            | Self::LogicalAction { successor, .. }
            | Self::ActionDisposition { successor, .. }
            | Self::ActiveClaim { successor, .. }
            | Self::ActiveCustody { successor, .. }
            | Self::ActionIntent { successor, .. }
            | Self::CaseResult { successor, .. } => Some(successor),
        }
    }

    fn admits_member(&self, member: &EffectCaseTransitionMember) -> bool {
        self.record_successor() == Some(member.record().coordinate())
            && match self {
                Self::RequestIdentity { .. } => {
                    member.kind() == EffectCaseTransitionMemberKind::CommonRequest
                }
                Self::LogicalAction { .. } => {
                    member.kind() == EffectCaseTransitionMemberKind::Admission
                }
                Self::ActionDisposition { .. } => matches!(
                    member.kind(),
                    EffectCaseTransitionMemberKind::Admission
                        | EffectCaseTransitionMemberKind::ConflictClaim
                        | EffectCaseTransitionMemberKind::DispatchIntent
                        | EffectCaseTransitionMemberKind::Receipt
                        | EffectCaseTransitionMemberKind::ReconciliationCase
                        | EffectCaseTransitionMemberKind::ReconciliationResult
                ),
                Self::ActiveClaim { .. } => {
                    member.kind() == EffectCaseTransitionMemberKind::ConflictClaim
                }
                Self::ActiveCustody { .. } => {
                    member.kind() == EffectCaseTransitionMemberKind::ReconciliationCustody
                }
                Self::ActionIntent { .. } => {
                    member.kind() == EffectCaseTransitionMemberKind::DispatchIntent
                }
                Self::CaseResult { .. } => matches!(
                    member.kind(),
                    EffectCaseTransitionMemberKind::ReconciliationCase
                        | EffectCaseTransitionMemberKind::ReconciliationResult
                ),
            }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EffectCaseIndexRole {
    RequestIdentity,
    LogicalAction,
    ActionDisposition,
    ActiveClaim,
    ActiveCustody,
    ActionIntent,
    CaseResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EffectCaseAssertionRole {
    LogicalAction,
    ActionDisposition,
    ActiveClaim,
    ActiveCustody,
    ActionIntent,
    CaseResult,
}

/// Closed point-query family for restart, replay, and reconciliation.
///
/// Each variant selects one typed private index. It permits neither scans nor
/// caller-defined index names; logical key derivation remains application-owned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectCaseIndexQuery {
    /// Resolve a caller request identity to its immutable request record.
    RequestIdentity(EffectCaseIndexKey),
    /// Resolve a logical-action key to its immutable admission record.
    LogicalAction(EffectCaseIndexKey),
    /// Resolve a logical-action key to its current disposition record.
    ActionDisposition(EffectCaseIndexKey),
    /// Resolve a conflict-resource key to its current claim record.
    ActiveClaim(EffectCaseIndexKey),
    /// Resolve a conflict-resource key to its current custody record.
    ActiveCustody(EffectCaseIndexKey),
    /// Resolve a logical-action dispatch slot to its immutable intent.
    ActionIntent(EffectCaseIndexKey),
    /// Resolve a reconciliation-case key to its current case or result.
    CaseResult(EffectCaseIndexKey),
}

impl EffectCaseIndexQuery {
    fn role(&self) -> EffectCaseIndexRole {
        match self {
            Self::RequestIdentity(_) => EffectCaseIndexRole::RequestIdentity,
            Self::LogicalAction(_) => EffectCaseIndexRole::LogicalAction,
            Self::ActionDisposition(_) => EffectCaseIndexRole::ActionDisposition,
            Self::ActiveClaim(_) => EffectCaseIndexRole::ActiveClaim,
            Self::ActiveCustody(_) => EffectCaseIndexRole::ActiveCustody,
            Self::ActionIntent(_) => EffectCaseIndexRole::ActionIntent,
            Self::CaseResult(_) => EffectCaseIndexRole::CaseResult,
        }
    }

    /// Borrow the exact application-derived index key.
    #[must_use]
    pub const fn key(&self) -> &EffectCaseIndexKey {
        match self {
            Self::RequestIdentity(key)
            | Self::LogicalAction(key)
            | Self::ActionDisposition(key)
            | Self::ActiveClaim(key)
            | Self::ActiveCustody(key)
            | Self::ActionIntent(key)
            | Self::CaseResult(key) => key,
        }
    }
}

/// Exact point-read result for one typed private effect-case index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectCaseIndexObservation {
    query: EffectCaseIndexQuery,
    record: Option<ControlRecord>,
}

impl EffectCaseIndexObservation {
    /// Construct an absent observation for one exact typed index key.
    #[must_use]
    pub const fn absent(query: EffectCaseIndexQuery) -> Self {
        Self {
            query,
            record: None,
        }
    }

    /// Bind an exact stored record to the requested typed index.
    ///
    /// # Errors
    ///
    /// Returns [`EffectCaseIndexObservationError::RecordSchema`] when the
    /// stored record cannot inhabit the requested index domain.
    pub fn exact_record(
        query: EffectCaseIndexQuery,
        record: ControlRecord,
    ) -> Result<Self, EffectCaseIndexObservationError> {
        query_accepts_schema(&query, record.schema_id().as_str())
            .then_some(Self {
                query,
                record: Some(record),
            })
            .ok_or(EffectCaseIndexObservationError::RecordSchema)
    }

    /// Borrow the exact role and key that were observed.
    #[must_use]
    pub const fn query(&self) -> &EffectCaseIndexQuery {
        &self.query
    }

    /// Borrow the exact stored record, or `None` when the key was absent.
    #[must_use]
    pub const fn record(&self) -> Option<&ControlRecord> {
        self.record.as_ref()
    }
}

/// A typed index observation could not represent the stored row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectCaseIndexObservationError {
    /// The stored signed-record schema is outside the selected index domain.
    #[error("effect-case index observation uses the wrong record schema")]
    RecordSchema,
    /// The stored immutable record does not equal the requested coordinate.
    #[error("effect-case record observation uses the wrong record coordinate")]
    RecordCoordinate,
}

/// Exact point-read result for one immutable control-record coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectCaseRecordObservation {
    coordinate: ControlRecordCoordinate,
    record: Option<ControlRecord>,
}

impl EffectCaseRecordObservation {
    /// Construct an absent observation bound to the requested coordinate.
    #[must_use]
    pub const fn absent(coordinate: ControlRecordCoordinate) -> Self {
        Self {
            coordinate,
            record: None,
        }
    }

    /// Bind an exact stored record to the requested coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectCaseIndexObservationError::RecordCoordinate`] when the
    /// stored record has a different immutable coordinate.
    pub fn exact(
        coordinate: ControlRecordCoordinate,
        record: ControlRecord,
    ) -> Result<Self, EffectCaseIndexObservationError> {
        (record.coordinate() == &coordinate)
            .then_some(Self {
                coordinate,
                record: Some(record),
            })
            .ok_or(EffectCaseIndexObservationError::RecordCoordinate)
    }

    /// Borrow the exact immutable coordinate that was observed.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Borrow the exact stored record, or `None` when it was absent.
    #[must_use]
    pub const fn record(&self) -> Option<&ControlRecord> {
        self.record.as_ref()
    }
}

/// Closed hard-domain checkpoint consequence of one effect-case transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectCaseCheckpointMutation {
    /// Alias an existing logical action without advancing its effect domain.
    Unchanged,
    /// Advance the affected hard domain exactly once with the effect mutation.
    Advance(Box<EffectCaseCheckpointTransition>),
}

impl EffectCaseCheckpointMutation {
    /// Wrap one proven exact checkpoint successor for atomic commit.
    #[must_use]
    pub fn advance(transition: EffectCaseCheckpointTransition) -> Self {
        Self::Advance(Box::new(transition))
    }
}

/// Closed physical transition awaiting one atomic control-store comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectCaseTransitionCandidate {
    store: AuthorityStoreCoordinate,
    kind: EffectCaseTransitionKind,
    members: Box<[EffectCaseTransitionMember]>,
    index_mutations: Box<[EffectCaseIndexMutation]>,
    index_assertions: Box<[EffectCaseIndexAssertion]>,
    checkpoint: EffectCaseCheckpointMutation,
}

impl EffectCaseTransitionCandidate {
    /// Prove one exact finite transaction shape and checkpoint successor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectCaseCandidateError`] for an incomplete or wrong shape,
    /// duplicate roles/coordinates/indexes, schema disagreement, an index head
    /// not closed by a candidate member, or a non-successor checkpoint pair.
    pub fn new(
        store: AuthorityStoreCoordinate,
        kind: EffectCaseTransitionKind,
        members: Vec<EffectCaseTransitionMember>,
        index_mutations: Vec<EffectCaseIndexMutation>,
        index_assertions: Vec<EffectCaseIndexAssertion>,
        checkpoint: EffectCaseCheckpointMutation,
    ) -> Result<Self, EffectCaseCandidateError> {
        validate_members(kind, &members)?;
        validate_index_mutations(kind, &members, &index_mutations)?;
        validate_index_assertions(kind, &index_mutations, &index_assertions)?;
        validate_action_disposition(kind, &members, &index_mutations, &index_assertions)?;
        let checkpoint_shape_is_exact = matches!(
            (kind, &checkpoint),
            (
                EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting),
                EffectCaseCheckpointMutation::Unchanged
            ) | (
                EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New)
                    | EffectCaseTransitionKind::CommitDispatchIntent
                    | EffectCaseTransitionKind::CancelBeforeDispatch
                    | EffectCaseTransitionKind::SettleWithoutReconciliation
                    | EffectCaseTransitionKind::OpenReconciliation
                    | EffectCaseTransitionKind::AdvanceReconciliation(_)
                    | EffectCaseTransitionKind::TransferCustody
                    | EffectCaseTransitionKind::ReopenReconciliation,
                EffectCaseCheckpointMutation::Advance(_)
            )
        );
        if !checkpoint_shape_is_exact {
            return Err(EffectCaseCandidateError::CheckpointShape);
        }

        let checkpoint_coordinates = match &checkpoint {
            EffectCaseCheckpointMutation::Unchanged => Vec::new(),
            EffectCaseCheckpointMutation::Advance(transition) => vec![
                transition.expected_record().coordinate(),
                transition.successor_record().coordinate(),
            ],
        };
        let expected_coordinate_count = members.len() + checkpoint_coordinates.len();
        let coordinates = members
            .iter()
            .map(|member| member.record().coordinate())
            .chain(checkpoint_coordinates)
            .collect::<BTreeSet<_>>();
        if coordinates.len() != expected_coordinate_count {
            return Err(EffectCaseCandidateError::DuplicateRecordCoordinate);
        }

        Ok(Self {
            store,
            kind,
            members: members.into_boxed_slice(),
            index_mutations: index_mutations.into_boxed_slice(),
            index_assertions: index_assertions.into_boxed_slice(),
            checkpoint,
        })
    }

    /// Borrow the exact store incarnation containing the selected domain.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the exact finite transaction shape.
    #[must_use]
    pub const fn kind(&self) -> EffectCaseTransitionKind {
        self.kind
    }

    /// Borrow the closed checkpoint consequence of this transition.
    #[must_use]
    pub const fn checkpoint_mutation(&self) -> &EffectCaseCheckpointMutation {
        &self.checkpoint
    }

    /// Borrow the predecessor checkpoint head when this transition advances it.
    #[must_use]
    pub const fn expected_checkpoint(&self) -> Option<&ControlCheckpointHead> {
        match &self.checkpoint {
            EffectCaseCheckpointMutation::Unchanged => None,
            EffectCaseCheckpointMutation::Advance(transition) => Some(transition.expected()),
        }
    }

    /// Borrow all immutable record appends in this closed shape.
    #[must_use]
    pub const fn members(&self) -> &[EffectCaseTransitionMember] {
        &self.members
    }

    /// Borrow every typed map/head compare-and-set mutation.
    #[must_use]
    pub const fn index_mutations(&self) -> &[EffectCaseIndexMutation] {
        &self.index_mutations
    }

    /// Borrow the exact physical index read set co-evaluated before commit.
    #[must_use]
    pub const fn index_assertions(&self) -> &[EffectCaseIndexAssertion] {
        &self.index_assertions
    }

    /// Borrow the immutable predecessor checkpoint record when advanced.
    #[must_use]
    pub const fn expected_checkpoint_record(&self) -> Option<&ControlRecord> {
        match &self.checkpoint {
            EffectCaseCheckpointMutation::Unchanged => None,
            EffectCaseCheckpointMutation::Advance(transition) => Some(transition.expected_record()),
        }
    }

    /// Borrow the successor checkpoint head when this transition advances it.
    #[must_use]
    pub const fn successor_checkpoint(&self) -> Option<&ControlCheckpointHead> {
        match &self.checkpoint {
            EffectCaseCheckpointMutation::Unchanged => None,
            EffectCaseCheckpointMutation::Advance(transition) => Some(transition.successor()),
        }
    }

    /// Borrow the immutable successor checkpoint record, when advanced.
    #[must_use]
    pub const fn successor_checkpoint_record(&self) -> Option<&ControlRecord> {
        match &self.checkpoint {
            EffectCaseCheckpointMutation::Unchanged => None,
            EffectCaseCheckpointMutation::Advance(transition) => {
                Some(transition.successor_record())
            }
        }
    }
}

/// Exact predecessor and successor checkpoint pair for one transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectCaseCheckpointTransition {
    expected: ControlCheckpointHead,
    expected_record: ControlRecord,
    successor: ControlCheckpointHead,
    successor_record: ControlRecord,
}

impl EffectCaseCheckpointTransition {
    /// Prove record/head equality and one exact hard-domain successor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectCaseCandidateError`] when either record disagrees with
    /// its head or the successor changes domain/epoch or skips a sequence.
    pub fn new(
        expected: ControlCheckpointHead,
        expected_record: ControlRecord,
        successor: ControlCheckpointHead,
        successor_record: ControlRecord,
    ) -> Result<Self, EffectCaseCandidateError> {
        validate_checkpoint_record(&expected, &expected_record)
            .map_err(|()| EffectCaseCandidateError::ExpectedCheckpointRecord)?;
        validate_checkpoint_record(&successor, &successor_record)
            .map_err(|()| EffectCaseCandidateError::SuccessorCheckpointRecord)?;
        if !successor.is_exact_successor_of(&expected) {
            return Err(EffectCaseCandidateError::CheckpointSuccession);
        }
        Ok(Self {
            expected,
            expected_record,
            successor,
            successor_record,
        })
    }

    /// Borrow the exact predecessor checkpoint head.
    #[must_use]
    pub const fn expected(&self) -> &ControlCheckpointHead {
        &self.expected
    }

    /// Borrow the immutable predecessor checkpoint record.
    #[must_use]
    pub const fn expected_record(&self) -> &ControlRecord {
        &self.expected_record
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

fn validate_members(
    kind: EffectCaseTransitionKind,
    members: &[EffectCaseTransitionMember],
) -> Result<(), EffectCaseCandidateError> {
    let observed = members
        .iter()
        .map(EffectCaseTransitionMember::kind)
        .collect::<BTreeSet<_>>();
    if observed.len() != members.len() || observed != required_members(kind) {
        return Err(EffectCaseCandidateError::MemberShape);
    }
    if members
        .iter()
        .any(|member| member.record().schema_id().as_str() != member.kind().schema_id())
    {
        return Err(EffectCaseCandidateError::MemberSchema);
    }
    Ok(())
}

fn validate_index_mutations(
    kind: EffectCaseTransitionKind,
    members: &[EffectCaseTransitionMember],
    mutations: &[EffectCaseIndexMutation],
) -> Result<(), EffectCaseCandidateError> {
    let observed = mutations
        .iter()
        .map(EffectCaseIndexMutation::role)
        .collect::<BTreeSet<_>>();
    if observed.len() != mutations.len() || observed != required_index_roles(kind) {
        return Err(EffectCaseCandidateError::IndexShape);
    }
    let keys = mutations
        .iter()
        .map(|mutation| (mutation.role(), mutation.key()))
        .collect::<BTreeSet<_>>();
    if keys.len() != mutations.len() {
        return Err(EffectCaseCandidateError::DuplicateIndexMutation);
    }
    if mutations.iter().any(|mutation| {
        !predecessor_shape_is_exact(kind, mutation)
            || !mutation_predecessor_schema_is_exact(mutation)
    }) {
        return Err(EffectCaseCandidateError::IndexPredecessorShape);
    }
    if mutations.iter().any(|mutation| {
        mutation.record_predecessor().is_some_and(|expected| {
            matches!(
                (expected, mutation.record_successor()),
                (EffectCaseIndexPredecessor::Exact(expected), Some(successor))
                    if expected.coordinate() == successor
            )
        }) || mutation.record_successor().is_some()
            && !members.iter().any(|member| mutation.admits_member(member))
    }) {
        return Err(EffectCaseCandidateError::IndexSuccessorClosure);
    }
    Ok(())
}

fn validate_index_assertions(
    kind: EffectCaseTransitionKind,
    mutations: &[EffectCaseIndexMutation],
    assertions: &[EffectCaseIndexAssertion],
) -> Result<(), EffectCaseCandidateError> {
    let observed = assertions
        .iter()
        .map(EffectCaseIndexAssertion::role)
        .collect::<BTreeSet<_>>();
    if observed.len() != assertions.len() || observed != required_assertion_roles(kind) {
        return Err(EffectCaseCandidateError::AssertionShape);
    }
    if assertions.iter().any(|assertion| {
        !assertion_predecessor_shape_is_exact(kind, assertion)
            || !assertion_predecessor_schema_is_exact(assertion)
    }) {
        return Err(EffectCaseCandidateError::AssertionPredecessorShape);
    }

    let overlaps_mutation = |assertion: &EffectCaseIndexAssertion| {
        mutations.iter().any(|mutation| {
            matches!(
                (assertion.role(), mutation.role()),
                (
                    EffectCaseAssertionRole::ActiveClaim,
                    EffectCaseIndexRole::ActiveClaim
                ) | (
                    EffectCaseAssertionRole::LogicalAction,
                    EffectCaseIndexRole::LogicalAction
                ) | (
                    EffectCaseAssertionRole::ActionDisposition,
                    EffectCaseIndexRole::ActionDisposition
                ) | (
                    EffectCaseAssertionRole::ActiveCustody,
                    EffectCaseIndexRole::ActiveCustody
                ) | (
                    EffectCaseAssertionRole::ActionIntent,
                    EffectCaseIndexRole::ActionIntent
                ) | (
                    EffectCaseAssertionRole::CaseResult,
                    EffectCaseIndexRole::CaseResult
                )
            ) && assertion.key() == mutation.key()
        })
    };
    if assertions.iter().any(overlaps_mutation) {
        return Err(EffectCaseCandidateError::AssertionMutationOverlap);
    }
    let mutation_key = |role| {
        mutations
            .iter()
            .find(|mutation| mutation.role() == role)
            .map(EffectCaseIndexMutation::key)
    };
    let assertion_key = |role| {
        assertions
            .iter()
            .find(|assertion| assertion.role() == role)
            .map(EffectCaseIndexAssertion::key)
    };
    let one_resource = match kind {
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New) => {
            mutation_key(EffectCaseIndexRole::ActiveClaim)
                == assertion_key(EffectCaseAssertionRole::ActiveCustody)
        }
        EffectCaseTransitionKind::OpenReconciliation => {
            mutation_key(EffectCaseIndexRole::ActiveClaim)
                == mutation_key(EffectCaseIndexRole::ActiveCustody)
        }
        EffectCaseTransitionKind::ReopenReconciliation => {
            mutation_key(EffectCaseIndexRole::ActiveCustody)
                == assertion_key(EffectCaseAssertionRole::ActiveClaim)
        }
        _ => true,
    };
    if !one_resource {
        return Err(EffectCaseCandidateError::ConflictResourceKeyMismatch);
    }
    let one_logical_action = match kind {
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New) => {
            mutation_key(EffectCaseIndexRole::LogicalAction)
                == mutation_key(EffectCaseIndexRole::ActionDisposition)
        }
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting) => {
            assertion_key(EffectCaseAssertionRole::LogicalAction)
                == assertion_key(EffectCaseAssertionRole::ActionDisposition)
        }
        EffectCaseTransitionKind::CommitDispatchIntent => {
            mutation_key(EffectCaseIndexRole::ActionIntent)
                == mutation_key(EffectCaseIndexRole::ActionDisposition)
        }
        EffectCaseTransitionKind::CancelBeforeDispatch
        | EffectCaseTransitionKind::SettleWithoutReconciliation
        | EffectCaseTransitionKind::OpenReconciliation => {
            assertion_key(EffectCaseAssertionRole::ActionIntent)
                == mutation_key(EffectCaseIndexRole::ActionDisposition)
        }
        _ => true,
    };
    if !one_logical_action {
        return Err(EffectCaseCandidateError::LogicalActionKeyMismatch);
    }
    Ok(())
}

fn validate_action_disposition(
    kind: EffectCaseTransitionKind,
    members: &[EffectCaseTransitionMember],
    mutations: &[EffectCaseIndexMutation],
    assertions: &[EffectCaseIndexAssertion],
) -> Result<(), EffectCaseCandidateError> {
    let exact = action_disposition_mutation_is_exact(kind, members, mutations)
        && action_disposition_assertion_is_exact(kind, assertions)
        && action_disposition_predecessor_is_coherent(kind, mutations, assertions);
    exact
        .then_some(())
        .ok_or(EffectCaseCandidateError::ActionDispositionTransition)
}

fn action_disposition_predecessor_is_coherent(
    kind: EffectCaseTransitionKind,
    mutations: &[EffectCaseIndexMutation],
    assertions: &[EffectCaseIndexAssertion],
) -> bool {
    use EffectCaseAssertionRole as Assertion;
    use EffectCaseIndexRole as Index;
    use EffectCaseTransitionKind as Transition;

    let mutation_predecessor = |role| {
        mutations
            .iter()
            .find(|mutation| mutation.role() == role)
            .and_then(EffectCaseIndexMutation::record_predecessor)
    };
    let assertion_predecessor = |role| {
        assertions
            .iter()
            .find(|assertion| assertion.role() == role)
            .and_then(EffectCaseIndexAssertion::record_predecessor)
    };
    let equal_exact = |left: Option<&EffectCaseIndexPredecessor>,
                       right: Option<&EffectCaseIndexPredecessor>| {
        matches!(
            (left, right),
            (
                Some(EffectCaseIndexPredecessor::Exact(left)),
                Some(EffectCaseIndexPredecessor::Exact(right))
            ) if left == right
        )
    };

    match kind {
        Transition::SettleWithoutReconciliation | Transition::OpenReconciliation => equal_exact(
            mutation_predecessor(Index::ActionDisposition),
            assertion_predecessor(Assertion::ActionIntent),
        ),
        Transition::AdvanceReconciliation(_) => equal_exact(
            mutation_predecessor(Index::ActionDisposition),
            mutation_predecessor(Index::CaseResult),
        ),
        Transition::ReopenReconciliation => equal_exact(
            mutation_predecessor(Index::ActionDisposition),
            assertion_predecessor(Assertion::CaseResult),
        ),
        _ => true,
    }
}

fn action_disposition_mutation_is_exact(
    kind: EffectCaseTransitionKind,
    members: &[EffectCaseTransitionMember],
    mutations: &[EffectCaseIndexMutation],
) -> bool {
    use EffectCaseIndexPredecessor::{Absent, Exact};
    use EffectCaseTransitionKind as Transition;
    use EffectCaseTransitionMemberKind as Member;

    let disposition = mutations.iter().find_map(|mutation| match mutation {
        EffectCaseIndexMutation::ActionDisposition {
            expected,
            successor,
            ..
        } => Some((expected, successor)),
        _ => None,
    });
    let member = |role| member_coordinate(members, role);

    match (kind, disposition) {
        (
            Transition::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting)
            | Transition::TransferCustody,
            None,
        ) => true,
        (
            Transition::AdmitAction(EffectCaseActionAdmissionKind::New),
            Some((Absent, successor)),
        ) => member(Member::Admission) == Some(successor),
        (Transition::CommitDispatchIntent, Some((Exact(expected), successor))) => {
            expected.schema_id().as_str() == EFFECT_ADMISSION_V1_SCHEMA
                && member(Member::DispatchIntent) == Some(successor)
        }
        (Transition::CancelBeforeDispatch, Some((Exact(expected), successor))) => {
            expected.schema_id().as_str() == EFFECT_ADMISSION_V1_SCHEMA
                && member(Member::ConflictClaim) == Some(successor)
        }
        (Transition::SettleWithoutReconciliation, Some((Exact(expected), successor))) => {
            expected.schema_id().as_str() == EFFECT_DISPATCH_INTENT_V1_SCHEMA
                && member(Member::Receipt) == Some(successor)
        }
        (Transition::OpenReconciliation, Some((Exact(expected), successor))) => {
            expected.schema_id().as_str() == EFFECT_DISPATCH_INTENT_V1_SCHEMA
                && member(Member::ReconciliationCase) == Some(successor)
        }
        (Transition::AdvanceReconciliation(_), Some((Exact(expected), successor))) => {
            matches!(
                expected.schema_id().as_str(),
                RECONCILIATION_CASE_V1_SCHEMA | RECONCILIATION_RESULT_V1_SCHEMA
            ) && member(Member::ReconciliationResult) == Some(successor)
        }
        (Transition::ReopenReconciliation, Some((Exact(expected), successor))) => {
            expected.schema_id().as_str() == RECONCILIATION_RESULT_V1_SCHEMA
                && member(Member::ReconciliationCase) == Some(successor)
        }
        _ => false,
    }
}

fn action_disposition_assertion_is_exact(
    kind: EffectCaseTransitionKind,
    assertions: &[EffectCaseIndexAssertion],
) -> bool {
    let disposition = assertions.iter().find_map(|assertion| match assertion {
        EffectCaseIndexAssertion::ActionDisposition { expected, .. } => Some(expected),
        _ => None,
    });
    match (kind, disposition) {
        (
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting),
            Some(EffectCaseIndexPredecessor::Exact(record)),
        ) => action_disposition_schema_is_admitted(record.schema_id().as_str()),
        (
            EffectCaseTransitionKind::TransferCustody,
            Some(EffectCaseIndexPredecessor::Exact(record)),
        ) => matches!(
            record.schema_id().as_str(),
            RECONCILIATION_CASE_V1_SCHEMA | RECONCILIATION_RESULT_V1_SCHEMA
        ),
        (
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New)
            | EffectCaseTransitionKind::CommitDispatchIntent
            | EffectCaseTransitionKind::CancelBeforeDispatch
            | EffectCaseTransitionKind::SettleWithoutReconciliation
            | EffectCaseTransitionKind::OpenReconciliation
            | EffectCaseTransitionKind::AdvanceReconciliation(_)
            | EffectCaseTransitionKind::ReopenReconciliation,
            None,
        ) => true,
        _ => false,
    }
}

fn member_coordinate(
    members: &[EffectCaseTransitionMember],
    role: EffectCaseTransitionMemberKind,
) -> Option<&ControlRecordCoordinate> {
    members
        .iter()
        .find(|member| member.kind() == role)
        .map(|member| member.record().coordinate())
}

fn action_disposition_schema_is_admitted(schema: &str) -> bool {
    matches!(
        schema,
        EFFECT_ADMISSION_V1_SCHEMA
            | EFFECT_CONFLICT_CLAIM_V1_SCHEMA
            | EFFECT_DISPATCH_INTENT_V1_SCHEMA
            | EFFECT_RECEIPT_V1_SCHEMA
            | RECONCILIATION_CASE_V1_SCHEMA
            | RECONCILIATION_RESULT_V1_SCHEMA
    )
}

fn query_accepts_schema(query: &EffectCaseIndexQuery, schema: &str) -> bool {
    match query.role() {
        EffectCaseIndexRole::RequestIdentity => schema == COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
        EffectCaseIndexRole::LogicalAction => schema == EFFECT_ADMISSION_V1_SCHEMA,
        EffectCaseIndexRole::ActionDisposition => action_disposition_schema_is_admitted(schema),
        EffectCaseIndexRole::ActiveClaim => schema == EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
        EffectCaseIndexRole::ActiveCustody => schema == RECONCILIATION_CUSTODY_V1_SCHEMA,
        EffectCaseIndexRole::ActionIntent => schema == EFFECT_DISPATCH_INTENT_V1_SCHEMA,
        EffectCaseIndexRole::CaseResult => matches!(
            schema,
            RECONCILIATION_CASE_V1_SCHEMA | RECONCILIATION_RESULT_V1_SCHEMA
        ),
    }
}

fn mutation_predecessor_schema_is_exact(mutation: &EffectCaseIndexMutation) -> bool {
    mutation
        .record_predecessor()
        .is_none_or(|expected| predecessor_schema_is_exact(mutation.role(), expected))
}

fn assertion_predecessor_schema_is_exact(assertion: &EffectCaseIndexAssertion) -> bool {
    let Some(expected) = assertion.record_predecessor() else {
        return true;
    };
    match assertion.role() {
        EffectCaseAssertionRole::LogicalAction => {
            expected_schema_is(expected, |schema| schema == EFFECT_ADMISSION_V1_SCHEMA)
        }
        EffectCaseAssertionRole::ActionDisposition => true,
        EffectCaseAssertionRole::ActiveClaim => {
            expected_schema_is(expected, |schema| schema == EFFECT_CONFLICT_CLAIM_V1_SCHEMA)
        }
        EffectCaseAssertionRole::ActiveCustody => expected_schema_is(expected, |schema| {
            schema == RECONCILIATION_CUSTODY_V1_SCHEMA
        }),
        EffectCaseAssertionRole::ActionIntent => expected_schema_is(expected, |schema| {
            schema == EFFECT_DISPATCH_INTENT_V1_SCHEMA
        }),
        EffectCaseAssertionRole::CaseResult => {
            expected_schema_is(expected, |schema| schema == RECONCILIATION_RESULT_V1_SCHEMA)
        }
    }
}

fn predecessor_schema_is_exact(
    role: EffectCaseIndexRole,
    expected: &EffectCaseIndexPredecessor,
) -> bool {
    match role {
        EffectCaseIndexRole::RequestIdentity => expected_schema_is(expected, |schema| {
            schema == COMMON_REQUEST_ENVELOPE_V1_SCHEMA
        }),
        EffectCaseIndexRole::LogicalAction => {
            expected_schema_is(expected, |schema| schema == EFFECT_ADMISSION_V1_SCHEMA)
        }
        EffectCaseIndexRole::ActionDisposition => true,
        EffectCaseIndexRole::ActiveClaim => {
            expected_schema_is(expected, |schema| schema == EFFECT_CONFLICT_CLAIM_V1_SCHEMA)
        }
        EffectCaseIndexRole::ActiveCustody => expected_schema_is(expected, |schema| {
            schema == RECONCILIATION_CUSTODY_V1_SCHEMA
        }),
        EffectCaseIndexRole::ActionIntent => expected_schema_is(expected, |schema| {
            schema == EFFECT_DISPATCH_INTENT_V1_SCHEMA
        }),
        EffectCaseIndexRole::CaseResult => expected_schema_is(expected, |schema| {
            matches!(
                schema,
                RECONCILIATION_CASE_V1_SCHEMA | RECONCILIATION_RESULT_V1_SCHEMA
            )
        }),
    }
}

fn expected_schema_is(
    expected: &EffectCaseIndexPredecessor,
    predicate: impl FnOnce(&str) -> bool,
) -> bool {
    match expected {
        EffectCaseIndexPredecessor::Absent => true,
        EffectCaseIndexPredecessor::Exact(record) => predicate(record.schema_id().as_str()),
    }
}

fn predecessor_shape_is_exact(
    kind: EffectCaseTransitionKind,
    mutation: &EffectCaseIndexMutation,
) -> bool {
    use EffectCaseIndexPredecessor::{Absent, Exact};
    use EffectCaseTransitionKind as Transition;

    matches!(
        (kind, mutation),
        (
            Transition::AdmitAction(_),
            EffectCaseIndexMutation::RequestIdentity {
                expected: Absent,
                ..
            }
        ) | (
            Transition::AdmitAction(EffectCaseActionAdmissionKind::New),
            EffectCaseIndexMutation::LogicalAction {
                expected: Absent,
                ..
            }
        ) | (
            Transition::AdmitAction(EffectCaseActionAdmissionKind::New),
            EffectCaseIndexMutation::ActionDisposition {
                expected: Absent,
                ..
            }
        ) | (
            Transition::AdmitAction(EffectCaseActionAdmissionKind::New),
            EffectCaseIndexMutation::ActiveClaim {
                expected: Absent | Exact(_),
                ..
            }
        ) | (
            Transition::CommitDispatchIntent,
            EffectCaseIndexMutation::ActionIntent {
                expected: Absent,
                ..
            }
        ) | (
            Transition::CommitDispatchIntent
                | Transition::CancelBeforeDispatch
                | Transition::SettleWithoutReconciliation
                | Transition::OpenReconciliation
                | Transition::AdvanceReconciliation(_)
                | Transition::TransferCustody
                | Transition::ReopenReconciliation,
            EffectCaseIndexMutation::ActionDisposition {
                expected: Exact(_),
                ..
            }
        ) | (
            Transition::CancelBeforeDispatch | Transition::SettleWithoutReconciliation,
            EffectCaseIndexMutation::ActiveClaim {
                expected: Exact(_),
                ..
            }
        ) | (
            Transition::OpenReconciliation,
            EffectCaseIndexMutation::ActiveClaim {
                expected: Exact(_),
                ..
            }
        ) | (
            Transition::OpenReconciliation,
            EffectCaseIndexMutation::ActiveCustody {
                expected: Absent | Exact(_),
                ..
            }
        ) | (
            Transition::OpenReconciliation | Transition::ReopenReconciliation,
            EffectCaseIndexMutation::CaseResult {
                expected: Absent,
                ..
            }
        ) | (
            Transition::AdvanceReconciliation(_),
            EffectCaseIndexMutation::RequestIdentity {
                expected: Absent,
                ..
            }
        ) | (
            Transition::AdvanceReconciliation(_),
            EffectCaseIndexMutation::CaseResult {
                expected: Exact(_),
                ..
            }
        ) | (
            Transition::AdvanceReconciliation(EffectCaseReconciliationAdvanceKind::Terminal)
                | Transition::TransferCustody
                | Transition::ReopenReconciliation,
            EffectCaseIndexMutation::ActiveCustody {
                expected: Exact(_),
                ..
            }
        )
    )
}

fn assertion_predecessor_shape_is_exact(
    kind: EffectCaseTransitionKind,
    assertion: &EffectCaseIndexAssertion,
) -> bool {
    use EffectCaseIndexPredecessor::{Absent, Exact};
    use EffectCaseReconciliationAdvanceKind as Advance;
    use EffectCaseTransitionKind as Transition;

    matches!(
        (kind, assertion),
        (
            Transition::AdmitAction(EffectCaseActionAdmissionKind::New),
            EffectCaseIndexAssertion::ActiveCustody {
                expected: Absent | Exact(_),
                ..
            }
        ) | (
            Transition::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting),
            EffectCaseIndexAssertion::LogicalAction {
                expected: Exact(_),
                ..
            } | EffectCaseIndexAssertion::ActionDisposition {
                expected: Exact(_),
                ..
            }
        ) | (
            Transition::CommitDispatchIntent,
            EffectCaseIndexAssertion::ActiveClaim {
                expected: Exact(_),
                ..
            }
        ) | (
            Transition::CancelBeforeDispatch,
            EffectCaseIndexAssertion::ActionIntent {
                expected: Absent,
                ..
            }
        ) | (
            Transition::SettleWithoutReconciliation | Transition::OpenReconciliation,
            EffectCaseIndexAssertion::ActionIntent {
                expected: Exact(_),
                ..
            }
        ) | (
            Transition::AdvanceReconciliation(Advance::Open),
            EffectCaseIndexAssertion::ActiveCustody {
                expected: Exact(_),
                ..
            }
        ) | (
            Transition::TransferCustody,
            EffectCaseIndexAssertion::ActionDisposition {
                expected: Exact(_),
                ..
            }
        ) | (
            Transition::ReopenReconciliation,
            EffectCaseIndexAssertion::ActiveClaim {
                expected: Exact(_),
                ..
            } | EffectCaseIndexAssertion::CaseResult {
                expected: Exact(_),
                ..
            }
        )
    )
}

fn required_members(kind: EffectCaseTransitionKind) -> BTreeSet<EffectCaseTransitionMemberKind> {
    use EffectCaseTransitionKind as Transition;
    use EffectCaseTransitionMemberKind as Member;

    match kind {
        Transition::AdmitAction(EffectCaseActionAdmissionKind::New) => [
            Member::CommonRequest,
            Member::Action,
            Member::ConflictClaim,
            Member::Admission,
        ]
        .into_iter()
        .collect(),
        Transition::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting) => {
            [Member::CommonRequest].into_iter().collect()
        }
        Transition::CommitDispatchIntent => [Member::DispatchIntent].into_iter().collect(),
        Transition::CancelBeforeDispatch => [Member::ConflictClaim].into_iter().collect(),
        Transition::SettleWithoutReconciliation => [Member::Receipt, Member::ConflictClaim]
            .into_iter()
            .collect(),
        Transition::OpenReconciliation => [
            Member::Receipt,
            Member::ConflictClaim,
            Member::ReconciliationCase,
            Member::ReconciliationCustody,
        ]
        .into_iter()
        .collect(),
        Transition::AdvanceReconciliation(EffectCaseReconciliationAdvanceKind::Open) => [
            Member::CommonRequest,
            Member::ReconciliationRequest,
            Member::ReconciliationResult,
        ]
        .into_iter()
        .collect(),
        Transition::AdvanceReconciliation(EffectCaseReconciliationAdvanceKind::Terminal) => [
            Member::CommonRequest,
            Member::ReconciliationRequest,
            Member::ReconciliationResult,
            Member::ReconciliationCustody,
        ]
        .into_iter()
        .collect(),
        Transition::TransferCustody => [Member::ReconciliationCustody].into_iter().collect(),
        Transition::ReopenReconciliation => {
            [Member::ReconciliationCase, Member::ReconciliationCustody]
                .into_iter()
                .collect()
        }
    }
}

fn required_index_roles(kind: EffectCaseTransitionKind) -> BTreeSet<EffectCaseIndexRole> {
    use EffectCaseIndexRole as Index;
    use EffectCaseTransitionKind as Transition;

    match kind {
        Transition::AdmitAction(EffectCaseActionAdmissionKind::New) => [
            Index::RequestIdentity,
            Index::LogicalAction,
            Index::ActionDisposition,
            Index::ActiveClaim,
        ]
        .into_iter()
        .collect(),
        Transition::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting) => {
            [Index::RequestIdentity].into_iter().collect()
        }
        Transition::CommitDispatchIntent => [Index::ActionIntent, Index::ActionDisposition]
            .into_iter()
            .collect(),
        Transition::CancelBeforeDispatch | Transition::SettleWithoutReconciliation => {
            [Index::ActiveClaim, Index::ActionDisposition]
                .into_iter()
                .collect()
        }
        Transition::OpenReconciliation => [
            Index::ActiveClaim,
            Index::ActiveCustody,
            Index::CaseResult,
            Index::ActionDisposition,
        ]
        .into_iter()
        .collect(),
        Transition::AdvanceReconciliation(EffectCaseReconciliationAdvanceKind::Open) => [
            Index::RequestIdentity,
            Index::CaseResult,
            Index::ActionDisposition,
        ]
        .into_iter()
        .collect(),
        Transition::AdvanceReconciliation(EffectCaseReconciliationAdvanceKind::Terminal) => [
            Index::RequestIdentity,
            Index::CaseResult,
            Index::ActiveCustody,
            Index::ActionDisposition,
        ]
        .into_iter()
        .collect(),
        Transition::TransferCustody => [Index::ActiveCustody].into_iter().collect(),
        Transition::ReopenReconciliation => [
            Index::ActiveCustody,
            Index::CaseResult,
            Index::ActionDisposition,
        ]
        .into_iter()
        .collect(),
    }
}

fn required_assertion_roles(kind: EffectCaseTransitionKind) -> BTreeSet<EffectCaseAssertionRole> {
    use EffectCaseAssertionRole as Assertion;
    use EffectCaseReconciliationAdvanceKind as Advance;
    use EffectCaseTransitionKind as Transition;

    match kind {
        Transition::AdmitAction(EffectCaseActionAdmissionKind::New) => {
            [Assertion::ActiveCustody].into_iter().collect()
        }
        Transition::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting) => {
            [Assertion::LogicalAction, Assertion::ActionDisposition]
                .into_iter()
                .collect()
        }
        Transition::CommitDispatchIntent => [Assertion::ActiveClaim].into_iter().collect(),
        Transition::CancelBeforeDispatch => [Assertion::ActionIntent].into_iter().collect(),
        Transition::SettleWithoutReconciliation | Transition::OpenReconciliation => {
            [Assertion::ActionIntent].into_iter().collect()
        }
        Transition::AdvanceReconciliation(Advance::Open) => {
            [Assertion::ActiveCustody].into_iter().collect()
        }
        Transition::AdvanceReconciliation(Advance::Terminal) => BTreeSet::new(),
        Transition::TransferCustody => [Assertion::ActionDisposition].into_iter().collect(),
        Transition::ReopenReconciliation => [Assertion::ActiveClaim, Assertion::CaseResult]
            .into_iter()
            .collect(),
    }
}

/// Physical closure failure for one effect-case transition candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EffectCaseCandidateError {
    /// Immutable record roles do not equal the selected transition's exact set.
    #[error("effect-case records do not match the selected transition shape")]
    MemberShape,
    /// A member's record schema does not match its closed enum variant.
    #[error("effect-case member uses the wrong schema identity")]
    MemberSchema,
    /// Typed index roles do not equal the selected transition's exact set.
    #[error("effect-case indexes do not match the selected transition shape")]
    IndexShape,
    /// Physical read roles do not equal the selected transition's exact set.
    #[error("effect-case assertions do not match the selected transition shape")]
    AssertionShape,
    /// A physical read expects an impossible absent/present state for its transition.
    #[error("effect-case assertion predecessor does not match the selected transition")]
    AssertionPredecessorShape,
    /// An index is redundantly represented in both read and write sets.
    #[error("effect-case assertion overlaps a typed index mutation")]
    AssertionMutationOverlap,
    /// Claim and custody indexes do not name the same conflict resource.
    #[error("effect-case claim and custody indexes use different resource keys")]
    ConflictResourceKeyMismatch,
    /// Immutable identity and mutable disposition do not name one logical action.
    #[error("effect-case indexes use different logical-action keys")]
    LogicalActionKeyMismatch,
    /// The selected transition cannot produce or retain this action disposition.
    #[error("effect-case action-disposition transition is not exact")]
    ActionDispositionTransition,
    /// The same typed physical index key is mutated twice.
    #[error("effect-case transition repeats a typed index mutation")]
    DuplicateIndexMutation,
    /// A map/head predecessor is not absent or present as its transition requires.
    #[error("effect-case index predecessor does not match the selected transition")]
    IndexPredecessorShape,
    /// An index successor is absent, wrongly typed, or equal to its predecessor.
    #[error("effect-case index successor is not closed by a new candidate member")]
    IndexSuccessorClosure,
    /// Two transaction rows name the same immutable record coordinate.
    #[error("effect-case transition repeats a record coordinate")]
    DuplicateRecordCoordinate,
    /// The predecessor head and its checkpoint record disagree.
    #[error("effect-case predecessor checkpoint record is not exact")]
    ExpectedCheckpointRecord,
    /// The successor head and its checkpoint record disagree.
    #[error("effect-case successor checkpoint record is not exact")]
    SuccessorCheckpointRecord,
    /// The successor does not advance the same domain and epoch exactly once.
    #[error("effect-case checkpoint is not the exact domain successor")]
    CheckpointSuccession,
    /// The selected transition requires the opposite checkpoint consequence.
    #[error("effect-case checkpoint mutation does not match the transition shape")]
    CheckpointShape,
}

/// Successful physical compare-and-commit outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectCaseCommitOutcome {
    /// The store committed this complete transition atomically.
    Committed,
    /// The complete transition already exists byte-for-byte.
    ExactReplay,
}

/// Exact read-back classification after restart or an ambiguous commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectCaseReadBackOutcome {
    /// The store remains at the exact predecessor and has no candidate rows.
    Predecessor,
    /// Every candidate row and the exact successor are present byte-for-byte.
    ExactSuccessor,
    /// Observed rows are partial, contradictory, or owned by another transition.
    ConflictingOrPartial,
}

/// Atomic physical custody for one admitted effect-case transition.
pub trait EffectCaseCustodyPort: crate::ControlTransactionPort {
    /// Compare the predecessor and commit all records, indexes, and successor.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] for conflict, recovery-required state,
    /// storage failure, or unknown commit outcome. Unknown outcome must be
    /// resolved through [`Self::observe_exact`], never inferred or retried.
    fn compare_and_commit(
        &mut self,
        candidate: &EffectCaseTransitionCandidate,
    ) -> Result<EffectCaseCommitOutcome, ControlStoreError>;

    /// Read back the complete candidate without mutating physical state.
    ///
    /// Exact equality is custody evidence only. It proves no semantic
    /// admission, native effect outcome, external observation, or authority.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when bounded exact observation fails.
    fn observe_exact(
        &mut self,
        candidate: &EffectCaseTransitionCandidate,
    ) -> Result<EffectCaseReadBackOutcome, ControlStoreError>;

    /// Read one exact typed private index without scanning or mutating it.
    ///
    /// The returned observation must preserve `query` and, when present, its
    /// record must inhabit the selected index domain.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when the bounded point read cannot be
    /// completed or its physical integrity cannot be proven.
    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &EffectCaseIndexQuery,
    ) -> Result<EffectCaseIndexObservation, ControlStoreError>;

    /// Read one immutable record by its exact coordinate without scanning.
    ///
    /// A present observation must contain the same coordinate requested by
    /// `coordinate`; application admission remains responsible for the record's
    /// semantic relationships.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when the bounded point read cannot be
    /// completed or its physical integrity cannot be proven.
    fn read_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
    ) -> Result<EffectCaseRecordObservation, ControlStoreError>;
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
#[path = "../tests/internal/effect_case.rs"]
mod tests;
