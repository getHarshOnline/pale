//! Resolve project ownership and repository duties without granting authority.

use std::collections::BTreeMap;

use pale_core::records::{
    ResponsibilityDerivationIrV1, WorkspaceDesiredTopologyV1, WorkspaceModuleId,
    WorkspaceResourceSelectionV1,
};

use super::WorkspaceError;

pub(in crate::workspace) fn validate_project_context(
    topology: &WorkspaceDesiredTopologyV1,
    resources: &[(WorkspaceModuleId, WorkspaceResourceSelectionV1)],
    responsibilities: &ResponsibilityDerivationIrV1,
) -> Result<(), WorkspaceError> {
    let resources = resources
        .iter()
        .map(|(module, resource)| ((module, resource.resource_id()), resource.kind()))
        .collect::<BTreeMap<_, _>>();
    for project in &topology.projects {
        let Some(owner) = &project.owner else {
            continue;
        };
        match resources.get(&(owner.module_id(), owner.resource_id())) {
            Some(kind) if *kind == owner.kind() => {}
            Some(_) => {
                return Err(WorkspaceError::NamespaceResolution(format!(
                    "project `{}` owner `{}::{}` has a different resource kind",
                    project.id,
                    owner.module_id(),
                    owner.resource_id(),
                )));
            }
            None => {
                return Err(WorkspaceError::NamespaceResolution(format!(
                    "project `{}` owner `{}::{}` is absent from the selected module resources",
                    project.id,
                    owner.module_id(),
                    owner.resource_id(),
                )));
            }
        }
    }
    let duties = responsibilities
        .derivations
        .iter()
        .map(|duty| &duty.derivation_id)
        .collect::<std::collections::BTreeSet<_>>();
    for membership in &topology.project_repositories {
        for duty in &membership.responsibility_derivation_ids {
            if !duties.contains(duty) {
                return Err(WorkspaceError::NamespaceResolution(format!(
                    "project `{}` repository `{}` references absent responsibility `{duty}`",
                    membership.project_id, membership.repository_id,
                )));
            }
        }
    }
    Ok(())
}
