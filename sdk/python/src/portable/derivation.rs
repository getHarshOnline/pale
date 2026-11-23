// Python derivation-proof projection fragment included by the shallow façade.

/// Opaque typed subject emitted by verified comparison evidence.
#[pyclass(name = "WorkspaceSubject", frozen)]
pub struct PyWorkspaceSubject {
    pub(crate) inner: PortableWorkspaceSubject,
}

impl PyWorkspaceSubject {
    /// Clone one API-owned compound coordinate.
    fn from_api(inner: &PortableWorkspaceSubject) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[pymethods]
impl PyWorkspaceSubject {
    /// Closed coordinate family.
    #[getter]
    pub fn kind(&self) -> PyWorkspaceSubjectKind {
        match self.inner.kind() {
            PortableWorkspaceSubjectKind::Source => PyWorkspaceSubjectKind::Source,
            PortableWorkspaceSubjectKind::Component => PyWorkspaceSubjectKind::Component,
            PortableWorkspaceSubjectKind::Module => PyWorkspaceSubjectKind::Module,
            PortableWorkspaceSubjectKind::Declaration => PyWorkspaceSubjectKind::Declaration,
            PortableWorkspaceSubjectKind::Import => PyWorkspaceSubjectKind::Import,
            PortableWorkspaceSubjectKind::Extension => PyWorkspaceSubjectKind::Extension,
            PortableWorkspaceSubjectKind::Policy => PyWorkspaceSubjectKind::Policy,
            PortableWorkspaceSubjectKind::ResponsibilityClosure => {
                PyWorkspaceSubjectKind::ResponsibilityClosure
            }
            PortableWorkspaceSubjectKind::OrganizationalTranslationClosure => {
                PyWorkspaceSubjectKind::OrganizationalTranslationClosure
            }
            PortableWorkspaceSubjectKind::ExperienceComposition => {
                PyWorkspaceSubjectKind::ExperienceComposition
            }
            PortableWorkspaceSubjectKind::Resource => PyWorkspaceSubjectKind::Resource,
            PortableWorkspaceSubjectKind::ResolutionLock => PyWorkspaceSubjectKind::ResolutionLock,
            PortableWorkspaceSubjectKind::UniverseLock => PyWorkspaceSubjectKind::UniverseLock,
            PortableWorkspaceSubjectKind::Workspace => PyWorkspaceSubjectKind::Workspace,
        }
    }

    /// Workspace identity when carried by this subject.
    #[getter]
    pub fn workspace_id(&self) -> Option<&str> {
        self.inner.workspace_id()
    }

    /// Source identity when carried by this subject.
    #[getter]
    pub fn source_id(&self) -> Option<&str> {
        self.inner.source_id()
    }

    /// Component identity when carried by this subject.
    #[getter]
    pub fn component_id(&self) -> Option<&str> {
        self.inner.component_id()
    }

    /// Module identity when carried by this subject.
    #[getter]
    pub fn module_id(&self) -> Option<&str> {
        self.inner.module_id()
    }

    /// Owning module for declaration, extension, policy, or resource subjects.
    #[getter]
    pub fn owner_module_id(&self) -> Option<&str> {
        self.inner.owner_module_id()
    }

    /// Namespace of a declaration, policy, or extension target.
    #[getter]
    pub fn declaration_namespace_id(&self) -> Option<&str> {
        self.inner.declaration_namespace_id()
    }

    /// Local declaration identity when carried by this subject.
    #[getter]
    pub fn declaration_id(&self) -> Option<&str> {
        self.inner.declaration_id()
    }

    /// Module that declares an import path.
    #[getter]
    pub fn importer_module_id(&self) -> Option<&str> {
        self.inner.importer_module_id()
    }

    /// Upstream module selected by an import path.
    #[getter]
    pub fn upstream_module_id(&self) -> Option<&str> {
        self.inner.upstream_module_id()
    }

    /// Exact upstream release selected by an import path.
    #[getter]
    pub fn upstream_release_id(&self) -> Option<&str> {
        self.inner.upstream_release_id()
    }

    /// Named upstream export selected by an import path.
    #[getter]
    pub fn export_id(&self) -> Option<&str> {
        self.inner.export_id()
    }

    /// Upstream namespace selected by an import path.
    #[getter]
    pub fn source_namespace_id(&self) -> Option<&str> {
        self.inner.source_namespace_id()
    }

    /// Importer-local namespace bound by an import path.
    #[getter]
    pub fn local_namespace_id(&self) -> Option<&str> {
        self.inner.local_namespace_id()
    }

    /// Namespace that owns an extension rule.
    #[getter]
    pub fn extension_namespace_id(&self) -> Option<&str> {
        self.inner.extension_namespace_id()
    }

    /// Workspace-local resource identity for a resource subject.
    #[getter]
    pub fn resource_id(&self) -> Option<&str> {
        self.inner.resource_id()
    }
}

/// Closed workspace-subject family.
#[pyclass(name = "WorkspaceSubjectKind", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyWorkspaceSubjectKind {
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

/// Closed provenance family of one verified causal fact.
#[pyclass(name = "WorkspaceProvenanceKind", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyWorkspaceProvenanceKind {
    /// Directly admitted logical source.
    Source,
    /// Exact selected module release.
    ModuleRelease,
    /// Exact resolved import path.
    Import,
    /// Exact workspace extension.
    Extension,
    /// Deterministic compiler aggregate.
    Compilation,
}

/// Lossless immutable provenance projection.
#[pyclass(name = "WorkspaceProvenance", frozen)]
pub struct PyWorkspaceProvenance {
    inner: PortableWorkspaceProvenance,
}

impl PyWorkspaceProvenance {
    fn from_api(inner: &PortableWorkspaceProvenance) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[pymethods]
impl PyWorkspaceProvenance {
    /// Closed provenance family.
    #[getter]
    pub fn kind(&self) -> PyWorkspaceProvenanceKind {
        match &self.inner {
            PortableWorkspaceProvenance::Source { .. } => PyWorkspaceProvenanceKind::Source,
            PortableWorkspaceProvenance::ModuleRelease { .. } => {
                PyWorkspaceProvenanceKind::ModuleRelease
            }
            PortableWorkspaceProvenance::Import { .. } => PyWorkspaceProvenanceKind::Import,
            PortableWorkspaceProvenance::Extension { .. } => PyWorkspaceProvenanceKind::Extension,
            PortableWorkspaceProvenance::Compilation => PyWorkspaceProvenanceKind::Compilation,
        }
    }

    /// Source identity for direct-source provenance.
    #[getter]
    pub fn source_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Source { source_id } => Some(source_id),
            _ => None,
        }
    }

    /// Owning module for module-release or extension provenance.
    #[getter]
    pub fn module_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::ModuleRelease { module_id, .. } => Some(module_id),
            PortableWorkspaceProvenance::Extension {
                owner_module_id, ..
            } => Some(owner_module_id),
            _ => None,
        }
    }

    /// Selected release for module-release provenance.
    #[getter]
    pub fn release_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::ModuleRelease { release_id, .. } => Some(release_id),
            _ => None,
        }
    }

    /// Sealed module digest for module-release provenance.
    #[getter]
    pub fn module_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::ModuleRelease { module_digest, .. } => Some(module_digest),
            _ => None,
        }
    }

    /// Importing module for import provenance.
    #[getter]
    pub fn importer_module_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                importer_module_id, ..
            } => Some(importer_module_id),
            _ => None,
        }
    }

    /// Upstream module for import provenance.
    #[getter]
    pub fn upstream_module_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                upstream_module_id, ..
            } => Some(upstream_module_id),
            _ => None,
        }
    }

    /// Upstream release for import provenance.
    #[getter]
    pub fn upstream_release_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                upstream_release_id,
                ..
            } => Some(upstream_release_id),
            _ => None,
        }
    }

    /// Export identity for import provenance.
    #[getter]
    pub fn export_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Import { export_id, .. } => Some(export_id),
            _ => None,
        }
    }

    /// Upstream namespace for import provenance.
    #[getter]
    pub fn source_namespace_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                source_namespace_id,
                ..
            } => Some(source_namespace_id),
            _ => None,
        }
    }

    /// Local namespace for import provenance.
    #[getter]
    pub fn local_namespace_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                local_namespace_id, ..
            } => Some(local_namespace_id),
            _ => None,
        }
    }

    /// Extension-owning namespace.
    #[getter]
    pub fn extension_namespace_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Extension {
                extension_namespace_id,
                ..
            } => Some(extension_namespace_id),
            _ => None,
        }
    }

    /// Target declaration namespace for extension provenance.
    #[getter]
    pub fn target_namespace_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Extension {
                target_namespace_id,
                ..
            } => Some(target_namespace_id),
            _ => None,
        }
    }

    /// Target declaration identity for extension provenance.
    #[getter]
    pub fn target_declaration_id(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceProvenance::Extension {
                target_declaration_id,
                ..
            } => Some(target_declaration_id),
            _ => None,
        }
    }
}

/// Closed fact family produced by one derivation rule.
#[pyclass(name = "WorkspaceFactKind", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyWorkspaceFactKind {
    /// Exact admitted source content.
    SourceContent,
    /// Canonical semantic component.
    Component,
    /// Canonical selected module.
    Module,
    /// Resolved declaration value.
    Declaration,
    /// Resolved import surface.
    ImportSurface,
    /// Resolved extension record, not the resulting target declaration.
    Extension,
    /// Effective policy.
    EffectivePolicy,
    /// Responsibility closure.
    ResponsibilityClosure,
    /// Organizational-translation closure.
    OrganizationalTranslationClosure,
    /// Experience-composition source closure.
    ExperienceComposition,
    /// Selected resource value.
    Resource,
    /// Resolution-lock identity.
    ResolutionLock,
    /// Universe-lock identity.
    UniverseLock,
    /// Workspace semantic-IR identity.
    SemanticIr,
}

/// Lossless immutable typed-fact projection.
#[pyclass(name = "WorkspaceFact", frozen)]
pub struct PyWorkspaceFact {
    inner: PortableWorkspaceFact,
}

impl PyWorkspaceFact {
    fn from_api(inner: &PortableWorkspaceFact) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[pymethods]
impl PyWorkspaceFact {
    /// Closed fact family.
    #[getter]
    pub fn kind(&self) -> PyWorkspaceFactKind {
        match &self.inner {
            PortableWorkspaceFact::SourceContent { .. } => PyWorkspaceFactKind::SourceContent,
            PortableWorkspaceFact::Component { .. } => PyWorkspaceFactKind::Component,
            PortableWorkspaceFact::Module { .. } => PyWorkspaceFactKind::Module,
            PortableWorkspaceFact::Declaration { .. } => PyWorkspaceFactKind::Declaration,
            PortableWorkspaceFact::ImportSurface { .. } => PyWorkspaceFactKind::ImportSurface,
            PortableWorkspaceFact::Extension { .. } => PyWorkspaceFactKind::Extension,
            PortableWorkspaceFact::EffectivePolicy { .. } => PyWorkspaceFactKind::EffectivePolicy,
            PortableWorkspaceFact::ResponsibilityClosure { .. } => {
                PyWorkspaceFactKind::ResponsibilityClosure
            }
            PortableWorkspaceFact::OrganizationalTranslationClosure { .. } => {
                PyWorkspaceFactKind::OrganizationalTranslationClosure
            }
            PortableWorkspaceFact::ExperienceComposition { .. } => {
                PyWorkspaceFactKind::ExperienceComposition
            }
            PortableWorkspaceFact::Resource { .. } => PyWorkspaceFactKind::Resource,
            PortableWorkspaceFact::ResolutionLock { .. } => PyWorkspaceFactKind::ResolutionLock,
            PortableWorkspaceFact::UniverseLock { .. } => PyWorkspaceFactKind::UniverseLock,
            PortableWorkspaceFact::SemanticIr { .. } => PyWorkspaceFactKind::SemanticIr,
        }
    }

    /// Source path for source-content facts.
    #[getter]
    pub fn source_path(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::SourceContent { source_path, .. } => Some(source_path),
            _ => None,
        }
    }

    /// Source digest for source-content or experience-composition facts.
    #[getter]
    pub fn source_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::SourceContent { source_digest, .. }
            | PortableWorkspaceFact::ExperienceComposition { source_digest } => Some(source_digest),
            _ => None,
        }
    }

    /// Exact admitted byte length for source-content facts.
    #[getter]
    pub fn byte_len(&self) -> Option<u32> {
        match self.inner {
            PortableWorkspaceFact::SourceContent { byte_len, .. } => Some(byte_len),
            _ => None,
        }
    }

    /// Component digest for component facts.
    #[getter]
    pub fn component_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::Component { component_digest } => Some(component_digest),
            _ => None,
        }
    }

    /// Module digest for module facts.
    #[getter]
    pub fn module_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::Module { module_digest } => Some(module_digest),
            _ => None,
        }
    }

    /// Domain-separated value digest for value-bearing fact families.
    #[getter]
    pub fn value_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::Declaration { value_digest }
            | PortableWorkspaceFact::Extension { value_digest }
            | PortableWorkspaceFact::EffectivePolicy { value_digest }
            | PortableWorkspaceFact::Resource { value_digest } => Some(value_digest),
            _ => None,
        }
    }

    /// Import-surface digest for import facts.
    #[getter]
    pub fn surface_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::ImportSurface { surface_digest } => Some(surface_digest),
            _ => None,
        }
    }

    /// Closure digest for responsibility or organizational-translation facts.
    #[getter]
    pub fn closure_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::ResponsibilityClosure { closure_digest }
            | PortableWorkspaceFact::OrganizationalTranslationClosure { closure_digest } => {
                Some(closure_digest)
            }
            _ => None,
        }
    }

    /// Resolution-lock digest for resolution facts.
    #[getter]
    pub fn resolution_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::ResolutionLock { resolution_digest } => Some(resolution_digest),
            _ => None,
        }
    }

    /// Universe-lock digest for universe facts.
    #[getter]
    pub fn universe_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::UniverseLock { universe_digest } => Some(universe_digest),
            _ => None,
        }
    }

    /// Workspace semantic-IR digest for the terminal fact.
    #[getter]
    pub fn workspace_semantic_ir_digest(&self) -> Option<&str> {
        match &self.inner {
            PortableWorkspaceFact::SemanticIr {
                workspace_semantic_ir_digest,
            } => Some(workspace_semantic_ir_digest),
            _ => None,
        }
    }
}

/// One exact source-change root.
#[pyclass(name = "WorkspaceChangeRoot", frozen)]
pub struct PyWorkspaceChangeRoot {
    inner: pale_sdk::PortableWorkspaceChangeRoot,
}

#[pymethods]
impl PyWorkspaceChangeRoot {
    /// Exact root content address.
    #[getter]
    pub fn root_digest(&self) -> &str {
        self.inner.root_digest()
    }

    /// Closed source transition.
    #[getter]
    pub fn kind(&self) -> PyWorkspaceChangeKind {
        match self.inner.kind() {
            pale_sdk::PortableWorkspaceChangeKind::Added => PyWorkspaceChangeKind::Added,
            pale_sdk::PortableWorkspaceChangeKind::Modified => PyWorkspaceChangeKind::Modified,
            pale_sdk::PortableWorkspaceChangeKind::Removed => PyWorkspaceChangeKind::Removed,
        }
    }

    /// Clone the typed subject for inspection or explanation.
    pub fn subject(&self) -> PyWorkspaceSubject {
        PyWorkspaceSubject::from_api(self.inner.subject())
    }

    /// Exact predecessor fact, absent only for an added root.
    pub fn prior_fact(&self) -> Option<PyWorkspaceFact> {
        self.inner.prior_fact().map(PyWorkspaceFact::from_api)
    }

    /// Exact current fact, absent only for a removed root.
    pub fn current_fact(&self) -> Option<PyWorkspaceFact> {
        self.inner.current_fact().map(PyWorkspaceFact::from_api)
    }
}

/// Closed source transition of one exact change root.
#[pyclass(name = "WorkspaceChangeKind", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyWorkspaceChangeKind {
    /// No predecessor fact exists.
    Added,
    /// Predecessor and current facts differ.
    Modified,
    /// No current fact exists.
    Removed,
}

/// One bounded page of canonically ordered change roots.
#[pyclass(name = "WorkspaceChangeRootPage", frozen)]
pub struct PyWorkspaceChangeRootPage {
    entries: Vec<PyWorkspaceChangeRoot>,
    next_offset: Option<u32>,
}

impl PyWorkspaceChangeRootPage {
    pub(crate) fn from_api(page: pale_sdk::PortableWorkspaceChangeRootPage) -> Self {
        Self {
            next_offset: page.next_offset(),
            entries: page
                .entries()
                .iter()
                .cloned()
                .map(|inner| PyWorkspaceChangeRoot { inner })
                .collect(),
        }
    }
}

#[pymethods]
impl PyWorkspaceChangeRootPage {
    /// Clone the canonically ordered page entries.
    #[getter]
    pub fn entries(&self) -> Vec<PyWorkspaceChangeRoot> {
        self.entries
            .iter()
            .map(|entry| PyWorkspaceChangeRoot {
                inner: entry.inner.clone(),
            })
            .collect()
    }

    /// Exact next offset, or no value for a terminal page.
    #[getter]
    pub fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }
}

/// One member of the complete current reachability partition.
#[pyclass(name = "WorkspaceImpact", frozen)]
pub struct PyWorkspaceImpact {
    inner: PortableWorkspaceImpact,
}

#[pymethods]
impl PyWorkspaceImpact {
    /// Clone the typed subject for inspection or explanation.
    pub fn subject(&self) -> PyWorkspaceSubject {
        PyWorkspaceSubject::from_api(self.inner.subject())
    }

    /// Root-backed reachability classification.
    #[getter]
    pub fn state(&self) -> PyWorkspaceImpactState {
        match self.inner.state() {
            PortableWorkspaceImpactState::Affected => PyWorkspaceImpactState::Affected,
            PortableWorkspaceImpactState::Unaffected => PyWorkspaceImpactState::Unaffected,
        }
    }

    /// Exact number of contributing roots.
    #[getter]
    pub fn contributing_root_count(&self) -> u32 {
        self.inner.contributing_root_count()
    }
}

/// Closed reachability state of one current subject.
#[pyclass(name = "WorkspaceImpactState", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyWorkspaceImpactState {
    /// Reachable from at least one exact root.
    Affected,
    /// Outside every exact root closure.
    Unaffected,
}

/// One bounded page of the complete current impact partition.
#[pyclass(name = "WorkspaceImpactPage", frozen)]
pub struct PyWorkspaceImpactPage {
    canonical_bytes: Vec<u8>,
    entries: Vec<PyWorkspaceImpact>,
    next_offset: Option<u32>,
}

impl PyWorkspaceImpactPage {
    pub(crate) fn from_api(page: pale_sdk::PortableWorkspaceImpactPage) -> Self {
        Self {
            canonical_bytes: page.canonical_bytes().to_vec(),
            next_offset: page.next_offset(),
            entries: page
                .entries()
                .iter()
                .cloned()
                .map(|inner| PyWorkspaceImpact { inner })
                .collect(),
        }
    }
}

#[pymethods]
impl PyWorkspaceImpactPage {
    /// Copy the SDK's sealed canonical page without reconstructing its entries.
    #[getter]
    pub fn canonical_bytes(&self, py: Python<'_>) -> Py<PyBytes> {
        crate::python_bytes(py, &self.canonical_bytes)
    }

    /// Clone the canonically ordered page entries.
    #[getter]
    pub fn entries(&self) -> Vec<PyWorkspaceImpact> {
        self.entries
            .iter()
            .map(|entry| PyWorkspaceImpact {
                inner: entry.inner.clone(),
            })
            .collect()
    }

    /// Exact next offset, or no value for a terminal page.
    #[getter]
    pub fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }
}

/// Closed deterministic compiler rule of one proof step.
#[pyclass(name = "WorkspaceDerivationRule", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyWorkspaceDerivationRule {
    /// Admit exact source bytes.
    SourceAdmission,
    /// Admit one semantic component.
    ComponentAdmission,
    /// Seal one selected module.
    ModuleSealing,
    /// Link a declaration to its owner.
    DeclarationLinking,
    /// Link one exact import surface.
    ImportLinking,
    /// Apply one extension.
    ExtensionApplication,
    /// Compose policy obligations.
    PolicyComposition,
    /// Lower responsibility sources.
    ResponsibilityLowering,
    /// Lower organizational translation.
    OrganizationalTranslation,
    /// Admit experience composition.
    ExperienceComposition,
    /// Retain one resource selection.
    ResourceSelection,
    /// Bind the resolution lock.
    ResolutionLocking,
    /// Bind the universe lock.
    UniverseLocking,
    /// Seal workspace semantic IR.
    SemanticIrSealing,
}

impl From<PortableWorkspaceDerivationRule> for PyWorkspaceDerivationRule {
    fn from(value: PortableWorkspaceDerivationRule) -> Self {
        match value {
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

/// Which admitted proof DAG owns a causal witness.
#[pyclass(
    name = "WorkspaceExplanationFrontier",
    eq,
    eq_int,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyWorkspaceExplanationFrontier {
    /// Predecessor DAG for a removed root.
    Predecessor,
    /// Current DAG for an added or modified root.
    Current,
}

/// One complete typed fact on a shortest causal path.
#[pyclass(name = "WorkspaceCausalStep", frozen)]
pub struct PyWorkspaceCausalStep {
    inner: pale_sdk::PortableWorkspaceCausalStep,
}

#[pymethods]
impl PyWorkspaceCausalStep {
    /// Content address of this exact proof fact.
    #[getter]
    pub fn node_digest(&self) -> &str {
        self.inner.node_digest()
    }

    /// Clone the semantic subject produced at this step.
    pub fn subject(&self) -> PyWorkspaceSubject {
        PyWorkspaceSubject::from_api(self.inner.subject())
    }

    /// Clone the exact provenance classification.
    pub fn provenance(&self) -> PyWorkspaceProvenance {
        PyWorkspaceProvenance::from_api(self.inner.provenance())
    }

    /// Closed deterministic compiler rule.
    #[getter]
    pub fn rule(&self) -> PyWorkspaceDerivationRule {
        self.inner.rule().into()
    }

    /// Clone the exact typed fact produced by the rule.
    pub fn fact(&self) -> PyWorkspaceFact {
        PyWorkspaceFact::from_api(self.inner.fact())
    }
}

/// Deterministic causal explanation result.
#[pyclass(name = "WorkspaceExplanation", frozen)]
pub struct PyWorkspaceExplanation {
    inner: PortableWorkspaceExplanation,
}

impl PyWorkspaceExplanation {
    pub(crate) fn from_api(inner: PortableWorkspaceExplanation) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyWorkspaceExplanation {
    /// Exact query outcome.
    #[getter]
    pub fn state(&self) -> PyWorkspaceExplanationState {
        match self.inner.state() {
            PortableWorkspaceExplanationState::NotPresent => {
                PyWorkspaceExplanationState::NotPresent
            }
            PortableWorkspaceExplanationState::Unaffected => {
                PyWorkspaceExplanationState::Unaffected
            }
            PortableWorkspaceExplanationState::Affected => PyWorkspaceExplanationState::Affected,
        }
    }

    /// Clone the exact query subject.
    pub fn subject(&self) -> PyWorkspaceSubject {
        PyWorkspaceSubject::from_api(self.inner.subject())
    }

    /// Deterministically selected exact root, when affected.
    pub fn root(&self) -> Option<PyWorkspaceChangeRoot> {
        self.inner.causal().map(|causal| PyWorkspaceChangeRoot {
            inner: causal.root().clone(),
        })
    }

    /// Proof DAG traversed by the witness, when affected.
    #[getter]
    pub fn frontier(&self) -> Option<PyWorkspaceExplanationFrontier> {
        self.inner.causal().map(|causal| match causal.frontier() {
            PortableWorkspaceExplanationFrontier::Predecessor => {
                PyWorkspaceExplanationFrontier::Predecessor
            }
            PortableWorkspaceExplanationFrontier::Current => {
                PyWorkspaceExplanationFrontier::Current
            }
        })
    }

    /// Complete shortest root-to-target typed proof sequence.
    pub fn steps(&self) -> Vec<PyWorkspaceCausalStep> {
        self.inner
            .causal()
            .map(|causal| {
                causal
                    .steps()
                    .iter()
                    .cloned()
                    .map(|inner| PyWorkspaceCausalStep { inner })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Closed outcome of one causal explanation query.
#[pyclass(name = "WorkspaceExplanationState", eq, eq_int, frozen, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyWorkspaceExplanationState {
    /// Subject absent from the current partition.
    NotPresent,
    /// Subject present but unaffected.
    Unaffected,
    /// Subject present with one causal witness.
    Affected,
}

/// Opaque verified comparison of two admitted workspaces.
#[pyclass(name = "WorkspaceComparison", frozen)]
pub struct PyWorkspaceComparison {
    inner: Arc<pale_sdk::PortableWorkspaceComparison>,
}

#[pymethods]
impl PyWorkspaceComparison {
    /// Principal-relative predecessor-universe digest.
    #[getter]
    pub fn prior_principal_universe_digest(&self) -> &str {
        self.inner.prior_principal_universe_digest()
    }

    /// Principal-relative current-universe digest.
    #[getter]
    pub fn current_principal_universe_digest(&self) -> &str {
        self.inner.current_principal_universe_digest()
    }

    /// Verified predecessor semantic-IR digest.
    #[getter]
    pub fn prior_semantic_ir_digest(&self) -> &str {
        self.inner.prior_semantic_ir_digest()
    }

    /// Verified current semantic-IR digest.
    #[getter]
    pub fn current_semantic_ir_digest(&self) -> &str {
        self.inner.current_semantic_ir_digest()
    }

    /// Exact derivation-receipt digest.
    #[getter]
    pub fn receipt_digest(&self) -> &str {
        self.inner.receipt_digest()
    }

    /// Compare presence and typed values for one complete semantic subject.
    ///
    /// A subject from another compilation is valid: absence is an explicit
    /// result, not a failed query. Python objects other than `WorkspaceSubject`
    /// are rejected. Equality is separate from dependency impact; this call
    /// does not read source or establish permission to disclose its results.
    pub fn compare_subject(
        &self,
        py: Python<'_>,
        subject: &PyWorkspaceSubject,
    ) -> PyWorkspaceSubjectComparison {
        let comparison = Arc::clone(&self.inner);
        let subject = subject.inner.clone();
        let inner = py.detach(move || comparison.compare_subject(&subject));
        PyWorkspaceSubjectComparison { inner }
    }

    /// Number of exact source-change roots.
    #[getter]
    pub fn change_root_count(&self) -> u32 {
        self.inner.change_root_count()
    }

    /// Number of current subjects reachable from a root.
    #[getter]
    pub fn affected_count(&self) -> u32 {
        self.inner.affected_count()
    }

    /// Number of current subjects outside every root closure.
    #[getter]
    pub fn unaffected_count(&self) -> u32 {
        self.inner.unaffected_count()
    }

    /// Project one bounded page of exact change roots.
    pub fn change_roots(
        &self,
        py: Python<'_>,
        offset: u32,
        limit: u16,
    ) -> PyWorkspaceChangeRootPage {
        let comparison = Arc::clone(&self.inner);
        py.detach(move || {
            PyWorkspaceChangeRootPage::from_api(comparison.change_roots(offset, limit))
        })
    }

    /// Project one bounded page of the complete impact partition.
    pub fn impacts(&self, py: Python<'_>, offset: u32, limit: u16) -> PyWorkspaceImpactPage {
        let comparison = Arc::clone(&self.inner);
        py.detach(move || PyWorkspaceImpactPage::from_api(comparison.impacts(offset, limit)))
    }

    /// Explain the dependency impact of a complete semantic subject.
    ///
    /// A subject from another compilation is valid. A missing subject returns
    /// `NotPresent`, not an origin error.
    ///
    /// # Errors
    ///
    /// Raises [`crate::PaleDiagnosticError`] if the admitted proof contradicts
    /// its retained subject or causal witness.
    pub fn explain(
        &self,
        py: Python<'_>,
        subject: &PyWorkspaceSubject,
    ) -> PyResult<PyWorkspaceExplanation> {
        let comparison = Arc::clone(&self.inner);
        let subject = subject.inner.clone();
        py.detach(move || comparison.explain(&subject))
            .map(PyWorkspaceExplanation::from_api)
            .map_err(|error| diagnostic_error(py, error))
    }
}
