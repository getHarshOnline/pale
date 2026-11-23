//! One seeded catalog shared by API admission laws and cross-leaf regeneration.

use pale_sdk::{NativeActionDescriptor, PaleApi};

/// Construct the catalog from current schema and descriptor owners.
pub(super) fn repository_catalog_fixture(
    api: &PaleApi,
) -> (
    pale_core::records::RepositoryNativeActionCatalogV1,
    Vec<NativeActionDescriptor>,
) {
    let descriptors = api.native_action_descriptors().expect("catalog projects");
    let contracts = descriptors
        .iter()
        .map(|descriptor| {
            let value = pale_core::canon::parse_strict(descriptor.canonical_contract_bytes())
                .expect("descriptor contract is strict JSON");
            serde_json::from_value(value).expect("descriptor contract decodes")
        })
        .collect();
    let identity = pale_contracts::record_schema_identity("repository-native-action-catalog.v1")
        .expect("repository action catalog identity is embedded");
    (
        pale_core::records::RepositoryNativeActionCatalogV1 {
            schema: identity.schema,
            schema_sha256: identity.schema_sha256,
            schema_version: 1,
            profile_digest: pale_core::records::ProfileDigest::parse("a".repeat(64)).unwrap(),
            instance_digest: pale_core::records::InstanceDigest::parse("b".repeat(64)).unwrap(),
            repository_id: pale_core::records::LogicalRepositoryIdV1::parse("pale").unwrap(),
            contracts,
        },
        descriptors,
    )
}
