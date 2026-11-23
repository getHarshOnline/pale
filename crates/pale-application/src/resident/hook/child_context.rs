//! Child context comes only from the recovered session's compiled policy and payload.

use super::{AdmittedResidentHookSession, ResidentApplication, ResidentHookPolicyAdmissionError};
use crate::hooks::{HookEvent, HookOutcome, RunPolicyFacts, RunPolicyPlan};

impl AdmittedResidentHookSession<'_> {
    /// Evaluate parent-context propagation for this admitted child-start event.
    /// When the policy declares child selectors, missing or nonmatching child
    /// identities receive no parent payload. When permitted, the payload comes
    /// from the retained generation; its digest was checked when that generation
    /// was admitted. No caller-supplied context is accepted.
    ///
    /// This evaluates the run policy only. It grants no child operation or native
    /// effect, emits no host response and does not apply lifecycle mode changes.
    ///
    /// # Errors
    /// Rejects an unselected event, changed session/route/context or an absent,
    /// ambiguous or altered injection. The authenticated carrier remains separate.
    pub fn evaluate_child_context(
        &self,
        application: &ResidentApplication<'_>,
    ) -> Result<HookOutcome, ResidentHookPolicyAdmissionError> {
        if self.request.routing.event() != HookEvent::SubagentStart {
            return Err(ResidentHookPolicyAdmissionError::WrongEvent);
        }
        let policy =
            self.request
                .select_hook_policy(application, &self.workspace_id, &self.session)?;
        let plan = RunPolicyPlan::evaluate(
            HookEvent::SubagentStart,
            &policy.hook_context.run_policy,
            RunPolicyFacts {
                child_type: self.request.event.agent_type(),
                tool_name: None,
                tool_command: None,
                exact_authorization: None,
            },
        );
        let injection = if plan.requires_injection() {
            policy
                .injection
                .ok_or(ResidentHookPolicyAdmissionError::InvalidInjection)?
        } else {
            ""
        };
        plan.render(injection)
            .map_err(|_| ResidentHookPolicyAdmissionError::InvalidInjection)
    }
}
