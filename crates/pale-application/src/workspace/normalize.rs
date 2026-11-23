//! Deterministic normalization of one linked workspace declaration graph.

use std::collections::BTreeMap;

use pale_core::records::{
    WorkspaceAuthorityScopeId, WorkspaceDeclarationV1, WorkspaceExclusiveAuthorityClaimV1,
    WorkspaceObligationId, WorkspacePolicyCeilingV1,
};
use serde::Serialize;

use super::link::{LinkedWorkspace, QualifiedDeclarationRef};
use super::WorkspaceError;

/// Effective policy accumulated for one normalized declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct EffectiveDeclarationPolicy {
    pub(super) ceiling: Option<WorkspacePolicyCeilingV1>,
    pub(super) obligations: Vec<WorkspaceObligationId>,
}

/// Deterministic desired-state graph before semantic sealing.
#[derive(Debug)]
pub(super) struct NormalizedWorkspace {
    pub(super) linked: LinkedWorkspace,
    pub(super) policies: BTreeMap<QualifiedDeclarationRef, EffectiveDeclarationPolicy>,
    pub(super) exclusive_authority:
        BTreeMap<WorkspaceAuthorityScopeId, WorkspaceExclusiveAuthorityClaimV1>,
}

fn accumulate_obligations(
    obligations: &mut Vec<WorkspaceObligationId>,
    additions: &[WorkspaceObligationId],
) -> Result<(), WorkspaceError> {
    *obligations = WorkspaceObligationId::try_union_sets(obligations, additions)?;
    Ok(())
}

/// Consume a linked graph and reduce all policy inputs to one canonical view.
pub(super) fn normalize_workspace(
    linked: LinkedWorkspace,
) -> Result<NormalizedWorkspace, WorkspaceError> {
    let mut policies = linked
        .declarations
        .iter()
        .map(|(id, declaration)| {
            let policy = match &declaration.declaration {
                WorkspaceDeclarationV1::Policy {
                    ceiling,
                    obligations,
                    ..
                } => EffectiveDeclarationPolicy {
                    ceiling: Some(ceiling.clone()),
                    obligations: obligations.clone(),
                },
                _ => EffectiveDeclarationPolicy {
                    ceiling: None,
                    obligations: Vec::new(),
                },
            };
            (id.clone(), policy)
        })
        .collect::<BTreeMap<_, _>>();
    let mut authority_claims = Vec::new();

    for declaration in linked.declarations.values() {
        if let WorkspaceDeclarationV1::Policy {
            exclusive_authority: claims,
            ..
        } = &declaration.declaration
        {
            authority_claims.extend(claims.iter().cloned());
        }
    }

    for extension in &linked.extensions {
        let effective = policies
            .get_mut(&extension.target)
            .ok_or_else(|| WorkspaceError::NamespaceResolution(extension.target.to_string()))?;
        if let Some(narrowing) = &extension.narrowed_ceiling {
            effective.ceiling = Some(match &effective.ceiling {
                Some(base) => base.meet(narrowing),
                None => narrowing.clone(),
            });
        }
        accumulate_obligations(&mut effective.obligations, &extension.added_obligations)?;
    }

    for surface in &linked.resolved_imports {
        for declaration_id in &surface.declarations {
            let effective = policies
                .get_mut(declaration_id)
                .ok_or_else(|| WorkspaceError::NamespaceResolution(declaration_id.to_string()))?;
            effective.ceiling = Some(match &effective.ceiling {
                Some(base) => base.meet(&surface.effective_ceiling),
                None => surface.effective_ceiling.clone(),
            });
            accumulate_obligations(&mut effective.obligations, &surface.obligations)?;
        }
        authority_claims.extend(surface.exclusive_authority.iter().cloned());
    }

    let exclusive_authority =
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&[], &authority_claims)?
            .into_iter()
            .map(|claim| (claim.scope.clone(), claim))
            .collect::<BTreeMap<WorkspaceAuthorityScopeId, WorkspaceExclusiveAuthorityClaimV1>>();

    Ok(NormalizedWorkspace {
        linked,
        policies,
        exclusive_authority,
    })
}
