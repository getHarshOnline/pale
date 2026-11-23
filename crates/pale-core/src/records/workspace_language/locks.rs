//! Exact resolution and universe-lock coordinates for workspace admission.
//!
//! These records bind desired immutable frontiers; their presence alone does
//! not prove publication, freshness, installation, or runtime activation.

use super::*;

/// Exact release resolved for one imported module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceResolvedReleaseV1 {
    /// Module identity.
    pub module_id: WorkspaceModuleId,
    /// Exact release identity.
    pub release_id: WorkspaceReleaseId,
    /// Exact module content identity.
    pub module_digest: WorkspaceModuleDigest,
    /// Exact publisher reference.
    pub publisher: WorkspacePublisherRefV1,
    /// Claimed publisher state at this resolution; see [`WorkspaceReleaseStateV1`].
    pub release_state: WorkspaceReleaseStateV1,
}

/// Exact module-release resolution frontier, distinct from adoption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceResolutionLockV1 {
    /// Stable wire schema identity.
    #[schemars(extend("const" = "pale/workspace-resolution-lock.v1"))]
    pub schema: String,
    /// Initial contract version.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Resolved releases in stable logical identity order.
    #[schemars(schema_with = "resolved_releases_schema")]
    pub releases: Vec<WorkspaceResolvedReleaseV1>,
    /// Domain-separated canonical identity of `releases`.
    pub resolution_digest: WorkspaceResolutionLockDigest,
}

/// A logical lock set cannot be sealed canonically.
#[derive(Debug, thiserror::Error)]
pub enum InvalidWorkspaceLock {
    /// The lock's schema identity or initial-v1 version differs.
    #[error("workspace {kind} lock has an invalid schema coordinate")]
    SchemaCoordinate {
        /// Closed lock family being verified.
        kind: &'static str,
    },
    /// Canonical encoding failed.
    #[error("workspace lock canonicalization failed: {0}")]
    Canonical(#[source] crate::canon::CanonError),
    /// A release identity appears more than once.
    #[error("workspace lock repeats module identity `{0}`")]
    DuplicateModule(WorkspaceModuleId),
    /// A retained release or adoption vector is not strictly ordered.
    #[error("workspace {kind} lock entries are not canonically ordered")]
    NonCanonical {
        /// Closed lock family being verified.
        kind: &'static str,
    },
    /// A lock vector exceeds the shared module ceiling.
    #[error("workspace lock {kind} count {observed} exceeds {limit}")]
    Limit {
        /// Closed lock vector whose bound was exceeded.
        kind: &'static str,
        /// Observed element count.
        observed: usize,
        /// Maximum admitted element count.
        limit: usize,
    },
    /// Claimed and recomputed lock identities differ.
    #[error("workspace {kind} lock digest does not match its canonical preimage")]
    DigestMismatch {
        /// Closed lock family being verified.
        kind: &'static str,
    },
}

impl WorkspaceResolutionLockV1 {
    /// Seal one exact release-resolution frontier.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceLock`] when the module bound is exceeded, an
    /// identity is duplicated, or the canonical release set cannot be encoded.
    pub fn seal(
        mut releases: Vec<WorkspaceResolvedReleaseV1>,
    ) -> Result<Self, InvalidWorkspaceLock> {
        if releases.len() > MAX_WORKSPACE_MODULES {
            return Err(InvalidWorkspaceLock::Limit {
                kind: "resolved release",
                observed: releases.len(),
                limit: MAX_WORKSPACE_MODULES,
            });
        }
        releases.sort_by(|left, right| left.module_id.cmp(&right.module_id));
        if let Some(module_id) =
            super::derivation::adjacent_duplicate(releases.iter().map(|release| &release.module_id))
        {
            return Err(InvalidWorkspaceLock::DuplicateModule(module_id.clone()));
        }
        let resolution_digest = WorkspaceResolutionLockDigest::hash_canonical(&(
            "pale.workspace-resolution-lock.v1",
            &releases,
        ))
        .map_err(InvalidWorkspaceLock::Canonical)?;
        Ok(Self {
            schema: "pale/workspace-resolution-lock.v1".into(),
            schema_version: 1,
            releases,
            resolution_digest,
        })
    }

    /// Verify the retained release set against its domain-separated identity.
    ///
    /// This preimage check assumes the record already passed wire/schema admission.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceLock`] when the schema coordinate, collection
    /// bound, canonical order, digest, or canonical encoding differs.
    pub fn verify(&self) -> Result<(), InvalidWorkspaceLock> {
        if self.schema != "pale/workspace-resolution-lock.v1" || self.schema_version != 1 {
            return Err(InvalidWorkspaceLock::SchemaCoordinate { kind: "resolution" });
        }
        if self.releases.len() > MAX_WORKSPACE_MODULES {
            return Err(InvalidWorkspaceLock::Limit {
                kind: "resolved release",
                observed: self.releases.len(),
                limit: MAX_WORKSPACE_MODULES,
            });
        }
        if !super::derivation::strictly_ordered(
            self.releases.iter().map(|release| &release.module_id),
        ) {
            return Err(InvalidWorkspaceLock::NonCanonical { kind: "resolution" });
        }
        let observed = WorkspaceResolutionLockDigest::hash_canonical(&(
            "pale.workspace-resolution-lock.v1",
            &self.releases,
        ))
        .map_err(InvalidWorkspaceLock::Canonical)?;
        if observed != self.resolution_digest {
            return Err(InvalidWorkspaceLock::DigestMismatch { kind: "resolution" });
        }
        Ok(())
    }
}

/// One exact release adopted into a principal-relative universe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceAdoptedReleaseV1 {
    /// Module identity.
    pub module_id: WorkspaceModuleId,
    /// Exact release identity.
    pub release_id: WorkspaceReleaseId,
    /// Exact module digest.
    pub module_digest: WorkspaceModuleDigest,
}

/// Explicitly selected module/import closure for one sovereign workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceUniverseLockV1 {
    /// Stable wire schema identity.
    #[schemars(extend("const" = "pale/workspace-universe-lock.v1"))]
    pub schema: String,
    /// Initial contract version.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Principal-owned workspace identity.
    pub workspace_id: WorkspaceId,
    /// Exact resolution lock consumed by this adoption frontier.
    pub resolution_digest: WorkspaceResolutionLockDigest,
    /// Explicitly adopted releases; an empty list is a valid sovereign root.
    #[schemars(schema_with = "adopted_releases_schema")]
    pub adopted_releases: Vec<WorkspaceAdoptedReleaseV1>,
    /// Domain-separated canonical universe-lock identity.
    pub universe_digest: WorkspaceUniverseLockDigest,
}

impl WorkspaceUniverseLockV1 {
    /// Seal one explicit principal-relative adoption frontier.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceLock`] when the module bound is exceeded, an
    /// identity is duplicated, or the canonical adoption set cannot be encoded.
    pub fn seal(
        workspace_id: WorkspaceId,
        resolution_digest: WorkspaceResolutionLockDigest,
        mut adopted_releases: Vec<WorkspaceAdoptedReleaseV1>,
    ) -> Result<Self, InvalidWorkspaceLock> {
        if adopted_releases.len() > MAX_WORKSPACE_MODULES {
            return Err(InvalidWorkspaceLock::Limit {
                kind: "adopted release",
                observed: adopted_releases.len(),
                limit: MAX_WORKSPACE_MODULES,
            });
        }
        adopted_releases.sort_by(|left, right| left.module_id.cmp(&right.module_id));
        if let Some(module_id) = super::derivation::adjacent_duplicate(
            adopted_releases.iter().map(|release| &release.module_id),
        ) {
            return Err(InvalidWorkspaceLock::DuplicateModule(module_id.clone()));
        }
        let universe_digest = WorkspaceUniverseLockDigest::hash_canonical(&(
            "pale.workspace-universe-lock.v1",
            &workspace_id,
            &resolution_digest,
            &adopted_releases,
        ))
        .map_err(InvalidWorkspaceLock::Canonical)?;
        Ok(Self {
            schema: "pale/workspace-universe-lock.v1".into(),
            schema_version: 1,
            workspace_id,
            resolution_digest,
            adopted_releases,
            universe_digest,
        })
    }

    /// Verify the retained adoption frontier against its identity.
    ///
    /// This preimage check assumes the record already passed wire/schema admission.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceLock`] when the schema coordinate, collection
    /// bound, canonical order, digest, or canonical encoding differs.
    pub fn verify(&self) -> Result<(), InvalidWorkspaceLock> {
        if self.schema != "pale/workspace-universe-lock.v1" || self.schema_version != 1 {
            return Err(InvalidWorkspaceLock::SchemaCoordinate { kind: "universe" });
        }
        if self.adopted_releases.len() > MAX_WORKSPACE_MODULES {
            return Err(InvalidWorkspaceLock::Limit {
                kind: "adopted release",
                observed: self.adopted_releases.len(),
                limit: MAX_WORKSPACE_MODULES,
            });
        }
        if !super::derivation::strictly_ordered(
            self.adopted_releases
                .iter()
                .map(|release| &release.module_id),
        ) {
            return Err(InvalidWorkspaceLock::NonCanonical { kind: "universe" });
        }
        let observed = WorkspaceUniverseLockDigest::hash_canonical(&(
            "pale.workspace-universe-lock.v1",
            &self.workspace_id,
            &self.resolution_digest,
            &self.adopted_releases,
        ))
        .map_err(InvalidWorkspaceLock::Canonical)?;
        if observed != self.universe_digest {
            return Err(InvalidWorkspaceLock::DigestMismatch { kind: "universe" });
        }
        Ok(())
    }
}
