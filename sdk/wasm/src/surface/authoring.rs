// WebAssembly authoring projection over the API-owned portable authoring surface.

/// Source-custody class whose API-owned limits protect authoring ingress.
#[wasm_bindgen(js_name = AuthoringIngressClass)]
#[derive(Clone, Copy)]
pub enum WasmAuthoringIngressClass {
    /// Portable workspace-language documents.
    Workspace,
    /// Private selections, bindings, locks, and lifecycle guidance.
    Private,
    /// Immutable Package distribution members.
    Package,
}

impl From<WasmAuthoringIngressClass> for AuthoringIngressClass {
    fn from(value: WasmAuthoringIngressClass) -> Self {
        match value {
            WasmAuthoringIngressClass::Workspace => Self::Workspace,
            WasmAuthoringIngressClass::Private => Self::Private,
            WasmAuthoringIngressClass::Package => Self::Package,
        }
    }
}

/// Explicit standalone grammar role selected by a WebAssembly host.
#[wasm_bindgen(js_name = StandaloneAuthoringRole)]
#[derive(Clone, Copy)]
pub enum WasmStandaloneAuthoringRole {
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

impl From<WasmStandaloneAuthoringRole> for StandaloneAuthoringRole {
    fn from(value: WasmStandaloneAuthoringRole) -> Self {
        match value {
            WasmStandaloneAuthoringRole::WorkspaceManifest => Self::WorkspaceManifest,
            WasmStandaloneAuthoringRole::WorkspaceModule => Self::WorkspaceModule,
            WasmStandaloneAuthoringRole::WorkspaceLanguagePrimitives => {
                Self::WorkspaceLanguagePrimitives
            }
            WasmStandaloneAuthoringRole::ResponsibilityDerivation => Self::ResponsibilityDerivation,
            WasmStandaloneAuthoringRole::OrganizationalTranslation => {
                Self::OrganizationalTranslation
            }
            WasmStandaloneAuthoringRole::ExperienceComposition => Self::ExperienceComposition,
            WasmStandaloneAuthoringRole::WorkspaceResolutionLock => Self::WorkspaceResolutionLock,
            WasmStandaloneAuthoringRole::WorkspaceUniverseLock => Self::WorkspaceUniverseLock,
            WasmStandaloneAuthoringRole::Capabilities => Self::Capabilities,
            WasmStandaloneAuthoringRole::MachineBinding => Self::MachineBinding,
            WasmStandaloneAuthoringRole::DistributionLock => Self::DistributionLock,
            WasmStandaloneAuthoringRole::PackageManifest => Self::PackageManifest,
            WasmStandaloneAuthoringRole::PackageDoctrine => Self::PackageDoctrine,
            WasmStandaloneAuthoringRole::PackageOperations => Self::PackageOperations,
            WasmStandaloneAuthoringRole::PackageRoles => Self::PackageRoles,
            WasmStandaloneAuthoringRole::PackageStacks => Self::PackageStacks,
            WasmStandaloneAuthoringRole::PackageRunModes => Self::PackageRunModes,
            WasmStandaloneAuthoringRole::PackagePolicyFamily => Self::PackagePolicyFamily,
            WasmStandaloneAuthoringRole::PackageGoverningOrder => Self::PackageGoverningOrder,
            WasmStandaloneAuthoringRole::PackageCapabilityPolicy => Self::PackageCapabilityPolicy,
            WasmStandaloneAuthoringRole::PackageToolRecipe => Self::PackageToolRecipe,
        }
    }
}

/// One structural path step supplied by a tolerant host parser.
#[wasm_bindgen(js_name = AuthoringPathSegment)]
pub struct WasmAuthoringPathSegment {
    inner: AuthoringPathSegment,
}

#[wasm_bindgen(js_class = AuthoringPathSegment)]
impl WasmAuthoringPathSegment {
    /// Construct one exact field step.
    #[wasm_bindgen(js_name = field)]
    pub fn field(value: String) -> Self {
        Self {
            inner: AuthoringPathSegment::Field(value),
        }
    }

    /// Construct one structural array-item step.
    #[wasm_bindgen(js_name = arrayItem)]
    pub fn array_item() -> Self {
        Self {
            inner: AuthoringPathSegment::ArrayItem,
        }
    }
}

/// Exact instance-path segment family.
#[wasm_bindgen(js_name = AuthoringInstancePathSegmentKind)]
#[derive(Clone, Copy)]
pub enum WasmAuthoringInstancePathSegmentKind {
    /// Exact decoded key.
    Field,
    /// Exact array or array-of-tables index.
    Index,
}

/// One exact path step used for semantic authoring navigation.
#[wasm_bindgen(js_name = AuthoringInstancePathSegment)]
pub struct WasmAuthoringInstancePathSegment {
    inner: AuthoringInstancePathSegment,
}

impl WasmAuthoringInstancePathSegment {
    fn from_api(inner: &AuthoringInstancePathSegment) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[wasm_bindgen(js_class = AuthoringInstancePathSegment)]
impl WasmAuthoringInstancePathSegment {
    /// Construct one exact field step.
    #[wasm_bindgen(js_name = field)]
    pub fn field(value: String) -> Self {
        Self {
            inner: AuthoringInstancePathSegment::Field(value),
        }
    }

    /// Construct one exact array-member step.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` when the value is not a JavaScript number
    /// representing an integer from zero through `u32::MAX`.
    #[wasm_bindgen(js_name = index)]
    pub fn index(
        #[wasm_bindgen(unchecked_param_type = "number")] value: JsValue,
    ) -> Result<Self, JsValue> {
        let value = javascript_u32(&value, u32::MAX, "authoring instance-path index")?;
        Ok(Self {
            inner: AuthoringInstancePathSegment::Index(value as usize),
        })
    }

    /// Return the exact segment family.
    pub fn kind(&self) -> WasmAuthoringInstancePathSegmentKind {
        match self.inner {
            AuthoringInstancePathSegment::Field(_) => WasmAuthoringInstancePathSegmentKind::Field,
            AuthoringInstancePathSegment::Index(_) => WasmAuthoringInstancePathSegmentKind::Index,
        }
    }

    /// Return the decoded key for a field segment.
    #[wasm_bindgen(getter = fieldValue)]
    pub fn field_value(&self) -> Option<String> {
        match &self.inner {
            AuthoringInstancePathSegment::Field(value) => Some(value.clone()),
            AuthoringInstancePathSegment::Index(_) => None,
        }
    }

    /// Return the exact array index as a non-lossy JavaScript `bigint`.
    #[wasm_bindgen(getter = indexValue)]
    pub fn index_value(&self) -> Option<u64> {
        match self.inner {
            AuthoringInstancePathSegment::Field(_) => None,
            AuthoringInstancePathSegment::Index(value) => Some(value as u64),
        }
    }
}

/// API-owned finite source limits for one custody class.
#[wasm_bindgen(js_name = AuthoringIngressLimits)]
pub struct WasmAuthoringIngressLimits {
    inner: AuthoringIngressLimits,
}

#[wasm_bindgen(js_class = AuthoringIngressLimits)]
impl WasmAuthoringIngressLimits {
    /// Maximum logical documents as a non-lossy JavaScript `bigint`.
    #[wasm_bindgen(getter = maxDocuments)]
    pub fn max_documents(&self) -> u64 {
        self.inner.max_documents() as u64
    }

    /// Maximum bytes retained from one logical document.
    #[wasm_bindgen(getter = maxDocumentBytes)]
    pub fn max_document_bytes(&self) -> u64 {
        self.inner.max_document_bytes()
    }

    /// Maximum aggregate bytes retained for the custody class.
    #[wasm_bindgen(getter = maxTotalBytes)]
    pub fn max_total_bytes(&self) -> u64 {
        self.inner.max_total_bytes()
    }
}

/// Availability of structural authoring help at one coordinate.
#[wasm_bindgen(js_name = AuthoringInspectionState)]
#[derive(Clone, Copy)]
pub enum WasmAuthoringInspectionState {
    /// The generated grammar contains the selected path.
    Available,
    /// The source is substantive text rather than TOML grammar.
    TextDocument,
    /// The compilation does not own the requested label.
    UnknownDocument,
    /// The structural path is outside the selected grammar.
    UnknownPath,
}

impl From<AuthoringInspectionState> for WasmAuthoringInspectionState {
    fn from(value: AuthoringInspectionState) -> Self {
        match value {
            AuthoringInspectionState::Available => Self::Available,
            AuthoringInspectionState::TextDocument => Self::TextDocument,
            AuthoringInspectionState::UnknownDocument => Self::UnknownDocument,
            AuthoringInspectionState::UnknownPath => Self::UnknownPath,
        }
    }
}

/// Structural value category admitted by one generated node.
#[wasm_bindgen(js_name = AuthoringValueKind)]
#[derive(Clone, Copy)]
pub enum WasmAuthoringValueKind {
    /// Boolean value.
    Boolean,
    /// Signed integer value.
    Integer,
    /// UTF-8 string value.
    String,
    /// Ordered array value.
    Array,
    /// TOML table or inline-table value.
    Table,
}

impl From<AuthoringValueKind> for WasmAuthoringValueKind {
    fn from(value: AuthoringValueKind) -> Self {
        match value {
            AuthoringValueKind::Boolean => Self::Boolean,
            AuthoringValueKind::Integer => Self::Integer,
            AuthoringValueKind::String => Self::String,
            AuthoringValueKind::Array => Self::Array,
            AuthoringValueKind::Table => Self::Table,
        }
    }
}

/// Requiredness of one child field across structural variants.
#[wasm_bindgen(js_name = AuthoringFieldPresence)]
#[derive(Clone, Copy)]
pub enum WasmAuthoringFieldPresence {
    /// Every applicable variant requires the field.
    Required,
    /// No applicable variant requires the field.
    Optional,
    /// Requiredness depends on the selected structural variant.
    VariantDependent,
}

impl From<AuthoringFieldPresence> for WasmAuthoringFieldPresence {
    fn from(value: AuthoringFieldPresence) -> Self {
        match value {
            AuthoringFieldPresence::Required => Self::Required,
            AuthoringFieldPresence::Optional => Self::Optional,
            AuthoringFieldPresence::VariantDependent => Self::VariantDependent,
        }
    }
}

/// Closed TOML literal family supplied by generated grammar facts.
#[wasm_bindgen(js_name = AuthoringLiteralKind)]
#[derive(Clone, Copy)]
pub enum WasmAuthoringLiteralKind {
    /// UTF-8 string literal.
    String,
    /// Signed integer literal.
    Integer,
    /// Boolean literal.
    Boolean,
}

/// Lossless projection of one generated TOML literal.
#[wasm_bindgen(js_name = AuthoringLiteral)]
pub struct WasmAuthoringLiteral {
    inner: AuthoringLiteral,
}

#[wasm_bindgen(js_class = AuthoringLiteral)]
impl WasmAuthoringLiteral {
    /// Return the literal family.
    pub fn kind(&self) -> WasmAuthoringLiteralKind {
        match self.inner {
            AuthoringLiteral::String(_) => WasmAuthoringLiteralKind::String,
            AuthoringLiteral::Integer(_) => WasmAuthoringLiteralKind::Integer,
            AuthoringLiteral::Boolean(_) => WasmAuthoringLiteralKind::Boolean,
        }
    }

    /// Return the string member when this is a string literal.
    #[wasm_bindgen(getter = stringValue)]
    pub fn string_value(&self) -> Option<String> {
        match &self.inner {
            AuthoringLiteral::String(value) => Some(value.clone()),
            AuthoringLiteral::Integer(_) | AuthoringLiteral::Boolean(_) => None,
        }
    }

    /// Return the signed integer member as a non-lossy JavaScript `bigint`.
    #[wasm_bindgen(getter = integerValue)]
    pub fn integer_value(&self) -> Option<i64> {
        match self.inner {
            AuthoringLiteral::Integer(value) => Some(value),
            AuthoringLiteral::String(_) | AuthoringLiteral::Boolean(_) => None,
        }
    }

    /// Return the Boolean member when this is a Boolean literal.
    #[wasm_bindgen(getter = booleanValue)]
    pub fn boolean_value(&self) -> Option<bool> {
        match self.inner {
            AuthoringLiteral::Boolean(value) => Some(value),
            AuthoringLiteral::String(_) | AuthoringLiteral::Integer(_) => None,
        }
    }
}

/// One allowed literal plus producer-owned documentation.
#[wasm_bindgen(js_name = AuthoringLiteralFact)]
pub struct WasmAuthoringLiteralFact {
    inner: AuthoringLiteralFact,
}

impl WasmAuthoringLiteralFact {
    fn from_api(inner: &AuthoringLiteralFact) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[wasm_bindgen(js_class = AuthoringLiteralFact)]
impl WasmAuthoringLiteralFact {
    /// Return the exact literal admitted by the generated node.
    pub fn value(&self) -> WasmAuthoringLiteral {
        WasmAuthoringLiteral {
            inner: self.inner.value().clone(),
        }
    }

    /// Return variant-specific producer documentation.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description().map(str::to_owned)
    }
}

/// One child key offered at the selected structural path.
#[wasm_bindgen(js_name = AuthoringFieldFact)]
pub struct WasmAuthoringFieldFact {
    inner: AuthoringFieldFact,
}

impl WasmAuthoringFieldFact {
    fn from_api(inner: &AuthoringFieldFact) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[wasm_bindgen(js_class = AuthoringFieldFact)]
impl WasmAuthoringFieldFact {
    /// Return the exact authored key spelling.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name().to_owned()
    }

    /// Return producer documentation for this field.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description().map(str::to_owned)
    }

    /// Return requiredness across applicable structural variants.
    pub fn presence(&self) -> WasmAuthoringFieldPresence {
        self.inner.presence().into()
    }

    /// Return all admitted structural value categories in stable order.
    #[wasm_bindgen(js_name = valueKinds, unchecked_return_type = "AuthoringValueKind[]")]
    pub fn value_kinds(&self) -> Vec<WasmAuthoringValueKind> {
        self.inner
            .value_kinds()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    /// Return the complete bounded literal vocabulary.
    #[wasm_bindgen(js_name = allowedValues)]
    pub fn allowed_values(&self) -> Vec<WasmAuthoringLiteralFact> {
        self.inner
            .allowed_values()
            .iter()
            .map(WasmAuthoringLiteralFact::from_api)
            .collect()
    }
}

/// Bounded structural facts for one authoring position.
#[wasm_bindgen(js_name = AuthoringNodeFact)]
pub struct WasmAuthoringNodeFact {
    inner: AuthoringNodeFact,
}

#[wasm_bindgen(js_class = AuthoringNodeFact)]
impl WasmAuthoringNodeFact {
    /// Return producer documentation for this structural context.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.inner.description().map(str::to_owned)
    }

    /// Return every admitted structural value category in stable order.
    #[wasm_bindgen(js_name = valueKinds, unchecked_return_type = "AuthoringValueKind[]")]
    pub fn value_kinds(&self) -> Vec<WasmAuthoringValueKind> {
        self.inner
            .value_kinds()
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    /// Return the complete bounded literal vocabulary.
    #[wasm_bindgen(js_name = allowedValues)]
    pub fn allowed_values(&self) -> Vec<WasmAuthoringLiteralFact> {
        self.inner
            .allowed_values()
            .iter()
            .map(WasmAuthoringLiteralFact::from_api)
            .collect()
    }

    /// Return every deterministic child-field fact in stable order.
    pub fn fields(&self) -> Vec<WasmAuthoringFieldFact> {
        self.inner
            .fields()
            .iter()
            .map(WasmAuthoringFieldFact::from_api)
            .collect()
    }

    /// Return the optional generated string format.
    #[wasm_bindgen(getter)]
    pub fn format(&self) -> Option<String> {
        self.inner.format().map(str::to_owned)
    }

    /// Return the optional generated string pattern.
    #[wasm_bindgen(getter)]
    pub fn pattern(&self) -> Option<String> {
        self.inner.pattern().map(str::to_owned)
    }

    /// Return the inclusive integer lower bound as a non-lossy `bigint`.
    #[wasm_bindgen(getter)]
    pub fn minimum(&self) -> Option<i64> {
        self.inner.minimum()
    }

    /// Return the inclusive integer upper bound as a non-lossy `bigint`.
    #[wasm_bindgen(getter)]
    pub fn maximum(&self) -> Option<i64> {
        self.inner.maximum()
    }

    /// Return the inclusive string-length lower bound as a non-lossy `bigint`.
    #[wasm_bindgen(getter = minLength)]
    pub fn min_length(&self) -> Option<u64> {
        self.inner.min_length()
    }

    /// Return the inclusive string-length upper bound as a non-lossy `bigint`.
    #[wasm_bindgen(getter = maxLength)]
    pub fn max_length(&self) -> Option<u64> {
        self.inner.max_length()
    }

    /// Return the inclusive collection-size lower bound as a non-lossy `bigint`.
    #[wasm_bindgen(getter = minItems)]
    pub fn min_items(&self) -> Option<u64> {
        self.inner.min_items()
    }

    /// Return the inclusive collection-size upper bound as a non-lossy `bigint`.
    #[wasm_bindgen(getter = maxItems)]
    pub fn max_items(&self) -> Option<u64> {
        self.inner.max_items()
    }
}

/// Complete source-role and schema-derived response for one authoring request.
#[wasm_bindgen(js_name = AuthoringInspection)]
pub struct WasmAuthoringInspection {
    inner: AuthoringInspection,
}

impl WasmAuthoringInspection {
    fn from_api(inner: AuthoringInspection) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = AuthoringInspection)]
impl WasmAuthoringInspection {
    /// Return availability for the exact request.
    pub fn state(&self) -> WasmAuthoringInspectionState {
        self.inner.state().into()
    }

    /// Return the selected grammar descriptor identity.
    #[wasm_bindgen(getter = schemaId)]
    pub fn schema_id(&self) -> Option<String> {
        self.inner.schema_id().map(str::to_owned)
    }

    /// Return the selected named definition for a shared grammar.
    #[wasm_bindgen(getter)]
    pub fn definition(&self) -> Option<String> {
        self.inner.definition().map(str::to_owned)
    }

    /// Return structural facts only when the state is available.
    pub fn node(&self) -> Option<WasmAuthoringNodeFact> {
        self.inner
            .node()
            .cloned()
            .map(|inner| WasmAuthoringNodeFact { inner })
    }
}

/// Closed namespace of one resolved editor-visible identity.
#[wasm_bindgen(js_name = AuthoringIdentityKind)]
#[derive(Clone, Copy)]
pub enum WasmAuthoringIdentityKind {
    /// Compiler-admitted source document.
    SourceDocument,
    /// Principal-owned workspace root.
    Workspace,
    /// Canonical project.
    Project,
    /// Canonical execution workspace.
    ExecutionWorkspace,
    /// Canonical repository.
    Repository,
    /// Neutral execution-path selection.
    ExecutionPath,
    /// Reusable workspace module.
    Module,
    /// Immutable module release.
    ModuleRelease,
    /// Content-addressed component.
    Component,
    /// Named module export.
    Export,
    /// Module-scoped namespace.
    Namespace,
    /// Qualified declaration.
    Declaration,
    /// Module-scoped logical resource.
    Resource,
    /// Private run-policy declaration.
    RunPolicy,
    /// Private lifecycle behavior.
    LifecycleBehavior,
    /// Portable capability identity.
    Capability,
    /// Machine-local authority root.
    AuthorityRoot,
    /// Exact provider-binding projection.
    ProviderBinding,
    /// Private machine binding.
    Machine,
    /// Responsibility derivation.
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
    /// Qualified stable organizational role.
    OrganizationalRole,
    /// Explicit organizational occupant.
    OrganizationalOccupant,
    /// Explicit organizational provider.
    OrganizationalProvider,
    /// Provider-neutral communication actor.
    CommunicationActor,
    /// Topic-scoped leadership participation.
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
    /// Composition provider.
    CompositionProvider,
    /// Composition provider binding.
    CompositionProviderBinding,
    /// Composition entry contract.
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
    /// Package operation-archetype resource.
    PackageOperationArchetype,
    /// Package operation resource.
    PackageOperation,
    /// Package role-archetype resource.
    PackageRoleArchetype,
    /// Package specialist-role resource.
    PackageRole,
    /// Package stack resource.
    PackageStack,
    /// Package run-mode resource.
    PackageRunMode,
    /// Package policy-family resource.
    PackagePolicyFamily,
    /// Package governing-order resource.
    PackageGoverningOrder,
    /// Package capability-policy resource.
    PackageCapabilityPolicy,
    /// Package tool-recipe resource.
    PackageToolRecipe,
    /// Immutable Package identity.
    Package,
}

impl From<AuthoringIdentityKind> for WasmAuthoringIdentityKind {
    fn from(value: AuthoringIdentityKind) -> Self {
        match value {
            AuthoringIdentityKind::SourceDocument => Self::SourceDocument,
            AuthoringIdentityKind::Workspace => Self::Workspace,
            AuthoringIdentityKind::Project => Self::Project,
            AuthoringIdentityKind::ExecutionWorkspace => Self::ExecutionWorkspace,
            AuthoringIdentityKind::Repository => Self::Repository,
            AuthoringIdentityKind::ExecutionPath => Self::ExecutionPath,
            AuthoringIdentityKind::Module => Self::Module,
            AuthoringIdentityKind::ModuleRelease => Self::ModuleRelease,
            AuthoringIdentityKind::Component => Self::Component,
            AuthoringIdentityKind::Export => Self::Export,
            AuthoringIdentityKind::Namespace => Self::Namespace,
            AuthoringIdentityKind::Declaration => Self::Declaration,
            AuthoringIdentityKind::Resource => Self::Resource,
            AuthoringIdentityKind::RunPolicy => Self::RunPolicy,
            AuthoringIdentityKind::LifecycleBehavior => Self::LifecycleBehavior,
            AuthoringIdentityKind::Capability => Self::Capability,
            AuthoringIdentityKind::AuthorityRoot => Self::AuthorityRoot,
            AuthoringIdentityKind::ProviderBinding => Self::ProviderBinding,
            AuthoringIdentityKind::Machine => Self::Machine,
            AuthoringIdentityKind::ResponsibilityDerivation => Self::ResponsibilityDerivation,
            AuthoringIdentityKind::ResponsibilityProjection => Self::ResponsibilityProjection,
            AuthoringIdentityKind::ResponsibilityGenealogyLink => Self::ResponsibilityGenealogyLink,
            AuthoringIdentityKind::LogicalScope => Self::LogicalScope,
            AuthoringIdentityKind::LogicalOrganization => Self::LogicalOrganization,
            AuthoringIdentityKind::OrganizationalGroup => Self::OrganizationalGroup,
            AuthoringIdentityKind::DisciplineTree => Self::DisciplineTree,
            AuthoringIdentityKind::OrganizationalRole => Self::OrganizationalRole,
            AuthoringIdentityKind::OrganizationalOccupant => Self::OrganizationalOccupant,
            AuthoringIdentityKind::OrganizationalProvider => Self::OrganizationalProvider,
            AuthoringIdentityKind::CommunicationActor => Self::CommunicationActor,
            AuthoringIdentityKind::LeadershipTopic => Self::LeadershipTopic,
            AuthoringIdentityKind::OrganizationalRelation => Self::OrganizationalRelation,
            AuthoringIdentityKind::OrganizationalObject => Self::OrganizationalObject,
            AuthoringIdentityKind::GroupLensProfile => Self::GroupLensProfile,
            AuthoringIdentityKind::DirectionalService => Self::DirectionalService,
            AuthoringIdentityKind::OrganizationalProjection => Self::OrganizationalProjection,
            AuthoringIdentityKind::OrganizationalInspection => Self::OrganizationalInspection,
            AuthoringIdentityKind::CompositionParticipant => Self::CompositionParticipant,
            AuthoringIdentityKind::CompositionResponsibilityContract => {
                Self::CompositionResponsibilityContract
            }
            AuthoringIdentityKind::CompositionParticipantManifest => {
                Self::CompositionParticipantManifest
            }
            AuthoringIdentityKind::CompositionProvider => Self::CompositionProvider,
            AuthoringIdentityKind::CompositionProviderBinding => Self::CompositionProviderBinding,
            AuthoringIdentityKind::CompositionEntryContract => Self::CompositionEntryContract,
            AuthoringIdentityKind::CompositionRequestTranslation => {
                Self::CompositionRequestTranslation
            }
            AuthoringIdentityKind::CompositionResultTranslation => {
                Self::CompositionResultTranslation
            }
            AuthoringIdentityKind::CompositionTerminationContract => {
                Self::CompositionTerminationContract
            }
            AuthoringIdentityKind::CompositionSettlementContract => {
                Self::CompositionSettlementContract
            }
            AuthoringIdentityKind::CompositionContinuityContract => {
                Self::CompositionContinuityContract
            }
            AuthoringIdentityKind::CompositionEdge => Self::CompositionEdge,
            AuthoringIdentityKind::ExperiencePath => Self::ExperiencePath,
            AuthoringIdentityKind::PackageDoctrine => Self::PackageDoctrine,
            AuthoringIdentityKind::PackageOperationArchetype => Self::PackageOperationArchetype,
            AuthoringIdentityKind::PackageOperation => Self::PackageOperation,
            AuthoringIdentityKind::PackageRoleArchetype => Self::PackageRoleArchetype,
            AuthoringIdentityKind::PackageRole => Self::PackageRole,
            AuthoringIdentityKind::PackageStack => Self::PackageStack,
            AuthoringIdentityKind::PackageRunMode => Self::PackageRunMode,
            AuthoringIdentityKind::PackagePolicyFamily => Self::PackagePolicyFamily,
            AuthoringIdentityKind::PackageGoverningOrder => Self::PackageGoverningOrder,
            AuthoringIdentityKind::PackageCapabilityPolicy => Self::PackageCapabilityPolicy,
            AuthoringIdentityKind::PackageToolRecipe => Self::PackageToolRecipe,
            AuthoringIdentityKind::Package => Self::Package,
        }
    }
}

/// Resolved binding-safe semantic identity.
#[wasm_bindgen(js_name = AuthoringIdentity)]
pub struct WasmAuthoringIdentity {
    inner: AuthoringIdentity,
}

impl WasmAuthoringIdentity {
    fn from_api(inner: &AuthoringIdentity) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[wasm_bindgen(js_class = AuthoringIdentity)]
impl WasmAuthoringIdentity {
    /// Return the non-interchangeable identity namespace.
    pub fn kind(&self) -> WasmAuthoringIdentityKind {
        self.inner.kind().into()
    }

    /// Return the canonical compiler-owned display form.
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> String {
        self.inner.value().to_owned()
    }

    /// Return the exact scalar spelling valid in the reference scope.
    #[wasm_bindgen(getter = authoredValue)]
    pub fn authored_value(&self) -> String {
        self.inner.authored_value().to_owned()
    }
}

/// Semantic role played by one exact admitted occurrence.
#[wasm_bindgen(js_name = AuthoringOccurrenceRole)]
#[derive(Clone, Copy)]
pub enum WasmAuthoringOccurrenceRole {
    /// The occurrence establishes the canonical definition.
    Definition,
    /// The occurrence resolves to a separate definition.
    Reference,
    /// The occurrence carries a content-bearing projection.
    Projection,
}

impl From<AuthoringOccurrenceRole> for WasmAuthoringOccurrenceRole {
    fn from(value: AuthoringOccurrenceRole) -> Self {
        match value {
            AuthoringOccurrenceRole::Definition => Self::Definition,
            AuthoringOccurrenceRole::Reference => Self::Reference,
            AuthoringOccurrenceRole::Projection => Self::Projection,
        }
    }
}

/// Exact source label and path supplied by the compiler, including on rejection.
#[wasm_bindgen(js_name = AuthoringSourceLocation)]
pub struct WasmAuthoringSourceLocation {
    inner: AuthoringSourceLocation,
}

impl WasmAuthoringSourceLocation {
    fn from_api(inner: &AuthoringSourceLocation) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[wasm_bindgen(js_class = AuthoringSourceLocation)]
impl WasmAuthoringSourceLocation {
    /// Return the compiler logical source label.
    #[wasm_bindgen(getter = sourceLabel)]
    pub fn source_label(&self) -> String {
        self.inner.source_label().to_owned()
    }

    /// Return the complete exact instance path in stable order.
    pub fn path(&self) -> Vec<WasmAuthoringInstancePathSegment> {
        self.inner
            .path()
            .iter()
            .map(WasmAuthoringInstancePathSegment::from_api)
            .collect()
    }
}

/// Kind of source correction proved by the compiler.
#[wasm_bindgen(js_name = AuthoringCorrectionKind)]
#[derive(Clone, Copy)]
pub enum WasmAuthoringCorrectionKind {
    /// Narrow an import visibility ceiling to its export's ceiling.
    NarrowImportVisibility,
}

impl From<AuthoringCorrectionKind> for WasmAuthoringCorrectionKind {
    fn from(value: AuthoringCorrectionKind) -> Self {
        match value {
            AuthoringCorrectionKind::NarrowImportVisibility => Self::NarrowImportVisibility,
        }
    }
}

/// Proposed source edit from a rejected configuration check.
///
/// Match the expected source at the supplied path before using the replacement.
/// This value preserves the compiler's proposal; it does not authorize a write.
#[wasm_bindgen(js_name = AuthoringCorrection)]
pub struct WasmAuthoringCorrection {
    inner: AuthoringCorrection,
}

impl WasmAuthoringCorrection {
    fn from_api(inner: &AuthoringCorrection) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[wasm_bindgen(js_class = AuthoringCorrection)]
impl WasmAuthoringCorrection {
    /// Return the compiler-proved correction kind.
    pub fn kind(&self) -> WasmAuthoringCorrectionKind {
        self.inner.kind().into()
    }

    /// Return the logical label of the source to edit.
    #[wasm_bindgen(getter = sourceLabel)]
    pub fn source_label(&self) -> String {
        self.inner.source_label().to_owned()
    }

    /// Return the ordered field and array-index steps to the source value.
    pub fn path(&self) -> Vec<WasmAuthoringInstancePathSegment> {
        self.inner
            .path()
            .iter()
            .map(WasmAuthoringInstancePathSegment::from_api)
            .collect()
    }

    /// Return the exact scalar text that must still occupy the edit range.
    #[wasm_bindgen(getter = expectedSource)]
    pub fn expected_source(&self) -> String {
        self.inner.expected_source().to_owned()
    }

    /// Return the replacement scalar text selected by the compiler.
    #[wasm_bindgen(getter = replacementSource)]
    pub fn replacement_source(&self) -> String {
        self.inner.replacement_source().to_owned()
    }

    /// Return the semantic subject that proves this correction.
    #[wasm_bindgen(js_name = proofSubject)]
    pub fn proof_subject(&self) -> WasmAuthoringIdentity {
        WasmAuthoringIdentity::from_api(self.inner.proof_subject())
    }

    /// Return the identity of the rejected diagnostic, without recomputing it.
    #[wasm_bindgen(getter = diagnosticIdentity)]
    pub fn diagnostic_identity(&self) -> String {
        self.inner.diagnostic_identity().as_str().to_owned()
    }
}

/// Semantic navigation result for one exact admitted occurrence.
#[wasm_bindgen(js_name = AuthoringNavigation)]
pub struct WasmAuthoringNavigation {
    inner: AuthoringNavigation,
}

impl WasmAuthoringNavigation {
    fn from_api(inner: AuthoringNavigation) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = AuthoringNavigation)]
impl WasmAuthoringNavigation {
    /// Return the resolved identity, when the coordinate is semantic.
    pub fn identity(&self) -> Option<WasmAuthoringIdentity> {
        self.inner.identity().map(WasmAuthoringIdentity::from_api)
    }

    /// Return the semantic role of the queried occurrence.
    #[wasm_bindgen(js_name = occurrenceRole)]
    pub fn occurrence_role(&self) -> Option<WasmAuthoringOccurrenceRole> {
        self.inner.occurrence_role().map(Into::into)
    }

    /// Return the canonical definition when the closure contains one.
    pub fn definition(&self) -> Option<WasmAuthoringSourceLocation> {
        self.inner
            .definition()
            .map(WasmAuthoringSourceLocation::from_api)
    }

    /// Return all retained references in deterministic order.
    pub fn references(&self) -> Vec<WasmAuthoringSourceLocation> {
        self.inner
            .references()
            .iter()
            .map(WasmAuthoringSourceLocation::from_api)
            .collect()
    }

    /// Return all bounded same-namespace completion alternatives.
    pub fn alternatives(&self) -> Vec<WasmAuthoringIdentity> {
        self.inner
            .alternatives()
            .iter()
            .map(WasmAuthoringIdentity::from_api)
            .collect()
    }

    /// Report whether the API ceiling omitted source references.
    #[wasm_bindgen(getter = referencesTruncated)]
    pub fn references_truncated(&self) -> bool {
        self.inner.references_truncated()
    }

    /// Report whether the API ceiling omitted completion alternatives.
    #[wasm_bindgen(getter = alternativesTruncated)]
    pub fn alternatives_truncated(&self) -> bool {
        self.inner.alternatives_truncated()
    }
}

#[wasm_bindgen(js_class = Pale)]
impl PaleWasm {
    /// Return API-owned ingress ceilings for one authoring custody class.
    #[wasm_bindgen(js_name = authoringIngressLimits)]
    pub fn authoring_ingress_limits(
        &self,
        class: WasmAuthoringIngressClass,
    ) -> WasmAuthoringIngressLimits {
        WasmAuthoringIngressLimits {
            inner: PaleApi::authoring_ingress_limits(class.into()),
        }
    }

    /// Validate one explicitly typed source without selecting project context.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with the API-owned canonical diagnostic when
    /// label, resource, syntax, or contract admission fails.
    #[wasm_bindgen(js_name = validateAuthoringDocument)]
    pub fn validate_authoring_document(
        &self,
        role: WasmStandaloneAuthoringRole,
        source_label: String,
        bytes: Box<[u8]>,
    ) -> Result<(), JsValue> {
        self.api
            .validate_authoring_document(role.into(), source_label, bytes.into_vec())
            .map_err(diagnostic_error)
    }

    /// Inspect generated facts for one explicitly selected standalone grammar.
    /// Path elements are consumed in order; failure releases the converted
    /// prefix and leaves later elements untouched.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` carrying the API-owned canonical diagnostic when
    /// the path exceeds its bound or generated authoring metadata is incoherent.
    /// An invalid or already consumed element returns a native `Error` without
    /// an API diagnostic.
    #[wasm_bindgen(js_name = inspectStandaloneAuthoring)]
    pub fn inspect_standalone_authoring(
        &self,
        role: WasmStandaloneAuthoringRole,
        #[wasm_bindgen(unchecked_param_type = "AuthoringPathSegment[]")] path: Vec<JsValue>,
    ) -> Result<WasmAuthoringInspection, JsValue> {
        let path = consume_authoring_path(path)?;
        self.api
            .inspect_standalone_authoring(role.into(), path)
            .map(WasmAuthoringInspection::from_api)
            .map_err(diagnostic_error)
    }
}

#[wasm_bindgen(js_class = PortableWorkspaceCompilation)]
impl WasmPortableWorkspaceCompilation {
    /// Inspect generated grammar retained by this admitted portable workspace.
    /// Path elements are consumed in order; failure releases the converted
    /// prefix and leaves later elements untouched.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` carrying the API-owned canonical diagnostic when
    /// the path exceeds its bound or generated authoring metadata is incoherent.
    /// An invalid or already consumed element returns a native `Error` without
    /// an API diagnostic.
    #[wasm_bindgen(js_name = inspectAuthoring)]
    pub fn inspect_authoring(
        &self,
        source_label: String,
        #[wasm_bindgen(unchecked_param_type = "AuthoringPathSegment[]")] path: Vec<JsValue>,
    ) -> Result<WasmAuthoringInspection, JsValue> {
        let path = consume_authoring_path(path)?;
        self.inner
            .inspect_authoring(AuthoringInspectionRequest::new(source_label, path))
            .map(WasmAuthoringInspection::from_api)
            .map_err(diagnostic_error)
    }

    /// Resolve one exact occurrence through this workspace's canonical linker.
    ///
    /// Path elements are consumed in order. On failure, earlier elements remain
    /// consumed and are released; later elements are untouched.
    ///
    /// # Errors
    /// Returns a native `Error` for an invalid or already consumed path element.
    #[wasm_bindgen(js_name = navigateAuthoring)]
    pub fn navigate_authoring(
        &self,
        source_label: String,
        #[wasm_bindgen(unchecked_param_type = "AuthoringInstancePathSegment[]")] path: Vec<JsValue>,
    ) -> Result<WasmAuthoringNavigation, JsValue> {
        let path = path
            .into_iter()
            .map(|value| {
                consume_exported_argument::<WasmAuthoringInstancePathSegment>(
                    value,
                    "path must contain unconsumed authoring instance path segments",
                )
                .map(|segment| segment.inner)
            })
            .collect::<Result<_, _>>()?;
        Ok(WasmAuthoringNavigation::from_api(
            self.inner
                .navigate_authoring(AuthoringNavigationRequest::new(source_label, path)),
        ))
    }
}

/// Consume grammar path elements in order; release a converted prefix on error.
fn consume_authoring_path(path: Vec<JsValue>) -> Result<Vec<AuthoringPathSegment>, JsValue> {
    path.into_iter()
        .map(|value| {
            consume_exported_argument::<WasmAuthoringPathSegment>(
                value,
                "path must contain unconsumed authoring path segments",
            )
            .map(|segment| segment.inner)
        })
        .collect()
}
