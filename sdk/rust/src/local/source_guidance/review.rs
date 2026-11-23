//! Remember a displayed proposal without retaining private file bodies.

use super::*;

const LIMIT: ReadLimit = ReadLimit::new(1024);

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Review {
    format: String,
    digest: ArtifactDigest,
}

fn path(host: SourceGuidanceHost) -> &'static str {
    match host {
        SourceGuidanceHost::Codex => ".pale-source-guidance-review-codex.json",
        SourceGuidanceHost::ClaudeCode => ".pale-source-guidance-review-claude-code.json",
    }
}

fn root(plan: &LocalSourceGuidancePlan) -> Result<ConfinedRoot, FsError> {
    let root = ConfinedRoot::open(Path::new(&plan.plan.root().canonical_path))?;
    if observe_machine_local_file(&root, path(plan.report.host), LIMIT)?.root != *plan.plan.root() {
        return Err(FsError::InvalidState(
            "guidance review repository changed".into(),
        ));
    }
    Ok(root)
}

fn decode(bytes: &[u8]) -> Result<Review, FsError> {
    let review: Review = serde_json::from_slice(bytes).map_err(|_| {
        FsError::InvalidState("guidance review is malformed; run guidance plan again".into())
    })?;
    if review.format != "pale-guidance-review-v1" {
        return Err(FsError::InvalidState(
            "guidance review format is unknown".into(),
        ));
    }
    Ok(review)
}

pub(super) fn read(
    api: &PaleApi,
    plan: &LocalSourceGuidancePlan,
) -> Result<ArtifactDigest, ApiError> {
    let result = (|| {
        let root = root(plan)?;
        let record = root
            .location(path(plan.report.host))?
            .read_file(LIMIT)?
            .ok_or_else(|| {
                FsError::InvalidState("no remembered guidance plan; run guidance plan first".into())
            })?;
        if record.mode != 0o600 {
            return Err(FsError::InvalidState(
                "guidance review permissions changed".into(),
            ));
        }
        Ok(decode(&record.bytes)?.digest)
    })();
    result.map_err(|error| effect::admission_failure(api, error, ApplicationFailureDisposition::Blocked, "source_guidance_review_missing_or_invalid", "no usable remembered guidance plan; run guidance plan for this repository and host first"))
}

impl PaleApi {
    /// Remember a proposal after displaying it, for a later digest-free apply.
    ///
    /// Saves only its identity in an owner-readable root file for the selected
    /// host. Instruction files remain unchanged. A later apply still rebuilds
    /// the plan and checks every source and target baseline. This record does
    /// not establish that a person read or approved the proposal.
    ///
    /// # Errors
    /// Refuses changed targets, unsafe review paths and malformed existing
    /// records. An interrupted save may require another plan; it never applies
    /// guidance. Native write uncertainty is reported separately.
    pub fn remember_local_source_guidance_plan(
        &self,
        plan: &LocalSourceGuidancePlan,
    ) -> Result<(), ApiError> {
        let result = (|| {
            let root = root(plan)?;
            for file in plan.plan.files() {
                if observe_machine_local_file(
                    &root,
                    file.rel_path(),
                    ReadLimit::new(MAX_GUIDANCE_FILE_BYTES),
                )? != *file.baseline()
                {
                    return Err(FsError::Collision(
                        file.rel_path().into(),
                        "guidance target changed while displaying the plan; plan again".into(),
                    ));
                }
            }
            let path = path(plan.report.host);
            let location = root.location(path)?;
            let before = observe_machine_local_file(&root, path, LIMIT)?;
            if let Some(bytes) = &before.bytes {
                decode(bytes)?;
            }
            let bytes = serde_json::to_vec(&Review {
                format: "pale-guidance-review-v1".into(),
                digest: plan.plan.digest().clone(),
            })
            .map_err(|_| FsError::InvalidState("cannot encode guidance review".into()))?;
            if observe_machine_local_file(&root, path, LIMIT)? != before {
                return Err(FsError::Collision(
                    path.into(),
                    "guidance review changed during save".into(),
                ));
            }
            if before.prior.is_some() {
                location.write_atomic(&bytes, 0o600)?;
            } else {
                location.write_atomic_noclobber(&bytes, 0o600)?;
            }
            let read_back = (|| {
                let after = location.read_file(LIMIT)?.ok_or_else(|| {
                    FsError::InvalidState("guidance review disappeared after save".into())
                })?;
                if after.bytes != bytes || after.mode != 0o600 {
                    return Err(FsError::InvalidState(
                        "guidance review read-back differs".into(),
                    ));
                }
                Ok(())
            })();
            read_back.map_err(|source| FsError::EffectUncertain {
                operation: "save guidance review",
                path: location.display_path(),
                source: Box::new(source),
            })?;
            Ok(())
        })();
        result.map_err(|error| {
            effect::native_failure(
                self,
                error,
                effect::NativeBoundary::Preparation,
                "source_guidance_review_save_failed",
            )
        })
    }
}
