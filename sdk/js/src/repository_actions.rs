//! Repository-bound native-action catalog admission through the Rust SDK.

use napi::bindgen_prelude::Buffer;
use napi::{Env, Result};
use napi_derive::napi;
use pale_sdk::RepositoryNativeActionCatalogRequest;

use crate::error::diagnostic_error;
use crate::{NativeActionDescriptor, Pale};

/// Repository-bound admitted action selection.
#[napi(js_name = "AdmittedRepositoryActions")]
pub struct AdmittedRepositoryActions {
    profile_digest: String,
    instance_digest: String,
    repository_id: String,
    descriptors: Vec<NativeActionDescriptor>,
}

#[napi]
impl AdmittedRepositoryActions {
    /// Exact profile digest carried by the admitted catalog.
    #[napi(getter)]
    pub fn profile_digest(&self) -> String {
        self.profile_digest.clone()
    }

    /// Exact instance digest carried by the admitted catalog.
    #[napi(getter)]
    pub fn instance_digest(&self) -> String {
        self.instance_digest.clone()
    }

    /// Exact repository identity carried by the admitted catalog.
    #[napi(getter)]
    pub fn repository_id(&self) -> String {
        self.repository_id.clone()
    }

    /// Clone the stable selected descriptors as one batch.
    #[napi]
    pub fn descriptors(&self) -> Vec<NativeActionDescriptor> {
        self.descriptors
            .iter()
            .map(|descriptor| NativeActionDescriptor::from_api(descriptor.clone_api()))
            .collect()
    }
}

#[napi]
impl Pale {
    /// Admit one repository-bound pure-action catalog.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when catalog,
    /// context, ordering, or registry parity fails admission.
    #[napi]
    pub fn admit_repository_native_actions(
        &self,
        env: Env,
        bytes: Buffer,
        profile_digest: String,
        instance_digest: String,
        repository_id: String,
    ) -> Result<AdmittedRepositoryActions> {
        self.api
            .admit_repository_native_actions(RepositoryNativeActionCatalogRequest::new(
                bytes.to_vec(),
                profile_digest,
                instance_digest,
                repository_id,
            ))
            .map(|admitted| AdmittedRepositoryActions {
                profile_digest: admitted.profile_digest().to_owned(),
                instance_digest: admitted.instance_digest().to_owned(),
                repository_id: admitted.repository_id().to_owned(),
                descriptors: admitted
                    .into_descriptors()
                    .into_iter()
                    .map(NativeActionDescriptor::from_api)
                    .collect(),
            })
            .map_err(|error| diagnostic_error(&env, error))
    }
}
