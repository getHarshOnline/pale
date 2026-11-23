//! Node-native projection of Package distribution inspection.

use napi::bindgen_prelude::Buffer;
use napi::{Env, Result};
use napi_derive::napi;
use pale_sdk::{
    PackageDistributionInspection as ApiPackageDistributionInspection, PackageDistributionRequest,
};

use crate::error::diagnostic_error;
use crate::{CompilationDocument, Pale};

/// Exact immutable identity of one inspected Package release.
#[napi(object, object_from_js = false, js_name = "PackageReleaseIdentity")]
pub struct PackageReleaseIdentity {
    /// Canonical Package identity.
    pub id: String,
    /// Exact owner-published release label.
    pub version: String,
    /// Digest of the complete manifest and owned source inventory.
    pub content_digest: String,
}

/// Canonically ordered lock inputs derived by the API owner.
#[napi(
    object,
    object_from_js = false,
    js_name = "PackageDistributionInspection"
)]
pub struct PackageDistributionInspection {
    /// Compiler contract required by the inspected releases.
    pub compiler: String,
    /// Digest of the generated schema manifest used for admission.
    pub schema_manifest_sha256: String,
    /// Digest of the complete generated schema byte set used for admission.
    pub schema_set_sha256: String,
    /// Canonically ordered exact Package release identities.
    pub packages: Vec<PackageReleaseIdentity>,
    /// Canonical API-owned bytes for durable transport.
    pub canonical_bytes: Buffer,
}

impl PackageDistributionInspection {
    /// Project API-owned facts without parsing or re-canonicalizing them.
    pub(crate) fn from_api(value: ApiPackageDistributionInspection) -> Self {
        Self {
            compiler: value.compiler().to_owned(),
            schema_manifest_sha256: value.schema_manifest_sha256().to_owned(),
            schema_set_sha256: value.schema_set_sha256().to_owned(),
            packages: value
                .packages()
                .iter()
                .map(|release| PackageReleaseIdentity {
                    id: release.id().to_owned(),
                    version: release.version().to_owned(),
                    content_digest: release.content_digest().to_owned(),
                })
                .collect(),
            canonical_bytes: value.canonical_bytes().to_vec().into(),
        }
    }
}

#[napi]
impl Pale {
    /// Inspect captured Package release coordinates without reading a lock.
    ///
    /// # Errors
    ///
    /// Returns a native error with separate `diagnostic` bytes when logical
    /// labels, finite source bounds, manifests, or source inventories fail the
    /// API-owned Package admission path.
    #[napi]
    pub fn inspect_package_distribution(
        &self,
        env: Env,
        documents: Vec<CompilationDocument>,
    ) -> Result<PackageDistributionInspection> {
        self.api
            .inspect_package_distribution(PackageDistributionRequest::new(
                documents
                    .into_iter()
                    .map(CompilationDocument::into_api)
                    .collect(),
            ))
            .map(PackageDistributionInspection::from_api)
            .map_err(|error| diagnostic_error(&env, error))
    }
}
