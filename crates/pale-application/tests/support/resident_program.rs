//! Test-only construction of an already-assembled resident program.

use super::*;

impl ResidentProgramDeploymentAuthority {
    pub(in crate::resident) fn fixture_from_current(
        context: &crate::resident::AmbientSessionAdmissionContext<'_>,
    ) -> Option<Self> {
        let current = context.current_principal_trust?;
        let runtime = context.runtime_authority?;
        Some(Self {
            selector: ResidentDeploymentAuthoritySelector {
                principal_id: runtime.authority.principal_id().clone(),
                deployment_binding_ref: context.deployment_binding_ref.clone(),
                deployment_binding_sha256: context
                    .deployment_binding
                    .canonical_binding_sha256()
                    .clone(),
                runtime_authority_ref: runtime.authority_ref.clone(),
                runtime_authority_sha256: runtime
                    .authority
                    .canonical_deployment_runtime_authority_sha256()
                    .clone(),
            },
            deployment_binding: context.deployment_binding.clone(),
            runtime_authority: runtime.authority.clone(),
            current_trust_ref: current.checkpoint_ref.clone(),
            current_trust: current.checkpoint.clone(),
        })
    }

    pub(in crate::resident) const fn fixture_selector(
        &self,
    ) -> &ResidentDeploymentAuthoritySelector {
        self.selector()
    }
}

impl<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
    ResidentProgram<'store, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
where
    WriterGuard: PrincipalWriterGuardPort,
    PolicyEvaluator:
        crate::resident::process_host::consult_recovery::ResidentConsultationPolicyEvaluator<
                StopInput,
            > + ResidentProcessInvokePolicyEvaluator,
    TurnOwner: ResidentProgramTurnOwner<WriterGuard, PolicyEvaluator, StopInput>,
{
    pub(in crate::resident) fn fixture_retain_generation(
        &mut self,
        deployment: usize,
        generation: crate::resident::AdmittedCompiledGeneration,
    ) {
        self.process.deployments[deployment]
            .application
            .fixture_retain_generation(generation);
    }

    pub(in crate::resident) fn fixture_from_process(
        process: ResidentProcessHost<'store, WriterGuard>,
        consult_policy: PolicyEvaluator,
        turn_owner: TurnOwner,
    ) -> Self {
        Self {
            process,
            deployment_authorities: Box::new([]),
            consult_policy,
            turn_owner,
            recovery_deployment: 0,
            recovery_source: ResidentProgramRecoverySource::RootProvider,
            stop_input: PhantomData,
        }
    }

    pub(in crate::resident) fn fixture_from_process_with_authorities(
        process: ResidentProcessHost<'store, WriterGuard>,
        deployment_authorities: Vec<ResidentProgramDeploymentAuthority>,
        consult_policy: PolicyEvaluator,
        turn_owner: TurnOwner,
    ) -> Self {
        Self {
            process,
            deployment_authorities: deployment_authorities.into_boxed_slice(),
            consult_policy,
            turn_owner,
            recovery_deployment: 0,
            recovery_source: ResidentProgramRecoverySource::RootProvider,
            stop_input: PhantomData,
        }
    }
}
