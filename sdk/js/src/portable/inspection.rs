// Node subject-inspection fragment included by the shallow portable façade.

/// Opaque continuation over this exact compilation, full subject and raw limits.
///
/// Keeps its compiled workspace alive until garbage collection. It cannot be
/// reconstructed from canonical bytes and grants no disclosure or execution authority.
#[napi(js_name = "PortableWorkspaceSubjectInspectionCursor")]
pub struct PortableWorkspaceSubjectInspectionCursor {
    pub(crate) inner: pale_sdk::PortableWorkspaceSubjectInspectionCursor,
}

/// Owned fact and source evidence from one retained supplied-input compilation.
///
/// Terminal and unselected results keep that compilation alive too. Child getters
/// copy values; the graph remains shared. Garbage collection releases this handle's
/// ownership, without changing semantic state or authorizing source disclosure.
#[napi(js_name = "PortableWorkspaceSubjectInspection")]
pub struct PortableWorkspaceSubjectInspection {
    pub(crate) inner: pale_sdk::PortableWorkspaceSubjectInspection,
}

#[napi]
impl PortableWorkspaceSubjectInspection {
    /// Workspace identity from this result's exact owner.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }

    /// Principal-universe identity from the same owner.
    #[napi(getter)]
    pub fn principal_universe_digest(&self) -> String {
        self.inner.principal_universe_digest().to_owned()
    }

    /// Semantic-IR digest, not a substitute for exact compilation custody.
    #[napi(getter)]
    pub fn semantic_ir_digest(&self) -> String {
        self.inner.semantic_ir_digest().to_owned()
    }

    /// Owner-admission evidence identity used by the compilation.
    #[napi(getter)]
    pub fn admission_evidence_digest(&self) -> String {
        self.inner.admission_evidence_digest().to_owned()
    }

    /// Copy the full subject originally supplied to this inspection.
    #[napi]
    pub fn subject(&self) -> WorkspaceSubject {
        WorkspaceSubject::from_api(self.inner.subject())
    }

    /// Copy the selected root, or return null for an unselected subject.
    #[napi]
    pub fn root(&self) -> Option<WorkspaceCausalStep> {
        self.inner
            .root()
            .cloned()
            .map(|inner| WorkspaceCausalStep { inner })
    }

    /// Copy this page's direct inputs in compiler proof-digest order.
    #[napi]
    pub fn dependencies(&self) -> Vec<WorkspaceCausalStep> {
        self.inner
            .dependencies()
            .iter()
            .cloned()
            .map(|inner| WorkspaceCausalStep { inner })
            .collect()
    }

    /// Copy direct named source matches with their compiler-assigned roles.
    ///
    /// # Errors
    /// Returns a projection error if a source index cannot fit the Node ABI.
    #[napi]
    pub fn occurrences(&self) -> Result<Vec<AuthoringOccurrence>> {
        self.inner
            .occurrences()
            .iter()
            .map(AuthoringOccurrence::from_api)
            .collect()
    }

    /// Source candidates examined this call, including nonmatches.
    #[napi(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }

    /// UTF-8 bytes copied into direct dependency strings on this page.
    #[napi(getter)]
    pub fn dependency_copied_string_bytes(&self) -> u32 {
        self.inner.dependency_copied_string_bytes()
    }

    /// UTF-8 bytes copied into occurrence and repeated record-anchor locations.
    #[napi(getter)]
    pub fn source_copied_string_bytes(&self) -> u32 {
        self.inner.source_copied_string_bytes()
    }

    /// Copy an owning import-path or extension record location, when available.
    /// Null does not establish subject absence; named references remain separate.
    ///
    /// # Errors
    /// Returns a projection error if a source index cannot fit the Node ABI.
    #[napi(getter)]
    pub fn record_anchor(&self) -> Result<Option<AuthoringSourceLocation>> {
        self.inner
            .record_anchor()
            .map(AuthoringSourceLocation::from_api)
            .transpose()
    }

    /// Whether direct input enumeration is complete independently of source scan.
    #[napi(getter)]
    pub fn inputs_complete(&self) -> bool {
        self.inner.inputs_complete()
    }

    /// Whether source scan exhaustion has been proved independently of inputs.
    #[napi(getter)]
    pub fn sources_complete(&self) -> bool {
        self.inner.sources_complete()
    }

    /// Whether both independently bounded sections are complete.
    #[napi(getter)]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Retain a continuation when either section remains unfinished; otherwise null.
    #[napi]
    pub fn continuation(&self) -> Option<PortableWorkspaceSubjectInspectionCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| PortableWorkspaceSubjectInspectionCursor { inner })
    }

    /// Check whether this result came from this same compilation, not merely an equal digest.
    #[napi]
    pub fn belongs_to(&self, workspace: &PortableWorkspaceCompilation) -> bool {
        self.inner.belongs_to(&workspace.inner)
    }

    /// Copy the SDK's already-computed canonical projection without re-encoding.
    /// The bytes do not encode or reconstruct the owner or continuation.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}

#[napi]
impl PortableWorkspaceCompilation {
    /// Inspect one subject's fact, direct inputs and source evidence without recompiling.
    ///
    /// All limits must be integers from 0 through 65535. The SDK applies its
    /// 64-row defaults and 256-row caps once. Resume with this exact compilation,
    /// full subject and all three raw limits. Empty pages can remain unfinished;
    /// completed sections do not restart. Row limits do not bound bytes, time or
    /// aggregate retained memory. This supplied-input query grants no recipient
    /// authority and performs no source-disclosure admission.
    ///
    /// # Errors
    /// Invalid numbers produce `InvalidArg`. Continuation mismatches and internal
    /// projection failures retain the SDK-owned canonical diagnostic.
    #[napi]
    pub fn inspect_subject(
        &self,
        env: Env,
        subject: &WorkspaceSubject,
        input_limit: f64,
        maximum_inspected: f64,
        maximum_returned: f64,
        after: Option<&PortableWorkspaceSubjectInspectionCursor>,
    ) -> Result<PortableWorkspaceSubjectInspection> {
        self.inner
            .inspect_subject(
                &subject.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(input_limit)?,
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
            )
            .map(|inner| PortableWorkspaceSubjectInspection { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}
