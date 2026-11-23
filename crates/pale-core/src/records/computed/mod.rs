//! Computed and digested output records, grouped by schema family.

mod executable_identity;
mod hook;
mod materialization;
mod profile;
mod projection;
mod result;

pub use executable_identity::{SurfaceExecutableIdentityV1, SURFACE_INTERFACE_PROTOCOL};
pub use hook::{HookContextV1, RepositoryVerifierBinding, SealedRunMode};
pub use materialization::{
    AgentSurfacePlanPreimageV1, AgentSurfacePlanSealErrorV1, AgentSurfacePlanV1, BaselineEntry,
    BaselineStateV1, DeferredEffect, ExistingMachineLocalFileV1, FilesystemDeviceIdV1,
    FilesystemInodeIdV1, GeneratedAgentSurfaceManifestPreimageV1, GeneratedAgentSurfaceManifestV1,
    InvalidUnixFileMode, MachineLocalDirectoryIdentity, MachineLocalEffectKind,
    MachineLocalEffectTransitionV1, MachineLocalEffectV1, MachineLocalFileIdentityV1,
    MachineLocalRootIdentity, MaterializationLineagePreimageV1, MaterializationLineageV1,
    MaterializationPlanPreimageV1, MaterializationPlanV1, MaterializationStatePreimageV1,
    MaterializationStateTransitionV1, MaterializationStateV1, Operation, PlannedFileV1,
    PriorMaterializationStateV1, SurfaceDigest, SurfaceOperationTransitionV1, UnixFileMode,
    AGENT_SURFACE_PLAN_V1_SCHEMA, MATERIALIZATION_LINEAGE_SCHEMA,
    MATERIALIZATION_LINEAGE_SCHEMA_VERSION, MATERIALIZATION_PLAN_V1_SCHEMA,
    MATERIALIZATION_PLAN_V1_SCHEMA_VERSION, MATERIALIZATION_STATE_PATH,
    MATERIALIZATION_STATE_SCHEMA, MATERIALIZATION_STATE_SCHEMA_VERSION,
};
pub use profile::{
    EffectiveProjectProfilePreimageV1, EffectiveProjectProfileSealErrorV1,
    EffectiveProjectProfileV1, ProfileRepository, ResolvedProjectInstancePreimageV1,
    ResolvedProjectInstanceV1, EFFECTIVE_PROJECT_PROFILE_SCHEMA,
    EFFECTIVE_PROJECT_PROFILE_SCHEMA_VERSION, RESOLVED_PROJECT_INSTANCE_SCHEMA,
};
pub use projection::{
    harness_injection_path, harness_projection_manifest_path, provider_binding_projection_root,
    HarnessProjectionManifestV1, ManagedFrontDoorDigest, HARNESS_PROJECTION_SCHEMA,
    HARNESS_PROJECTION_SCHEMA_VERSION,
};
pub use result::{
    MachineLocalEffectObservationV1, MachineLocalPostFileV1, MachineLocalReceiptOperationV1,
    MaterializationReceiptPreimageV1, MaterializationReceiptSealErrorV1,
    MaterializationReceiptTransitionV1, MaterializationReceiptV1, ReceiptOperation,
    RunResultPreimageV1, RunResultV1, RunTargetResult, SurfaceCreationObservationV1,
    SurfaceOperationObservationV1, UnchangedDispositionV1, MATERIALIZATION_RECEIPT_V1_SCHEMA,
    MATERIALIZATION_RECEIPT_V1_SCHEMA_VERSION, RUN_RESULT_V1_SCHEMA, RUN_RESULT_V1_SCHEMA_VERSION,
};
