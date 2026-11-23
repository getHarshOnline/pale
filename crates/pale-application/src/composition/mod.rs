//! Combine a workspace's declared participants, execution paths and providers
//! into one checked composition plan.
//!
//! `compile`, `workspace` and `resident` build on it; it builds on no other module
//! in this crate. The compiler consumes only explicit, already-constituted
//! contracts. It does not infer authority, provider selection, activation,
//! invocation, native effect, settlement occurrence, or continuity state.

use pale_core::records::{
    CompositionGraphKindV1, EffectiveProjectProfileV1, ExecutionPathSelectionV1,
    ExperienceCompositionDeclarationsV1, ExperienceCompositionPlanPreimageV1,
    ExperienceCompositionPlanV1, EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_ID,
    EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_VERSION,
};

mod candidate;
mod graph;
pub(crate) mod normalize;
pub(crate) mod organization;
mod path;
mod provider;
pub(crate) mod responsibility;

#[cfg(test)]
#[path = "../../tests/internal/composition/responsibility_organization.rs"]
mod responsibility_organization;

pub(crate) use candidate::{reject_candidate_only_record_identifiers, CandidateIdentityRecordV1};
pub(crate) use graph::participant_id;
use graph::CompositionGraphs;
pub(crate) use organization::{lower_organizational_translation, OrganizationalTranslationError};
use path::{validate_edges_and_paths, ValidatedCompositionPaths};
use provider::validate_providers;
pub(crate) use responsibility::{
    lower_responsibility_derivations, reject_candidate_only_derivation_identifiers,
    reject_candidate_only_surfaces, ResponsibilityDerivationError,
};

// These ceilings reject accidental Cartesian composition before graph building;
// they are composition-compiler input bounds, not runtime capacity claims.
/// Maximum declarations retained across all neutral contract families.
/// Shared upper bound for one admitted semantic declaration graph.
pub(crate) const MAX_SEMANTIC_DECLARATIONS: usize = 1_024;
/// Maximum explicitly selected experience paths in one reference compilation.
const MAX_SELECTED_PATHS: usize = 256;

/// One closed provider-binding law violated by an authored declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProviderBindingViolation {
    /// The claimed identity does not bind the complete declaration body.
    #[error("claimed binding identity does not match the complete canonical binding body")]
    BindingIdentityMismatch,
    /// The deployment identity does not bind the selected physical deployment.
    #[error("deployment-activation identity does not match the selected physical deployment")]
    DeploymentIdentityMismatch,
    /// The declared schemas do not exactly govern the selected responsibility.
    #[error("responsibility schema identities do not exactly govern the typed responsibility")]
    ResponsibilitySchemaSetMismatch,
    /// The participant does not own a governing responsibility contract.
    #[error("participant does not own a governing responsibility contract")]
    ParticipantOwnershipMismatch,
    /// A governing contract does not contain the declared responsibility.
    #[error("typed responsibility is absent from a governing responsibility contract")]
    ResponsibilityNotGoverned,
}

/// Typed cause retained while admitting declarations or sealing a composition plan.
#[derive(Debug, thiserror::Error)]
pub enum CompositionError {
    /// A fixed embedded schema identity or generated record validation failed.
    #[error(transparent)]
    Schema(#[from] pale_contracts::SchemaError),
    /// A caller-declared schema reference failed exact registry admission.
    #[error("declared composition schema `{schema_id}` was rejected: {source}")]
    DeclaredSchema {
        /// Declared schema member; not a filesystem location.
        schema_id: String,
        /// Exact registry rejection, kept separate from fixed embedded lookups.
        #[source]
        source: pale_contracts::SchemaError,
    },
    /// A composition identity or plan body could not be represented canonically.
    #[error("cannot canonicalize experience composition: {0}")]
    Canonicalization(#[source] pale_core::canon::CanonError),
    /// One typed record identity occurs more than once.
    #[error("duplicate {kind} id `{id}`")]
    DuplicateId {
        /// Record family containing the collision.
        kind: &'static str,
        /// Repeated canonical identity.
        id: String,
    },
    /// One exact reference does not resolve.
    #[error("{kind} `{owner}` references missing {target_kind} `{target}`")]
    MissingReference {
        /// Referring record family.
        kind: &'static str,
        /// Referring record identity.
        owner: String,
        /// Expected target family.
        target_kind: &'static str,
        /// Missing target identity.
        target: String,
    },
    /// A provider binding is ambiguous or asymmetric.
    #[error("provider compatibility `{provider}` is invalid: {reason}")]
    ProviderCompatibility {
        /// Provider declaring the invalid relation.
        provider: String,
        /// Deterministic violated invariant.
        reason: String,
    },
    /// One selected provider binding contradicts its responsibility contract.
    #[error("provider binding `{binding}` is invalid: {violation}")]
    ProviderBinding {
        /// Provider-binding declaration that failed relational validation.
        binding: String,
        /// Closed law that rejected this declaration.
        violation: ProviderBindingViolation,
    },
    /// A directional delegation is missing a faithful translation or exceeds authority.
    #[error("delegation `{delegation}` is invalid: {reason}")]
    Delegation {
        /// Invalid directional delegation.
        delegation: String,
        /// Deterministic violated invariant.
        reason: String,
    },
    /// A non-delegation composition edge violates its intrinsic contract.
    #[error("composition edge `{edge}` is invalid: {reason}")]
    Edge {
        /// Invalid typed edge identity.
        edge: String,
        /// Deterministic violated invariant.
        reason: String,
    },
    /// A selected path has an invalid reference, connectivity, cycle, or budget relation.
    #[error("experience path `{path}` is invalid: {reason}")]
    Path {
        /// Invalid selected path.
        path: String,
        /// Deterministic violated invariant.
        reason: String,
    },
    /// The bounded reference compiler rejected a Cartesian declaration surface.
    #[error("composition declaration exceeds the bounded v1 limit for {kind}")]
    DeclarationLimit {
        /// Declaration family exceeding the fixed bound.
        kind: &'static str,
    },
    /// A workspace-selected execution path does not resolve in the neutral plan.
    #[error(
        "execution-path selection `{selection}` references unavailable experience path `{experience_path}`"
    )]
    PathSelection {
        /// Stable workspace-owned selection identity.
        selection: String,
        /// Neutral experience-path identity that failed exact resolution.
        experience_path: String,
    },
}

/// Loaded v1 composition records before reference resolution and graph checks.
pub(crate) type CompositionSourceV1 = ExperienceCompositionDeclarationsV1;

/// Preserve the trust boundary at which a composition cause arose.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CompositionPlanFailure {
    /// Supplied declarations could not be admitted or linked.
    #[error(transparent)]
    Declaration(CompositionError),
    /// Already-linked state could not produce a valid sealed output.
    #[error(transparent)]
    GeneratedOutput(CompositionError),
}

/// Bounded, deterministically normalized declarations ready for graph linking.
struct NormalizedComposition {
    source: CompositionSourceV1,
}

impl NormalizedComposition {
    fn admit(
        execution_paths: &[ExecutionPathSelectionV1],
        mut source: CompositionSourceV1,
    ) -> Result<Self, CompositionError> {
        normalize::normalize_source(&mut source)?;
        validate_execution_path_selections(execution_paths, &source)?;
        Ok(Self { source })
    }
}

/// Prove the contextual workspace selections against the neutral path set.
///
/// [`normalize::normalize_source`] has already rejected duplicate path IDs and
/// sorted the declarations. A successful binary search therefore proves that
/// each [`ExecutionPathSelectionV1::experience_path_id`] resolves exactly once.
/// The selection's terminal harness remains profile-owned and is deliberately
/// absent from the neutral composition plan.
fn validate_execution_path_selections(
    execution_paths: &[ExecutionPathSelectionV1],
    source: &CompositionSourceV1,
) -> Result<(), CompositionError> {
    for selection in execution_paths {
        let experience_path = selection.experience_path_id();
        if source
            .selected_experience_paths
            .binary_search_by(|path| path.id.as_str().cmp(experience_path))
            .is_err()
        {
            return Err(CompositionError::PathSelection {
                selection: selection.id().as_str().to_owned(),
                experience_path: experience_path.to_owned(),
            });
        }
    }
    Ok(())
}

/// Linked declarations with every reference, path, and graph invariant closed.
///
/// Selected-path explanations come only from [`ValidatedCompositionPaths`].
/// The linked phase has no fallback that can reconstruct proof-shaped output
/// from unproved source declarations.
struct LinkedComposition {
    normalized: NormalizedComposition,
    graphs: CompositionGraphs,
    path_proofs: ValidatedCompositionPaths,
}

impl LinkedComposition {
    fn link(normalized: NormalizedComposition) -> Result<Self, CompositionError> {
        let mut graphs = CompositionGraphs::default();
        let participants = validate_providers(
            &normalized.source.participant_provider_manifests,
            &mut graphs,
        )?;
        let path_proofs = validate_edges_and_paths(&normalized.source, &participants, &mut graphs)?;
        graphs.reject_operational_cycles()?;
        Ok(Self {
            normalized,
            graphs,
            path_proofs,
        })
    }

    fn seal(self) -> Result<ExperienceCompositionPlanV1, CompositionPlanFailure> {
        let Self {
            normalized,
            graphs,
            path_proofs,
        } = self;
        let NormalizedComposition { source } = normalized;
        let schema_identity =
            pale_contracts::record_schema_identity(EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_ID)
                .map_err(CompositionError::Schema)
                .map_err(CompositionPlanFailure::GeneratedOutput)?;
        let reason_paths = path_proofs.into_reason_paths();
        let plan = ExperienceCompositionPlanV1::seal(ExperienceCompositionPlanPreimageV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_version: EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_VERSION,
            declarations: source,
            provider_compatibility_graph: graphs
                .view(CompositionGraphKindV1::ProviderCompatibility),
            delegation_graph: graphs.view(CompositionGraphKindV1::WorkDelegation),
            ui_embedding_graph: graphs.view(CompositionGraphKindV1::UiEmbedding),
            plugin_dependency_graph: graphs.view(CompositionGraphKindV1::PluginDependency),
            native_application_graph: graphs.view(CompositionGraphKindV1::NativeApplicationCall),
            terminal_leaf_graph: graphs.view(CompositionGraphKindV1::TerminalLeaf),
            evidence_settlement_graph: graphs.view(CompositionGraphKindV1::ObservationSettlement),
            reason_paths,
        })
        .map_err(CompositionError::Canonicalization)
        .map_err(CompositionPlanFailure::GeneratedOutput)?;
        pale_contracts::validate_record(&plan, EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_ID)
            .map_err(CompositionError::Schema)
            .map_err(CompositionPlanFailure::GeneratedOutput)?;
        Ok(plan)
    }
}

/// Compile the current workspace's explicit neutral composition declarations.
///
/// The profile supplies contextual path selections, including the separately
/// selected terminal harness, while `source` supplies shape-admitted neutral
/// declarations whose identities and relations still require proof. This
/// compiler proves the exact selection-to-path join and the declared sparse
/// graphs; it neither infers owner availability nor proves
/// activation, release, or a native effect.
pub(crate) fn compile_current_composition_plan(
    profile: &EffectiveProjectProfileV1,
    source: CompositionSourceV1,
) -> Result<ExperienceCompositionPlanV1, CompositionPlanFailure> {
    compile_composition_plan(&profile.execution_paths, source)
}

fn compile_composition_plan(
    execution_paths: &[ExecutionPathSelectionV1],
    source: CompositionSourceV1,
) -> Result<ExperienceCompositionPlanV1, CompositionPlanFailure> {
    let normalized = NormalizedComposition::admit(execution_paths, source)
        .map_err(CompositionPlanFailure::Declaration)?;
    LinkedComposition::link(normalized)
        .map_err(CompositionPlanFailure::Declaration)?
        .seal()
}
#[cfg(test)]
#[path = "../../tests/internal/composition/reference_compiler.rs"]
mod tests;
