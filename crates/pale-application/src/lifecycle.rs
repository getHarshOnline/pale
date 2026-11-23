//! Turn an admitted lifecycle behaviour into the hook-context record that
//! compilation and host rendering share, without the guidance text.
//!
//! `compile` and `resident` build on it; it builds on no other module in this
//! crate.

use pale_core::records::{AdmittedLifecycleBehaviorV1, LifecycleBehavior};

/// Lower admitted lifecycle mechanics to the hook-context record shape.
///
/// [`AdmittedLifecycleBehaviorV1::guidance`] contains materialized Markdown;
/// [`LifecycleBehavior::guidance`] is a source path. The two fields are not
/// interchangeable, so this projection deliberately emits `None` and renderers
/// carry the admitted guidance bytes through a separate sealed artifact.
pub(crate) fn mechanics(behaviour: &AdmittedLifecycleBehaviorV1) -> LifecycleBehavior {
    LifecycleBehavior {
        id: behaviour.id.clone(),
        description: behaviour.description.clone(),
        guidance: None,
        event: behaviour.event,
        predicate_kind: behaviour.predicate_kind,
        matchers: behaviour.matchers.clone(),
        case_insensitive: behaviour.case_insensitive,
        existing_targets_only: behaviour.existing_targets_only,
        allow_admitted_operation: behaviour.allow_admitted_operation,
        action: behaviour.action,
        activate_mode: behaviour.activate_mode.clone(),
        modes: behaviour.modes.clone(),
        tool_reqs: behaviour.tool_reqs.clone(),
        failure_posture: behaviour.failure_posture,
        state_scope: behaviour.state_scope,
        governed_by: behaviour.governed_by.clone(),
        timeout_ms: behaviour.timeout_ms,
        fallback: behaviour.fallback.clone(),
    }
}

/// Project one canonical lifecycle inventory exactly once per compilation.
pub(crate) fn mechanics_set(behaviours: &[AdmittedLifecycleBehaviorV1]) -> Vec<LifecycleBehavior> {
    behaviours.iter().map(mechanics).collect()
}
