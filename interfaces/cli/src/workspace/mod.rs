//! Local workspace inspection, plan publication, and materialization reporting.
//!
//! These commands select request order and render results. The shared
//! [`crate::command::CommandDispatcher`] calls the SDK for inspection, plan
//! publication, saved-plan admission, file changes, and reconciliation.
//! Inspection does not write targets; publishing a plan does write a plan file.

mod configuration_review;
mod inspection;
mod materialization;
mod plan_publication;
mod source_guidance;
mod summary;

pub(super) use configuration_review::cmd_review_configuration;
pub(super) use inspection::{
    cmd_compile, cmd_distribution_info, cmd_surface_status, cmd_validate, SurfaceStatusOutcome,
};
pub(super) use materialization::cmd_materialize;
pub(super) use plan_publication::cmd_plan;
pub(super) use source_guidance::cmd_guidance;
pub(super) use summary::cmd_summary;
