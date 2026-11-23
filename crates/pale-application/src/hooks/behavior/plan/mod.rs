//! One-pass lifecycle decisions reused by output and state stages.

use pale_core::records::{
    LifecycleAction, LifecycleBehavior, ModeAxis, PackageResourceRefV1, PredicateKind,
};

use super::context::{evaluate_behavior, Evaluation, HookContext};
use crate::hooks::HookEvent;
use crate::session_state::SessionRuntimeState;

mod bindings;
mod payload;
mod state;

pub use bindings::LifecyclePlanError;
pub use payload::{BoundLifecyclePayloads, LifecyclePayloadError};

use bindings::ModeBindings;
use state::PlannedStateTransition;

#[derive(Debug, Clone, Copy)]
struct PlannedBehavior<'a> {
    behavior: &'a LifecycleBehavior,
    evaluation: Evaluation,
}

/// One immutable evaluation of the lifecycle catalog for a hook invocation.
///
/// Payload demand, output, state transitions, and retry markers all consume
/// decisions made against the same pre-transition session snapshot.
#[must_use = "the evaluated lifecycle plan must drive output and state decisions"]
#[derive(Debug)]
pub struct LifecyclePlan<'a> {
    event: HookEvent,
    entries: Vec<PlannedBehavior<'a>>,
    fallbacks: Vec<&'a LifecycleBehavior>,
    state_transitions: Vec<PlannedStateTransition<'a>>,
    mode_defaults: Vec<(ModeAxis, PackageResourceRefV1)>,
    first_touch_paths: Vec<String>,
}

impl<'a> LifecyclePlan<'a> {
    /// Evaluate the catalog once against the supplied pre-commit state.
    ///
    /// Entries retain catalog order; skipped records are discarded so later
    /// stages cannot reevaluate predicates against changed state.
    ///
    /// # Errors
    ///
    /// Rejects incoherent sealed mode, default, active-state, behavior-scope,
    /// or activation bindings before output or state mutation.
    pub fn evaluate(
        event: HookEvent,
        behaviors: &'a [LifecycleBehavior],
        context: &HookContext,
        state: Option<&SessionRuntimeState>,
    ) -> Result<Self, LifecyclePlanError> {
        let mode_bindings = ModeBindings::validate(behaviors, context)?;
        let entries: Vec<_> = behaviors
            .iter()
            .filter_map(|behavior| {
                let evaluation = evaluate_behavior(behavior, event, context, state);
                (evaluation != Evaluation::Skip).then_some(PlannedBehavior {
                    behavior,
                    evaluation,
                })
            })
            .collect();
        let fallbacks = if event == HookEvent::SubagentStart && !context.child_context_admitted {
            Vec::new()
        } else {
            behaviors
                .iter()
                .filter(|behavior| {
                    !crate::hooks::harness_supports_event(context.adapter, behavior.event)
                        && behavior.fallback_for(event).is_some()
                })
                .collect()
        };

        let mut state_transitions = Vec::new();
        let mut record_touch = false;
        for planned in &entries {
            let behavior = planned.behavior;
            if planned.evaluation != Evaluation::Fire {
                continue;
            }
            if behavior.predicate_kind == PredicateKind::FirstTouch {
                record_touch = true;
            }
            match behavior.action {
                LifecycleAction::Activate => {
                    let mode = behavior.activate_mode.as_ref().ok_or_else(|| {
                        LifecyclePlanError::ActivationTargetMissing(behavior.id.clone())
                    })?;
                    let axis = mode_bindings.axis_for(mode).ok_or_else(|| {
                        LifecyclePlanError::ActivationTargetUnknown {
                            behavior: behavior.id.clone(),
                            mode: mode.clone(),
                        }
                    })?;
                    state_transitions.push(PlannedStateTransition::Activate { axis, mode });
                }
                LifecycleAction::Deactivate => {
                    state_transitions.push(PlannedStateTransition::RestoreDefaults);
                }
                LifecycleAction::Capture => {
                    state_transitions.push(PlannedStateTransition::Capture(&behavior.description));
                }
                LifecycleAction::Rebaseline
                | LifecycleAction::Inject
                | LifecycleAction::InjectModes
                | LifecycleAction::Deny
                | LifecycleAction::Ask
                | LifecycleAction::Warn
                | LifecycleAction::Guide
                | LifecycleAction::Notify
                | LifecycleAction::Signal
                | LifecycleAction::NoOp => {}
            }
        }

        Ok(Self {
            event,
            entries,
            fallbacks,
            state_transitions,
            mode_defaults: context.mode_defaults.to_vec(),
            first_touch_paths: if record_touch {
                context
                    .target_facts
                    .iter()
                    .map(|fact| fact.path.clone())
                    .collect()
            } else {
                Vec::new()
            },
        })
    }

    /// Whether this event arms a sealed refresh for the next context-bearing prompt.
    pub fn requests_rebaseline(&self) -> bool {
        self.entries.iter().any(|planned| {
            planned.evaluation == Evaluation::Fire
                && planned.behavior.action == LifecycleAction::Rebaseline
        })
    }
}
