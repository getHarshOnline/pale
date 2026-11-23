//! Closed-set values grouped by the domain that owns their meaning.

mod capability;
mod harness;
mod lifecycle;
mod materialization;
mod mode;

pub use capability::{
    Approval, Availability, BindingState, CapabilityProvider, CapabilityUnavailability,
    EffectClass, Mechanics, OperationSupport, ProviderExecutionAvailability, Requirement,
    ToolObservability,
};
pub use harness::ChildPropagation;
pub use lifecycle::{FailurePosture, LifecycleAction, LifecycleEvent, PredicateKind, StateScope};
pub use materialization::{Outcome, PlanKind, SurfaceAction, SurfaceOwnership};
pub use mode::{ModeAxis, ModeDefaultSource, ModeScope, ReasoningEffort};
