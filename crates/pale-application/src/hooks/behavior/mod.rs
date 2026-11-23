//! Lifecycle behavior planning, output shaping, and session-state transitions.
//!
//! One [`LifecyclePlan`] evaluates the catalog against a pre-commit state
//! snapshot. Payload demand, output, state transitions, and retry markers then
//! consume those same decisions.

mod context;
mod plan;

pub use context::{protected_write_requires_verification, HookContext};
pub use plan::{BoundLifecyclePayloads, LifecyclePayloadError, LifecyclePlan, LifecyclePlanError};
