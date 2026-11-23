//! Test-only recovery snapshots for retained provider dispatches.

use super::*;
use crate::resident::RecoveredOperationObservation;
use std::sync::Arc;

pub(in crate::resident) struct RetainedProviderRecoveryFixture<'operation> {
    operation: &'operation IssuedLocalOperation,
    request: &'operation CommonRequestEnvelopeV1,
    session: &'operation AdmittedAmbientSession,
    lease: &'operation OperationLeaseV1,
    generation: &'operation AdmittedCompiledGeneration,
    selection: ControlRecord,
    resolution: (ResidentRecordRefV1, OperationObservationV1),
    preparation: (ResidentRecordRefV1, OperationObservationV1),
    context_binding: Option<(ResidentRecordRefV1, OperationObservationV1)>,
    dispatched: (ResidentRecordRefV1, OperationObservationV1),
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    store: pale_ports::AuthorityStoreCoordinate,
}

impl RetainedProviderRecoveryFixture<'_> {
    pub(in crate::resident) fn selection(&self) -> ControlRecord {
        self.selection.clone()
    }

    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (ResidentRecordRefV1, ControlStateCheckpointV1) {
        self.checkpoint.clone()
    }

    pub(in crate::resident) fn prepared_recovered(
        &self,
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    ) -> RecoveredOperationObservation {
        let (handle_ref, handle) = self.operation.handle();
        RecoveredOperationObservation::history_fixture_for_handle(
            crate::resident::record_projection::coordinate(
                handle_ref,
                handle.canonical_operation_handle_sha256().as_str(),
            )
            .expect("provider preparation fixture handle"),
            self.preparation.clone(),
            Some(self.resolution.clone()),
            self.context_binding.clone(),
            self.preparation.clone(),
            checkpoint,
            self.store.clone(),
        )
    }

    pub(in crate::resident) fn history(&self) -> OperationAttemptHistoryInput<'_> {
        OperationAttemptHistoryInput {
            operation: self.operation,
            request: self.request,
            session: self.session.lease(),
            lease: self.lease,
            generation: self.generation,
            dispatched: &self.dispatched.1,
            selection: &self.selection,
            maximum_selection_bytes: 65_536,
        }
    }

    pub(in crate::resident) fn recovered(
        &self,
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    ) -> RecoveredOperationObservation {
        let (handle_ref, handle) = self.operation.handle();
        RecoveredOperationObservation::history_fixture_for_handle(
            crate::resident::record_projection::coordinate(
                handle_ref,
                handle.canonical_operation_handle_sha256().as_str(),
            )
            .expect("provider recovery fixture handle"),
            self.dispatched.clone(),
            Some(self.preparation.clone()),
            self.context_binding.clone(),
            self.dispatched.clone(),
            checkpoint,
            self.store.clone(),
        )
    }
}

impl<'operation> RetainedProviderOperationDispatch<'operation> {
    pub(in crate::resident) const fn request(&self) -> &'operation CommonRequestEnvelopeV1 {
        self.dispatch.attempt.context.request
    }

    pub(in crate::resident) fn recovery_fixture(
        &self,
    ) -> RetainedProviderRecoveryFixture<'operation> {
        let context = &self.dispatch.attempt.context;
        RetainedProviderRecoveryFixture {
            operation: context.operation,
            request: context.request,
            session: context.session,
            lease: context.lease,
            generation: context.generation,
            selection: self
                .dispatch
                .attempt
                .selection
                .record(
                    context.selection_ref.clone(),
                    context.maximum_selection_bytes,
                )
                .expect("retained provider selection remains encodable"),
            resolution: self.dispatch.attempt.preparation.predecessor.clone(),
            preparation: {
                let (reference, observation) = self.dispatch.attempt.observation();
                (reference.clone(), observation.clone())
            },
            context_binding: context.context_binding.map(|binding| {
                let (reference, observation) = binding.observation();
                (reference.clone(), observation.clone())
            }),
            dispatched: {
                let (reference, observation) = self.observation();
                (reference.clone(), observation.clone())
            },
            checkpoint: self.checkpoint.clone(),
            store: self.custody.candidate().store().clone(),
        }
    }

    pub(in crate::resident) fn into_recovered_release_fixture(
        self,
    ) -> RecoveredProviderDispatchRelease {
        let RetainedProviderOperationDispatch {
            dispatch,
            custody,
            checkpoint,
        } = self;
        let PreparedProviderOperationDispatch {
            attempt,
            observation_ref,
            observation,
        } = dispatch;
        let PreparedProviderOperationAttempt {
            context,
            preparation,
            selection,
            result_admission,
        } = attempt;
        let prepared = (
            preparation.observation().0.clone(),
            preparation.observation().1.clone(),
        );
        let selection_ref = preparation.selection.coordinate().record_ref().clone();
        let historical_request = context.request.clone();
        let recovered_attempt = RecoveredProviderAttempt::from_retained_fixture(
            selection,
            AttemptSelectionContext {
                operation: context.operation,
                request_ref: context.operation.request_coordinate().record_ref(),
                request: context.request,
                session: context.session.lease(),
                lease: context.lease,
                generation: context.generation,
            },
        )
        .expect("retained provider selection remains recoverable");
        let provider_request = OwnedProviderInvocationRequest::capture(
            recovered_attempt
                .provider_request()
                .expect("retained provider request remains valid"),
        );
        let recovered_dispatch = Arc::new(PreparedRecoveredOperationDispatch::new(
            context.operation,
            (&prepared.0, &prepared.1),
            selection_ref,
            context.request.expires_at(),
            observation_ref.clone(),
            observation.clone(),
            (&checkpoint.0, &checkpoint.1),
        ));
        let recovered_dispatch =
            recovery::provider_recovery_test_support::recovered_provider_dispatch(
                recovered_dispatch,
                historical_request,
                provider_request,
                result_admission.to_owned_selection(),
            );
        let recovered_custody = CompletedRecoveredDispatchCustody::from_retained_fixture(
            (observation_ref, observation),
            checkpoint,
            custody,
        );
        RecoveredProviderDispatchRelease::new(recovered_dispatch, recovered_custody)
            .unwrap_or_else(|_| panic!("retained provider dispatch remains an exact release"))
    }
}
