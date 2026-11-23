// Node façade-method fragment included by the shallow projection owner.
#[napi]
impl Pale {
    /// Inspect the complete embedded public-contract bundle coordinate.
    #[napi]
    pub fn contract_bundle_identity(&self) -> ContractBundleIdentity {
        let identity = self.api.contract_bundle_identity();
        ContractBundleIdentity {
            schema_manifest_sha256: identity.schema_manifest_sha256().to_owned(),
            schema_set_sha256: identity.schema_set_sha256().to_owned(),
            member_count: identity.member_count(),
        }
    }

    /// Look up one façade-selected generated schema.
    ///
    /// # Errors
    ///
    /// Returns a native error with the canonical summary and separate
    /// `diagnostic` bytes when the identity is not selected.
    #[napi]
    pub fn contract_schema(&self, env: Env, schema_id: String) -> Result<ContractSchema> {
        self.api
            .contract_schema(&schema_id)
            .map(|schema| ContractSchema {
                registry_id: schema.registry_id().to_owned(),
                schema_uri: schema.schema_uri().to_owned(),
                wire_schema_id: schema.wire_schema_id().map(str::to_owned),
                schema_sha256: schema.schema_sha256().to_owned(),
                bytes: schema.bytes().to_vec().into(),
            })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Validate and canonicalize one selected contract document.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the schema
    /// identity or document fails API-owned contract admission.
    #[napi]
    pub fn validate_contract_document(
        &self,
        env: Env,
        schema_id: String,
        bytes: Buffer,
    ) -> Result<ValidatedContractDocument> {
        self.api
            .validate_contract_document(ContractDocument::new(schema_id, bytes.to_vec()))
            .map(|document| ValidatedContractDocument {
                schema_id: document.schema_id().to_owned(),
                schema_sha256: document.schema_sha256().to_owned(),
                canonical_bytes: document.canonical_bytes().to_vec().into(),
            })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Project a typed owner classification into canonical `diagnostic.v1` bytes.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the
    /// projection cannot be admitted or canonically encoded.
    #[napi]
    pub fn project_diagnostic(
        &self,
        env: Env,
        projection: DiagnosticProjection,
    ) -> Result<Buffer> {
        self.api
            .project_diagnostic(projection.into_api())
            .map(|diagnostic| diagnostic.canonical_bytes().to_vec().into())
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Admit canonical `diagnostic.v1` bytes and return the same bytes.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the input
    /// is not the canonical initial-v1 diagnostic contract.
    #[napi]
    pub fn admit_diagnostic(&self, env: Env, bytes: Buffer) -> Result<Buffer> {
        self.api
            .admit_diagnostic(&bytes)
            .map(|diagnostic| diagnostic.canonical_bytes().to_vec().into())
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Compile one path-free workspace source batch.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when source or
    /// workspace meaning fails API-owned admission.
    #[napi]
    pub fn compile_portable_workspace(
        &self,
        env: Env,
        request: PortableWorkspaceRequest,
    ) -> Result<PortableWorkspaceCompilation> {
        self.api
            .compile_portable_workspace(request.into_api())
            .map(|inner| PortableWorkspaceCompilation { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Compare two admitted portable workspace handles.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the
    /// workspaces cannot form one valid comparison coordinate.
    #[napi]
    pub fn compare_portable_workspaces(
        &self,
        env: Env,
        prior: &PortableWorkspaceCompilation,
        current: &PortableWorkspaceCompilation,
    ) -> Result<WorkspaceComparison> {
        self.api
            .compare_portable_workspaces(&prior.inner, &current.inner)
            .map(|inner| WorkspaceComparison { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Return the complete effect-free native-action descriptor batch.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes if the compiled
    /// registry contradicts its generated action contracts.
    #[napi]
    pub fn native_action_catalog(&self, env: Env) -> Result<Vec<NativeActionDescriptor>> {
        self.api
            .native_action_descriptors()
            .map(|descriptors| {
                descriptors
                    .into_iter()
                    .map(NativeActionDescriptor::from_api)
                    .collect()
            })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Execute one complete tagged effect-free native action.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the tagged
    /// request fails admission or pure execution.
    #[napi]
    pub fn invoke_native_action(
        &self,
        env: Env,
        action_id: String,
        input: Buffer,
    ) -> Result<Buffer> {
        self.api
            .invoke_native_action(NativeActionRequest::new(action_id, input.to_vec()))
            .map(|execution| execution.canonical_bytes().to_vec().into())
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Execute one carrier-selected effect-free native-action payload.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when the action,
    /// payload, or pure execution fails API-owned admission.
    #[napi]
    pub fn invoke_native_action_payload(
        &self,
        env: Env,
        action_id: String,
        payload: Buffer,
    ) -> Result<Buffer> {
        self.api
            .invoke_native_action_payload(NativeActionPayloadRequest::new(
                action_id,
                payload.to_vec(),
            ))
            .map(|execution| execution.canonical_bytes().to_vec().into())
            .map_err(|error| diagnostic_error(&env, error))
    }
}
