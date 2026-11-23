//! Ownership-aware source-guidance reconciliation, without runtime admission.
//!
//! Plans bind exact source projections, native roots, file baselines and parent
//! identities. They are proposals, not effect capabilities. The native writer
//! must rebuild/admit a reviewed plan and reobserve every effect boundary.
//! Ownership metadata is private reconciliation state for cooperating writers;
//! it is neither authenticated authority nor a portable runtime manifest.
//! This module constructs proposals only; native publication and recovery are
//! separate responsibilities and are not established by a successful plan.

use std::collections::BTreeMap;

use pale_core::records::{ArtifactDigest, MachineLocalRootIdentity, UnixFileMode};
use pale_ports::MachineLocalFileObservation;

use crate::{
    compile::EffectiveProfileCompilation,
    render::{SourceGuidanceHost, SourceGuidanceProjection},
};

mod admission;
mod entrypoint;
mod excludes;
mod ownership;
mod planning;
mod recovery;

pub use admission::GuidanceAdmissionError;
pub use pale_ports::SourceGuidanceFileChange as GuidanceFileChange;
pub use recovery::GuidanceRecoveryAdmissionError;

/// Maximum bytes in a private ownership record.
pub const MAX_OWNERSHIP_BYTES: usize = 1024 * 1024;
/// Maximum bytes observed or proposed for one guidance file or entrypoint.
pub const MAX_GUIDANCE_FILE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum whole files in one host's ownership record.
pub const MAX_GUIDANCE_FILES: usize = pale_ports::MAX_SOURCE_GUIDANCE_OWNED_FILES;
/// Maximum observed plus proposed payload bytes admitted by one complete plan.
pub const MAX_GUIDANCE_PLAN_BYTES: usize = 64 * 1024 * 1024;

/// Explicit local reconciliation intent, separate from runtime plan classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuidancePlanIntent {
    /// Reconcile the selected methods, removing superseded owned files.
    Apply,
    /// Remove only this host's owned files and exact instruction contribution.
    Remove,
}

/// One proposed change paired with its full native observation.
pub struct GuidancePlannedFile {
    rel_path: String,
    baseline: MachineLocalFileObservation,
    change: GuidanceFileChange,
}

impl GuidancePlannedFile {
    /// Exact confined destination beneath the plan's retained root.
    #[must_use]
    pub fn rel_path(&self) -> &str {
        &self.rel_path
    }

    /// Pre-write bytes, metadata and parent partition. Not mutation authority.
    #[must_use]
    pub fn baseline(&self) -> &MachineLocalFileObservation {
        &self.baseline
    }

    /// Proposed effect; `Keep` remains subject to fresh read-back.
    #[must_use]
    pub fn change(&self) -> &GuidanceFileChange {
        &self.change
    }
}

/// Complete immutable proposal, deliberately incompatible with runtime plans.
pub struct SourceGuidancePlan {
    root: MachineLocalRootIdentity,
    intent: GuidancePlanIntent,
    digest: ArtifactDigest,
    owner_sha256: ArtifactDigest,
    ownership_path: String,
    files: Vec<GuidancePlannedFile>,
    publication_order: Vec<String>,
}

impl SourceGuidancePlan {
    /// Native target directory identity, distinct from logical repository identity.
    #[must_use]
    pub fn root(&self) -> &MachineLocalRootIdentity {
        &self.root
    }

    /// Exact local reconciliation intent.
    #[must_use]
    pub fn intent(&self) -> GuidancePlanIntent {
        self.intent
    }

    /// Domain-separated content/baseline identity; never an invocation grant.
    #[must_use]
    pub fn digest(&self) -> &ArtifactDigest {
        &self.digest
    }

    /// Private metadata destination, to be published after the method files.
    #[must_use]
    pub fn ownership_path(&self) -> &str {
        &self.ownership_path
    }

    /// Deterministically ordered proposals, including entrypoint and ownership state.
    #[must_use]
    pub fn files(&self) -> &[GuidancePlannedFile] {
        &self.files
    }

    /// Changed targets in exact publication order, covered by the plan digest.
    #[must_use]
    pub fn publication_order(&self) -> &[String] {
        &self.publication_order
    }

    /// Admit only this exact reviewed proposal for the native filesystem handoff.
    ///
    /// This is not a runtime grant. The SDK must rebuild from fresh sources and
    /// the adapter must reobserve all native baselines before any target write.
    ///
    /// # Errors
    /// Rejects a different reviewed plan digest without creating an effect capability.
    pub fn admit(
        self,
        reviewed: &ArtifactDigest,
    ) -> Result<pale_ports::AdmittedSourceGuidanceEffect, GuidanceAdmissionError> {
        admission::admit(self, reviewed)
    }
}

/// A planning refusal before any filesystem mutation.
#[derive(Debug, thiserror::Error)]
pub enum GuidancePlanError {
    /// The source profile cannot project the requested methods.
    #[error(transparent)]
    Projection(#[from] crate::render::SourceGuidanceError),
    /// The selected context permits observation only.
    #[error("selected repository does not permit source-guidance mutation")]
    ReadOnly,
    /// Native observation contradicts the selected checkout or other observations.
    #[error("source-guidance baseline is invalid: {0}")]
    Baseline(&'static str),
    /// Private ownership state is malformed or belongs to a different owner/root.
    #[error("source-guidance ownership is invalid: {0}")]
    Ownership(&'static str),
    /// Reconciliation would take over an unowned path or edited owned content.
    #[error("source-guidance collision at `{path}`: {reason}")]
    Collision {
        /// Exact selected repository-relative path.
        path: String,
        /// Bounded explanation without file content.
        reason: &'static str,
    },
    /// A bounded source/file inventory exceeds its resource budget.
    #[error("source-guidance {0} limit exceeded")]
    Limit(&'static str),
    /// An exact plan identity could not be encoded.
    #[error(transparent)]
    Canonical(#[from] pale_core::canon::CanonError),
    /// Supplied private ownership bytes could not be decoded.
    #[error("source-guidance ownership decoding rejected")]
    Decode(#[source] serde_json::Error),
    /// Compiler-proposed private ownership could not be encoded.
    #[error("source-guidance ownership encoding rejected")]
    Encode(#[source] serde_json::Error),
}

impl GuidancePlanError {
    /// Application-owned diagnostic consequence, without an executed effect.
    #[must_use]
    pub const fn failure_disposition(&self) -> crate::ApplicationFailureDisposition {
        use crate::ApplicationFailureDisposition::{Blocked, Internal, InvalidInput};
        match self {
            Self::Projection(source) => source.failure_disposition(),
            Self::ReadOnly | Self::Collision { .. } => Blocked,
            Self::Baseline(_) | Self::Ownership(_) | Self::Limit(_) | Self::Decode(_) => {
                InvalidInput
            }
            Self::Canonical(_) | Self::Encode(_) => Internal,
        }
    }
}

/// Retained first phase after private ownership admission, before target reads.
pub struct SourceGuidancePreparation {
    context: ownership::Owner,
    profile_digest: pale_core::records::ProfileDigest,
    root: MachineLocalRootIdentity,
    host: SourceGuidanceHost,
    requested: RequestedChange,
    state: Option<ownership::Ownership>,
    state_observation: MachineLocalFileObservation,
}

enum RequestedChange {
    Apply(SourceGuidanceProjection),
    Remove,
}

/// Private native-state location for one host, without reading it.
#[must_use]
pub fn ownership_path(host: SourceGuidanceHost) -> String {
    format!(
        ".pale/source-guidance-state/{}.json",
        host.adapter().as_str()
    )
}

/// Start write-free planning from the exact selected profile and state observation.
///
/// # Errors
///
/// Rejects an unselected/read-only repository, mismatched native root,
/// inconsistent state observation, or malformed/cross-owner private metadata.
pub fn prepare_source_guidance(
    compilation: &EffectiveProfileCompilation,
    repository: &pale_core::records::LogicalRepositoryIdV1,
    host: SourceGuidanceHost,
    intent: GuidancePlanIntent,
    state_observation: MachineLocalFileObservation,
) -> Result<SourceGuidancePreparation, GuidancePlanError> {
    ownership::prepare(compilation, repository, host, intent, state_observation)
}

impl SourceGuidancePreparation {
    fn entry_path(&self) -> &'static str {
        self.state.as_ref().map_or_else(
            || self.host.adapter().projection_layout().front_door(),
            |state| state.entry_path(self.host),
        )
    }

    /// Exact remaining paths the filesystem owner must observe, excluding state.
    #[must_use]
    pub fn required_paths(&self) -> Vec<String> {
        planning::required_paths(self)
    }

    /// Complete a proposal from exactly the requested native observations.
    ///
    /// # Errors
    ///
    /// Rejects missing/extra/invalid baselines, file or entrypoint collisions,
    /// changed ownership, and resource-limit violations. This performs no writes.
    pub fn finish(
        self,
        observations: BTreeMap<String, MachineLocalFileObservation>,
    ) -> Result<SourceGuidancePlan, GuidancePlanError> {
        planning::finish(self, observations)
    }
}
