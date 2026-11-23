//! Logical scope admission for native recovery, without decoding native journals.

use super::{ownership, EffectiveProfileCompilation, GuidancePlanError, SourceGuidanceHost};
use pale_core::records::{
    ArtifactDigest, ExecutionWorkspaceRepositoryAccessV1, LogicalRepositoryIdV1,
};
use pale_ports::{AdmittedSourceGuidanceRecovery, SourceGuidanceRecoveryProposal};
use std::collections::BTreeSet;

/// A recovery claim does not match its selected owner or exact reviewed state.
#[derive(Debug, thiserror::Error)]
pub enum GuidanceRecoveryAdmissionError {
    /// Source-owner selection or canonical identity could not be established.
    #[error(transparent)]
    Scope(#[from] GuidancePlanError),
    /// Native observations or the selected recovery decision changed.
    #[error("reviewed source-guidance recovery differs from the current proposal")]
    ReviewedPlanChanged,
    /// The originating logical owner/host differs from the selected profile.
    #[error("source-guidance recovery belongs to a different owner or host")]
    OwnerMismatch,
    /// A recovery claim would escape the selected host's guidance ownership.
    #[error("source-guidance recovery target is outside the selected ownership scope")]
    TargetScope,
}

impl GuidanceRecoveryAdmissionError {
    /// Stable consequence; no native effect is admitted by an error.
    #[must_use]
    pub const fn failure_disposition(&self) -> crate::ApplicationFailureDisposition {
        match self {
            Self::Scope(error) => error.failure_disposition(),
            Self::ReviewedPlanChanged => crate::ApplicationFailureDisposition::Blocked,
            Self::OwnerMismatch | Self::TargetScope => {
                crate::ApplicationFailureDisposition::InvalidInput
            }
        }
    }
}

impl EffectiveProfileCompilation {
    /// Admit the native owner's recovery proposal against current logical scope.
    ///
    /// Does not project a role or require a runtime execution path. Current owner,
    /// selected mutable repository, canonical binding and exact reviewed recovery
    /// digest remain required. Native journal validation and fresh observation
    /// checks belong to the filesystem adapter.
    ///
    /// # Errors
    /// Rejects an unselected/read-only root, another owner/host, changed review,
    /// out-of-scope paths or directory claims unrelated to admitted file paths.
    pub fn admit_source_guidance_recovery(
        &self,
        repository: &LogicalRepositoryIdV1,
        host: SourceGuidanceHost,
        proposal: &SourceGuidanceRecoveryProposal,
        reviewed: &ArtifactDigest,
    ) -> Result<AdmittedSourceGuidanceRecovery, GuidanceRecoveryAdmissionError> {
        if proposal.digest() != reviewed {
            return Err(GuidanceRecoveryAdmissionError::ReviewedPlanChanged);
        }
        self.validate_source_guidance_recovery_scope(repository, host, proposal)?;
        Ok(AdmittedSourceGuidanceRecovery::from_application_admission(
            proposal.clone(),
        ))
    }
    /// Validate recovery ownership and target scope without minting an effect capability.
    ///
    /// # Errors
    /// Rejects an unselected/read-only or mismatched root, owner/host and target scope.
    pub fn validate_source_guidance_recovery_scope(
        &self,
        repository: &LogicalRepositoryIdV1,
        host: SourceGuidanceHost,
        proposal: &SourceGuidanceRecoveryProposal,
    ) -> Result<(), GuidanceRecoveryAdmissionError> {
        let profile = self.profile();
        let selected = profile
            .repositories
            .iter()
            .find(|r| &r.id == repository)
            .ok_or_else(|| {
                GuidancePlanError::Projection(
                    crate::render::SourceGuidanceError::RepositoryNotSelected,
                )
            })?;
        if selected.access != ExecutionWorkspaceRepositoryAccessV1::Mutate {
            return Err(GuidancePlanError::ReadOnly.into());
        }
        if self.instance().repositories.get(repository) != Some(&proposal.root().canonical_path) {
            return Err(GuidancePlanError::Baseline(
                "selected checkout differs from the recovery root",
            )
            .into());
        }
        let owner = ownership::Owner {
            workspace: profile.workspace_id.clone(),
            project: profile.project_id.clone(),
            execution_workspace: profile.execution_workspace_id.clone(),
            repository: repository.clone(),
            host,
        };
        if &ownership::identity_digest("owner", &owner)? != proposal.owner_sha256() {
            return Err(GuidanceRecoveryAdmissionError::OwnerMismatch);
        }
        let entry = host.adapter().projection_layout().front_door();
        let state = super::ownership_path(host);
        let files = proposal.files().iter().collect::<BTreeSet<_>>();
        if files.is_empty()
            || files.len() != proposal.files().len()
            || files.len() > pale_ports::MAX_SOURCE_GUIDANCE_TRANSACTION_FILES
            || files.iter().any(|p| {
                !ownership::owned_path(host, p)
                    && p.as_str() != entry
                    && !(host == SourceGuidanceHost::ClaudeCode && p.as_str() == "CLAUDE.md")
                    && p.as_str() != state
                    && p.as_str() != super::excludes::PATH
            })
        {
            return Err(GuidanceRecoveryAdmissionError::TargetScope);
        }
        let dirs = proposal.directories().iter().collect::<BTreeSet<_>>();
        if dirs.len() != proposal.directories().len()
            || dirs.iter().any(|d| {
                pale_core::paths::ConfinedRelativePath::parse(d).is_none()
                    || !files.iter().any(|p| p.starts_with(&format!("{d}/")))
            })
        {
            return Err(GuidanceRecoveryAdmissionError::TargetScope);
        }
        Ok(())
    }
}
