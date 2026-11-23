// Python workspace-compilation projection fragment included by the shallow façade.

/// Opaque admitted portable workspace.
#[pyclass(name = "PortableWorkspaceCompilation", frozen)]
pub struct PyPortableWorkspaceCompilation {
    pub(crate) inner: Arc<PortableWorkspaceCompilation>,
}

#[pymethods]
impl PyPortableWorkspaceCompilation {
    /// Workspace identity declared by the admitted root manifest.
    #[getter]
    pub fn workspace_id(&self) -> &str {
        self.inner.workspace_id()
    }

    /// Principal-relative admitted-universe digest.
    #[getter]
    pub fn principal_universe_digest(&self) -> &str {
        self.inner.principal_universe_digest()
    }

    /// Verified semantic-IR digest.
    #[getter]
    pub fn semantic_ir_digest(&self) -> &str {
        self.inner.semantic_ir_digest()
    }

    /// Exact owner-proof frontier digest.
    #[getter]
    pub fn admission_evidence_digest(&self) -> &str {
        self.inner.admission_evidence_digest()
    }

    /// Number of captured source documents consumed at ingress.
    #[getter]
    pub fn source_document_count(&self) -> u32 {
        self.inner.source_document_count()
    }

    /// Number of retained causal source coordinates.
    #[getter]
    pub fn derivation_source_count(&self) -> u32 {
        self.inner.derivation_source_count()
    }

    /// Number of retained causal derivation nodes.
    #[getter]
    pub fn derivation_node_count(&self) -> u32 {
        self.inner.derivation_node_count()
    }

    /// Number of exact owner-proof records consumed at admission.
    #[getter]
    pub fn admission_evidence_count(&self) -> u32 {
        self.inner.admission_evidence_count()
    }

    /// Number of adopted module releases in the closure.
    #[getter]
    pub fn adopted_release_count(&self) -> u32 {
        self.inner.adopted_release_count()
    }

    /// Number of selected Package releases.
    #[getter]
    pub fn package_release_count(&self) -> u32 {
        self.inner.package_release_count()
    }

    /// Number of selected attestor-scoped normative Pack releases.
    #[getter]
    pub fn normative_pack_release_count(&self) -> u32 {
        self.inner.normative_pack_release_count()
    }
}
