//! Private-workspace input records.
//!
//! Absolute paths appear only in machine bindings; portable records carry
//! logical ids.

mod capabilities;
mod distribution;
mod evidence;
mod machine;
mod session;
mod workspace;

/// Schema-only key grammar for private binding identifiers.
pub(super) struct BindingIdSchema;

impl schemars::JsonSchema for BindingIdSchema {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "BindingId".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::BindingIdSchema").into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::Schema::try_from(serde_json::json!({
            "type": "string",
            "pattern": "^[a-z][a-z0-9_-]*$"
        }))
        .expect("the static binding identity schema is valid")
    }
}

/// Schema-only grammar for portable capability identities used as map keys.
pub(super) struct CapabilityIdSchema;

impl schemars::JsonSchema for CapabilityIdSchema {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "CapabilityId".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::CapabilityIdSchema").into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::Schema::try_from(serde_json::json!({
            "type": "string",
            "pattern": "^[a-z][a-z0-9_-]*(?:\\.[a-z][a-z0-9_-]*)*$"
        }))
        .expect("the static capability identity schema is valid")
    }
}

/// Schema-only unique list of nonempty command strings.
pub(super) struct NonEmptyCommandListSchema;

impl schemars::JsonSchema for NonEmptyCommandListSchema {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "NonEmptyCommandList".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::NonEmptyCommandListSchema").into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::Schema::try_from(serde_json::json!({
            "type": "array",
            "uniqueItems": true,
            "items": {
                "type": "string",
                "minLength": 1
            }
        }))
        .expect("the static command-list schema is valid")
    }
}

pub use capabilities::{BoundaryCapability, CapabilitiesFile, RepositoryCommandCapabilities};
pub use distribution::{DistributionLockFile, DistributionMeta};
pub use evidence::{
    DurableEvidenceBinding, EvidenceRetentionSeconds, MaterializationLineageBinding,
    RouteRecordLimit,
};
pub use machine::{
    MachineBindingFile, MachineMeta, NativeHarnessBinding, ProviderRepositoryProjectionV1,
    RepositoryCheckoutBindingV1, SurfaceExecutableBinding, UserPreferences,
};
pub use session::{
    AuthorityAccess, AuthorityAccessClass, ReadingOrderEntry, SessionModel,
    SessionRuntimeSelection, SessionStateBinding, SessionTtlSeconds,
};
pub use workspace::{
    WorkspaceManifestFile, WorkspaceMeta, MAX_EXPERIENCE_COMPOSITION_SOURCES,
    MAX_ORGANIZATIONAL_TRANSLATION_SOURCES, MAX_RESPONSIBILITY_DERIVATION_SOURCES,
};
