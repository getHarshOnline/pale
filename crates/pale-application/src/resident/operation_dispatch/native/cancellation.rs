//! Prepare same-owner cancellation while the selected native call remains uninvoked.

use super::*;
use crate::diagnostic::{AdmittedDiagnostic, DiagnosticAdmissionError, DiagnosticProjection};
use crate::resident::record_projection;

/// Current same-session authority and exact retained preparation to cancel.
///
/// This is not an external Cancel request or authority over another session.
pub struct NativeOperationCancellationInput<'a> {
    /// Database-and-anchor evidence of this exact preparation.
    pub completed: &'a CompletedOperationObservationCustody,
    /// Current authority of the original admitted session.
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    /// Independently admitted observation signer for the current runtime.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh immutable reference for the cancellation diagnostic.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the cancelled observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Maximum canonical diagnostic bytes admitted before signing.
    pub maximum_diagnostic_bytes: usize,
    /// Existing owner of the operation's storage coordinate.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
}

/// Signed cancellation evidence awaiting the existing checkpoint and atomic append.
///
/// This borrows the operation and owns the exact predecessor observation. It
/// grants no invocation permission and does not establish that cancellation won
/// its storage race.
#[must_use = "commit the exact cancellation append before reporting terminal cancellation"]
pub struct PreparedNativeOperationCancellation<'a> {
    operation: &'a IssuedLocalOperation,
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    diagnostic: AdmittedDiagnostic,
    diagnostic_record: ControlRecord,
}

/// Which unopened-call transition supplies the cancellation evidence.
pub(super) enum NativeCancellationPoint {
    BeforeDispatch,
    BeforeRelease,
}

impl<'operation> PreparedNativeOperationAttempt<'operation, '_> {
    /// Prepare cancellation of this owner's retained, self-contained native call.
    ///
    /// The call remains retained when preparation or signing is refused. No handler
    /// runs and no cancellation token is sent. The original session, request and
    /// lease must still be valid; this method grants no cancellation authority
    /// after their execution validity expires.
    ///
    /// The exact preparation must already have database-and-anchor evidence.
    /// That historical evidence need not still be current: the atomic append
    /// decides whether cancellation or dispatch wins against the same predecessor.
    /// A losing cancellation cannot assert that a dispatched call never started.
    ///
    /// # Errors
    /// Rejects unlike custody, stale authority, retained context obligations,
    /// reused references, oversized diagnostics and observation-signing failures.
    pub fn prepare_cancellation<Authority, Rotation, Sign>(
        &self,
        input: NativeOperationCancellationInput<'_>,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedNativeOperationCancellation<'_>, OperationAttemptError>
    where
        Authority: AmbientSessionAuthorityPort,
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
        self.context.validate_current_at(input.current, authority)?;
        self.context.require_context_ready()?;
        self.prepare_append(input.resolve_store)?
            .verify_completed_custody(input.completed)?;
        self.prepare_cancellation_evidence(
            self.preparation.observation(),
            NativeCancellationPoint::BeforeDispatch,
            input,
            admit_rotation,
            sign,
        )
    }

    // The caller proves custody and current authority for its own unopened phase.
    // This shared owner constructs the diagnostic and exact successor, not permission.
    pub(super) fn prepare_cancellation_evidence<Rotation, Sign>(
        &self,
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
        point: NativeCancellationPoint,
        input: NativeOperationCancellationInput<'_>,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<PreparedNativeOperationCancellation<'operation>, OperationAttemptError>
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
        if input.diagnostic_ref == input.observation_ref {
            return Err(OperationAttemptError::Association);
        }
        self.validate_successor_reference(&input.diagnostic_ref)?;
        self.validate_successor_reference(&input.observation_ref)?;
        if input.diagnostic_ref == *predecessor.0 || input.observation_ref == *predecessor.0 {
            return Err(OperationAttemptError::Association);
        }
        signing::validate_runtime_signer(
            input.signing.signing_authority,
            input.current,
            OPERATION_OBSERVATION_V1_SCHEMA,
        )?;
        let (action_id, request_digest) = self
            .selection
            .native_action_identity()
            .ok_or(OperationAttemptError::Association)?;
        let identity = pale_contracts::record_schema_identity("diagnostic.v1")
            .map_err(DiagnosticAdmissionError::Registry)?;
        let (code, summary) = match point {
            NativeCancellationPoint::BeforeDispatch => (
                "native_operation_cancelled_before_dispatch",
                "native action was cancelled before dispatch",
            ),
            NativeCancellationPoint::BeforeRelease => (
                "native_operation_cancelled_before_release",
                "native action was cancelled before its handler was released",
            ),
        };
        let diagnostic = DiagnosticProjection {
            code: code.to_string(),
            stage: DiagnosticStageV1::NativeAction,
            disposition: DiagnosticDispositionV1::Blocked,
            summary: summary.to_string(),
            subject: Some(("action".to_string(), action_id.to_string())),
            source: None,
            schema_reference: None,
            causes: Vec::new(),
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: Some((
                "native_action_request".to_string(),
                request_digest.to_string(),
            )),
            generation: None,
            receipt: None,
            omitted_causes: 0,
        }
        .admit(&identity)?;
        if diagnostic.canonical_bytes().len() > input.maximum_diagnostic_bytes {
            return Err(OperationAttemptError::BoundExceeded);
        }
        let (diagnostic_digest, diagnostic_record) =
            record_projection::diagnostic_record(&input.diagnostic_ref, &diagnostic)
                .map_err(OperationObservationCustodyProjectionError::from)?;
        let (handle_ref, handle) = self.context.operation.handle();
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
            input.current.now,
            admit_rotation,
            sign,
        )?;
        Ok(PreparedNativeOperationCancellation {
            operation: self.context.operation,
            predecessor: (predecessor.0.clone(), predecessor.1.clone()),
            observation_ref: input.observation_ref,
            observation,
            diagnostic,
            diagnostic_record,
        })
    }
}

impl PreparedNativeOperationCancellation<'_> {
    /// Borrow the diagnostic without claiming it has been retained.
    #[must_use]
    pub fn diagnostic(&self) -> &AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the signed, not-yet-committed cancellation observation.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Prepare both records against the retained preparation or dispatch observation.
    ///
    /// # Errors
    /// Preserves store-resolution, lineage and physical projection failures.
    pub fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        prepare_diagnostic_observation_append(
            self.operation.handle(),
            (&self.predecessor.0, &self.predecessor.1),
            self.observation(),
            &self.diagnostic_record,
            resolve_store,
        )
    }
}
