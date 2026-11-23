//! Shared loader for the CLI activation-workspace fixture.

#[path = "activation_workspace.rs"]
mod activation_workspace;
#[path = "distribution.rs"]
pub(super) mod synthetic_distribution;

pub(crate) use activation_workspace::{lay_out, MachineInputs};
