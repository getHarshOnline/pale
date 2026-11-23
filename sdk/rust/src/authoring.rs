//! Binding-safe structural authoring projection over admitted source custody.

use pale_application::workspace as application;

use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticFactV1Wire, DiagnosticRetryV1,
    DiagnosticStageV1, RecordSchemaIdentity,
};

use crate::error::FailureSpecification;
use crate::{ApiError, EffectiveProfileCompilation, PaleApi, PortableWorkspaceCompilation};

mod occurrences;
mod source;
pub use occurrences::{
    AuthoringOccurrence, AuthoringOccurrencePage, AuthoringOccurrencePageCursor,
    AuthoringOccurrencePageRequest, PortableWorkspaceSourceInspection,
    PortableWorkspaceSourceSubject, PortableWorkspaceSubjectOccurrences,
    MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES,
};
pub use source::StandaloneAuthoringRole;

/// Maximum structural or exact-instance depth accepted by authoring queries.
///
/// Tolerant parsers and binding clients should use this same ceiling before
/// constructing a request so an oversized path is rejected without truncation.
pub const AUTHORING_PATH_SEGMENT_LIMIT: usize = 64;

/// One structural step from an admitted TOML root to an editor cursor.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthoringPathSegment {
    /// Descend through one exact key.
    Field(String),
    /// Descend through one array or array-of-tables item.
    ArrayItem,
}

/// One exact step through an admitted TOML instance.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum AuthoringInstancePathSegment {
    /// Descend through one decoded key.
    Field(String),
    /// Select one exact array or array-of-tables member.
    Index(usize),
}

impl From<AuthoringInstancePathSegment> for application::AuthoringInstancePathSegment {
    fn from(value: AuthoringInstancePathSegment) -> Self {
        match value {
            AuthoringInstancePathSegment::Field(field) => Self::Field(field),
            AuthoringInstancePathSegment::Index(index) => Self::Index(index),
        }
    }
}

impl From<&application::AuthoringInstancePathSegment> for AuthoringInstancePathSegment {
    fn from(value: &application::AuthoringInstancePathSegment) -> Self {
        match value {
            application::AuthoringInstancePathSegment::Field(field) => Self::Field(field.clone()),
            application::AuthoringInstancePathSegment::Index(index) => Self::Index(*index),
        }
    }
}

/// Exact admitted source coordinate requested for semantic navigation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringNavigationRequest {
    source_label: String,
    path: Vec<AuthoringInstancePathSegment>,
}

impl AuthoringNavigationRequest {
    /// Bind one compiler logical label to an exact syntax-derived instance path.
    #[must_use]
    pub fn new(source_label: String, path: Vec<AuthoringInstancePathSegment>) -> Self {
        Self { source_label, path }
    }
}

/// Closed namespace of a resolved editor-visible identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthoringIdentityKind {
    /// One exact compiler-admitted source document.
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
    /// Immutable release of one workspace module.
    ModuleRelease,
    /// Content-addressed workspace-language component.
    Component,
    /// Named module export.
    Export,
    /// Module-scoped namespace.
    Namespace,
    /// Module- and namespace-qualified declaration.
    Declaration,
    /// Module-scoped logical resource.
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
    /// Private machine-binding identity.
    Machine,
    /// Attributable responsibility-derivation identity.
    ResponsibilityDerivation,
    /// Responsibility native-projection identity.
    ResponsibilityProjection,
    /// Responsibility Genealogy-link identity.
    ResponsibilityGenealogyLink,
    /// Namespace-qualified logical scope.
    LogicalScope,
    /// Namespace-qualified logical organization.
    LogicalOrganization,
    /// Namespace-qualified organizational group.
    OrganizationalGroup,
    /// Namespace-qualified discipline tree.
    DisciplineTree,
    /// Namespace-qualified stable organizational role.
    OrganizationalRole,
    /// Organizational occupant identity introduced by an explicit binding.
    OrganizationalOccupant,
    /// Organizational provider identity introduced by an explicit binding.
    OrganizationalProvider,
    /// Provider-neutral communication-actor binding identity.
    CommunicationActor,
    /// Topic-scoped leadership-participation identity.
    LeadershipTopic,
    /// Organizational relation identity.
    OrganizationalRelation,
    /// Owner-selected organizational object.
    OrganizationalObject,
    /// Group-lens authority-profile identity.
    GroupLensProfile,
    /// Directional-service contract identity.
    DirectionalService,
    /// Organizational native-projection identity.
    OrganizationalProjection,
    /// Organizational inspection identity.
    OrganizationalInspection,
    /// Separately tagged participant in the neutral composition graph.
    CompositionParticipant,
    /// Neutral composition responsibility-contract identity.
    CompositionResponsibilityContract,
    /// Neutral composition participant/provider-manifest identity.
    CompositionParticipantManifest,
    /// Provider declared within a composition manifest.
    CompositionProvider,
    /// Neutral composition provider-binding identity.
    CompositionProviderBinding,
    /// Neutral composition entry-contract identity.
    CompositionEntryContract,
    /// Neutral request-translation identity.
    CompositionRequestTranslation,
    /// Neutral result-translation identity.
    CompositionResultTranslation,
    /// Neutral termination-contract identity.
    CompositionTerminationContract,
    /// Neutral settlement-contract identity.
    CompositionSettlementContract,
    /// Neutral continuity-contract identity.
    CompositionContinuityContract,
    /// Typed neutral composition-edge identity.
    CompositionEdge,
    /// Selected neutral experience-path identity.
    ExperiencePath,
    /// Package-owned doctrine resource.
    PackageDoctrine,
    /// Package-owned operation-archetype resource.
    PackageOperationArchetype,
    /// Package-owned concrete operation resource.
    PackageOperation,
    /// Package-owned role-archetype resource.
    PackageRoleArchetype,
    /// Package-owned specialist-role resource.
    PackageRole,
    /// Package-owned stack resource.
    PackageStack,
    /// Package-owned run-mode resource.
    PackageRunMode,
    /// Package-owned policy-family resource.
    PackagePolicyFamily,
    /// Package-owned governing-order resource.
    PackageGoverningOrder,
    /// Package-owned capability-policy resource.
    PackageCapabilityPolicy,
    /// Package-owned tool-recipe resource.
    PackageToolRecipe,
    /// Immutable Package identity.
    Package,
}

impl From<application::AuthoringIdentityKind> for AuthoringIdentityKind {
    fn from(value: application::AuthoringIdentityKind) -> Self {
        match value {
            application::AuthoringIdentityKind::SourceDocument => Self::SourceDocument,
            application::AuthoringIdentityKind::Workspace => Self::Workspace,
            application::AuthoringIdentityKind::Project => Self::Project,
            application::AuthoringIdentityKind::ExecutionWorkspace => Self::ExecutionWorkspace,
            application::AuthoringIdentityKind::Repository => Self::Repository,
            application::AuthoringIdentityKind::ExecutionPath => Self::ExecutionPath,
            application::AuthoringIdentityKind::Module => Self::Module,
            application::AuthoringIdentityKind::ModuleRelease => Self::ModuleRelease,
            application::AuthoringIdentityKind::Component => Self::Component,
            application::AuthoringIdentityKind::Export => Self::Export,
            application::AuthoringIdentityKind::Namespace => Self::Namespace,
            application::AuthoringIdentityKind::Declaration => Self::Declaration,
            application::AuthoringIdentityKind::Resource => Self::Resource,
            application::AuthoringIdentityKind::RunPolicy => Self::RunPolicy,
            application::AuthoringIdentityKind::LifecycleBehavior => Self::LifecycleBehavior,
            application::AuthoringIdentityKind::Capability => Self::Capability,
            application::AuthoringIdentityKind::AuthorityRoot => Self::AuthorityRoot,
            application::AuthoringIdentityKind::ProviderBinding => Self::ProviderBinding,
            application::AuthoringIdentityKind::Machine => Self::Machine,
            application::AuthoringIdentityKind::ResponsibilityDerivation => {
                Self::ResponsibilityDerivation
            }
            application::AuthoringIdentityKind::ResponsibilityProjection => {
                Self::ResponsibilityProjection
            }
            application::AuthoringIdentityKind::ResponsibilityGenealogyLink => {
                Self::ResponsibilityGenealogyLink
            }
            application::AuthoringIdentityKind::LogicalScope => Self::LogicalScope,
            application::AuthoringIdentityKind::LogicalOrganization => Self::LogicalOrganization,
            application::AuthoringIdentityKind::OrganizationalGroup => Self::OrganizationalGroup,
            application::AuthoringIdentityKind::DisciplineTree => Self::DisciplineTree,
            application::AuthoringIdentityKind::OrganizationalRole => Self::OrganizationalRole,
            application::AuthoringIdentityKind::OrganizationalOccupant => {
                Self::OrganizationalOccupant
            }
            application::AuthoringIdentityKind::OrganizationalProvider => {
                Self::OrganizationalProvider
            }
            application::AuthoringIdentityKind::CommunicationActor => Self::CommunicationActor,
            application::AuthoringIdentityKind::LeadershipTopic => Self::LeadershipTopic,
            application::AuthoringIdentityKind::OrganizationalRelation => {
                Self::OrganizationalRelation
            }
            application::AuthoringIdentityKind::OrganizationalObject => Self::OrganizationalObject,
            application::AuthoringIdentityKind::GroupLensProfile => Self::GroupLensProfile,
            application::AuthoringIdentityKind::DirectionalService => Self::DirectionalService,
            application::AuthoringIdentityKind::OrganizationalProjection => {
                Self::OrganizationalProjection
            }
            application::AuthoringIdentityKind::OrganizationalInspection => {
                Self::OrganizationalInspection
            }
            application::AuthoringIdentityKind::CompositionParticipant => {
                Self::CompositionParticipant
            }
            application::AuthoringIdentityKind::CompositionResponsibilityContract => {
                Self::CompositionResponsibilityContract
            }
            application::AuthoringIdentityKind::CompositionParticipantManifest => {
                Self::CompositionParticipantManifest
            }
            application::AuthoringIdentityKind::CompositionProvider => Self::CompositionProvider,
            application::AuthoringIdentityKind::CompositionProviderBinding => {
                Self::CompositionProviderBinding
            }
            application::AuthoringIdentityKind::CompositionEntryContract => {
                Self::CompositionEntryContract
            }
            application::AuthoringIdentityKind::CompositionRequestTranslation => {
                Self::CompositionRequestTranslation
            }
            application::AuthoringIdentityKind::CompositionResultTranslation => {
                Self::CompositionResultTranslation
            }
            application::AuthoringIdentityKind::CompositionTerminationContract => {
                Self::CompositionTerminationContract
            }
            application::AuthoringIdentityKind::CompositionSettlementContract => {
                Self::CompositionSettlementContract
            }
            application::AuthoringIdentityKind::CompositionContinuityContract => {
                Self::CompositionContinuityContract
            }
            application::AuthoringIdentityKind::CompositionEdge => Self::CompositionEdge,
            application::AuthoringIdentityKind::ExperiencePath => Self::ExperiencePath,
            application::AuthoringIdentityKind::PackageDoctrine => Self::PackageDoctrine,
            application::AuthoringIdentityKind::PackageOperationArchetype => {
                Self::PackageOperationArchetype
            }
            application::AuthoringIdentityKind::PackageOperation => Self::PackageOperation,
            application::AuthoringIdentityKind::PackageRoleArchetype => Self::PackageRoleArchetype,
            application::AuthoringIdentityKind::PackageRole => Self::PackageRole,
            application::AuthoringIdentityKind::PackageStack => Self::PackageStack,
            application::AuthoringIdentityKind::PackageRunMode => Self::PackageRunMode,
            application::AuthoringIdentityKind::PackagePolicyFamily => Self::PackagePolicyFamily,
            application::AuthoringIdentityKind::PackageGoverningOrder => {
                Self::PackageGoverningOrder
            }
            application::AuthoringIdentityKind::PackageCapabilityPolicy => {
                Self::PackageCapabilityPolicy
            }
            application::AuthoringIdentityKind::PackageToolRecipe => Self::PackageToolRecipe,
            application::AuthoringIdentityKind::Package => Self::Package,
        }
    }
}

/// One binding-safe semantic identity returned by the public façade.
///
/// Equality preserves the compiler's complete semantic key, including its
/// reference scope. The three serialized fields are a readable description,
/// not a reconstructible complete key or authority: equal descriptions need
/// not identify equal semantic subjects. Bindings retain the opaque identity
/// separately. The identity itself has no compilation-generation affinity;
/// query continuations must retain their exact owning compilation separately.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthoringIdentity {
    kind: AuthoringIdentityKind,
    inner: application::AuthoringIdentity,
}

impl std::fmt::Debug for AuthoringIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthoringIdentity")
            .field("kind", &self.kind())
            .field("value", &self.value())
            .field("authored_value", &self.authored_value())
            .finish()
    }
}

impl serde::Serialize for AuthoringIdentity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct as _;

        let mut description = serializer.serialize_struct("AuthoringIdentity", 3)?;
        description.serialize_field("kind", &self.kind())?;
        description.serialize_field("value", self.value())?;
        description.serialize_field("authored_value", self.authored_value())?;
        description.end()
    }
}

impl AuthoringIdentity {
    /// Return the non-interchangeable identity namespace.
    #[must_use]
    pub const fn kind(&self) -> AuthoringIdentityKind {
        self.kind
    }

    /// Borrow the canonical compiler-owned display form.
    #[must_use]
    pub fn value(&self) -> &str {
        self.inner.value()
    }

    /// Borrow the exact TOML scalar spelling valid in this reference scope.
    #[must_use]
    pub fn authored_value(&self) -> &str {
        self.inner.authored_value()
    }
}

/// A source label and field/index path reported by the compiler.
///
/// This location can refer to an admitted compilation or rejected input. It
/// contains no source text and does not establish that the input was admitted.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AuthoringSourceLocation {
    source_label: String,
    path: Vec<AuthoringInstancePathSegment>,
}

/// Semantic role played by one exact admitted source occurrence.
///
/// This closed projection preserves the linker's definition/reference law for
/// editor protocols without exposing an application-owned graph node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthoringOccurrenceRole {
    /// The occurrence establishes the canonical local definition.
    Definition,
    /// The occurrence resolves to a separately defined identity.
    Reference,
    /// The occurrence carries a content-bearing interface projection.
    Projection,
}

/// Closed class of compiler-proved source correction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthoringCorrectionKind {
    /// Narrow an import visibility ceiling to the selected export's ceiling.
    NarrowImportVisibility,
}

impl From<application::AuthoringCorrectionKind> for AuthoringCorrectionKind {
    fn from(value: application::AuthoringCorrectionKind) -> Self {
        match value {
            application::AuthoringCorrectionKind::NarrowImportVisibility => {
                Self::NarrowImportVisibility
            }
        }
    }
}

/// Binding-safe exact source edit proved by a rejected compiler operation.
///
/// Consumers must match [`Self::expected_source`] at [`Self::path`] before
/// applying [`Self::replacement_source`]. The diagnostic identity identifies
/// the diagnostic that accompanies this correction, not a unique invocation.
/// Neither the location nor the correction grants authority to write source.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AuthoringCorrection {
    kind: AuthoringCorrectionKind,
    source_label: String,
    path: Vec<AuthoringInstancePathSegment>,
    expected_source: String,
    replacement_source: String,
    proof_subject: AuthoringIdentity,
    diagnostic_identity: crate::DiagnosticIdentity,
}

impl AuthoringCorrection {
    pub(crate) fn from_evidence(
        value: &application::AuthoringCorrectionEvidence,
        diagnostic_identity: crate::DiagnosticIdentity,
    ) -> Self {
        Self {
            kind: value.kind().into(),
            source_label: value.source_label().to_owned(),
            path: value.path().iter().map(Into::into).collect(),
            expected_source: value.expected_source().to_owned(),
            replacement_source: value.replacement_source().to_owned(),
            proof_subject: authoring_identity(value.proof_subject()),
            diagnostic_identity,
        }
    }

    /// Return the compiler-proved correction class.
    #[must_use]
    pub const fn kind(&self) -> AuthoringCorrectionKind {
        self.kind
    }

    /// Borrow the compiler logical label of the edited source.
    #[must_use]
    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// Borrow the exact decoded-key and array-index path.
    #[must_use]
    pub fn path(&self) -> &[AuthoringInstancePathSegment] {
        &self.path
    }

    /// Borrow the exact scalar bytes required at the edit range.
    #[must_use]
    pub fn expected_source(&self) -> &str {
        &self.expected_source
    }

    /// Borrow the exact replacement scalar bytes selected by the compiler.
    #[must_use]
    pub fn replacement_source(&self) -> &str {
        &self.replacement_source
    }

    /// Borrow the resolved semantic subject that proves the correction.
    #[must_use]
    pub const fn proof_subject(&self) -> &AuthoringIdentity {
        &self.proof_subject
    }

    /// Borrow the exact rejected diagnostic identity.
    #[must_use]
    pub const fn diagnostic_identity(&self) -> &crate::DiagnosticIdentity {
        &self.diagnostic_identity
    }
}

impl From<application::AuthoringOccurrenceRole> for AuthoringOccurrenceRole {
    fn from(value: application::AuthoringOccurrenceRole) -> Self {
        match value {
            application::AuthoringOccurrenceRole::Definition => Self::Definition,
            application::AuthoringOccurrenceRole::Reference => Self::Reference,
            application::AuthoringOccurrenceRole::Projection => Self::Projection,
        }
    }
}

impl AuthoringSourceLocation {
    /// Copy one structured location already resolved by the application owner.
    pub(crate) fn from_application(value: &application::WorkspaceAuthoringLocation) -> Self {
        Self::new(
            value.source_label().to_owned(),
            value.path().iter().map(Into::into).collect(),
        )
    }

    /// Construct an owned logical source location inside the façade.
    pub(crate) fn new(source_label: String, path: Vec<AuthoringInstancePathSegment>) -> Self {
        Self { source_label, path }
    }

    /// Borrow the compiler logical source label.
    #[must_use]
    pub fn source_label(&self) -> &str {
        &self.source_label
    }

    /// Borrow the exact decoded-key and array-index path.
    #[must_use]
    pub fn path(&self) -> &[AuthoringInstancePathSegment] {
        &self.path
    }
}

/// Semantic navigation result for one exact admitted source occurrence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AuthoringNavigation {
    identity: Option<AuthoringIdentity>,
    occurrence_role: Option<AuthoringOccurrenceRole>,
    definition: Option<AuthoringSourceLocation>,
    references: Vec<AuthoringSourceLocation>,
    alternatives: Vec<AuthoringIdentity>,
    references_truncated: bool,
    alternatives_truncated: bool,
}

impl AuthoringNavigation {
    /// Borrow the resolved identity, or `None` for a structural-only coordinate.
    #[must_use]
    pub const fn identity(&self) -> Option<&AuthoringIdentity> {
        self.identity.as_ref()
    }

    /// Return the exact semantic role of the queried occurrence.
    #[must_use]
    pub const fn occurrence_role(&self) -> Option<AuthoringOccurrenceRole> {
        self.occurrence_role
    }

    /// Borrow the canonical definition when the admitted closure contains one.
    #[must_use]
    pub const fn definition(&self) -> Option<&AuthoringSourceLocation> {
        self.definition.as_ref()
    }

    /// Borrow all retained references in deterministic source/path order.
    #[must_use]
    pub fn references(&self) -> &[AuthoringSourceLocation] {
        &self.references
    }

    /// Borrow same-namespace identities eligible for value completion.
    #[must_use]
    pub fn alternatives(&self) -> &[AuthoringIdentity] {
        &self.alternatives
    }

    /// Whether the fixed public reference ceiling omitted source occurrences.
    #[must_use]
    pub const fn references_truncated(&self) -> bool {
        self.references_truncated
    }

    /// Whether the fixed public completion ceiling omitted eligible identities.
    #[must_use]
    pub const fn alternatives_truncated(&self) -> bool {
        self.alternatives_truncated
    }
}

impl From<AuthoringPathSegment> for application::AuthoringPathSegment {
    fn from(value: AuthoringPathSegment) -> Self {
        match value {
            AuthoringPathSegment::Field(field) => Self::Field(field),
            AuthoringPathSegment::ArrayItem => Self::ArrayItem,
        }
    }
}

/// Exact source and structural path requested by an editor client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringInspectionRequest {
    source_label: String,
    path: Vec<AuthoringPathSegment>,
}

impl AuthoringInspectionRequest {
    /// Bind one logical source label to a syntax-derived structural path.
    ///
    /// Construction does not parse source, infer its role, or admit a semantic
    /// identity. The selected compilation remains the source-role owner.
    #[must_use]
    pub fn new(source_label: String, path: Vec<AuthoringPathSegment>) -> Self {
        Self { source_label, path }
    }
}

/// Availability of structural help at one admitted source coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthoringInspectionState {
    /// The generated grammar contains the selected path.
    Available,
    /// The admitted source is substantive text, not TOML grammar.
    TextDocument,
    /// The compilation does not own the requested logical label.
    UnknownDocument,
    /// The TOML source is known but the structural path is outside its grammar.
    UnknownPath,
}

/// Source-custody class whose canonical capture limits protect editor ingress.
///
/// The classes are intentionally coarser than [`StandaloneAuthoringRole`]:
/// roles that enter the same application capture boundary must share its
/// resource law instead of acquiring leaf-specific limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoringIngressClass {
    /// Portable workspace-language documents.
    Workspace,
    /// Private selections, bindings, locks, and lifecycle guidance.
    Private,
    /// Immutable Package distribution members.
    Package,
}

/// Finite source limits enforced before tolerant editor parsing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthoringIngressLimits {
    max_documents: usize,
    max_document_bytes: u64,
    max_total_bytes: u64,
}

impl AuthoringIngressLimits {
    /// Maximum logical documents retained for this custody class.
    #[must_use]
    pub const fn max_documents(self) -> usize {
        self.max_documents
    }

    /// Maximum UTF-8 bytes retained from one logical document.
    #[must_use]
    pub const fn max_document_bytes(self) -> u64 {
        self.max_document_bytes
    }

    /// Maximum aggregate bytes retained for this custody class.
    #[must_use]
    pub const fn max_total_bytes(self) -> u64 {
        self.max_total_bytes
    }
}

/// TOML-compatible closed literal offered by the Rust-owned grammar.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum AuthoringLiteral {
    /// UTF-8 string member.
    String(String),
    /// Signed integer member.
    Integer(i64),
    /// Boolean member.
    Boolean(bool),
}

/// One literal and its variant-specific producer documentation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AuthoringLiteralFact {
    value: AuthoringLiteral,
    description: Option<String>,
}

impl AuthoringLiteralFact {
    /// Exact literal admitted by the generated node.
    #[must_use]
    pub const fn value(&self) -> &AuthoringLiteral {
        &self.value
    }

    /// Variant-specific producer documentation.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Structural value categories admitted by one generated node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
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

/// Requiredness of one child field across structural variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthoringFieldPresence {
    /// Every applicable variant requires the field.
    Required,
    /// No applicable variant requires the field.
    Optional,
    /// Requiredness depends on the selected structural variant.
    VariantDependent,
}

/// One child key offered at the selected structural path.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AuthoringFieldFact {
    name: String,
    description: Option<String>,
    presence: AuthoringFieldPresence,
    value_kinds: Vec<AuthoringValueKind>,
    allowed_values: Vec<AuthoringLiteralFact>,
}

impl AuthoringFieldFact {
    /// Exact authored key spelling.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Producer documentation for this field.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Requiredness across applicable structural variants.
    #[must_use]
    pub const fn presence(&self) -> AuthoringFieldPresence {
        self.presence
    }

    /// Structural value categories admitted for this field.
    #[must_use]
    pub fn value_kinds(&self) -> &[AuthoringValueKind] {
        &self.value_kinds
    }

    /// Closed literal vocabulary for this field.
    #[must_use]
    pub fn allowed_values(&self) -> &[AuthoringLiteralFact] {
        &self.allowed_values
    }
}

/// Owned, bounded structural facts for one editor position.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AuthoringNodeFact {
    description: Option<String>,
    value_kinds: Vec<AuthoringValueKind>,
    allowed_values: Vec<AuthoringLiteralFact>,
    fields: Vec<AuthoringFieldFact>,
    format: Option<String>,
    pattern: Option<String>,
    minimum: Option<i64>,
    maximum: Option<i64>,
    min_length: Option<u64>,
    max_length: Option<u64>,
    min_items: Option<u64>,
    max_items: Option<u64>,
}

impl AuthoringNodeFact {
    /// Producer documentation for this exact structural context.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Structural categories admitted at this path.
    #[must_use]
    pub fn value_kinds(&self) -> &[AuthoringValueKind] {
        &self.value_kinds
    }

    /// Closed literal vocabulary admitted at this path.
    #[must_use]
    pub fn allowed_values(&self) -> &[AuthoringLiteralFact] {
        &self.allowed_values
    }

    /// Deterministically ordered child fields.
    #[must_use]
    pub fn fields(&self) -> &[AuthoringFieldFact] {
        &self.fields
    }

    /// Optional generated string format.
    #[must_use]
    pub fn format(&self) -> Option<&str> {
        self.format.as_deref()
    }

    /// Optional generated string pattern.
    #[must_use]
    pub fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    /// Inclusive integer lower bound.
    #[must_use]
    pub const fn minimum(&self) -> Option<i64> {
        self.minimum
    }

    /// Inclusive integer upper bound.
    #[must_use]
    pub const fn maximum(&self) -> Option<i64> {
        self.maximum
    }

    /// Inclusive string-length lower bound.
    #[must_use]
    pub const fn min_length(&self) -> Option<u64> {
        self.min_length
    }

    /// Inclusive string-length upper bound.
    #[must_use]
    pub const fn max_length(&self) -> Option<u64> {
        self.max_length
    }

    /// Inclusive collection-size lower bound.
    #[must_use]
    pub const fn min_items(&self) -> Option<u64> {
        self.min_items
    }

    /// Inclusive collection-size upper bound.
    #[must_use]
    pub const fn max_items(&self) -> Option<u64> {
        self.max_items
    }
}

/// Complete source-role and schema-derived response for one request.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AuthoringInspection {
    state: AuthoringInspectionState,
    schema_id: Option<String>,
    definition: Option<String>,
    node: Option<AuthoringNodeFact>,
}

impl AuthoringInspection {
    /// Availability classification for this exact request.
    #[must_use]
    pub const fn state(&self) -> AuthoringInspectionState {
        self.state
    }

    /// Descriptor identity of the selected TOML grammar, when one exists.
    #[must_use]
    pub fn schema_id(&self) -> Option<&str> {
        self.schema_id.as_deref()
    }

    /// Selected named definition for a shared definition-set grammar.
    #[must_use]
    pub fn definition(&self) -> Option<&str> {
        self.definition.as_deref()
    }

    /// Structural facts when [`Self::state`] is
    /// [`AuthoringInspectionState::Available`].
    #[must_use]
    pub const fn node(&self) -> Option<&AuthoringNodeFact> {
        self.node.as_ref()
    }
}

/// Typed source retained behind an API-owned authoring diagnostic.
#[derive(Debug, thiserror::Error)]
enum AuthoringQueryError {
    /// The retained continuation does not belong to the complete requested query.
    #[error("authoring continuation does not match its requested query")]
    ContinuationMismatch,
    /// The syntax path exceeds the fixed editor-query bound.
    #[error("authoring path has {observed} segments; maximum is {maximum}")]
    PathTooDeep {
        /// Supplied structural depth.
        observed: usize,
        /// Fixed API ceiling.
        maximum: usize,
    },
    /// Generated metadata contradicted its registry or reference graph.
    #[error("generated authoring metadata is inconsistent: {0}")]
    Metadata(#[source] AuthoringMetadataFailure),
}

/// Closed internal classification of an embedded metadata contradiction.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
enum AuthoringMetadataFailure {
    /// Embedded schema lookup or annotation parsing failed.
    #[error("embedded authoring registry is incoherent")]
    Registry,
    /// A compiler-selected named definition is absent.
    #[error("a selected authoring definition is absent")]
    UnknownDefinition,
    /// An embedded authoring reference cannot be resolved.
    #[error("an embedded authoring reference is invalid")]
    InvalidReference,
    /// Reference expansion exceeded its finite safety bound.
    #[error("authoring reference expansion exceeded its bound")]
    ReferenceDepth,
}

impl PortableWorkspaceCompilation {
    /// Inspect structural authoring facts for one admitted portable document.
    ///
    /// Only compiler-selected source roles are consulted. The path must come
    /// from a syntax layer; this method does not parse TOML or resolve semantic
    /// identities.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the path exceeds its fixed bound or generated
    /// annotation metadata contradicts its registry. Unknown documents and
    /// paths are ordinary typed [`AuthoringInspectionState`] values.
    pub fn inspect_authoring(
        &self,
        request: AuthoringInspectionRequest,
    ) -> Result<AuthoringInspection, ApiError> {
        inspect_source_map(self.application_universe().authoring_sources(), request)
            .map_err(|error| authoring_query_failure(&self.diagnostic_identity, error))
    }

    /// Resolve one admitted source occurrence through the canonical linker.
    ///
    /// The result carries logical labels and exact instance paths only. The
    /// calling editor owns live text and converts those paths into protocol
    /// ranges; neither source bytes nor compiler graph nodes cross the façade.
    #[must_use]
    pub fn navigate_authoring(&self, request: AuthoringNavigationRequest) -> AuthoringNavigation {
        navigate(self.application_universe().authoring_semantics(), request)
    }
}

fn navigate(
    semantics: &application::AuthoringSemanticIndex,
    request: AuthoringNavigationRequest,
) -> AuthoringNavigation {
    const MAX_NAVIGATION_MEMBERS: usize = 256;

    let path = request
        .path
        .into_iter()
        .map(application::AuthoringInstancePathSegment::from)
        .collect::<Vec<_>>();
    let Some(occurrence) = semantics.occurrence(&request.source_label, &path) else {
        let mut alternatives = semantics
            .completion_alternatives_at(&request.source_label, &path)
            .take(MAX_NAVIGATION_MEMBERS + 1)
            .map(|occurrence| authoring_identity(occurrence.identity()))
            .collect::<Vec<_>>();
        let alternatives_truncated = alternatives.len() > MAX_NAVIGATION_MEMBERS;
        alternatives.truncate(MAX_NAVIGATION_MEMBERS);
        return AuthoringNavigation {
            identity: None,
            occurrence_role: None,
            definition: None,
            references: Vec::new(),
            alternatives,
            references_truncated: false,
            alternatives_truncated,
        };
    };
    let identity = occurrence.identity();
    let reference_count = semantics.reference_count(identity);
    let references = semantics
        .references(identity)
        .take(MAX_NAVIGATION_MEMBERS)
        .map(source_location)
        .collect();
    let mut alternatives = semantics
        .completion_alternatives(identity)
        .take(MAX_NAVIGATION_MEMBERS + 1)
        .map(|occurrence| authoring_identity(occurrence.identity()))
        .collect::<Vec<_>>();
    let alternatives_truncated = alternatives.len() > MAX_NAVIGATION_MEMBERS;
    alternatives.truncate(MAX_NAVIGATION_MEMBERS);
    AuthoringNavigation {
        identity: Some(authoring_identity(identity)),
        occurrence_role: Some(occurrence.role().into()),
        definition: semantics.definition(identity).map(source_location),
        references,
        alternatives,
        references_truncated: reference_count > MAX_NAVIGATION_MEMBERS,
        alternatives_truncated,
    }
}

fn authoring_identity(value: &application::AuthoringIdentity) -> AuthoringIdentity {
    AuthoringIdentity {
        kind: value.kind().into(),
        inner: value.clone(),
    }
}

fn source_location(value: &application::AuthoringOccurrence) -> AuthoringSourceLocation {
    AuthoringSourceLocation {
        source_label: value.source_label().to_owned(),
        path: value.path().iter().map(Into::into).collect(),
    }
}

impl EffectiveProfileCompilation {
    /// Inspect structural authoring facts across workspace, private, and Package sources.
    ///
    /// The effective-profile phase retains only logical labels and exact grammar
    /// selections. Source bytes, syntax trees, provider payloads, and secrets do
    /// not cross this projection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the path exceeds its fixed bound or embedded
    /// generated metadata is incoherent. Absent documents and paths remain typed
    /// result states.
    pub fn inspect_authoring(
        &self,
        request: AuthoringInspectionRequest,
    ) -> Result<AuthoringInspection, ApiError> {
        inspect_source_map(self.compilation.authoring_sources(), request)
            .map_err(|error| authoring_query_failure(&self.diagnostic_identity, error))
    }

    /// Resolve one source occurrence retained by the effective compiler phase.
    #[must_use]
    pub fn navigate_authoring(&self, request: AuthoringNavigationRequest) -> AuthoringNavigation {
        navigate(self.compilation.authoring_semantics(), request)
    }
}

impl PaleApi {
    /// Return the application-owned capture ceilings for one authoring class.
    ///
    /// An editor applies these values before constructing mutable text or a
    /// tolerant syntax tree. The compiler repeats the same checks at its trust
    /// boundary; this early projection is resource admission, not semantic
    /// validation and grants no authority to otherwise unused documents.
    #[must_use]
    pub const fn authoring_ingress_limits(class: AuthoringIngressClass) -> AuthoringIngressLimits {
        match class {
            AuthoringIngressClass::Workspace => AuthoringIngressLimits {
                max_documents: application::CapturedWorkspaceSource::MAX_DOCUMENTS,
                max_document_bytes: application::CapturedWorkspaceDocument::MAX_BYTES as u64,
                max_total_bytes: application::CapturedWorkspaceSource::MAX_DOCUMENTS as u64
                    * application::CapturedWorkspaceDocument::MAX_BYTES as u64,
            },
            AuthoringIngressClass::Private => AuthoringIngressLimits {
                max_documents: application::CapturedPrivateWorkspaceInputs::MAX_DOCUMENTS,
                max_document_bytes: application::CapturedPrivateDocument::MAX_BYTES as u64,
                max_total_bytes: application::CapturedPrivateWorkspaceInputs::MAX_TOTAL_BYTES,
            },
            AuthoringIngressClass::Package => AuthoringIngressLimits {
                max_documents: pale_application::compile::CapturedPackageSource::MAX_ENTRIES,
                max_document_bytes: pale_application::compile::CapturedPackageDocument::MAX_BYTES,
                max_total_bytes: pale_application::compile::CapturedPackageSource::MAX_TOTAL_BYTES,
            },
        }
    }

    /// Validate one explicitly typed source without selecting project context.
    ///
    /// The application-owned parser and generated grammar are applied to the
    /// exact supplied bytes. This operation is intended for ordinary editor
    /// analysis when a complete effective-profile context is unavailable; it
    /// proves neither cross-record coherence nor runtime readiness.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ApiError`] for an unsafe label, bounded-input failure,
    /// TOML syntax failure, or contract violation.
    pub fn validate_authoring_document(
        &self,
        role: StandaloneAuthoringRole,
        source_label: String,
        bytes: Vec<u8>,
    ) -> Result<(), crate::ApiError> {
        application::validate_standalone_authoring_document(
            application::AuthoringSourceRole::from(role),
            &source_label,
            &bytes,
        )
        .map_err(|error| {
            crate::portable_workspace::workspace_failure(&self.diagnostic_identity, error)
        })
    }

    /// Inspect a standalone private grammar selected explicitly by its host.
    ///
    /// This projection does not admit source bytes, choose a private value, or
    /// prove a machine realization. It exists so an editor can provide the
    /// same generated structural help before a project-specific effective
    /// profile is selected.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the path exceeds its bound or generated
    /// authoring metadata contradicts the registered contract.
    pub fn inspect_standalone_authoring(
        &self,
        role: StandaloneAuthoringRole,
        path: Vec<AuthoringPathSegment>,
    ) -> Result<AuthoringInspection, ApiError> {
        let grammar = match application::AuthoringSourceRole::from(role).document_kind() {
            application::AuthoringDocumentKind::Toml(grammar) => grammar,
            application::AuthoringDocumentKind::PackagePage => {
                application::AuthoringTomlGrammar::record("package-page.v1")
            }
            application::AuthoringDocumentKind::Text => {
                unreachable!("standalone authoring roles select declaration grammars")
            }
        };
        inspect_grammar(grammar, path)
            .map_err(|error| authoring_query_failure(&self.diagnostic_identity, error))
    }
}

fn inspect_source_map(
    sources: &application::AuthoringSourceMap,
    request: AuthoringInspectionRequest,
) -> Result<AuthoringInspection, AuthoringQueryError> {
    if request.path.len() > AUTHORING_PATH_SEGMENT_LIMIT {
        return Err(AuthoringQueryError::PathTooDeep {
            observed: request.path.len(),
            maximum: AUTHORING_PATH_SEGMENT_LIMIT,
        });
    }
    let Some(kind) = sources.document(&request.source_label) else {
        return Ok(state(AuthoringInspectionState::UnknownDocument, None));
    };
    let grammar = match kind {
        application::AuthoringDocumentKind::Toml(grammar) => grammar,
        application::AuthoringDocumentKind::PackagePage => {
            application::AuthoringTomlGrammar::record("package-page.v1")
        }
        application::AuthoringDocumentKind::Text => {
            return Ok(state(AuthoringInspectionState::TextDocument, None))
        }
    };
    let schema_id = Some(grammar.schema_id().to_owned());
    let definition = grammar.definition_name().map(str::to_owned);
    inspect_grammar_inner(grammar, request.path, schema_id, definition)
}

fn inspect_grammar(
    grammar: application::AuthoringTomlGrammar,
    path: Vec<AuthoringPathSegment>,
) -> Result<AuthoringInspection, AuthoringQueryError> {
    if path.len() > AUTHORING_PATH_SEGMENT_LIMIT {
        return Err(AuthoringQueryError::PathTooDeep {
            observed: path.len(),
            maximum: AUTHORING_PATH_SEGMENT_LIMIT,
        });
    }
    inspect_grammar_inner(
        grammar,
        path,
        Some(grammar.schema_id().to_owned()),
        grammar.definition_name().map(str::to_owned),
    )
}

fn inspect_grammar_inner(
    grammar: application::AuthoringTomlGrammar,
    path: Vec<AuthoringPathSegment>,
    schema_id: Option<String>,
    definition: Option<String>,
) -> Result<AuthoringInspection, AuthoringQueryError> {
    let path = path
        .into_iter()
        .map(application::AuthoringPathSegment::from)
        .collect::<Vec<_>>();
    match application::inspect_authoring_node(grammar, &path) {
        Ok(node) => Ok(AuthoringInspection {
            state: AuthoringInspectionState::Available,
            schema_id,
            definition,
            node: Some(node.into()),
        }),
        Err(application::AuthoringSchemaError::UnknownPath) => Ok(AuthoringInspection {
            state: AuthoringInspectionState::UnknownPath,
            schema_id,
            definition,
            node: None,
        }),
        Err(error) => Err(AuthoringQueryError::Metadata(project_metadata_failure(
            error,
        ))),
    }
}

fn project_metadata_failure(error: application::AuthoringSchemaError) -> AuthoringMetadataFailure {
    match error {
        application::AuthoringSchemaError::Registry(_) => AuthoringMetadataFailure::Registry,
        application::AuthoringSchemaError::UnknownDefinition { .. } => {
            AuthoringMetadataFailure::UnknownDefinition
        }
        application::AuthoringSchemaError::InvalidReference(_) => {
            AuthoringMetadataFailure::InvalidReference
        }
        application::AuthoringSchemaError::ReferenceDepth => {
            AuthoringMetadataFailure::ReferenceDepth
        }
        application::AuthoringSchemaError::UnknownPath => {
            unreachable!("unknown authoring paths are projected as inspection state")
        }
    }
}

/// Bind a retained authoring continuation mismatch without exposing its coordinate.
pub(crate) fn authoring_continuation_failure(identity: &RecordSchemaIdentity) -> ApiError {
    authoring_query_failure(identity, AuthoringQueryError::ContinuationMismatch)
}

/// Bind one authoring-query failure to the façade's canonical diagnostic owner.
fn authoring_query_failure(
    identity: &RecordSchemaIdentity,
    error: AuthoringQueryError,
) -> ApiError {
    let (code, stage, disposition, summary, retry, help, facts) = match &error {
        AuthoringQueryError::ContinuationMismatch => (
            "authoring_continuation_mismatch",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "authoring continuation does not match its requested query",
            DiagnosticRetryV1::AfterCorrection,
            vec![
                "Restart the authoring query without the prior continuation, then continue only from the new page.",
            ],
            Vec::new(),
        ),
        AuthoringQueryError::PathTooDeep { observed, maximum } => (
            "authoring_path_too_deep",
            DiagnosticStageV1::InputAdmission,
            DiagnosticDispositionV1::InvalidInput,
            "authoring path exceeds the editor-query bound",
            DiagnosticRetryV1::AfterCorrection,
            vec!["Reduce the authoring path to the stated maximum, then retry."],
            vec![
                authoring_limit_fact("unit", "segments"),
                authoring_limit_fact("observed", &observed.to_string()),
                authoring_limit_fact("maximum", &maximum.to_string()),
            ],
        ),
        AuthoringQueryError::Metadata(_) => (
            "authoring_metadata_incoherent",
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            "generated authoring metadata is inconsistent",
            DiagnosticRetryV1::NotIndicated,
            Vec::new(),
            Vec::new(),
        ),
    };
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code,
            stage,
            disposition,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help,
            facts,
        },
    )
}

fn authoring_limit_fact(key: &str, value: &str) -> DiagnosticFactV1Wire {
    DiagnosticFactV1Wire {
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

#[cfg(test)]
#[path = "../tests/internal/authoring.rs"]
mod tests;

fn state(state: AuthoringInspectionState, schema_id: Option<String>) -> AuthoringInspection {
    AuthoringInspection {
        state,
        schema_id,
        definition: None,
        node: None,
    }
}

impl From<application::AuthoringLiteral> for AuthoringLiteral {
    fn from(value: application::AuthoringLiteral) -> Self {
        match value {
            application::AuthoringLiteral::String(value) => Self::String(value),
            application::AuthoringLiteral::Integer(value) => Self::Integer(value),
            application::AuthoringLiteral::Boolean(value) => Self::Boolean(value),
        }
    }
}

impl From<application::AuthoringLiteralFact> for AuthoringLiteralFact {
    fn from(value: application::AuthoringLiteralFact) -> Self {
        Self {
            value: value.value.into(),
            description: value.description,
        }
    }
}

impl From<application::AuthoringValueKind> for AuthoringValueKind {
    fn from(value: application::AuthoringValueKind) -> Self {
        match value {
            application::AuthoringValueKind::Boolean => Self::Boolean,
            application::AuthoringValueKind::Integer => Self::Integer,
            application::AuthoringValueKind::String => Self::String,
            application::AuthoringValueKind::Array => Self::Array,
            application::AuthoringValueKind::Table => Self::Table,
        }
    }
}

impl From<application::AuthoringFieldPresence> for AuthoringFieldPresence {
    fn from(value: application::AuthoringFieldPresence) -> Self {
        match value {
            application::AuthoringFieldPresence::Required => Self::Required,
            application::AuthoringFieldPresence::Optional => Self::Optional,
            application::AuthoringFieldPresence::VariantDependent => Self::VariantDependent,
        }
    }
}

impl From<application::AuthoringFieldFact> for AuthoringFieldFact {
    fn from(value: application::AuthoringFieldFact) -> Self {
        Self {
            name: value.name,
            description: value.description,
            presence: value.presence.into(),
            value_kinds: value.value_kinds.into_iter().map(Into::into).collect(),
            allowed_values: value.allowed_values.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<application::AuthoringNodeFact> for AuthoringNodeFact {
    fn from(value: application::AuthoringNodeFact) -> Self {
        Self {
            description: value.description,
            value_kinds: value.value_kinds.into_iter().map(Into::into).collect(),
            allowed_values: value.allowed_values.into_iter().map(Into::into).collect(),
            fields: value.fields.into_iter().map(Into::into).collect(),
            format: value.format,
            pattern: value.pattern,
            minimum: value.minimum,
            maximum: value.maximum,
            min_length: value.min_length,
            max_length: value.max_length,
            min_items: value.min_items,
            max_items: value.max_items,
        }
    }
}
