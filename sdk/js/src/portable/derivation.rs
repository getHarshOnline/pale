// Node derivation-proof projection fragment included by the shallow façade.

/// Opaque typed subject returned by comparison or source inspection.
#[napi(js_name = "WorkspaceSubject")]
pub struct WorkspaceSubject {
    pub(crate) inner: PortableWorkspaceSubject,
}

impl WorkspaceSubject {
    /// Clone one API-owned compound subject.
    pub(crate) fn from_api(inner: &PortableWorkspaceSubject) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[napi]
impl WorkspaceSubject {
    /// Stable coordinate-family name.
    #[napi(getter)]
    pub fn kind(&self) -> WorkspaceSubjectKind {
        match self.inner.kind() {
            PortableWorkspaceSubjectKind::Source => WorkspaceSubjectKind::Source,
            PortableWorkspaceSubjectKind::Component => WorkspaceSubjectKind::Component,
            PortableWorkspaceSubjectKind::Module => WorkspaceSubjectKind::Module,
            PortableWorkspaceSubjectKind::Declaration => WorkspaceSubjectKind::Declaration,
            PortableWorkspaceSubjectKind::Import => WorkspaceSubjectKind::Import,
            PortableWorkspaceSubjectKind::Extension => WorkspaceSubjectKind::Extension,
            PortableWorkspaceSubjectKind::Policy => WorkspaceSubjectKind::Policy,
            PortableWorkspaceSubjectKind::ResponsibilityClosure => {
                WorkspaceSubjectKind::ResponsibilityClosure
            }
            PortableWorkspaceSubjectKind::OrganizationalTranslationClosure => {
                WorkspaceSubjectKind::OrganizationalTranslationClosure
            }
            PortableWorkspaceSubjectKind::ExperienceComposition => {
                WorkspaceSubjectKind::ExperienceComposition
            }
            PortableWorkspaceSubjectKind::Resource => WorkspaceSubjectKind::Resource,
            PortableWorkspaceSubjectKind::ResolutionLock => WorkspaceSubjectKind::ResolutionLock,
            PortableWorkspaceSubjectKind::UniverseLock => WorkspaceSubjectKind::UniverseLock,
            PortableWorkspaceSubjectKind::Workspace => WorkspaceSubjectKind::Workspace,
        }
    }

    /// Workspace identity when this is a workspace subject.
    #[napi(getter)]
    pub fn workspace_id(&self) -> Option<String> {
        self.inner.workspace_id().map(str::to_owned)
    }

    /// Source identity when this is a source subject.
    #[napi(getter)]
    pub fn source_id(&self) -> Option<String> {
        self.inner.source_id().map(str::to_owned)
    }

    /// Component identity when this is a component subject.
    #[napi(getter)]
    pub fn component_id(&self) -> Option<String> {
        self.inner.component_id().map(str::to_owned)
    }

    /// Module identity when this is a module subject.
    #[napi(getter)]
    pub fn module_id(&self) -> Option<String> {
        self.inner.module_id().map(str::to_owned)
    }

    /// Owning module identity, when carried.
    #[napi(getter)]
    pub fn owner_module_id(&self) -> Option<String> {
        self.inner.owner_module_id().map(str::to_owned)
    }
    /// Declaration namespace identity, when carried.
    #[napi(getter)]
    pub fn declaration_namespace_id(&self) -> Option<String> {
        self.inner.declaration_namespace_id().map(str::to_owned)
    }
    /// Declaration identity, when carried.
    #[napi(getter)]
    pub fn declaration_id(&self) -> Option<String> {
        self.inner.declaration_id().map(str::to_owned)
    }
    /// Importer module identity, when carried.
    #[napi(getter)]
    pub fn importer_module_id(&self) -> Option<String> {
        self.inner.importer_module_id().map(str::to_owned)
    }
    /// Upstream module identity, when carried.
    #[napi(getter)]
    pub fn upstream_module_id(&self) -> Option<String> {
        self.inner.upstream_module_id().map(str::to_owned)
    }
    /// Upstream release identity, when carried.
    #[napi(getter)]
    pub fn upstream_release_id(&self) -> Option<String> {
        self.inner.upstream_release_id().map(str::to_owned)
    }
    /// Export identity, when carried.
    #[napi(getter)]
    pub fn export_id(&self) -> Option<String> {
        self.inner.export_id().map(str::to_owned)
    }
    /// Source namespace identity, when carried.
    #[napi(getter)]
    pub fn source_namespace_id(&self) -> Option<String> {
        self.inner.source_namespace_id().map(str::to_owned)
    }
    /// Local namespace identity, when carried.
    #[napi(getter)]
    pub fn local_namespace_id(&self) -> Option<String> {
        self.inner.local_namespace_id().map(str::to_owned)
    }
    /// Extension namespace identity, when carried.
    #[napi(getter)]
    pub fn extension_namespace_id(&self) -> Option<String> {
        self.inner.extension_namespace_id().map(str::to_owned)
    }
    /// Resource identity, when carried.
    #[napi(getter)]
    pub fn resource_id(&self) -> Option<String> {
        self.inner.resource_id().map(str::to_owned)
    }
}

/// Closed workspace-subject family.
#[napi(js_name = "WorkspaceSubjectKind")]
pub enum WorkspaceSubjectKind {
    /// Logical source.
    Source,
    /// Semantic component.
    Component,
    /// Selected module.
    Module,
    /// Declaration.
    Declaration,
    /// Import path.
    Import,
    /// Extension rule.
    Extension,
    /// Effective policy.
    Policy,
    /// Responsibility closure.
    ResponsibilityClosure,
    /// Organizational-translation closure.
    OrganizationalTranslationClosure,
    /// Experience composition.
    ExperienceComposition,
    /// Selected resource.
    Resource,
    /// Resolution lock.
    ResolutionLock,
    /// Universe lock.
    UniverseLock,
    /// Whole workspace.
    Workspace,
}

/// Closed provenance family.
#[napi(js_name = "WorkspaceProvenanceKind")]
pub enum WorkspaceProvenanceKind {
    /// Direct source.
    Source,
    /// Selected module release.
    ModuleRelease,
    /// Resolved import.
    Import,
    /// Workspace extension.
    Extension,
    /// Compiler aggregate.
    Compilation,
}

/// Lossless provenance snapshot; fields not belonging to the selected closed kind are absent.
#[napi(object, object_from_js = false, js_name = "WorkspaceProvenance")]
pub struct WorkspaceProvenance {
    /// Closed provenance family.
    pub kind: WorkspaceProvenanceKind,
    /// Direct source identity.
    pub source_id: Option<String>,
    /// Module or extension owner identity.
    pub module_id: Option<String>,
    /// Selected release identity.
    pub release_id: Option<String>,
    /// Selected module digest.
    pub module_digest: Option<String>,
    /// Importing module identity.
    pub importer_module_id: Option<String>,
    /// Upstream module identity.
    pub upstream_module_id: Option<String>,
    /// Upstream release identity.
    pub upstream_release_id: Option<String>,
    /// Selected export identity.
    pub export_id: Option<String>,
    /// Upstream namespace identity.
    pub source_namespace_id: Option<String>,
    /// Importer-local namespace identity.
    pub local_namespace_id: Option<String>,
    /// Extension-owning namespace identity.
    pub extension_namespace_id: Option<String>,
    /// Extension target namespace identity.
    pub target_namespace_id: Option<String>,
    /// Extension target declaration identity.
    pub target_declaration_id: Option<String>,
}

impl WorkspaceProvenance {
    fn from_api(value: &PortableWorkspaceProvenance) -> Self {
        let mut out = Self {
            kind: WorkspaceProvenanceKind::Compilation,
            source_id: None,
            module_id: None,
            release_id: None,
            module_digest: None,
            importer_module_id: None,
            upstream_module_id: None,
            upstream_release_id: None,
            export_id: None,
            source_namespace_id: None,
            local_namespace_id: None,
            extension_namespace_id: None,
            target_namespace_id: None,
            target_declaration_id: None,
        };
        match value {
            PortableWorkspaceProvenance::Source { source_id } => {
                out.kind = WorkspaceProvenanceKind::Source;
                out.source_id = Some(source_id.clone());
            }
            PortableWorkspaceProvenance::ModuleRelease {
                module_id,
                release_id,
                module_digest,
            } => {
                out.kind = WorkspaceProvenanceKind::ModuleRelease;
                out.module_id = Some(module_id.clone());
                out.release_id = Some(release_id.clone());
                out.module_digest = Some(module_digest.clone());
            }
            PortableWorkspaceProvenance::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => {
                out.kind = WorkspaceProvenanceKind::Import;
                out.importer_module_id = Some(importer_module_id.clone());
                out.upstream_module_id = Some(upstream_module_id.clone());
                out.upstream_release_id = Some(upstream_release_id.clone());
                out.export_id = Some(export_id.clone());
                out.source_namespace_id = Some(source_namespace_id.clone());
                out.local_namespace_id = Some(local_namespace_id.clone());
            }
            PortableWorkspaceProvenance::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => {
                out.kind = WorkspaceProvenanceKind::Extension;
                out.module_id = Some(owner_module_id.clone());
                out.extension_namespace_id = Some(extension_namespace_id.clone());
                out.target_namespace_id = Some(target_namespace_id.clone());
                out.target_declaration_id = Some(target_declaration_id.clone());
            }
            PortableWorkspaceProvenance::Compilation => {}
        }
        out
    }
}

/// Closed derivation-fact family.
#[napi(js_name = "WorkspaceFactKind")]
pub enum WorkspaceFactKind {
    /// Admitted source content.
    SourceContent,
    /// Semantic component.
    Component,
    /// Selected module.
    Module,
    /// Resolved declaration.
    Declaration,
    /// Resolved import surface.
    ImportSurface,
    /// Applied extension.
    Extension,
    /// Effective policy.
    EffectivePolicy,
    /// Responsibility closure.
    ResponsibilityClosure,
    /// Organizational-translation closure.
    OrganizationalTranslationClosure,
    /// Experience composition.
    ExperienceComposition,
    /// Selected resource.
    Resource,
    /// Resolution lock.
    ResolutionLock,
    /// Universe lock.
    UniverseLock,
    /// Terminal semantic IR.
    SemanticIr,
}

/// Lossless fact snapshot; fields not belonging to the selected closed kind are absent.
#[napi(object, object_from_js = false, js_name = "WorkspaceFact")]
pub struct WorkspaceFact {
    /// Closed fact family.
    pub kind: WorkspaceFactKind,
    /// Logical source path.
    pub source_path: Option<String>,
    /// Source-content digest.
    pub source_digest: Option<String>,
    /// Exact source byte length.
    pub byte_len: Option<u32>,
    /// Component digest.
    pub component_digest: Option<String>,
    /// Module digest.
    pub module_digest: Option<String>,
    /// Domain-separated value digest.
    pub value_digest: Option<String>,
    /// Import-surface digest.
    pub surface_digest: Option<String>,
    /// Closure digest.
    pub closure_digest: Option<String>,
    /// Resolution-lock digest.
    pub resolution_digest: Option<String>,
    /// Universe-lock digest.
    pub universe_digest: Option<String>,
    /// Workspace semantic-IR digest.
    pub workspace_semantic_ir_digest: Option<String>,
}

impl WorkspaceFact {
    fn from_api(value: &PortableWorkspaceFact) -> Self {
        let mut out = Self {
            kind: WorkspaceFactKind::SourceContent,
            source_path: None,
            source_digest: None,
            byte_len: None,
            component_digest: None,
            module_digest: None,
            value_digest: None,
            surface_digest: None,
            closure_digest: None,
            resolution_digest: None,
            universe_digest: None,
            workspace_semantic_ir_digest: None,
        };
        match value {
            PortableWorkspaceFact::SourceContent {
                source_path,
                source_digest,
                byte_len,
            } => {
                out.source_path = Some(source_path.clone());
                out.source_digest = Some(source_digest.clone());
                out.byte_len = Some(*byte_len);
            }
            PortableWorkspaceFact::Component { component_digest } => {
                out.kind = WorkspaceFactKind::Component;
                out.component_digest = Some(component_digest.clone());
            }
            PortableWorkspaceFact::Module { module_digest } => {
                out.kind = WorkspaceFactKind::Module;
                out.module_digest = Some(module_digest.clone());
            }
            PortableWorkspaceFact::Declaration { value_digest } => {
                out.kind = WorkspaceFactKind::Declaration;
                out.value_digest = Some(value_digest.clone());
            }
            PortableWorkspaceFact::ImportSurface { surface_digest } => {
                out.kind = WorkspaceFactKind::ImportSurface;
                out.surface_digest = Some(surface_digest.clone());
            }
            PortableWorkspaceFact::Extension { value_digest } => {
                out.kind = WorkspaceFactKind::Extension;
                out.value_digest = Some(value_digest.clone());
            }
            PortableWorkspaceFact::EffectivePolicy { value_digest } => {
                out.kind = WorkspaceFactKind::EffectivePolicy;
                out.value_digest = Some(value_digest.clone());
            }
            PortableWorkspaceFact::ResponsibilityClosure { closure_digest } => {
                out.kind = WorkspaceFactKind::ResponsibilityClosure;
                out.closure_digest = Some(closure_digest.clone());
            }
            PortableWorkspaceFact::OrganizationalTranslationClosure { closure_digest } => {
                out.kind = WorkspaceFactKind::OrganizationalTranslationClosure;
                out.closure_digest = Some(closure_digest.clone());
            }
            PortableWorkspaceFact::ExperienceComposition { source_digest } => {
                out.kind = WorkspaceFactKind::ExperienceComposition;
                out.source_digest = Some(source_digest.clone());
            }
            PortableWorkspaceFact::Resource { value_digest } => {
                out.kind = WorkspaceFactKind::Resource;
                out.value_digest = Some(value_digest.clone());
            }
            PortableWorkspaceFact::ResolutionLock { resolution_digest } => {
                out.kind = WorkspaceFactKind::ResolutionLock;
                out.resolution_digest = Some(resolution_digest.clone());
            }
            PortableWorkspaceFact::UniverseLock { universe_digest } => {
                out.kind = WorkspaceFactKind::UniverseLock;
                out.universe_digest = Some(universe_digest.clone());
            }
            PortableWorkspaceFact::SemanticIr {
                workspace_semantic_ir_digest,
            } => {
                out.kind = WorkspaceFactKind::SemanticIr;
                out.workspace_semantic_ir_digest = Some(workspace_semantic_ir_digest.clone());
            }
        }
        out
    }
}

/// Equality of two shared compiler outputs, independent of dependency impact.
#[napi]
pub enum WorkspaceValueComparison {
    /// Both typed outputs are equal.
    Equal,
    /// The typed outputs differ.
    Changed,
}

/// Subject membership and typed values in the retained comparison pair.
///
/// Missing sides have no value. Only a shared subject carries an equality
/// decision; an affected subject can still have equal outputs.
#[expect(
    clippy::large_enum_variant,
    reason = "This output-only value goes directly to napi conversion; boxing adds an allocation without reducing retained state."
)]
#[napi(
    discriminant = "membership",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum WorkspaceSubjectValues {
    /// Neither compilation contains this subject.
    Absent,
    /// Only the predecessor contains this subject.
    PriorOnly {
        /// Predecessor's complete typed output.
        prior: WorkspaceFact,
    },
    /// Only the current compilation contains this subject.
    CurrentOnly {
        /// Current compilation's complete typed output.
        current: WorkspaceFact,
    },
    /// Both compilations contain this full subject identity.
    Shared {
        /// Predecessor's complete typed output.
        prior: WorkspaceFact,
        /// Current compilation's complete typed output.
        current: WorkspaceFact,
        /// Equality established by the application.
        comparison: WorkspaceValueComparison,
    },
}

impl WorkspaceSubjectValues {
    fn from_api(value: &pale_sdk::PortableWorkspaceSubjectValues) -> Self {
        use pale_sdk::{
            PortableWorkspaceSubjectValues as Values, PortableWorkspaceValueComparison,
        };

        match value {
            Values::Absent => Self::Absent,
            Values::PriorOnly { prior } => Self::PriorOnly {
                prior: WorkspaceFact::from_api(prior),
            },
            Values::CurrentOnly { current } => Self::CurrentOnly {
                current: WorkspaceFact::from_api(current),
            },
            Values::Shared {
                prior,
                current,
                comparison,
            } => Self::Shared {
                prior: WorkspaceFact::from_api(prior),
                current: WorkspaceFact::from_api(current),
                comparison: match comparison {
                    PortableWorkspaceValueComparison::Equal => WorkspaceValueComparison::Equal,
                    PortableWorkspaceValueComparison::Changed => WorkspaceValueComparison::Changed,
                },
            },
        }
    }
}

/// Owned comparison of one subject, with the SDK's exact canonical bytes.
///
/// The receipt identifies the parent comparison. Its existing summary supplies
/// the prior/current universe and semantic identities. This answer is not a
/// new receipt, a compilation handle or evidence of native state.
#[napi]
pub struct WorkspaceSubjectComparison {
    inner: pale_sdk::PortableWorkspaceSubjectComparison,
}

#[napi]
impl WorkspaceSubjectComparison {
    /// Identity of the parent comparison that produced this answer.
    #[napi(getter)]
    pub fn receipt_digest(&self) -> String {
        self.inner.receipt_digest().to_owned()
    }

    /// Copy the full subject, including compound identity fields.
    #[napi]
    pub fn subject(&self) -> WorkspaceSubject {
        WorkspaceSubject::from_api(self.inner.subject())
    }

    /// Copy membership and values without deriving equality in JavaScript.
    #[napi(getter)]
    pub fn values(&self) -> WorkspaceSubjectValues {
        WorkspaceSubjectValues::from_api(self.inner.values())
    }

    /// Copy the API-owned projection without parsing or re-encoding it.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}

/// One exact source-change root.
#[napi(js_name = "WorkspaceChangeRoot")]
pub struct WorkspaceChangeRoot {
    inner: pale_sdk::PortableWorkspaceChangeRoot,
}

#[napi]
impl WorkspaceChangeRoot {
    /// Exact root content address.
    #[napi(getter)]
    pub fn root_digest(&self) -> String {
        self.inner.root_digest().to_owned()
    }

    /// Closed transition name.
    #[napi(getter)]
    pub fn kind(&self) -> WorkspaceChangeKind {
        match self.inner.kind() {
            pale_sdk::PortableWorkspaceChangeKind::Added => WorkspaceChangeKind::Added,
            pale_sdk::PortableWorkspaceChangeKind::Modified => WorkspaceChangeKind::Modified,
            pale_sdk::PortableWorkspaceChangeKind::Removed => WorkspaceChangeKind::Removed,
        }
    }

    /// Clone the typed causal subject.
    #[napi]
    pub fn subject(&self) -> WorkspaceSubject {
        WorkspaceSubject::from_api(self.inner.subject())
    }

    /// Exact predecessor fact, absent only for an added root.
    #[napi]
    pub fn prior_fact(&self) -> Option<WorkspaceFact> {
        self.inner.prior_fact().map(WorkspaceFact::from_api)
    }
    /// Exact current fact, absent only for a removed root.
    #[napi]
    pub fn current_fact(&self) -> Option<WorkspaceFact> {
        self.inner.current_fact().map(WorkspaceFact::from_api)
    }
}

/// Closed source transition.
#[napi(js_name = "WorkspaceChangeKind")]
pub enum WorkspaceChangeKind {
    /// No predecessor fact exists.
    Added,
    /// Predecessor and current facts differ.
    Modified,
    /// No current fact exists.
    Removed,
}

/// One bounded page of change roots.
#[napi(js_name = "WorkspaceChangeRootPage")]
pub struct WorkspaceChangeRootPage {
    inner: pale_sdk::PortableWorkspaceChangeRootPage,
}

impl WorkspaceChangeRootPage {
    /// Retain the exact SDK page and its preconstructed canonical bytes.
    pub(crate) const fn from_api(inner: pale_sdk::PortableWorkspaceChangeRootPage) -> Self {
        Self { inner }
    }
}

#[napi]
impl WorkspaceChangeRootPage {
    /// Copy canonically ordered root entries.
    #[napi(getter)]
    pub fn entries(&self) -> Vec<WorkspaceChangeRoot> {
        self.inner
            .entries()
            .iter()
            .cloned()
            .map(|inner| WorkspaceChangeRoot { inner })
            .collect()
    }

    /// Exact next offset, or no value at the terminal page.
    #[napi(getter)]
    pub fn next_offset(&self) -> Either<u32, Undefined> {
        self.inner.next_offset().into()
    }

    /// Copy the SDK's sealed canonical page without rebuilding its entries.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}

/// One current subject in the complete reachability partition.
#[napi(js_name = "WorkspaceImpact")]
pub struct WorkspaceImpact {
    inner: PortableWorkspaceImpact,
}

#[napi]
impl WorkspaceImpact {
    /// Clone the typed causal subject.
    #[napi]
    pub fn subject(&self) -> WorkspaceSubject {
        WorkspaceSubject::from_api(self.inner.subject())
    }

    /// Root-backed reachability classification.
    #[napi(getter)]
    pub fn state(&self) -> WorkspaceImpactState {
        match self.inner.state() {
            PortableWorkspaceImpactState::Affected => WorkspaceImpactState::Affected,
            PortableWorkspaceImpactState::Unaffected => WorkspaceImpactState::Unaffected,
        }
    }

    /// Exact number of contributing roots.
    #[napi(getter)]
    pub fn contributing_root_count(&self) -> u32 {
        self.inner.contributing_root_count()
    }
}

/// Closed reachability classification.
#[napi(js_name = "WorkspaceImpactState")]
pub enum WorkspaceImpactState {
    /// Reachable from at least one root.
    Affected,
    /// Outside every root closure.
    Unaffected,
}

/// One bounded page of the complete impact partition.
#[napi(js_name = "WorkspaceImpactPage")]
pub struct WorkspaceImpactPage {
    inner: pale_sdk::PortableWorkspaceImpactPage,
}

impl WorkspaceImpactPage {
    /// Retain the exact API page and its preconstructed canonical bytes.
    pub(crate) const fn from_api(inner: pale_sdk::PortableWorkspaceImpactPage) -> Self {
        Self { inner }
    }
}

#[napi]
impl WorkspaceImpactPage {
    /// Copy canonically ordered impact entries without exposing retained state.
    #[napi(getter)]
    pub fn entries(&self) -> Vec<WorkspaceImpact> {
        self.inner
            .entries()
            .iter()
            .cloned()
            .map(|inner| WorkspaceImpact { inner })
            .collect()
    }

    /// Exact next offset, or no value at the terminal page.
    #[napi(getter)]
    pub fn next_offset(&self) -> Either<u32, Undefined> {
        self.inner.next_offset().into()
    }

    /// Copy the SDK's sealed canonical page without reconstructing its entries.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}

/// Deterministic causal explanation result.
#[napi(js_name = "WorkspaceExplanation")]
pub struct WorkspaceExplanation {
    inner: PortableWorkspaceExplanation,
}

impl WorkspaceExplanation {
    /// Retain the canonical explanation without rebuilding its proof.
    pub(crate) fn from_api(inner: PortableWorkspaceExplanation) -> Self {
        Self { inner }
    }
}

#[napi]
impl WorkspaceExplanation {
    /// Exact query outcome.
    #[napi(getter)]
    pub fn state(&self) -> WorkspaceExplanationState {
        match self.inner.state() {
            PortableWorkspaceExplanationState::NotPresent => WorkspaceExplanationState::NotPresent,
            PortableWorkspaceExplanationState::Unaffected => WorkspaceExplanationState::Unaffected,
            PortableWorkspaceExplanationState::Affected => WorkspaceExplanationState::Affected,
        }
    }

    /// Exact query subject.
    #[napi]
    pub fn subject(&self) -> WorkspaceSubject {
        WorkspaceSubject::from_api(self.inner.subject())
    }
    /// Deterministically selected root, when affected.
    #[napi]
    pub fn root(&self) -> Option<WorkspaceChangeRoot> {
        self.inner.causal().map(|c| WorkspaceChangeRoot {
            inner: c.root().clone(),
        })
    }
    /// Proof frontier, when affected.
    #[napi(getter)]
    pub fn frontier(&self) -> Option<WorkspaceExplanationFrontier> {
        self.inner.causal().map(|c| match c.frontier() {
            PortableWorkspaceExplanationFrontier::Predecessor => {
                WorkspaceExplanationFrontier::Predecessor
            }
            PortableWorkspaceExplanationFrontier::Current => WorkspaceExplanationFrontier::Current,
        })
    }
    /// Complete shortest typed proof sequence.
    #[napi]
    pub fn steps(&self) -> Vec<WorkspaceCausalStep> {
        self.inner
            .causal()
            .map(|c| {
                c.steps()
                    .iter()
                    .cloned()
                    .map(|inner| WorkspaceCausalStep { inner })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Closed explanation state.
#[napi(js_name = "WorkspaceExplanationState")]
pub enum WorkspaceExplanationState {
    /// Subject is absent from the current partition.
    NotPresent,
    /// Subject exists but is unaffected.
    Unaffected,
    /// Subject has a causal witness.
    Affected,
}
/// Proof DAG frontier.
#[napi(js_name = "WorkspaceExplanationFrontier")]
pub enum WorkspaceExplanationFrontier {
    /// Predecessor proof DAG.
    Predecessor,
    /// Current proof DAG.
    Current,
}
/// Closed deterministic derivation rule.
#[napi(js_name = "WorkspaceDerivationRule")]
pub enum WorkspaceDerivationRule {
    /// Admit exact source bytes.
    SourceAdmission,
    /// Admit a semantic component.
    ComponentAdmission,
    /// Seal a selected module.
    ModuleSealing,
    /// Link a declaration to its owner.
    DeclarationLinking,
    /// Link an exact import surface.
    ImportLinking,
    /// Apply an extension.
    ExtensionApplication,
    /// Compose effective policy.
    PolicyComposition,
    /// Lower responsibility sources.
    ResponsibilityLowering,
    /// Lower organizational translation.
    OrganizationalTranslation,
    /// Admit experience composition.
    ExperienceComposition,
    /// Retain resource selection.
    ResourceSelection,
    /// Bind the resolution lock.
    ResolutionLocking,
    /// Bind the universe lock.
    UniverseLocking,
    /// Seal workspace semantic IR.
    SemanticIrSealing,
}
impl From<PortableWorkspaceDerivationRule> for WorkspaceDerivationRule {
    fn from(v: PortableWorkspaceDerivationRule) -> Self {
        match v {
            PortableWorkspaceDerivationRule::SourceAdmission => Self::SourceAdmission,
            PortableWorkspaceDerivationRule::ComponentAdmission => Self::ComponentAdmission,
            PortableWorkspaceDerivationRule::ModuleSealing => Self::ModuleSealing,
            PortableWorkspaceDerivationRule::DeclarationLinking => Self::DeclarationLinking,
            PortableWorkspaceDerivationRule::ImportLinking => Self::ImportLinking,
            PortableWorkspaceDerivationRule::ExtensionApplication => Self::ExtensionApplication,
            PortableWorkspaceDerivationRule::PolicyComposition => Self::PolicyComposition,
            PortableWorkspaceDerivationRule::ResponsibilityLowering => Self::ResponsibilityLowering,
            PortableWorkspaceDerivationRule::OrganizationalTranslation => {
                Self::OrganizationalTranslation
            }
            PortableWorkspaceDerivationRule::ExperienceComposition => Self::ExperienceComposition,
            PortableWorkspaceDerivationRule::ResourceSelection => Self::ResourceSelection,
            PortableWorkspaceDerivationRule::ResolutionLocking => Self::ResolutionLocking,
            PortableWorkspaceDerivationRule::UniverseLocking => Self::UniverseLocking,
            PortableWorkspaceDerivationRule::SemanticIrSealing => Self::SemanticIrSealing,
        }
    }
}

/// One complete typed compiler fact with its derivation and provenance.
#[napi(js_name = "WorkspaceCausalStep")]
pub struct WorkspaceCausalStep {
    inner: pale_sdk::PortableWorkspaceCausalStep,
}
#[napi]
impl WorkspaceCausalStep {
    /// Exact content address of this proof fact.
    #[napi(getter)]
    pub fn node_digest(&self) -> String {
        self.inner.node_digest().to_owned()
    }
    /// Clone the semantic subject produced at this step.
    #[napi]
    pub fn subject(&self) -> WorkspaceSubject {
        WorkspaceSubject::from_api(self.inner.subject())
    }
    /// Project the exact provenance classification.
    #[napi]
    pub fn provenance(&self) -> WorkspaceProvenance {
        WorkspaceProvenance::from_api(self.inner.provenance())
    }
    /// Closed deterministic compiler rule.
    #[napi(getter)]
    pub fn rule(&self) -> WorkspaceDerivationRule {
        self.inner.rule().into()
    }
    /// Project the exact typed fact produced by the rule.
    #[napi]
    pub fn fact(&self) -> WorkspaceFact {
        WorkspaceFact::from_api(self.inner.fact())
    }
}

/// Opaque verified comparison of two admitted workspaces.
#[napi(js_name = "WorkspaceComparison")]
pub struct WorkspaceComparison {
    inner: pale_sdk::PortableWorkspaceComparison,
}

impl WorkspaceComparison {
    pub(crate) fn inner(&self) -> &pale_sdk::PortableWorkspaceComparison {
        &self.inner
    }
}

#[napi]
impl WorkspaceComparison {
    /// Snapshot the API-owned comparison coordinates and partition counts.
    #[napi]
    pub fn summary(&self) -> WorkspaceComparisonSummary {
        WorkspaceComparisonSummary::from_api(self.inner().summary())
    }
    /// Test whether this comparison retains these exact ordered compilations.
    ///
    /// Equal public digests do not substitute for exact opaque source custody.
    /// This check grants no disclosure or runtime authority.
    #[napi]
    pub fn compares(
        &self,
        prior: &PortableWorkspaceCompilation,
        current: &PortableWorkspaceCompilation,
    ) -> bool {
        self.inner().compares(&prior.inner, &current.inner)
    }
    /// Principal-relative predecessor universe digest.
    #[napi(getter)]
    pub fn prior_principal_universe_digest(&self) -> String {
        self.inner().prior_principal_universe_digest().to_owned()
    }
    /// Principal-relative current universe digest.
    #[napi(getter)]
    pub fn current_principal_universe_digest(&self) -> String {
        self.inner().current_principal_universe_digest().to_owned()
    }
    /// Predecessor semantic-IR digest.
    #[napi(getter)]
    pub fn prior_semantic_ir_digest(&self) -> String {
        self.inner().prior_semantic_ir_digest().to_owned()
    }
    /// Current semantic-IR digest.
    #[napi(getter)]
    pub fn current_semantic_ir_digest(&self) -> String {
        self.inner().current_semantic_ir_digest().to_owned()
    }
    /// Exact derivation-receipt digest.
    #[napi(getter)]
    pub fn receipt_digest(&self) -> String {
        self.inner().receipt_digest().to_owned()
    }
    /// Exact change-root count.
    #[napi(getter)]
    pub fn change_root_count(&self) -> u32 {
        self.inner().change_root_count()
    }
    /// Exact affected-subject count.
    #[napi(getter)]
    pub fn affected_count(&self) -> u32 {
        self.inner().affected_count()
    }
    /// Exact unaffected-subject count.
    #[napi(getter)]
    pub fn unaffected_count(&self) -> u32 {
        self.inner().unaffected_count()
    }

    /// Project one bounded page of exact change roots.
    #[napi]
    pub fn change_roots(&self, offset: u32, limit: u16) -> WorkspaceChangeRootPage {
        WorkspaceChangeRootPage::from_api(self.inner().change_roots(offset, limit))
    }

    /// Project one bounded page of the complete impact partition.
    #[napi]
    pub fn impacts(&self, offset: u32, limit: u16) -> WorkspaceImpactPage {
        WorkspaceImpactPage::from_api(self.inner().impacts(offset, limit))
    }

    /// Compare one subject's presence and typed output in this retained pair.
    ///
    /// A compiler-produced subject from another comparison is also accepted;
    /// absence from both sides is a successful result. This synchronous query
    /// copies at most two selected facts and their encoded projection, not a
    /// fixed byte bound. It does not recompile, traverse impact or observe native
    /// state. Inspect dependency impact separately with `impacts` or `explain`.
    #[napi]
    pub fn compare_subject(&self, subject: &WorkspaceSubject) -> WorkspaceSubjectComparison {
        WorkspaceSubjectComparison {
            inner: self.inner().compare_subject(&subject.inner),
        }
    }

    /// Explain one subject obtained from this comparison.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the subject
    /// did not originate in this comparison coordinate.
    #[napi]
    pub fn explain(&self, env: Env, subject: &WorkspaceSubject) -> Result<WorkspaceExplanation> {
        self.inner()
            .explain(&subject.inner)
            .map(WorkspaceExplanation::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }
}

/// Owned comparison coordinates and complete partition counts.
///
/// This is the same API summary whether obtained from a workspace comparison
/// or a configuration-improvement preview; it owns no causal graph.
#[napi]
pub struct WorkspaceComparisonSummary {
    inner: pale_sdk::PortableWorkspaceComparisonSummary,
}
impl WorkspaceComparisonSummary {
    /// Retain the API summary and its exact preconstructed canonical bytes.
    pub(crate) fn from_api(inner: pale_sdk::PortableWorkspaceComparisonSummary) -> Self {
        Self { inner }
    }
}
#[napi]
impl WorkspaceComparisonSummary {
    /// Exact predecessor admitted-universe identity.
    #[napi(getter)]
    pub fn prior_principal_universe_digest(&self) -> String {
        self.inner.prior_principal_universe_digest().to_owned()
    }
    /// Exact current admitted-universe identity.
    #[napi(getter)]
    pub fn current_principal_universe_digest(&self) -> String {
        self.inner.current_principal_universe_digest().to_owned()
    }
    /// Predecessor semantic-IR identity.
    #[napi(getter)]
    pub fn prior_semantic_ir_digest(&self) -> String {
        self.inner.prior_semantic_ir_digest().to_owned()
    }
    /// Current semantic-IR identity.
    #[napi(getter)]
    pub fn current_semantic_ir_digest(&self) -> String {
        self.inner.current_semantic_ir_digest().to_owned()
    }
    /// Canonical derivation receipt identity.
    #[napi(getter)]
    pub fn receipt_digest(&self) -> String {
        self.inner.receipt_digest().to_owned()
    }
    /// Exact number of change roots.
    #[napi(getter)]
    pub fn change_root_count(&self) -> u32 {
        self.inner.change_root_count()
    }
    /// Number of affected current subjects.
    #[napi(getter)]
    pub fn affected_count(&self) -> u32 {
        self.inner.affected_count()
    }
    /// Number of unaffected current subjects.
    #[napi(getter)]
    pub fn unaffected_count(&self) -> u32 {
        self.inner.unaffected_count()
    }
    /// Exact API-owned canonical summary bytes.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}
