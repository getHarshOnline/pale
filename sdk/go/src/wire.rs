//! Go-owned request records and lossless conversion into `pale-sdk` values.

use base64::Engine as _;
use pale_sdk::{
    AuthoringIngressClass, AuthoringInstancePathSegment, AuthoringPathSegment,
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticProjection, DiagnosticRetry,
    DiagnosticStage, EffectiveProfileCompilationRequest, EffectiveProfileExplanationHandle,
    EffectiveProfilePageRequest, EffectiveProfileSection, MachineLocalEffectCandidate,
    ObservationCancellation, ObservationCategory,
    ObservationDeltaRequest as ApiObservationDeltaRequest,
    ObservationFilter as ApiObservationFilter, ObservationFreshnessClass, ObservationLensLimits,
    ObservationLensRequest as ApiObservationLensRequest, ObservationPageLimits,
    ObservationPageRequest as ApiObservationPageRequest, OneShotBaselineEntry,
    OneShotBaselineObservation, OneShotCompilationRequest, OneShotPlanKind, OneShotPlanningRequest,
    OneShotPriorMaterialization, OneShotPrivateInputs, PortableWorkspaceDocument,
    PortableWorkspaceRequest, PortableWorkspaceSubjectSelector, StandaloneAuthoringRole,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalConfigurationImprovementRequest {
    pub baseline_manifest: String,
    pub candidate_manifest: String,
    pub project_id: String,
    pub execution_workspace_id: String,
    pub machine_id: String,
}

/// Scope-specific private input; semantic validation belongs to the API.
#[derive(Deserialize)]
#[serde(
    tag = "scope",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum LocalControlStorageInspectionRequest {
    Principal {
        runtime_state_root: String,
        principal_id: String,
        logical_store_id: String,
        control_incarnation_id: String,
    },
    Deployment {
        runtime_state_root: String,
        workspace_id: String,
        deployment_binding_ref: String,
        deployment_binding_sha256: String,
        logical_store_id: String,
        control_incarnation_id: String,
    },
}

impl From<LocalControlStorageInspectionRequest> for pale_sdk::LocalControlStorageInspectionRequest {
    fn from(value: LocalControlStorageInspectionRequest) -> Self {
        match value {
            LocalControlStorageInspectionRequest::Principal {
                runtime_state_root,
                principal_id,
                logical_store_id,
                control_incarnation_id,
            } => Self::principal(
                runtime_state_root,
                principal_id,
                logical_store_id,
                control_incarnation_id,
            ),
            LocalControlStorageInspectionRequest::Deployment {
                runtime_state_root,
                workspace_id,
                deployment_binding_ref,
                deployment_binding_sha256,
                logical_store_id,
                control_incarnation_id,
            } => Self::deployment(
                runtime_state_root,
                workspace_id,
                deployment_binding_ref,
                deployment_binding_sha256,
                logical_store_id,
                control_incarnation_id,
            ),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Document {
    pub(crate) label: String,
    pub(crate) bytes: String,
}

impl Document {
    pub(crate) fn decode(self) -> Result<(String, Vec<u8>), crate::BridgeFailure> {
        decode_bytes(&self.bytes).map(|bytes| (self.label, bytes))
    }

    pub(crate) fn compilation(self) -> Result<pale_sdk::CompilationDocument, crate::BridgeFailure> {
        self.decode()
            .map(|(label, bytes)| pale_sdk::CompilationDocument::new(label, bytes))
    }
}

pub(crate) fn decode_bytes(value: &str) -> Result<Vec<u8>, crate::BridgeFailure> {
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|error| crate::BridgeFailure::request(format!("invalid base64 bytes: {error}")))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SchemaRequest {
    pub(crate) schema_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ContractDocumentRequest {
    pub(crate) schema_id: String,
    pub(crate) bytes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BytesRequest {
    pub(crate) bytes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HandleRequest {
    pub(crate) handle: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Stage {
    InputAdmission,
    ContractValidation,
    SemanticAdmission,
    PackageLoading,
    WorkspaceLoading,
    Compilation,
    Rendering,
    OperationAdmission,
    ResidentAdmission,
    ResidentRecovery,
    Planning,
    Materialization,
    NativeAction,
    HostObservation,
    Internal,
}

impl From<Stage> for DiagnosticStage {
    fn from(value: Stage) -> Self {
        match value {
            Stage::InputAdmission => Self::InputAdmission,
            Stage::ContractValidation => Self::ContractValidation,
            Stage::SemanticAdmission => Self::SemanticAdmission,
            Stage::PackageLoading => Self::PackageLoading,
            Stage::WorkspaceLoading => Self::WorkspaceLoading,
            Stage::Compilation => Self::Compilation,
            Stage::Rendering => Self::Rendering,
            Stage::OperationAdmission => Self::OperationAdmission,
            Stage::ResidentAdmission => Self::ResidentAdmission,
            Stage::ResidentRecovery => Self::ResidentRecovery,
            Stage::Planning => Self::Planning,
            Stage::Materialization => Self::Materialization,
            Stage::NativeAction => Self::NativeAction,
            Stage::HostObservation => Self::HostObservation,
            Stage::Internal => Self::Internal,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Disposition {
    OperationalFailure,
    InvalidInput,
    Blocked,
    InternalFailure,
    EffectOutcomeUnknown,
}

impl From<Disposition> for DiagnosticDisposition {
    fn from(value: Disposition) -> Self {
        match value {
            Disposition::OperationalFailure => Self::OperationalFailure,
            Disposition::InvalidInput => Self::InvalidInput,
            Disposition::Blocked => Self::Blocked,
            Disposition::InternalFailure => Self::InternalFailure,
            Disposition::EffectOutcomeUnknown => Self::EffectOutcomeUnknown,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Retry {
    NotIndicated,
    AfterCorrection,
    AfterStateChange,
    ReconcileFirst,
}

impl From<Retry> for DiagnosticRetry {
    fn from(value: Retry) -> Self {
        match value {
            Retry::NotIndicated => Self::NotIndicated,
            Retry::AfterCorrection => Self::AfterCorrection,
            Retry::AfterStateChange => Self::AfterStateChange,
            Retry::ReconcileFirst => Self::ReconcileFirst,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum EffectCertainty {
    NotApplicable,
    NotStarted,
    Confirmed,
    OutcomeUnknown,
}

impl From<EffectCertainty> for DiagnosticEffectCertainty {
    fn from(value: EffectCertainty) -> Self {
        match value {
            EffectCertainty::NotApplicable => Self::NotApplicable,
            EffectCertainty::NotStarted => Self::NotStarted,
            EffectCertainty::Confirmed => Self::Confirmed,
            EffectCertainty::OutcomeUnknown => Self::OutcomeUnknown,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DiagnosticRequest {
    pub(crate) code: String,
    pub(crate) stage: Stage,
    pub(crate) disposition: Disposition,
    pub(crate) summary: String,
    pub(crate) retry: Retry,
    pub(crate) effect: EffectCertainty,
    #[serde(default)]
    pub(crate) causes: Vec<String>,
    #[serde(default)]
    pub(crate) omitted_causes: u32,
    #[serde(default)]
    pub(crate) help: Vec<String>,
}

impl DiagnosticRequest {
    pub(crate) fn into_api(self) -> DiagnosticProjection {
        DiagnosticProjection::new(
            self.code,
            self.stage.into(),
            self.disposition.into(),
            self.summary,
            self.retry.into(),
            self.effect.into(),
        )
        .with_causes(self.causes, self.omitted_causes)
        .with_help(self.help)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum IngressClass {
    Workspace,
    Private,
    Package,
}

impl From<IngressClass> for AuthoringIngressClass {
    fn from(value: IngressClass) -> Self {
        match value {
            IngressClass::Workspace => Self::Workspace,
            IngressClass::Private => Self::Private,
            IngressClass::Package => Self::Package,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AuthoringRole {
    WorkspaceManifest,
    WorkspaceModule,
    WorkspaceLanguagePrimitives,
    ResponsibilityDerivation,
    OrganizationalTranslation,
    ExperienceComposition,
    WorkspaceResolutionLock,
    WorkspaceUniverseLock,
    Capabilities,
    MachineBinding,
    DistributionLock,
    PackageManifest,
    PackageDoctrine,
    PackageOperations,
    PackageRoles,
    PackageStacks,
    PackageRunModes,
    PackagePolicyFamily,
    PackageGoverningOrder,
    PackageCapabilityPolicy,
    PackageToolRecipe,
}

impl From<AuthoringRole> for StandaloneAuthoringRole {
    fn from(value: AuthoringRole) -> Self {
        match value {
            AuthoringRole::WorkspaceManifest => Self::WorkspaceManifest,
            AuthoringRole::WorkspaceModule => Self::WorkspaceModule,
            AuthoringRole::WorkspaceLanguagePrimitives => Self::WorkspaceLanguagePrimitives,
            AuthoringRole::ResponsibilityDerivation => Self::ResponsibilityDerivation,
            AuthoringRole::OrganizationalTranslation => Self::OrganizationalTranslation,
            AuthoringRole::ExperienceComposition => Self::ExperienceComposition,
            AuthoringRole::WorkspaceResolutionLock => Self::WorkspaceResolutionLock,
            AuthoringRole::WorkspaceUniverseLock => Self::WorkspaceUniverseLock,
            AuthoringRole::Capabilities => Self::Capabilities,
            AuthoringRole::MachineBinding => Self::MachineBinding,
            AuthoringRole::DistributionLock => Self::DistributionLock,
            AuthoringRole::PackageManifest => Self::PackageManifest,
            AuthoringRole::PackageDoctrine => Self::PackageDoctrine,
            AuthoringRole::PackageOperations => Self::PackageOperations,
            AuthoringRole::PackageRoles => Self::PackageRoles,
            AuthoringRole::PackageStacks => Self::PackageStacks,
            AuthoringRole::PackageRunModes => Self::PackageRunModes,
            AuthoringRole::PackagePolicyFamily => Self::PackagePolicyFamily,
            AuthoringRole::PackageGoverningOrder => Self::PackageGoverningOrder,
            AuthoringRole::PackageCapabilityPolicy => Self::PackageCapabilityPolicy,
            AuthoringRole::PackageToolRecipe => Self::PackageToolRecipe,
        }
    }
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub(crate) enum StructuralPathSegment {
    Field(String),
    ArrayItem,
}

impl From<StructuralPathSegment> for AuthoringPathSegment {
    fn from(value: StructuralPathSegment) -> Self {
        match value {
            StructuralPathSegment::Field(field) => Self::Field(field),
            StructuralPathSegment::ArrayItem => Self::ArrayItem,
        }
    }
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub(crate) enum InstancePathSegment {
    Field(String),
    Index(usize),
}

impl From<InstancePathSegment> for AuthoringInstancePathSegment {
    fn from(value: InstancePathSegment) -> Self {
        match value {
            InstancePathSegment::Field(field) => Self::Field(field),
            InstancePathSegment::Index(index) => Self::Index(index),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StandaloneValidationRequest {
    pub(crate) role: AuthoringRole,
    pub(crate) source_label: String,
    pub(crate) bytes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StandaloneInspectionRequest {
    pub(crate) role: AuthoringRole,
    pub(crate) path: Vec<StructuralPathSegment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuthoringInspectionRequest {
    pub(crate) handle: u64,
    pub(crate) source_label: String,
    pub(crate) path: Vec<StructuralPathSegment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AuthoringNavigationRequest {
    pub(crate) handle: u64,
    pub(crate) source_label: String,
    pub(crate) path: Vec<InstancePathSegment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PortableRequest {
    pub(crate) root_manifest: String,
    pub(crate) documents: Vec<Document>,
}

impl PortableRequest {
    pub(crate) fn into_api(self) -> Result<PortableWorkspaceRequest, crate::BridgeFailure> {
        self.documents
            .into_iter()
            .map(Document::decode)
            .collect::<Result<Vec<_>, _>>()
            .map(|documents| {
                PortableWorkspaceRequest::new(
                    self.root_manifest,
                    documents
                        .into_iter()
                        .map(|(label, bytes)| PortableWorkspaceDocument::new(label, bytes))
                        .collect(),
                )
            })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ComparisonRequest {
    pub(crate) prior: u64,
    pub(crate) current: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PageRequest {
    pub(crate) handle: u64,
    pub(crate) offset: u32,
    pub(crate) limit: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkspaceExplanationRequest {
    pub(crate) handle: u64,
    pub(crate) subject: WorkspaceSubject,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum WorkspaceSubject {
    Source {
        source_id: String,
    },
    Component {
        component_id: String,
    },
    Module {
        module_id: String,
    },
    Declaration {
        owner_module_id: String,
        namespace_id: String,
        declaration_id: String,
    },
    Import {
        importer_module_id: String,
        upstream_module_id: String,
        upstream_release_id: String,
        export_id: String,
        source_namespace_id: String,
        local_namespace_id: String,
    },
    Extension {
        owner_module_id: String,
        extension_namespace_id: String,
        target_namespace_id: String,
        target_declaration_id: String,
    },
    Policy {
        owner_module_id: String,
        namespace_id: String,
        declaration_id: String,
    },
    ResponsibilityClosure,
    OrganizationalTranslationClosure,
    ExperienceComposition,
    Resource {
        owner_module_id: String,
        resource_id: String,
    },
    ResolutionLock,
    UniverseLock,
    Workspace {
        workspace_id: String,
    },
}

impl From<WorkspaceSubject> for PortableWorkspaceSubjectSelector {
    fn from(value: WorkspaceSubject) -> Self {
        match value {
            WorkspaceSubject::Source { source_id } => Self::Source { source_id },
            WorkspaceSubject::Component { component_id } => Self::Component { component_id },
            WorkspaceSubject::Module { module_id } => Self::Module { module_id },
            WorkspaceSubject::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            WorkspaceSubject::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => Self::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            },
            WorkspaceSubject::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => Self::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            },
            WorkspaceSubject::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            WorkspaceSubject::ResponsibilityClosure => Self::ResponsibilityClosure,
            WorkspaceSubject::OrganizationalTranslationClosure => {
                Self::OrganizationalTranslationClosure
            }
            WorkspaceSubject::ExperienceComposition => Self::ExperienceComposition,
            WorkspaceSubject::Resource {
                owner_module_id,
                resource_id,
            } => Self::Resource {
                owner_module_id,
                resource_id,
            },
            WorkspaceSubject::ResolutionLock => Self::ResolutionLock,
            WorkspaceSubject::UniverseLock => Self::UniverseLock,
            WorkspaceSubject::Workspace { workspace_id } => Self::Workspace { workspace_id },
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PrivateInputs {
    pub(crate) capabilities: Document,
    pub(crate) machine_binding: Document,
    pub(crate) distribution_lock: Document,
    #[serde(default)]
    pub(crate) lifecycle_guidance: Vec<Document>,
}

impl PrivateInputs {
    pub(crate) fn into_api(self) -> Result<OneShotPrivateInputs, crate::BridgeFailure> {
        Ok(OneShotPrivateInputs::new(
            self.capabilities.compilation()?,
            self.machine_binding.compilation()?,
            self.distribution_lock.compilation()?,
            self.lifecycle_guidance
                .into_iter()
                .map(Document::compilation)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EffectiveProfileRequest {
    pub(crate) workspace: u64,
    pub(crate) project_id: String,
    pub(crate) execution_workspace_id: String,
    pub(crate) machine_id: String,
    pub(crate) private_inputs: PrivateInputs,
    pub(crate) package_documents: Vec<Document>,
}

impl EffectiveProfileRequest {
    pub(crate) fn into_api(
        self,
    ) -> Result<EffectiveProfileCompilationRequest, crate::BridgeFailure> {
        Ok(EffectiveProfileCompilationRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.private_inputs.into_api()?,
            self.package_documents
                .into_iter()
                .map(Document::compilation)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProfileSection {
    IntendedUseCase,
    EffectiveConfiguration,
    CapabilitiesAndOperations,
    MachineRealization,
    GapsAndExplanations,
}

impl From<ProfileSection> for EffectiveProfileSection {
    fn from(value: ProfileSection) -> Self {
        match value {
            ProfileSection::IntendedUseCase => Self::IntendedUseCase,
            ProfileSection::EffectiveConfiguration => Self::EffectiveConfiguration,
            ProfileSection::CapabilitiesAndOperations => Self::CapabilitiesAndOperations,
            ProfileSection::MachineRealization => Self::MachineRealization,
            ProfileSection::GapsAndExplanations => Self::GapsAndExplanations,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfilePageRequest {
    pub(crate) handle: u64,
    pub(crate) section: ProfileSection,
    pub(crate) offset: u32,
    pub(crate) limit: u16,
}

impl ProfilePageRequest {
    pub(crate) fn selector(&self) -> EffectiveProfilePageRequest {
        EffectiveProfilePageRequest::new(self.section.into(), self.offset, self.limit)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileExplanationRequest {
    pub(crate) handle: u64,
    pub(crate) explanation: String,
}

impl ProfileExplanationRequest {
    pub(crate) fn explanation(
        &self,
    ) -> Result<EffectiveProfileExplanationHandle, crate::BridgeFailure> {
        EffectiveProfileExplanationHandle::parse(self.explanation.clone())
            .map_err(|error| crate::BridgeFailure::request(error.to_string()))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileAuthoringJoinRequest {
    pub(crate) handle: u64,
    pub(crate) identity: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActionRequest {
    pub(crate) action_id: String,
    pub(crate) bytes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepositoryActionRequest {
    pub(crate) bytes: String,
    pub(crate) profile_digest: String,
    pub(crate) instance_digest: String,
    pub(crate) repository_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OneShotRequest {
    pub(crate) project_id: String,
    pub(crate) execution_workspace_id: String,
    pub(crate) machine_id: String,
    pub(crate) private_inputs: PrivateInputs,
    pub(crate) package_documents: Vec<Document>,
    pub(crate) workspace: PortableRequest,
}

impl OneShotRequest {
    pub(crate) fn into_api(self) -> Result<OneShotCompilationRequest, crate::BridgeFailure> {
        Ok(OneShotCompilationRequest::new(
            self.project_id,
            self.execution_workspace_id,
            self.machine_id,
            self.private_inputs.into_api()?,
            self.package_documents
                .into_iter()
                .map(Document::compilation)
                .collect::<Result<Vec<_>, _>>()?,
            self.workspace.into_api()?,
        ))
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PlanKind {
    Apply,
    Adopt,
    Remove,
}

impl From<PlanKind> for OneShotPlanKind {
    fn from(value: PlanKind) -> Self {
        match value {
            PlanKind::Apply => Self::Apply,
            PlanKind::Adopt => Self::Adopt,
            PlanKind::Remove => Self::Remove,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum PriorMaterialization {
    Unowned,
    Owned { canonical_state: String },
}

impl PriorMaterialization {
    pub(crate) fn into_api(self) -> Result<OneShotPriorMaterialization, crate::BridgeFailure> {
        match self {
            Self::Unowned => Ok(OneShotPriorMaterialization::Unowned),
            Self::Owned { canonical_state } => Ok(OneShotPriorMaterialization::Owned {
                canonical_state: decode_bytes(&canonical_state)?,
            }),
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum BaselineObservation {
    Absent,
    Present { sha256: String, unix_mode: u32 },
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BaselineEntry {
    pub(crate) relative_path: String,
    pub(crate) observation: BaselineObservation,
}

impl BaselineEntry {
    pub(crate) fn into_api(self) -> OneShotBaselineEntry {
        let observation = match self.observation {
            BaselineObservation::Absent => OneShotBaselineObservation::Absent,
            BaselineObservation::Present { sha256, unix_mode } => {
                OneShotBaselineObservation::Present { sha256, unix_mode }
            }
        };
        OneShotBaselineEntry::new(self.relative_path, observation)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ObservationRequest {
    pub(crate) compilation: u64,
    pub(crate) repository_id: String,
    pub(crate) kind: PlanKind,
    pub(crate) prior: PriorMaterialization,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlanningRequest {
    pub(crate) compilation: u64,
    pub(crate) repository_id: String,
    pub(crate) kind: PlanKind,
    pub(crate) prior: PriorMaterialization,
    pub(crate) baseline: Vec<BaselineEntry>,
}

impl PlanningRequest {
    pub(crate) fn request(self) -> Result<OneShotPlanningRequest, crate::BridgeFailure> {
        Ok(OneShotPlanningRequest::new(
            self.repository_id,
            self.kind.into(),
            self.prior.into_api()?,
            self.baseline
                .into_iter()
                .map(BaselineEntry::into_api)
                .collect(),
        ))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BindPlanRequest {
    pub(crate) candidate: u64,
    pub(crate) effects: Vec<String>,
}

impl BindPlanRequest {
    pub(crate) fn effects(self) -> Result<Vec<MachineLocalEffectCandidate>, crate::BridgeFailure> {
        self.effects
            .into_iter()
            .map(|bytes| decode_bytes(&bytes).map(MachineLocalEffectCandidate::new))
            .collect()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AdmitPlanRequest {
    pub(crate) bound_plan: u64,
    pub(crate) saved_bytes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PackageDistributionRequest {
    pub(crate) documents: Vec<Document>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResidentHookRequest {
    pub(crate) event: String,
    pub(crate) workspace: String,
    pub(crate) repository: String,
    pub(crate) provider_binding: String,
    pub(crate) provider_binding_identity: String,
    pub(crate) payload: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalWorkspaceRequest {
    pub(crate) manifest: String,
    pub(crate) project_id: String,
    pub(crate) execution_workspace_id: String,
    pub(crate) machine_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalActionInputRequest {
    pub(crate) action_id: String,
    pub(crate) root: String,
    pub(crate) relative_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CapturedActionRequest {
    pub(crate) handle: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalRepositoryRequest {
    pub(crate) compilation: u64,
    pub(crate) repository_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalPlanningRequest {
    pub(crate) compilation: u64,
    pub(crate) repository_id: String,
    pub(crate) kind: PlanKind,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalPlanPublicationRequest {
    pub(crate) plan: u64,
    pub(crate) output_directory: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalRunAdmissionRequest {
    pub(crate) compilation: u64,
    pub(crate) kind: PlanKind,
    pub(crate) saved_plan_directory: String,
    pub(crate) evidence_directory: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalRunRequest {
    pub(crate) run: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CurrentExecutableBindingVerificationRequest {
    pub(crate) compilation: u64,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ObservationCategoryRequest {
    OperationExposure,
    CapabilityRequirementDisposition,
    Health,
    Readiness,
    Effect,
    NativeOwner,
    Reconciliation,
    ProcessIntent,
}

impl From<ObservationCategoryRequest> for ObservationCategory {
    fn from(value: ObservationCategoryRequest) -> Self {
        match value {
            ObservationCategoryRequest::OperationExposure => Self::OperationExposure,
            ObservationCategoryRequest::CapabilityRequirementDisposition => {
                Self::CapabilityRequirementDisposition
            }
            ObservationCategoryRequest::Health => Self::Health,
            ObservationCategoryRequest::Readiness => Self::Readiness,
            ObservationCategoryRequest::Effect => Self::Effect,
            ObservationCategoryRequest::NativeOwner => Self::NativeOwner,
            ObservationCategoryRequest::Reconciliation => Self::Reconciliation,
            ObservationCategoryRequest::ProcessIntent => Self::ProcessIntent,
        }
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ObservationFreshnessRequest {
    Fresh,
    Stale,
    Unknown,
}

impl From<ObservationFreshnessRequest> for ObservationFreshnessClass {
    fn from(value: ObservationFreshnessRequest) -> Self {
        match value {
            ObservationFreshnessRequest::Fresh => Self::Fresh,
            ObservationFreshnessRequest::Stale => Self::Stale,
            ObservationFreshnessRequest::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ObservationCancellationRequest {
    Active,
    Cancelled,
}

impl From<ObservationCancellationRequest> for ObservationCancellation {
    fn from(value: ObservationCancellationRequest) -> Self {
        match value {
            ObservationCancellationRequest::Active => Self::Active,
            ObservationCancellationRequest::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ObservationFilterRequest {
    pub(crate) category: Option<ObservationCategoryRequest>,
    pub(crate) subject: Option<u64>,
    pub(crate) freshness: Option<ObservationFreshnessRequest>,
}

impl ObservationFilterRequest {
    pub(crate) fn into_api(
        self,
        client: &crate::bridge_state::ClientState,
    ) -> Result<ApiObservationFilter, crate::BridgeFailure> {
        let filter = self
            .category
            .map_or_else(ApiObservationFilter::all, |category| {
                ApiObservationFilter::all().with_category(category.into())
            });
        let filter = match self.subject {
            Some(handle) => {
                filter.with_subject(client.observation_subjects.get(&handle).ok_or_else(|| {
                    crate::BridgeFailure::invalid_handle("observation subject", handle)
                })?)
            }
            None => filter,
        };
        Ok(match self.freshness {
            Some(freshness) => filter.with_freshness(freshness.into()),
            None => filter,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ObserveEffectiveProfileRequest {
    pub(crate) profile: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ObservationPageRequest {
    pub(crate) snapshot: u64,
    pub(crate) filter: ObservationFilterRequest,
    pub(crate) maximum_inspected: u16,
    pub(crate) maximum_returned: u16,
    pub(crate) maximum_copied_string_bytes: u32,
    pub(crate) cancellation: ObservationCancellationRequest,
    pub(crate) after: Option<u64>,
}

impl ObservationPageRequest {
    pub(crate) fn into_api(
        self,
        client: &crate::bridge_state::ClientState,
    ) -> Result<(u64, ApiObservationPageRequest), crate::BridgeFailure> {
        let request = ApiObservationPageRequest::bounded(
            self.filter.into_api(client)?,
            ObservationPageLimits::new(
                self.maximum_inspected,
                self.maximum_returned,
                self.maximum_copied_string_bytes,
            ),
            self.cancellation.into(),
        );
        let request = match self.after {
            Some(handle) => request.after(
                client
                    .observation_page_cursors
                    .get(&handle)
                    .ok_or_else(|| {
                        crate::BridgeFailure::invalid_handle("observation page cursor", handle)
                    })?
                    .clone(),
            ),
            None => request,
        };
        Ok((self.snapshot, request))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ObservationDeltaRequest {
    pub(crate) current: u64,
    pub(crate) prior: u64,
    pub(crate) maximum_changes: u16,
    pub(crate) cancellation: ObservationCancellationRequest,
}

impl ObservationDeltaRequest {
    pub(crate) fn into_api(
        self,
        client: &crate::bridge_state::ClientState,
    ) -> Result<(u64, u64, ApiObservationDeltaRequest), crate::BridgeFailure> {
        let prior = client
            .observation_snapshots
            .get(&self.prior)
            .ok_or_else(|| {
                crate::BridgeFailure::invalid_handle("observation snapshot", self.prior)
            })?;
        Ok((
            self.current,
            self.prior,
            ApiObservationDeltaRequest::new(
                prior.resume_coordinate(),
                self.maximum_changes,
                self.cancellation.into(),
            ),
        ))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ObservationCausalPathRequest {
    pub(crate) snapshot: u64,
    pub(crate) subject: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ObservationLensRequest {
    pub(crate) snapshot: u64,
    pub(crate) filter: ObservationFilterRequest,
    pub(crate) maximum_inspected: u16,
    pub(crate) maximum_nodes: u16,
    pub(crate) maximum_copied_string_bytes: u32,
    pub(crate) include_causal_paths: bool,
    pub(crate) maximum_causal_path_lookups: u16,
    pub(crate) maximum_causal_steps: u16,
    pub(crate) after: Option<u64>,
    pub(crate) cancellation: ObservationCancellationRequest,
}

impl ObservationLensRequest {
    pub(crate) fn into_api(
        self,
        client: &crate::bridge_state::ClientState,
    ) -> Result<(u64, ApiObservationLensRequest), crate::BridgeFailure> {
        let request = ApiObservationLensRequest::bounded(
            self.filter.into_api(client)?,
            ObservationLensLimits::new(
                self.maximum_inspected,
                self.maximum_nodes,
                self.maximum_copied_string_bytes,
                self.maximum_causal_path_lookups,
                self.maximum_causal_steps,
            ),
            self.include_causal_paths,
            self.cancellation.into(),
        );
        let request = match self.after {
            Some(handle) => request.after(
                client
                    .observation_lens_cursors
                    .get(&handle)
                    .ok_or_else(|| {
                        crate::BridgeFailure::invalid_handle("observation lens cursor", handle)
                    })?
                    .clone(),
            ),
            None => request,
        };
        Ok((self.snapshot, request))
    }
}
