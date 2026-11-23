//! Atomic validation and application of planned session transitions.

use pale_core::records::{ModeAxis, PackageResourceRefV1};

use super::LifecyclePlan;
use crate::session_state::{SessionRuntimeState, SessionStateError, StateEffect};

#[derive(Debug, Clone, Copy)]
/// Deferred session-state mutation emitted by lifecycle planning.
pub(super) enum PlannedStateTransition<'a> {
    Activate {
        axis: ModeAxis,
        mode: &'a PackageResourceRefV1,
    },
    RestoreDefaults,
    Capture(&'a str),
}

impl LifecyclePlan<'_> {
    /// Apply every planned state transition in catalog order.
    ///
    /// # Errors
    ///
    /// Rejects the complete transition set before mutation if a mode change
    /// would exceed the bounded session-state contract.
    pub fn apply_state_actions(
        &self,
        state: &mut SessionRuntimeState,
    ) -> Result<Vec<StateEffect>, SessionStateError> {
        if self.state_transitions.is_empty() && self.first_touch_paths.is_empty() {
            return Ok(Vec::new());
        }
        if self
            .state_transitions
            .iter()
            .any(|transition| !matches!(transition, PlannedStateTransition::Capture(_)))
        {
            let mut projected_modes = state.active_modes().clone();
            for transition in &self.state_transitions {
                match transition {
                    PlannedStateTransition::Activate { axis, mode } => {
                        projected_modes.insert(*axis, (*mode).clone());
                    }
                    PlannedStateTransition::RestoreDefaults => {
                        projected_modes = self.mode_defaults.iter().cloned().collect();
                    }
                    PlannedStateTransition::Capture(_) => {}
                }
            }
            SessionRuntimeState::validate_mode_selection(&projected_modes)?;
        }

        let mut effects = Vec::with_capacity(
            self.state_transitions
                .len()
                .saturating_add(self.first_touch_paths.len()),
        );
        for transition in &self.state_transitions {
            match transition {
                PlannedStateTransition::Activate { axis, mode } => {
                    effects.extend(state.activate(*axis, mode)?);
                }
                PlannedStateTransition::RestoreDefaults => {
                    effects.extend(state.restore_defaults(&self.mode_defaults)?);
                }
                PlannedStateTransition::Capture(description) => {
                    effects.extend(state.capture(description));
                }
            }
        }
        for path in &self.first_touch_paths {
            effects.extend(state.note_first_touch(path));
        }
        Ok(effects)
    }
}
