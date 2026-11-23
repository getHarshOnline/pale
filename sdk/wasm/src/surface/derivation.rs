// WebAssembly derivation-proof projection fragment included by the crate façade.

/// Opaque comparison and retained proof owner for two workspace handles.
#[wasm_bindgen(js_name = PortableWorkspaceComparison)]
pub struct WasmPortableWorkspaceComparison {
    inner: PortableWorkspaceComparison,
}

#[wasm_bindgen(js_class = PortableWorkspaceComparison)]
impl WasmPortableWorkspaceComparison {
    /// Principal-relative predecessor universe digest.
    #[wasm_bindgen(getter = priorPrincipalUniverseDigest)]
    pub fn prior_principal_universe_digest(&self) -> String {
        self.inner.prior_principal_universe_digest().to_owned()
    }
    /// Principal-relative current universe digest.
    #[wasm_bindgen(getter = currentPrincipalUniverseDigest)]
    pub fn current_principal_universe_digest(&self) -> String {
        self.inner.current_principal_universe_digest().to_owned()
    }
    /// Predecessor semantic-IR digest.
    #[wasm_bindgen(getter = priorSemanticIrDigest)]
    pub fn prior_semantic_ir_digest(&self) -> String {
        self.inner.prior_semantic_ir_digest().to_owned()
    }
    /// Current semantic-IR digest.
    #[wasm_bindgen(getter = currentSemanticIrDigest)]
    pub fn current_semantic_ir_digest(&self) -> String {
        self.inner.current_semantic_ir_digest().to_owned()
    }
    /// Domain-separated identity of the verified derivation receipt.
    #[wasm_bindgen(getter = receiptDigest)]
    pub fn receipt_digest(&self) -> String {
        self.inner.receipt_digest().to_owned()
    }

    /// Number of exact source-change roots.
    #[wasm_bindgen(getter = changeRootCount)]
    pub fn change_root_count(&self) -> u32 {
        self.inner.change_root_count()
    }

    /// Number of current subjects reachable from a root.
    #[wasm_bindgen(getter = affectedCount)]
    pub fn affected_count(&self) -> u32 {
        self.inner.affected_count()
    }

    /// Number of current subjects outside every root closure.
    #[wasm_bindgen(getter = unaffectedCount)]
    pub fn unaffected_count(&self) -> u32 {
        self.inner.unaffected_count()
    }

    /// Project one bounded page of exact change roots.
    ///
    /// Offset and limit must be JavaScript numbers representing exact integers.
    /// The offset range is `0..=u32::MAX`; the limit range is `0..=u16::MAX`.
    /// Invalid values return a host error instead of being coerced.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` when either page coordinate is not an exact
    /// integer in its declared range.
    #[wasm_bindgen(js_name = changeRoots)]
    pub fn change_roots(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number")] offset: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] limit: JsValue,
    ) -> Result<WasmWorkspaceChangeRootPage, JsValue> {
        let offset = javascript_u32(&offset, u32::MAX, "change-root page offset")?;
        let limit = javascript_u32(
            &limit,
            u32::from(u16::MAX),
            "change-root page limit",
        )? as u16;
        let page = self.inner.change_roots(offset, limit);
        Ok(WasmWorkspaceChangeRootPage {
            canonical_bytes: page.canonical_bytes().to_vec(),
            entries: page
                .entries()
                .iter()
                .cloned()
                .map(WasmWorkspaceChangeRoot::from_api)
                .collect(),
            next_offset: page.next_offset(),
        })
    }

    /// Project one bounded page of the complete impact partition.
    ///
    /// Offset and limit must be JavaScript numbers representing exact integers.
    /// The offset range is `0..=u32::MAX`; the limit range is `0..=u16::MAX`.
    /// Invalid values return a host error instead of being coerced.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` when either page coordinate is not an exact
    /// integer in its declared range.
    #[wasm_bindgen]
    pub fn impacts(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number")] offset: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] limit: JsValue,
    ) -> Result<WasmWorkspaceImpactPage, JsValue> {
        let offset = javascript_u32(&offset, u32::MAX, "impact page offset")?;
        let limit = javascript_u32(&limit, u32::from(u16::MAX), "impact page limit")?
            as u16;
        let page = self.inner.impacts(offset, limit);
        Ok(WasmWorkspaceImpactPage {
            canonical_bytes: page.canonical_bytes().to_vec(),
            entries: page
                .entries()
                .iter()
                .cloned()
                .map(WasmWorkspaceImpact::from_api)
                .collect(),
            next_offset: page.next_offset(),
        })
    }

    /// Compare the typed outputs for one full subject identity.
    ///
    /// Membership is separate from equality and dependency impact. A valid
    /// subject from another compilation can be absent from both sides.
    /// This synchronous call copies up to two facts and encodes the answer;
    /// it does not provide a fixed byte or elapsed-time bound.
    ///
    /// A valid supplied subject handle is consumed. Use the result's `subject()`
    /// method to obtain a new copy for another query.
    ///
    /// # Errors
    ///
    /// Returns a host error for a missing, freed, or wrong-class subject without
    /// retaining a borrow of this comparison.
    #[wasm_bindgen(js_name = compareSubject)]
    pub fn compare_subject(
        &self,
        #[wasm_bindgen(unchecked_param_type = "WorkspaceSubject")] subject: JsValue,
    ) -> Result<WasmWorkspaceSubjectComparison, JsValue> {
        let subject = comparison_subject(subject)?;
        Ok(WasmWorkspaceSubjectComparison {
            inner: self.inner.compare_subject(&subject.inner),
        })
    }

    /// Explain a full subject identity against this retained comparison.
    ///
    /// A valid subject from another compilation is accepted. If this pair does
    /// not contain it, the answer is `NotPresent`, not an identity error.
    ///
    /// # Errors
    ///
    /// Returns a native `Error` with separate structured `diagnostic` data if
    /// admitted proof contradicts its comparison identities or causal invariants.
    pub fn explain(
        &self,
        subject: &WasmWorkspaceSubject,
    ) -> Result<WasmWorkspaceExplanation, JsValue> {
        self.inner
            .explain(&subject.inner)
            .map(WasmWorkspaceExplanation::from_api)
            .map_err(diagnostic_error)
    }
}

/// Validate ownership before accessing the subject's native value.
///
/// A borrowed exported-class argument can throw for a zero pointer after the
/// receiver has already been borrowed. The pinned consuming conversion returns
/// an ordinary error instead, so the receiver's borrow is released normally.
/// [`wasm_bindgen::convert::TryFromJsValue`] is an unstable internal API; upgrades
/// must pass the actual-JavaScript rejection, consumption and release tests.
fn comparison_subject(value: JsValue) -> Result<WasmWorkspaceSubject, JsValue> {
    use wasm_bindgen::convert::TryFromJsValue;

    WasmWorkspaceSubject::try_from_js_value(value).map_err(|_| {
        javascript_error("subject must be an unconsumed workspace subject")
            .unwrap_or_else(std::convert::identity)
    })
}

/// Subject membership in the comparison's predecessor and current compilations.
#[wasm_bindgen(js_name = WorkspaceSubjectMembership)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceSubjectMembership {
    /// Neither compilation contains the subject.
    Absent,
    /// Only the predecessor contains the subject.
    PriorOnly,
    /// Only the current compilation contains the subject.
    CurrentOnly,
    /// Both compilations contain the subject.
    Shared,
}

/// Equality of shared compiler outputs, independent of dependency impact.
#[wasm_bindgen(js_name = WorkspaceValueComparison)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceValueComparison {
    /// Both typed outputs are equal.
    Equal,
    /// The typed outputs differ.
    Changed,
}

/// Owned SDK answer for one subject, not another retained compilation.
///
/// The receipt joins the parent comparison's prior/current identities. This
/// result and copied facts remain usable after the parent is freed. Free each
/// returned handle when finished; no handle grants disclosure or effect authority.
#[wasm_bindgen(js_name = WorkspaceSubjectComparison)]
pub struct WasmWorkspaceSubjectComparison {
    inner: PortableWorkspaceSubjectComparison,
}

#[wasm_bindgen(js_class = WorkspaceSubjectComparison)]
impl WasmWorkspaceSubjectComparison {
    /// Identity of the parent comparison that produced this answer.
    #[wasm_bindgen(getter = receiptDigest)]
    pub fn receipt_digest(&self) -> String {
        self.inner.receipt_digest().to_owned()
    }

    /// Copy the full subject, including all compound identity fields.
    pub fn subject(&self) -> WasmWorkspaceSubject {
        WasmWorkspaceSubject::from_api(self.inner.subject())
    }

    /// Identify which sides contain a value, without deriving equality.
    pub fn membership(&self) -> WasmWorkspaceSubjectMembership {
        match self.inner.values() {
            PortableWorkspaceSubjectValues::Absent => WasmWorkspaceSubjectMembership::Absent,
            PortableWorkspaceSubjectValues::PriorOnly { .. } => {
                WasmWorkspaceSubjectMembership::PriorOnly
            }
            PortableWorkspaceSubjectValues::CurrentOnly { .. } => {
                WasmWorkspaceSubjectMembership::CurrentOnly
            }
            PortableWorkspaceSubjectValues::Shared { .. } => WasmWorkspaceSubjectMembership::Shared,
        }
    }

    /// Copy the predecessor fact for prior-only or shared membership.
    ///
    /// Returns no handle for absent or current-only membership.
    #[wasm_bindgen(js_name = priorFact)]
    pub fn prior_fact(&self) -> Option<WasmWorkspaceFact> {
        match self.inner.values() {
            PortableWorkspaceSubjectValues::PriorOnly { prior }
            | PortableWorkspaceSubjectValues::Shared { prior, .. } => Some(WasmWorkspaceFact {
                inner: prior.clone(),
            }),
            _ => None,
        }
    }

    /// Copy the current fact for current-only or shared membership.
    ///
    /// Returns no handle for absent or prior-only membership.
    #[wasm_bindgen(js_name = currentFact)]
    pub fn current_fact(&self) -> Option<WasmWorkspaceFact> {
        match self.inner.values() {
            PortableWorkspaceSubjectValues::CurrentOnly { current }
            | PortableWorkspaceSubjectValues::Shared { current, .. } => Some(WasmWorkspaceFact {
                inner: current.clone(),
            }),
            _ => None,
        }
    }

    /// Return equality only for shared membership; otherwise return no value.
    ///
    /// An affected subject can still have equal outputs.
    pub fn comparison(&self) -> Option<WasmWorkspaceValueComparison> {
        match self.inner.values() {
            PortableWorkspaceSubjectValues::Shared { comparison, .. } => Some(match comparison {
                PortableWorkspaceValueComparison::Equal => WasmWorkspaceValueComparison::Equal,
                PortableWorkspaceValueComparison::Changed => WasmWorkspaceValueComparison::Changed,
            }),
            _ => None,
        }
    }

    /// Copy the SDK's exact canonical bytes without parsing or re-encoding them.
    #[wasm_bindgen(js_name = canonicalBytes)]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.inner.canonical_bytes().to_vec()
    }
}

/// Opaque typed subject coordinate emitted by verified comparison evidence.
#[wasm_bindgen(js_name = WorkspaceSubject)]
pub struct WasmWorkspaceSubject {
    inner: PortableWorkspaceSubject,
}

impl WasmWorkspaceSubject {
    /// Clone one API-owned subject without reparsing a compound identity.
    fn from_api(inner: &PortableWorkspaceSubject) -> Self {
        Self {
            inner: inner.clone(),
        }
    }
}

#[wasm_bindgen(js_class = WorkspaceSubject)]
impl WasmWorkspaceSubject {
    /// Return the closed coordinate family.
    pub fn kind(&self) -> WasmWorkspaceSubjectKind {
        match self.inner.kind() {
            PortableWorkspaceSubjectKind::Source => WasmWorkspaceSubjectKind::Source,
            PortableWorkspaceSubjectKind::Component => WasmWorkspaceSubjectKind::Component,
            PortableWorkspaceSubjectKind::Module => WasmWorkspaceSubjectKind::Module,
            PortableWorkspaceSubjectKind::Declaration => WasmWorkspaceSubjectKind::Declaration,
            PortableWorkspaceSubjectKind::Import => WasmWorkspaceSubjectKind::Import,
            PortableWorkspaceSubjectKind::Extension => WasmWorkspaceSubjectKind::Extension,
            PortableWorkspaceSubjectKind::Policy => WasmWorkspaceSubjectKind::Policy,
            PortableWorkspaceSubjectKind::ResponsibilityClosure => {
                WasmWorkspaceSubjectKind::ResponsibilityClosure
            }
            PortableWorkspaceSubjectKind::OrganizationalTranslationClosure => {
                WasmWorkspaceSubjectKind::OrganizationalTranslationClosure
            }
            PortableWorkspaceSubjectKind::ExperienceComposition => {
                WasmWorkspaceSubjectKind::ExperienceComposition
            }
            PortableWorkspaceSubjectKind::Resource => WasmWorkspaceSubjectKind::Resource,
            PortableWorkspaceSubjectKind::ResolutionLock => {
                WasmWorkspaceSubjectKind::ResolutionLock
            }
            PortableWorkspaceSubjectKind::UniverseLock => WasmWorkspaceSubjectKind::UniverseLock,
            PortableWorkspaceSubjectKind::Workspace => WasmWorkspaceSubjectKind::Workspace,
        }
    }

    /// Receipt-local source identity, when this is a source subject.
    #[wasm_bindgen(getter = sourceId)]
    pub fn source_id(&self) -> Option<String> {
        self.inner.source_id().map(str::to_owned)
    }

    /// Component identity, when this is a component subject.
    #[wasm_bindgen(getter = componentId)]
    pub fn component_id(&self) -> Option<String> {
        self.inner.component_id().map(str::to_owned)
    }

    /// Module identity, when this is a module subject.
    #[wasm_bindgen(getter = moduleId)]
    pub fn module_id(&self) -> Option<String> {
        self.inner.module_id().map(str::to_owned)
    }

    /// Owning module for declaration-like subjects.
    #[wasm_bindgen(getter = ownerModuleId)]
    pub fn owner_module_id(&self) -> Option<String> {
        self.inner.owner_module_id().map(str::to_owned)
    }

    /// Namespace of a declaration-like subject.
    #[wasm_bindgen(getter = declarationNamespaceId)]
    pub fn declaration_namespace_id(&self) -> Option<String> {
        self.inner.declaration_namespace_id().map(str::to_owned)
    }

    /// Local declaration identity, when carried by this subject.
    #[wasm_bindgen(getter = declarationId)]
    pub fn declaration_id(&self) -> Option<String> {
        self.inner.declaration_id().map(str::to_owned)
    }

    /// Importer module identity, when carried.
    #[wasm_bindgen(getter = importerModuleId)]
    pub fn importer_module_id(&self) -> Option<String> {
        self.inner.importer_module_id().map(str::to_owned)
    }
    /// Upstream module identity, when carried.
    #[wasm_bindgen(getter = upstreamModuleId)]
    pub fn upstream_module_id(&self) -> Option<String> {
        self.inner.upstream_module_id().map(str::to_owned)
    }
    /// Upstream release identity, when carried.
    #[wasm_bindgen(getter = upstreamReleaseId)]
    pub fn upstream_release_id(&self) -> Option<String> {
        self.inner.upstream_release_id().map(str::to_owned)
    }
    /// Selected export identity, when carried.
    #[wasm_bindgen(getter = exportId)]
    pub fn export_id(&self) -> Option<String> {
        self.inner.export_id().map(str::to_owned)
    }
    /// Upstream namespace identity, when carried.
    #[wasm_bindgen(getter = sourceNamespaceId)]
    pub fn source_namespace_id(&self) -> Option<String> {
        self.inner.source_namespace_id().map(str::to_owned)
    }
    /// Importer-local namespace identity, when carried.
    #[wasm_bindgen(getter = localNamespaceId)]
    pub fn local_namespace_id(&self) -> Option<String> {
        self.inner.local_namespace_id().map(str::to_owned)
    }
    /// Extension namespace identity, when carried.
    #[wasm_bindgen(getter = extensionNamespaceId)]
    pub fn extension_namespace_id(&self) -> Option<String> {
        self.inner.extension_namespace_id().map(str::to_owned)
    }
    /// Resource identity, when carried.
    #[wasm_bindgen(getter = resourceId)]
    pub fn resource_id(&self) -> Option<String> {
        self.inner.resource_id().map(str::to_owned)
    }

    /// Workspace identity, when this is the complete workspace subject.
    #[wasm_bindgen(getter = workspaceId)]
    pub fn workspace_id(&self) -> Option<String> {
        self.inner.workspace_id().map(str::to_owned)
    }
}

/// Closed workspace-subject family.
#[wasm_bindgen(js_name = WorkspaceSubjectKind)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceSubjectKind {
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

/// Lossless provenance projection for one causal step.
#[wasm_bindgen(js_name = WorkspaceProvenance)]
pub struct WasmWorkspaceProvenance {
    inner: PortableWorkspaceProvenance,
}
#[wasm_bindgen(js_class = WorkspaceProvenance)]
impl WasmWorkspaceProvenance {
    /// Closed provenance family.
    pub fn kind(&self) -> WasmWorkspaceProvenanceKind {
        match self.inner {
            PortableWorkspaceProvenance::Source { .. } => WasmWorkspaceProvenanceKind::Source,
            PortableWorkspaceProvenance::ModuleRelease { .. } => {
                WasmWorkspaceProvenanceKind::ModuleRelease
            }
            PortableWorkspaceProvenance::Import { .. } => WasmWorkspaceProvenanceKind::Import,
            PortableWorkspaceProvenance::Extension { .. } => WasmWorkspaceProvenanceKind::Extension,
            PortableWorkspaceProvenance::Compilation => WasmWorkspaceProvenanceKind::Compilation,
        }
    }
    /// Direct source identity.
    #[wasm_bindgen(getter = sourceId)]
    pub fn source_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Source { source_id } => Some(source_id.clone()),
            _ => None,
        }
    }
    /// Module or extension owner identity.
    #[wasm_bindgen(getter = moduleId)]
    pub fn module_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::ModuleRelease { module_id, .. } => Some(module_id.clone()),
            PortableWorkspaceProvenance::Extension {
                owner_module_id, ..
            } => Some(owner_module_id.clone()),
            _ => None,
        }
    }
    /// Selected release identity.
    #[wasm_bindgen(getter = releaseId)]
    pub fn release_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::ModuleRelease { release_id, .. } => {
                Some(release_id.clone())
            }
            _ => None,
        }
    }
    /// Selected module digest.
    #[wasm_bindgen(getter = moduleDigest)]
    pub fn module_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::ModuleRelease { module_digest, .. } => {
                Some(module_digest.clone())
            }
            _ => None,
        }
    }
    /// Importer module identity.
    #[wasm_bindgen(getter = importerModuleId)]
    pub fn importer_module_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                importer_module_id, ..
            } => Some(importer_module_id.clone()),
            _ => None,
        }
    }
    /// Upstream module identity.
    #[wasm_bindgen(getter = upstreamModuleId)]
    pub fn upstream_module_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                upstream_module_id, ..
            } => Some(upstream_module_id.clone()),
            _ => None,
        }
    }
    /// Upstream release identity.
    #[wasm_bindgen(getter = upstreamReleaseId)]
    pub fn upstream_release_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                upstream_release_id,
                ..
            } => Some(upstream_release_id.clone()),
            _ => None,
        }
    }
    /// Selected export identity.
    #[wasm_bindgen(getter = exportId)]
    pub fn export_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Import { export_id, .. } => Some(export_id.clone()),
            _ => None,
        }
    }
    /// Upstream namespace identity.
    #[wasm_bindgen(getter = sourceNamespaceId)]
    pub fn source_namespace_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                source_namespace_id,
                ..
            } => Some(source_namespace_id.clone()),
            _ => None,
        }
    }
    /// Local namespace identity.
    #[wasm_bindgen(getter = localNamespaceId)]
    pub fn local_namespace_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Import {
                local_namespace_id, ..
            } => Some(local_namespace_id.clone()),
            _ => None,
        }
    }
    /// Extension namespace identity.
    #[wasm_bindgen(getter = extensionNamespaceId)]
    pub fn extension_namespace_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Extension {
                extension_namespace_id,
                ..
            } => Some(extension_namespace_id.clone()),
            _ => None,
        }
    }
    /// Extension target namespace identity.
    #[wasm_bindgen(getter = targetNamespaceId)]
    pub fn target_namespace_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Extension {
                target_namespace_id,
                ..
            } => Some(target_namespace_id.clone()),
            _ => None,
        }
    }
    /// Extension target declaration identity.
    #[wasm_bindgen(getter = targetDeclarationId)]
    pub fn target_declaration_id(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceProvenance::Extension {
                target_declaration_id,
                ..
            } => Some(target_declaration_id.clone()),
            _ => None,
        }
    }
}
/// Closed provenance family.
#[wasm_bindgen(js_name = WorkspaceProvenanceKind)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceProvenanceKind {
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

/// Lossless typed fact projection for a root or causal step.
#[wasm_bindgen(js_name = WorkspaceFact)]
pub struct WasmWorkspaceFact {
    inner: PortableWorkspaceFact,
}
#[wasm_bindgen(js_class = WorkspaceFact)]
impl WasmWorkspaceFact {
    /// Closed fact family.
    pub fn kind(&self) -> WasmWorkspaceFactKind {
        match self.inner {
            PortableWorkspaceFact::SourceContent { .. } => WasmWorkspaceFactKind::SourceContent,
            PortableWorkspaceFact::Component { .. } => WasmWorkspaceFactKind::Component,
            PortableWorkspaceFact::Module { .. } => WasmWorkspaceFactKind::Module,
            PortableWorkspaceFact::Declaration { .. } => WasmWorkspaceFactKind::Declaration,
            PortableWorkspaceFact::ImportSurface { .. } => WasmWorkspaceFactKind::ImportSurface,
            PortableWorkspaceFact::Extension { .. } => WasmWorkspaceFactKind::Extension,
            PortableWorkspaceFact::EffectivePolicy { .. } => WasmWorkspaceFactKind::EffectivePolicy,
            PortableWorkspaceFact::ResponsibilityClosure { .. } => {
                WasmWorkspaceFactKind::ResponsibilityClosure
            }
            PortableWorkspaceFact::OrganizationalTranslationClosure { .. } => {
                WasmWorkspaceFactKind::OrganizationalTranslationClosure
            }
            PortableWorkspaceFact::ExperienceComposition { .. } => {
                WasmWorkspaceFactKind::ExperienceComposition
            }
            PortableWorkspaceFact::Resource { .. } => WasmWorkspaceFactKind::Resource,
            PortableWorkspaceFact::ResolutionLock { .. } => WasmWorkspaceFactKind::ResolutionLock,
            PortableWorkspaceFact::UniverseLock { .. } => WasmWorkspaceFactKind::UniverseLock,
            PortableWorkspaceFact::SemanticIr { .. } => WasmWorkspaceFactKind::SemanticIr,
        }
    }
    /// Source path.
    #[wasm_bindgen(getter = sourcePath)]
    pub fn source_path(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::SourceContent { source_path, .. } => Some(source_path.clone()),
            _ => None,
        }
    }
    /// Source digest.
    #[wasm_bindgen(getter = sourceDigest)]
    pub fn source_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::SourceContent { source_digest, .. }
            | PortableWorkspaceFact::ExperienceComposition { source_digest } => {
                Some(source_digest.clone())
            }
            _ => None,
        }
    }
    /// Source byte length.
    #[wasm_bindgen(getter = byteLen)]
    pub fn byte_len(&self) -> Option<u32> {
        match self.inner {
            PortableWorkspaceFact::SourceContent { byte_len, .. } => Some(byte_len),
            _ => None,
        }
    }
    /// Component digest.
    #[wasm_bindgen(getter = componentDigest)]
    pub fn component_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::Component { component_digest } => Some(component_digest.clone()),
            _ => None,
        }
    }
    /// Module digest.
    #[wasm_bindgen(getter = moduleDigest)]
    pub fn module_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::Module { module_digest } => Some(module_digest.clone()),
            _ => None,
        }
    }
    /// Value digest.
    #[wasm_bindgen(getter = valueDigest)]
    pub fn value_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::Declaration { value_digest }
            | PortableWorkspaceFact::Extension { value_digest }
            | PortableWorkspaceFact::EffectivePolicy { value_digest }
            | PortableWorkspaceFact::Resource { value_digest } => Some(value_digest.clone()),
            _ => None,
        }
    }
    /// Import-surface digest.
    #[wasm_bindgen(getter = surfaceDigest)]
    pub fn surface_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::ImportSurface { surface_digest } => Some(surface_digest.clone()),
            _ => None,
        }
    }
    /// Closure digest.
    #[wasm_bindgen(getter = closureDigest)]
    pub fn closure_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::ResponsibilityClosure { closure_digest }
            | PortableWorkspaceFact::OrganizationalTranslationClosure { closure_digest } => {
                Some(closure_digest.clone())
            }
            _ => None,
        }
    }
    /// Resolution-lock digest.
    #[wasm_bindgen(getter = resolutionDigest)]
    pub fn resolution_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::ResolutionLock { resolution_digest } => {
                Some(resolution_digest.clone())
            }
            _ => None,
        }
    }
    /// Universe-lock digest.
    #[wasm_bindgen(getter = universeDigest)]
    pub fn universe_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::UniverseLock { universe_digest } => {
                Some(universe_digest.clone())
            }
            _ => None,
        }
    }
    /// Workspace semantic-IR digest.
    #[wasm_bindgen(getter = workspaceSemanticIrDigest)]
    pub fn workspace_semantic_ir_digest(&self) -> Option<String> {
        match &self.inner {
            PortableWorkspaceFact::SemanticIr {
                workspace_semantic_ir_digest,
            } => Some(workspace_semantic_ir_digest.clone()),
            _ => None,
        }
    }
}
/// Closed derivation-fact family.
#[wasm_bindgen(js_name = WorkspaceFactKind)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceFactKind {
    /// Source content.
    SourceContent,
    /// Component.
    Component,
    /// Module.
    Module,
    /// Declaration.
    Declaration,
    /// Import surface.
    ImportSurface,
    /// Extension.
    Extension,
    /// Effective policy.
    EffectivePolicy,
    /// Responsibility closure.
    ResponsibilityClosure,
    /// Organizational-translation closure.
    OrganizationalTranslationClosure,
    /// Experience composition.
    ExperienceComposition,
    /// Resource.
    Resource,
    /// Resolution lock.
    ResolutionLock,
    /// Universe lock.
    UniverseLock,
    /// Semantic IR.
    SemanticIr,
}

/// One exact source-change root from a verified comparison receipt.
#[wasm_bindgen(js_name = WorkspaceChangeRoot)]
pub struct WasmWorkspaceChangeRoot {
    inner: pale_sdk::PortableWorkspaceChangeRoot,
}

impl WasmWorkspaceChangeRoot {
    /// Retain the API-owned root projection.
    fn from_api(inner: pale_sdk::PortableWorkspaceChangeRoot) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = WorkspaceChangeRoot)]
impl WasmWorkspaceChangeRoot {
    /// Content address of the exact root.
    #[wasm_bindgen(getter = rootDigest)]
    pub fn root_digest(&self) -> String {
        self.inner.root_digest().to_owned()
    }

    /// Closed transition.
    pub fn kind(&self) -> WasmWorkspaceChangeKind {
        match self.inner.kind() {
            pale_sdk::PortableWorkspaceChangeKind::Added => WasmWorkspaceChangeKind::Added,
            pale_sdk::PortableWorkspaceChangeKind::Modified => WasmWorkspaceChangeKind::Modified,
            pale_sdk::PortableWorkspaceChangeKind::Removed => WasmWorkspaceChangeKind::Removed,
        }
    }

    /// Clone the typed subject for inspection or explanation.
    pub fn subject(&self) -> WasmWorkspaceSubject {
        WasmWorkspaceSubject::from_api(self.inner.subject())
    }
    /// Exact predecessor fact, absent only for an added root.
    #[wasm_bindgen(js_name = priorFact)]
    pub fn prior_fact(&self) -> Option<WasmWorkspaceFact> {
        self.inner
            .prior_fact()
            .cloned()
            .map(|inner| WasmWorkspaceFact { inner })
    }
    /// Exact current fact, absent only for a removed root.
    #[wasm_bindgen(js_name = currentFact)]
    pub fn current_fact(&self) -> Option<WasmWorkspaceFact> {
        self.inner
            .current_fact()
            .cloned()
            .map(|inner| WasmWorkspaceFact { inner })
    }
}
/// Closed source transition.
#[wasm_bindgen(js_name = WorkspaceChangeKind)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceChangeKind {
    /// Added fact.
    Added,
    /// Modified fact.
    Modified,
    /// Removed fact.
    Removed,
}

/// One bounded page of exact change roots.
#[wasm_bindgen(js_name = WorkspaceChangeRootPage)]
pub struct WasmWorkspaceChangeRootPage {
    canonical_bytes: Vec<u8>,
    entries: Vec<WasmWorkspaceChangeRoot>,
    next_offset: Option<u32>,
}

#[wasm_bindgen(js_class = WorkspaceChangeRootPage)]
impl WasmWorkspaceChangeRootPage {
    /// Copy the SDK's exact canonical page without reconstructing typed entries.
    #[wasm_bindgen(js_name = canonicalBytes)]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.canonical_bytes.clone()
    }

    /// Consume the page and return its owned entries in canonical order.
    #[wasm_bindgen(js_name = intoEntries)]
    pub fn into_entries(self) -> Vec<WasmWorkspaceChangeRoot> {
        self.entries
    }

    /// Next exact offset, or `undefined` at the end.
    #[wasm_bindgen(getter = nextOffset)]
    pub fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }
}

/// One current subject in the complete reachability partition.
#[wasm_bindgen(js_name = WorkspaceImpact)]
pub struct WasmWorkspaceImpact {
    inner: PortableWorkspaceImpact,
}

impl WasmWorkspaceImpact {
    /// Retain the API-owned impact projection.
    fn from_api(inner: PortableWorkspaceImpact) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = WorkspaceImpact)]
impl WasmWorkspaceImpact {
    /// Clone the typed subject for inspection or explanation.
    pub fn subject(&self) -> WasmWorkspaceSubject {
        WasmWorkspaceSubject::from_api(self.inner.subject())
    }

    /// Root-backed reachability classification.
    pub fn state(&self) -> WasmWorkspaceImpactState {
        match self.inner.state() {
            PortableWorkspaceImpactState::Affected => WasmWorkspaceImpactState::Affected,
            PortableWorkspaceImpactState::Unaffected => WasmWorkspaceImpactState::Unaffected,
        }
    }

    /// Exact count of roots whose closure reaches this subject.
    #[wasm_bindgen(getter = contributingRootCount)]
    pub fn contributing_root_count(&self) -> u32 {
        self.inner.contributing_root_count()
    }
}
/// Closed impact classification.
#[wasm_bindgen(js_name = WorkspaceImpactState)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceImpactState {
    /// Root-reachable.
    Affected,
    /// Outside every root closure.
    Unaffected,
}

/// One bounded page of the complete impact partition.
#[wasm_bindgen(js_name = WorkspaceImpactPage)]
pub struct WasmWorkspaceImpactPage {
    canonical_bytes: Vec<u8>,
    entries: Vec<WasmWorkspaceImpact>,
    next_offset: Option<u32>,
}

#[wasm_bindgen(js_class = WorkspaceImpactPage)]
impl WasmWorkspaceImpactPage {
    /// Copy the SDK's exact canonical page without reconstructing typed entries.
    #[wasm_bindgen(js_name = canonicalBytes)]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.canonical_bytes.clone()
    }

    /// Consume the page and return its owned entries in canonical order.
    #[wasm_bindgen(js_name = intoEntries)]
    pub fn into_entries(self) -> Vec<WasmWorkspaceImpact> {
        self.entries
    }

    /// Next exact offset, or `undefined` at the end.
    #[wasm_bindgen(getter = nextOffset)]
    pub fn next_offset(&self) -> Option<u32> {
        self.next_offset
    }
}

/// Deterministic causal explanation for one comparison subject.
#[wasm_bindgen(js_name = WorkspaceExplanation)]
pub struct WasmWorkspaceExplanation {
    inner: PortableWorkspaceExplanation,
}

impl WasmWorkspaceExplanation {
    /// Retain the API-owned causal result.
    fn from_api(inner: PortableWorkspaceExplanation) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = WorkspaceExplanation)]
impl WasmWorkspaceExplanation {
    /// Copy the SDK's exact canonical explanation, including its causal witness.
    #[wasm_bindgen(js_name = canonicalBytes)]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.inner.canonical_bytes().to_vec()
    }

    /// Exact query outcome.
    pub fn state(&self) -> WasmWorkspaceExplanationState {
        match self.inner.state() {
            PortableWorkspaceExplanationState::NotPresent => {
                WasmWorkspaceExplanationState::NotPresent
            }
            PortableWorkspaceExplanationState::Unaffected => {
                WasmWorkspaceExplanationState::Unaffected
            }
            PortableWorkspaceExplanationState::Affected => WasmWorkspaceExplanationState::Affected,
        }
    }

    /// Clone the typed query subject.
    pub fn subject(&self) -> WasmWorkspaceSubject {
        WasmWorkspaceSubject::from_api(self.inner.subject())
    }

    /// Deterministically selected root, when affected.
    pub fn root(&self) -> Option<WasmWorkspaceChangeRoot> {
        self.inner
            .causal()
            .map(|c| WasmWorkspaceChangeRoot::from_api(c.root().clone()))
    }
    /// Proof DAG frontier, when affected.
    pub fn frontier(&self) -> Option<WasmWorkspaceExplanationFrontier> {
        self.inner.causal().map(|c| match c.frontier() {
            PortableWorkspaceExplanationFrontier::Predecessor => {
                WasmWorkspaceExplanationFrontier::Predecessor
            }
            PortableWorkspaceExplanationFrontier::Current => {
                WasmWorkspaceExplanationFrontier::Current
            }
        })
    }
    /// Complete shortest typed proof sequence.
    pub fn steps(&self) -> Vec<WasmWorkspaceCausalStep> {
        self.inner
            .causal()
            .map(|c| {
                c.steps()
                    .iter()
                    .cloned()
                    .map(|inner| WasmWorkspaceCausalStep { inner })
                    .collect()
            })
            .unwrap_or_default()
    }
}
/// Closed explanation state.
#[wasm_bindgen(js_name = WorkspaceExplanationState)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceExplanationState {
    /// Subject absent.
    NotPresent,
    /// Present and unaffected.
    Unaffected,
    /// Present with a witness.
    Affected,
}
/// Proof DAG frontier.
#[wasm_bindgen(js_name = WorkspaceExplanationFrontier)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceExplanationFrontier {
    /// Predecessor DAG.
    Predecessor,
    /// Current DAG.
    Current,
}
/// Closed deterministic derivation rule.
#[wasm_bindgen(js_name = WorkspaceDerivationRule)]
#[derive(Clone, Copy)]
pub enum WasmWorkspaceDerivationRule {
    /// Source admission.
    SourceAdmission,
    /// Component admission.
    ComponentAdmission,
    /// Module sealing.
    ModuleSealing,
    /// Declaration linking.
    DeclarationLinking,
    /// Import linking.
    ImportLinking,
    /// Extension application.
    ExtensionApplication,
    /// Policy composition.
    PolicyComposition,
    /// Responsibility lowering.
    ResponsibilityLowering,
    /// Organizational translation.
    OrganizationalTranslation,
    /// Experience composition.
    ExperienceComposition,
    /// Resource selection.
    ResourceSelection,
    /// Resolution locking.
    ResolutionLocking,
    /// Universe locking.
    UniverseLocking,
    /// Semantic-IR sealing.
    SemanticIrSealing,
}
impl From<PortableWorkspaceDerivationRule> for WasmWorkspaceDerivationRule {
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
/// One complete typed fact on a shortest causal path.
#[wasm_bindgen(js_name = WorkspaceCausalStep)]
pub struct WasmWorkspaceCausalStep {
    inner: pale_sdk::PortableWorkspaceCausalStep,
}
#[wasm_bindgen(js_class = WorkspaceCausalStep)]
impl WasmWorkspaceCausalStep {
    /// Exact proof-node digest.
    #[wasm_bindgen(getter = nodeDigest)]
    pub fn node_digest(&self) -> String {
        self.inner.node_digest().to_owned()
    }
    /// Semantic subject.
    pub fn subject(&self) -> WasmWorkspaceSubject {
        WasmWorkspaceSubject::from_api(self.inner.subject())
    }
    /// Exact provenance.
    pub fn provenance(&self) -> WasmWorkspaceProvenance {
        WasmWorkspaceProvenance {
            inner: self.inner.provenance().clone(),
        }
    }
    /// Deterministic rule.
    pub fn rule(&self) -> WasmWorkspaceDerivationRule {
        self.inner.rule().into()
    }
    /// Exact produced fact.
    pub fn fact(&self) -> WasmWorkspaceFact {
        WasmWorkspaceFact {
            inner: self.inner.fact().clone(),
        }
    }
}
