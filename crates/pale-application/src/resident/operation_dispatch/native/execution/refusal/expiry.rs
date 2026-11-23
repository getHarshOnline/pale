//! Record observed expiry of the original unopened native call.

use super::*;
use crate::diagnostic::{AdmittedDiagnostic, DiagnosticAdmissionError, DiagnosticProjection};
use crate::resident::record_projection;

/// Recording inputs, not renewed execution or cancellation authority.
pub struct NativeOperationExpiryInput<'a> {
    /// Independently admitted observation signer at recording time, as for a late result.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Fresh immutable reference for the existing diagnostic contract.
    pub diagnostic_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the expiry observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Recording time within retention, no earlier than the retained time history.
    /// This supplied value does not establish expiry or clock trust.
    pub observed_at: UnixTimeV1,
    /// Maximum canonical diagnostic bytes accepted before signing.
    pub maximum_diagnostic_bytes: usize,
    /// Per-record ceiling for exact original dispatch-custody reads.
    pub maximum_record_bytes: usize,
    /// Existing owner of the operation's storage coordinate.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
}

/// Failed recording preparation with the same unopened call and time history.
#[must_use = "inspect the failure and retain the original refusal without renewing execution"]
pub struct NativeOperationExpiryRefusal<'operation, 'registry> {
    retained: Box<(
        OperationAttemptError,
        NativeOperationInvocationRefusal<'operation, 'registry>,
    )>,
}

impl std::fmt::Debug for NativeOperationExpiryRefusal<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("NativeOperationExpiryRefusal { retained: .. }")
    }
}

impl std::fmt::Display for NativeOperationExpiryRefusal<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("native expiry was not prepared")
    }
}

impl std::error::Error for NativeOperationExpiryRefusal<'_, '_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.retained.0)
    }
}

impl<'operation, 'registry> NativeOperationExpiryRefusal<'operation, 'registry> {
    /// Inspect the recording failure, separately from the original release refusal.
    #[must_use]
    pub fn reason(&self) -> &OperationAttemptError {
        &self.retained.0
    }

    /// Recover the same unopened call, without changing its clock history or budget.
    pub fn into_invocation_refusal(
        self,
    ) -> NativeOperationInvocationRefusal<'operation, 'registry> {
        self.retained.1
    }
}

/// An expiry diagnostic and exact successor awaiting joint checkpoint custody.
/// No handler or release-capable refusal survives successful preparation.
#[must_use = "commit the diagnostic and observation together before reporting durable expiry"]
pub struct PreparedNativeOperationExpiry<'operation> {
    operation: &'operation IssuedLocalOperation,
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    diagnostic: AdmittedDiagnostic,
    diagnostic_record: ControlRecord,
}

// These causes are derived from retained observations, never from caller input
// or a generic temporal refusal. Process duration proves only this call's
// observed timeout exhaustion; it is not reconstructed lease-expiry evidence.
enum ExpiryCause {
    HandleValidity,
    OperationLeaseValidity,
    PreReleaseTimeout,
}

impl<'operation, 'registry> NativeOperationInvocationRefusal<'operation, 'registry> {
    /// Record factual expiry while permanently discarding the original unopened call.
    ///
    /// Only an accepted retained wall observation crossing the original handle
    /// or operation-lease expiry, or the exact retained pre-release timeout
    /// failure, establishes eligibility. A supplied time floor, closed clock,
    /// revocation, authority error or backward clock is not expiry evidence.
    /// Signing authority is independently admitted for recording; this does
    /// not revalidate, renew or execute the expired session. No clock is sampled.
    ///
    /// Requires the original New dispatch confirmation and bounded current
    /// custody/anchor comparison. Preparation performs no writes. The eventual
    /// append still compares the exact predecessor and must advance its anchor.
    /// Sequential custody reads are not a cross-process lock. Failure returns
    /// the same refusal and unchanged time history; success retains no handler.
    ///
    /// # Errors
    /// Rejects absent expiry evidence, replay or uncertain original custody,
    /// changed heads/anchors, backward recording time, expired retention, aliased
    /// references, oversized diagnostics and failed observation admission/signing.
    pub fn prepare_expiry<Rotation, Sign>(
        self,
        input: NativeOperationExpiryInput<'_>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<
        PreparedNativeOperationExpiry<'operation>,
        NativeOperationExpiryRefusal<'operation, 'registry>,
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
        let prepared = (|| {
            let release = &self.retained.release;
            let dispatch = &release.dispatch;
            let attempt = &dispatch.attempt;
            let (handle_ref, handle) = attempt.context.operation.handle();
            let (floor, greatest) = release.time.coordinates();
            let cause = if matches!(self.retained.reason, OperationAttemptError::Deadline) {
                ExpiryCause::PreReleaseTimeout
            } else if greatest.is_some_and(|now| now >= attempt.context.lease.expires_at()) {
                ExpiryCause::OperationLeaseValidity
            } else if greatest.is_some_and(|now| now >= *handle.expires_at()) {
                ExpiryCause::HandleValidity
            } else {
                return Err(OperationAttemptError::Association);
            };
            if input.observed_at < floor
                || input.observed_at < *dispatch.observation.observed_at()
                || greatest.is_some_and(|now| input.observed_at < now)
            {
                return Err(OperationAttemptError::TimeOrder);
            }
            if input.observed_at > *handle.retain_until() {
                return Err(OperationAttemptError::RetentionElapsed);
            }
            if input.diagnostic_ref == input.observation_ref
                || input.diagnostic_ref == dispatch.observation_ref
                || input.observation_ref == dispatch.observation_ref
            {
                return Err(OperationAttemptError::Association);
            }
            attempt.validate_successor_reference(&input.diagnostic_ref)?;
            attempt.validate_successor_reference(&input.observation_ref)?;
            attempt.context.require_context_ready()?;
            dispatch
                .prepare_append(input.resolve_store)?
                .verify_fresh_dispatch_custody(
                    &release.completed,
                    custody,
                    anchor,
                    input.maximum_record_bytes,
                )?;
            let (action_id, request_digest) = attempt
                .selection
                .native_action_identity()
                .ok_or(OperationAttemptError::Association)?;
            let identity = pale_contracts::record_schema_identity("diagnostic.v1")
                .map_err(DiagnosticAdmissionError::Registry)?;
            let summary = match cause {
                ExpiryCause::HandleValidity => {
                    "operation handle expired before the retained native handler was released"
                }
                ExpiryCause::OperationLeaseValidity => {
                    "operation lease expired before the retained native handler was released"
                }
                ExpiryCause::PreReleaseTimeout => {
                    "operation timeout elapsed before the retained native handler was released"
                }
            };
            let diagnostic = DiagnosticProjection {
                code: "native_operation_expired_before_release".to_string(),
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
            let (digest, diagnostic_record) =
                record_projection::diagnostic_record(&input.diagnostic_ref, &diagnostic)
                    .map_err(OperationObservationCustodyProjectionError::from)?;
            let observation = signing::issue_observation(
                &OperationObservationAdmissionContextV1 {
                    signed: input.signing,
                    handle_ref,
                    handle,
                    predecessor: Some(dispatch.observation()),
                },
                OperationObservationStateV1::Expired {
                    diagnostic: ImmutableRecordReferenceV1 {
                        record_ref: input.diagnostic_ref,
                        record_sha256: digest,
                    },
                },
                input.observed_at,
                admit_rotation,
                sign,
            )?;
            Ok(PreparedNativeOperationExpiry {
                operation: attempt.context.operation,
                predecessor: (
                    dispatch.observation_ref.clone(),
                    dispatch.observation.clone(),
                ),
                observation_ref: input.observation_ref,
                observation,
                diagnostic,
                diagnostic_record,
            })
        })();
        prepared.map_err(|reason| NativeOperationExpiryRefusal {
            retained: Box::new((reason, self)),
        })
    }
}

impl PreparedNativeOperationExpiry<'_> {
    /// Borrow the exact diagnostic, not a custody confirmation.
    #[must_use]
    pub fn diagnostic(&self) -> &AdmittedDiagnostic {
        &self.diagnostic
    }

    /// Borrow the signed, nonconclusive expiry observation.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Prepare the diagnostic and observation for the existing atomic append.
    /// # Errors
    /// Preserves exact lineage, store-resolution and record-projection failures.
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
