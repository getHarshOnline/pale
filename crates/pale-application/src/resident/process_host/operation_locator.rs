//! Exact frame-locator binding for retained resident operations.
//!
//! This private adapter changes a transport locator into the existing protected
//! recovery inputs. It does not select a deployment, renew authority, classify
//! an operation, scan a store, or decide how an operation continues.

use pale_core::records::{
    InvalidDigest, InvalidResidentRecordV1, OperationHandleDigest, ResidentRecordRefV1,
};
use pale_ports::{
    ControlRecordCoordinate, ControlRecordDigest, ControlRecordError, ResidentOperationFrameLocator,
};

use super::restart::{
    ResidentProcessA2aCancelTaskRestartInput, ResidentProcessA2aTaskRestartInput,
    ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartInput,
    ResidentProcessRestartReadLimits,
};
use crate::resident::{
    LocalOperationHandleRecoveryInput, LocalOperationRecoveryContext, OperationControlObservation,
    OperationControlRecoveryInput, OperationObservationSelection, RecoveredOperationControl,
};

/// One frame locator after both fields pass their owning representation checks.
///
/// Construction establishes only an exact typed coordinate. Protected recovery
/// must still admit the stored request, handle, session, checkpoint, observation,
/// signatures, accumulator membership, and independent anchor.
pub(super) struct ResidentProgramOperationLocator {
    coordinate: ControlRecordCoordinate,
    handle_digest: OperationHandleDigest,
}

impl ResidentProgramOperationLocator {
    /// Retain the exact locator of one already issued application operation.
    ///
    /// This is a representation adapter for protected point recovery. It does
    /// not treat issuance as current liveness or skip the recovery admissions.
    pub(super) fn from_issued(
        operation: &crate::resident::IssuedLocalOperation,
    ) -> Result<Self, ResidentProgramOperationLocatorError> {
        let (record_ref, handle) = operation.handle();
        let handle_digest = handle.canonical_operation_handle_sha256().clone();
        let stored_digest = ControlRecordDigest::parse(handle_digest.as_str().to_owned())
            .map_err(ResidentProgramOperationLocatorError::StoredDigest)?;
        Ok(Self {
            coordinate: ControlRecordCoordinate::new(record_ref.clone(), stored_digest),
            handle_digest,
        })
    }

    /// Parse one untrusted frame locator without reading resident custody.
    pub(super) fn parse(
        locator: &ResidentOperationFrameLocator,
    ) -> Result<Self, ResidentProgramOperationLocatorError> {
        let record_ref =
            ResidentRecordRefV1::parse(locator.operation_handle_record_ref().to_owned())
                .map_err(ResidentProgramOperationLocatorError::RecordReference)?;
        let handle_digest =
            OperationHandleDigest::parse(locator.operation_handle_sha256().to_owned())
                .map_err(ResidentProgramOperationLocatorError::HandleDigest)?;
        let stored_digest = ControlRecordDigest::parse(handle_digest.as_str().to_owned())
            .map_err(ResidentProgramOperationLocatorError::StoredDigest)?;
        Ok(Self {
            coordinate: ControlRecordCoordinate::new(record_ref, stored_digest),
            handle_digest,
        })
    }

    /// Borrow the exact storage coordinate selected by the frame.
    pub(super) const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Bind this locator to the existing protected current-head recovery input.
    ///
    /// Observe and Cancel can place this value directly in their existing input.
    /// Advance can use the same value to establish exact locator proof before it
    /// enters one of the process host's existing protected classifiers.
    pub(super) fn recovery_input<'a>(
        &'a self,
        context: LocalOperationRecoveryContext<'a>,
        maximum_parent_depth: usize,
        limits: ResidentProcessRestartReadLimits,
    ) -> OperationControlRecoveryInput<'a> {
        OperationControlRecoveryInput {
            operation: LocalOperationHandleRecoveryInput {
                context,
                expected_handle: &self.coordinate,
                maximum_parent_depth,
            },
            observation: OperationObservationSelection::SelectedHead,
            observation_limits: limits.observation,
            checkpoint_limits: limits.checkpoints,
            membership_limits: limits.membership,
        }
    }

    /// Verify that protected recovery returned this exact handle.
    ///
    /// The protected recovery already proves the handle's canonical digest. This
    /// final comparison prevents a caller from recovering one handle and then
    /// advancing, cancelling, or disclosing another frame locator.
    pub(super) fn admit_recovered(
        &self,
        recovered: &RecoveredOperationControl,
    ) -> Result<(), ResidentProgramOperationLocatorError> {
        let (record_ref, handle) = recovered.operation().operation().handle();
        self.admit_handle(record_ref, handle.canonical_operation_handle_sha256())
    }

    /// Construct the existing restart input only after exact protected recovery.
    ///
    /// The returned input contains no locator because the existing restart owner
    /// derives its expected handle from protected start evidence. Requiring the
    /// recovered control here is the gate that prevents an unqualified prefix or
    /// deployment sweep from standing in for exact locator proof.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn restart_input<'a>(
        &self,
        recovered: &RecoveredOperationControl,
        operation: LocalOperationRecoveryContext<'a>,
        maximum_parent_depth: usize,
        current_head: ResidentProcessRestartReadLimits,
        maximum_selection_bytes: usize,
        a2a_task: Option<ResidentProcessA2aTaskRestartInput<'a>>,
        a2a_cancel_task: Option<ResidentProcessA2aCancelTaskRestartInput<'a>>,
    ) -> Result<ResidentProcessOperationRestartInput<'a>, ResidentProgramOperationLocatorError>
    {
        self.admit_recovered(recovered)?;
        if recovered.operation().store() != operation.store {
            return Err(ResidentProgramOperationLocatorError::Association);
        }
        Ok(ResidentProcessOperationRestartInput {
            committed_observation_limits: None,
            operation,
            maximum_parent_depth,
            current_head,
            maximum_selection_bytes,
            a2a_task,
            a2a_cancel_task,
        })
    }

    /// Verify a read-only Observe result before it crosses the Program boundary.
    pub(super) fn admit_observation(
        &self,
        observation: &OperationControlObservation,
    ) -> Result<(), ResidentProgramOperationLocatorError> {
        let (record_ref, handle) = observation.operation();
        self.admit_handle(record_ref, handle.canonical_operation_handle_sha256())
    }

    /// Verify that an existing restart classifier returned this exact operation.
    ///
    /// This does not reinterpret the classifier's phase. The closed disposition
    /// remains owned by the existing process-host restart implementation.
    pub(super) fn admit_restart_disposition(
        &self,
        disposition: &ResidentProcessOperationRestartDisposition,
    ) -> Result<(), ResidentProgramOperationLocatorError> {
        let operation = restart_disposition_operation(disposition);
        let (record_ref, handle) = operation.handle();
        self.admit_handle(record_ref, handle.canonical_operation_handle_sha256())
    }

    fn admit_handle(
        &self,
        record_ref: &ResidentRecordRefV1,
        digest: &OperationHandleDigest,
    ) -> Result<(), ResidentProgramOperationLocatorError> {
        if record_ref != self.coordinate.record_ref()
            || digest != &self.handle_digest
            || digest.as_str() != self.coordinate.record_sha256().as_str()
        {
            return Err(ResidentProgramOperationLocatorError::Association);
        }
        Ok(())
    }
}

/// Why a transport locator could not select one exact protected operation.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentProgramOperationLocatorError {
    /// The claimed immutable record reference is not a valid resident reference.
    #[error("operation frame locator has an invalid handle record reference")]
    RecordReference(#[source] InvalidResidentRecordV1),
    /// The claimed handle digest is not a lowercase SHA-256 value.
    #[error("operation frame locator has an invalid typed handle digest")]
    HandleDigest(#[source] InvalidDigest),
    /// The typed handle digest could not enter the physical custody coordinate.
    #[error("operation frame locator has an invalid stored handle digest")]
    StoredDigest(#[source] ControlRecordError),
    /// Recovery, observation, or restart returned a different operation.
    #[error("operation frame locator does not match the protected operation")]
    Association,
}

pub(super) fn restart_disposition_operation(
    disposition: &ResidentProcessOperationRestartDisposition,
) -> &crate::resident::IssuedLocalOperation {
    match disposition {
        ResidentProcessOperationRestartDisposition::NativeHookResponse(value) => {
            value.start.control.operation().operation()
        }
        ResidentProcessOperationRestartDisposition::Terminal(value)
        | ResidentProcessOperationRestartDisposition::RecoveryOnly(value)
        | ResidentProcessOperationRestartDisposition::Consult(value) => {
            value.control.operation().operation()
        }
        ResidentProcessOperationRestartDisposition::LocalCancellationRequested(value) => {
            value.start.control.operation().operation()
        }
        ResidentProcessOperationRestartDisposition::OperationResolution(value) => {
            value.start.control.operation().operation()
        }
        ResidentProcessOperationRestartDisposition::NativePrepared(value)
        | ResidentProcessOperationRestartDisposition::ProviderPrepared(value) => {
            value.start.control.operation().operation()
        }
        ResidentProcessOperationRestartDisposition::ProviderResolved(value) => {
            value.start.control.operation().operation()
        }
        ResidentProcessOperationRestartDisposition::Native(value)
        | ResidentProcessOperationRestartDisposition::Provider(value)
        | ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(value) => {
            value.start.control.operation().operation()
        }
        ResidentProcessOperationRestartDisposition::A2aTask(value) => {
            value.attempt.start.control.operation().operation()
        }
        ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(value) => {
            value.attempt.start.control.operation().operation()
        }
    }
}
