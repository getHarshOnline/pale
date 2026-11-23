//! Compiler observations for neutral execution paths and repository projections.
//!
//! [`ExecutionPathSupport`] retains compiler proof and activation selection.
//! Provider invocation readiness belongs to the operation-scoped, generation-bound
//! provider invocation vector rather than this portable path projection.

use pale_core::records::ExecutionPathWitnessV1;
use serde::Serialize;

use super::GeneratedSurface;
use crate::hooks::HookRequirements;

/// Compiler observation for one selected neutral execution path.
///
/// The nested [`ExecutionPathWitnessV1`] binds the selection to the exact
/// admitted composition path. This portable row makes no provider invocation,
/// delegated-owner, activation-effect, or whole-path readiness claim.
#[derive(Debug, Clone)]
pub struct ExecutionPathSupport {
    /// Compiler-owned proof of the selected path and its delegation edges.
    pub execution_path_witness: ExecutionPathWitnessV1,
    /// Whether workspace configuration selected this path for the repository.
    pub activation_selected: bool,
    /// Policy families represented by this projection.
    pub policy_ids: Vec<String>,
    /// Exact lifecycle wiring requirements resolved for this path.
    ///
    /// Rendering, validation, and outer summaries consume this retained value;
    /// they must not reconstruct requirements from generated surfaces.
    pub hook_requirements: HookRequirements,
    /// Human-readable qualification; never a substitute for typed fields.
    pub note: String,
}

/// Canonical repository-seal projection owned beside execution-path support.
#[derive(Serialize)]
pub(crate) struct ExecutionPathSupportSeal<'a> {
    /// Exact compiler witness retained by the sealed repository summary.
    execution_path_witness: &'a ExecutionPathWitnessV1,
    /// Whether workspace configuration selected this path for activation.
    activation_selected: bool,
    /// Stable policy identities represented by the path projection.
    policy_ids: Vec<&'a str>,
    /// Native lifecycle events required by the retained hook algebra.
    hook_events: Vec<&'static str>,
    /// Exact tool identities subject to pre-tool gating.
    gated_tools: Vec<&'a str>,
    /// Human-readable evidence ceiling paired with the typed coordinates.
    note: &'a str,
}

impl ExecutionPathSupport {
    /// Borrow the exact canonical fields used by the repository-render seal.
    pub(crate) fn seal_projection<'a>(
        &'a self,
        policy_ids: Vec<&'a str>,
    ) -> ExecutionPathSupportSeal<'a> {
        ExecutionPathSupportSeal {
            execution_path_witness: &self.execution_path_witness,
            activation_selected: self.activation_selected,
            policy_ids,
            hook_events: self
                .hook_requirements
                .events()
                .iter()
                .map(|event| event.as_str())
                .collect(),
            gated_tools: self
                .hook_requirements
                .gated_tools()
                .iter()
                .map(String::as_str)
                .collect(),
            note: &self.note,
        }
    }
}

/// Compiler-produced render of one repository for its selected paths.
///
/// Materialization still requires an identity-bound manifest and admitted plan;
/// mutating this observational output cannot bypass either boundary.
#[derive(Debug, Clone)]
pub struct ProjectRender {
    /// All generated files and deferred logical artifacts.
    pub surfaces: Vec<GeneratedSurface>,
    /// One independently qualified row per selected execution path.
    pub support: Vec<ExecutionPathSupport>,
}
