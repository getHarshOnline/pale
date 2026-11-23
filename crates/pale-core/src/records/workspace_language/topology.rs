//! Canonical desired project, execution-workspace, and repository topology.
//!
//! These records describe principal-owned desired meaning. Machine checkout
//! paths and provider-native coordinates are projections over this graph; they
//! cannot create, merge, or delete its identities or relationships.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    paths::OwnedConfinedRelativePath,
    records::{
        ExperienceCompositionPlanDigest, ExperienceCompositionPlanV1, PackageIdV1,
        PackageResourceRefV1, ProjectOwnerReferenceV1, ProviderBindingRefV1,
        ResponsibilityDerivationId, ResponsibilityPathIdentityV1, ResponsibilityV1, RunPolicyIdV1,
        SessionModel,
    },
};

use super::{InvalidWorkspaceLanguageId, PORTABLE_ID_MAX_LENGTH, PORTABLE_ID_PATTERN};

portable_id!(
    /// Stable desired project identity; never a directory, checkout, or provider container.
    LogicalProjectIdV1,
    "logical project id"
);
portable_id!(
    /// Stable desired execution-workspace identity; never a project or worktree.
    LogicalExecutionWorkspaceIdV1,
    "logical execution workspace id"
);
portable_id!(
    /// Stable desired repository identity, independent of every checkout and provider projection.
    LogicalRepositoryIdV1,
    "logical repository id"
);
portable_id!(
    /// Stable selection identity for one neutral experience path.
    ExecutionPathSelectionIdV1,
    "execution path selection id"
);

/// Maximum members on each canonical desired-topology axis.
pub const MAX_WORKSPACE_TOPOLOGY_MEMBERS: usize = 1_024;
/// Maximum explicit relationships in one desired topology.
pub const MAX_WORKSPACE_TOPOLOGY_RELATIONSHIPS: usize = 4_096;

/// One terminal execution selection over an existing neutral experience path.
///
/// The experience path owns directional delegation and provider references.
/// This record selects that path without reconstructing either graph or embedding
/// a product-shaped route matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPathSelectionV1 {
    /// Stable identity used by execution-workspace declarations.
    pub id: ExecutionPathSelectionIdV1,
    /// Existing [`crate::records::ExperiencePathV1`] identity.
    #[schemars(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$"))]
    pub experience_path_id: String,
}

impl ExecutionPathSelectionV1 {
    /// Borrow the stable selection identity.
    #[must_use]
    pub fn id(&self) -> &ExecutionPathSelectionIdV1 {
        &self.id
    }

    /// Borrow the neutral [`crate::records::ExperiencePathV1`] identity.
    #[must_use]
    pub fn experience_path_id(&self) -> &str {
        &self.experience_path_id
    }
}

/// Compiler projection proving which admitted composition path a selection names.
///
/// This nested record is not a source declaration or a top-level contract
/// family. Its fields remain claims until the enclosing artifact and the exact
/// [`ExperienceCompositionPlanV1`] are admitted. Keeping the ordered delegation
/// edge identities makes directness a structural law rather than a path-name or
/// product heuristic: a workspace is direct exactly when it declares no
/// delegation edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPathWitnessV1 {
    /// Desired execution-path selection projected by the compiler.
    pub selection: ExecutionPathSelectionV1,
    /// Domain-separated identity of the complete selected responsibility path.
    pub responsibility_path_identity: ResponsibilityPathIdentityV1,
    /// Ordered work-delegation edge identities from the admitted path.
    #[schemars(
        length(max = 256),
        inner(length(min = 1, max = 256), regex(pattern = r"^[A-Za-z0-9._/-]+$")),
        extend("uniqueItems" = true)
    )]
    pub work_delegation_edge_ids: Vec<String>,
    /// Exact terminal provider binding derived from the selected path.
    ///
    /// Native-only paths without a terminal leaf carry `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_provider_binding: Option<ProviderBindingRefV1>,
    /// Canonical identity of the composition plan that supplied the path.
    pub composition_plan_digest: ExperienceCompositionPlanDigest,
}

impl ExecutionPathWitnessV1 {
    /// Project one selected execution path from an identity-consistent compiled plan.
    ///
    /// The caller still owns semantic admission of `plan`; this constructor
    /// checks the plan self-digest, exact selected-path lookup, and the selected
    /// path's responsibility identity before copying the minimum proof closure.
    /// It grants no provider readiness, activation, native authority, or effect.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidExecutionPathWitnessV1`] when the plan or path identity
    /// is stale, the selection names no path, or the plan repeats that identity.
    pub fn from_compiled_plan(
        selection: &ExecutionPathSelectionV1,
        plan: &ExperienceCompositionPlanV1,
    ) -> Result<Self, InvalidExecutionPathWitnessV1> {
        if plan.recompute_digest()? != plan.plan_digest {
            return Err(InvalidExecutionPathWitnessV1::CompositionPlanDigestMismatch);
        }
        let mut paths = plan
            .declarations
            .selected_experience_paths
            .iter()
            .filter(|path| path.id == selection.experience_path_id);
        let path =
            paths
                .next()
                .ok_or_else(|| InvalidExecutionPathWitnessV1::UnknownExperiencePath {
                    path_id: selection.experience_path_id.clone(),
                })?;
        if paths.next().is_some() {
            return Err(InvalidExecutionPathWitnessV1::RepeatedExperiencePath {
                path_id: selection.experience_path_id.clone(),
            });
        }
        if path.recompute_identity()? != path.identity {
            return Err(
                InvalidExecutionPathWitnessV1::ResponsibilityPathIdentityMismatch {
                    path_id: selection.experience_path_id.clone(),
                },
            );
        }
        let mut termination_contracts = plan
            .declarations
            .termination_contracts
            .iter()
            .filter(|contract| contract.id == path.termination_contract_id);
        let termination_contract = termination_contracts.next().ok_or_else(|| {
            InvalidExecutionPathWitnessV1::UnknownTerminationContract {
                contract_id: path.termination_contract_id.clone(),
            }
        })?;
        if termination_contracts.next().is_some() {
            return Err(InvalidExecutionPathWitnessV1::RepeatedTerminationContract {
                contract_id: path.termination_contract_id.clone(),
            });
        }
        let terminal_provider_binding =
            if let Some(terminal_leaf) = &termination_contract.terminal_leaf {
                let mut bindings = plan
                    .declarations
                    .provider_bindings
                    .iter()
                    .filter(|binding| {
                        path.provider_binding_ids.iter().any(|id| id == &binding.id)
                            && binding.participant == *terminal_leaf
                            && binding.responsibility == ResponsibilityV1::TerminalHarnessExecution
                    });
                let binding = bindings.next().ok_or_else(|| {
                    InvalidExecutionPathWitnessV1::MissingTerminalProviderBinding {
                        path_id: path.id.clone(),
                    }
                })?;
                if bindings.next().is_some() {
                    return Err(
                        InvalidExecutionPathWitnessV1::RepeatedTerminalProviderBinding {
                            path_id: path.id.clone(),
                        },
                    );
                }
                if binding.recompute_identity()? != binding.binding_identity {
                    return Err(
                        InvalidExecutionPathWitnessV1::TerminalProviderBindingIdentityMismatch {
                            provider_binding_id: binding.id.clone(),
                        },
                    );
                }
                Some(ProviderBindingRefV1 {
                    provider_binding_id: binding.id.clone(),
                    provider_binding_identity: binding.binding_identity.clone(),
                })
            } else {
                None
            };
        Ok(Self {
            selection: selection.clone(),
            responsibility_path_identity: path.identity.clone(),
            work_delegation_edge_ids: path.work_delegation_edge_ids.clone(),
            terminal_provider_binding,
            composition_plan_digest: plan.plan_digest.clone(),
        })
    }

    /// Return whether the admitted path contains no work-delegation edge.
    ///
    /// This is the sole direct-path classifier. Path spelling, provider name,
    /// and edge count outside this witness cannot substitute for
    /// the exact empty-edge proof.
    #[must_use]
    pub fn is_direct(&self) -> bool {
        self.work_delegation_edge_ids.is_empty()
    }

    /// Borrow the exact terminal provider binding derived from the path.
    #[must_use]
    pub const fn terminal_provider_binding(&self) -> Option<&ProviderBindingRefV1> {
        self.terminal_provider_binding.as_ref()
    }
}

/// Failure to project a coherent execution-path witness from a compiled plan.
#[derive(Debug, thiserror::Error)]
pub enum InvalidExecutionPathWitnessV1 {
    /// The plan body does not reproduce its claimed canonical digest.
    #[error("composition plan digest does not match its complete body")]
    CompositionPlanDigestMismatch,
    /// The execution-path selection names no selected experience path.
    #[error("execution-path selection names unknown experience path `{path_id}`")]
    UnknownExperiencePath {
        /// Missing experience-path identity.
        path_id: String,
    },
    /// The plan repeats the selected experience-path identity.
    #[error("composition plan repeats experience path `{path_id}`")]
    RepeatedExperiencePath {
        /// Repeated experience-path identity.
        path_id: String,
    },
    /// The selected path names no declared termination contract.
    #[error("execution path names unknown termination contract `{contract_id}`")]
    UnknownTerminationContract {
        /// Missing termination-contract identity.
        contract_id: String,
    },
    /// The composition plan repeats the selected termination-contract identity.
    #[error("composition plan repeats termination contract `{contract_id}`")]
    RepeatedTerminationContract {
        /// Repeated termination-contract identity.
        contract_id: String,
    },
    /// A terminal path has no matching terminal-execution provider binding.
    #[error("experience path `{path_id}` has no terminal provider binding")]
    MissingTerminalProviderBinding {
        /// Experience-path identity missing its terminal binding.
        path_id: String,
    },
    /// A terminal path has more than one matching terminal-execution provider binding.
    #[error("experience path `{path_id}` has multiple terminal provider bindings")]
    RepeatedTerminalProviderBinding {
        /// Ambiguous experience-path identity.
        path_id: String,
    },
    /// The terminal provider binding carries a stale body identity.
    #[error("terminal provider binding `{provider_binding_id}` has a stale identity")]
    TerminalProviderBindingIdentityMismatch {
        /// Provider-binding identity whose body and digest disagree.
        provider_binding_id: String,
    },
    /// The selected path body does not reproduce its claimed responsibility identity.
    #[error("experience path `{path_id}` has a stale responsibility-path identity")]
    ResponsibilityPathIdentityMismatch {
        /// Experience-path identity whose body and digest disagree.
        path_id: String,
    },
    /// Canonicalization failed while an owning identity was recomputed.
    #[error(transparent)]
    Canonicalization(#[from] crate::canon::CanonError),
}

/// One canonical desired project declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogicalProjectV1 {
    /// Stable project identity.
    pub id: LogicalProjectIdV1,
    /// Human-readable title; never identity or authority.
    #[schemars(length(min = 1, max = 1_024))]
    pub title: String,
    /// Explicit owner reference; absence leaves ownership unspecified.
    /// Neither the project's directory nor its repository users supply this fact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<ProjectOwnerReferenceV1>,
    /// Private run-policy identity selected for this project.
    pub run_policy: RunPolicyIdV1,
    /// Package roots whose exact dependency closure is resolved from the distribution lock.
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub package_seeds: Vec<PackageIdV1>,
    /// Logical authority roots resolved only through the selected machine projection.
    #[schemars(length(max = 1_024), inner(length(min = 1, max = 255)), extend("uniqueItems" = true))]
    pub authority_root_ids: Vec<String>,
    /// Project capability source relative to the root workspace manifest.
    pub capabilities_file: OwnedConfinedRelativePath,
}

impl LogicalProjectV1 {
    /// Borrow the stable logical project identity.
    #[must_use]
    pub fn id(&self) -> &LogicalProjectIdV1 {
        &self.id
    }
}

/// One canonical desired execution-workspace declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LogicalExecutionWorkspaceV1 {
    /// Stable execution-workspace identity.
    pub id: LogicalExecutionWorkspaceIdV1,
    /// Human-readable title; never identity or authority.
    #[schemars(length(min = 1, max = 1_024))]
    pub title: String,
    /// Exact neutral path selections available in this bounded workspace.
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub execution_path_ids: Vec<ExecutionPathSelectionIdV1>,
    /// Optional session, authority-access, and lifecycle ceiling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionModel>,
}

impl LogicalExecutionWorkspaceV1 {
    /// Borrow the stable logical execution-workspace identity.
    #[must_use]
    pub fn id(&self) -> &LogicalExecutionWorkspaceIdV1 {
        &self.id
    }
}

/// One canonical desired repository declaration.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct LogicalRepositoryV1 {
    /// Stable repository identity.
    pub id: LogicalRepositoryIdV1,
    /// Human-readable title; never a checkout locator or native repository coordinate.
    #[schemars(length(min = 1, max = 1_024))]
    pub title: String,
}

impl LogicalRepositoryV1 {
    /// Borrow the stable logical repository identity.
    #[must_use]
    pub fn id(&self) -> &LogicalRepositoryIdV1 {
        &self.id
    }
}

/// Project-relative repository meaning and deterministic materialization order.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ProjectRepositoryMembershipV1 {
    /// Project containing the relationship.
    pub project_id: LogicalProjectIdV1,
    /// Canonically declared repository participating in the project.
    pub repository_id: LogicalRepositoryIdV1,
    /// Exact Package-owned semantic role of this repository in this project.
    pub role: PackageResourceRefV1,
    /// Explicit stack overlays selected for this membership.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub stacks: Vec<PackageResourceRefV1>,
    /// Owner-qualified policy families selected for this membership.
    #[serde(default)]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub families: Vec<PackageResourceRefV1>,
    /// Existing duty derivations relevant to this repository in this project.
    /// Their providers, consumers, acceptance and permissions remain independent
    /// of project ownership and repository access.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub responsibility_derivation_ids: Vec<ResponsibilityDerivationId>,
    /// Zero-based materialization order within the project.
    #[schemars(with = "u32")]
    pub materialization_order: u32,
}

/// Explicit many-to-many project to execution-workspace membership.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ProjectExecutionWorkspaceMembershipV1 {
    /// Project participating in the execution workspace.
    pub project_id: LogicalProjectIdV1,
    /// Execution workspace available to the project.
    pub execution_workspace_id: LogicalExecutionWorkspaceIdV1,
}

wire_enum!(
    /// Maximum repository access admitted by one execution-workspace membership.
    ExecutionWorkspaceRepositoryAccessV1 {
        /// Repository may be observed but not mutated through this membership.
        Read => "read",
        /// Repository mutation may be considered later under an independent effect admission.
        Mutate => "mutate"
    }
);

impl ExecutionWorkspaceRepositoryAccessV1 {
    /// Whether this membership may proceed to independent repository-effect admission.
    ///
    /// `true` is only a ceiling established by the desired graph. It does not
    /// prove a plan, authorization, native effect, or read-back. Conversely,
    /// [`Self::Read`] can never be widened by a machine checkout, operation
    /// binding, provider projection, or host-tool classification.
    #[must_use]
    pub const fn permits_repository_mutation(self) -> bool {
        matches!(self, Self::Mutate)
    }
}

/// Explicit many-to-many execution-workspace to repository membership.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ExecutionWorkspaceRepositoryMembershipV1 {
    /// Execution workspace containing the relationship.
    pub execution_workspace_id: LogicalExecutionWorkspaceIdV1,
    /// Canonically declared repository available in the workspace.
    pub repository_id: LogicalRepositoryIdV1,
    /// Maximum access this relationship permits before effect admission.
    pub access: ExecutionWorkspaceRepositoryAccessV1,
}

/// Complete canonical desired topology for one principal workspace.
///
/// All vectors are finite logical sets and therefore use canonical order. An
/// absent relationship is an explicit absence; [`Self::verify`] never infers an
/// edge from matching names, filesystem nesting, checkouts, or provider state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDesiredTopologyV1 {
    /// Canonical project declarations.
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub projects: Vec<LogicalProjectV1>,
    /// Canonical execution-workspace declarations.
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub execution_workspaces: Vec<LogicalExecutionWorkspaceV1>,
    /// Canonical repository declarations.
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub repositories: Vec<LogicalRepositoryV1>,
    /// Canonical neutral execution-path selections.
    #[schemars(length(max = 1_024), extend("uniqueItems" = true))]
    pub execution_paths: Vec<ExecutionPathSelectionV1>,
    /// Explicit project-to-repository edges.
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub project_repositories: Vec<ProjectRepositoryMembershipV1>,
    /// Explicit project-to-execution-workspace edges.
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub project_execution_workspaces: Vec<ProjectExecutionWorkspaceMembershipV1>,
    /// Explicit execution-workspace-to-repository edges.
    #[schemars(length(max = 4_096), extend("uniqueItems" = true))]
    pub execution_workspace_repositories: Vec<ExecutionWorkspaceRepositoryMembershipV1>,
}

impl WorkspaceDesiredTopologyV1 {
    /// Verify finite canonical sets, references, and project materialization order.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceDesiredTopologyV1`] when any axis is too large,
    /// is not canonically ordered, repeats an identity, references an absent
    /// declaration, or gives one project a non-contiguous repository order.
    pub fn verify(&self) -> Result<(), InvalidWorkspaceDesiredTopologyV1> {
        bounded(
            "project",
            self.projects.len(),
            MAX_WORKSPACE_TOPOLOGY_MEMBERS,
        )?;
        bounded(
            "execution workspace",
            self.execution_workspaces.len(),
            MAX_WORKSPACE_TOPOLOGY_MEMBERS,
        )?;
        bounded(
            "repository",
            self.repositories.len(),
            MAX_WORKSPACE_TOPOLOGY_MEMBERS,
        )?;
        bounded(
            "execution path",
            self.execution_paths.len(),
            MAX_WORKSPACE_TOPOLOGY_MEMBERS,
        )?;
        for (kind, observed) in [
            ("project repository", self.project_repositories.len()),
            (
                "project execution workspace",
                self.project_execution_workspaces.len(),
            ),
            (
                "execution workspace repository",
                self.execution_workspace_repositories.len(),
            ),
        ] {
            bounded(kind, observed, MAX_WORKSPACE_TOPOLOGY_RELATIONSHIPS)?;
        }

        canonical_unique_by("project", &self.projects, |value| value.id.clone())?;
        canonical_unique_by("execution workspace", &self.execution_workspaces, |value| {
            value.id.clone()
        })?;
        canonical_unique_by("repository", &self.repositories, |value| value.id.clone())?;
        canonical_unique_by("execution path", &self.execution_paths, |value| {
            value.id.clone()
        })?;
        canonical_unique_by("project repository", &self.project_repositories, |edge| {
            (edge.project_id.clone(), edge.repository_id.clone())
        })?;
        canonical_unique_by(
            "project execution workspace",
            &self.project_execution_workspaces,
            |edge| (edge.project_id.clone(), edge.execution_workspace_id.clone()),
        )?;
        canonical_unique_by(
            "execution workspace repository",
            &self.execution_workspace_repositories,
            |edge| {
                (
                    edge.execution_workspace_id.clone(),
                    edge.repository_id.clone(),
                )
            },
        )?;

        let projects = self
            .projects
            .iter()
            .map(|value| &value.id)
            .collect::<BTreeSet<_>>();
        let workspaces = self
            .execution_workspaces
            .iter()
            .map(|value| &value.id)
            .collect::<BTreeSet<_>>();
        let repositories = self
            .repositories
            .iter()
            .map(|value| &value.id)
            .collect::<BTreeSet<_>>();
        let paths = self
            .execution_paths
            .iter()
            .map(|value| &value.id)
            .collect::<BTreeSet<_>>();

        for workspace in &self.execution_workspaces {
            canonical_unique("execution-workspace path", &workspace.execution_path_ids)?;
            for path in &workspace.execution_path_ids {
                ensure_ref(paths.contains(path), "execution path", path.as_str())?;
            }
        }
        for project in &self.projects {
            canonical_unique("project package seed", &project.package_seeds)?;
            canonical_unique("project authority root", &project.authority_root_ids)?;
        }
        for edge in &self.project_repositories {
            canonical_unique("project repository stack", &edge.stacks)?;
            canonical_unique("project repository family", &edge.families)?;
            bounded(
                "project repository responsibility",
                edge.responsibility_derivation_ids.len(),
                MAX_WORKSPACE_TOPOLOGY_MEMBERS,
            )?;
            canonical_unique(
                "project repository responsibility",
                &edge.responsibility_derivation_ids,
            )?;
            ensure_ref(
                projects.contains(&edge.project_id),
                "project",
                edge.project_id.as_str(),
            )?;
            ensure_ref(
                repositories.contains(&edge.repository_id),
                "repository",
                edge.repository_id.as_str(),
            )?;
        }
        for edge in &self.project_execution_workspaces {
            ensure_ref(
                projects.contains(&edge.project_id),
                "project",
                edge.project_id.as_str(),
            )?;
            ensure_ref(
                workspaces.contains(&edge.execution_workspace_id),
                "execution workspace",
                edge.execution_workspace_id.as_str(),
            )?;
        }
        for edge in &self.execution_workspace_repositories {
            ensure_ref(
                workspaces.contains(&edge.execution_workspace_id),
                "execution workspace",
                edge.execution_workspace_id.as_str(),
            )?;
            ensure_ref(
                repositories.contains(&edge.repository_id),
                "repository",
                edge.repository_id.as_str(),
            )?;
        }

        let mut orders = BTreeMap::<&LogicalProjectIdV1, Vec<u32>>::new();
        for edge in &self.project_repositories {
            orders
                .entry(&edge.project_id)
                .or_default()
                .push(edge.materialization_order);
        }
        for (project, mut observed) in orders {
            observed.sort_unstable();
            let expected = (0..observed.len() as u32).collect::<Vec<_>>();
            if observed != expected {
                return Err(InvalidWorkspaceDesiredTopologyV1::MaterializationOrder {
                    project: project.to_string(),
                });
            }
        }
        Ok(())
    }
}

fn bounded(
    kind: &'static str,
    observed: usize,
    limit: usize,
) -> Result<(), InvalidWorkspaceDesiredTopologyV1> {
    if observed <= limit {
        Ok(())
    } else {
        Err(InvalidWorkspaceDesiredTopologyV1::Limit {
            kind,
            observed,
            limit,
        })
    }
}

fn canonical_unique<T: Ord>(
    kind: &'static str,
    values: &[T],
) -> Result<(), InvalidWorkspaceDesiredTopologyV1> {
    for pair in values.windows(2) {
        match pair[0].cmp(&pair[1]) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => {
                return Err(InvalidWorkspaceDesiredTopologyV1::Duplicate { kind })
            }
            std::cmp::Ordering::Greater => {
                return Err(InvalidWorkspaceDesiredTopologyV1::NonCanonical { kind })
            }
        }
    }
    Ok(())
}

fn canonical_unique_by<T, K: Ord>(
    kind: &'static str,
    values: &[T],
    key: impl Fn(&T) -> K,
) -> Result<(), InvalidWorkspaceDesiredTopologyV1> {
    for pair in values.windows(2) {
        match key(&pair[0]).cmp(&key(&pair[1])) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => {
                return Err(InvalidWorkspaceDesiredTopologyV1::Duplicate { kind })
            }
            std::cmp::Ordering::Greater => {
                return Err(InvalidWorkspaceDesiredTopologyV1::NonCanonical { kind })
            }
        }
    }
    Ok(())
}

fn ensure_ref(
    present: bool,
    kind: &'static str,
    identity: &str,
) -> Result<(), InvalidWorkspaceDesiredTopologyV1> {
    if present {
        Ok(())
    } else {
        Err(InvalidWorkspaceDesiredTopologyV1::UnknownReference {
            kind,
            identity: identity.to_owned(),
        })
    }
}

/// A principal-owned desired topology violates its finite graph contract.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidWorkspaceDesiredTopologyV1 {
    /// One topology axis exceeds its initial-v1 ceiling.
    #[error("workspace topology {kind} count {observed} exceeds {limit}")]
    Limit {
        /// Axis whose bound was exceeded.
        kind: &'static str,
        /// Observed member count.
        observed: usize,
        /// Maximum admitted count.
        limit: usize,
    },
    /// One logical set repeats a complete member.
    #[error("workspace topology repeats a {kind} member")]
    Duplicate {
        /// Repeated logical-set axis.
        kind: &'static str,
    },
    /// One logical set is not in canonical order.
    #[error("workspace topology {kind} members are not canonically ordered")]
    NonCanonical {
        /// Non-canonical logical-set axis.
        kind: &'static str,
    },
    /// One relationship or path selection names an absent declaration.
    #[error("workspace topology references unknown {kind} `{identity}`")]
    UnknownReference {
        /// Referenced identity kind.
        kind: &'static str,
        /// Missing identity.
        identity: String,
    },
    /// Project repository orders are not exactly `0..n`.
    #[error("project `{project}` repository materialization order is not contiguous from zero")]
    MaterializationOrder {
        /// Project with the invalid order set.
        project: String,
    },
}
