//! Admission of sealed run-mode and lifecycle bindings.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    LifecycleAction, LifecycleBehavior, LifecycleBehaviorIdV1, ModeAxis, PackageResourceRefV1,
};

use super::HookContext;

/// A sealed lifecycle catalog cannot use incoherent mode bindings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LifecyclePlanError {
    /// More than one selected mode uses the same identity.
    #[error("mode `{0}` is bound more than once")]
    DuplicateMode(PackageResourceRefV1),
    /// More than one default is declared for the same independent axis.
    #[error("mode axis `{0}` has more than one default")]
    DuplicateDefault(ModeAxis),
    /// A declared default is absent from the catalog or belongs to another axis.
    #[error("default mode `{mode}` is not selected on axis `{axis}")]
    InvalidDefault {
        /// Axis whose default is incoherent.
        axis: ModeAxis,
        /// Declared default mode identity.
        mode: PackageResourceRefV1,
    },
    /// A selected mode axis has no default to restore.
    #[error("mode axis `{0}` has no default")]
    MissingDefault(ModeAxis),
    /// Persisted session state names a mode absent from the sealed catalog.
    #[error("active mode `{0}` is absent from the sealed catalog")]
    UnknownActiveMode(PackageResourceRefV1),
    /// Persisted session state contains the same mode identity more than once.
    #[error("active mode `{0}` is selected more than once")]
    DuplicateActiveMode(PackageResourceRefV1),
    /// Persisted session state contains more than one mode on an independent axis.
    #[error("active modes `{first}` and `{second}` both occupy axis `{axis}")]
    MultipleActiveModes {
        /// Independent axis with conflicting active values.
        axis: ModeAxis,
        /// First active value observed on the axis.
        first: PackageResourceRefV1,
        /// Conflicting active value observed on the axis.
        second: PackageResourceRefV1,
    },
    /// A selected mode axis has no active value in the session snapshot.
    #[error("mode axis `{0}` has no active session value")]
    MissingActiveMode(ModeAxis),
    /// A lifecycle behavior is scoped to a mode absent from the sealed catalog.
    #[error("lifecycle behavior `{behavior}` references unknown mode `{mode}`")]
    BehaviorModeUnknown {
        /// Lifecycle behavior whose scope cannot be resolved.
        behavior: LifecycleBehaviorIdV1,
        /// Missing selected mode identity.
        mode: PackageResourceRefV1,
    },
    /// An activation behavior omits its target mode.
    #[error("activation behavior `{0}` has no target mode")]
    ActivationTargetMissing(LifecycleBehaviorIdV1),
    /// An activation behavior targets a mode absent from the sealed catalog.
    #[error("activation behavior `{behavior}` targets unknown mode `{mode}`")]
    ActivationTargetUnknown {
        /// Lifecycle behavior whose action cannot be resolved.
        behavior: LifecycleBehaviorIdV1,
        /// Missing selected mode identity.
        mode: PackageResourceRefV1,
    },
}

/// Validated mapping from selected run-mode identities to their independent axes.
pub(super) struct ModeBindings<'a> {
    modes_by_id: BTreeMap<&'a PackageResourceRefV1, ModeAxis>,
}

impl<'a> ModeBindings<'a> {
    /// Validate defaults, active selections, and every behavior mode reference together.
    pub(super) fn validate(
        behaviors: &[LifecycleBehavior],
        context: &HookContext<'a>,
    ) -> Result<Self, LifecyclePlanError> {
        let mut modes_by_id = BTreeMap::new();
        let mut selected_axes = BTreeSet::new();
        for (mode, axis) in context.mode_axes {
            if modes_by_id.insert(mode, *axis).is_some() {
                return Err(LifecyclePlanError::DuplicateMode(mode.clone()));
            }
            selected_axes.insert(*axis);
        }

        let mut defaults_by_axis = BTreeMap::new();
        for (axis, mode) in context.mode_defaults {
            if defaults_by_axis.insert(*axis, mode).is_some() {
                return Err(LifecyclePlanError::DuplicateDefault(*axis));
            }
            if modes_by_id.get(mode) != Some(axis) {
                return Err(LifecyclePlanError::InvalidDefault {
                    axis: *axis,
                    mode: mode.clone(),
                });
            }
        }
        for axis in &selected_axes {
            if !defaults_by_axis.contains_key(axis) {
                return Err(LifecyclePlanError::MissingDefault(*axis));
            }
        }

        let mut active_ids = BTreeSet::new();
        let mut active_by_axis = BTreeMap::new();
        for mode in context.active_modes {
            if !active_ids.insert(mode) {
                return Err(LifecyclePlanError::DuplicateActiveMode(mode.clone()));
            }
            let axis = modes_by_id
                .get(mode)
                .ok_or_else(|| LifecyclePlanError::UnknownActiveMode(mode.clone()))?;
            if let Some(first) = active_by_axis.insert(*axis, mode) {
                return Err(LifecyclePlanError::MultipleActiveModes {
                    axis: *axis,
                    first: first.clone(),
                    second: mode.clone(),
                });
            }
        }
        for axis in &selected_axes {
            if !active_by_axis.contains_key(axis) {
                return Err(LifecyclePlanError::MissingActiveMode(*axis));
            }
        }

        for behavior in behaviors {
            for mode in &behavior.modes {
                if !modes_by_id.contains_key(mode) {
                    return Err(LifecyclePlanError::BehaviorModeUnknown {
                        behavior: behavior.id.clone(),
                        mode: mode.clone(),
                    });
                }
            }
            if behavior.action == LifecycleAction::Activate {
                let mode = behavior.activate_mode.as_ref().ok_or_else(|| {
                    LifecyclePlanError::ActivationTargetMissing(behavior.id.clone())
                })?;
                if !modes_by_id.contains_key(mode) {
                    return Err(LifecyclePlanError::ActivationTargetUnknown {
                        behavior: behavior.id.clone(),
                        mode: mode.clone(),
                    });
                }
            }
        }
        Ok(Self { modes_by_id })
    }

    /// Return the axis of an already validated mode identity.
    pub(super) fn axis_for(&self, mode: &PackageResourceRefV1) -> Option<ModeAxis> {
        self.modes_by_id.get(mode).copied()
    }
}
