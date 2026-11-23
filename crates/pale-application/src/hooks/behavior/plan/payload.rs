//! Payload demand, admission, and output composition for an evaluated plan.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{LifecycleAction, LifecycleBehaviorIdV1};

use super::{Evaluation, LifecyclePlan};
use crate::hooks::HookOutcome;

/// Required lifecycle content was not supplied after adapter integrity checks.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LifecyclePayloadError {
    /// A fired action needs the sealed project injection.
    #[error("lifecycle plan requires the project injection payload")]
    InjectionMissing,
    /// A fired action needs the sealed active-mode context.
    #[error("lifecycle plan requires the active-mode payload")]
    ModeContextMissing,
    /// A fired guidance action has no corresponding verified body.
    #[error("lifecycle plan requires guidance for behavior `{0}`")]
    GuidanceMissing(LifecycleBehaviorIdV1),
}

/// Borrowed payloads admitted for, and inseparable from, one evaluated plan.
pub struct BoundLifecyclePayloads<'plan, 'catalog, 'payload> {
    plan: &'plan LifecyclePlan<'catalog>,
    injection: &'payload str,
    mode_context: &'payload str,
    guidance: &'payload BTreeMap<LifecycleBehaviorIdV1, String>,
}

impl std::fmt::Debug for BoundLifecyclePayloads<'_, '_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BoundLifecyclePayloads")
            .field("injection_present", &!self.injection.is_empty())
            .field("mode_context_present", &!self.mode_context.is_empty())
            .field("guidance_count", &self.guidance.len())
            .finish()
    }
}

fn stable_unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

impl<'a> LifecyclePlan<'a> {
    /// Whether this event needs the separately sealed base injection payload.
    pub fn requires_injection(&self) -> bool {
        self.entries.iter().any(|planned| {
            planned.evaluation == Evaluation::Fire
                && planned.behavior.action == LifecycleAction::Inject
        })
    }

    /// Whether this event needs the separately sealed active-mode payload.
    pub fn requires_mode_context(&self) -> bool {
        self.entries.iter().any(|planned| {
            planned.evaluation == Evaluation::Fire
                && planned.behavior.action == LifecycleAction::InjectModes
        })
    }

    /// Fired behaviors whose human-facing guidance body can affect output.
    pub fn guidance_ids(&self) -> impl Iterator<Item = &'a LifecycleBehaviorIdV1> + '_ {
        self.entries
            .iter()
            .filter(|planned| {
                planned.evaluation == Evaluation::Fire
                    && planned.behavior.guidance.is_some()
                    && matches!(
                        planned.behavior.action,
                        LifecycleAction::Deny
                            | LifecycleAction::Ask
                            | LifecycleAction::Guide
                            | LifecycleAction::Warn
                            | LifecycleAction::Notify
                            | LifecycleAction::Signal
                    )
            })
            .map(|planned| &planned.behavior.id)
    }

    /// Bind already integrity-checked payloads to this exact evaluation.
    ///
    /// # Errors
    ///
    /// Rejects an absent project injection, mode context, or guidance body that
    /// a fired action requires.
    pub fn bind_payloads<'plan, 'payload>(
        &'plan self,
        injection: &'payload str,
        mode_context: &'payload str,
        guidance: &'payload BTreeMap<LifecycleBehaviorIdV1, String>,
    ) -> Result<BoundLifecyclePayloads<'plan, 'a, 'payload>, LifecyclePayloadError> {
        if self.requires_injection() && injection.is_empty() {
            return Err(LifecyclePayloadError::InjectionMissing);
        }
        if self.requires_mode_context() && mode_context.is_empty() {
            return Err(LifecyclePayloadError::ModeContextMissing);
        }
        if let Some(id) = self.guidance_ids().find(|id| !guidance.contains_key(*id)) {
            return Err(LifecyclePayloadError::GuidanceMissing(id.clone()));
        }
        Ok(BoundLifecyclePayloads {
            plan: self,
            injection,
            mode_context,
            guidance,
        })
    }

    fn render_payloads(&self, payloads: &BoundLifecyclePayloads<'_, 'a, '_>) -> HookOutcome {
        let mut deny = Vec::new();
        let mut ask = Vec::new();
        let mut contexts = self
            .fallbacks
            .iter()
            .filter_map(|behavior| behavior.fallback_for(self.event).map(str::to_string))
            .collect::<Vec<_>>();
        for planned in &self.entries {
            let behavior = planned.behavior;
            match planned.evaluation {
                Evaluation::Skip => continue,
                Evaluation::DenyUnevaluable => {
                    deny.push(format!(
                        "Lifecycle boundary `{}` cannot be evaluated from the available host facts; denying.",
                        behavior.id
                    ));
                    continue;
                }
                Evaluation::Fire => {}
            }
            let body = payloads
                .guidance
                .get(&behavior.id)
                .map(String::as_str)
                .unwrap_or(&behavior.description);
            match behavior.action {
                LifecycleAction::Deny => deny.push(body.to_string()),
                LifecycleAction::Ask => ask.push(body.to_string()),
                LifecycleAction::Inject => contexts.push(payloads.injection.to_string()),
                LifecycleAction::InjectModes => contexts.push(payloads.mode_context.to_string()),
                LifecycleAction::Guide
                | LifecycleAction::Warn
                | LifecycleAction::Notify
                | LifecycleAction::Signal => contexts.push(body.to_string()),
                LifecycleAction::Activate
                | LifecycleAction::Deactivate
                | LifecycleAction::Capture
                | LifecycleAction::Rebaseline
                | LifecycleAction::NoOp => {}
            }
        }
        if !deny.is_empty() {
            return HookOutcome::Deny(stable_unique(deny).join("\n\n"));
        }
        if !ask.is_empty() {
            return HookOutcome::Ask(stable_unique(ask).join("\n\n"));
        }
        HookOutcome::Context(
            stable_unique(contexts.into_iter().filter(|body| !body.is_empty())).join("\n\n"),
        )
    }
}

impl BoundLifecyclePayloads<'_, '_, '_> {
    /// Resolve host-neutral output using only the plan that admitted these payloads.
    pub fn render(self) -> HookOutcome {
        self.plan.render_payloads(&self)
    }
}
