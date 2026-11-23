//! Read-only association of reconciliation evidence with its durable operation.
//!
//! The operation-custody request index already owns the durable request-to-
//! operation relation. Effect-case records own the attempt lineage. This
//! module joins those existing facts without adding another index or changing
//! either owner. The result proves correlation only. It is not an operation
//! successor, settlement, acceptance, retry permission, or current authority.

use pale_core::records::{
    CapabilityAttemptIdV1, CommonRequestEnvelopeV1, EffectDispatchIntentV1, EffectReceiptDigest,
    EffectReceiptV1, InvalidResidentRecordV1, ReconciliationResultDigest, ReconciliationResultV1,
    ResidentRecordRefV1,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordCoordinate, ControlStoreError,
    EffectCaseCustodyPort, OperationCustodyPort,
};

use crate::resident::{read_operation_start_for_request, OperationRecoveryError};

use super::record::{project_intent, project_receipt, project_request, project_result};
use super::{AdmittedEffectCaseRecord, EffectCaseProjectionError};

/// Admitted effect records whose exact stored bytes must be read back.
pub struct ReconciliationEffectAttemptInput<'a> {
    /// Admitted store containing the existing effect-case records.
    pub store: &'a AuthorityStoreCoordinate,
    /// Original effect request retained by both custody owners.
    pub effect_request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
    /// Durable intent that allocated the effect attempt before dispatch.
    pub dispatch_intent: AdmittedEffectCaseRecord<'a, EffectDispatchIntentV1>,
    /// Exact effect receipt considered by the reconciliation result.
    pub effect_receipt: AdmittedEffectCaseRecord<'a, EffectReceiptV1>,
    /// Reconciliation result that names the receipt and attempt.
    pub reconciliation_result: AdmittedEffectCaseRecord<'a, ReconciliationResultV1>,
}

/// Exact stored effect lineage admitted for an operation association read.
///
/// Construction requires byte-for-byte read-back of every supplied record at
/// its supplied immutable reference. The contained semantic records must have
/// passed their existing contextual admission before this read. This proof is
/// historical evidence only, not current effect or operation authority.
#[derive(Clone, Copy)]
pub struct RecoveredReconciliationEffectAttempt<'a> {
    store: &'a AuthorityStoreCoordinate,
    effect_request: AdmittedEffectCaseRecord<'a, CommonRequestEnvelopeV1>,
    dispatch_intent: AdmittedEffectCaseRecord<'a, EffectDispatchIntentV1>,
    effect_receipt: AdmittedEffectCaseRecord<'a, EffectReceiptV1>,
    reconciliation_result: AdmittedEffectCaseRecord<'a, ReconciliationResultV1>,
}

/// Existing operation selection plus one stored reconciliation attempt proof.
pub struct ReconciliationOperationAssociationInput<'a> {
    /// Effect-case evidence recovered from the same admitted store incarnation.
    pub effect_attempt: RecoveredReconciliationEffectAttempt<'a>,
    /// Operation handle selected by the caller's already-admitted operation path.
    pub expected_operation_handle: &'a ControlRecordCoordinate,
    /// Independent maximum for each operation-custody record read.
    pub maximum_record_bytes: usize,
}

/// Exact read-only relation between one reconciliation result and one effect attempt.
///
/// The operation handle is the coordinate already retained by operation
/// custody. The remaining coordinates are immutable effect-case evidence. This
/// value says only that the attempt belongs to that durable operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationOperationAssociation {
    operation_handle: ControlRecordCoordinate,
    effect_attempt_id: CapabilityAttemptIdV1,
    dispatch_intent_ref: ResidentRecordRefV1,
    effect_receipt_ref: ResidentRecordRefV1,
    effect_receipt_sha256: EffectReceiptDigest,
    reconciliation_result_ref: ResidentRecordRefV1,
    reconciliation_result_sha256: ReconciliationResultDigest,
}

impl ReconciliationOperationAssociation {
    /// Borrow the exact durable operation-handle coordinate.
    #[must_use]
    pub const fn operation_handle(&self) -> &ControlRecordCoordinate {
        &self.operation_handle
    }

    /// Borrow the exact effect-attempt identity.
    #[must_use]
    pub const fn effect_attempt_id(&self) -> &CapabilityAttemptIdV1 {
        &self.effect_attempt_id
    }

    /// Borrow the immutable dispatch-intent reference.
    #[must_use]
    pub const fn dispatch_intent_ref(&self) -> &ResidentRecordRefV1 {
        &self.dispatch_intent_ref
    }

    /// Borrow the immutable effect-receipt reference and digest.
    #[must_use]
    pub const fn effect_receipt(&self) -> (&ResidentRecordRefV1, &EffectReceiptDigest) {
        (&self.effect_receipt_ref, &self.effect_receipt_sha256)
    }

    /// Borrow the immutable reconciliation-result reference and digest.
    #[must_use]
    pub const fn reconciliation_result(
        &self,
    ) -> (&ResidentRecordRefV1, &ReconciliationResultDigest) {
        (
            &self.reconciliation_result_ref,
            &self.reconciliation_result_sha256,
        )
    }
}

/// Read back and admit the exact stored effect lineage used by the join.
///
/// Each record is first checked by its existing core structural law and the
/// complete cross-record lineage below. The function then projects the same
/// admitted value through the effect-case storage owner and requires exact
/// byte-for-byte point-read equality. It writes nothing.
///
/// # Errors
///
/// Returns a closed error when a semantic record is invalid, the effect
/// lineage differs, projection fails, or exact stored bytes are absent or
/// unlike. This does not repeat the existing historical signature admission.
pub fn recover_reconciliation_effect_attempt<'a>(
    input: ReconciliationEffectAttemptInput<'a>,
    effect_store: &mut dyn EffectCaseCustodyPort,
) -> Result<RecoveredReconciliationEffectAttempt<'a>, ReconciliationOperationAssociationError> {
    let request = input.effect_request.record;
    let intent = input.dispatch_intent.record;
    let receipt = input.effect_receipt.record;
    let result = input.reconciliation_result.record;

    request.verify()?;
    intent.verify()?;
    receipt.verify()?;
    result.verify()?;

    let request_binding = request.local_request_binding()?;
    let receipt_is_considered = result.effect_receipts().iter().any(|coordinate| {
        coordinate.receipt_ref == *input.effect_receipt.record_ref
            && coordinate.receipt_sha256 == *receipt.canonical_receipt_sha256()
    });
    let attempt_is_known = result
        .known_attempt_ids()
        .binary_search(receipt.attempt_id())
        .is_ok();
    let exact = intent.request_ref() == input.effect_request.record_ref
        && intent.canonical_request_sha256() == request.canonical_request_sha256()
        && receipt.request_binding() == &request_binding
        && receipt.effect_dispatch_intent_ref() == input.dispatch_intent.record_ref
        && receipt.effect_dispatch_intent_sha256() == intent.canonical_dispatch_intent_sha256()
        && receipt.attempt_id() == intent.attempt_id()
        && receipt.action_id() == intent.action_id()
        && receipt.action_id() == result.action_id()
        && receipt.effect_admission_ref() == intent.effect_admission_ref()
        && receipt.effect_admission_ref() == result.effect_admission_ref()
        && receipt.effect_admission_sha256() == intent.effect_admission_sha256()
        && receipt.effect_admission_sha256() == result.effect_admission_sha256()
        && receipt.principal_scope() == &request.scope().principal_scope
        && receipt.controller_scope() == &request.scope().controller_scope
        && receipt.relationship_scope() == &request.scope().relationship_scope
        && result.principal_scope() == &request.scope().principal_scope
        && result.controller_scope() == &request.scope().controller_scope
        && result.relationship_scope() == &request.scope().relationship_scope
        && receipt_is_considered
        && attempt_is_known;
    if !exact {
        return Err(ReconciliationOperationAssociationError::EffectLineage);
    }

    for expected in [
        project_request(input.effect_request)?,
        project_intent(input.dispatch_intent)?,
        project_receipt(input.effect_receipt)?,
        project_result(input.reconciliation_result)?,
    ] {
        require_exact_effect_record(effect_store, input.store, &expected)?;
    }

    Ok(RecoveredReconciliationEffectAttempt {
        store: input.store,
        effect_request: input.effect_request,
        dispatch_intent: input.dispatch_intent,
        effect_receipt: input.effect_receipt,
        reconciliation_result: input.reconciliation_result,
    })
}

fn require_exact_effect_record(
    effect_store: &mut dyn EffectCaseCustodyPort,
    store: &AuthorityStoreCoordinate,
    expected: &ControlRecord,
) -> Result<(), ReconciliationOperationAssociationError> {
    let observation = effect_store.read_record(store, expected.coordinate())?;
    match observation.record() {
        Some(stored) if stored == expected => Ok(()),
        Some(_) => Err(ReconciliationOperationAssociationError::EffectEvidenceMismatch),
        None => Err(ReconciliationOperationAssociationError::MissingEffectEvidence),
    }
}

/// Read the operation selected by the original request and prove its effect attempt.
///
/// The function performs the existing bounded operation-start point reads. It
/// writes nothing and does not inspect or advance the latest operation state.
/// A result can therefore arrive after an unknown-outcome observation without
/// being mistaken for an operation transition.
///
/// # Errors
///
/// Returns a closed error when the request has no durable operation, the
/// selected operation is not that request's operation, or bounded operation
/// recovery fails.
pub fn read_reconciliation_operation_association(
    input: ReconciliationOperationAssociationInput<'_>,
    operation_store: &mut dyn OperationCustodyPort,
) -> Result<ReconciliationOperationAssociation, ReconciliationOperationAssociationError> {
    let attempt = input.effect_attempt;
    let request = attempt.effect_request.record;
    let receipt = attempt.effect_receipt.record;
    let result = attempt.reconciliation_result.record;

    let recovered = read_operation_start_for_request(
        operation_store,
        attempt.store,
        request,
        input.maximum_record_bytes,
    )?
    .ok_or(ReconciliationOperationAssociationError::MissingOperation)?;
    if recovered.request().coordinate().record_ref() != attempt.effect_request.record_ref
        || recovered.handle().coordinate() != input.expected_operation_handle
    {
        return Err(ReconciliationOperationAssociationError::OperationMismatch);
    }

    Ok(ReconciliationOperationAssociation {
        operation_handle: recovered.handle().coordinate().clone(),
        effect_attempt_id: receipt.attempt_id().clone(),
        dispatch_intent_ref: attempt.dispatch_intent.record_ref.clone(),
        effect_receipt_ref: attempt.effect_receipt.record_ref.clone(),
        effect_receipt_sha256: receipt.canonical_receipt_sha256().clone(),
        reconciliation_result_ref: attempt.reconciliation_result.record_ref.clone(),
        reconciliation_result_sha256: result.canonical_reconciliation_sha256().clone(),
    })
}

/// Closed association-read failure classification.
#[derive(Debug, thiserror::Error)]
pub enum ReconciliationOperationAssociationError {
    /// One supplied semantic record failed its own structural admission law.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// Canonical projection of an admitted effect-case record failed.
    #[error(transparent)]
    Projection(#[from] EffectCaseProjectionError),
    /// Exact point-read from either custody owner failed.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// Existing bounded operation-custody recovery failed.
    #[error(transparent)]
    OperationRecovery(#[from] OperationRecoveryError),
    /// The exact original request has no durable operation start.
    #[error("effect request has no durable operation")]
    MissingOperation,
    /// One effect-case record was absent from its exact supplied coordinate.
    #[error("effect attempt evidence is not retained")]
    MissingEffectEvidence,
    /// Stored effect-case bytes differ from the admitted value at that coordinate.
    #[error("retained effect attempt evidence differs from the admitted value")]
    EffectEvidenceMismatch,
    /// The selected operation is not the operation retained for this request.
    #[error("effect request resolves to another durable operation")]
    OperationMismatch,
    /// The result, receipt, intent, attempt, or request lineage differs.
    #[error("reconciliation result does not name this effect attempt")]
    EffectLineage,
}
