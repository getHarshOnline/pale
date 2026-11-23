//! Embedded storage manifest and ordered migration-source verification.

use pale_core::sha256_hex;
use rusqlite::Connection;
use serde::Deserialize;

use crate::schema::resultant_schema_sha256;
use crate::{
    AuthorityJournalProfileV1, BackupRestorePolicyV1, ControlSqliteError,
    ControlStorageProfileRequirement, SqliteProfileV1,
};

/// Human-reviewable storage manifest embedded into the adapter binary.
///
/// [`verify_embedded_storage_sources`] derives and verifies the canonical JCS
/// identity, so repository whitespace is not part of the durable coordinate.
const MANIFEST_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/manifests/pale-control-storage.v1.json"
));
/// Exact initial-v1 principal-trust DDL whose digest is bound by [`MANIFEST_BYTES`].
const PRINCIPAL_TRUST_MIGRATION_V1: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/principal-trust/0001_initial_v1.sql"
));
/// Exact initial-v1 deployment-control DDL whose digest is bound by [`MANIFEST_BYTES`].
const DEPLOYMENT_CONTROL_MIGRATION_V1: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/deployment-control/0001_initial_v1.sql"
));

/// Only manifest identity understood by this initial-v1 adapter.
const MANIFEST_ID: &str = "pale-control-storage.v1";
/// Only storage-manifest version understood by this initial-v1 adapter.
const MANIFEST_VERSION: u32 = 1;
include!("storage_manifest_pin.rs");
/// Physical control-store class selected by one source-owned migration set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlStoreKind {
    /// Principal-scoped trust and allocation store.
    PrincipalTrust,
    /// One immutable-deployment-bound operational control store.
    DeploymentControl,
}

impl ControlStoreKind {
    /// Stable adapter-private manifest and database spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PrincipalTrust => "principal_trust",
            Self::DeploymentControl => "deployment_control",
        }
    }
}

/// One digest-verified migration in a store's ordered source set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedMigrationSource {
    id: String,
    ordinal: u32,
    path: String,
    sha256: String,
    sql: &'static str,
}

impl VerifiedMigrationSource {
    /// Stable migration identity.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// One-based position in its store's immutable migration set.
    #[must_use]
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    /// Repository-relative source path bound by the manifest.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Lowercase SHA-256 of the exact SQL source bytes.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Borrow the exact embedded SQL after its digest has been verified.
    #[cfg(any(test, feature = "bench"))]
    pub(crate) const fn sql(&self) -> &'static str {
        self.sql
    }
}

/// Verified source contract for exactly one control-store kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedStoreSource {
    kind: ControlStoreKind,
    application_id: i32,
    schema_id: String,
    schema_sha256: String,
    target_schema_version: u32,
    migrations: [VerifiedMigrationSource; 1],
}

impl VerifiedStoreSource {
    /// Store kind governed by this source set.
    #[must_use]
    pub const fn kind(&self) -> ControlStoreKind {
        self.kind
    }

    /// `SQLite` application ID that prevents cross-kind opening.
    #[must_use]
    pub const fn application_id(&self) -> i32 {
        self.application_id
    }

    /// Adapter-private schema identity persisted in the store header.
    #[must_use]
    pub fn schema_id(&self) -> &str {
        &self.schema_id
    }

    /// SHA-256 of the derived canonical resultant-schema projection.
    #[must_use]
    pub fn schema_sha256(&self) -> &str {
        &self.schema_sha256
    }

    /// Current initial-v1 physical schema version.
    #[must_use]
    pub const fn target_schema_version(&self) -> u32 {
        self.target_schema_version
    }

    /// Complete ordered migration set for this store kind.
    #[must_use]
    pub fn migrations(&self) -> &[VerifiedMigrationSource] {
        &self.migrations
    }

    /// Return the structurally non-empty initial-v1 migration head.
    pub(crate) fn migration_head(&self) -> &VerifiedMigrationSource {
        &self.migrations[0]
    }
}

/// Digest-verified initial-v1 storage source embedded in the adapter binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedStorageSources {
    manifest_sha256: String,
    sqlite_profile: SqliteProfileV1,
    authority_journal_profile: AuthorityJournalProfileV1,
    backup_restore_policy: BackupRestorePolicyV1,
    sqlite_version: String,
    principal_trust: VerifiedStoreSource,
    deployment_control: VerifiedStoreSource,
}

impl VerifiedStorageSources {
    /// SHA-256 of the canonical JCS manifest payload.
    #[must_use]
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }

    /// `SQLite` profile whose unresolved fields gate Pale Runtime assembly.
    #[must_use]
    pub const fn sqlite_profile(&self) -> &SqliteProfileV1 {
        &self.sqlite_profile
    }

    /// Physical authority-journal profile gated by measured corruption and recovery evidence.
    #[must_use]
    pub const fn authority_journal_profile(&self) -> &AuthorityJournalProfileV1 {
        &self.authority_journal_profile
    }

    /// Backup/restore frontier unresolved before Pale Runtime assembly.
    #[must_use]
    pub const fn backup_restore_policy(&self) -> BackupRestorePolicyV1 {
        self.backup_restore_policy
    }

    /// Return every unresolved `SQLite` and authority-journal admission coordinate.
    #[must_use]
    pub fn unresolved_profile_requirements(&self) -> Vec<ControlStorageProfileRequirement> {
        let mut unresolved = self.sqlite_profile.unresolved_requirements();
        unresolved.extend(self.authority_journal_profile.unresolved_requirements());
        unresolved.extend(self.backup_restore_policy.unresolved_requirements());
        unresolved
    }

    /// Whether the complete physical control-storage profile has been selected.
    #[must_use]
    pub fn is_runtime_profile_selected(&self) -> bool {
        self.sqlite_profile.is_selected()
            && self.authority_journal_profile.is_selected()
            && self.backup_restore_policy.is_selected()
    }

    /// Exact bundled `SQLite` library version bound by the manifest.
    #[must_use]
    pub fn sqlite_version(&self) -> &str {
        &self.sqlite_version
    }

    /// Verified source set for `kind`.
    #[must_use]
    pub fn store(&self, kind: ControlStoreKind) -> &VerifiedStoreSource {
        match kind {
            ControlStoreKind::PrincipalTrust => &self.principal_trust,
            ControlStoreKind::DeploymentControl => &self.deployment_control,
        }
    }
}

/// Verify the embedded manifest value and both exact migration byte sets.
///
/// This function performs no filesystem or persistent-database access. It does
/// apply each embedded migration to a transient in-memory database so the
/// declared headers and resultant schema can be derived rather than trusted.
/// Successful verification does not select the unresolved `SQLite` durability
/// profile or admit a resident writer.
///
/// # Errors
///
/// Returns [`ControlSqliteError`] when JSON decoding, JCS identity, exact
/// manifest fields, ordered migration coordinates, migration hashes, applied
/// header values, or resultant-schema hashes differ.
pub fn verify_embedded_storage_sources() -> Result<VerifiedStorageSources, ControlSqliteError> {
    verify_storage_sources(
        MANIFEST_BYTES,
        MANIFEST_SHA256,
        PRINCIPAL_TRUST_MIGRATION_V1,
        DEPLOYMENT_CONTROL_MIGRATION_V1,
    )
}

/// Shared private verifier for embedded production bytes and development output.
fn verify_storage_sources(
    manifest_bytes: &[u8],
    expected_pin: &str,
    principal_sql: &'static str,
    deployment_sql: &'static str,
) -> Result<VerifiedStorageSources, ControlSqliteError> {
    let value = decode_manifest_value(manifest_bytes, expected_pin)?;
    let manifest = decode_manifest(value)?;
    let principal = verify_store_source(
        ControlStoreKind::PrincipalTrust,
        manifest.stores.principal_trust,
        manifest.migration_sets.principal_trust,
        principal_sql,
    )?;
    let deployment = verify_store_source(
        ControlStoreKind::DeploymentControl,
        manifest.stores.deployment_control,
        manifest.migration_sets.deployment_control,
        deployment_sql,
    )?;
    Ok(VerifiedStorageSources {
        manifest_sha256: expected_pin.to_owned(),
        sqlite_profile: manifest.sqlite_profile,
        authority_journal_profile: manifest.authority_journal_profile,
        backup_restore_policy: manifest.backup_restore_policy,
        sqlite_version: manifest.sqlite.library_version,
        principal_trust: principal,
        deployment_control: deployment,
    })
}

/// Decode and check the same JCS pin before either verification or regeneration.
fn decode_manifest_value(
    manifest_bytes: &[u8],
    expected_pin: &str,
) -> Result<serde_json::Value, ControlSqliteError> {
    let payload = manifest_bytes.strip_suffix(b"\n").unwrap_or(manifest_bytes);
    let value: serde_json::Value =
        serde_json::from_slice(payload).map_err(ControlSqliteError::ManifestDecode)?;
    let canonical = serde_jcs::to_vec(&value)
        .map_err(|_| ControlSqliteError::SourceInvariant("manifest JCS encoding"))?;
    if sha256_hex(&canonical) != expected_pin {
        return Err(ControlSqliteError::ManifestDigestMismatch);
    }
    Ok(value)
}

/// The sole strict private manifest shape and constant-policy validation.
fn decode_manifest(value: serde_json::Value) -> Result<StorageManifest, ControlSqliteError> {
    let manifest: StorageManifest =
        serde_json::from_value(value).map_err(ControlSqliteError::ManifestDecode)?;
    validate_manifest_constants(&manifest)?;

    Ok(manifest)
}

/// Reject manifest bytes that drift from dependency, fixture, migration, or profile law.
fn validate_manifest_constants(manifest: &StorageManifest) -> Result<(), ControlSqliteError> {
    if manifest.manifest_id != MANIFEST_ID || manifest.manifest_version != MANIFEST_VERSION {
        return Err(ControlSqliteError::SourceInvariant("manifest identity"));
    }
    if manifest.fixture_roots != ["adapters/control-store/tests/support"] {
        return Err(ControlSqliteError::SourceInvariant("fixture roots"));
    }
    if manifest.sqlite.library_version != rusqlite::version()
        || manifest.sqlite.link_strategy != "bundled"
        || manifest.sqlite.required_cargo_features != ["backup", "bundled"]
    {
        return Err(ControlSqliteError::SourceInvariant(
            "SQLite dependency identity",
        ));
    }
    if manifest.sqlite_profile.journal_mode != "wal"
        || !manifest.sqlite_profile.foreign_keys
        || manifest.sqlite_profile.trusted_schema
        || !manifest.sqlite_profile.has_coherent_frontier()
        || manifest.sqlite_profile.is_selected()
        || !manifest.authority_journal_profile.has_coherent_frontier()
        || manifest.authority_journal_profile.is_selected()
    {
        return Err(ControlSqliteError::SourceInvariant(
            "control-storage profile frontier",
        ));
    }
    if manifest.backup_restore_policy != BackupRestorePolicyV1::NoneInitialV1
        || manifest.backup_restore_policy.is_selected()
    {
        return Err(ControlSqliteError::SourceInvariant(
            "backup and restore policy",
        ));
    }
    Ok(())
}

/// Bind one physical store declaration to its sole embedded initial-v1 migration.
///
/// The migration file remains the authored source. Its digest and the separately
/// derived resultant-schema digest are both required so a migration identity can
/// never masquerade as proof of the schema that applying it actually creates.
fn verify_store_source(
    kind: ControlStoreKind,
    store: StoreManifest,
    migrations: Vec<MigrationManifest>,
    sql: &'static str,
) -> Result<VerifiedStoreSource, ControlSqliteError> {
    let [migration]: [MigrationManifest; 1] = migrations
        .try_into()
        .map_err(|_| ControlSqliteError::SourceInvariant("initial-v1 migration count"))?;
    if migration.id != "0001_initial_v1" || migration.ordinal != 1 {
        return Err(ControlSqliteError::SourceInvariant(
            "initial-v1 migration coordinate",
        ));
    }
    let expected_path = format!(
        "adapters/control-store/migrations/{}/0001_initial_v1.sql",
        match kind {
            ControlStoreKind::PrincipalTrust => "principal-trust",
            ControlStoreKind::DeploymentControl => "deployment-control",
        }
    );
    if migration.path != expected_path || sha256_hex(sql.as_bytes()) != migration.sha256 {
        return Err(ControlSqliteError::MigrationDigestMismatch {
            migration_id: migration.id,
        });
    }
    if store.target_schema_version != 1 {
        return Err(ControlSqliteError::SourceInvariant(
            "initial-v1 source-version frontier",
        ));
    }
    let connection = Connection::open_in_memory().map_err(|source| {
        ControlSqliteError::sqlite("open source-verification database", source)
    })?;
    connection.execute_batch(sql).map_err(|source| {
        ControlSqliteError::sqlite("apply source-verification migration", source)
    })?;
    let observed_application_id: i64 = connection
        .query_row("PRAGMA application_id", [], |row| row.get(0))
        .map_err(|source| ControlSqliteError::sqlite("verify source application ID", source))?;
    if observed_application_id != i64::from(store.application_id) {
        return Err(ControlSqliteError::SourceInvariant(
            "migration application ID",
        ));
    }
    let observed_user_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|source| ControlSqliteError::sqlite("verify source schema version", source))?;
    if observed_user_version != i64::from(store.target_schema_version) {
        return Err(ControlSqliteError::SourceInvariant(
            "migration target schema version",
        ));
    }
    if resultant_schema_sha256(&connection)? != store.schema_sha256 {
        return Err(ControlSqliteError::SourceInvariant(
            "resultant physical schema hash",
        ));
    }
    Ok(VerifiedStoreSource {
        kind,
        application_id: store.application_id,
        schema_id: store.schema_id,
        schema_sha256: store.schema_sha256,
        target_schema_version: store.target_schema_version,
        migrations: [VerifiedMigrationSource {
            id: migration.id,
            ordinal: migration.ordinal,
            path: migration.path,
            sha256: migration.sha256,
            sql,
        }],
    })
}

/// Strict private decode shape for [`MANIFEST_BYTES`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageManifest {
    authority_journal_profile: AuthorityJournalProfileV1,
    backup_restore_policy: BackupRestorePolicyV1,
    fixture_roots: Vec<String>,
    manifest_id: String,
    manifest_version: u32,
    migration_sets: MigrationSets,
    sqlite: SqliteDependency,
    sqlite_profile: SqliteProfileV1,
    stores: StoreManifests,
}

/// The two independently ordered physical migration sets.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MigrationSets {
    deployment_control: Vec<MigrationManifest>,
    principal_trust: Vec<MigrationManifest>,
}

/// One source-owned migration coordinate before byte verification.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MigrationManifest {
    id: String,
    ordinal: u32,
    path: String,
    sha256: String,
}

/// Exact bundled library and required feature identity.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SqliteDependency {
    library_version: String,
    link_strategy: String,
    required_cargo_features: Vec<String>,
}

/// Physical schema declarations keyed by closed store kind.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreManifests {
    deployment_control: StoreManifest,
    principal_trust: StoreManifest,
}

/// One adapter-private resultant initial-v1 schema.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreManifest {
    application_id: i32,
    schema_id: String,
    schema_sha256: String,
    target_schema_version: u32,
}

#[cfg(test)]
#[path = "../tests/internal/sources.rs"]
mod tests;
