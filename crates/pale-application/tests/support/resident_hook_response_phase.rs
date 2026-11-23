//! Exercise the production response phase over an adapter's actual custody boundary.
//! This helper supplies no response permission, production key or host admission.

use super::operation_dispatch::hook_response::continuation::ResponsePhase;
use super::*;
use pale_core::records::*;
use pale_ports::{AntiRollbackAnchorPort, ControlRecord, OperationCustodyPort};

/// Opaque owner of the same signed phase retained by an ordinary native hook.
pub struct ResponsePhaseFixture(ResponsePhase);

impl ResponsePhaseFixture {
    /// Retain already-signed fixture records before any projection or physical I/O.
    pub fn new(
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
        observation: (ResidentRecordRefV1, OperationObservationV1),
        checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        identity: OperationCheckpointInput,
    ) -> Self {
        Self(ResponsePhase::new(
            predecessor,
            observation,
            checkpoint,
            identity,
        ))
    }

    /// Attempt the real physical phase once, preserving its owner on failure.
    /// A successful result establishes custody only, never response release.
    pub fn try_complete(
        &mut self,
        operation: &IssuedLocalOperation,
        response: Option<&ControlRecord>,
        inputs: &mut ResponsePhaseInputs<'_>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> bool {
        self.0
            .complete(
                operation,
                response,
                &mut super::operation_dispatch::hook_response::continuation::ResponsePhaseInputs {
                    checkpoint_schema: inputs.checkpoint_schema,
                    checkpoint_authority: inputs.checkpoint_authority,
                    resolve_store: inputs.resolve_store,
                    accumulator_limits: inputs.accumulator_limits,
                    sign_checkpoint: &mut *inputs.sign_checkpoint,
                },
                custody,
                anchor,
            )
            .is_ok()
    }

    /// Apply the production dispatch freshness gate to the completed fixture phase.
    ///
    /// # Errors
    /// Returns the custody owner's exact refusal for replay, uncertainty or changed heads.
    pub fn verify_fresh_dispatch(
        &self,
        operation: &IssuedLocalOperation,
        resolver: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<(), OperationDispatchCustodyError> {
        super::operation_dispatch::hook_response::continuation::phase_append(
            operation,
            &self.0.predecessor,
            &self.0.observation,
            None,
            resolver,
        )
        .expect("fixture append is valid")
        .verify_fresh_dispatch_custody(
            self.0
                .completed
                .as_ref()
                .expect("fixture custody is complete"),
            custody,
            anchor,
            65_536,
        )
    }

    /// Return the checkpoint retained before attachment or commitment.
    pub fn checkpoint(&self) -> Option<&ControlStateCheckpointV1> {
        self.0.checkpoint.as_ref()
    }

    /// Return exact completed custody, if the physical phase established it.
    pub fn completed(&self) -> Option<&CompletedOperationObservationCustody> {
        self.0.completed.as_ref()
    }
}

/// Encode a fixture response using the production record encoder.
pub fn response_record(
    reference: ResidentRecordRefV1,
    response: &pale_ports::ResidentHookResponseRecord,
) -> ControlRecord {
    let canonical = pale_core::canon::to_canonical_string(response).expect("fixture encodes");
    super::operation_dispatch::hook_response::response_record(reference, response, &canonical)
        .expect("fixture response record")
}

/// Fixture-selected physical inputs for the production response phase.
pub struct ResponsePhaseInputs<'a> {
    /// Exact checkpoint schema.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Fixture checkpoint authority.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Fixture authority-store resolver.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    /// Finite physical-work limits.
    pub accumulator_limits: pale_ports::AccumulatorPreparationLimits,
    /// Checkpoint signer whose calls can be counted by the fixture.
    pub sign_checkpoint: &'a mut HookResponseCheckpointSigner<'a>,
}

/// Reconstruct exact response history through its production owner.
///
/// # Errors
/// Returns the production association, custody or bound failure. This helper
/// exposes saved bytes for assertions and provides no releasable frame.
pub fn recover_response(
    operation: &RecoveredLocalOperation,
    observation: &RecoveredOperationObservation,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_record_bytes: usize,
) -> Result<ControlRecord, OperationAttemptError> {
    super::operation_dispatch::hook_response::recovery::recover_native_hook_response(
        operation.operation(),
        operation.request(),
        observation,
        custody,
        anchor,
        maximum_record_bytes,
        maximum_record_bytes,
    )
    .map(|recovered| recovered.record)
}
