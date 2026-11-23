// Node native-action projection fragment included by the shallow façade.

/// One API-owned native-action descriptor.
#[napi(js_name = "NativeActionDescriptor")]
pub struct NativeActionDescriptor {
    inner: ApiNativeActionDescriptor,
}

impl NativeActionDescriptor {
    /// Retain one registry-owned descriptor.
    pub(crate) fn from_api(inner: ApiNativeActionDescriptor) -> Self {
        Self { inner }
    }

    /// Clone the immutable API descriptor for a repeated host batch.
    pub(crate) fn clone_api(&self) -> ApiNativeActionDescriptor {
        self.inner.clone()
    }
}

#[napi]
impl NativeActionDescriptor {
    /// Exact compiled action identity.
    #[napi(getter)]
    pub fn action_id(&self) -> String {
        self.inner.action_id().to_owned()
    }

    /// Registry-owned concise description.
    #[napi(getter)]
    pub fn description(&self) -> String {
        self.inner.description().to_owned()
    }

    /// Exact canonical action contract bytes.
    #[napi(getter)]
    pub fn contract_bytes(&self) -> Buffer {
        self.inner.canonical_contract_bytes().to_vec().into()
    }

    /// Self-contained payload schema bytes.
    #[napi(getter)]
    pub fn payload_schema_bytes(&self) -> Buffer {
        self.inner.canonical_payload_schema_bytes().to_vec().into()
    }
}
