//! Shared artifacts rendered before execution-path context and activation.

mod discovery;
mod execution_paths;
mod lifecycle;

use std::collections::BTreeMap;

use crate::terminal_adapters::TerminalAdapterDescriptor;

use super::family_content::OrderedFamily;
use super::{GeneratedSurface, HarnessSurfaceRefs, RenderContext, RenderError, RenderIdentity};
/// Render repository-wide artifacts shared by every selected native harness.
pub(super) fn render_shared_artifacts(
    input: &RenderContext<'_, '_>,
    identity: &RenderIdentity<'_>,
    by_harness: &BTreeMap<TerminalAdapterDescriptor, Vec<OrderedFamily<'_>>>,
    harness_artifacts: &BTreeMap<TerminalAdapterDescriptor, HarnessSurfaceRefs>,
    read_only_paths: &[String],
    protected_runtime_dirs: &[String],
    surfaces: &mut Vec<GeneratedSurface>,
) -> Result<(), RenderError> {
    let projection = &input.projection;
    discovery::render_discovery_artifacts(
        input,
        identity,
        by_harness,
        harness_artifacts,
        surfaces,
    )?;
    let lifecycle_artifacts = lifecycle::render_lifecycle_artifacts(
        projection.identity.profile_record(),
        projection.experience_composition_plan,
        identity,
        projection.repository,
        projection.semantics,
        by_harness,
        surfaces,
    )?;
    execution_paths::render_execution_path_artifacts(
        input,
        identity,
        execution_paths::ExecutionPathArtifactSources {
            by_harness,
            harness_artifacts,
            read_only_paths,
            protected_runtime_dirs,
            lifecycle: &lifecycle_artifacts,
        },
        surfaces,
    )
}
