//! Independent run-mode axes and bounded transitions.

use std::collections::BTreeMap;

use pale_core::records::{ModeAxis, PackageResourceRefV1};

use super::{
    SessionRuntimeState, SessionStateError, StateEffect, MAX_ACTIVE_MODES, MAX_STATE_TEXT_BYTES,
};

fn within_bounds(modes: &BTreeMap<ModeAxis, PackageResourceRefV1>) -> bool {
    modes.len() <= MAX_ACTIVE_MODES
        && modes.values().all(|value| {
            value.package_id.as_str().len() + value.resource_id.as_str().len()
                <= MAX_STATE_TEXT_BYTES
        })
}

fn validated_entries(
    entries: &[(ModeAxis, PackageResourceRefV1)],
) -> Result<BTreeMap<ModeAxis, PackageResourceRefV1>, SessionStateError> {
    let mut modes = BTreeMap::new();
    for (axis, mode) in entries {
        if modes.insert(*axis, mode.clone()).is_some() {
            return Err(SessionStateError::ActiveModesBound);
        }
    }
    SessionRuntimeState::validate_mode_selection(&modes)?;
    Ok(modes)
}

impl SessionRuntimeState {
    /// Require one bounded, non-empty mode identity per independent axis.
    pub(crate) fn validate_mode_selection(
        modes: &BTreeMap<ModeAxis, PackageResourceRefV1>,
    ) -> Result<(), SessionStateError> {
        if within_bounds(modes) {
            Ok(())
        } else {
            Err(SessionStateError::ActiveModesBound)
        }
    }

    /// Read the active mode selected on each independent axis.
    pub fn active_modes(&self) -> &BTreeMap<ModeAxis, PackageResourceRefV1> {
        &self.active_modes
    }

    /// Fill absent axes from a bounded, duplicate-free default selection.
    /// Returns whether the state gained at least one default.
    ///
    /// # Errors
    ///
    /// Leaves state unchanged when a mode identity is empty, an axis is
    /// duplicated, or the merged state exceeds its bound.
    pub fn ensure_defaults(
        &mut self,
        defaults: &[(ModeAxis, PackageResourceRefV1)],
    ) -> Result<bool, SessionStateError> {
        let defaults = validated_entries(defaults)?;
        let mut next = self.active_modes.clone();
        for (axis, mode) in defaults {
            next.entry(axis).or_insert(mode);
        }
        Self::validate_mode_selection(&next)?;
        let changed = next != self.active_modes;
        self.active_modes = next;
        Ok(changed)
    }

    /// Activate one non-empty run-mode identity on its axis.
    /// Returns no effect when that exact mode is already active.
    ///
    /// # Errors
    ///
    /// Leaves state unchanged when the identity is empty or the resulting mode
    /// set exceeds its bound.
    pub fn activate(
        &mut self,
        axis: ModeAxis,
        mode: &PackageResourceRefV1,
    ) -> Result<Option<StateEffect>, SessionStateError> {
        let mut next = self.active_modes.clone();
        next.insert(axis, mode.clone());
        Self::validate_mode_selection(&next)?;
        if next == self.active_modes {
            return Ok(None);
        }
        self.active_modes = next;
        Ok(Some(StateEffect::Activated {
            axis,
            mode: mode.clone(),
        }))
    }

    /// Replace active overrides with a bounded, duplicate-free default selection.
    /// Returns no effect when the defaults already equal the active selection.
    ///
    /// # Errors
    ///
    /// Leaves state unchanged when an identity is empty, an axis is duplicated,
    /// or the supplied state exceeds its bound.
    pub fn restore_defaults(
        &mut self,
        defaults: &[(ModeAxis, PackageResourceRefV1)],
    ) -> Result<Option<StateEffect>, SessionStateError> {
        let next = validated_entries(defaults)?;
        if next == self.active_modes {
            return Ok(None);
        }
        self.active_modes = next;
        Ok(Some(StateEffect::DefaultsRestored))
    }
}
