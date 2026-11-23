//! Application-owned closure and custody phases for one resident effect case.
//!
//! The module keeps semantic records typed until a single canonical-JCS
//! projection, derives every private index key from an explicit initial-v1
//! domain preimage, and then hands one closed transaction to
//! [`pale_ports::EffectCaseCustodyPort`]. An effect dispatch becomes affine evidence only
//! after both the database transaction and its independently retained
//! hard-domain anchor are proven. In the law below, `C_db` is the database
//! commit disposition and `C_anchor` is the independent-anchor disposition.
//! Each must be either newly committed or recovered exactly before dispatch is
//! eligible:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/effect-case-dispatch-eligibility.display.mathml"
))]
//!
//! Exact replay is custody evidence, never permission to redispatch. Neither
//! this module nor its port performs a provider call or native effect.
use pale_core::canon;
use pale_core::records::{
    ActionEnvelopeV1, CommonRequestEnvelopeV1, ControlStateCheckpointV1, EffectAdmissionV1,
    EffectConflictClaimV1, EffectDispatchIntentV1, EffectReceiptV1, InvalidResidentRecordV1,
    ReconciliationCaseV1, ReconciliationCustodyV1, ReconciliationRequestV1, ReconciliationResultV1,
    ResidentRecordRefV1,
};
use pale_ports::{
    AntiRollbackAnchorTransitionError, AuthorityStoreCoordinate, ControlRecordError,
    EffectCaseActionAdmissionKind, EffectCaseCandidateError, EffectCaseCheckpointMutation,
    EffectCaseIndexAssertion, EffectCaseIndexMutation, EffectCaseTransitionCandidate,
    EffectCaseTransitionKind, EffectCaseTransitionMember,
};

use super::record_projection::ControlRecordProjectionError;

mod operation_association;
mod phase;
mod projection;
mod record;
mod replay;

use projection::{project_checkpoint, project_transition};

pub use operation_association::{
    read_reconciliation_operation_association, recover_reconciliation_effect_attempt,
    ReconciliationEffectAttemptInput, ReconciliationOperationAssociation,
    ReconciliationOperationAssociationError, ReconciliationOperationAssociationInput,
    RecoveredReconciliationEffectAttempt,
};
pub use phase::{
    CompletedEffectCaseTransition, DatabaseCommittedEffectCaseAnchorPending,
    EffectCaseAnchorEvidence, EffectCaseAnchorFailure, EffectCaseAnchorFailureReason,
    EffectCaseCommitFailure, EffectCaseCommitFailureReason, EffectCaseCommitSuccess,
    EffectCaseDatabaseCommitEvidence, EffectDispatchAuthorization, PreparedEffectCaseTransition,
};
pub use replay::{
    lookup_effect_case_replay, EffectCaseReplayDisposition, EffectCaseReplayLookupError,
};

/// Separates a caller, signer epoch, and request identity from every other index tuple.
const REQUEST_IDENTITY_KEY_DOMAIN: &[u8] = b"PALE_EFFECT_CASE_REQUEST_IDENTITY_INDEX_KEY_V1\0";
/// Separates an action target, operation, and business idempotency identity.
const LOGICAL_ACTION_KEY_DOMAIN: &[u8] = b"PALE_EFFECT_CASE_LOGICAL_ACTION_INDEX_KEY_V1\0";
/// Separates the typed resource over which mutually exclusive effect custody is held.
const CONFLICT_RESOURCE_KEY_DOMAIN: &[u8] = b"PALE_EFFECT_CASE_CONFLICT_RESOURCE_INDEX_KEY_V1\0";
/// Separates one reconciliation case within its scoped logical action.
const CASE_RESULT_KEY_DOMAIN: &[u8] = b"PALE_EFFECT_CASE_CASE_RESULT_INDEX_KEY_V1\0";

/// One typed admitted record paired with its immutable resident reference.
pub struct AdmittedEffectCaseRecord<'a, R> {
    record_ref: &'a ResidentRecordRefV1,
    record: &'a R,
}

impl<R> Copy for AdmittedEffectCaseRecord<'_, R> {}

impl<R> Clone for AdmittedEffectCaseRecord<'_, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, R> AdmittedEffectCaseRecord<'a, R> {
    /// Bind the already-resolved immutable reference to its admitted value.
    #[must_use]
    pub const fn new(record_ref: &'a ResidentRecordRefV1, record: &'a R) -> Self {
        Self { record_ref, record }
    }
}

/// The exact immutable record currently naming one logical action's disposition.
#[derive(Clone, Copy)]
pub enum EffectCaseDisposition<'a> {
    /// An admitted action that has not yet acquired a dispatch intent.
    Admission(AdmittedEffectCaseRecord<'a, EffectAdmissionV1>),
    /// A terminal released or transferred conflict claim.
    Claim(AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>),
    /// A durable intent that may or may not have reached native transport.
    DispatchIntent(AdmittedEffectCaseRecord<'a, EffectDispatchIntentV1>),
    /// An immutable native observation, whether certain or ambiguous.
    Receipt(AdmittedEffectCaseRecord<'a, EffectReceiptV1>),
    /// An open or reopened reconciliation case.
    ReconciliationCase(AdmittedEffectCaseRecord<'a, ReconciliationCaseV1>),
    /// The latest immutable reconciliation observation for a case.
    ReconciliationResult(AdmittedEffectCaseRecord<'a, ReconciliationResultV1>),
}

/// Complete typed semantic input for one of the eight physical transitions.
pub enum EffectCaseTransitionInput<'a> {
    /// Admit a new logical action and acquire its active conflict claim.
    AdmitNew {
        /// Exact signed request carrying `action`.
        request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Immutable action body bound by `request`.
        action: AdmittedEffectCaseRecord<'a, ActionEnvelopeV1>,
        /// New active conflict claim for the action resource.
        claim: AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>,
        /// Independent semantic admission of the request and action.
        admission: AdmittedEffectCaseRecord<'a, EffectAdmissionV1>,
        /// Exact terminal claim previously heading the resource, when present.
        prior_claim: Option<AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>>,
        /// Exact closed custody paired with a transferred prior claim, when present.
        prior_custody: Option<AdmittedEffectCaseRecord<'a, ReconciliationCustodyV1>>,
    },
    /// Bind another exact signed request to an already admitted logical action.
    AliasExisting {
        /// New signed request identity being admitted as an alias.
        request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Action body whose scoped idempotency identity already exists.
        action: AdmittedEffectCaseRecord<'a, ActionEnvelopeV1>,
        /// Immutable admission that owns the existing logical-action map.
        admission: AdmittedEffectCaseRecord<'a, EffectAdmissionV1>,
        /// Exact current disposition asserted without advancing it.
        disposition: EffectCaseDisposition<'a>,
    },
    /// Persist the sole initial-v1 dispatch intent before any native call.
    CommitDispatchIntent {
        /// Original admitted effect request.
        request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Action body carried by `request`.
        action: AdmittedEffectCaseRecord<'a, ActionEnvelopeV1>,
        /// Current semantic admission.
        admission: AdmittedEffectCaseRecord<'a, EffectAdmissionV1>,
        /// Exact active conflict claim asserted by the transition.
        active_claim: AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>,
        /// Preallocated attempt and durable dispatch intent.
        intent: AdmittedEffectCaseRecord<'a, EffectDispatchIntentV1>,
    },
    /// Release an admitted action after cancellation but before dispatch.
    CancelBeforeDispatch {
        /// Original admitted effect request.
        request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Action body carried by `request`.
        action: AdmittedEffectCaseRecord<'a, ActionEnvelopeV1>,
        /// Current semantic admission.
        admission: AdmittedEffectCaseRecord<'a, EffectAdmissionV1>,
        /// Claim that must still head the resource.
        active_claim: AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>,
        /// Terminal cancelled successor claim.
        released_claim: AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>,
    },
    /// Retain a terminal receipt and release its claim atomically.
    SettleWithoutReconciliation {
        /// Original admitted effect request.
        request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Action body carried by `request`.
        action: AdmittedEffectCaseRecord<'a, ActionEnvelopeV1>,
        /// Durable intent that preceded the observed effect.
        intent: AdmittedEffectCaseRecord<'a, EffectDispatchIntentV1>,
        /// Claim that must still head the resource.
        active_claim: AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>,
        /// Terminal released successor claim bound to `receipt`.
        released_claim: AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>,
        /// Terminal effect observation that requires no reconciliation.
        receipt: AdmittedEffectCaseRecord<'a, EffectReceiptV1>,
    },
    /// Retain an ambiguous receipt while opening reconciliation custody.
    OpenReconciliation {
        /// Original admitted effect request.
        request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Action body carried by `request`.
        action: AdmittedEffectCaseRecord<'a, ActionEnvelopeV1>,
        /// Durable intent that preceded the ambiguous observation.
        intent: AdmittedEffectCaseRecord<'a, EffectDispatchIntentV1>,
        /// Claim that must still head the resource.
        active_claim: AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>,
        /// Terminal claim transferring exclusive resolution to `custody`.
        transferred_claim: AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>,
        /// Ambiguous effect observation requiring reconciliation.
        receipt: AdmittedEffectCaseRecord<'a, EffectReceiptV1>,
        /// Initial reconciliation case reserved by `intent`.
        case: AdmittedEffectCaseRecord<'a, ReconciliationCaseV1>,
        /// New active custody for `case`.
        custody: AdmittedEffectCaseRecord<'a, ReconciliationCustodyV1>,
        /// Exact closed prior custody when the resource was transferred before.
        prior_custody: Option<AdmittedEffectCaseRecord<'a, ReconciliationCustodyV1>>,
    },
    /// Append one read-only reconciliation observation under active custody.
    AdvanceReconciliation {
        /// Original effect request used to recover the logical-action key.
        effect_request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Original immutable action body.
        action: AdmittedEffectCaseRecord<'a, ActionEnvelopeV1>,
        /// Signed reconciliation request being appended as immutable lineage.
        request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Typed reconciliation body carried by `request`.
        reconciliation_request: AdmittedEffectCaseRecord<'a, ReconciliationRequestV1>,
        /// Case receiving the observation.
        case: AdmittedEffectCaseRecord<'a, ReconciliationCaseV1>,
        /// Exact case or result currently naming the action disposition.
        predecessor: EffectCaseDisposition<'a>,
        /// Exact active reconciliation custody.
        custody: AdmittedEffectCaseRecord<'a, ReconciliationCustodyV1>,
        /// New immutable reconciliation observation.
        result: AdmittedEffectCaseRecord<'a, ReconciliationResultV1>,
        /// Terminal custody successor, required exactly when `result` closes the case.
        closed_custody: Option<AdmittedEffectCaseRecord<'a, ReconciliationCustodyV1>>,
    },
    /// Transfer open reconciliation custody without rewriting case history.
    TransferCustody {
        /// Original admitted effect request.
        request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Original immutable action body.
        action: AdmittedEffectCaseRecord<'a, ActionEnvelopeV1>,
        /// Exact current logical-action disposition asserted by the transfer.
        disposition: EffectCaseDisposition<'a>,
        /// Active predecessor custody.
        custody: AdmittedEffectCaseRecord<'a, ReconciliationCustodyV1>,
        /// Active successor custody.
        successor: AdmittedEffectCaseRecord<'a, ReconciliationCustodyV1>,
    },
    /// Reopen a previously closed case under newly acquired custody.
    ReopenReconciliation {
        /// Original admitted effect request.
        request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
        /// Original immutable action body.
        action: AdmittedEffectCaseRecord<'a, ActionEnvelopeV1>,
        /// Terminal transferred claim still heading the resource.
        active_claim: AdmittedEffectCaseRecord<'a, EffectConflictClaimV1>,
        /// Case closed by `predecessor_result`.
        predecessor_case: AdmittedEffectCaseRecord<'a, ReconciliationCaseV1>,
        /// Exact terminal predecessor result.
        predecessor_result: AdmittedEffectCaseRecord<'a, ReconciliationResultV1>,
        /// Exact closed predecessor custody.
        closed_custody: AdmittedEffectCaseRecord<'a, ReconciliationCustodyV1>,
        /// New predecessor-bound reopened case.
        case: AdmittedEffectCaseRecord<'a, ReconciliationCaseV1>,
        /// New active custody for the reopened case.
        custody: AdmittedEffectCaseRecord<'a, ReconciliationCustodyV1>,
    },
}

/// Exact admitted hard-domain checkpoint pair plus its construction verifier.
#[derive(Clone, Copy)]
pub struct AdmittedEffectCaseCheckpointTransition<'a> {
    predecessor_ref: &'a ResidentRecordRefV1,
    predecessor: &'a ControlStateCheckpointV1,
    successor_ref: &'a ResidentRecordRefV1,
    successor: &'a ControlStateCheckpointV1,
    verifier: &'a dyn EffectCaseCheckpointConstructionVerifier,
}

impl<'a> AdmittedEffectCaseCheckpointTransition<'a> {
    /// Bind an admitted checkpoint pair to the owner of its accumulator proof.
    #[must_use]
    pub const fn new(
        predecessor_ref: &'a ResidentRecordRefV1,
        predecessor: &'a ControlStateCheckpointV1,
        successor_ref: &'a ResidentRecordRefV1,
        successor: &'a ControlStateCheckpointV1,
        verifier: &'a dyn EffectCaseCheckpointConstructionVerifier,
    ) -> Self {
        Self {
            predecessor_ref,
            predecessor,
            successor_ref,
            successor,
            verifier,
        }
    }
}

/// Exact projected mutation shown to the checkpoint-construction owner.
pub struct EffectCaseCheckpointConstruction<'a> {
    kind: EffectCaseTransitionKind,
    members: &'a [EffectCaseTransitionMember],
    mutations: &'a [EffectCaseIndexMutation],
    assertions: &'a [EffectCaseIndexAssertion],
    checkpoints: &'a AdmittedEffectCaseCheckpointTransition<'a>,
}

impl EffectCaseCheckpointConstruction<'_> {
    /// Return the physical transition kind being checkpointed.
    #[must_use]
    pub const fn kind(&self) -> EffectCaseTransitionKind {
        self.kind
    }
    /// Borrow the exact immutable records appended by the transaction.
    #[must_use]
    pub const fn members(&self) -> &[EffectCaseTransitionMember] {
        self.members
    }
    /// Borrow the complete typed compare-and-set mutation set.
    #[must_use]
    pub const fn mutations(&self) -> &[EffectCaseIndexMutation] {
        self.mutations
    }
    /// Borrow the complete non-mutating read-assertion set.
    #[must_use]
    pub const fn assertions(&self) -> &[EffectCaseIndexAssertion] {
        self.assertions
    }
    /// Borrow the admitted predecessor and successor checkpoint records.
    #[must_use]
    pub const fn checkpoints(&self) -> (&ControlStateCheckpointV1, &ControlStateCheckpointV1) {
        (self.checkpoints.predecessor, self.checkpoints.successor)
    }
}

/// Owner boundary proving that a signed checkpoint represents this exact mutation.
pub trait EffectCaseCheckpointConstructionVerifier {
    /// Verify that `construction` is exactly represented by its signed successor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectCaseCheckpointConstructionRejection`] when the supplied
    /// checkpoint was not constructed from the complete projected mutation.
    fn verify(
        &self,
        construction: &EffectCaseCheckpointConstruction<'_>,
    ) -> Result<(), EffectCaseCheckpointConstructionRejection>;
}

impl<F> EffectCaseCheckpointConstructionVerifier for F
where
    F: Fn(
        &EffectCaseCheckpointConstruction<'_>,
    ) -> Result<(), EffectCaseCheckpointConstructionRejection>,
{
    fn verify(
        &self,
        construction: &EffectCaseCheckpointConstruction<'_>,
    ) -> Result<(), EffectCaseCheckpointConstructionRejection> {
        self(construction)
    }
}

/// The checkpoint owner rejected the exact projected mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("control checkpoint does not represent the exact effect-case mutation")]
pub struct EffectCaseCheckpointConstructionRejection;

/// Semantic or physical projection failure before database access.
#[derive(Debug, thiserror::Error)]
pub enum EffectCaseProjectionError {
    /// An admitted semantic record failed its own invariant verification.
    #[error(transparent)]
    Semantic(#[from] InvalidResidentRecordV1),
    /// Records or checkpoints do not form the selected transition lineage.
    #[error("effect-case records do not form the selected exact lineage")]
    Lineage,
    /// A typed key or record could not be canonically encoded.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// A projected immutable record was not physically admissible.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// The projected transaction did not satisfy its closed physical shape.
    #[error(transparent)]
    Candidate(#[from] EffectCaseCandidateError),
    /// The checkpoint pair did not form an admissible anti-rollback transition.
    #[error(transparent)]
    Anchor(#[from] AntiRollbackAnchorTransitionError),
    /// The checkpoint-construction owner rejected the exact mutation witness.
    #[error(transparent)]
    Checkpoint(#[from] EffectCaseCheckpointConstructionRejection),
}

impl From<ControlRecordProjectionError> for EffectCaseProjectionError {
    fn from(value: ControlRecordProjectionError) -> Self {
        match value {
            ControlRecordProjectionError::Canonicalize(error) => Self::Canonicalize(error),
            ControlRecordProjectionError::Record(error) => Self::Record(error),
        }
    }
}

/// Prepare the exact physical transaction selected by `input`.
///
/// Every non-alias transition requires one exact checkpoint successor and a
/// successful owner construction witness. Alias admission changes only the
/// request-identity map and therefore neither advances nor verifies a domain
/// checkpoint.
///
/// # Errors
///
/// Returns [`EffectCaseProjectionError`] when semantic lineage, canonical
/// projection, the closed physical transaction shape, checkpoint succession,
/// or the owner-provided checkpoint construction witness is invalid.
pub fn prepare_effect_case_transition(
    store: AuthorityStoreCoordinate,
    input: EffectCaseTransitionInput<'_>,
    checkpoint: Option<AdmittedEffectCaseCheckpointTransition<'_>>,
) -> Result<PreparedEffectCaseTransition, EffectCaseProjectionError> {
    let projected = project_transition(input)?;
    let (checkpoint_mutation, anchor) = match (projected.kind, checkpoint) {
        (
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting),
            None,
        ) => (EffectCaseCheckpointMutation::Unchanged, None),
        (
            EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting),
            Some(_),
        )
        | (_, None) => return Err(EffectCaseProjectionError::Lineage),
        (_, Some(checkpoints)) => {
            let (mutation, anchor) = project_checkpoint(
                &store,
                projected.kind,
                &projected.members,
                &projected.mutations,
                &projected.assertions,
                &checkpoints,
            )?;
            (
                EffectCaseCheckpointMutation::Advance(Box::new(mutation)),
                Some(anchor),
            )
        }
    };
    let candidate = EffectCaseTransitionCandidate::new(
        store,
        projected.kind,
        projected.members,
        projected.mutations,
        projected.assertions,
        checkpoint_mutation,
    )?;
    Ok(PreparedEffectCaseTransition {
        candidate,
        anchor,
        dispatch_intent: projected.dispatch_intent,
    })
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/effect_case.rs"]
mod tests;
