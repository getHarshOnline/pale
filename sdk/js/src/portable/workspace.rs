// Node workspace-source projection fragment included by the shallow façade.

/// One logical workspace document captured in Node-owned memory.
#[napi(object, object_to_js = false, js_name = "PortableWorkspaceDocument")]
pub struct PortableWorkspaceDocument {
    /// Logical relative label with no host-path authority.
    pub label: String,
    /// Complete captured document bytes.
    pub bytes: Buffer,
}

/// Batched request for portable workspace compilation.
#[napi(object, object_to_js = false, js_name = "PortableWorkspaceRequest")]
pub struct PortableWorkspaceRequest {
    /// Logical label of the workspace root manifest.
    pub root_manifest: String,
    /// Complete captured document batch.
    pub documents: Vec<PortableWorkspaceDocument>,
}

impl PortableWorkspaceRequest {
    /// Consume Node-owned values into the API request owner.
    pub(crate) fn into_api(self) -> ApiPortableWorkspaceRequest {
        ApiPortableWorkspaceRequest::new(
            self.root_manifest,
            self.documents
                .into_iter()
                .map(|document| {
                    ApiPortableWorkspaceDocument::new(document.label, document.bytes.to_vec())
                })
                .collect(),
        )
    }
}

/// Opaque admitted workspace; garbage collection has no semantic effect.
#[napi(js_name = "PortableWorkspaceCompilation")]
pub struct PortableWorkspaceCompilation {
    pub(crate) inner: ApiPortableWorkspaceCompilation,
}

#[napi]
impl PortableWorkspaceCompilation {
    /// Admitted workspace identity.
    #[napi(getter)]
    pub fn workspace_id(&self) -> String {
        self.inner.workspace_id().to_owned()
    }

    /// Principal-relative admitted-universe digest.
    #[napi(getter)]
    pub fn principal_universe_digest(&self) -> String {
        self.inner.principal_universe_digest().to_owned()
    }

    /// Verified semantic-IR digest.
    #[napi(getter)]
    pub fn semantic_ir_digest(&self) -> String {
        self.inner.semantic_ir_digest().to_owned()
    }

    /// Exact owner-proof frontier digest.
    #[napi(getter)]
    pub fn admission_evidence_digest(&self) -> String {
        self.inner.admission_evidence_digest().to_owned()
    }

    /// Number of admitted logical source documents.
    #[napi(getter)]
    pub fn source_document_count(&self) -> u32 {
        self.inner.source_document_count()
    }
    /// Number of source nodes in the derivation graph.
    #[napi(getter)]
    pub fn derivation_source_count(&self) -> u32 {
        self.inner.derivation_source_count()
    }
    /// Number of nodes in the derivation graph.
    #[napi(getter)]
    pub fn derivation_node_count(&self) -> u32 {
        self.inner.derivation_node_count()
    }
    /// Number of admission-evidence records.
    #[napi(getter)]
    pub fn admission_evidence_count(&self) -> u32 {
        self.inner.admission_evidence_count()
    }
    /// Number of selected adopted releases.
    #[napi(getter)]
    pub fn adopted_release_count(&self) -> u32 {
        self.inner.adopted_release_count()
    }
    /// Number of selected Package releases.
    #[napi(getter)]
    pub fn package_release_count(&self) -> u32 {
        self.inner.package_release_count()
    }
    /// Number of selected normative Pack releases.
    #[napi(getter)]
    pub fn normative_pack_release_count(&self) -> u32 {
        self.inner.normative_pack_release_count()
    }
}
