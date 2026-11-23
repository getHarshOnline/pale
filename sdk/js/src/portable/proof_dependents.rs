// Node projection of the retained compiler's direct-consumer scan.

/// Opaque continuation bound to the exact compilation, full subject and limits.
///
/// Keeps its compilation alive until garbage collection. It cannot be rebuilt
/// from bytes or digest strings and grants no disclosure or execution authority.
#[napi(js_name = "PortableWorkspaceDependentCursor")]
pub struct PortableWorkspaceDependentCursor {
    inner: pale_sdk::PortableWorkspaceDependentCursor,
}

/// Selected fact and direct consumers found within one candidate scan budget.
///
/// Terminal and unselected results keep their compilation alive too. Child
/// getters copy existing values while the graph remains shared. Garbage
/// collection releases this handle's ownership without changing semantic state.
#[napi(js_name = "PortableWorkspaceDependentPage")]
pub struct PortableWorkspaceDependentPage {
    inner: pale_sdk::PortableWorkspaceDependentPage,
}

#[napi]
impl PortableWorkspaceDependentPage {
    /// Workspace identity from the exact producing compilation.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }

    /// Principal-universe identity from the same owner.
    #[napi(getter)]
    pub fn principal_universe_digest(&self) -> String {
        self.inner.principal_universe_digest().to_owned()
    }

    /// Semantic-IR digest, not a replacement for compilation custody.
    #[napi(getter)]
    pub fn semantic_ir_digest(&self) -> String {
        self.inner.semantic_ir_digest().to_owned()
    }

    /// Owner-admission evidence used by that compilation.
    #[napi(getter)]
    pub fn admission_evidence_digest(&self) -> String {
        self.inner.admission_evidence_digest().to_owned()
    }

    /// Copy the full subject whose direct consumers were requested.
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

    /// Copy consumers in compiler node-digest order.
    /// Each uses the root as a direct input under its own derivation rule;
    /// this does not claim transitive impact or runtime causation.
    #[napi]
    pub fn dependents(&self) -> Vec<WorkspaceCausalStep> {
        self.inner
            .dependents()
            .iter()
            .cloned()
            .map(|inner| WorkspaceCausalStep { inner })
            .collect()
    }

    /// Candidate nodes examined, including leaves and nonmatches.
    ///
    /// # Errors
    /// Returns a projection error if the count cannot fit the Node ABI.
    #[napi(getter)]
    pub fn inspected(&self) -> Result<u32> {
        u32::try_from(self.inner.inspected()).map_err(|_| {
            napi::Error::new(
                napi::Status::GenericFailure,
                "dependent inspected count exceeds the Node ABI",
            )
        })
    }

    /// Actual candidate input-search comparisons, excluding root lookup.
    /// The count can vary with the standard library; it is not semantic identity,
    /// generation membership, a time bound or causal evidence.
    ///
    /// # Errors
    /// Returns a projection error if the count cannot fit the Node ABI.
    #[napi(getter)]
    pub fn input_digest_comparisons(&self) -> Result<u32> {
        u32::try_from(self.inner.input_digest_comparisons()).map_err(|_| {
            napi::Error::new(
                napi::Status::GenericFailure,
                "dependent comparison count exceeds the Node ABI",
            )
        })
    }

    /// Whether the candidate scan is exhausted or the root is unselected.
    #[napi(getter)]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Retain the next candidate position, or return null after exhaustion.
    #[napi]
    pub fn continuation(&self) -> Option<PortableWorkspaceDependentCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| PortableWorkspaceDependentCursor { inner })
    }

    /// Check whether this result came from this same compilation, not equal digests.
    #[napi]
    pub fn belongs_to(&self, workspace: &PortableWorkspaceCompilation) -> bool {
        self.inner.belongs_to(&workspace.inner)
    }

    /// Copy the SDK's already-computed canonical projection without re-encoding.
    /// The bytes do not reconstruct the owner or continuation.
    #[napi(getter)]
    pub fn canonical_bytes(&self) -> Buffer {
        self.inner.canonical_bytes().to_vec().into()
    }
}

#[napi]
impl PortableWorkspaceCompilation {
    /// Find one-hop direct consumers without recompiling or building a reverse index.
    ///
    /// Each limit must be an integer from 0 through 65535. The SDK applies
    /// independent 64-row defaults and 256-row caps. Every candidate is charged,
    /// including nonmatches; empty pages can remain incomplete. Resume with this
    /// exact compilation, full subject and both raw limits. A missing cursor
    /// starts a new scan, not an implicit continuation of a completed result.
    /// Candidate and row limits combine with a fixed aggregate input-comparison
    /// ceiling. Reaching any ceiling leaves the next candidate in the continuation.
    /// These bounds do not limit bytes, elapsed time or retained memory. This
    /// supplied-input query admits no recipient or native-effect authority.
    ///
    /// # Errors
    /// Invalid numbers produce `InvalidArg`. Continuation and internal failures
    /// retain the SDK-owned canonical diagnostic.
    #[napi]
    pub fn direct_dependents(
        &self,
        env: Env,
        subject: &WorkspaceSubject,
        maximum_inspected: f64,
        maximum_returned: f64,
        after: Option<&PortableWorkspaceDependentCursor>,
    ) -> Result<PortableWorkspaceDependentPage> {
        self.inner
            .direct_dependents(
                &subject.inner,
                after.map(|cursor| &cursor.inner),
                authoring_page_limit(maximum_inspected)?,
                authoring_page_limit(maximum_returned)?,
            )
            .map(|inner| PortableWorkspaceDependentPage { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }
}
