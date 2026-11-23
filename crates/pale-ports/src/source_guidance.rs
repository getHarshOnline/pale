//! Opaque source-guidance effect handoff, independent of runtime plans.

use crate::MachineLocalFileObservation;
use pale_core::records::{ArtifactDigest, MachineLocalRootIdentity, UnixFileMode};

/// Maximum whole files in one host's guidance ownership inventory.
pub const MAX_SOURCE_GUIDANCE_OWNED_FILES: usize = 4096;
/// Maximum transaction inventory: disjoint old and new owned files, plus the
/// host entrypoint, ownership record and shared Git exclusions.
pub const MAX_SOURCE_GUIDANCE_TRANSACTION_FILES: usize = 2 * MAX_SOURCE_GUIDANCE_OWNED_FILES + 3;

/// Closed proposed/admitted file intent; data alone grants no write authority.
#[derive(Clone, PartialEq, Eq)]
pub enum SourceGuidanceFileChange {
    /// Preserve the exact observed bytes, permissions or absence.
    Keep,
    /// Publish these exact UTF-8 bytes and permissions.
    Write {
        /// Complete proposed file content, possibly including native project text.
        content: String,
        /// Desired Unix permission bits.
        mode: UnixFileMode,
    },
    /// Delete only this observed file.
    Delete,
}

impl std::fmt::Debug for SourceGuidanceFileChange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keep => formatter.write_str("Keep"),
            Self::Delete => formatter.write_str("Delete"),
            Self::Write { content, mode } => formatter
                .debug_struct("Write")
                .field("bytes", &content.len())
                .field("mode", mode)
                .finish(),
        }
    }
}

/// One file carried only within an admitted source-guidance effect.
pub struct SourceGuidanceEffectFile {
    rel_path: String,
    baseline: MachineLocalFileObservation,
    change: SourceGuidanceFileChange,
}

impl SourceGuidanceEffectFile {
    /// Internal application-to-adapter construction, after reviewed-plan equality.
    #[doc(hidden)]
    #[must_use]
    pub fn from_application_admission(
        rel_path: String,
        baseline: MachineLocalFileObservation,
        change: SourceGuidanceFileChange,
    ) -> Self {
        Self {
            rel_path,
            baseline,
            change,
        }
    }

    /// Exact confined target retained by the application admission owner.
    #[doc(hidden)]
    #[must_use]
    pub fn rel_path(&self) -> &str {
        &self.rel_path
    }

    /// Exact native baseline, including byte and ancestor identities.
    #[doc(hidden)]
    #[must_use]
    pub const fn baseline(&self) -> &MachineLocalFileObservation {
        &self.baseline
    }

    /// Admitted bytes and mode, deletion, or retained state.
    #[doc(hidden)]
    #[must_use]
    pub const fn change(&self) -> &SourceGuidanceFileChange {
        &self.change
    }
}

/// Reviewed source-guidance proposal admitted for a confined native handoff.
///
/// Not cloneable, serializable or a runtime saved plan. The native adapter still
/// checks root, all file/parent baselines and each subsequent publication edge.
pub struct AdmittedSourceGuidanceEffect {
    root: MachineLocalRootIdentity,
    digest: ArtifactDigest,
    owner_sha256: ArtifactDigest,
    files: Vec<SourceGuidanceEffectFile>,
}

impl std::fmt::Debug for AdmittedSourceGuidanceEffect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedSourceGuidanceEffect")
            .field("digest", &self.digest)
            .field("files", &self.files.len())
            .finish_non_exhaustive()
    }
}

impl AdmittedSourceGuidanceEffect {
    /// Internal handoff from the owning application's reviewed-plan admission.
    #[doc(hidden)]
    #[must_use]
    pub fn from_application_admission(
        root: MachineLocalRootIdentity,
        digest: ArtifactDigest,
        owner_sha256: ArtifactDigest,
        files: Vec<SourceGuidanceEffectFile>,
    ) -> Self {
        Self {
            root,
            digest,
            owner_sha256,
            files,
        }
    }

    /// Retained root for the confined adapter; not an instruction to open it.
    #[doc(hidden)]
    #[must_use]
    pub const fn root(&self) -> &MachineLocalRootIdentity {
        &self.root
    }

    /// Exact reviewed proposal identity.
    #[must_use]
    pub const fn digest(&self) -> &ArtifactDigest {
        &self.digest
    }

    /// Domain-separated logical owner, including the selected native host.
    #[doc(hidden)]
    #[must_use]
    pub const fn owner_sha256(&self) -> &ArtifactDigest {
        &self.owner_sha256
    }

    /// Files in application-owned publication order, with unchanged checks last.
    #[doc(hidden)]
    #[must_use]
    pub fn files(&self) -> &[SourceGuidanceEffectFile] {
        &self.files
    }
}

/// Native recovery claims awaiting logical scope and reviewed-decision admission.
///
/// This contains no body bytes or native handles and grants no mutation authority.
#[derive(Clone, PartialEq, Eq)]
pub struct SourceGuidanceRecoveryProposal {
    root: MachineLocalRootIdentity,
    owner_sha256: ArtifactDigest,
    digest: ArtifactDigest,
    files: Vec<String>,
    directories: Vec<String>,
}

impl SourceGuidanceRecoveryProposal {
    /// Native-to-application claims captured by the maintained filesystem owner.
    #[doc(hidden)]
    #[must_use]
    pub fn from_native_capture(
        root: MachineLocalRootIdentity,
        owner_sha256: ArtifactDigest,
        digest: ArtifactDigest,
        files: Vec<String>,
        directories: Vec<String>,
    ) -> Self {
        Self {
            root,
            owner_sha256,
            digest,
            files,
            directories,
        }
    }
    /// Exact native root whose pathname and identity must still be checked.
    #[doc(hidden)]
    #[must_use]
    pub const fn root(&self) -> &MachineLocalRootIdentity {
        &self.root
    }
    /// Logical originating owner hash, not native authentication.
    #[doc(hidden)]
    #[must_use]
    pub const fn owner_sha256(&self) -> &ArtifactDigest {
        &self.owner_sha256
    }
    /// Native journal, observations, intent and exact proposed recovery identity.
    #[must_use]
    pub const fn digest(&self) -> &ArtifactDigest {
        &self.digest
    }
    /// Confined file claims requiring application namespace admission.
    #[doc(hidden)]
    #[must_use]
    pub fn files(&self) -> &[String] {
        &self.files
    }
    /// Conditional empty-directory cleanup claims requiring scope admission.
    #[doc(hidden)]
    #[must_use]
    pub fn directories(&self) -> &[String] {
        &self.directories
    }
}

/// Application-admitted exact recovery proposal; not a runtime grant.
pub struct AdmittedSourceGuidanceRecovery {
    proposal: SourceGuidanceRecoveryProposal,
}
impl AdmittedSourceGuidanceRecovery {
    /// Internal result of logical-owner, path-scope and reviewed-digest admission.
    #[doc(hidden)]
    #[must_use]
    pub fn from_application_admission(proposal: SourceGuidanceRecoveryProposal) -> Self {
        Self { proposal }
    }
    /// Native adapter's equality gate against its retained proposal.
    #[doc(hidden)]
    #[must_use]
    pub const fn proposal(&self) -> &SourceGuidanceRecoveryProposal {
        &self.proposal
    }
}
