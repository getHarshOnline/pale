//! Initial store creation and fail-closed open classification.

pub(crate) mod journal;
pub(crate) mod transaction;

#[cfg(any(test, feature = "bench"))]
use std::path::{Path, PathBuf};

use pale_core::records::{ControlIncarnationIdV1, ControlStoreIdV1, ControlTransactionSequenceV1};
use pale_core::sha256_hex;
#[cfg(any(test, feature = "bench"))]
use rusqlite::{params, OpenFlags, TransactionBehavior};
use rusqlite::{Connection, OptionalExtension};

#[cfg(any(test, feature = "bench"))]
use crate::candidate::CandidateReservation;
use crate::error::{sqlite_error_is_corruption, sqlite_error_is_operational};
use crate::location::{
    ControlStoreLocation, ControlStoreScope, FileIdentity, PersistentJournalMode,
    StoreFileObservation,
};
use crate::schema::resultant_schema_sha256;
use crate::{
    ControlSqliteError, ControlStorageProfileRequirement, ControlStoreKind, VerifiedStorageSources,
    VerifiedStoreSource,
};

/// Exact logical and incarnation identity expected at one physical store path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlStoreIdentity {
    scope: ControlStoreScope,
    logical_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
}

impl ControlStoreIdentity {
    /// Construct an identity from already-validated resident ID domains.
    #[must_use]
    pub(crate) const fn new(
        scope: ControlStoreScope,
        logical_store_id: ControlStoreIdV1,
        control_incarnation_id: ControlIncarnationIdV1,
    ) -> Self {
        Self {
            scope,
            logical_store_id,
            control_incarnation_id,
        }
    }

    /// Store kind selected by this identity.
    #[must_use]
    pub(crate) const fn kind(&self) -> ControlStoreKind {
        self.scope.kind()
    }

    /// Raw principal or deployment coordinates bound to this physical store.
    #[must_use]
    pub(crate) const fn scope(&self) -> &ControlStoreScope {
        &self.scope
    }

    /// Stable logical store ID preserved across an authorized recovery incarnation.
    #[must_use]
    pub(crate) const fn logical_store_id(&self) -> &ControlStoreIdV1 {
        &self.logical_store_id
    }

    /// Exact physical control-store incarnation.
    #[must_use]
    pub(crate) const fn control_incarnation_id(&self) -> &ControlIncarnationIdV1 {
        &self.control_incarnation_id
    }
}

/// Fail-closed reason that prevents a physical store from normal opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StoreOpenRejection {
    /// Filesystem entry is not a regular, non-symlink database file.
    UnsafeEntryKind,
    /// `SQLite` could not decode the expected initial-v1 physical structures.
    CorruptOrPartial,
    /// Store application ID or persisted kind names the other store class.
    WrongStoreKind,
    /// Logical store identity differs from the caller's immutable expectation.
    WrongLogicalStoreId,
    /// Physical incarnation differs from the caller's immutable expectation.
    WrongControlIncarnation,
    /// Persisted raw principal identity differs from the typed trust-store scope.
    WrongPrincipalScope,
    /// Persisted workspace or deployment-binding coordinate differs from its scope.
    WrongDeploymentScope,
    /// Persisted schema or source-manifest identity differs from embedded source.
    SourceIdentityMismatch,
    /// Physical schema is newer than this binary understands.
    NewerVersion {
        /// Observed physical schema version.
        observed: u32,
        /// Greatest source-owned version understood by this binary.
        supported: u32,
    },
    /// Runtime `SQLite` library or compile-option observation differs from the store.
    SqliteBuildMismatch,
    /// A fixed structural safety setting differs from the storage manifest.
    StructuralProfileMismatch,
}

/// Structural result of inspecting one expected store path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StoreOpenClassification {
    /// No filesystem entry exists at the expected path.
    Missing,
    /// A zero-length regular file exists but contains no initialized store.
    Empty,
    /// Current source is intact, but measurement-owned `SQLite` settings are absent.
    ProfileUnselected {
        /// Exact fields that still prevent Pale Runtime assembly.
        unresolved: Vec<ControlStorageProfileRequirement>,
    },
    /// Current source and a fully selected profile are both verified.
    Ready,
    /// The path must remain unopened until an explicit recovery path resolves it.
    Rejected(StoreOpenRejection),
}

/// Evidence returned after creating an initial-v1 store without Pale Runtime assembly.
#[cfg(any(test, feature = "bench"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitializedStore {
    path: PathBuf,
    identity: ControlStoreIdentity,
    manifest_sha256: String,
    compile_options_sha256: String,
}

#[cfg(any(test, feature = "bench"))]
impl InitializedStore {
    /// Candidate database path written by initialization.
    #[must_use]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Exact logical and incarnation identity persisted in the store header.
    #[must_use]
    pub(crate) const fn identity(&self) -> &ControlStoreIdentity {
        &self.identity
    }

    /// Canonical storage-manifest digest persisted in the store header.
    #[must_use]
    pub(crate) fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }

    /// Digest of the sorted bundled `SQLite` compile-option observation.
    #[must_use]
    pub(crate) fn compile_options_sha256(&self) -> &str {
        &self.compile_options_sha256
    }
}

/// Create one structurally complete initial-v1 store at a new path.
///
/// Initialization verifies embedded source, fixes the already-selected WAL mode,
/// applies the complete migration set atomically, and records the exact `SQLite`
/// build observation. It returns no open connection and proves no resident
/// readiness: [`classify_store`] remains [`StoreOpenClassification::ProfileUnselected`]
/// until measurement freezes every gated profile field.
///
/// # Errors
///
/// Returns [`ControlSqliteError`] when source verification fails, the target
/// already exists, `SQLite` differs from the pinned library version, WAL cannot be
/// selected, migration or header insertion fails, or integrity checking fails.
#[cfg(any(test, feature = "bench"))]
pub(crate) fn initialize_unadmitted_store(
    location: &ControlStoreLocation,
    identity: ControlStoreIdentity,
) -> Result<InitializedStore, ControlSqliteError> {
    require_location_scope(location, &identity)?;
    let sources = crate::verify_embedded_storage_sources()?;
    let migration_sql = sources.store(identity.kind()).migration_head().sql();
    initialize_unadmitted_store_with_sql(location, identity, sources, migration_sql)
}

/// Run initialization against an explicit SQL source while retaining cleanup ownership.
///
/// [`initialize_unadmitted_store`] supplies the verified embedded migration. This
/// lower boundary keeps candidate cleanup correct for every SQL application failure.
#[cfg(any(test, feature = "bench"))]
fn initialize_unadmitted_store_with_sql(
    location: &ControlStoreLocation,
    identity: ControlStoreIdentity,
    sources: VerifiedStorageSources,
    migration_sql: &str,
) -> Result<InitializedStore, ControlSqliteError> {
    require_location_scope(location, &identity)?;
    let reservation = CandidateReservation::reserve_store(location)?;
    let mut connection = match reservation.open_sqlite("open reserved store candidate") {
        Ok(connection) => connection,
        Err(failure) => return reservation.finish(Err(failure)),
    };
    let initialization =
        initialize_connection_with_sql(&mut connection, &identity, &sources, migration_sql);
    drop(connection);
    let compile_options_sha256 = reservation.finish(initialization)?;

    Ok(InitializedStore {
        path: location.path(),
        identity,
        manifest_sha256: sources.manifest_sha256().to_owned(),
        compile_options_sha256,
    })
}

/// Apply fixed connection safety law and the sole initial-v1 migration atomically.
///
/// The returned digest identifies the runtime compile-option observation; it is
/// evidence, not selection of the still-unresolved performance profile.
#[cfg(any(test, feature = "bench"))]
fn initialize_connection_with_sql(
    connection: &mut Connection,
    identity: &ControlStoreIdentity,
    sources: &VerifiedStorageSources,
    migration_sql: &str,
) -> Result<String, ControlSqliteError> {
    let store_source = sources.store(identity.kind());
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|source| ControlSqliteError::sqlite("enable SQLite foreign keys", source))?;
    connection
        .pragma_update(None, "trusted_schema", false)
        .map_err(|source| ControlSqliteError::sqlite("disable SQLite trusted schema", source))?;
    verify_connection_pragmas(connection)?;
    let observed_version = sqlite_version(connection)?;
    if observed_version != sources.sqlite_version() {
        return Err(ControlSqliteError::SourceInvariant(
            "runtime SQLite library version",
        ));
    }
    let compile_options_sha256 = compile_options_sha256(connection)?;
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .map_err(|source| ControlSqliteError::sqlite("select WAL journal mode", source))?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(ControlSqliteError::SourceInvariant(
            "runtime WAL journal mode",
        ));
    }
    verify_journal_mode(connection)?;

    let migration = store_source.migration_head();
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|source| ControlSqliteError::sqlite("begin initial-v1 migration", source))?;
    transaction
        .execute_batch(migration_sql)
        .map_err(|source| ControlSqliteError::sqlite("apply initial-v1 migration", source))?;
    verify_migration_realization(&transaction, store_source)?;
    transaction
        .execute(
            "INSERT INTO pale_control_store_header \
             (singleton, store_kind, logical_store_id, control_incarnation_id, store_schema_id, \
              store_schema_sha256, store_schema_version, storage_manifest_sha256, migration_head_sha256, \
              logical_transaction_sequence) \
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1)",
            params![
                identity.kind().as_str(),
                identity.logical_store_id().as_str(),
                identity.control_incarnation_id().as_str(),
                store_source.schema_id(),
                store_source.schema_sha256(),
                store_source.target_schema_version(),
                sources.manifest_sha256(),
                migration.sha256(),
            ],
        )
        .map_err(|source| ControlSqliteError::sqlite("write control-store header", source))?;
    write_scope_binding(&transaction, identity.scope())?;
    transaction
        .execute(
            "INSERT INTO pale_control_store_profile_observation \
             (singleton, sqlite_version, compile_options_sha256, journal_mode, foreign_keys, trusted_schema) \
             VALUES (1, ?1, ?2, 'wal', 1, 0)",
            params![observed_version, compile_options_sha256],
        )
        .map_err(|source| ControlSqliteError::sqlite("write SQLite build observation", source))?;
    transaction
        .execute(
            "INSERT INTO pale_control_store_migration_receipt \
             (migration_ordinal, migration_id, migration_sha256) VALUES (?1, ?2, ?3)",
            params![migration.ordinal(), migration.id(), migration.sha256()],
        )
        .map_err(|source| {
            ControlSqliteError::sqlite("record applied initial-v1 migration", source)
        })?;
    transaction
        .commit()
        .map_err(|source| ControlSqliteError::sqlite("commit initial-v1 migration", source))?;
    verify_integrity(connection)?;
    Ok(compile_options_sha256)
}

/// Persist the raw typed scope separately from its digest-keyed path projection.
#[cfg(any(test, feature = "bench"))]
fn write_scope_binding(
    connection: &Connection,
    scope: &ControlStoreScope,
) -> Result<(), ControlSqliteError> {
    match scope {
        ControlStoreScope::Principal(scope) => connection
            .execute(
                "INSERT INTO pale_principal_trust_store_binding (singleton, principal_id) \
                 VALUES (1, ?1)",
                [scope.principal_id().as_str()],
            )
            .map(drop)
            .map_err(|source| {
                ControlSqliteError::sqlite("write principal trust-store binding", source)
            }),
        ControlStoreScope::Deployment(scope) => connection
            .execute(
                "INSERT INTO pale_deployment_control_store_binding \
                 (singleton, workspace_id, deployment_binding_ref, deployment_binding_sha256) \
                 VALUES (1, ?1, ?2, ?3)",
                params![
                    scope.workspace_id().as_str(),
                    scope.deployment_binding_ref().as_str(),
                    scope.deployment_binding_sha256().as_str(),
                ],
            )
            .map(drop)
            .map_err(|source| {
                ControlSqliteError::sqlite("write deployment control-store binding", source)
            }),
    }
}

/// Reject a canonical locator combined with another semantic scope.
fn require_location_scope(
    location: &ControlStoreLocation,
    identity: &ControlStoreIdentity,
) -> Result<(), ControlSqliteError> {
    if location.scope() == identity.scope() {
        Ok(())
    } else {
        Err(ControlSqliteError::StoreLocationScopeMismatch)
    }
}

/// Inspect an expected store without granting a usable runtime connection.
///
/// The classification is intentionally richer than success/failure: missing,
/// empty, profile-unselected, ready, and every fail-closed
/// rejection have different recovery consequences. `SQLite`, PID, or filesystem
/// discovery alone never supplies resident-writer authority.
///
/// # Errors
///
/// Returns [`ControlSqliteError`] when source verification or filesystem access
/// fails, or when `SQLite` reports an operational condition such as permission,
/// availability, or lock contention. Persistent byte corruption and decodable
/// contract contradictions become typed [`StoreOpenClassification::Rejected`]
/// results.
#[cfg(any(test, feature = "bench"))]
pub(crate) fn classify_store(
    location: &ControlStoreLocation,
    expected: &ControlStoreIdentity,
) -> Result<StoreOpenClassification, ControlSqliteError> {
    require_location_scope(location, expected)?;
    let sources = crate::verify_embedded_storage_sources()?;
    classify_store_with_sources(location, expected, &sources)
}

/// Classify one pathname against an already-verified embedded source set.
#[cfg(any(test, feature = "bench"))]
pub(crate) fn classify_store_with_sources(
    location: &ControlStoreLocation,
    expected: &ControlStoreIdentity,
    sources: &VerifiedStorageSources,
) -> Result<StoreOpenClassification, ControlSqliteError> {
    require_location_scope(location, expected)?;
    classify_observed_store(location, |identity| {
        let connection = match location.open_existing_sqlite(
            OpenFlags::SQLITE_OPEN_READ_ONLY,
            identity,
            "open control store for classification",
        ) {
            Ok(connection) => connection,
            Err(error) => return classify_open_failure(error),
        };
        classify_connection_with_sources(&connection, expected, sources)
    })
}

/// Inspect one physical store without writing its main file or sidecars.
pub(crate) fn inspect_store_with_sources(
    location: &ControlStoreLocation,
    expected: &ControlStoreIdentity,
    sources: &VerifiedStorageSources,
) -> Result<StoreOpenClassification, ControlSqliteError> {
    require_location_scope(location, expected)?;
    classify_observed_store(location, |identity| {
        match location.persistent_journal_mode(identity)? {
            PersistentJournalMode::Wal => {}
            PersistentJournalMode::Other => {
                return Ok(StoreOpenClassification::Rejected(
                    StoreOpenRejection::StructuralProfileMismatch,
                ));
            }
            PersistentJournalMode::Incomplete => return Ok(rejected_corrupt()),
        }
        let connection = match location.open_existing_sqlite_immutable(
            identity,
            "open control store for immutable classification",
        ) {
            Ok(connection) => connection,
            Err(error) => return classify_open_failure(error),
        };
        classify_connection_with_sources_at_mode(
            &connection,
            expected,
            sources,
            JournalModeCheck::PersistentHeaderVerified,
        )
    })
}

/// Observe the physical leaf once before selecting a read-only classifier.
fn classify_observed_store(
    location: &ControlStoreLocation,
    classify_regular: impl FnOnce(FileIdentity) -> Result<StoreOpenClassification, ControlSqliteError>,
) -> Result<StoreOpenClassification, ControlSqliteError> {
    match location.entry()? {
        StoreFileObservation::Missing => Ok(StoreOpenClassification::Missing),
        StoreFileObservation::Unsafe => Ok(StoreOpenClassification::Rejected(
            StoreOpenRejection::UnsafeEntryKind,
        )),
        StoreFileObservation::Regular { empty: true, .. } => Ok(StoreOpenClassification::Empty),
        StoreFileObservation::Regular { identity, .. } => classify_regular(identity),
    }
}

/// Reduce malformed bytes at open while preserving operational unavailability.
fn classify_open_failure(
    error: ControlSqliteError,
) -> Result<StoreOpenClassification, ControlSqliteError> {
    if matches!(
        &error,
        ControlSqliteError::Sqlite { source, .. } if sqlite_error_is_corruption(source)
    ) {
        Ok(rejected_corrupt())
    } else {
        Err(error)
    }
}

/// Classify the exact open connection that a caller will continue consuming.
///
/// The caller retains the open connection throughout classification, so no
/// second pathname lookup can silently substitute the inspected database.
#[cfg(any(test, feature = "bench"))]
pub(crate) fn classify_connection_with_sources(
    connection: &Connection,
    expected: &ControlStoreIdentity,
    sources: &VerifiedStorageSources,
) -> Result<StoreOpenClassification, ControlSqliteError> {
    classify_connection_with_sources_at_mode(
        connection,
        expected,
        sources,
        JournalModeCheck::Connection,
    )
}

/// Journal-mode proof available to one connection classifier.
#[derive(Clone, Copy)]
enum JournalModeCheck {
    /// Query the retained live connection.
    #[cfg(any(test, feature = "bench"))]
    Connection,
    /// The immutable observer verified the persistent file-format header.
    PersistentHeaderVerified,
}

/// Classify one connection under its exact journal-mode evidence.
fn classify_connection_with_sources_at_mode(
    connection: &Connection,
    expected: &ControlStoreIdentity,
    sources: &VerifiedStorageSources,
    journal_mode: JournalModeCheck,
) -> Result<StoreOpenClassification, ControlSqliteError> {
    if let Err(error) = connection.pragma_update(None, "foreign_keys", true) {
        return classify_sqlite_failure("enable classification foreign keys", error);
    }
    if let Err(error) = connection.pragma_update(None, "trusted_schema", false) {
        return classify_sqlite_failure("disable classification trusted schema", error);
    }
    if let Err(error) = verify_connection_pragmas(connection) {
        return classify_adapter_failure(error, StoreOpenRejection::StructuralProfileMismatch);
    }
    if let Err(error) = verify_integrity(connection) {
        return classify_adapter_failure(error, StoreOpenRejection::CorruptOrPartial);
    }
    let source = sources.store(expected.kind());
    let application_id: i64 =
        match connection.query_row("PRAGMA application_id", [], |row| row.get(0)) {
            Ok(value) => value,
            Err(error) => return classify_sqlite_failure("read SQLite application ID", error),
        };
    if application_id != i64::from(source.application_id()) {
        return Ok(StoreOpenClassification::Rejected(
            StoreOpenRejection::WrongStoreKind,
        ));
    }
    let user_version: i64 = match connection.query_row("PRAGMA user_version", [], |row| row.get(0))
    {
        Ok(value) => value,
        Err(error) => return classify_sqlite_failure("read SQLite user version", error),
    };
    let observed_version = match u32::try_from(user_version) {
        Ok(version) => version,
        Err(_) => return Ok(rejected_corrupt()),
    };
    if observed_version > source.target_schema_version() {
        return Ok(StoreOpenClassification::Rejected(
            StoreOpenRejection::NewerVersion {
                observed: observed_version,
                supported: source.target_schema_version(),
            },
        ));
    }
    if observed_version < source.target_schema_version() {
        return Ok(rejected_corrupt());
    }
    let live_schema_sha256 = match resultant_schema_sha256(connection) {
        Ok(digest) => digest,
        Err(error) => {
            return classify_adapter_failure(error, StoreOpenRejection::CorruptOrPartial);
        }
    };
    if live_schema_sha256 != source.schema_sha256() {
        return Ok(StoreOpenClassification::Rejected(
            StoreOpenRejection::SourceIdentityMismatch,
        ));
    }
    let header = match read_header(connection) {
        Ok(Some(header)) => header,
        Ok(None) => return Ok(rejected_corrupt()),
        Err(error) => return classify_sqlite_failure("read control-store header", error),
    };
    if u64::try_from(header.logical_transaction_sequence)
        .ok()
        .and_then(|value| ControlTransactionSequenceV1::new(value).ok())
        .is_none()
    {
        return Ok(rejected_corrupt());
    }
    if header.store_kind != expected.kind().as_str() {
        return Ok(StoreOpenClassification::Rejected(
            StoreOpenRejection::WrongStoreKind,
        ));
    }
    if header.logical_store_id != expected.logical_store_id().as_str() {
        return Ok(StoreOpenClassification::Rejected(
            StoreOpenRejection::WrongLogicalStoreId,
        ));
    }
    if header.control_incarnation_id != expected.control_incarnation_id().as_str() {
        return Ok(StoreOpenClassification::Rejected(
            StoreOpenRejection::WrongControlIncarnation,
        ));
    }
    match scope_binding_matches(connection, expected.scope()) {
        Ok(true) => {}
        Ok(false) => {
            return Ok(StoreOpenClassification::Rejected(match expected.scope() {
                ControlStoreScope::Principal(_) => StoreOpenRejection::WrongPrincipalScope,
                ControlStoreScope::Deployment(_) => StoreOpenRejection::WrongDeploymentScope,
            }));
        }
        Err(error) => return classify_sqlite_failure("read control-store scope binding", error),
    }
    if header.store_schema_id != source.schema_id()
        || header.store_schema_sha256 != source.schema_sha256()
        || header.store_schema_version != source.target_schema_version()
        || header.storage_manifest_sha256 != sources.manifest_sha256()
        || header.migration_head_sha256 != source.migration_head().sha256()
    {
        return Ok(StoreOpenClassification::Rejected(
            StoreOpenRejection::SourceIdentityMismatch,
        ));
    }
    match migration_receipt_matches(connection, source) {
        Ok(true) => {}
        Ok(false) => {
            return Ok(StoreOpenClassification::Rejected(
                StoreOpenRejection::SourceIdentityMismatch,
            ));
        }
        Err(error) => {
            return classify_sqlite_failure("read initial-v1 migration receipt", error);
        }
    }
    match journal_mode {
        #[cfg(any(test, feature = "bench"))]
        JournalModeCheck::Connection => {
            if let Err(error) = verify_journal_mode(connection) {
                return classify_adapter_failure(
                    error,
                    StoreOpenRejection::StructuralProfileMismatch,
                );
            }
        }
        JournalModeCheck::PersistentHeaderVerified => {}
    }
    let observation = match read_build_observation(connection) {
        Ok(Some(observation)) => observation,
        Ok(None) => return Ok(rejected_corrupt()),
        Err(error) => return classify_sqlite_failure("read SQLite build observation", error),
    };
    let runtime_version = match sqlite_version(connection) {
        Ok(version) => version,
        Err(error) => {
            return classify_adapter_failure(error, StoreOpenRejection::CorruptOrPartial);
        }
    };
    let runtime_options = match compile_options_sha256(connection) {
        Ok(digest) => digest,
        Err(error) => {
            return classify_adapter_failure(error, StoreOpenRejection::CorruptOrPartial);
        }
    };
    if observation.sqlite_version != sources.sqlite_version()
        || runtime_version != sources.sqlite_version()
        || observation.compile_options_sha256 != runtime_options
        || !observation.journal_mode.eq_ignore_ascii_case("wal")
        || observation.foreign_keys != 1
        || observation.trusted_schema != 0
        || sources
            .sqlite_profile()
            .compile_options_sha256
            .as_ref()
            .is_some_and(|expected| expected != &runtime_options)
    {
        return Ok(StoreOpenClassification::Rejected(
            StoreOpenRejection::SqliteBuildMismatch,
        ));
    }
    let unresolved = sources.unresolved_profile_requirements();
    if unresolved.is_empty() {
        Ok(StoreOpenClassification::Ready)
    } else {
        Ok(StoreOpenClassification::ProfileUnselected { unresolved })
    }
}

/// Require `SQLite`'s complete integrity check to return its sole success value.
pub(crate) fn verify_integrity(connection: &Connection) -> Result<(), ControlSqliteError> {
    let result: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|source| ControlSqliteError::sqlite("verify SQLite integrity", source))?;
    if result == "ok" {
        Ok(())
    } else {
        Err(ControlSqliteError::SourceInvariant(
            "SQLite integrity check",
        ))
    }
}

/// Re-read connection-local foreign-key and trusted-schema safety settings.
fn verify_connection_pragmas(connection: &Connection) -> Result<(), ControlSqliteError> {
    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|source| ControlSqliteError::sqlite("verify foreign-key enforcement", source))?;
    let trusted_schema: i64 = connection
        .query_row("PRAGMA trusted_schema", [], |row| row.get(0))
        .map_err(|source| ControlSqliteError::sqlite("verify trusted-schema policy", source))?;
    if foreign_keys == 1 && trusted_schema == 0 {
        Ok(())
    } else {
        Err(ControlSqliteError::SourceInvariant(
            "structural SQLite pragma profile",
        ))
    }
}

/// Require the persistent journal mode selected by the storage manifest.
#[cfg(any(test, feature = "bench"))]
fn verify_journal_mode(connection: &Connection) -> Result<(), ControlSqliteError> {
    let journal_mode: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(|source| ControlSqliteError::sqlite("verify SQLite journal mode", source))?;
    if journal_mode.eq_ignore_ascii_case("wal") {
        Ok(())
    } else {
        Err(ControlSqliteError::SourceInvariant(
            "structural SQLite journal mode",
        ))
    }
}

/// Read the runtime `SQLite` library version from the active connection.
pub(crate) fn sqlite_version(connection: &Connection) -> Result<String, ControlSqliteError> {
    connection
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .map_err(|source| ControlSqliteError::sqlite("observe SQLite library version", source))
}

/// Hash the sorted, deduplicated runtime `SQLite` compile-option set.
pub(crate) fn compile_options_sha256(
    connection: &Connection,
) -> Result<String, ControlSqliteError> {
    let mut statement = connection
        .prepare("PRAGMA compile_options")
        .map_err(|source| {
            ControlSqliteError::sqlite("prepare compile-option observation", source)
        })?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|source| ControlSqliteError::sqlite("read compile-option observation", source))?;
    let mut options = rows.collect::<Result<Vec<_>, _>>().map_err(|source| {
        ControlSqliteError::sqlite("decode compile-option observation", source)
    })?;
    options.sort_unstable();
    options.dedup();
    Ok(sha256_hex(options.join("\n").as_bytes()))
}

/// Read the singleton private header without interpreting its identity claims.
fn read_header(connection: &Connection) -> rusqlite::Result<Option<StoreHeader>> {
    connection
        .query_row(
            "SELECT store_kind, logical_store_id, control_incarnation_id, store_schema_id, \
             store_schema_sha256, store_schema_version, storage_manifest_sha256, migration_head_sha256, \
             logical_transaction_sequence \
             FROM pale_control_store_header WHERE singleton = 1",
            [],
            |row| {
                Ok(StoreHeader {
                    store_kind: row.get(0)?,
                    logical_store_id: row.get(1)?,
                    control_incarnation_id: row.get(2)?,
                    store_schema_id: row.get(3)?,
                    store_schema_sha256: row.get(4)?,
                    store_schema_version: row.get(5)?,
                    storage_manifest_sha256: row.get(6)?,
                    migration_head_sha256: row.get(7)?,
                    logical_transaction_sequence: row.get(8)?,
                })
            },
        )
        .optional()
}

/// Compare the kind-specific raw binding row with its typed expected scope.
fn scope_binding_matches(
    connection: &Connection,
    expected: &ControlStoreScope,
) -> rusqlite::Result<bool> {
    match expected {
        ControlStoreScope::Principal(scope) => {
            let observed = connection
                .query_row(
                    "SELECT principal_id FROM pale_principal_trust_store_binding \
                     WHERE singleton = 1",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            Ok(observed.as_deref() == Some(scope.principal_id().as_str()))
        }
        ControlStoreScope::Deployment(scope) => {
            let observed = connection
                .query_row(
                    "SELECT workspace_id, deployment_binding_ref, deployment_binding_sha256 \
                     FROM pale_deployment_control_store_binding WHERE singleton = 1",
                    [],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?;
            Ok(observed
                .as_ref()
                .is_some_and(|(workspace, binding_ref, binding_sha256)| {
                    workspace == scope.workspace_id().as_str()
                        && binding_ref == scope.deployment_binding_ref().as_str()
                        && binding_sha256 == scope.deployment_binding_sha256().as_str()
                }))
        }
    }
}

/// Read the `SQLite` build and fixed-profile observations captured at initialization.
fn read_build_observation(
    connection: &Connection,
) -> rusqlite::Result<Option<SqliteBuildObservation>> {
    connection
        .query_row(
            "SELECT sqlite_version, compile_options_sha256, journal_mode, foreign_keys, trusted_schema \
             FROM pale_control_store_profile_observation WHERE singleton = 1",
            [],
            |row| {
                Ok(SqliteBuildObservation {
                    sqlite_version: row.get(0)?,
                    compile_options_sha256: row.get(1)?,
                    journal_mode: row.get(2)?,
                    foreign_keys: row.get(3)?,
                    trusted_schema: row.get(4)?,
                })
            },
        )
        .optional()
}

/// Require exactly one applied initial-v1 receipt matching the header head.
fn migration_receipt_matches(
    connection: &Connection,
    source: &VerifiedStoreSource,
) -> rusqlite::Result<bool> {
    let row = connection
        .query_row(
            "SELECT migration_ordinal, migration_id, migration_sha256 \
             FROM pale_control_store_migration_receipt",
            [],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional();
    let count: rusqlite::Result<u32> = connection.query_row(
        "SELECT COUNT(*) FROM pale_control_store_migration_receipt",
        [],
        |row| row.get(0),
    );
    match (row?, count?) {
        (Some((ordinal, id, sha256)), 1) => {
            let expected = source.migration_head();
            Ok(ordinal == expected.ordinal() && id == expected.id() && sha256 == expected.sha256())
        }
        _ => Ok(false),
    }
}

/// Prove that one applied migration realizes the source-owned physical schema.
#[cfg(any(test, feature = "bench"))]
fn verify_migration_realization(
    connection: &Connection,
    source: &VerifiedStoreSource,
) -> Result<(), ControlSqliteError> {
    let application_id: i64 = connection
        .query_row("PRAGMA application_id", [], |row| row.get(0))
        .map_err(|error| ControlSqliteError::sqlite("verify migration application ID", error))?;
    let user_version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| ControlSqliteError::sqlite("verify migration user version", error))?;
    let schema_sha256 = resultant_schema_sha256(connection)?;
    if application_id == i64::from(source.application_id())
        && user_version == i64::from(source.target_schema_version())
        && schema_sha256 == source.schema_sha256()
    {
        Ok(())
    } else {
        Err(ControlSqliteError::SourceInvariant(
            "applied migration resultant schema",
        ))
    }
}

/// Preserve host-operation failures while reducing malformed bytes to rejection.
fn classify_sqlite_failure(
    operation: &'static str,
    source: rusqlite::Error,
) -> Result<StoreOpenClassification, ControlSqliteError> {
    if sqlite_error_is_operational(&source) {
        Err(ControlSqliteError::sqlite(operation, source))
    } else {
        Ok(rejected_corrupt())
    }
}

/// Interpret one adapter check without erasing its `SQLite` source category.
fn classify_adapter_failure(
    error: ControlSqliteError,
    structural: StoreOpenRejection,
) -> Result<StoreOpenClassification, ControlSqliteError> {
    match error {
        ControlSqliteError::Sqlite { ref source, .. } if sqlite_error_is_operational(source) => {
            Err(error)
        }
        ControlSqliteError::Sqlite { .. } => Ok(rejected_corrupt()),
        ControlSqliteError::SourceInvariant(_) => Ok(StoreOpenClassification::Rejected(structural)),
        other => Err(other),
    }
}

/// Canonical classification for undecodable or structurally incomplete bytes.
const fn rejected_corrupt() -> StoreOpenClassification {
    StoreOpenClassification::Rejected(StoreOpenRejection::CorruptOrPartial)
}

/// Untrusted adapter-private singleton header read from a candidate store.
struct StoreHeader {
    store_kind: String,
    logical_store_id: String,
    control_incarnation_id: String,
    store_schema_id: String,
    store_schema_sha256: String,
    store_schema_version: u32,
    storage_manifest_sha256: String,
    migration_head_sha256: String,
    logical_transaction_sequence: i64,
}

/// Untrusted initialization-time `SQLite` observation read from a candidate store.
struct SqliteBuildObservation {
    sqlite_version: String,
    compile_options_sha256: String,
    journal_mode: String,
    foreign_keys: i64,
    trusted_schema: i64,
}

#[cfg(test)]
#[path = "../tests/internal/store.rs"]
mod tests;
