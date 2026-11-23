//! Portable project profiles and their machine-resolved instances.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::super::{
    validate_release_closures, AdmittedGoverningOrderV1, AdmittedLifecycleBehaviorV1,
    CapabilityBinding, CapabilityIdV1, CapabilityNarrowingV1, DurableEvidenceBinding,
    ExecutionPathSelectionV1, ExecutionWorkspaceRepositoryAccessV1, GoverningAlgebraErrorV1,
    InstanceDigest, InvalidLifecycleBehavior, InvalidRunPolicy, LogicalExecutionWorkspaceIdV1,
    LogicalProjectIdV1, LogicalRepositoryIdV1, MachineOverlayDigest, MaterializationLineageBinding,
    ModeAxis, NativeHarnessBinding, NormativePackReleaseV1, OperationBindingEntryV1,
    PackageReleaseV1, PackageResourceRefV1, PackageSemanticIrDigest, ProfileDigest,
    ProviderRepositoryProjectionV1, ReleaseClosureErrorV1, RunPolicyEntry, SchemaId,
    SemanticSelection, SessionModel, SessionStateBinding, SurfaceExecutableBinding, WorkspaceId,
};

/// Stable wire identity carried by [`EffectiveProjectProfileV1::schema`].
pub const EFFECTIVE_PROJECT_PROFILE_SCHEMA: &str = "pale.agent-surface.effective-profile.v1";

/// Wire major carried by [`EffectiveProjectProfileV1::schema_version`].
pub const EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION: u32 = 1;

/// Stable wire identity carried by [`ResolvedProjectInstanceV1::schema`].
pub const RESOLVED_PROJECT_INSTANCE_SCHEMA: &str = "pale.agent-surface.resolved-instance.v1";

/// Portable repository semantics bound into `profile_digest`: access ceiling,
/// role, stack and family selection, and repository-native capability commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfileRepository {
    /// Logical repository ID, independent of its machine checkout path.
    pub id: LogicalRepositoryIdV1,
    /// Maximum access admitted by the selected execution-workspace edge.
    ///
    /// This ceiling is distinct from later operation/effect admission.
    /// [`ExecutionWorkspaceRepositoryAccessV1::Read`] cannot be widened merely
    /// because a project or machine can resolve the same repository identity.
    pub access: ExecutionWorkspaceRepositoryAccessV1,
    /// Exact Package-owned semantic repository role.
    pub role: PackageResourceRefV1,
    /// Explicit stack overlays selected for this repository. Detection may be
    /// used as a validation aid, but it never silently changes this portable
    /// selection.
    #[serde(default)]
    pub stacks: Vec<PackageResourceRefV1>,
    /// Validated owner-qualified policy-family selections.
    pub families: Vec<PackageResourceRefV1>,
    /// Exact mapping from capability class to shell-free repository commands. The resolver
    /// selects only the class required by an operation.
    #[serde(default)]
    pub capability_commands: BTreeMap<CapabilityIdV1, Vec<String>>,
    /// Mutating commands that must never run automatically.
    #[serde(default)]
    pub mutating_not_readonly: Vec<String>,
    /// Optional descriptive note inherited from the repository-role command
    /// declaration; it grants no execution authority.
    #[serde(default)]
    pub command_note: Option<String>,
}

/// Portable semantic result containing no machine paths.
///
/// Its self-digest identifies the canonical encoding of this record. Exact
/// Package source bytes are bound by [`Self::package_closure`], while the selected,
/// dependency-closed semantic graph is bound by
/// [`Self::package_semantic_ir_digest`]. Neither coordinate implies Pack
/// attestation, Pale Runtime assembly, activation, or native effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EffectiveProjectProfileV1 {
    /// Stable wire identity of the effective-profile major.
    #[schemars(with = "String", extend("const" = EFFECTIVE_PROJECT_PROFILE_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Effective-profile schema major.
    #[schemars(extend("const" = EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION))]
    pub schema_version: u32,
    /// Principal workspace that owns the desired graph compiled into this profile.
    pub workspace_id: WorkspaceId,
    /// Canonical logical project resolved into this profile.
    pub project_id: LogicalProjectIdV1,
    /// Canonical execution workspace selected for this compilation.
    pub execution_workspace_id: LogicalExecutionWorkspaceIdV1,
    /// Exact neutral execution-path selections admitted for the workspace.
    pub execution_paths: Vec<ExecutionPathSelectionV1>,
    /// Exact private lifecycle policy selected by the project.
    ///
    /// Carrying the full record makes the profile digest bind the mechanics it
    /// enforces; no Package catalog or application-authored default is consulted.
    pub run_policy: RunPolicyEntry,
    /// Exact selected Package-lock closure, including transitive dependencies.
    #[schemars(extend("uniqueItems" = true))]
    pub package_closure: Vec<PackageReleaseV1>,
    /// Exact selected normative Pack closure, separate from Package procedure.
    #[schemars(extend("uniqueItems" = true))]
    pub normative_pack_closure: Vec<NormativePackReleaseV1>,
    /// Exact selected and admitted Package semantic closure.
    pub package_semantic_ir_digest: PackageSemanticIrDigest,
    /// Logical authority-root IDs; machine paths remain outside this record.
    pub authority_root_ids: Vec<String>,
    /// Effective materialization order: declared order when present, otherwise
    /// repository ID order. Plans, execution, receipts, and run results preserve it.
    pub repositories: Vec<ProfileRepository>,
    /// Canonically normalized scoped constraints, narrowing, and partial orders.
    pub governing_order: AdmittedGoverningOrderV1,
    /// Project occurrence seeds over selected Package semantic exports.
    #[serde(default)]
    pub semantic: SemanticSelection,
    /// Normalized private narrowing of Package-owned capability procedure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_narrowing: Option<CapabilityNarrowingV1>,
    /// Project-local provider bindings for neutral capability classes.
    #[serde(default)]
    pub capability_bindings: BTreeMap<CapabilityIdV1, CapabilityBinding>,
    /// Owner-qualified private exposure and mechanical operation bindings.
    #[serde(default)]
    #[schemars(extend("uniqueItems" = true))]
    pub operation_bindings: Vec<OperationBindingEntryV1>,
    /// Private lifecycle mechanics admitted for this profile.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifecycle: Vec<AdmittedLifecycleBehaviorV1>,
    /// Where the volatile forbidden-output-token set is resolved from.
    #[serde(default)]
    pub forbidden_token_authority: Vec<String>,
    /// The typed session model, when the execution workspace declares one.
    /// Part of the digested body, so a session change moves `profile_digest`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionModel>,
    /// SHA-256 identity of the complete portable profile body.
    pub profile_digest: ProfileDigest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectiveProjectProfileWireV1 {
    schema: SchemaId,
    schema_sha256: crate::records::SchemaDigest,
    schema_version: u32,
    workspace_id: WorkspaceId,
    project_id: LogicalProjectIdV1,
    execution_workspace_id: LogicalExecutionWorkspaceIdV1,
    execution_paths: Vec<ExecutionPathSelectionV1>,
    run_policy: RunPolicyEntry,
    package_closure: Vec<PackageReleaseV1>,
    normative_pack_closure: Vec<NormativePackReleaseV1>,
    package_semantic_ir_digest: PackageSemanticIrDigest,
    authority_root_ids: Vec<String>,
    repositories: Vec<ProfileRepository>,
    governing_order: AdmittedGoverningOrderV1,
    #[serde(default)]
    semantic: SemanticSelection,
    #[serde(default)]
    capability_narrowing: Option<CapabilityNarrowingV1>,
    #[serde(default)]
    capability_bindings: BTreeMap<CapabilityIdV1, CapabilityBinding>,
    #[serde(default)]
    operation_bindings: Vec<OperationBindingEntryV1>,
    #[serde(default)]
    lifecycle: Vec<AdmittedLifecycleBehaviorV1>,
    #[serde(default)]
    forbidden_token_authority: Vec<String>,
    #[serde(default)]
    session: Option<SessionModel>,
    profile_digest: ProfileDigest,
}

impl<'de> Deserialize<'de> for EffectiveProjectProfileV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = EffectiveProjectProfileWireV1::deserialize(deserializer)?;
        let value = Self {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            workspace_id: wire.workspace_id,
            project_id: wire.project_id,
            execution_workspace_id: wire.execution_workspace_id,
            execution_paths: wire.execution_paths,
            run_policy: wire.run_policy,
            package_closure: wire.package_closure,
            normative_pack_closure: wire.normative_pack_closure,
            package_semantic_ir_digest: wire.package_semantic_ir_digest,
            authority_root_ids: wire.authority_root_ids,
            repositories: wire.repositories,
            governing_order: wire.governing_order,
            semantic: wire.semantic,
            capability_narrowing: wire.capability_narrowing,
            capability_bindings: wire.capability_bindings,
            operation_bindings: wire.operation_bindings,
            lifecycle: wire.lifecycle,
            forbidden_token_authority: wire.forbidden_token_authority,
            session: wire.session,
            profile_digest: wire.profile_digest,
        };
        value.verify().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

/// Unsealed construction input for [`EffectiveProjectProfileV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EffectiveProjectProfilePreimageV1 {
    /// Stable wire identity of the effective-profile major.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Effective-profile schema major.
    pub schema_version: u32,
    /// Principal workspace that owns the selected desired graph.
    pub workspace_id: WorkspaceId,
    /// Canonical logical project resolved into this profile.
    pub project_id: LogicalProjectIdV1,
    /// Canonical execution workspace selected for this compilation.
    pub execution_workspace_id: LogicalExecutionWorkspaceIdV1,
    /// Exact neutral execution-path selections admitted for the workspace.
    pub execution_paths: Vec<ExecutionPathSelectionV1>,
    /// Exact private lifecycle policy selected by the project.
    pub run_policy: RunPolicyEntry,
    /// Selected package-lock entries.
    pub package_closure: Vec<PackageReleaseV1>,
    /// Exact selected normative Pack closure.
    pub normative_pack_closure: Vec<NormativePackReleaseV1>,
    /// Exact selected and admitted Package semantic closure.
    pub package_semantic_ir_digest: PackageSemanticIrDigest,
    /// Logical authority-root identities.
    pub authority_root_ids: Vec<String>,
    /// Effective materialization order.
    pub repositories: Vec<ProfileRepository>,
    /// Canonically normalized scoped governing relations.
    pub governing_order: AdmittedGoverningOrderV1,
    /// Project selection over semantic catalogs.
    #[serde(default)]
    pub semantic: SemanticSelection,
    /// Normalized private narrowing of Package-owned capability procedure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_narrowing: Option<CapabilityNarrowingV1>,
    /// Project-local provider bindings.
    #[serde(default)]
    pub capability_bindings: BTreeMap<CapabilityIdV1, CapabilityBinding>,
    /// Private exposure and mechanical operation bindings.
    #[serde(default)]
    pub operation_bindings: Vec<OperationBindingEntryV1>,
    /// Private lifecycle mechanics admitted for this profile.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifecycle: Vec<AdmittedLifecycleBehaviorV1>,
    /// Authorities for volatile forbidden-output tokens.
    #[serde(default)]
    pub forbidden_token_authority: Vec<String>,
    /// Typed session and activation model, when selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionModel>,
}

/// Failure to validate and canonically seal an effective profile.
#[derive(Debug, thiserror::Error)]
pub enum EffectiveProjectProfileSealErrorV1 {
    /// The public record does not carry the exact initial-v1 schema coordinate.
    #[error("effective profile schema coordinate is not initial-v1")]
    SchemaCoordinate,
    /// The selected Package or normative Pack frontier is not admitted.
    #[error(transparent)]
    ReleaseClosure(#[from] ReleaseClosureErrorV1),
    /// Scoped governing relations violate their admitted algebra.
    #[error(transparent)]
    GoverningAlgebra(#[from] GoverningAlgebraErrorV1),
    /// The selected private run policy is internally contradictory.
    #[error(transparent)]
    RunPolicy(#[from] InvalidRunPolicy),
    /// One admitted lifecycle behavior contradicts its deterministic grammar.
    #[error("lifecycle behavior at index {index} is invalid: {source}")]
    Lifecycle {
        /// Zero-based behavior position in the sealed profile.
        index: usize,
        /// Typed lifecycle grammar failure.
        #[source]
        source: InvalidLifecycleBehavior,
    },
    /// Two private bindings claim the same Package operation.
    #[error("effective profile repeats operation binding {0:?}")]
    DuplicateOperationBinding(PackageResourceRefV1),
    /// Neutral execution-path selections repeat an identity or are out of order.
    #[error("effective profile execution paths are not a canonical identity set")]
    ExecutionPathOrder,
    /// Two effective repository records repeat one canonical repository identity.
    #[error("effective profile repeats repository {0}")]
    DuplicateRepository(LogicalRepositoryIdV1),
    /// Logical authority-root identities repeat or are out of canonical order.
    #[error("effective profile authority roots are not a canonical identity set")]
    AuthorityRootOrder,
    /// The claimed canonical profile identity differs from its validated body.
    #[error("effective profile digest does not match its canonical preimage")]
    DigestMismatch,
    /// The admitted profile body cannot be represented by canonical JSON.
    #[error(transparent)]
    Canon(#[from] crate::canon::CanonError),
}

impl EffectiveProjectProfileV1 {
    /// Seal a complete portable profile with its canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`EffectiveProjectProfileSealErrorV1`] when either exact release
    /// closure is unbounded/non-canonical or the admitted body cannot satisfy
    /// canonical JSON.
    pub fn seal(
        preimage: EffectiveProjectProfilePreimageV1,
    ) -> Result<Self, EffectiveProjectProfileSealErrorV1> {
        validate_effective_profile(&preimage)?;
        let profile_digest = ProfileDigest::hash_canonical(&preimage)?;
        let EffectiveProjectProfilePreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            workspace_id,
            project_id,
            execution_workspace_id,
            execution_paths,
            run_policy,
            package_closure,
            normative_pack_closure,
            package_semantic_ir_digest,
            authority_root_ids,
            repositories,
            governing_order,
            semantic,
            capability_narrowing,
            capability_bindings,
            operation_bindings,
            lifecycle,
            forbidden_token_authority,
            session,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            workspace_id,
            project_id,
            execution_workspace_id,
            execution_paths,
            run_policy,
            package_closure,
            normative_pack_closure,
            package_semantic_ir_digest,
            authority_root_ids,
            repositories,
            governing_order,
            semantic,
            capability_narrowing,
            capability_bindings,
            operation_bindings,
            lifecycle,
            forbidden_token_authority,
            session,
            profile_digest,
        })
    }

    /// Clone the unhashed profile body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> EffectiveProjectProfilePreimageV1 {
        EffectiveProjectProfilePreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            workspace_id: self.workspace_id.clone(),
            project_id: self.project_id.clone(),
            execution_workspace_id: self.execution_workspace_id.clone(),
            execution_paths: self.execution_paths.clone(),
            run_policy: self.run_policy.clone(),
            package_closure: self.package_closure.clone(),
            normative_pack_closure: self.normative_pack_closure.clone(),
            package_semantic_ir_digest: self.package_semantic_ir_digest.clone(),
            authority_root_ids: self.authority_root_ids.clone(),
            repositories: self.repositories.clone(),
            governing_order: self.governing_order.clone(),
            semantic: self.semantic.clone(),
            capability_narrowing: self.capability_narrowing.clone(),
            capability_bindings: self.capability_bindings.clone(),
            operation_bindings: self.operation_bindings.clone(),
            lifecycle: self.lifecycle.clone(),
            forbidden_token_authority: self.forbidden_token_authority.clone(),
            session: self.session.clone(),
        }
    }

    /// Recompute the canonical identity of this profile's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_digest(&self) -> Result<ProfileDigest, crate::canon::CanonError> {
        ProfileDigest::hash_canonical(&self.to_preimage())
    }

    /// Revalidate a deserialized profile and its canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`EffectiveProjectProfileSealErrorV1`] for the same structural
    /// failure rejected by [`Self::seal`], or when the retained digest differs.
    pub fn verify(&self) -> Result<(), EffectiveProjectProfileSealErrorV1> {
        let preimage = self.to_preimage();
        validate_effective_profile(&preimage)?;
        let observed = ProfileDigest::hash_canonical(&preimage)?;
        if observed == self.profile_digest {
            Ok(())
        } else {
            Err(EffectiveProjectProfileSealErrorV1::DigestMismatch)
        }
    }
}

fn validate_effective_profile(
    profile: &EffectiveProjectProfilePreimageV1,
) -> Result<(), EffectiveProjectProfileSealErrorV1> {
    if profile.schema.as_str() != EFFECTIVE_PROJECT_PROFILE_SCHEMA
        || profile.schema_version != EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION
    {
        return Err(EffectiveProjectProfileSealErrorV1::SchemaCoordinate);
    }
    profile.run_policy.validate_contract()?;
    validate_release_closures(&profile.package_closure, &profile.normative_pack_closure)?;
    profile.governing_order.validate()?;
    for pair in profile.execution_paths.windows(2) {
        if pair[0].id >= pair[1].id {
            return Err(EffectiveProjectProfileSealErrorV1::ExecutionPathOrder);
        }
    }
    for pair in profile.authority_root_ids.windows(2) {
        if pair[0] >= pair[1] {
            return Err(EffectiveProjectProfileSealErrorV1::AuthorityRootOrder);
        }
    }
    let mut repositories = BTreeSet::new();
    for repository in &profile.repositories {
        if !repositories.insert(&repository.id) {
            return Err(EffectiveProjectProfileSealErrorV1::DuplicateRepository(
                repository.id.clone(),
            ));
        }
    }
    let mut operations = BTreeSet::new();
    for binding in &profile.operation_bindings {
        if !operations.insert(&binding.operation) {
            return Err(
                EffectiveProjectProfileSealErrorV1::DuplicateOperationBinding(
                    binding.operation.clone(),
                ),
            );
        }
    }
    for (index, behavior) in profile.lifecycle.iter().enumerate() {
        behavior
            .validate_contract()
            .map_err(|source| EffectiveProjectProfileSealErrorV1::Lifecycle { index, source })?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/internal/records/computed/profile.rs"]
mod checked_deserialization_tests;

/// Machine resolution of one effective profile. Paths and executable bindings
/// move `instance_digest` without changing portable `profile_digest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedProjectInstanceV1 {
    /// Stable wire identity of the resolved-instance major.
    #[schemars(with = "String", extend("const" = RESOLVED_PROJECT_INSTANCE_SCHEMA))]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Digest of the portable profile resolved onto this machine.
    pub profile_digest: ProfileDigest,
    /// Stable machine-binding identity, not a hostname or repository ID.
    pub machine_id: String,
    /// Logical repository IDs to canonical absolute checkout paths.
    pub repositories: BTreeMap<LogicalRepositoryIdV1, String>,
    /// Explicit machine authority for out-of-checkout Git metadata used by
    /// linked worktrees. Ordinary clones omit this map.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub git_metadata_roots: BTreeMap<LogicalRepositoryIdV1, String>,
    /// Selected provider-native projections over canonical repository identities.
    ///
    /// The provider owns each opaque native coordinate. Presence here records
    /// only the selected machine projection; it proves no provider readiness,
    /// authority, mutation, or read-back.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_repositories: Vec<ProviderRepositoryProjectionV1>,
    /// Logical authority-root IDs to canonical absolute machine paths.
    pub authority_roots: BTreeMap<String, String>,
    /// Effective user-local mode defaults not overridden by the project.
    /// These are instance data and never move the portable profile digest.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub user_mode_defaults: BTreeMap<ModeAxis, PackageResourceRefV1>,
    /// The machine-local Pale executable selected by path, byte digest, version,
    /// and interface protocol. A binding change moves `instance_digest` but
    /// never `profile_digest`. Presence does not prove resident liveness.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub surface_executable: Option<SurfaceExecutableBinding>,
    /// Selected native harnesses resolved to exact machine-local executables.
    /// Unrelated machine bindings remain outside this instance and its digest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(extend("uniqueItems" = true))]
    pub native_harnesses: Vec<NativeHarnessBinding>,
    /// Machine-bound ephemeral session storage. This changes the instance
    /// digest, never the portable profile digest.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub session_state: Option<SessionStateBinding>,
    /// Recoverable, retention-bounded evidence storage. This is machine-local
    /// instance data and is neither ephemeral session continuity nor immutable
    /// materialization lineage.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub durable_evidence: Option<DurableEvidenceBinding>,
    /// Durable machine-local ownership lineage. This is deliberately distinct
    /// from expiring hook-session state.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub materialization_lineage: Option<MaterializationLineageBinding>,
    /// Canonical identity of the complete machine-resolved instance body.
    pub instance_digest: InstanceDigest,
}

/// Unsealed construction input for [`ResolvedProjectInstanceV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedProjectInstancePreimageV1 {
    /// Stable wire identity of the resolved-instance major.
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Digest of the portable profile resolved onto this machine.
    pub profile_digest: ProfileDigest,
    /// Stable machine-binding identity.
    pub machine_id: String,
    /// Logical repository identities to canonical checkout paths.
    pub repositories: BTreeMap<LogicalRepositoryIdV1, String>,
    /// Explicit machine authority for linked-worktree Git metadata.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub git_metadata_roots: BTreeMap<LogicalRepositoryIdV1, String>,
    /// Selected provider-native repository projections.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_repositories: Vec<ProviderRepositoryProjectionV1>,
    /// Logical authority-root identities to canonical paths.
    pub authority_roots: BTreeMap<String, String>,
    /// Effective user-local mode defaults.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub user_mode_defaults: BTreeMap<ModeAxis, PackageResourceRefV1>,
    /// Selected machine-local Pale executable, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_executable: Option<SurfaceExecutableBinding>,
    /// Selected native harnesses and their executable identities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub native_harnesses: Vec<NativeHarnessBinding>,
    /// Machine-bound ephemeral session storage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_state: Option<SessionStateBinding>,
    /// Recoverable, retention-bounded evidence storage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub durable_evidence: Option<DurableEvidenceBinding>,
    /// Durable machine-local ownership lineage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materialization_lineage: Option<MaterializationLineageBinding>,
}

/// Canonical machine-only projection of [`ResolvedProjectInstanceV1`].
///
/// The portable profile coordinate and the instance's schema/self-digest are
/// intentionally excluded. The resulting identity therefore answers only
/// whether the resolved machine overlay changed; it cannot substitute for the
/// complete [`InstanceDigest`].
#[derive(Serialize)]
struct ResolvedMachineOverlayPreimageV1<'a> {
    domain: &'static str,
    machine_id: &'a str,
    repositories: &'a BTreeMap<LogicalRepositoryIdV1, String>,
    git_metadata_roots: &'a BTreeMap<LogicalRepositoryIdV1, String>,
    provider_repositories: &'a [ProviderRepositoryProjectionV1],
    authority_roots: &'a BTreeMap<String, String>,
    user_mode_defaults: &'a BTreeMap<ModeAxis, PackageResourceRefV1>,
    surface_executable: &'a Option<SurfaceExecutableBinding>,
    native_harnesses: &'a [NativeHarnessBinding],
    session_state: &'a Option<SessionStateBinding>,
    durable_evidence: &'a Option<DurableEvidenceBinding>,
    materialization_lineage: &'a Option<MaterializationLineageBinding>,
}

impl ResolvedProjectInstanceV1 {
    /// Seal a complete machine-resolved instance with its canonical identity.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the preimage cannot satisfy
    /// the canonical JSON contract.
    pub fn seal(
        preimage: ResolvedProjectInstancePreimageV1,
    ) -> Result<Self, crate::canon::CanonError> {
        let instance_digest = InstanceDigest::hash_canonical(&preimage)?;
        let ResolvedProjectInstancePreimageV1 {
            schema,
            schema_sha256,
            profile_digest,
            machine_id,
            repositories,
            git_metadata_roots,
            provider_repositories,
            authority_roots,
            user_mode_defaults,
            surface_executable,
            native_harnesses,
            session_state,
            durable_evidence,
            materialization_lineage,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            profile_digest,
            machine_id,
            repositories,
            git_metadata_roots,
            provider_repositories,
            authority_roots,
            user_mode_defaults,
            surface_executable,
            native_harnesses,
            session_state,
            durable_evidence,
            materialization_lineage,
            instance_digest,
        })
    }

    /// Clone the unhashed instance body for explicit amendment and resealing.
    #[must_use]
    pub fn to_preimage(&self) -> ResolvedProjectInstancePreimageV1 {
        ResolvedProjectInstancePreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            profile_digest: self.profile_digest.clone(),
            machine_id: self.machine_id.clone(),
            repositories: self.repositories.clone(),
            git_metadata_roots: self.git_metadata_roots.clone(),
            provider_repositories: self.provider_repositories.clone(),
            authority_roots: self.authority_roots.clone(),
            user_mode_defaults: self.user_mode_defaults.clone(),
            surface_executable: self.surface_executable.clone(),
            native_harnesses: self.native_harnesses.clone(),
            session_state: self.session_state.clone(),
            durable_evidence: self.durable_evidence.clone(),
            materialization_lineage: self.materialization_lineage.clone(),
        }
    }

    /// Compute the domain-separated identity of this instance's machine overlay.
    ///
    /// This projection binds paths, machine-local executable selections,
    /// session/evidence locations, and adapter bindings. It deliberately omits
    /// [`Self::profile_digest`] and [`Self::instance_digest`], keeping the
    /// portable profile, complete instance, and machine overlay as three
    /// independently typed compilation coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] if the machine-only projection
    /// cannot be represented as canonical JSON.
    pub fn machine_overlay_digest(&self) -> Result<MachineOverlayDigest, crate::canon::CanonError> {
        MachineOverlayDigest::hash_canonical(&ResolvedMachineOverlayPreimageV1 {
            domain: "pale.resolved-machine-overlay.v1",
            machine_id: &self.machine_id,
            repositories: &self.repositories,
            git_metadata_roots: &self.git_metadata_roots,
            provider_repositories: &self.provider_repositories,
            authority_roots: &self.authority_roots,
            user_mode_defaults: &self.user_mode_defaults,
            surface_executable: &self.surface_executable,
            native_harnesses: &self.native_harnesses,
            session_state: &self.session_state,
            durable_evidence: &self.durable_evidence,
            materialization_lineage: &self.materialization_lineage,
        })
    }

    /// Recompute the canonical identity of this instance's unhashed body.
    ///
    /// # Errors
    ///
    /// Returns [`crate::canon::CanonError`] when the body cannot satisfy the
    /// canonical JSON contract.
    pub fn recompute_digest(&self) -> Result<InstanceDigest, crate::canon::CanonError> {
        InstanceDigest::hash_canonical(&self.to_preimage())
    }
}
