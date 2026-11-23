//! Receipt and run-result wire records for materialization.
//!
//! Console output is not a receipt. A verified receipt binds the profile,
//! manifest, plan, operations, and resulting state so drift is visible. These
//! public records are still claims when constructed or decoded: `seal` computes
//! a self-digest but does not observe an effect or admit the claimed evidence.

use serde::{Deserialize, Serialize};

use super::super::{
    validate_release_closures, ArtifactDigest, InstanceDigest, LogicalExecutionWorkspaceIdV1,
    LogicalProjectIdV1, LogicalRepositoryIdV1, ManifestDigest, MaterializationLineageDigest,
    MaterializationStateDigest, NormativePackReleaseV1, OneShotCompilationDigest, Outcome,
    PackageReleaseV1, PackageSemanticIrDigest, PlanDigest, PlanKind, ProfileDigest, ReceiptDigest,
    ReleaseClosureErrorV1, RunDigest, SchemaId, SchemaManifestDigest, SchemaSetDigest,
};
use super::materialization::DeferredEffect;
use super::materialization::{
    MachineLocalEffectKind, MachineLocalFileIdentityV1, MachineLocalRootIdentity, UnixFileMode,
};

/// Stable wire identity carried by [`MaterializationReceiptV1::schema`].
pub const MATERIALIZATION_RECEIPT_V1_SCHEMA: &str = "pale.agent-surface.receipt.v1";
/// Wire major of [`MaterializationReceiptV1`].
pub const MATERIALIZATION_RECEIPT_V1_SCHEMA_VERSION: u32 = 1;
/// Stable wire identity carried by [`RunResultV1::schema`].
pub const RUN_RESULT_V1_SCHEMA: &str = "pale.agent-surface.run-result.v1";
/// Wire major of [`RunResultV1`].
pub const RUN_RESULT_V1_SCHEMA_VERSION: u32 = 1;

/// Whether an unchanged-content operation also repaired permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UnchangedDispositionV1 {
    /// Existing bytes and permissions already matched the plan.
    Verified,
    /// Existing bytes matched while permissions were repaired to the planned mode.
    ModeRepaired,
}

/// Wire representation of a successful repository-surface creation observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "target", rename_all = "snake_case", deny_unknown_fields)]
pub enum SurfaceCreationObservationV1 {
    /// The operation created a new target file.
    AbsentFile {
        /// SHA-256 observed after creation.
        post_sha256: ArtifactDigest,
    },
    /// The operation inserted a new managed block into an existing user-owned file.
    ExistingFile {
        /// SHA-256 of the complete file before insertion.
        pre_sha256: ArtifactDigest,
        /// SHA-256 of the complete file after insertion.
        post_sha256: ArtifactDigest,
    },
}

/// Wire representation of a successful repository-surface operation observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum SurfaceOperationObservationV1 {
    /// An absent surface was created.
    Create {
        /// Whether creation wrote an absent file or inserted into an existing one.
        target: SurfaceCreationObservationV1,
    },
    /// A pre-existing generated surface entered Pale ownership.
    Adopt {
        /// SHA-256 observed before adoption.
        pre_sha256: ArtifactDigest,
        /// SHA-256 observed after adoption.
        post_sha256: ArtifactDigest,
    },
    /// An owned surface was replaced.
    Update {
        /// SHA-256 observed before replacement.
        pre_sha256: ArtifactDigest,
        /// SHA-256 observed after replacement.
        post_sha256: ArtifactDigest,
    },
    /// Owned content remained byte-identical.
    Unchanged {
        /// Stable SHA-256 observed before and after the operation.
        sha256: ArtifactDigest,
        /// Whether permissions also required repair.
        disposition: UnchangedDispositionV1,
    },
    /// A previously owned whole file was removed.
    Delete {
        /// SHA-256 observed before removal.
        pre_sha256: ArtifactDigest,
    },
    /// A previously owned managed block was removed from a user-owned file.
    Splice {
        /// SHA-256 observed before the managed-block edit.
        pre_sha256: ArtifactDigest,
        /// SHA-256 observed after the managed-block edit.
        post_sha256: ArtifactDigest,
    },
}

impl SurfaceOperationObservationV1 {
    /// Whether the successful operation changed bytes, permissions, or ownership.
    #[must_use]
    pub const fn changed(&self) -> bool {
        !matches!(
            self,
            Self::Unchanged {
                disposition: UnchangedDispositionV1::Verified,
                ..
            }
        )
    }
}

/// Claimed operation effect with structurally complete content evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReceiptOperation {
    /// Confined repository-relative target affected by the operation.
    pub rel_path: String,
    /// Claimed successful action and its required pre/post identities.
    pub observation: SurfaceOperationObservationV1,
    /// Validated Unix permissions after the operation, or before deletion.
    pub mode: UnixFileMode,
}

/// Claimed identity of a machine-local file observed after publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineLocalPostFileV1 {
    /// SHA-256 of the complete published bytes.
    pub sha256: ArtifactDigest,
    /// Device and inode identity captured after publication.
    pub identity: MachineLocalFileIdentityV1,
}

/// Wire representation of a successful machine-local effect observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum MachineLocalEffectObservationV1 {
    /// An absent file and any listed parent directories were created.
    Create {
        /// Exact post-effect file identity.
        post: MachineLocalPostFileV1,
        /// Parent directories created by this transaction, shallowest first.
        created_directories: Vec<String>,
    },
    /// An existing file was replaced.
    Update {
        /// SHA-256 observed before replacement.
        pre_sha256: ArtifactDigest,
        /// Exact post-effect file identity.
        post: MachineLocalPostFileV1,
    },
    /// A managed portion of an existing file was replaced.
    Splice {
        /// SHA-256 observed before the managed edit.
        pre_sha256: ArtifactDigest,
        /// Exact post-effect file identity.
        post: MachineLocalPostFileV1,
    },
    /// An existing file remained byte- and identity-stable.
    Unchanged {
        /// Exact post-effect file identity.
        state: MachineLocalPostFileV1,
    },
    /// An existing file was removed.
    Delete {
        /// SHA-256 observed before removal.
        pre_sha256: ArtifactDigest,
    },
}

impl MachineLocalEffectObservationV1 {
    /// Whether the successful machine-local effect changed the target.
    #[must_use]
    pub const fn changed(&self) -> bool {
        !matches!(self, Self::Unchanged { .. })
    }
}

/// Claimed observed result of one admitted machine-local effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineLocalReceiptOperationV1 {
    /// Typed purpose copied from the admitted effect.
    pub kind: MachineLocalEffectKind,
    /// Exact machine-local root identity copied from the admitted effect.
    pub root: MachineLocalRootIdentity,
    /// Confined path relative to `root`.
    pub rel_path: String,
    /// Exact successful transition and its required identity evidence.
    pub observation: MachineLocalEffectObservationV1,
    /// Validated Unix mode after the effect, or before deletion.
    pub mode: UnixFileMode,
}

/// State and lineage transition claimed by a successful receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaterializationReceiptTransitionV1 {
    /// First ownership state and lineage were published.
    Initialized {
        /// Published ownership-state identity.
        post_state_digest: MaterializationStateDigest,
        /// Published machine-local lineage identity.
        post_lineage_digest: MaterializationLineageDigest,
    },
    /// Existing ownership state and lineage advanced atomically.
    Advanced {
        /// Trusted ownership-state identity before the transaction.
        prior_state_digest: MaterializationStateDigest,
        /// Published ownership-state identity after the transaction.
        post_state_digest: MaterializationStateDigest,
        /// Trusted lineage identity before the transaction.
        prior_lineage_digest: MaterializationLineageDigest,
        /// Published lineage identity after the transaction.
        post_lineage_digest: MaterializationLineageDigest,
    },
    /// Existing ownership state and lineage were removed.
    Removed {
        /// Trusted ownership-state identity before removal.
        prior_state_digest: MaterializationStateDigest,
        /// Trusted lineage identity before removal.
        prior_lineage_digest: MaterializationLineageDigest,
    },
    /// The target had no owned state or lineage to remove.
    NoOwnedState {},
}

impl MaterializationReceiptTransitionV1 {
    /// Derive the operation class encoded by this receipt transition.
    #[must_use]
    pub const fn plan_kind(&self) -> PlanKind {
        match self {
            Self::Initialized { .. } | Self::Advanced { .. } => PlanKind::Apply,
            Self::Removed { .. } | Self::NoOwnedState {} => PlanKind::Remove,
        }
    }
}

/// Digest-bearing per-target receipt claim covering planned effects and results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MaterializationReceiptV1 {
    /// Exact wire identity owned by `pale-core`.
    #[schemars(with = "String", extend("const" = MATERIALIZATION_RECEIPT_V1_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Wire major of this receipt.
    #[schemars(extend("const" = MATERIALIZATION_RECEIPT_V1_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Descriptor-manifest identity copied from the admitted plan.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity copied from the admitted plan.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity copied from the admitted plan.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Logical repository whose transaction this receipt closes.
    pub repository_id: LogicalRepositoryIdV1,
    /// Claimed successful state and lineage transition.
    pub transition: MaterializationReceiptTransitionV1,
    /// Portable profile identity copied from the admitted plan.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity copied from the admitted plan.
    pub instance_digest: InstanceDigest,
    /// Generated manifest identity copied from the admitted plan.
    pub manifest_digest: ManifestDigest,
    /// Exact Package-release closure copied from the admitted surface plan.
    #[schemars(extend("uniqueItems" = true))]
    pub package_closure: Vec<PackageReleaseV1>,
    /// Exact normative Pack closure copied from the admitted surface plan.
    #[schemars(extend("uniqueItems" = true))]
    pub normative_pack_closure: Vec<NormativePackReleaseV1>,
    /// Exact selected and admitted Package semantic closure copied from the plan.
    pub package_semantic_ir_digest: PackageSemanticIrDigest,
    /// Digest of the nested repository-surface plan.
    pub surface_plan_digest: PlanDigest,
    /// Claimed admitted-plan identity bound to this transaction.
    pub plan_digest: PlanDigest,
    /// Results for every planned repository-surface operation in plan order.
    pub surface_operations: Vec<ReceiptOperation>,
    /// Results for every planned machine-local effect in plan order.
    pub machine_local_effects: Vec<MachineLocalReceiptOperationV1>,
    /// Declared effects that remained outside this transaction.
    pub deferred_effects: Vec<DeferredEffect>,
    /// Claimed canonical identity of the complete receipt body.
    pub receipt_digest: ReceiptDigest,
}

/// Unsealed construction input for [`MaterializationReceiptV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaterializationReceiptPreimageV1 {
    /// Exact wire identity owned by `pale-core`.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Wire major of this receipt.
    pub schema_version: u32,
    /// Descriptor-manifest identity copied from the admitted plan.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity copied from the admitted plan.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Logical repository whose transaction this receipt closes.
    pub repository_id: LogicalRepositoryIdV1,
    /// Exact successful state and lineage transition.
    pub transition: MaterializationReceiptTransitionV1,
    /// Portable profile identity copied from the admitted plan.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity copied from the admitted plan.
    pub instance_digest: InstanceDigest,
    /// Generated manifest identity copied from the admitted plan.
    pub manifest_digest: ManifestDigest,
    /// Exact Package-release closure copied from the admitted surface plan.
    pub package_closure: Vec<PackageReleaseV1>,
    /// Exact normative Pack closure copied from the admitted surface plan.
    pub normative_pack_closure: Vec<NormativePackReleaseV1>,
    /// Exact selected and admitted Package semantic closure copied from the plan.
    pub package_semantic_ir_digest: PackageSemanticIrDigest,
    /// Digest of the nested repository-surface plan.
    pub surface_plan_digest: PlanDigest,
    /// Exact admitted-plan identity bound to this transaction.
    pub plan_digest: PlanDigest,
    /// Results for every repository-surface operation.
    pub surface_operations: Vec<ReceiptOperation>,
    /// Results for every machine-local effect.
    pub machine_local_effects: Vec<MachineLocalReceiptOperationV1>,
    /// Declared effects outside this transaction.
    pub deferred_effects: Vec<DeferredEffect>,
}

/// Failure to validate and canonically seal a materialization receipt.
#[derive(Debug, thiserror::Error)]
pub enum MaterializationReceiptSealErrorV1 {
    /// The selected Package or normative Pack frontier is not admitted.
    #[error(transparent)]
    ReleaseClosure(#[from] ReleaseClosureErrorV1),
    /// The admitted receipt body cannot be represented by canonical JSON.
    #[error(transparent)]
    Canon(#[from] crate::canon::CanonError),
}

impl MaterializationReceiptV1 {
    /// Compute and attach the canonical identity of a receipt body.
    ///
    /// This does not prove that the claimed observations or effects occurred.
    ///
    /// # Errors
    ///
    /// Returns [`MaterializationReceiptSealErrorV1`] when either exact release
    /// closure is unbounded/non-canonical or the admitted body cannot satisfy
    /// canonical JSON.
    pub fn seal(
        preimage: MaterializationReceiptPreimageV1,
    ) -> Result<Self, MaterializationReceiptSealErrorV1> {
        validate_release_closures(&preimage.package_closure, &preimage.normative_pack_closure)?;
        let receipt_digest = ReceiptDigest::hash_canonical(&preimage)?;
        let MaterializationReceiptPreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            repository_id,
            transition,
            profile_digest,
            instance_digest,
            manifest_digest,
            package_closure,
            normative_pack_closure,
            package_semantic_ir_digest,
            surface_plan_digest,
            plan_digest,
            surface_operations,
            machine_local_effects,
            deferred_effects,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            repository_id,
            transition,
            profile_digest,
            instance_digest,
            manifest_digest,
            package_closure,
            normative_pack_closure,
            package_semantic_ir_digest,
            surface_plan_digest,
            plan_digest,
            surface_operations,
            machine_local_effects,
            deferred_effects,
            receipt_digest,
        })
    }

    /// Clone the unhashed receipt body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> MaterializationReceiptPreimageV1 {
        MaterializationReceiptPreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            schema_manifest_sha256: self.schema_manifest_sha256.clone(),
            schema_set_sha256: self.schema_set_sha256.clone(),
            one_shot_compilation_digest: self.one_shot_compilation_digest.clone(),
            repository_id: self.repository_id.clone(),
            transition: self.transition.clone(),
            profile_digest: self.profile_digest.clone(),
            instance_digest: self.instance_digest.clone(),
            manifest_digest: self.manifest_digest.clone(),
            package_closure: self.package_closure.clone(),
            normative_pack_closure: self.normative_pack_closure.clone(),
            package_semantic_ir_digest: self.package_semantic_ir_digest.clone(),
            surface_plan_digest: self.surface_plan_digest.clone(),
            plan_digest: self.plan_digest.clone(),
            surface_operations: self.surface_operations.clone(),
            machine_local_effects: self.machine_local_effects.clone(),
            deferred_effects: self.deferred_effects.clone(),
        }
    }

    /// Recompute the canonical identity of this receipt's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_digest(&self) -> Result<ReceiptDigest, crate::canon::CanonError> {
        ReceiptDigest::hash_canonical(&self.to_preimage())
    }
}

/// One repository's claimed terminal result within an execution-workspace run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunTargetResult {
    /// The target claims completion with a successful receipt identity.
    Succeeded {
        /// Logical repository represented by this target row.
        repository_id: LogicalRepositoryIdV1,
        /// Immutable receipt identity for the completed target.
        receipt_digest: ReceiptDigest,
    },
    /// The target failed before publishing an effect or receipt.
    FailedNoEffect {
        /// Logical repository represented by this target row.
        repository_id: LogicalRepositoryIdV1,
    },
}

impl RunTargetResult {
    /// Return the logical repository represented by this target row.
    #[must_use]
    pub const fn repository_id(&self) -> &LogicalRepositoryIdV1 {
        match self {
            Self::Succeeded { repository_id, .. } | Self::FailedNoEffect { repository_id } => {
                repository_id
            }
        }
    }

    /// Return the claimed receipt identity from a successful variant.
    #[must_use]
    pub const fn receipt_digest(&self) -> Option<&ReceiptDigest> {
        match self {
            Self::Succeeded { receipt_digest, .. } => Some(receipt_digest),
            Self::FailedNoEffect { .. } => None,
        }
    }

    /// Derive the target outcome from its exact variant.
    #[must_use]
    pub const fn outcome(&self) -> Outcome {
        match self {
            Self::Succeeded { .. } => Outcome::Succeeded,
            Self::FailedNoEffect { .. } => Outcome::FailedNoEffect,
        }
    }
}

/// Digest-bearing project and execution-workspace run-result claim.
///
/// In particular, [`Outcome::Partial`] remains distinct from success evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunResultV1 {
    /// Stable wire identity of the run result.
    #[schemars(with = "String", extend("const" = RUN_RESULT_V1_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Run-result schema major.
    #[schemars(extend("const" = RUN_RESULT_V1_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Descriptor-manifest identity shared by every target compilation record.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity shared by every target compilation record.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity shared by every target.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Logical project whose ordered targets were attempted.
    pub project_id: LogicalProjectIdV1,
    /// Logical execution workspace used for the attempt.
    pub execution_workspace_id: LogicalExecutionWorkspaceIdV1,
    /// Apply or remove class shared by every target result.
    pub plan_kind: PlanKind,
    /// Portable profile identity shared by every target plan.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity shared by every target plan.
    pub instance_digest: InstanceDigest,
    /// Per-target results in declared materialization order.
    pub targets: Vec<RunTargetResult>,
    /// Claimed canonical identity of the complete run-result body.
    pub run_digest: RunDigest,
}

/// Unsealed construction input for [`RunResultV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunResultPreimageV1 {
    /// Stable wire identity of the run result.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Run-result schema major.
    pub schema_version: u32,
    /// Descriptor-manifest identity shared by every target.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Active raw-schema-set identity shared by every target.
    pub schema_set_sha256: SchemaSetDigest,
    /// Complete post-render one-shot compilation identity.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
    /// Logical project whose targets were attempted.
    pub project_id: LogicalProjectIdV1,
    /// Logical execution workspace used for the attempt.
    pub execution_workspace_id: LogicalExecutionWorkspaceIdV1,
    /// Apply or remove class shared by every target.
    pub plan_kind: PlanKind,
    /// Portable profile identity shared by every target.
    pub profile_digest: ProfileDigest,
    /// Machine-resolved identity shared by every target.
    pub instance_digest: InstanceDigest,
    /// Per-target results in declared materialization order.
    pub targets: Vec<RunTargetResult>,
}

impl RunResultV1 {
    /// Compute and attach the canonical identity of a run-result body.
    ///
    /// This does not verify the referenced receipts or their native effects.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the preimage cannot satisfy
    /// the canonical JSON contract.
    pub fn seal(preimage: RunResultPreimageV1) -> Result<Self, crate::canon::CanonError> {
        let run_digest = RunDigest::hash_canonical(&preimage)?;
        let RunResultPreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            project_id,
            execution_workspace_id,
            plan_kind,
            profile_digest,
            instance_digest,
            targets,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            schema_manifest_sha256,
            schema_set_sha256,
            one_shot_compilation_digest,
            project_id,
            execution_workspace_id,
            plan_kind,
            profile_digest,
            instance_digest,
            targets,
            run_digest,
        })
    }

    /// Clone the unhashed run-result body for amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> RunResultPreimageV1 {
        RunResultPreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            schema_manifest_sha256: self.schema_manifest_sha256.clone(),
            schema_set_sha256: self.schema_set_sha256.clone(),
            one_shot_compilation_digest: self.one_shot_compilation_digest.clone(),
            project_id: self.project_id.clone(),
            execution_workspace_id: self.execution_workspace_id.clone(),
            plan_kind: self.plan_kind,
            profile_digest: self.profile_digest.clone(),
            instance_digest: self.instance_digest.clone(),
            targets: self.targets.clone(),
        }
    }

    /// Recompute the canonical identity of this result's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_digest(&self) -> Result<RunDigest, crate::canon::CanonError> {
        RunDigest::hash_canonical(&self.to_preimage())
    }

    /// Derive the aggregate outcome from the complete target sequence.
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        let succeeded = self
            .targets
            .iter()
            .filter(|target| matches!(target, RunTargetResult::Succeeded { .. }))
            .count();
        match (succeeded, self.targets.len()) {
            (0, _) => Outcome::FailedNoEffect,
            (count, total) if count == total => Outcome::Succeeded,
            _ => Outcome::Partial,
        }
    }
}
