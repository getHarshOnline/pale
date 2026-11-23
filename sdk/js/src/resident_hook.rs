//! Admit repository hook inputs through the Rust SDK; do not start a resident.

use napi::bindgen_prelude::Buffer;
use napi::{Env, Result};
use napi_derive::napi;
use pale_sdk::{
    ResidentHookAdmission as ApiResidentHookAdmission,
    ResidentHookRequest as ApiResidentHookRequest,
};

use crate::error::diagnostic_error;
use crate::Pale;

/// Complete native event awaiting API-owned resident-hook admission.
#[napi(object, object_to_js = false, js_name = "ResidentHookRequest")]
pub struct ResidentHookRequest {
    /// Static lifecycle-event identity.
    pub event: String,
    /// Logical workspace identity.
    pub workspace: String,
    /// Logical repository identity.
    pub repository: String,
    /// Stable provider-binding locator.
    pub provider_binding: String,
    /// Domain-separated identity of the complete provider-binding body.
    pub provider_binding_identity: String,
    /// Complete bounded native event payload.
    pub payload: Buffer,
}

/// Opaque proof that static routing and native payload shape agree.
///
/// The handle grants no carrier, active generation, session, policy, or effect
/// authority. Dropping it has no external consequence.
#[napi(js_name = "ResidentHookAdmission")]
pub struct ResidentHookAdmission {
    inner: ApiResidentHookAdmission,
}

#[napi]
impl Pale {
    /// Admit one repository-hook request without selecting or invoking a carrier.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when routing,
    /// identity, lifecycle-event, provider-binding, or payload admission fails.
    #[napi]
    pub fn admit_resident_hook(
        &self,
        env: Env,
        request: ResidentHookRequest,
    ) -> Result<ResidentHookAdmission> {
        self.api
            .admit_resident_hook(ApiResidentHookRequest::new(
                request.event,
                request.workspace,
                request.repository,
                request.provider_binding,
                request.provider_binding_identity,
                request.payload.to_vec(),
            ))
            .map(|inner| ResidentHookAdmission { inner })
            .map_err(|error| diagnostic_error(&env, error))
    }

    /// Reject an admitted hook explicitly when no authenticated carrier exists.
    ///
    /// This is the API-owned bootstrap non-effect projection. It never attempts
    /// IPC, falls back to one-shot evaluation, or infers resident availability.
    ///
    /// # Errors
    ///
    /// Always returns the canonical unavailable-carrier diagnostic as a native
    /// error with separate `diagnostic` bytes.
    #[napi]
    pub fn resident_hook_transport_unavailable(
        &self,
        env: Env,
        admission: &ResidentHookAdmission,
    ) -> Result<()> {
        Err(diagnostic_error(
            &env,
            self.api
                .resident_hook_transport_unavailable(&admission.inner),
        ))
    }
}
