//! Test-only recovery and diagnostic fault models for A2A custody.

use super::*;

pub(in crate::resident) struct A2aSendRecoveryFixture<'operation> {
    provider:
        super::super::super::provider::test_support::RetainedProviderRecoveryFixture<'operation>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

impl A2aSendRecoveryFixture<'_> {
    pub(in crate::resident) fn history(&self) -> OperationAttemptHistoryInput<'_> {
        self.provider.history()
    }

    pub(in crate::resident) fn recovered(&self) -> RecoveredOperationObservation {
        self.provider.recovered(self.checkpoint.clone())
    }
}

impl<'operation, 'call> A2aSendCustodyFailure<'operation, 'call> {
    pub(in crate::resident) fn recovery_fixture(
        &self,
    ) -> Option<A2aSendRecoveryFixture<'operation>> {
        if !self.is_recovery_only() {
            return None;
        }
        Some(A2aSendRecoveryFixture {
            provider: self.release.dispatch.recovery_fixture()?,
            checkpoint: self.release.checkpoint.clone(),
        })
    }
}

impl A2aSendOutcomeUnknown {
    /// Model an unavailable diagnostic contract after carrier uncertainty.
    pub(in crate::resident) fn diagnostic_failure_fixture(
        self: Box<Self>,
    ) -> Box<A2aSendDiagnosticFailure> {
        let Self {
            custody,
            context,
            response,
            diagnostic: _,
        } = *self;
        Box::new(A2aSendDiagnosticFailure {
            custody,
            context,
            response,
            kind: A2aSendDiagnosticFailureKind::OutcomeUnknown(
                "the A2A carrier outcome remains unknown",
            ),
            reason: crate::diagnostic::DiagnosticAdmissionError::UnknownSchemaCoordinate,
        })
    }

    /// Model retained evidence that cannot form a complete diagnostic.
    pub(in crate::resident) fn persistent_diagnostic_failure_fixture(
        self: Box<Self>,
    ) -> Box<A2aSendDiagnosticFailure> {
        let Self {
            custody,
            context,
            response: _,
            diagnostic: _,
        } = *self;
        Box::new(A2aSendDiagnosticFailure {
            custody,
            context,
            response: None,
            kind: A2aSendDiagnosticFailureKind::NoTask {
                facts: A2aInitialResponseFacts::DirectMessage,
                observed_at: UnixTimeV1::new(1).expect("positive fixture time"),
            },
            reason: crate::diagnostic::DiagnosticAdmissionError::UnknownSchemaCoordinate,
        })
    }
}

impl A2aSendDiagnosticFailure {
    /// Restore the original uncertainty after a modeled diagnostic fault.
    pub(in crate::resident) fn restore_outcome_unknown_fixture(self: Box<Self>) -> Box<Self> {
        let Self {
            custody,
            context,
            response,
            kind: _,
            reason: _,
        } = *self;
        Box::new(Self {
            custody,
            context,
            response,
            kind: A2aSendDiagnosticFailureKind::OutcomeUnknown(
                "the A2A carrier outcome remains unknown",
            ),
            reason: crate::diagnostic::DiagnosticAdmissionError::UnknownSchemaCoordinate,
        })
    }
}
