//! Connect adapter custody to the production restart classifier and expiry turn.
//! Writer ownership and admitted deployment association are explicit fixtures.
//! Request, handle, head, checkpoint, response and expiry checks use their owners.

use super::super::{
    AdmittedResidentDeploymentAssociation, ReadyPrincipalResidentWriter, ReadyResidentDeployment,
    ResidentDeploymentAuthoritySelector, ResidentDeploymentOperationAccess,
    ResidentOperationControlAssociation, ResidentOperationHost,
    ResidentProcessExistingOperationStart,
};
use super::*;
use crate::resident::ResidentApplication;

#[path = "resident_hook_response_sweep.rs"]
mod cold_sweep;

/// Attach a fixture diagnostic through the production observation projection.
///
/// # Panics
/// Panics if the supplied fixture cannot form an admitted observation append.
pub fn append_with_diagnostic_fixture<'a>(
    transition: crate::resident::OperationObservationTransition<'a>,
    resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
    diagnostic: ControlRecord,
) -> crate::resident::PreparedOperationObservationAppend<'a> {
    crate::resident::prepare_operation_observation_append(transition, resolve_store)
        .expect("fixture observation append")
        .with_associated_record(pale_ports::OperationCustodyRecord::Diagnostic(diagnostic))
}

/// A fixture process around an application opened through its public owner.
pub struct RestartFixture<'a> {
    process: ResidentProcessHost<'a, ()>,
    selector: ResidentDeploymentAuthoritySelector,
}

/// Exact classification retained without any response release or task owner.
pub struct ClassifiedFixture(RetainedInvokePhase);

impl ClassifiedFixture {
    /// Borrow the saved bytes only when the actual classifier selected a response.
    pub fn response(&self) -> Option<&ControlRecord> {
        match &self.0 {
            RetainedInvokePhase::Deferred(value) => match value.as_ref() {
                restart::ResidentProcessOperationRestartDisposition::NativeHookResponse(value) => {
                    Some(&value.response.record)
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Ask the production retained-phase selector whether expiry may be attempted.
    pub fn expiry_eligible(&self) -> bool {
        self.0.native_hook_expiry_start().is_some()
    }

    /// Whether restart selected reconciliation without an executable continuation.
    pub fn recovery_only(&self) -> bool {
        matches!(&self.0, RetainedInvokePhase::Deferred(value)
            if matches!(value.as_ref(), restart::ResidentProcessOperationRestartDisposition::RecoveryOnly(_)))
    }
}

/// The service's exact physical continuation, with no executable response.
pub struct ExpiryFixture(PreDispatchExpiryTurn);

impl ExpiryFixture {
    /// Whether custody and independent anchoring completed.
    pub fn complete(&self) -> bool {
        matches!(self.0, PreDispatchExpiryTurn::Complete)
    }

    /// Whether the existing owner retained interrupted physical work.
    pub fn pending(&self) -> bool {
        matches!(self.0, PreDispatchExpiryTurn::Pending { .. })
    }

    /// Continue the same signed physical phase through the service's custody owner.
    pub fn continue_custody(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        inputs: ResidentServicePreDispatchExpiryContinuationInputs<'_>,
    ) -> Self {
        let PreDispatchExpiryTurn::Pending { observed_at, state } = self.0 else {
            return self;
        };
        Self(
            match crate::resident::operation_dispatch::continue_owned_operation_observation_custody(
                state,
                observed_at,
                inputs.checkpoint_schema,
                inputs.checkpoint_authority,
                inputs.resolve_store,
                inputs.accumulator_limits,
                custody,
                anchor,
                &mut *inputs.sign_checkpoint,
            ) {
                Ok(_) => PreDispatchExpiryTurn::Complete,
                Err(state) => PreDispatchExpiryTurn::Pending { observed_at, state },
            },
        )
    }
}

impl<'a> RestartFixture<'a> {
    /// Supply the fixture's admitted association and unproved writer ownership.
    pub fn new(
        application: Box<ResidentApplication<'a>>,
        signing: &SigningAuthorityBindingV1,
    ) -> Self {
        let runtime = signing
            .integrated_runtime_authority()
            .expect("fixture runtime");
        let association = AdmittedResidentDeploymentAssociation {
            principal_id: signing.principal_id().clone(),
            store: application.store().clone(),
            deployment_binding_ref: runtime.deployment_binding_ref.clone(),
            deployment_binding_sha256: DeploymentBindingDigest::parse(
                runtime.deployment_binding_sha256.as_str(),
            )
            .unwrap(),
            runtime_authority_ref: runtime.deployment_runtime_authority_ref.clone(),
            runtime_authority_sha256: DeploymentRuntimeAuthorityDigest::parse(
                runtime.deployment_runtime_authority_sha256.as_str(),
            )
            .unwrap(),
        };
        let selector = ResidentDeploymentAuthoritySelector::from_association(&association);
        let writer = ReadyPrincipalResidentWriter {
            principal_id: selector.principal_id.clone(),
            opened_deployment_stores: vec![application.store().clone()].into_boxed_slice(),
            held_writer: (),
        };
        let one = NonZeroUsize::new(1).unwrap();
        let tasks = ResidentOperationTaskState::new(
            ResidentOperationTaskLimits::new(one),
            one,
            ResidentAggregateTaskLimits::new(one),
        )
        .unwrap();
        let deployment = ReadyResidentDeployment::new(association, application).unwrap();
        Self {
            process: ResidentProcessHost::new(
                writer,
                vec![deployment],
                one,
                ResidentOperationHost::new(tasks),
            )
            .unwrap(),
            selector,
        }
    }

    /// Recover the actual stored start and run the full root restart classifier.
    ///
    /// # Errors
    /// Returns the production recovery or classification refusal as diagnostic text.
    pub fn classify(
        &self,
        request: &CommonRequestEnvelopeV1,
        context: LocalOperationRecoveryContext<'_>,
        custody: &mut dyn OperationCustodyPort,
        sessions: &dyn SessionLeaseCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        admission: OperationControlRecoveryAdmission<'_>,
    ) -> Result<ClassifiedFixture, String> {
        let evidence = crate::resident::operation_custody::read_operation_start_for_request(
            custody,
            context.store,
            request,
            context.maximum_record_bytes,
        )
        .map_err(|error| format!("{error:?}"))?
        .ok_or("fixture start absent")?;
        let association = ResidentOperationControlAssociation {
            current_request_authority: context.handle_signing_authority,
            historical_handle_authority: context.handle_signing_authority,
            recovery_store: context.store,
            checkpoint_authority: context.checkpoint_authority,
        };
        let input = restart::ResidentProcessOperationRestartInput {
            committed_observation_limits: None,
            operation: context,
            maximum_parent_depth: 0,
            current_head: restart::ResidentProcessRestartReadLimits {
                observation: context.observation_limits,
                checkpoints: context.checkpoint_limits,
                membership: context.membership_limits,
            },
            maximum_selection_bytes: context.maximum_record_bytes,
            a2a_task: None,
            a2a_cancel_task: None,
        };
        let result = self
            .process
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector: self.selector.clone(),
                    evidence: Box::new(evidence),
                },
                association,
                ResidentDeploymentOperationAccess {
                    store: context.store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .map(|value| ClassifiedFixture(RetainedInvokePhase::Deferred(Box::new(value))))
            .map_err(|error| format!("{error:?}"));
        assert_eq!(self.process.operations.outstanding_task_reservations(), 0);
        result
    }

    /// Run the service's actual expiry preparation and physical retention turn.
    ///
    /// # Errors
    /// Returns refusal before a physical continuation exists. No output is released.
    pub fn expire(
        &self,
        classified: &ClassifiedFixture,
        custody: &mut dyn OperationCustodyPort,
        sessions: &dyn SessionLeaseCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        inputs: ResidentServicePreDispatchExpiryPreparationInputs<'_>,
    ) -> Result<ExpiryFixture, String> {
        let start = classified
            .0
            .native_hook_expiry_start()
            .ok_or("not an expiry candidate")?;
        let store = self.process.deployments[0].store();
        let result = run_pre_dispatch_expiry_turn(
            &self.process,
            start,
            &mut ResidentDeploymentOperationAccess {
                store,
                custody,
                sessions,
                anchor,
            },
            inputs,
        )
        .map(ExpiryFixture)
        .map_err(|error| format!("{error:?}"));
        assert_eq!(self.process.operations.outstanding_task_reservations(), 0);
        result
    }
}
