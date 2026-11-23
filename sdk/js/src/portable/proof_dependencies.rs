// Node projection of the compiler's direct proof inputs.

/// Opaque continuation for one compilation, full subject and raw limit.
///
/// Keeps its compilation alive until garbage collection. It cannot be rebuilt
/// from canonical bytes or digests and grants no disclosure or execution authority.
#[napi(js_name = "PortableWorkspaceDependencyCursor")]
pub struct PortableWorkspaceDependencyCursor {
    inner: pale_sdk::PortableWorkspaceDependencyCursor,
}

/// Copied selected fact and one page of its direct proof inputs.
///
/// A pending continuation retains the compilation. Terminal results contain
/// owned copies without retaining its graph. Both remain readable independently
/// of the caller's compilation variable; neither confers authority.
#[napi(js_name = "PortableWorkspaceDependencyPage")]
pub struct PortableWorkspaceDependencyPage {
    inner: pale_sdk::PortableWorkspaceDependencyPage,
}

#[napi]
impl PortableWorkspaceDependencyPage {
    /// Principal-universe identity of the queried compilation.
    #[napi(getter)]
    pub fn principal_universe_digest(&self) -> String {
        self.inner.principal_universe_digest().to_owned()
    }

    /// Semantic-IR identity, not a substitute for continuation custody.
    #[napi(getter)]
    pub fn semantic_ir_digest(&self) -> String {
        self.inner.semantic_ir_digest().to_owned()
    }

    /// Copy the complete selected subject.
    #[napi]
    pub fn subject(&self) -> WorkspaceSubject {
        WorkspaceSubject::from_api(self.inner.subject())
    }

    /// Copy the selected fact, or return null if the subject is unselected.
    #[napi]
    pub fn root(&self) -> Option<WorkspaceCausalStep> {
        self.inner
            .root()
            .cloned()
            .map(|inner| WorkspaceCausalStep { inner })
    }

    /// Copy direct input facts in compiler proof-digest order.
    /// Each input contributes to this root under the root's derivation rule;
    /// this is not a transitive closure or a provider-readiness decision.
    #[napi]
    pub fn dependencies(&self) -> Vec<WorkspaceCausalStep> {
        self.inner
            .dependencies()
            .iter()
            .cloned()
            .map(|inner| WorkspaceCausalStep { inner })
            .collect()
    }

    /// UTF-8 bytes copied into the dependency-step string fields.
    #[napi(getter)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes() as u32
    }

    /// Whether this input list is exhausted or the subject is unselected.
    #[napi(getter)]
    pub fn is_complete(&self) -> bool {
        self.inner.continuation().is_none()
    }

    /// Retain the next exact input position, or return null after exhaustion.
    #[napi]
    pub fn continuation(&self) -> Option<PortableWorkspaceDependencyCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| PortableWorkspaceDependencyCursor { inner })
    }

    /// Copy the SDK's canonical projection without re-encoding or cursor bytes.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}

#[napi]
impl PortableWorkspaceCompilation {
    /// Page one selected fact's direct inputs without recompilation or a graph walk.
    ///
    /// The limit must be an integer from 0 through 65535. The SDK owns the
    /// 64-input default and 256-input cap. Resume with the same compilation,
    /// full subject and raw limit; equal digests or equal normalized limits
    /// cannot replace that custody. A missing cursor starts a new query.
    /// A selected fact with no inputs differs from a missing root. The row limit
    /// and fixed copied-string byte ceiling do not bound time or total memory,
    /// or admit a recipient.
    ///
    /// # Errors
    /// Invalid numbers produce `InvalidArg`. Continuation and internal failures
    /// retain the SDK-owned canonical diagnostic.
    #[napi]
    pub fn direct_dependencies(
        &self,
        env: Env,
        subject: &WorkspaceSubject,
        limit: f64,
        after: Option<&PortableWorkspaceDependencyCursor>,
    ) -> Result<PortableWorkspaceDependencyPage> {
        self.inner
            .direct_dependencies(
                &subject.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(limit)?,
            )
            .map(|inner| PortableWorkspaceDependencyPage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}
