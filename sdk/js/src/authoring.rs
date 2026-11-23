//! Node-native projection of API-owned authoring facts and navigation.
//!
//! The wrapper preserves the closed source roles, structural paths, and
//! semantic identities exposed by [`pale_sdk`]. It never parses TOML, infers
//! a document role, or repeats compiler navigation.

use napi::bindgen_prelude::{BigInt, Buffer, ClassInstance};
use napi::{Env, Error, Result, Status};
use napi_derive::napi;
use pale_sdk::{
    AuthoringCorrection as ApiAuthoringCorrection,
    AuthoringCorrectionKind as ApiAuthoringCorrectionKind,
    AuthoringFieldFact as ApiAuthoringFieldFact,
    AuthoringFieldPresence as ApiAuthoringFieldPresence, AuthoringIdentity as ApiAuthoringIdentity,
    AuthoringIdentityKind as ApiAuthoringIdentityKind,
    AuthoringIngressClass as ApiAuthoringIngressClass,
    AuthoringInspection as ApiAuthoringInspection, AuthoringInspectionRequest,
    AuthoringInspectionState as ApiAuthoringInspectionState,
    AuthoringInstancePathSegment as ApiAuthoringInstancePathSegment, AuthoringLiteral,
    AuthoringLiteralFact as ApiAuthoringLiteralFact, AuthoringNavigation as ApiAuthoringNavigation,
    AuthoringNavigationRequest, AuthoringNodeFact as ApiAuthoringNodeFact,
    AuthoringOccurrence as ApiAuthoringOccurrence,
    AuthoringOccurrencePage as ApiAuthoringOccurrencePage,
    AuthoringOccurrencePageCursor as ApiAuthoringOccurrencePageCursor,
    AuthoringOccurrencePageRequest, AuthoringOccurrenceRole as ApiAuthoringOccurrenceRole,
    AuthoringPathSegment as ApiAuthoringPathSegment,
    AuthoringSourceLocation as ApiAuthoringSourceLocation,
    AuthoringValueKind as ApiAuthoringValueKind,
    PortableWorkspaceSourceInspection as ApiPortableWorkspaceSourceInspection,
    PortableWorkspaceSourceSubject as ApiPortableWorkspaceSourceSubject,
    PortableWorkspaceSubjectOccurrences as ApiPortableWorkspaceSubjectOccurrences,
    StandaloneAuthoringRole as ApiStandaloneAuthoringRole,
};

/// Fixed native ceiling for source-location strings copied by one occurrence page.
const _: () =
    assert!(pale_sdk::MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES <= u32::MAX as usize);
/// Maximum UTF-8 bytes copied by one authoring occurrence page.
#[napi(js_name = "MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES")]
pub const MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES: u32 =
    pale_sdk::MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES as u32;

use crate::error::diagnostic_error;
use crate::{Pale, PortableWorkspaceCompilation, WorkspaceSubject};

/// Resource-custody class whose API-owned limits bound authoring ingress.
#[napi(js_name = "AuthoringIngressClass")]
pub enum AuthoringIngressClass {
    /// Portable workspace-language documents.
    Workspace,
    /// Private selections, bindings, locks, and lifecycle guidance.
    Private,
    /// Immutable Package distribution members.
    Package,
}

impl From<AuthoringIngressClass> for ApiAuthoringIngressClass {
    fn from(value: AuthoringIngressClass) -> Self {
        match value {
            AuthoringIngressClass::Workspace => Self::Workspace,
            AuthoringIngressClass::Private => Self::Private,
            AuthoringIngressClass::Package => Self::Package,
        }
    }
}

/// Exact finite capture limits selected by [`AuthoringIngressClass`].
#[napi(object, object_from_js = false, js_name = "AuthoringIngressLimits")]
pub struct AuthoringIngressLimits {
    /// Maximum logical documents in the selected custody class.
    pub max_documents: u32,
    /// Maximum UTF-8 bytes in one logical document.
    pub max_document_bytes: BigInt,
    /// Maximum aggregate UTF-8 bytes in the custody class.
    pub max_total_bytes: BigInt,
}

/// Explicit standalone grammar role; filenames never select this value.
#[napi(js_name = "StandaloneAuthoringRole")]
pub enum StandaloneAuthoringRole {
    /// Root portable workspace manifest.
    WorkspaceManifest,
    /// Reusable workspace module.
    WorkspaceModule,
    /// Workspace-language primitive declarations.
    WorkspaceLanguagePrimitives,
    /// Responsibility derivation declarations.
    ResponsibilityDerivation,
    /// Organization-to-responsibility translation declarations.
    OrganizationalTranslation,
    /// Experience-composition declarations.
    ExperienceComposition,
    /// Exact module-resolution lock.
    WorkspaceResolutionLock,
    /// Exact workspace-universe lock.
    WorkspaceUniverseLock,
    /// Private capability and operation bindings.
    Capabilities,
    /// Private machine realization and authority-root bindings.
    MachineBinding,
    /// Exact admitted distribution identities.
    DistributionLock,
    /// Package manifest and resource inventory.
    PackageManifest,
    /// Package doctrine declarations.
    PackageDoctrine,
    /// Package operation declarations.
    PackageOperations,
    /// Package role declarations.
    PackageRoles,
    /// Package stack declarations.
    PackageStacks,
    /// Package run-mode declarations.
    PackageRunModes,
    /// Package policy-family declaration.
    PackagePolicyFamily,
    /// Package governing-order declaration.
    PackageGoverningOrder,
    /// Package capability-policy declaration.
    PackageCapabilityPolicy,
    /// Package tool-recipe declaration.
    PackageToolRecipe,
}

impl From<StandaloneAuthoringRole> for ApiStandaloneAuthoringRole {
    fn from(value: StandaloneAuthoringRole) -> Self {
        match value {
            StandaloneAuthoringRole::WorkspaceManifest => Self::WorkspaceManifest,
            StandaloneAuthoringRole::WorkspaceModule => Self::WorkspaceModule,
            StandaloneAuthoringRole::WorkspaceLanguagePrimitives => {
                Self::WorkspaceLanguagePrimitives
            }
            StandaloneAuthoringRole::ResponsibilityDerivation => Self::ResponsibilityDerivation,
            StandaloneAuthoringRole::OrganizationalTranslation => Self::OrganizationalTranslation,
            StandaloneAuthoringRole::ExperienceComposition => Self::ExperienceComposition,
            StandaloneAuthoringRole::WorkspaceResolutionLock => Self::WorkspaceResolutionLock,
            StandaloneAuthoringRole::WorkspaceUniverseLock => Self::WorkspaceUniverseLock,
            StandaloneAuthoringRole::Capabilities => Self::Capabilities,
            StandaloneAuthoringRole::MachineBinding => Self::MachineBinding,
            StandaloneAuthoringRole::DistributionLock => Self::DistributionLock,
            StandaloneAuthoringRole::PackageManifest => Self::PackageManifest,
            StandaloneAuthoringRole::PackageDoctrine => Self::PackageDoctrine,
            StandaloneAuthoringRole::PackageOperations => Self::PackageOperations,
            StandaloneAuthoringRole::PackageRoles => Self::PackageRoles,
            StandaloneAuthoringRole::PackageStacks => Self::PackageStacks,
            StandaloneAuthoringRole::PackageRunModes => Self::PackageRunModes,
            StandaloneAuthoringRole::PackagePolicyFamily => Self::PackagePolicyFamily,
            StandaloneAuthoringRole::PackageGoverningOrder => Self::PackageGoverningOrder,
            StandaloneAuthoringRole::PackageCapabilityPolicy => Self::PackageCapabilityPolicy,
            StandaloneAuthoringRole::PackageToolRecipe => Self::PackageToolRecipe,
        }
    }
}

/// One structural grammar step used for hover and completion inspection.
#[napi(js_name = "AuthoringPathSegment")]
pub struct AuthoringPathSegment {
    inner: ApiAuthoringPathSegment,
}

#[napi]
impl AuthoringPathSegment {
    /// Construct one exact field step.
    #[napi(factory)]
    pub fn field(name: String) -> Self {
        Self {
            inner: ApiAuthoringPathSegment::Field(name),
        }
    }

    /// Construct one structural array-item step.
    #[napi(factory)]
    pub fn array_item() -> Self {
        Self {
            inner: ApiAuthoringPathSegment::ArrayItem,
        }
    }
}

/// One exact decoded-key or array-index step used for semantic navigation.
#[napi(js_name = "AuthoringInstancePathSegment")]
pub struct AuthoringInstancePathSegment {
    inner: ApiAuthoringInstancePathSegment,
}

#[napi]
impl AuthoringInstancePathSegment {
    /// Construct one exact decoded-key step.
    #[napi(factory)]
    pub fn field(name: String) -> Self {
        Self {
            inner: ApiAuthoringInstancePathSegment::Field(name),
        }
    }

    /// Construct one exact array-member step.
    #[napi(factory)]
    pub fn index(index: u32) -> Self {
        Self {
            inner: ApiAuthoringInstancePathSegment::Index(index as usize),
        }
    }
}

/// Availability of generated grammar facts at one admitted source coordinate.
#[napi(js_name = "AuthoringInspectionState")]
pub enum AuthoringInspectionState {
    /// The selected structural path exists.
    Available,
    /// The selected document is substantive text rather than TOML.
    TextDocument,
    /// The compilation does not own the logical label.
    UnknownDocument,
    /// The document is known but the structural path is outside its grammar.
    UnknownPath,
}

impl From<ApiAuthoringInspectionState> for AuthoringInspectionState {
    fn from(value: ApiAuthoringInspectionState) -> Self {
        match value {
            ApiAuthoringInspectionState::Available => Self::Available,
            ApiAuthoringInspectionState::TextDocument => Self::TextDocument,
            ApiAuthoringInspectionState::UnknownDocument => Self::UnknownDocument,
            ApiAuthoringInspectionState::UnknownPath => Self::UnknownPath,
        }
    }
}

/// Structural TOML value category admitted at one generated grammar node.
#[napi(js_name = "AuthoringValueKind")]
pub enum AuthoringValueKind {
    /// Boolean value.
    Boolean,
    /// Integer value.
    Integer,
    /// String value.
    String,
    /// Ordered array value.
    Array,
    /// TOML table or inline-table value.
    Table,
}

impl From<ApiAuthoringValueKind> for AuthoringValueKind {
    fn from(value: ApiAuthoringValueKind) -> Self {
        match value {
            ApiAuthoringValueKind::Boolean => Self::Boolean,
            ApiAuthoringValueKind::Integer => Self::Integer,
            ApiAuthoringValueKind::String => Self::String,
            ApiAuthoringValueKind::Array => Self::Array,
            ApiAuthoringValueKind::Table => Self::Table,
        }
    }
}

/// Requiredness of a child field across structural variants.
#[napi(js_name = "AuthoringFieldPresence")]
pub enum AuthoringFieldPresence {
    /// Every applicable variant requires the field.
    Required,
    /// No applicable variant requires the field.
    Optional,
    /// Requiredness depends on the structural variant.
    VariantDependent,
}

impl From<ApiAuthoringFieldPresence> for AuthoringFieldPresence {
    fn from(value: ApiAuthoringFieldPresence) -> Self {
        match value {
            ApiAuthoringFieldPresence::Required => Self::Required,
            ApiAuthoringFieldPresence::Optional => Self::Optional,
            ApiAuthoringFieldPresence::VariantDependent => Self::VariantDependent,
        }
    }
}

/// Closed TOML literal family returned by generated authoring metadata.
#[napi(js_name = "AuthoringLiteralKind")]
pub enum AuthoringLiteralKind {
    /// UTF-8 string member.
    String,
    /// Signed integer member.
    Integer,
    /// Boolean member.
    Boolean,
}

/// One allowed literal and its producer documentation.
#[napi(object, object_from_js = false, js_name = "AuthoringLiteralFact")]
pub struct AuthoringLiteralFact {
    /// Non-interchangeable literal family.
    pub kind: AuthoringLiteralKind,
    /// Value for the `String` member of [`AuthoringLiteralKind`].
    pub string_value: Option<String>,
    /// Exact value for the `Integer` member of [`AuthoringLiteralKind`].
    pub integer_value: Option<BigInt>,
    /// Value for the `Boolean` member of [`AuthoringLiteralKind`].
    pub boolean_value: Option<bool>,
    /// Variant-specific producer documentation.
    pub description: Option<String>,
}

impl AuthoringLiteralFact {
    fn from_api(value: &ApiAuthoringLiteralFact) -> Self {
        let (kind, string_value, integer_value, boolean_value) = match value.value() {
            AuthoringLiteral::String(value) => (
                AuthoringLiteralKind::String,
                Some(value.clone()),
                None,
                None,
            ),
            AuthoringLiteral::Integer(value) => (
                AuthoringLiteralKind::Integer,
                None,
                Some(BigInt::from(*value)),
                None,
            ),
            AuthoringLiteral::Boolean(value) => {
                (AuthoringLiteralKind::Boolean, None, None, Some(*value))
            }
        };
        Self {
            kind,
            string_value,
            integer_value,
            boolean_value,
            description: value.description().map(str::to_owned),
        }
    }
}

/// One generated child-field fact.
#[napi(object, object_from_js = false, js_name = "AuthoringFieldFact")]
pub struct AuthoringFieldFact {
    /// Exact authored key spelling.
    pub name: String,
    /// Producer documentation for the field.
    pub description: Option<String>,
    /// Requiredness across applicable structural variants.
    pub presence: AuthoringFieldPresence,
    /// Structural value categories admitted for the field.
    pub value_kinds: Vec<AuthoringValueKind>,
    /// Closed literal vocabulary for the field.
    pub allowed_values: Vec<AuthoringLiteralFact>,
}

impl AuthoringFieldFact {
    fn from_api(value: &ApiAuthoringFieldFact) -> Self {
        Self {
            name: value.name().to_owned(),
            description: value.description().map(str::to_owned),
            presence: value.presence().into(),
            value_kinds: value
                .value_kinds()
                .iter()
                .copied()
                .map(Into::into)
                .collect(),
            allowed_values: value
                .allowed_values()
                .iter()
                .map(AuthoringLiteralFact::from_api)
                .collect(),
        }
    }
}

/// Complete generated structural facts for one inspected path.
#[napi(object, object_from_js = false, js_name = "AuthoringNodeFact")]
pub struct AuthoringNodeFact {
    /// Producer documentation for the exact structural context.
    pub description: Option<String>,
    /// Structural categories admitted at the path.
    pub value_kinds: Vec<AuthoringValueKind>,
    /// Closed literal vocabulary admitted at the path.
    pub allowed_values: Vec<AuthoringLiteralFact>,
    /// Deterministically ordered child fields.
    pub fields: Vec<AuthoringFieldFact>,
    /// Optional generated string format.
    pub format: Option<String>,
    /// Optional generated string pattern.
    pub pattern: Option<String>,
    /// Inclusive integer lower bound.
    pub minimum: Option<BigInt>,
    /// Inclusive integer upper bound.
    pub maximum: Option<BigInt>,
    /// Inclusive string-length lower bound.
    pub min_length: Option<BigInt>,
    /// Inclusive string-length upper bound.
    pub max_length: Option<BigInt>,
    /// Inclusive collection-size lower bound.
    pub min_items: Option<BigInt>,
    /// Inclusive collection-size upper bound.
    pub max_items: Option<BigInt>,
}

impl AuthoringNodeFact {
    fn from_api(value: &ApiAuthoringNodeFact) -> Self {
        Self {
            description: value.description().map(str::to_owned),
            value_kinds: value
                .value_kinds()
                .iter()
                .copied()
                .map(Into::into)
                .collect(),
            allowed_values: value
                .allowed_values()
                .iter()
                .map(AuthoringLiteralFact::from_api)
                .collect(),
            fields: value
                .fields()
                .iter()
                .map(AuthoringFieldFact::from_api)
                .collect(),
            format: value.format().map(str::to_owned),
            pattern: value.pattern().map(str::to_owned),
            minimum: value.minimum().map(BigInt::from),
            maximum: value.maximum().map(BigInt::from),
            min_length: value.min_length().map(BigInt::from),
            max_length: value.max_length().map(BigInt::from),
            min_items: value.min_items().map(BigInt::from),
            max_items: value.max_items().map(BigInt::from),
        }
    }
}

/// Complete API-owned structural inspection response.
#[napi(object, object_from_js = false, js_name = "AuthoringInspection")]
pub struct AuthoringInspection {
    /// Availability classification for the exact request.
    pub state: AuthoringInspectionState,
    /// Descriptor identity of the selected grammar, when available.
    pub schema_id: Option<String>,
    /// Selected named definition for a shared-definition grammar.
    pub definition: Option<String>,
    /// Generated structural facts when `state` is `Available`.
    pub node: Option<AuthoringNodeFact>,
}

impl AuthoringInspection {
    pub(crate) fn from_api(value: ApiAuthoringInspection) -> Self {
        Self {
            state: value.state().into(),
            schema_id: value.schema_id().map(str::to_owned),
            definition: value.definition().map(str::to_owned),
            node: value.node().map(AuthoringNodeFact::from_api),
        }
    }
}

/// Exact semantic role of one admitted source occurrence.
#[napi(js_name = "AuthoringOccurrenceRole")]
pub enum AuthoringOccurrenceRole {
    /// Canonical local definition.
    Definition,
    /// Reference to a separately defined identity.
    Reference,
    /// Content-bearing interface projection.
    Projection,
}

impl From<ApiAuthoringOccurrenceRole> for AuthoringOccurrenceRole {
    fn from(value: ApiAuthoringOccurrenceRole) -> Self {
        match value {
            ApiAuthoringOccurrenceRole::Definition => Self::Definition,
            ApiAuthoringOccurrenceRole::Reference => Self::Reference,
            ApiAuthoringOccurrenceRole::Projection => Self::Projection,
        }
    }
}

/// Closed namespace of one compiler-resolved authoring identity.
#[napi(js_name = "AuthoringIdentityKind")]
pub enum AuthoringIdentityKind {
    /// Compiler-admitted source document.
    SourceDocument,
    /// Principal-owned workspace root.
    Workspace,
    /// Canonical desired project.
    Project,
    /// Canonical desired execution workspace.
    ExecutionWorkspace,
    /// Canonical desired repository.
    Repository,
    /// Neutral execution-path selection.
    ExecutionPath,
    /// Reusable workspace module.
    Module,
    /// Immutable module release.
    ModuleRelease,
    /// Content-addressed workspace component.
    Component,
    /// Named module export.
    Export,
    /// Module-scoped namespace.
    Namespace,
    /// Qualified declaration.
    Declaration,
    /// Module-scoped resource.
    Resource,
    /// Private run-policy declaration.
    RunPolicy,
    /// Private lifecycle behavior.
    LifecycleBehavior,
    /// Portable capability identity.
    Capability,
    /// Machine-local authority-root projection.
    AuthorityRoot,
    /// Exact provider-binding projection.
    ProviderBinding,
    /// Private machine binding.
    Machine,
    /// Attributable responsibility derivation.
    ResponsibilityDerivation,
    /// Responsibility native projection.
    ResponsibilityProjection,
    /// Responsibility Genealogy link.
    ResponsibilityGenealogyLink,
    /// Qualified logical scope.
    LogicalScope,
    /// Qualified logical organization.
    LogicalOrganization,
    /// Qualified organizational group.
    OrganizationalGroup,
    /// Qualified discipline tree.
    DisciplineTree,
    /// Stable organizational role.
    OrganizationalRole,
    /// Organizational occupant binding.
    OrganizationalOccupant,
    /// Organizational provider binding.
    OrganizationalProvider,
    /// Communication actor binding.
    CommunicationActor,
    /// Leadership topic.
    LeadershipTopic,
    /// Organizational relation.
    OrganizationalRelation,
    /// Owner-selected organizational object.
    OrganizationalObject,
    /// Group-lens authority profile.
    GroupLensProfile,
    /// Directional-service contract.
    DirectionalService,
    /// Organizational native projection.
    OrganizationalProjection,
    /// Organizational inspection.
    OrganizationalInspection,
    /// Neutral composition participant.
    CompositionParticipant,
    /// Composition responsibility contract.
    CompositionResponsibilityContract,
    /// Composition participant manifest.
    CompositionParticipantManifest,
    /// Provider in a composition manifest.
    CompositionProvider,
    /// Neutral composition provider binding.
    CompositionProviderBinding,
    /// Neutral composition entry contract.
    CompositionEntryContract,
    /// Neutral request translation.
    CompositionRequestTranslation,
    /// Neutral result translation.
    CompositionResultTranslation,
    /// Neutral termination contract.
    CompositionTerminationContract,
    /// Neutral settlement contract.
    CompositionSettlementContract,
    /// Neutral continuity contract.
    CompositionContinuityContract,
    /// Typed neutral composition edge.
    CompositionEdge,
    /// Selected neutral experience path.
    ExperiencePath,
    /// Package doctrine resource.
    PackageDoctrine,
    /// Package operation archetype.
    PackageOperationArchetype,
    /// Package concrete operation.
    PackageOperation,
    /// Package role archetype.
    PackageRoleArchetype,
    /// Package specialist role.
    PackageRole,
    /// Package stack.
    PackageStack,
    /// Package run mode.
    PackageRunMode,
    /// Package policy family.
    PackagePolicyFamily,
    /// Package governing order.
    PackageGoverningOrder,
    /// Package capability policy.
    PackageCapabilityPolicy,
    /// Package tool recipe.
    PackageToolRecipe,
    /// Immutable Package identity.
    Package,
}

impl From<ApiAuthoringIdentityKind> for AuthoringIdentityKind {
    fn from(value: ApiAuthoringIdentityKind) -> Self {
        match value {
            ApiAuthoringIdentityKind::SourceDocument => Self::SourceDocument,
            ApiAuthoringIdentityKind::Workspace => Self::Workspace,
            ApiAuthoringIdentityKind::Project => Self::Project,
            ApiAuthoringIdentityKind::ExecutionWorkspace => Self::ExecutionWorkspace,
            ApiAuthoringIdentityKind::Repository => Self::Repository,
            ApiAuthoringIdentityKind::ExecutionPath => Self::ExecutionPath,
            ApiAuthoringIdentityKind::Module => Self::Module,
            ApiAuthoringIdentityKind::ModuleRelease => Self::ModuleRelease,
            ApiAuthoringIdentityKind::Component => Self::Component,
            ApiAuthoringIdentityKind::Export => Self::Export,
            ApiAuthoringIdentityKind::Namespace => Self::Namespace,
            ApiAuthoringIdentityKind::Declaration => Self::Declaration,
            ApiAuthoringIdentityKind::Resource => Self::Resource,
            ApiAuthoringIdentityKind::RunPolicy => Self::RunPolicy,
            ApiAuthoringIdentityKind::LifecycleBehavior => Self::LifecycleBehavior,
            ApiAuthoringIdentityKind::Capability => Self::Capability,
            ApiAuthoringIdentityKind::AuthorityRoot => Self::AuthorityRoot,
            ApiAuthoringIdentityKind::ProviderBinding => Self::ProviderBinding,
            ApiAuthoringIdentityKind::Machine => Self::Machine,
            ApiAuthoringIdentityKind::ResponsibilityDerivation => Self::ResponsibilityDerivation,
            ApiAuthoringIdentityKind::ResponsibilityProjection => Self::ResponsibilityProjection,
            ApiAuthoringIdentityKind::ResponsibilityGenealogyLink => {
                Self::ResponsibilityGenealogyLink
            }
            ApiAuthoringIdentityKind::LogicalScope => Self::LogicalScope,
            ApiAuthoringIdentityKind::LogicalOrganization => Self::LogicalOrganization,
            ApiAuthoringIdentityKind::OrganizationalGroup => Self::OrganizationalGroup,
            ApiAuthoringIdentityKind::DisciplineTree => Self::DisciplineTree,
            ApiAuthoringIdentityKind::OrganizationalRole => Self::OrganizationalRole,
            ApiAuthoringIdentityKind::OrganizationalOccupant => Self::OrganizationalOccupant,
            ApiAuthoringIdentityKind::OrganizationalProvider => Self::OrganizationalProvider,
            ApiAuthoringIdentityKind::CommunicationActor => Self::CommunicationActor,
            ApiAuthoringIdentityKind::LeadershipTopic => Self::LeadershipTopic,
            ApiAuthoringIdentityKind::OrganizationalRelation => Self::OrganizationalRelation,
            ApiAuthoringIdentityKind::OrganizationalObject => Self::OrganizationalObject,
            ApiAuthoringIdentityKind::GroupLensProfile => Self::GroupLensProfile,
            ApiAuthoringIdentityKind::DirectionalService => Self::DirectionalService,
            ApiAuthoringIdentityKind::OrganizationalProjection => Self::OrganizationalProjection,
            ApiAuthoringIdentityKind::OrganizationalInspection => Self::OrganizationalInspection,
            ApiAuthoringIdentityKind::CompositionParticipant => Self::CompositionParticipant,
            ApiAuthoringIdentityKind::CompositionResponsibilityContract => {
                Self::CompositionResponsibilityContract
            }
            ApiAuthoringIdentityKind::CompositionParticipantManifest => {
                Self::CompositionParticipantManifest
            }
            ApiAuthoringIdentityKind::CompositionProvider => Self::CompositionProvider,
            ApiAuthoringIdentityKind::CompositionProviderBinding => {
                Self::CompositionProviderBinding
            }
            ApiAuthoringIdentityKind::CompositionEntryContract => Self::CompositionEntryContract,
            ApiAuthoringIdentityKind::CompositionRequestTranslation => {
                Self::CompositionRequestTranslation
            }
            ApiAuthoringIdentityKind::CompositionResultTranslation => {
                Self::CompositionResultTranslation
            }
            ApiAuthoringIdentityKind::CompositionTerminationContract => {
                Self::CompositionTerminationContract
            }
            ApiAuthoringIdentityKind::CompositionSettlementContract => {
                Self::CompositionSettlementContract
            }
            ApiAuthoringIdentityKind::CompositionContinuityContract => {
                Self::CompositionContinuityContract
            }
            ApiAuthoringIdentityKind::CompositionEdge => Self::CompositionEdge,
            ApiAuthoringIdentityKind::ExperiencePath => Self::ExperiencePath,
            ApiAuthoringIdentityKind::PackageDoctrine => Self::PackageDoctrine,
            ApiAuthoringIdentityKind::PackageOperationArchetype => Self::PackageOperationArchetype,
            ApiAuthoringIdentityKind::PackageOperation => Self::PackageOperation,
            ApiAuthoringIdentityKind::PackageRoleArchetype => Self::PackageRoleArchetype,
            ApiAuthoringIdentityKind::PackageRole => Self::PackageRole,
            ApiAuthoringIdentityKind::PackageStack => Self::PackageStack,
            ApiAuthoringIdentityKind::PackageRunMode => Self::PackageRunMode,
            ApiAuthoringIdentityKind::PackagePolicyFamily => Self::PackagePolicyFamily,
            ApiAuthoringIdentityKind::PackageGoverningOrder => Self::PackageGoverningOrder,
            ApiAuthoringIdentityKind::PackageCapabilityPolicy => Self::PackageCapabilityPolicy,
            ApiAuthoringIdentityKind::PackageToolRecipe => Self::PackageToolRecipe,
            ApiAuthoringIdentityKind::Package => Self::Package,
        }
    }
}

/// Opaque API-owned semantic identity eligible for effective-profile joining.
#[napi(js_name = "AuthoringIdentity")]
pub struct AuthoringIdentity {
    pub(crate) inner: ApiAuthoringIdentity,
}

#[napi]
impl AuthoringIdentity {
    /// Non-interchangeable semantic namespace.
    #[napi(getter)]
    pub fn kind(&self) -> AuthoringIdentityKind {
        self.inner.kind().into()
    }

    /// Canonical compiler-owned display form.
    #[napi(getter)]
    pub fn value(&self) -> String {
        self.inner.value().to_owned()
    }

    /// Exact TOML scalar spelling valid in the reference scope.
    #[napi(getter)]
    pub fn authored_value(&self) -> String {
        self.inner.authored_value().to_owned()
    }
}

/// Opaque continuation tied to one compilation, query, and pair of limits.
///
/// Each JavaScript handle keeps the SDK cursor's shared compilation alive until
/// garbage collection releases it. The cursor cannot be reconstructed from text
/// and does not grant permission to disclose source locations.
#[napi(js_name = "AuthoringOccurrencePageCursor")]
pub struct AuthoringOccurrencePageCursor {
    inner: ApiAuthoringOccurrencePageCursor,
}

/// One compiler-resolved source occurrence, without source text.
#[napi(object, object_from_js = false, js_name = "AuthoringOccurrence")]
pub struct AuthoringOccurrence {
    /// Logical source label and decoded field/index path.
    pub location: AuthoringSourceLocation,
    /// Definition, reference, or projection role assigned by the compiler.
    pub role: AuthoringOccurrenceRole,
}

impl AuthoringOccurrence {
    pub(crate) fn from_api(value: &ApiAuthoringOccurrence) -> Result<Self> {
        Ok(Self {
            location: AuthoringSourceLocation::from_api(value.location())?,
            role: value.role().into(),
        })
    }
}

/// Matching occurrences and scan progress from the retained compiler index.
///
/// An empty page can be incomplete. Row limits combine with one fixed ceiling
/// over copied source labels and field names.
#[napi(js_name = "AuthoringOccurrencePage")]
pub struct AuthoringOccurrencePage {
    pub(crate) inner: ApiAuthoringOccurrencePage,
}

#[napi]
impl AuthoringOccurrencePage {
    /// Copy matching locations and roles in compiler source/path order.
    ///
    /// # Errors
    ///
    /// Returns a projection error if a source index exceeds the Node path ABI.
    #[napi]
    pub fn occurrences(&self) -> Result<Vec<AuthoringOccurrence>> {
        self.inner
            .occurrences()
            .iter()
            .map(AuthoringOccurrence::from_api)
            .collect()
    }

    /// Index entries examined on this page, including nonmatches.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }

    /// UTF-8 bytes copied into returned source labels and field names.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }

    /// Whether the scan proved exhaustion of the retained index.
    #[napi(getter)]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Share the continuation, or return `null` after proven exhaustion.
    #[napi]
    pub fn continuation(&self) -> Option<AuthoringOccurrencePageCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| AuthoringOccurrencePageCursor { inner })
    }
}

/// Direct named meaning at a source location, separate from proof membership.
#[napi(js_name = "PortableWorkspaceSourceState")]
pub enum PortableWorkspaceSourceState {
    /// No occurrence is indexed; the source or field may still exist.
    NoOccurrence,
    /// The occurrence has no supported direct proof-subject mapping.
    NoDenotation,
    /// The occurrence names a subject excluded from the selected proof.
    Unselected,
    /// The occurrence names a subject included in the selected proof.
    Selected,
}

/// A source answer retaining the exact compilation that supplied it.
///
/// Equal semantic digests can accompany different unselected source inputs.
/// Garbage collection releases the shared compiler handle, not native state.
/// This inspection grants neither source disclosure nor execution permission.
#[napi(js_name = "PortableWorkspaceSourceInspection")]
pub struct PortableWorkspaceSourceInspection {
    inner: ApiPortableWorkspaceSourceInspection,
}

#[napi]
impl PortableWorkspaceSourceInspection {
    /// Direct named meaning and selected-proof membership from the compiler.
    #[napi(getter)]
    pub fn state(&self) -> PortableWorkspaceSourceState {
        match self.inner.result() {
            ApiPortableWorkspaceSourceSubject::NoOccurrence => {
                PortableWorkspaceSourceState::NoOccurrence
            }
            ApiPortableWorkspaceSourceSubject::NoDenotation { .. } => {
                PortableWorkspaceSourceState::NoDenotation
            }
            ApiPortableWorkspaceSourceSubject::Unselected { .. } => {
                PortableWorkspaceSourceState::Unselected
            }
            ApiPortableWorkspaceSourceSubject::Selected { .. } => {
                PortableWorkspaceSourceState::Selected
            }
        }
    }

    /// Requested location, including when no occurrence was found.
    ///
    /// # Errors
    /// Returns a projection error if an array index exceeds the Node path ABI.
    #[napi(getter)]
    pub fn location(&self) -> Result<AuthoringSourceLocation> {
        AuthoringSourceLocation::from_api(self.inner.location())
    }

    /// Copy the location and role; absent only for `NoOccurrence`.
    ///
    /// # Errors
    /// Returns a projection error if an array index exceeds the Node path ABI.
    #[napi]
    pub fn occurrence(&self) -> Result<Option<AuthoringOccurrence>> {
        match self.inner.result() {
            ApiPortableWorkspaceSourceSubject::NoOccurrence => Ok(None),
            ApiPortableWorkspaceSourceSubject::NoDenotation { occurrence }
            | ApiPortableWorkspaceSourceSubject::Unselected { occurrence, .. }
            | ApiPortableWorkspaceSourceSubject::Selected { occurrence, .. } => {
                AuthoringOccurrence::from_api(occurrence).map(Some)
            }
        }
    }

    /// Copy the full subject for `Unselected` or `Selected`; otherwise `null`.
    #[napi]
    pub fn subject(&self) -> Option<WorkspaceSubject> {
        match self.inner.result() {
            ApiPortableWorkspaceSourceSubject::Unselected { subject, .. }
            | ApiPortableWorkspaceSourceSubject::Selected { subject, .. } => {
                Some(WorkspaceSubject::from_api(subject))
            }
            ApiPortableWorkspaceSourceSubject::NoOccurrence
            | ApiPortableWorkspaceSourceSubject::NoDenotation { .. } => None,
        }
    }

    /// Match the exact retained compilation, not merely its semantic digests.
    #[napi]
    pub fn belongs_to(&self, workspace: &PortableWorkspaceCompilation) -> bool {
        self.inner.belongs_to(&workspace.inner)
    }
}

/// Membership in the selected proof, independently of source-page matches.
#[napi(js_name = "PortableWorkspaceSubjectMembership")]
pub enum PortableWorkspaceSubjectMembership {
    /// Captured occurrences may exist without selected proof membership.
    Unselected,
    /// The subject is selected, even if it has no direct named occurrences.
    Selected,
}

/// Compiler membership plus a bounded page of direct named occurrences.
///
/// An empty incomplete page must be resumed. The underlying cursor retains
/// the exact compilation, full subject, query kind, and both raw limits.
#[napi(js_name = "PortableWorkspaceSubjectOccurrences")]
pub struct PortableWorkspaceSubjectOccurrences {
    inner: ApiPortableWorkspaceSubjectOccurrences,
}

#[napi]
impl PortableWorkspaceSubjectOccurrences {
    /// Selected-proof membership; it does not summarize source-page contents.
    #[napi(getter)]
    pub fn membership(&self) -> PortableWorkspaceSubjectMembership {
        match self.inner {
            ApiPortableWorkspaceSubjectOccurrences::Unselected(_) => {
                PortableWorkspaceSubjectMembership::Unselected
            }
            ApiPortableWorkspaceSubjectOccurrences::Selected(_) => {
                PortableWorkspaceSubjectMembership::Selected
            }
        }
    }

    /// Copy the SDK page without recompiling or restarting its scan.
    #[napi]
    pub fn page(&self) -> AuthoringOccurrencePage {
        let (ApiPortableWorkspaceSubjectOccurrences::Unselected(page)
        | ApiPortableWorkspaceSubjectOccurrences::Selected(page)) = &self.inner;
        AuthoringOccurrencePage {
            inner: page.clone(),
        }
    }
}

pub(crate) fn occurrence_request(
    identity: &AuthoringIdentity,
    maximum_inspected: u16,
    maximum_returned: u16,
    after: Option<&AuthoringOccurrencePageCursor>,
) -> AuthoringOccurrencePageRequest {
    let request = AuthoringOccurrencePageRequest::new(
        identity.inner.clone(),
        maximum_inspected,
        maximum_returned,
    );
    match after {
        Some(cursor) => request.after(cursor.inner.clone()),
        None => request,
    }
}

/// Reject numeric coercion before a raw page limit reaches SDK cursor binding.
pub(crate) fn authoring_page_limit(value: f64) -> Result<u16> {
    if value.is_finite() && value.fract() == 0.0 && (0.0..=f64::from(u16::MAX)).contains(&value) {
        Ok(value as u16)
    } else {
        Err(Error::new(
            Status::InvalidArg,
            "authoring page limit must be an integer from 0 through 65535",
        ))
    }
}

/// One exact source location retained by the compiler, including a rejected input.
#[napi(object, object_from_js = false, js_name = "AuthoringSourceLocation")]
pub struct AuthoringSourceLocation {
    /// Compiler logical source label.
    pub source_label: String,
    /// Exact decoded-key and array-index path.
    pub path: Vec<AuthoringLocationSegment>,
}

/// Read-only projection of one exact source-location path segment.
#[napi(object, object_from_js = false, js_name = "AuthoringLocationSegment")]
pub struct AuthoringLocationSegment {
    /// Field name for a decoded-key step.
    pub field: Option<String>,
    /// Array index for an array-member step.
    pub index: Option<u32>,
}

impl AuthoringSourceLocation {
    pub(crate) fn from_api(value: &ApiAuthoringSourceLocation) -> Result<Self> {
        Ok(Self {
            source_label: value.source_label().to_owned(),
            path: location_path(value.path())?,
        })
    }
}

fn location_path(
    path: &[ApiAuthoringInstancePathSegment],
) -> Result<Vec<AuthoringLocationSegment>> {
    path.iter()
        .map(|segment| match segment {
            ApiAuthoringInstancePathSegment::Field(field) => Ok(AuthoringLocationSegment {
                field: Some(field.clone()),
                index: None,
            }),
            ApiAuthoringInstancePathSegment::Index(index) => u32::try_from(*index)
                .map(|index| AuthoringLocationSegment {
                    field: None,
                    index: Some(index),
                })
                .map_err(|_| {
                    Error::new(
                        Status::GenericFailure,
                        "API authoring array index exceeds the Node uint32 projection",
                    )
                }),
        })
        .collect()
}

/// Closed class of source correction proved by the compiler.
#[napi(js_name = "AuthoringCorrectionKind")]
pub enum AuthoringCorrectionKind {
    /// Narrow an import visibility ceiling to the selected export's ceiling.
    NarrowImportVisibility,
}

/// Copied compiler-proved edit, not permission to change a source document.
///
/// Callers must match the exact expected source before applying a replacement.
/// Caught errors still require ordinary JavaScript narrowing; `instanceof
/// AuthoringCorrection` identifies a native correction member, not write authority.
#[napi(js_name = "AuthoringCorrection")]
pub struct AuthoringCorrection {
    inner: ApiAuthoringCorrection,
}

impl AuthoringCorrection {
    pub(crate) fn from_api(value: &ApiAuthoringCorrection) -> Self {
        Self {
            inner: value.clone(),
        }
    }
}

#[napi]
impl AuthoringCorrection {
    /// Compiler-proved correction class.
    #[napi(getter)]
    pub fn kind(&self) -> AuthoringCorrectionKind {
        match self.inner.kind() {
            ApiAuthoringCorrectionKind::NarrowImportVisibility => {
                AuthoringCorrectionKind::NarrowImportVisibility
            }
        }
    }

    /// Logical label of the edited source.
    #[napi(getter)]
    pub fn source_label(&self) -> String {
        self.inner.source_label().to_owned()
    }

    /// Exact decoded-key and array-index path.
    ///
    /// # Errors
    ///
    /// Returns a projection error if an API array index exceeds Node's `uint32` ABI.
    #[napi(getter)]
    pub fn path(&self) -> Result<Vec<AuthoringLocationSegment>> {
        location_path(self.inner.path())
    }

    /// Exact source spelling that must still exist before any edit.
    #[napi(getter)]
    pub fn expected_source(&self) -> String {
        self.inner.expected_source().to_owned()
    }

    /// Exact replacement spelling selected by the compiler.
    #[napi(getter)]
    pub fn replacement_source(&self) -> String {
        self.inner.replacement_source().to_owned()
    }

    /// Retained semantic subject proving this correction, not an edit capability.
    #[napi(getter)]
    pub fn proof_subject(&self) -> AuthoringIdentity {
        AuthoringIdentity {
            inner: self.inner.proof_subject().clone(),
        }
    }

    /// Identity of the exact rejected diagnostic.
    #[napi(getter)]
    pub fn diagnostic_identity(&self) -> String {
        self.inner.diagnostic_identity().as_str().to_owned()
    }
}

/// Retained semantic-navigation result from one admitted compilation.
#[napi(js_name = "AuthoringNavigation")]
pub struct AuthoringNavigation {
    pub(crate) inner: ApiAuthoringNavigation,
}

#[napi]
impl AuthoringNavigation {
    /// Resolved identity, or `None` for a structural-only coordinate.
    #[napi]
    pub fn identity(&self) -> Option<AuthoringIdentity> {
        self.inner
            .identity()
            .cloned()
            .map(|inner| AuthoringIdentity { inner })
    }

    /// Exact semantic role of the queried occurrence.
    #[napi(getter)]
    pub fn occurrence_role(&self) -> Option<AuthoringOccurrenceRole> {
        self.inner.occurrence_role().map(Into::into)
    }

    /// Canonical definition retained by the compilation, when one exists.
    ///
    /// # Errors
    ///
    /// Returns an internal projection error only if an API-owned array index
    /// cannot be represented by the Node `uint32` path ABI.
    #[napi]
    pub fn definition(&self) -> Result<Option<AuthoringSourceLocation>> {
        self.inner
            .definition()
            .map(AuthoringSourceLocation::from_api)
            .transpose()
    }

    /// Deterministically ordered retained reference locations.
    ///
    /// # Errors
    ///
    /// Returns an internal projection error only if an API-owned array index
    /// cannot be represented by the Node `uint32` path ABI.
    #[napi]
    pub fn references(&self) -> Result<Vec<AuthoringSourceLocation>> {
        self.inner
            .references()
            .iter()
            .map(AuthoringSourceLocation::from_api)
            .collect()
    }

    /// Same-namespace identities eligible for value completion.
    #[napi]
    pub fn alternatives(&self) -> Vec<AuthoringIdentity> {
        self.inner
            .alternatives()
            .iter()
            .cloned()
            .map(|inner| AuthoringIdentity { inner })
            .collect()
    }

    /// Whether the API reference ceiling omitted occurrences.
    #[napi(getter)]
    pub fn references_truncated(&self) -> bool {
        self.inner.references_truncated()
    }

    /// Whether the API completion ceiling omitted identities.
    #[napi(getter)]
    pub fn alternatives_truncated(&self) -> bool {
        self.inner.alternatives_truncated()
    }
}

pub(crate) fn structural_path(
    path: &[ClassInstance<AuthoringPathSegment>],
) -> Vec<ApiAuthoringPathSegment> {
    path.iter().map(|segment| segment.inner.clone()).collect()
}

pub(crate) fn instance_path(
    path: &[ClassInstance<AuthoringInstancePathSegment>],
) -> Vec<ApiAuthoringInstancePathSegment> {
    path.iter().map(|segment| segment.inner.clone()).collect()
}

#[napi]
impl Pale {
    /// Return the application-owned capture ceilings for one source class.
    ///
    /// # Errors
    ///
    /// Returns an internal projection error if the API document-count ceiling
    /// cannot be represented by the Node `uint32` ABI.
    #[napi]
    pub fn authoring_ingress_limits(
        &self,
        source_class: AuthoringIngressClass,
    ) -> Result<AuthoringIngressLimits> {
        let limits = pale_sdk::PaleApi::authoring_ingress_limits(source_class.into());
        Ok(AuthoringIngressLimits {
            max_documents: u32::try_from(limits.max_documents()).map_err(|_| {
                Error::new(
                    Status::GenericFailure,
                    "API authoring document limit exceeds the Node uint32 projection",
                )
            })?,
            max_document_bytes: BigInt::from(limits.max_document_bytes()),
            max_total_bytes: BigInt::from(limits.max_total_bytes()),
        })
    }

    /// Validate one explicitly typed standalone authoring document.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when source
    /// capture, TOML parsing, or API-owned grammar admission fails.
    #[napi]
    pub fn validate_standalone_authoring(
        &self,
        env: Env,
        role: StandaloneAuthoringRole,
        source_label: String,
        bytes: Buffer,
    ) -> Result<()> {
        self.api
            .validate_authoring_document(role.into(), source_label, bytes.to_vec())
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Inspect generated grammar facts for one standalone authoring role.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` carrying the API-owned canonical diagnostic for
    /// an oversized path or incoherent generated authoring metadata.
    #[napi]
    pub fn inspect_standalone_authoring(
        &self,
        env: Env,
        role: StandaloneAuthoringRole,
        path: Vec<ClassInstance<AuthoringPathSegment>>,
    ) -> Result<AuthoringInspection> {
        self.api
            .inspect_standalone_authoring(role.into(), structural_path(&path))
            .map(AuthoringInspection::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Inspect generated grammar facts retained by an admitted workspace.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` carrying the API-owned canonical diagnostic for
    /// an oversized path or incoherent generated authoring metadata.
    #[napi]
    pub fn inspect_portable_workspace_authoring(
        &self,
        env: Env,
        workspace: &PortableWorkspaceCompilation,
        source_label: String,
        path: Vec<ClassInstance<AuthoringPathSegment>>,
    ) -> Result<AuthoringInspection> {
        workspace
            .inner
            .inspect_authoring(AuthoringInspectionRequest::new(
                source_label,
                structural_path(&path),
            ))
            .map(AuthoringInspection::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Resolve one exact source occurrence retained by an admitted workspace.
    #[napi]
    pub fn navigate_portable_workspace_authoring(
        &self,
        workspace: &PortableWorkspaceCompilation,
        source_label: String,
        path: Vec<ClassInstance<AuthoringInstancePathSegment>>,
    ) -> AuthoringNavigation {
        AuthoringNavigation {
            inner: workspace
                .inner
                .navigate_authoring(AuthoringNavigationRequest::new(
                    source_label,
                    instance_path(&path),
                )),
        }
    }

    /// Find occurrences of a compiler-produced identity without recompiling.
    ///
    /// Limits must be integers from 0 through 65535. Zero selects 64 and values
    /// above 256 are capped by the SDK. Resume with the same raw limits and
    /// identity on this exact workspace. An empty page can still continue.
    ///
    /// # Errors
    ///
    /// Invalid numeric inputs produce `InvalidArg`. An unlike continuation
    /// produces an error carrying the canonical API diagnostic.
    #[napi]
    pub fn portable_workspace_authoring_occurrences(
        &self,
        env: Env,
        workspace: &PortableWorkspaceCompilation,
        identity: &AuthoringIdentity,
        maximum_inspected: f64,
        maximum_returned: f64,
        after: Option<&AuthoringOccurrencePageCursor>,
    ) -> Result<AuthoringOccurrencePage> {
        workspace
            .inner
            .authoring_occurrences(occurrence_request(
                identity,
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
                after,
            ))
            .map(|inner| AuthoringOccurrencePage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Read the subject directly named at one captured source location.
    ///
    /// The result retains this compilation even when the subject is unselected.
    /// It does not follow proof inputs or turn nearby names into record anchors.
    ///
    /// # Errors
    /// Returns the canonical API diagnostic when the path exceeds 64 segments.
    #[napi]
    pub fn portable_workspace_source_subject(
        &self,
        env: Env,
        workspace: &PortableWorkspaceCompilation,
        source_label: String,
        path: Vec<ClassInstance<AuthoringInstancePathSegment>>,
    ) -> Result<PortableWorkspaceSourceInspection> {
        workspace
            .inner
            .source_subject(AuthoringNavigationRequest::new(
                source_label,
                instance_path(&path),
            ))
            .map(|inner| PortableWorkspaceSourceInspection { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Find direct named occurrences of a full proof subject without recompiling.
    ///
    /// Limits must be integers from 0 through 65535. Zero selects 64; the SDK
    /// caps values above 256. Resume on this compilation with the same subject
    /// and raw limits. Identity-query cursors cannot resume a subject query.
    /// Limits count examined and returned rows, not bytes, time or retained
    /// memory. This supplied-input query does not authorize source disclosure.
    ///
    /// # Errors
    /// Invalid numbers produce `InvalidArg`. An unlike continuation produces
    /// an error carrying the canonical API diagnostic before the scan begins.
    #[napi]
    pub fn portable_workspace_subject_occurrences(
        &self,
        env: Env,
        workspace: &PortableWorkspaceCompilation,
        subject: &WorkspaceSubject,
        maximum_inspected: f64,
        maximum_returned: f64,
        after: Option<&AuthoringOccurrencePageCursor>,
    ) -> Result<PortableWorkspaceSubjectOccurrences> {
        workspace
            .inner
            .subject_occurrences(
                &subject.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
            )
            .map(|inner| PortableWorkspaceSubjectOccurrences { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}
