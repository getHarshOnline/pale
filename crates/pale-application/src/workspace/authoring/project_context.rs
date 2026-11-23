//! Source navigation for explicit project owners and repository duty references.

use super::*;

impl AuthoringSemanticIndex {
    pub(super) fn index_project_context(&mut self, linked: &LinkedWorkspace) {
        let label = linked.source.manifest_source.source_path.as_str();
        let topology = &linked.source.manifest.workspace.desired_topology;
        let topology_base = path([field("workspace"), field("desired_topology")]);
        for (index, project) in topology.projects.iter().enumerate() {
            if let Some(owner) = &project.owner {
                let base = nested(
                    &topology_base,
                    [field("projects"), at(index), field("owner")],
                );
                self.reference(
                    module_identity(owner.module_id().as_str()),
                    label,
                    child(&base, "module_id"),
                );
                self.reference_denoted(
                    DenotedAuthoringIdentity::resource(owner.module_id(), owner.resource_id()),
                    label,
                    child(&base, "resource_id"),
                );
            }
        }
        for (index, membership) in topology.project_repositories.iter().enumerate() {
            let base = nested(&topology_base, [field("project_repositories"), at(index)]);
            for (duty_index, duty) in membership.responsibility_derivation_ids.iter().enumerate() {
                self.reference(
                    simple_identity(
                        AuthoringIdentityKind::ResponsibilityDerivation,
                        duty.to_string(),
                    ),
                    label,
                    nested(
                        &base,
                        [field("responsibility_derivation_ids"), at(duty_index)],
                    ),
                );
            }
        }
    }
}
