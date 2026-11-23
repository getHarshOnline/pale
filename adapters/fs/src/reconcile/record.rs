//! Staging for the target ownership record and its independent lineage anchor.
//!
//! Neither record is trusted alone. Their canonical bytes and identities move in
//! the same in-process recovery set, while the lineage remains outside the target
//! repository.

use std::borrow::Cow;
use std::path::Path;

use pale_core::records::{
    AgentSurfacePlanV1, ArtifactDigest, MaterializationLineageDigest,
    MaterializationLineagePreimageV1, MaterializationLineageV1, PlanDigest, RecordSchemaIdentity,
    SurfaceAction, MATERIALIZATION_LINEAGE_SCHEMA_VERSION,
};

use super::{Mutation, Staged};
use crate::confine::{ConfinedLocation, ReadLimit};
use crate::errors::{data_error, FsError};
use crate::materialization_state::{lineage_relative, target_root_sha256};

/// Existing authenticated internal record before transition classification.
struct ExistingRecord {
    bytes: Vec<u8>,
    mode: u32,
}

/// Maximum bytes admitted for an internal state or lineage record.
const INTERNAL_RECORD_LIMIT: ReadLimit = ReadLimit::new(1 << 20);
/// Owner-only mode required for internal state and lineage records.
const INTERNAL_RECORD_MODE: u32 = 0o600;

/// Stage one authenticated internal-record transition, if it has an effect.
pub(super) fn stage(
    location: ConfinedLocation,
    receipt_path: &str,
    prior_bytes: Option<&[u8]>,
    next_bytes: Option<Vec<u8>>,
    record_receipt: bool,
) -> Result<Option<Staged<'static>>, FsError> {
    let existing = inspect(&location, receipt_path, prior_bytes)?;
    let (action, mutation, post_sha256) = match (existing.as_ref(), next_bytes) {
        (None, None) => return Ok(None),
        (None, Some(bytes)) => {
            let post = ArtifactDigest::hash_bytes(&bytes);
            (
                SurfaceAction::Create,
                Mutation::Write(Cow::Owned(bytes)),
                Some(post),
            )
        }
        (Some(_), None) => (SurfaceAction::Delete, Mutation::Delete, None),
        (Some(current), Some(bytes))
            if current.bytes == bytes && current.mode == INTERNAL_RECORD_MODE =>
        {
            (
                SurfaceAction::Unchanged,
                Mutation::Unchanged,
                Some(ArtifactDigest::hash_bytes(&current.bytes)),
            )
        }
        (Some(_), Some(bytes)) => {
            let post = ArtifactDigest::hash_bytes(&bytes);
            (
                SurfaceAction::Update,
                Mutation::Write(Cow::Owned(bytes)),
                Some(post),
            )
        }
    };
    Ok(Some(Staged {
        location,
        rel_path: receipt_path.into(),
        action,
        mutation,
        pre_sha256: existing
            .as_ref()
            .map(|record| ArtifactDigest::hash_bytes(&record.bytes)),
        pre_mode: existing.as_ref().map(|record| record.mode),
        post_sha256,
        mode: INTERNAL_RECORD_MODE,
        record_receipt,
    }))
}

fn inspect(
    location: &ConfinedLocation,
    record_name: &str,
    expected_bytes: Option<&[u8]>,
) -> Result<Option<ExistingRecord>, FsError> {
    let existing = location.read_file(INTERNAL_RECORD_LIMIT)?;
    if existing.is_some() != expected_bytes.is_some() {
        return Err(FsError::Collision(
            record_name.into(),
            "internal record existence changed after ownership verification".into(),
        ));
    }
    let Some(existing) = existing else {
        return Ok(None);
    };
    if expected_bytes != Some(existing.bytes.as_slice()) {
        return Err(FsError::Collision(
            record_name.into(),
            "internal record bytes changed after ownership verification".into(),
        ));
    }
    Ok(Some(ExistingRecord {
        bytes: existing.bytes,
        mode: existing.mode,
    }))
}

/// Build canonical next-lineage bytes and their self-digest for an apply plan.
pub(super) fn next_lineage(
    plan: &AgentSurfacePlanV1,
    admitted_plan_digest: &PlanDigest,
    target_root_canon: &Path,
    lineage_schema_identity: &RecordSchemaIdentity,
) -> Result<Option<(Vec<u8>, MaterializationLineageDigest)>, FsError> {
    let Some(state) = plan.transition.next_state() else {
        return Ok(None);
    };
    let record_path = lineage_relative(target_root_canon);
    let lineage = MaterializationLineageV1::seal(MaterializationLineagePreimageV1 {
        schema: lineage_schema_identity.schema.clone(),
        schema_sha256: lineage_schema_identity.schema_sha256.clone(),
        schema_version: MATERIALIZATION_LINEAGE_SCHEMA_VERSION,
        schema_manifest_sha256: plan.schema_manifest_sha256.clone(),
        schema_set_sha256: plan.schema_set_sha256.clone(),
        one_shot_compilation_digest: plan.one_shot_compilation_digest.clone(),
        repository_id: plan.repository_id.clone(),
        target_root_sha256: target_root_sha256(target_root_canon),
        profile_digest: state.profile_digest.clone(),
        instance_digest: state.instance_digest.clone(),
        manifest_digest: state.manifest_digest.clone(),
        state_digest: state.state_digest.clone(),
        admitted_plan_digest: admitted_plan_digest.clone(),
    })
    .map_err(|source| data_error("compute next lineage digest", &record_path, source))?;
    let digest = lineage.lineage_digest.clone();
    let bytes = pale_core::canon::to_canonical_string(&lineage)
        .map_err(|source| data_error("encode next lineage", &record_path, source))?
        .into_bytes();
    Ok(Some((bytes, digest)))
}

#[cfg(all(test, unix))]
#[path = "../../tests/internal/reconcile/record.rs"]
mod tests;
