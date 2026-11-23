//! Read materialization state and verify it against an independent lineage record.

use std::path::Path;

use pale_core::canon;
use pale_core::records::{
    LogicalRepositoryIdV1, MaterializationContractIdentity, MaterializationLineageV1,
    MaterializationStateV1, TargetRootDigest, MATERIALIZATION_LINEAGE_SCHEMA,
    MATERIALIZATION_LINEAGE_SCHEMA_VERSION, MATERIALIZATION_STATE_PATH,
    MATERIALIZATION_STATE_SCHEMA, MATERIALIZATION_STATE_SCHEMA_VERSION,
};

use crate::confine::{ConfinedLocation, ConfinedRoot, ReadLimit};
use crate::errors::{data_error, FsError};

/// Repository state paired with its independently rooted lineage anchor.
pub(crate) struct TrustedMaterialization {
    /// Canonical repository-local ownership record.
    pub(crate) state: MaterializationStateV1,
    /// Canonical machine-bound authority for that exact state.
    pub(crate) lineage: MaterializationLineageV1,
}

/// Maximum bytes admitted for one materialization-state authority record.
const MATERIALIZATION_STATE_LIMIT: ReadLimit = ReadLimit::new(1 << 20);
/// Maximum bytes admitted for one materialization-lineage authority record.
const MATERIALIZATION_LINEAGE_LIMIT: ReadLimit = ReadLimit::new(256 * 1024);

fn invalid(message: impl Into<String>) -> FsError {
    FsError::InvalidState(message.into())
}

/// Validate schema identity, self-digest, owned-path grammar, and canonical ordering.
pub(super) fn validate_state(
    state: &MaterializationStateV1,
    expected: &MaterializationContractIdentity,
) -> Result<(), FsError> {
    validate_state_at(state, expected, Path::new(MATERIALIZATION_STATE_PATH))
}

fn validate_state_at(
    state: &MaterializationStateV1,
    expected: &MaterializationContractIdentity,
    record_path: &Path,
) -> Result<(), FsError> {
    if state.schema != expected.state_schema.schema
        || state.schema_sha256 != expected.state_schema.schema_sha256
        || state.schema.as_str() != MATERIALIZATION_STATE_SCHEMA
        || state.schema_version != MATERIALIZATION_STATE_SCHEMA_VERSION
        || state.schema_manifest_sha256 != expected.schema_registry.schema_manifest_sha256
        || state.schema_set_sha256 != expected.schema_registry.schema_set_sha256
    {
        return Err(invalid("unsupported record identity"));
    }
    let actual = state.recompute_digest().map_err(|source| {
        data_error("compute materialization-state digest", record_path, source)
    })?;
    if state.state_digest != actual {
        return Err(invalid("record self-digest does not match its content"));
    }
    let mut previous = None;
    for surface in &state.surfaces {
        if pale_core::paths::SurfaceTarget::parse(&surface.rel_path)
            .is_none_or(|target| !target.is_filesystem())
            || surface.rel_path == MATERIALIZATION_STATE_PATH
        {
            return Err(invalid(format!(
                "invalid owned surface {}",
                surface.rel_path
            )));
        }
        if previous.is_some_and(|path: &str| path >= surface.rel_path.as_str()) {
            return Err(invalid(
                "surface paths are not unique and canonically ordered",
            ));
        }
        previous = Some(surface.rel_path.as_str());
    }
    Ok(())
}

fn read_state(
    root: &ConfinedRoot,
    expected: &MaterializationContractIdentity,
) -> Result<Option<MaterializationStateV1>, FsError> {
    let location = root.location(MATERIALIZATION_STATE_PATH)?;
    let record_path = location.display_path();
    let Some(existing) = location.read_file(MATERIALIZATION_STATE_LIMIT)? else {
        return Ok(None);
    };
    let bytes = existing.bytes;
    let value = canon::parse_strict(&bytes)
        .map_err(|source| data_error("parse materialization state", &record_path, source))?;
    let state: MaterializationStateV1 = serde_json::from_value(value)
        .map_err(|source| data_error("decode materialization state", &record_path, source))?;
    validate_state_at(&state, expected, &record_path)?;
    let canonical = canon::to_canonical_string(&state)
        .map_err(|source| data_error("encode materialization state", &record_path, source))?;
    if canonical.as_bytes() != bytes {
        return Err(invalid("record bytes are not canonical"));
    }
    Ok(Some(state))
}

/// Derive the machine-local identity used to bind lineage to a canonical target root.
///
/// The digest covers the platform's encoded canonical path bytes. It identifies a
/// configured location, not filesystem object identity; mutation paths separately
/// retain and reverify a root handle.
pub fn target_root_sha256(root_canon: &Path) -> TargetRootDigest {
    TargetRootDigest::hash_bytes(root_canon.as_os_str().as_encoded_bytes())
}

/// Derive the independent lineage-record path for one canonical target location.
pub(crate) fn lineage_relative(target_root_canon: &Path) -> String {
    format!(
        "materialization-lineage/{}.json",
        target_root_sha256(target_root_canon)
    )
}

fn validate_lineage(
    lineage: &MaterializationLineageV1,
    expected: &MaterializationContractIdentity,
    record_path: &Path,
) -> Result<(), FsError> {
    if lineage.schema != expected.lineage_schema.schema
        || lineage.schema_sha256 != expected.lineage_schema.schema_sha256
        || lineage.schema.as_str() != MATERIALIZATION_LINEAGE_SCHEMA
        || lineage.schema_version != MATERIALIZATION_LINEAGE_SCHEMA_VERSION
        || lineage.schema_manifest_sha256 != expected.schema_registry.schema_manifest_sha256
        || lineage.schema_set_sha256 != expected.schema_registry.schema_set_sha256
    {
        return Err(invalid("unsupported lineage identity"));
    }
    if lineage.recompute_digest().map_err(|source| {
        data_error(
            "compute materialization-lineage digest",
            record_path,
            source,
        )
    })? != lineage.lineage_digest
    {
        return Err(invalid("lineage self-digest does not match its content"));
    }
    Ok(())
}

fn read_lineage(
    location: &ConfinedLocation,
    expected: &MaterializationContractIdentity,
) -> Result<Option<MaterializationLineageV1>, FsError> {
    let record_path = location.display_path();
    let Some(existing) = location.read_file(MATERIALIZATION_LINEAGE_LIMIT)? else {
        return Ok(None);
    };
    let bytes = existing.bytes;
    let value = canon::parse_strict(&bytes)
        .map_err(|source| data_error("parse materialization lineage", &record_path, source))?;
    let lineage: MaterializationLineageV1 = serde_json::from_value(value)
        .map_err(|source| data_error("decode materialization lineage", &record_path, source))?;
    validate_lineage(&lineage, expected, &record_path)?;
    let canonical = canon::to_canonical_string(&lineage)
        .map_err(|source| data_error("encode materialization lineage", &record_path, source))?;
    if canonical.as_bytes() != bytes {
        return Err(invalid("lineage bytes are not canonical"));
    }
    Ok(Some(lineage))
}

/// Pair repository state with its independent lineage after exact identity checks.
pub(crate) fn read_trusted_from_roots(
    target_root: &ConfinedRoot,
    lineage_root: &ConfinedRoot,
    repository_id: &LogicalRepositoryIdV1,
    expected: &MaterializationContractIdentity,
) -> Result<Option<TrustedMaterialization>, FsError> {
    target_root.verify_path_identity()?;
    lineage_root.verify_path_identity()?;
    let target_root_canon = target_root.canonical_path();
    let lineage_root_canon = lineage_root.canonical_path();
    if lineage_root_canon.starts_with(target_root_canon)
        || target_root_canon.starts_with(lineage_root_canon)
    {
        return Err(invalid(
            "machine-bound lineage root must be independent of the target repository",
        ));
    }
    let state = read_state(target_root, expected)?;
    let lineage = read_lineage(
        &lineage_root.location(&lineage_relative(target_root_canon))?,
        expected,
    )?;
    let trusted = match (state, lineage) {
        (None, None) => None,
        (Some(_), None) => Err(FsError::Collision(
            MATERIALIZATION_STATE_PATH.into(),
            "repository-local state has no machine-bound admitted lineage".into(),
        ))?,
        (None, Some(_)) => Err(FsError::Collision(
            MATERIALIZATION_STATE_PATH.into(),
            "machine-bound lineage exists but target state is missing".into(),
        ))?,
        (Some(state), Some(lineage)) => {
            if &state.repository_id != repository_id
                || &lineage.repository_id != repository_id
                || lineage.target_root_sha256 != target_root_sha256(target_root_canon)
                || lineage.state_digest != state.state_digest
                || lineage.profile_digest != state.profile_digest
                || lineage.instance_digest != state.instance_digest
                || lineage.manifest_digest != state.manifest_digest
                || lineage.schema_manifest_sha256 != state.schema_manifest_sha256
                || lineage.schema_set_sha256 != state.schema_set_sha256
                || lineage.one_shot_compilation_digest != state.one_shot_compilation_digest
            {
                return Err(FsError::Collision(
                    MATERIALIZATION_STATE_PATH.into(),
                    "target state does not match its machine-bound admitted lineage".into(),
                ));
            }
            Some(TrustedMaterialization { state, lineage })
        }
    };
    target_root.verify_path_identity()?;
    lineage_root.verify_path_identity()?;
    Ok(trusted)
}

/// Read target state only after matching it to an independent machine-bound
/// lineage anchor.
///
/// Both records are size-bounded, strictly decoded, canonically encoded, and
/// self-digest verified before their repository and instance identities are paired.
/// This read does not acquire the reconciliation lock and is not a mutation token.
///
/// # Errors
///
/// Rejects unsafe, overlapping, or changed roots; missing record pairs; malformed,
/// non-canonical, or self-inconsistent records; identity mismatches; and bounded
/// read failures.
pub fn read_materialization_state(
    target_root: &Path,
    lineage_root: &Path,
    repository_id: &LogicalRepositoryIdV1,
    expected: &MaterializationContractIdentity,
) -> Result<Option<MaterializationStateV1>, FsError> {
    let target_root = ConfinedRoot::open(target_root)?;
    let lineage_root = ConfinedRoot::open(lineage_root)?;
    Ok(
        read_trusted_from_roots(&target_root, &lineage_root, repository_id, expected)?
            .map(|trusted| trusted.state),
    )
}
