//! Generated-surface manifests and materialization-plan records.
//!
//! The `seal` constructors compute canonical self-digests; they do not validate
//! the semantic provenance of their inputs, observe a filesystem, admit a plan,
//! or perform an effect. On decoded values, every digest and observation remains
//! a claim until the owning application or adapter recomputes and verifies it.

use serde::{Deserialize, Serialize};

use crate::canon;

use super::super::{
    validate_release_closures, ArtifactDigest, CompilationInputDigest, ExecutionPathSelectionIdV1,
    InstanceDigest, LogicalExecutionWorkspaceIdV1, LogicalProjectIdV1, LogicalRepositoryIdV1,
    ManifestDigest, MaterializationLineageDigest, MaterializationStateDigest,
    NormativePackReleaseV1, OneShotCompilationDigest, PackageReleaseV1, PackageSemanticIrDigest,
    PlanDigest, PlanKind, ProfileDigest, ReleaseClosureErrorV1, SchemaId, SchemaManifestDigest,
    SchemaSetDigest, SurfaceAction, SurfaceOwnership, TargetRootDigest,
};

/// Stable wire identity carried by [`AgentSurfacePlanV1::schema`].
pub const AGENT_SURFACE_PLAN_V1_SCHEMA: &str = "pale.agent-surface-plan.v1";
/// Stable wire identity carried by [`MaterializationPlanV1::schema`].
pub const MATERIALIZATION_PLAN_V1_SCHEMA: &str = "pale.materialization-plan.v1";
/// Wire major of [`MaterializationPlanV1`].
pub const MATERIALIZATION_PLAN_V1_SCHEMA_VERSION: u32 = 1;

/// Repository-relative path used for [`MaterializationStateV1`].
pub const MATERIALIZATION_STATE_PATH: &str = ".pale/materialization-state.json";
/// Stable wire identity carried by [`MaterializationStateV1::schema`].
pub const MATERIALIZATION_STATE_SCHEMA: &str = "pale.agent-surface.materialization-state.v1";
/// Wire major carried by [`MaterializationStateV1::schema_version`].
pub const MATERIALIZATION_STATE_SCHEMA_VERSION: u32 = 1;
/// Stable wire identity carried by [`MaterializationLineageV1::schema`].
pub const MATERIALIZATION_LINEAGE_SCHEMA: &str = "pale.agent-surface.materialization-lineage.v1";
/// Wire major carried by [`MaterializationLineageV1::schema_version`].
pub const MATERIALIZATION_LINEAGE_SCHEMA_VERSION: u32 = 1;

/// Claimed content and ownership identity of one rendered surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceDigest {
    /// Confined repository-relative target or declared logical artifact path.
    pub rel_path: String,
    /// Renderer-defined semantic kind used for reconciliation and inspection.
    pub kind: String,
    /// Whether Pale owns a delimited block or the complete target file.
    pub ownership: SurfaceOwnership,
    /// Exact neutral execution paths that consume this rendered surface.
    pub execution_path_consumers: Vec<ExecutionPathSelectionIdV1>,
    /// Whether the portable artifact may enter a separately authorized commit.
    pub committable: bool,
    /// SHA-256 identity of the rendered bytes or managed block.
    pub sha256: ArtifactDigest,
}

/// Digest-bearing claim about the surface set last materialized into one target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterializationStateV1 {
    /// Stable wire identity for the target-local ownership record.
    #[schemars(with = "String", extend("const" = MATERIALIZATION_STATE_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Materialization-state schema major.
    #[schemars(extend("const" = MATERIALIZATION_STATE_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Descriptor-manifest identity claimed for the owning compilation.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity claimed for the owning compilation.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Logical repository whose generated surfaces are recorded.
    pub repository_id: LogicalRepositoryIdV1,
    /// Portable profile identity that produced the surfaces.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity used for the materialization.
    pub instance_digest: InstanceDigest,
    /// Claimed generated manifest from which this state was applied.
    pub manifest_digest: ManifestDigest,
    /// Canonically ordered complete set of Pale-owned target surfaces.
    pub surfaces: Vec<SurfaceDigest>,
    /// Claimed canonical identity of the complete ownership-state body.
    pub state_digest: MaterializationStateDigest,
}

/// Unsealed construction input for [`MaterializationStateV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaterializationStatePreimageV1 {
    /// Stable wire identity for the target-local ownership record.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Materialization-state schema major.
    pub schema_version: u32,
    /// Descriptor-manifest identity supplied for the owning compilation.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity supplied for the owning compilation.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Logical repository whose generated surfaces are recorded.
    pub repository_id: LogicalRepositoryIdV1,
    /// Portable profile identity that produced the surfaces.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity used for the materialization.
    pub instance_digest: InstanceDigest,
    /// Generated-manifest identity supplied for this state.
    pub manifest_digest: ManifestDigest,
    /// Canonically ordered complete set of Pale-owned target surfaces.
    pub surfaces: Vec<SurfaceDigest>,
}

impl MaterializationStateV1 {
    /// Compute and attach the canonical identity of an ownership-state body.
    ///
    /// This hashes the supplied claims; it does not observe or admit target state.
    ///
    /// # Errors
    ///
    /// Returns [`canon::CanonError`] when the preimage cannot satisfy the
    /// canonical JSON contract.
    pub fn seal(preimage: MaterializationStatePreimageV1) -> Result<Self, canon::CanonError> {
        let state_digest = MaterializationStateDigest::hash_canonical(&preimage)?;
        let MaterializationStatePreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            repository_id,
            profile_digest,
            instance_digest,
            manifest_digest,
            surfaces,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            repository_id,
            profile_digest,
            instance_digest,
            manifest_digest,
            surfaces,
            state_digest,
        })
    }

    /// Clone the unhashed state body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> MaterializationStatePreimageV1 {
        MaterializationStatePreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            schema_manifest_sha256: self.schema_manifest_sha256.clone(),
            schema_set_sha256: self.schema_set_sha256.clone(),
            one_shot_compilation_digest: self.one_shot_compilation_digest.clone(),
            repository_id: self.repository_id.clone(),
            profile_digest: self.profile_digest.clone(),
            instance_digest: self.instance_digest.clone(),
            manifest_digest: self.manifest_digest.clone(),
            surfaces: self.surfaces.clone(),
        }
    }

    /// Recompute the canonical identity of this state's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`canon::CanonError`] when the body cannot satisfy the canonical
    /// JSON contract.
    pub fn recompute_digest(&self) -> Result<MaterializationStateDigest, canon::CanonError> {
        MaterializationStateDigest::hash_canonical(&self.to_preimage())
    }
}

/// Digest-bearing machine-local lineage claim for a materialization transition.
///
/// Trust requires independent verification of the admitted plan, state and
/// lineage self-digests, and observed publication. [`Self::seal`] alone proves
/// only the canonical identity of its supplied preimage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterializationLineageV1 {
    /// Stable wire identity for the machine-local lineage anchor.
    #[schemars(with = "String", extend("const" = MATERIALIZATION_LINEAGE_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Materialization-lineage schema major.
    #[schemars(extend("const" = MATERIALIZATION_LINEAGE_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Descriptor-manifest identity copied from the admitted plan/state.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity copied from the admitted plan/state.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Logical repository paired with the target-local state.
    pub repository_id: LogicalRepositoryIdV1,
    /// Digest of the canonical target-root path bytes.
    pub target_root_sha256: TargetRootDigest,
    /// Portable profile identity admitted by the plan.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity admitted by the plan.
    pub instance_digest: InstanceDigest,
    /// Generated manifest identity admitted by the plan.
    pub manifest_digest: ManifestDigest,
    /// Self-digest of the independently stored target-local ownership record.
    pub state_digest: MaterializationStateDigest,
    /// Claimed admitted-plan identity bound to this transition.
    pub admitted_plan_digest: PlanDigest,
    /// Claimed canonical identity of the complete lineage body.
    pub lineage_digest: MaterializationLineageDigest,
}

/// Unsealed construction input for [`MaterializationLineageV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaterializationLineagePreimageV1 {
    /// Stable wire identity for the machine-local lineage anchor.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Materialization-lineage schema major.
    pub schema_version: u32,
    /// Descriptor-manifest identity copied from the admitted plan/state.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity copied from the admitted plan/state.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Logical repository paired with the target-local state.
    pub repository_id: LogicalRepositoryIdV1,
    /// Digest of the canonical target-root path bytes.
    pub target_root_sha256: TargetRootDigest,
    /// Portable profile identity admitted by the plan.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity admitted by the plan.
    pub instance_digest: InstanceDigest,
    /// Generated manifest identity admitted by the plan.
    pub manifest_digest: ManifestDigest,
    /// Self-digest of the independently stored target-local ownership record.
    pub state_digest: MaterializationStateDigest,
    /// Exact admitted-plan identity bound to this transition.
    pub admitted_plan_digest: PlanDigest,
}

impl MaterializationLineageV1 {
    /// Compute and attach the canonical identity of a lineage body.
    ///
    /// # Errors
    ///
    /// Returns [`canon::CanonError`] when the preimage cannot satisfy the
    /// canonical JSON contract.
    pub fn seal(preimage: MaterializationLineagePreimageV1) -> Result<Self, canon::CanonError> {
        let lineage_digest = MaterializationLineageDigest::hash_canonical(&preimage)?;
        let MaterializationLineagePreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            repository_id,
            target_root_sha256,
            profile_digest,
            instance_digest,
            manifest_digest,
            state_digest,
            admitted_plan_digest,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            repository_id,
            target_root_sha256,
            profile_digest,
            instance_digest,
            manifest_digest,
            state_digest,
            admitted_plan_digest,
            lineage_digest,
        })
    }

    /// Clone the unhashed lineage body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> MaterializationLineagePreimageV1 {
        MaterializationLineagePreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            schema_manifest_sha256: self.schema_manifest_sha256.clone(),
            schema_set_sha256: self.schema_set_sha256.clone(),
            one_shot_compilation_digest: self.one_shot_compilation_digest.clone(),
            repository_id: self.repository_id.clone(),
            target_root_sha256: self.target_root_sha256.clone(),
            profile_digest: self.profile_digest.clone(),
            instance_digest: self.instance_digest.clone(),
            manifest_digest: self.manifest_digest.clone(),
            state_digest: self.state_digest.clone(),
            admitted_plan_digest: self.admitted_plan_digest.clone(),
        }
    }

    /// Recompute the canonical identity of this lineage's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`canon::CanonError`] when the body cannot satisfy the canonical
    /// JSON contract.
    pub fn recompute_digest(&self) -> Result<MaterializationLineageDigest, canon::CanonError> {
        MaterializationLineageDigest::hash_canonical(&self.to_preimage())
    }
}

/// Digest-bearing projection of what a compiler claims it rendered and for whom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeneratedAgentSurfaceManifestV1 {
    /// Stable wire identity of the generated-surface manifest.
    #[schemars(
        with = "String",
        extend("const" = "pale.agent-surface.generated-manifest.v1")
    )]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Descriptor-manifest identity selected before rendering.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity selected before rendering.
    pub schema_set_sha256: SchemaSetDigest,
    /// Pre-render compilation identity. The post-render one-shot digest is
    /// deliberately absent because this manifest is one of its inputs.
    pub compilation_input_digest: CompilationInputDigest,
    /// Compiler release that resolved the profile and surface set.
    pub compiler_version: String,
    /// Renderer release that produced the exact surface bytes.
    pub renderer_version: String,
    /// Logical project from the portable profile.
    pub project_id: LogicalProjectIdV1,
    /// Logical execution workspace from the portable profile.
    pub execution_workspace_id: LogicalExecutionWorkspaceIdV1,
    /// Logical repository for this projection.
    pub repository_id: LogicalRepositoryIdV1,
    /// Portable profile identity carried by every output.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity carried by every output.
    pub instance_digest: InstanceDigest,
    /// Complete canonically ordered output digest inventory.
    pub outputs: Vec<SurfaceDigest>,
    /// Claimed canonical identity of the complete generated-manifest body.
    pub manifest_digest: ManifestDigest,
}

/// Unsealed construction input for [`GeneratedAgentSurfaceManifestV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GeneratedAgentSurfaceManifestPreimageV1 {
    /// Stable wire identity of the generated-surface manifest.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Descriptor-manifest identity selected before rendering.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity selected before rendering.
    pub schema_set_sha256: SchemaSetDigest,
    /// Pre-render compilation identity.
    pub compilation_input_digest: CompilationInputDigest,
    /// Compiler release that resolved the profile and surface set.
    pub compiler_version: String,
    /// Renderer release that produced the exact surface bytes.
    pub renderer_version: String,
    /// Logical project from the portable profile.
    pub project_id: LogicalProjectIdV1,
    /// Logical execution workspace from the portable profile.
    pub execution_workspace_id: LogicalExecutionWorkspaceIdV1,
    /// Logical repository for this projection.
    pub repository_id: LogicalRepositoryIdV1,
    /// Portable profile identity carried by every output.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity carried by every output.
    pub instance_digest: InstanceDigest,
    /// Complete canonically ordered output digest inventory.
    pub outputs: Vec<SurfaceDigest>,
}

impl GeneratedAgentSurfaceManifestV1 {
    /// Compute and attach the canonical identity of a generated-manifest body.
    ///
    /// # Errors
    ///
    /// Returns [`canon::CanonError`] when the preimage cannot satisfy the
    /// canonical JSON contract.
    pub fn seal(
        preimage: GeneratedAgentSurfaceManifestPreimageV1,
    ) -> Result<Self, canon::CanonError> {
        let manifest_digest = ManifestDigest::hash_canonical(&preimage)?;
        let GeneratedAgentSurfaceManifestPreimageV1 {
            schema,
            schema_sha256,
            schema_manifest_sha256,
            schema_set_sha256,
            compilation_input_digest,
            compiler_version,
            renderer_version,
            project_id,
            execution_workspace_id,
            repository_id,
            profile_digest,
            instance_digest,
            outputs,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_manifest_sha256,
            schema_set_sha256,
            compilation_input_digest,
            compiler_version,
            renderer_version,
            project_id,
            execution_workspace_id,
            repository_id,
            profile_digest,
            instance_digest,
            outputs,
            manifest_digest,
        })
    }

    /// Clone the unhashed manifest body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> GeneratedAgentSurfaceManifestPreimageV1 {
        GeneratedAgentSurfaceManifestPreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_manifest_sha256: self.schema_manifest_sha256.clone(),
            schema_set_sha256: self.schema_set_sha256.clone(),
            compilation_input_digest: self.compilation_input_digest.clone(),
            compiler_version: self.compiler_version.clone(),
            renderer_version: self.renderer_version.clone(),
            project_id: self.project_id.clone(),
            execution_workspace_id: self.execution_workspace_id.clone(),
            repository_id: self.repository_id.clone(),
            profile_digest: self.profile_digest.clone(),
            instance_digest: self.instance_digest.clone(),
            outputs: self.outputs.clone(),
        }
    }

    /// Recompute the canonical identity of this manifest's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`canon::CanonError`] when the body cannot satisfy the canonical
    /// JSON contract.
    pub fn recompute_digest(&self) -> Result<ManifestDigest, canon::CanonError> {
        ManifestDigest::hash_canonical(&self.to_preimage())
    }
}

/// Validated Unix permission bits carried by materialization contracts.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(try_from = "u32", into = "u32")]
#[schemars(extend("maximum" = 4_095))]
pub struct UnixFileMode(u32);

impl UnixFileMode {
    /// Largest Unix permission value accepted by materialization contracts.
    pub const MAX: u32 = 0o7777;
    /// Default permissions for generated, non-secret repository content.
    pub const DEFAULT_FILE: Self = Self(0o644);

    /// Construct a mode when no bits outside `0o7777` are set.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidUnixFileMode`] when `value` contains unsupported bits.
    pub const fn new(value: u32) -> Result<Self, InvalidUnixFileMode> {
        if value <= Self::MAX {
            Ok(Self(value))
        } else {
            Err(InvalidUnixFileMode { value })
        }
    }

    /// Return the validated Unix permission bits.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for UnixFileMode {
    type Error = InvalidUnixFileMode;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<UnixFileMode> for u32 {
    fn from(value: UnixFileMode) -> Self {
        value.get()
    }
}

/// A numeric Unix mode containing bits outside the materialization ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Unix file mode {value:#o} exceeds 0o7777")]
pub struct InvalidUnixFileMode {
    value: u32,
}

/// Wire representation of a baseline observation for one repository surface.
///
/// An effect adapter creates this from confined filesystem observation; a
/// decoded value is only a claim until that observation is repeated or admitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum BaselineStateV1 {
    /// No regular file existed at baseline capture time.
    Absent {},
    /// A regular file existed with these exact bytes and permissions.
    Present {
        /// SHA-256 of the observed bytes.
        sha256: ArtifactDigest,
        /// Observed Unix permission bits.
        mode: UnixFileMode,
    },
}

/// Pre-mutation state of one target-owned path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaselineEntry {
    /// Confined repository-relative target inspected before planning.
    pub rel_path: String,
    /// Claimed observed presence state.
    pub observation: BaselineStateV1,
}

/// Exact ownership transition planned for one repository surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum SurfaceOperationTransitionV1 {
    /// Create an absent generated surface.
    Create {
        /// Filesystem region Pale will own.
        ownership: SurfaceOwnership,
        /// SHA-256 of the bytes or managed block to materialize.
        desired_sha256: ArtifactDigest,
    },
    /// Explicitly adopt a pre-existing generated surface.
    Adopt {
        /// Filesystem region Pale will own.
        ownership: SurfaceOwnership,
        /// SHA-256 of the bytes or managed block to materialize.
        desired_sha256: ArtifactDigest,
    },
    /// Replace an existing owned surface.
    Update {
        /// Filesystem region Pale owns.
        ownership: SurfaceOwnership,
        /// SHA-256 of the replacement bytes or managed block.
        desired_sha256: ArtifactDigest,
    },
    /// Verify an existing owned surface without changing its content.
    Unchanged {
        /// Filesystem region Pale owns.
        ownership: SurfaceOwnership,
        /// SHA-256 of the owned bytes or managed block.
        owned_sha256: ArtifactDigest,
    },
    /// Delete a previously owned whole file.
    Delete {
        /// SHA-256 of the prior owned file.
        prior_owned_sha256: ArtifactDigest,
    },
    /// Remove a previously owned managed block from a user-owned file.
    Splice {
        /// SHA-256 of the prior owned block.
        prior_owned_sha256: ArtifactDigest,
    },
}

impl SurfaceOperationTransitionV1 {
    /// Project the operation action without storing a second wire discriminator.
    #[must_use]
    pub const fn action(&self) -> SurfaceAction {
        match self {
            Self::Create { .. } => SurfaceAction::Create,
            Self::Adopt { .. } => SurfaceAction::Adopt,
            Self::Update { .. } => SurfaceAction::Update,
            Self::Unchanged { .. } => SurfaceAction::Unchanged,
            Self::Delete { .. } => SurfaceAction::Delete,
            Self::Splice { .. } => SurfaceAction::Splice,
        }
    }

    /// Project the filesystem ownership class encoded by this transition.
    #[must_use]
    pub const fn ownership(&self) -> SurfaceOwnership {
        match self {
            Self::Create { ownership, .. }
            | Self::Adopt { ownership, .. }
            | Self::Update { ownership, .. }
            | Self::Unchanged { ownership, .. } => *ownership,
            Self::Delete { .. } => SurfaceOwnership::WholeFile,
            Self::Splice { .. } => SurfaceOwnership::ManagedBlock,
        }
    }

    /// Return the desired or prior-owned content digest used by admission.
    #[must_use]
    pub const fn content_sha256(&self) -> &ArtifactDigest {
        match self {
            Self::Create { desired_sha256, .. }
            | Self::Adopt { desired_sha256, .. }
            | Self::Update { desired_sha256, .. } => desired_sha256,
            Self::Unchanged { owned_sha256, .. } => owned_sha256,
            Self::Delete { prior_owned_sha256 } | Self::Splice { prior_owned_sha256 } => {
                prior_owned_sha256
            }
        }
    }
}

/// One planned materialization effect and its expected content identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    /// Confined repository-relative target of this materialization effect.
    pub rel_path: String,
    /// Renderer-defined semantic kind of the target.
    pub kind: String,
    /// Exact transition, including its ownership and content identity.
    pub transition: SurfaceOperationTransitionV1,
    /// Validated Unix permission bits after the operation, or before deletion.
    pub mode: UnixFileMode,
    /// Whether the resulting portable artifact may enter a separate Git review.
    pub committable: bool,
}

/// A declared host effect omitted from this materialization transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeferredEffect {
    /// Native-harness route or host class that would consume the omitted effect.
    pub host: String,
    /// Semantic surface kind of the omitted effect.
    pub kind: String,
    /// Declared logical target; never interpreted as mutation authority.
    pub target: String,
    /// Human-readable reason the effect remains outside this transaction.
    pub note: String,
}

/// JSON Schema for one full-range filesystem coordinate encoded as decimal text.
///
/// RFC 8785 canonicalizes JSON numbers through the ECMAScript number model, but
/// native device and inode identities occupy the complete [`u64`] domain. The
/// portable record therefore carries their exact decimal spelling as a string.
const FILESYSTEM_U64_DECIMAL_PATTERN: &str = "^(?:0|[1-9][0-9]{0,18}|1[0-7][0-9]{18}|18[0-3][0-9]{17}|184[0-3][0-9]{16}|1844[0-5][0-9]{15}|18446[0-6][0-9]{14}|184467[0-3][0-9]{13}|1844674[0-3][0-9]{12}|184467440[0-6][0-9]{10}|1844674407[0-2][0-9]{9}|18446744073[0-6][0-9]{8}|1844674407370[0-8][0-9]{6}|18446744073709[0-4][0-9]{5}|184467440737095[0-4][0-9]{4}|18446744073709550[0-9]{3}|18446744073709551[0-5][0-9]{2}|1844674407370955160[0-9]|1844674407370955161[0-5])$";

/// Project canonical decimal text over the complete [`u64`] domain.
fn filesystem_u64_decimal_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::Schema::try_from(serde_json::json!({
        "type": "string",
        "minLength": 1,
        "maxLength": 20,
        "pattern": FILESYSTEM_U64_DECIMAL_PATTERN
    }))
    .expect("the static filesystem-coordinate schema is valid")
}

fn parse_filesystem_u64_decimal(value: &str) -> Result<u64, &'static str> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("filesystem coordinate must be canonical unsigned decimal text");
    }
    value
        .parse()
        .map_err(|_| "filesystem coordinate exceeds the unsigned 64-bit range")
}

macro_rules! filesystem_coordinate {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, schemars::JsonSchema)]
        #[schemars(transparent)]
        pub struct $name(
            #[schemars(schema_with = "filesystem_u64_decimal_schema")] u64,
        );

        impl $name {
            /// Capture one exact native filesystem coordinate.
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Return the native unsigned coordinate.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.0.to_string())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                parse_filesystem_u64_decimal(&value)
                    .map(Self)
                    .map_err(serde::de::Error::custom)
            }
        }
    };
}

filesystem_coordinate!(
    /// Full-range native filesystem device identity.
    ///
    /// Wire JSON uses canonical unsigned decimal text rather than a number, so
    /// every native [`u64`] value survives JCS exactly. Deserialization rejects
    /// numeric JSON, signs, noncanonical leading zeros, and overflow. This
    /// coordinate remains distinct from [`FilesystemInodeIdV1`] and proves no
    /// filesystem identity without its owning observation context.
    FilesystemDeviceIdV1
);
filesystem_coordinate!(
    /// Full-range native filesystem inode identity.
    ///
    /// Wire JSON uses canonical unsigned decimal text rather than a number, so
    /// every native [`u64`] value survives JCS exactly. Deserialization rejects
    /// numeric JSON, signs, noncanonical leading zeros, and overflow. This
    /// coordinate remains distinct from [`FilesystemDeviceIdV1`] and proves no
    /// filesystem identity without its owning observation context.
    FilesystemInodeIdV1
);

wire_enum!(
    /// Closed class of machine-bound effects admitted alongside repository surfaces.
    MachineLocalEffectKind {
        /// Managed protection in a checkout's common Git exclude file.
        GitExcludeProtection => "git_exclude_protection"
    }
);

/// Wire representation of a machine-local root identity observation.
///
/// A confined adapter captures and later revalidates these coordinates through
/// retained descriptors; constructing or decoding this record proves no path or
/// directory identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineLocalRootIdentity {
    /// Canonical absolute path used only to reopen and verify the same directory.
    pub canonical_path: String,
    /// Filesystem device number of the retained directory.
    pub device: FilesystemDeviceIdV1,
    /// Filesystem inode number of the retained directory.
    pub inode: FilesystemInodeIdV1,
}

/// Claimed identity of one directory beneath a machine-local root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineLocalDirectoryIdentity {
    /// Confined directory path relative to the retained root.
    pub rel_path: String,
    /// Filesystem device number observed from the opened directory.
    pub device: FilesystemDeviceIdV1,
    /// Filesystem inode number observed from the opened directory.
    pub inode: FilesystemInodeIdV1,
}

/// Claimed filesystem identity of one machine-local regular file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineLocalFileIdentityV1 {
    /// Filesystem device number of the file.
    pub device: FilesystemDeviceIdV1,
    /// Filesystem inode number of the file.
    pub inode: FilesystemInodeIdV1,
}

/// Content and permissions planned for one regular file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannedFileV1 {
    /// SHA-256 of the complete file bytes.
    pub sha256: ArtifactDigest,
    /// Validated Unix permission bits.
    pub mode: UnixFileMode,
}

/// Wire representation of one observed machine-local regular file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExistingMachineLocalFileV1 {
    /// SHA-256 of the complete observed bytes.
    pub sha256: ArtifactDigest,
    /// Observed Unix permission bits.
    pub mode: UnixFileMode,
    /// Device and inode identity captured through confined observation.
    pub identity: MachineLocalFileIdentityV1,
}

/// Planned transition for one machine-local regular file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum MachineLocalEffectTransitionV1 {
    /// Create an absent machine-local file and any listed missing ancestors.
    Create {
        /// Exact post-effect content and permissions.
        post: PlannedFileV1,
        /// Missing parents the transaction may create, shallowest first.
        create_directories: Vec<String>,
    },
    /// Replace an existing machine-local file.
    Update {
        /// Exact pre-effect state.
        prior: ExistingMachineLocalFileV1,
        /// Exact post-effect content and permissions.
        post: PlannedFileV1,
    },
    /// Replace a managed portion of an existing machine-local file.
    Splice {
        /// Exact pre-effect state.
        prior: ExistingMachineLocalFileV1,
        /// Exact post-effect content and permissions.
        post: PlannedFileV1,
    },
    /// Verify an existing file without changing it.
    Unchanged {
        /// Exact state that must remain present.
        state: ExistingMachineLocalFileV1,
    },
    /// Delete an existing machine-local file.
    Delete {
        /// Exact pre-effect state.
        prior: ExistingMachineLocalFileV1,
    },
}

impl MachineLocalEffectTransitionV1 {
    /// Project the action without admitting repository-surface `adopt`.
    #[must_use]
    pub const fn action(&self) -> SurfaceAction {
        match self {
            Self::Create { .. } => SurfaceAction::Create,
            Self::Update { .. } => SurfaceAction::Update,
            Self::Splice { .. } => SurfaceAction::Splice,
            Self::Unchanged { .. } => SurfaceAction::Unchanged,
            Self::Delete { .. } => SurfaceAction::Delete,
        }
    }

    /// Return the exact prior file state when one existed.
    #[must_use]
    pub const fn prior(&self) -> Option<&ExistingMachineLocalFileV1> {
        match self {
            Self::Create { .. } => None,
            Self::Update { prior, .. } | Self::Splice { prior, .. } | Self::Delete { prior } => {
                Some(prior)
            }
            Self::Unchanged { state } => Some(state),
        }
    }

    /// Return the planned post-effect mode, or claimed prior mode for deletion.
    #[must_use]
    pub const fn mode(&self) -> UnixFileMode {
        match self {
            Self::Create { post, .. } | Self::Update { post, .. } | Self::Splice { post, .. } => {
                post.mode
            }
            Self::Unchanged { state } => state.mode,
            Self::Delete { prior } => prior.mode,
        }
    }

    /// Return the parent paths this transition proposes creating.
    #[must_use]
    pub fn create_directories(&self) -> &[String] {
        match self {
            Self::Create {
                create_directories, ..
            } => create_directories,
            _ => &[],
        }
    }
}

/// One machine-bound file effect proposed by a v1 materialization plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineLocalEffectV1 {
    /// Typed purpose of this local effect.
    pub kind: MachineLocalEffectKind,
    /// Claimed common-directory identity that a confined adapter must revalidate.
    pub root: MachineLocalRootIdentity,
    /// Confined path relative to `root`; never a repository surface identity.
    pub rel_path: String,
    /// Existing parent-directory identities, shallowest first.
    pub retained_directories: Vec<MachineLocalDirectoryIdentity>,
    /// Exact pre/post transition and any authorized ancestor creation.
    pub transition: MachineLocalEffectTransitionV1,
}

/// Planned ownership-state transition selected by one repository-surface plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaterializationStateTransitionV1 {
    /// First apply into a target with no prior Pale ownership state.
    Initialize {
        /// Complete ownership state to publish.
        next_state: MaterializationStateV1,
    },
    /// Apply from a referenced prior ownership state to its successor.
    Advance {
        /// Prior-state claim and the compilation attributed to it.
        prior: PriorMaterializationStateV1,
        /// Complete ownership state to publish.
        next_state: MaterializationStateV1,
    },
    /// Remove a referenced prior ownership state.
    Remove {
        /// Prior-state claim and the compilation attributed to it.
        prior: PriorMaterializationStateV1,
    },
    /// Confirm removal where no Pale ownership state exists.
    RemoveUnowned {},
}

/// Claimed identity of one prior materialization state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PriorMaterializationStateV1 {
    /// Claimed canonical digest of the prior ownership state.
    pub state_digest: MaterializationStateDigest,
    /// One-shot compilation attributed to the prior state.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
}

impl MaterializationStateTransitionV1 {
    /// Derive the operation class selected by this transition.
    #[must_use]
    pub const fn plan_kind(&self) -> PlanKind {
        match self {
            Self::Initialize { .. } | Self::Advance { .. } => PlanKind::Apply,
            Self::Remove { .. } | Self::RemoveUnowned {} => PlanKind::Remove,
        }
    }

    /// Return the prior-state claim when one exists.
    #[must_use]
    pub const fn prior(&self) -> Option<&PriorMaterializationStateV1> {
        match self {
            Self::Advance { prior, .. } | Self::Remove { prior } => Some(prior),
            Self::Initialize { .. } | Self::RemoveUnowned {} => None,
        }
    }

    /// Return the complete state that an apply transition publishes.
    #[must_use]
    pub const fn next_state(&self) -> Option<&MaterializationStateV1> {
        match self {
            Self::Initialize { next_state } | Self::Advance { next_state, .. } => Some(next_state),
            Self::Remove { .. } | Self::RemoveUnowned {} => None,
        }
    }
}

/// Saved repository-surface plan candidate for later admission.
///
/// [`crate::admission::admit_plan`] admits the enclosing materialization plan
/// only when its self-digests and a caller-supplied rebuild agree. The effect
/// adapter still reobserves targets immediately before mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentSurfacePlanV1 {
    /// Stable wire identity of the repository-surface subplan.
    #[schemars(with = "String", extend("const" = AGENT_SURFACE_PLAN_V1_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Descriptor-manifest identity claimed by the owning compilation.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity claimed by the owning compilation.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Exact lawful prior-to-next ownership transition.
    pub transition: MaterializationStateTransitionV1,
    /// Auditable operation marker; plan admission is established separately.
    pub operation_marker: String,
    /// Declared admission posture for evidence only. This string never authorizes
    /// mutation; [`crate::admission::admit_plan`] returns independently verified
    /// evidence for the mutation boundary.
    pub admission: String,
    /// Compiler release that resolved the plan inputs.
    pub compiler_version: String,
    /// Renderer release that produced the expected payloads.
    pub renderer_version: String,
    /// Exact transitive Package-release closure used for compilation.
    #[schemars(extend("uniqueItems" = true))]
    pub package_closure: Vec<PackageReleaseV1>,
    /// Exact normative Pack closure used for compilation.
    #[schemars(extend("uniqueItems" = true))]
    pub normative_pack_closure: Vec<NormativePackReleaseV1>,
    /// Portable profile identity claimed by this plan.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity claimed by this plan.
    pub instance_digest: InstanceDigest,
    /// Generated-surface manifest identity claimed by this plan.
    pub manifest_digest: ManifestDigest,
    /// Exact selected and admitted Package semantic closure used by this plan.
    pub package_semantic_ir_digest: PackageSemanticIrDigest,
    /// Logical repository targeted by every operation in this plan.
    pub repository_id: LogicalRepositoryIdV1,
    /// Canonically ordered pre-mutation observations for every relevant target.
    pub target_baseline: Vec<BaselineEntry>,
    /// Canonically ordered, fully specified filesystem effects.
    pub operations: Vec<Operation>,
    /// Declared host effects intentionally omitted from this transaction.
    pub deferred_effects: Vec<DeferredEffect>,
    /// Claimed canonical identity of the complete repository-surface plan body.
    pub plan_digest: PlanDigest,
}

/// Unsealed construction input for [`AgentSurfacePlanV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentSurfacePlanPreimageV1 {
    /// Stable wire identity of the repository-surface subplan.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Descriptor-manifest identity admitted by the owning compilation.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity admitted by the owning compilation.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Exact lawful prior-to-next ownership transition.
    pub transition: MaterializationStateTransitionV1,
    /// Auditable operation marker.
    pub operation_marker: String,
    /// Declared admission posture retained as evidence only.
    pub admission: String,
    /// Compiler release that resolved the plan inputs.
    pub compiler_version: String,
    /// Renderer release that produced the expected payloads.
    pub renderer_version: String,
    /// Exact transitive Package-release closure used for compilation.
    pub package_closure: Vec<PackageReleaseV1>,
    /// Exact normative Pack closure used for compilation.
    pub normative_pack_closure: Vec<NormativePackReleaseV1>,
    /// Portable profile identity admitted by this plan.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity admitted by this plan.
    pub instance_digest: InstanceDigest,
    /// Generated-surface manifest identity admitted by this plan.
    pub manifest_digest: ManifestDigest,
    /// Exact selected and admitted Package semantic closure used by this plan.
    pub package_semantic_ir_digest: PackageSemanticIrDigest,
    /// Logical repository targeted by every operation.
    pub repository_id: LogicalRepositoryIdV1,
    /// Canonically ordered pre-mutation observations.
    pub target_baseline: Vec<BaselineEntry>,
    /// Canonically ordered, fully specified filesystem effects.
    pub operations: Vec<Operation>,
    /// Declared host effects outside this transaction.
    pub deferred_effects: Vec<DeferredEffect>,
}

/// Failure to validate and canonically seal a repository-surface plan.
#[derive(Debug, thiserror::Error)]
pub enum AgentSurfacePlanSealErrorV1 {
    /// The selected Package or normative Pack frontier is not admitted.
    #[error(transparent)]
    ReleaseClosure(#[from] ReleaseClosureErrorV1),
    /// The admitted plan body cannot be represented by canonical JSON.
    #[error(transparent)]
    Canon(#[from] canon::CanonError),
}

impl AgentSurfacePlanV1 {
    /// Compute and attach the canonical identity of a surface-plan body.
    ///
    /// # Errors
    ///
    /// Returns [`AgentSurfacePlanSealErrorV1`] when either exact release closure
    /// is unbounded/non-canonical or the admitted body cannot satisfy canonical
    /// JSON.
    pub fn seal(preimage: AgentSurfacePlanPreimageV1) -> Result<Self, AgentSurfacePlanSealErrorV1> {
        validate_release_closures(&preimage.package_closure, &preimage.normative_pack_closure)?;
        let plan_digest = PlanDigest::hash_canonical(&preimage)?;
        let AgentSurfacePlanPreimageV1 {
            schema,
            schema_sha256,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            transition,
            operation_marker,
            admission,
            compiler_version,
            renderer_version,
            package_closure,
            normative_pack_closure,
            profile_digest,
            instance_digest,
            manifest_digest,
            package_semantic_ir_digest,
            repository_id,
            target_baseline,
            operations,
            deferred_effects,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            transition,
            operation_marker,
            admission,
            compiler_version,
            renderer_version,
            package_closure,
            normative_pack_closure,
            profile_digest,
            instance_digest,
            manifest_digest,
            package_semantic_ir_digest,
            repository_id,
            target_baseline,
            operations,
            deferred_effects,
            plan_digest,
        })
    }

    /// Clone the unhashed plan body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> AgentSurfacePlanPreimageV1 {
        AgentSurfacePlanPreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_manifest_sha256: self.schema_manifest_sha256.clone(),
            schema_set_sha256: self.schema_set_sha256.clone(),
            one_shot_compilation_digest: self.one_shot_compilation_digest.clone(),
            transition: self.transition.clone(),
            operation_marker: self.operation_marker.clone(),
            admission: self.admission.clone(),
            compiler_version: self.compiler_version.clone(),
            renderer_version: self.renderer_version.clone(),
            package_closure: self.package_closure.clone(),
            normative_pack_closure: self.normative_pack_closure.clone(),
            profile_digest: self.profile_digest.clone(),
            instance_digest: self.instance_digest.clone(),
            manifest_digest: self.manifest_digest.clone(),
            package_semantic_ir_digest: self.package_semantic_ir_digest.clone(),
            repository_id: self.repository_id.clone(),
            target_baseline: self.target_baseline.clone(),
            operations: self.operations.clone(),
            deferred_effects: self.deferred_effects.clone(),
        }
    }

    /// Recompute the canonical identity of this plan's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`canon::CanonError`] when the body cannot satisfy the canonical
    /// JSON contract.
    pub fn recompute_digest(&self) -> Result<PlanDigest, canon::CanonError> {
        PlanDigest::hash_canonical(&self.to_preimage())
    }
}

/// Saved v1 plan candidate binding a repository-surface plan to the declared
/// machine-local effects for the same target transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterializationPlanV1 {
    /// Exact wire identity owned by `pale-core`.
    #[schemars(with = "String", extend("const" = MATERIALIZATION_PLAN_V1_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Wire major of this envelope.
    #[schemars(extend("const" = MATERIALIZATION_PLAN_V1_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Descriptor-manifest identity copied from the nested surface plan.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity copied from the nested surface plan.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity copied from the nested plan.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Complete repository-surface subplan carrying its own digest claim.
    pub surface_plan: AgentSurfacePlanV1,
    /// Canonically ordered machine-bound effects outside repository ownership.
    pub machine_local_effects: Vec<MachineLocalEffectV1>,
    /// Claimed canonical identity binding the surface plan and machine-local effects.
    pub plan_digest: PlanDigest,
}

/// Unsealed construction input for [`MaterializationPlanV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaterializationPlanPreimageV1 {
    /// Exact wire identity owned by `pale-core`.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Wire major of this envelope.
    pub schema_version: u32,
    /// Descriptor-manifest identity copied from the nested surface plan.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity copied from the nested surface plan.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Complete repository-surface subplan.
    pub surface_plan: AgentSurfacePlanV1,
    /// Canonically ordered machine-bound effects.
    pub machine_local_effects: Vec<MachineLocalEffectV1>,
}

impl MaterializationPlanV1 {
    /// Compute and attach the canonical identity of a materialization-plan body.
    ///
    /// # Errors
    ///
    /// Returns [`canon::CanonError`] when the preimage cannot satisfy the
    /// canonical JSON contract.
    pub fn seal(preimage: MaterializationPlanPreimageV1) -> Result<Self, canon::CanonError> {
        let plan_digest = PlanDigest::hash_canonical(&preimage)?;
        let MaterializationPlanPreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            surface_plan,
            machine_local_effects,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            surface_plan,
            machine_local_effects,
            plan_digest,
        })
    }

    /// Clone the unhashed plan body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> MaterializationPlanPreimageV1 {
        MaterializationPlanPreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            schema_manifest_sha256: self.schema_manifest_sha256.clone(),
            schema_set_sha256: self.schema_set_sha256.clone(),
            one_shot_compilation_digest: self.one_shot_compilation_digest.clone(),
            surface_plan: self.surface_plan.clone(),
            machine_local_effects: self.machine_local_effects.clone(),
        }
    }

    /// Recompute the canonical identity of this plan's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`canon::CanonError`] when the body cannot satisfy the canonical
    /// JSON contract.
    pub fn recompute_digest(&self) -> Result<PlanDigest, canon::CanonError> {
        PlanDigest::hash_canonical(&self.to_preimage())
    }
}
