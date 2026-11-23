//! Reviewed proposal equality and the opaque native effect handoff.

use super::{ArtifactDigest, SourceGuidancePlan};
use pale_ports::{AdmittedSourceGuidanceEffect, SourceGuidanceEffectFile};
use std::collections::BTreeMap;

/// A reviewed source proposal no longer identifies the freshly rebuilt plan.
#[derive(Debug, thiserror::Error)]
pub enum GuidanceAdmissionError {
    /// Source selection, baseline, identity or publication order changed.
    #[error("reviewed source-guidance plan differs from the current proposal")]
    ReviewedPlanChanged,
}

impl GuidanceAdmissionError {
    /// The caller must review current state; no write is admitted by this error.
    #[must_use]
    pub const fn failure_disposition(&self) -> crate::ApplicationFailureDisposition {
        crate::ApplicationFailureDisposition::Blocked
    }
}

pub(super) fn admit(
    mut plan: SourceGuidancePlan,
    reviewed: &ArtifactDigest,
) -> Result<AdmittedSourceGuidanceEffect, GuidanceAdmissionError> {
    if &plan.digest != reviewed {
        return Err(GuidanceAdmissionError::ReviewedPlanChanged);
    }
    let order = plan
        .publication_order
        .iter()
        .enumerate()
        .map(|(i, path)| (path.clone(), i))
        .collect::<BTreeMap<_, _>>();
    plan.files.sort_by(|a, b| {
        order
            .get(a.rel_path())
            .unwrap_or(&usize::MAX)
            .cmp(order.get(b.rel_path()).unwrap_or(&usize::MAX))
            .then_with(|| a.rel_path().cmp(b.rel_path()))
    });
    let files = plan
        .files
        .into_iter()
        .map(|file| {
            SourceGuidanceEffectFile::from_application_admission(
                file.rel_path,
                file.baseline,
                file.change,
            )
        })
        .collect();
    Ok(AdmittedSourceGuidanceEffect::from_application_admission(
        plan.root,
        plan.digest,
        plan.owner_sha256,
        files,
    ))
}
