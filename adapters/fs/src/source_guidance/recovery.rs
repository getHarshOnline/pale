//! Reviewed cold reconciliation; native evidence remains distinct from origin.

use super::{
    journal::{self, Change, Journal, Phase, Post, MAX_JOURNAL_BYTES},
    SOURCE_GUIDANCE_LOCK_PATH, SOURCE_GUIDANCE_RECOVERY_PATH,
};
use crate::confine::FileIdentity;
use crate::file_transaction::{Backup, FileSnapshot};
use crate::{acquire_advisory_lock, observe_machine_local_file, ConfinedRoot, FsError, ReadLimit};
use pale_core::records::{
    ArtifactDigest, ExistingMachineLocalFileV1, FilesystemDeviceIdV1, FilesystemInodeIdV1,
    MachineLocalDirectoryIdentity, MachineLocalFileIdentityV1,
};
use pale_ports::{
    AdmittedSourceGuidanceRecovery, MachineLocalFileObservation, SourceGuidanceFileChange,
    SourceGuidanceRecoveryProposal,
};
use std::{collections::BTreeMap, time::Duration};

const FILE_LIMIT: usize = 8 * 1024 * 1024;
const CAPTURE_LIMIT: usize = 64 * 1024 * 1024;

/// Exact recovery decision, selected before its journal/observation identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceGuidanceRecoveryIntent {
    /// Restore only the retained before-images, preserving independently changed entries.
    RestoreBefore,
    /// Keep a complete witnessed after-state and finish its commit/cleanup.
    FinalizePublished,
}

/// What the fresh file observation establishes, without guessing execution origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceGuidanceRecoveryEvidence {
    /// Current bytes/mode match the retained before-image, or both are absent.
    BeforeImagePresent,
    /// Current bytes/mode/inode match an exact recorded publication checkpoint.
    CheckpointedAfterImage,
    /// Bytes/mode match the intended after-image but no post identity was recorded.
    /// Reviewing restoration of this entry does not prove who created it.
    UncheckpointedAfterImage,
}

/// One file's native evidence and exact proposed recovery effect.
pub struct SourceGuidanceRecoveryFile {
    path: String,
    evidence: SourceGuidanceRecoveryEvidence,
    change: SourceGuidanceFileChange,
    observed: MachineLocalFileObservation,
}
impl SourceGuidanceRecoveryFile {
    /// Confined destination beneath the selected native root.
    #[must_use]
    pub fn rel_path(&self) -> &str {
        &self.path
    }
    /// Exact current evidence; uncheckpointed is explicitly uncertain origin.
    #[must_use]
    pub const fn evidence(&self) -> SourceGuidanceRecoveryEvidence {
        self.evidence
    }
    /// Proposed keep/write/delete; no mutation occurred during planning.
    #[must_use]
    pub const fn change(&self) -> &SourceGuidanceFileChange {
        &self.change
    }

    /// Captured native baseline for the owning SDK's deliberate review projection.
    #[doc(hidden)]
    #[must_use]
    pub const fn baseline(&self) -> &MachineLocalFileObservation {
        &self.observed
    }
}

/// Current identity of a transaction-created directory, before conditional cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceGuidanceRecoveryDirectoryState {
    /// Already absent; no directory will be created or removed.
    Missing,
    /// Matches the recorded created identity; remove only if still empty.
    IdentityMatched,
    /// Independent replacement; preserve it, including when empty.
    Replaced,
}
/// Directory cleanup is conditional and never takes over local contents.
pub struct SourceGuidanceRecoveryDirectory {
    path: String,
    state: SourceGuidanceRecoveryDirectoryState,
}
impl SourceGuidanceRecoveryDirectory {
    /// Confined directory considered for cleanup.
    #[must_use]
    pub fn rel_path(&self) -> &str {
        &self.path
    }
    /// Missing, exact recorded identity or a preserved replacement.
    #[must_use]
    pub const fn state(&self) -> SourceGuidanceRecoveryDirectoryState {
        self.state
    }
}

/// Effect-free native proposal. Logical owner/path/review admission is separate.
pub struct SourceGuidanceRecoveryPlan {
    root: ConfinedRoot,
    journal: Journal,
    journal_observation: MachineLocalFileObservation,
    intent: SourceGuidanceRecoveryIntent,
    files: Vec<SourceGuidanceRecoveryFile>,
    directories: Vec<SourceGuidanceRecoveryDirectory>,
    directory_observations: BTreeMap<String, Option<MachineLocalDirectoryIdentity>>,
    proposal: SourceGuidanceRecoveryProposal,
}
impl std::fmt::Debug for SourceGuidanceRecoveryPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceGuidanceRecoveryPlan")
            .field("digest", self.proposal.digest())
            .field("intent", &self.intent)
            .field("files", &self.files.len())
            .finish_non_exhaustive()
    }
}

/// Confirmed recovery outcome; preserved directories may contain local adaptations.
#[derive(Debug)]
pub struct SourceGuidanceRecoveryObservation {
    /// Exact reviewed recovery identity.
    pub recovery_sha256: ArtifactDigest,
    /// Selected decision, not a claim about execution origin of uncheckpointed files.
    pub intent: SourceGuidanceRecoveryIntent,
    /// Files whose before-images were restored or whose created leaves were removed.
    pub changed_files: usize,
    /// Recorded empty directories removed after exact identity checks.
    pub removed_directories: Vec<String>,
    /// Recorded directories retained because replaced or no longer empty.
    pub retained_directories: Vec<String>,
    /// Persistent cooperating-lock effect, independent of restored file content.
    pub coordination_effect: crate::AdvisoryLockEffect,
}

/// Read and validate a private journal and exact current file/directory observations.
///
/// Returns `None` only when the journal is absent. Reads are bounded and repeated,
/// without acquiring a lock or writing anything. A selected restore explicitly
/// includes any listed uncheckpointed matching after-images; independent changes
/// and checkpointed replacement inodes are refused rather than overwritten.
///
/// # Errors
/// Rejects malformed/cross-root/private-mode journals, conflicting decisions,
/// changed captures, invalid inventories, independent edits or unsafe paths.
pub fn plan_source_guidance_recovery(
    root: ConfinedRoot,
    intent: SourceGuidanceRecoveryIntent,
) -> Result<Option<SourceGuidanceRecoveryPlan>, FsError> {
    let journal_observation = observe_machine_local_file(
        &root,
        SOURCE_GUIDANCE_RECOVERY_PATH,
        ReadLimit::new(MAX_JOURNAL_BYTES),
    )?;
    let Some(bytes) = &journal_observation.bytes else {
        return Ok(None);
    };
    if journal_observation
        .prior
        .as_ref()
        .is_none_or(|p| p.mode.get() != 0o600)
    {
        return Err(journal::invalid("recovery journal is not owner-only"));
    }
    let journal = Journal::decode(bytes, &root)?;
    match (intent, journal.phase) {
        (SourceGuidanceRecoveryIntent::RestoreBefore, Phase::Committed)
        | (SourceGuidanceRecoveryIntent::FinalizePublished, Phase::Restoring | Phase::Restored) => {
            return Err(journal::invalid(
                "recovery intent contradicts the recorded decision",
            ))
        }
        _ => {}
    }
    let mut files = Vec::new();
    let mut total = 0_usize;
    for file in &journal.files {
        let observed = observe_machine_local_file(
            &root,
            &file.path,
            ReadLimit::new(FILE_LIMIT.min(CAPTURE_LIMIT.saturating_sub(total))),
        )?;
        total += observed.bytes.as_ref().map_or(0, Vec::len);
        let before = matches_before(file, &observed);
        let after = matches_after(file, &observed);
        let evidence = if before && intent == SourceGuidanceRecoveryIntent::RestoreBefore {
            SourceGuidanceRecoveryEvidence::BeforeImagePresent
        } else if after {
            match &file.published {
                None => SourceGuidanceRecoveryEvidence::UncheckpointedAfterImage,
                Some(Post::Absent) if observed.prior.is_none() => {
                    SourceGuidanceRecoveryEvidence::CheckpointedAfterImage
                }
                Some(Post::Present { state }) if observed.prior.as_ref() == Some(state) => {
                    SourceGuidanceRecoveryEvidence::CheckpointedAfterImage
                }
                _ => {
                    return Err(collision(
                        &file.path,
                        "checkpointed file was independently replaced",
                    ))
                }
            }
        } else {
            return Err(collision(
                &file.path,
                "current file differs from both admitted recovery images",
            ));
        };
        let change = match (intent, evidence) {
            (SourceGuidanceRecoveryIntent::FinalizePublished, _)
            | (_, SourceGuidanceRecoveryEvidence::BeforeImagePresent) => {
                SourceGuidanceFileChange::Keep
            }
            (_, _) => match &file.prior {
                Some(prior) => SourceGuidanceFileChange::Write {
                    content: prior.content.clone(),
                    mode: prior.state.mode,
                },
                None => SourceGuidanceFileChange::Delete,
            },
        };
        files.push(SourceGuidanceRecoveryFile {
            path: file.path.clone(),
            evidence,
            change,
            observed,
        });
    }
    let mut directory_observations = BTreeMap::new();
    for directory in journal
        .retained_directories
        .iter()
        .chain(&journal.created_directories)
    {
        let actual = observe_directory(&root, &directory.rel_path)?;
        if actual.as_ref() != Some(directory)
            && files.iter().any(|f| {
                !matches!(f.change, SourceGuidanceFileChange::Keep)
                    && f.path.starts_with(&format!("{}/", directory.rel_path))
            })
        {
            return Err(collision(
                &directory.rel_path,
                "recovery mutation would cross a changed parent identity",
            ));
        }
        directory_observations.insert(directory.rel_path.clone(), actual);
    }
    let directories = if intent == SourceGuidanceRecoveryIntent::RestoreBefore {
        journal
            .created_directories
            .iter()
            .map(|d| SourceGuidanceRecoveryDirectory {
                path: d.rel_path.clone(),
                state: match directory_observations
                    .get(&d.rel_path)
                    .and_then(Option::as_ref)
                {
                    None => SourceGuidanceRecoveryDirectoryState::Missing,
                    Some(current) if current == d => {
                        SourceGuidanceRecoveryDirectoryState::IdentityMatched
                    }
                    Some(_) => SourceGuidanceRecoveryDirectoryState::Replaced,
                },
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    // Reobserve every captured input without pretending the sequence is atomic.
    if observe_machine_local_file(
        &root,
        SOURCE_GUIDANCE_RECOVERY_PATH,
        ReadLimit::new(MAX_JOURNAL_BYTES),
    )? != journal_observation
    {
        return Err(journal::invalid("recovery journal changed during capture"));
    }
    for file in &files {
        if observe_machine_local_file(&root, &file.path, ReadLimit::new(FILE_LIMIT))?
            != file.observed
        {
            return Err(collision(
                &file.path,
                "recovery target changed during capture",
            ));
        }
    }
    for (path, observed) in &directory_observations {
        if observe_directory(&root, path)? != *observed {
            return Err(collision(path, "recovery directory changed during capture"));
        }
    }
    let file_preimages = files.iter().map(|f| serde_json::json!({"path":f.path,"evidence":f.evidence,"before":f.observed.prior,
        "parents":f.observed.retained_parent_directories,"missing":f.observed.missing_parent_directories,"change":change_identity(&f.change)})).collect::<Vec<_>>();
    let digest = ArtifactDigest::hash_bytes(pale_core::canon::to_canonical_string(&serde_json::json!({
        "domain":"pale.native-source-guidance-recovery.v1","root":root.machine_local_identity()?,"journal":journal_observation.prior,
        "intent":intent,"files":file_preimages,"directories":directory_observations,
    })).map_err(|e| crate::errors::data_error("encode source-guidance recovery identity", root.canonical_path(), e))?.as_bytes());
    let proposal = SourceGuidanceRecoveryProposal::from_native_capture(
        root.machine_local_identity()?,
        journal.owner_sha256.clone(),
        digest,
        files.iter().map(|f| f.path.clone()).collect(),
        directories.iter().map(|d| d.path.clone()).collect(),
    );
    Ok(Some(SourceGuidanceRecoveryPlan {
        root,
        journal,
        journal_observation,
        intent,
        files,
        directories,
        directory_observations,
        proposal,
    }))
}

impl SourceGuidanceRecoveryPlan {
    /// Exact identity to review, bound to intent, journal and current native state.
    #[must_use]
    pub const fn digest(&self) -> &ArtifactDigest {
        self.proposal.digest()
    }
    /// Explicit selected recovery decision.
    #[must_use]
    pub const fn intent(&self) -> SourceGuidanceRecoveryIntent {
        self.intent
    }
    /// Native claims for the application's logical scope/review gate.
    #[doc(hidden)]
    #[must_use]
    pub const fn proposal(&self) -> &SourceGuidanceRecoveryProposal {
        &self.proposal
    }
    /// Complete file effects and their evidence, in original publication order.
    #[must_use]
    pub fn files(&self) -> &[SourceGuidanceRecoveryFile] {
        &self.files
    }
    /// Exact-identity empty-directory cleanup candidates; replacements are retained.
    #[must_use]
    pub fn directories(&self) -> &[SourceGuidanceRecoveryDirectory] {
        &self.directories
    }

    /// Reobserve under the native lock and execute only this admitted proposal.
    ///
    /// A restore decision is durably recorded before leaf effects. Interruption
    /// can therefore resume from matching before-images without replaying a
    /// completed write. Newer file replacements/edits are never taken over.
    ///
    /// # Errors
    /// Rejects a different admission or changed observation before mutation.
    /// After the decision checkpoint, errors retain the journal and possible
    /// visibility for explicit read-back; recovery never silently reverses itself.
    pub fn execute(
        mut self,
        admitted: AdmittedSourceGuidanceRecovery,
    ) -> Result<SourceGuidanceRecoveryObservation, FsError> {
        if admitted.proposal() != &self.proposal {
            return Err(journal::invalid(
                "recovery admission differs from the retained proposal",
            ));
        }
        let (_lock, coordination) = acquire_advisory_lock(
            &self.root,
            SOURCE_GUIDANCE_LOCK_PATH,
            Duration::from_secs(10),
            "source-guidance recovery",
        )
        .map_err(|f| {
            let effect = f.effect();
            f.into_source().with_coordination_effect(effect)
        })?;
        self.reobserve()
            .map_err(|e| e.with_coordination_effect(coordination))?;
        let mut custody = Some(self.journal_observation.clone());
        let result = (|| {
            self.journal.phase = match self.intent {
                SourceGuidanceRecoveryIntent::RestoreBefore => Phase::Restoring,
                SourceGuidanceRecoveryIntent::FinalizePublished => Phase::Committed,
            };
            journal::persist(&self.root, &self.journal, &mut custody)?;
            let mut expected = self
                .files
                .iter()
                .map(|f| (f.path.clone(), f.observed.clone()))
                .collect::<BTreeMap<_, _>>();
            let mut changed_files = 0;
            if self.intent == SourceGuidanceRecoveryIntent::RestoreBefore {
                for file in self.files.iter().rev() {
                    if matches!(file.change, SourceGuidanceFileChange::Keep) {
                        continue;
                    }
                    if observe_machine_local_file(
                        &self.root,
                        &file.path,
                        ReadLimit::new(FILE_LIMIT),
                    )? != file.observed
                    {
                        return Err(collision(
                            &file.path,
                            "recovery target changed before its effect",
                        ));
                    }
                    let post = restore_file(&self.root, file)?;
                    expected.insert(file.path.clone(), post);
                    changed_files += 1;
                }
            }
            for (path, observed) in &expected {
                if observe_machine_local_file(&self.root, path, ReadLimit::new(FILE_LIMIT))?
                    != *observed
                {
                    return Err(collision(path, "recovery read-back differs"));
                }
            }
            let mut removed_directories = Vec::new();
            let mut retained_directories = Vec::new();
            if self.intent == SourceGuidanceRecoveryIntent::RestoreBefore {
                self.journal.phase = Phase::Restored;
                journal::persist(&self.root, &self.journal, &mut custody)?;
                let mut dirs = self.directories.iter().collect::<Vec<_>>();
                dirs.sort_by_key(|d| std::cmp::Reverse(d.path.matches('/').count()));
                for directory in dirs {
                    match directory.state {
                        SourceGuidanceRecoveryDirectoryState::Missing => {}
                        SourceGuidanceRecoveryDirectoryState::Replaced => {
                            retained_directories.push(directory.path.clone())
                        }
                        SourceGuidanceRecoveryDirectoryState::IdentityMatched => {
                            let prior = self
                                .directory_observations
                                .get(&directory.path)
                                .and_then(Option::as_ref)
                                .ok_or_else(|| {
                                    journal::invalid("recovery directory identity is absent")
                                })?;
                            match self
                                .root
                                .location(&directory.path)?
                                .remove_directory_if_identity(FileIdentity {
                                    device: prior.device.get(),
                                    inode: prior.inode.get(),
                                }) {
                                Ok(true) => removed_directories.push(directory.path.clone()),
                                Ok(false) => {}
                                Err(FsError::Io { source, .. })
                                    if source.kind() == std::io::ErrorKind::DirectoryNotEmpty =>
                                {
                                    retained_directories.push(directory.path.clone())
                                }
                                Err(error) => return Err(error),
                            }
                        }
                    }
                }
            }
            // Pruning can change only the parent partition of absent before-images.
            for file in &self.files {
                let actual =
                    observe_machine_local_file(&self.root, &file.path, ReadLimit::new(FILE_LIMIT))?;
                let expected = expected
                    .get(&file.path)
                    .ok_or_else(|| journal::invalid("recovery final observation is missing"))?;
                if actual.prior != expected.prior || actual.bytes != expected.bytes {
                    return Err(collision(
                        &file.path,
                        "recovery final bytes or identity changed",
                    ));
                }
            }
            journal::remove(&self.root, &mut custody)?;
            Ok(SourceGuidanceRecoveryObservation {
                recovery_sha256: self.proposal.digest().clone(),
                intent: self.intent,
                changed_files,
                removed_directories,
                retained_directories,
                coordination_effect: coordination,
            })
        })();
        result.map_err(|source| {
            crate::errors::effect_uncertain(
                "recover source-guidance publication",
                self.root.canonical_path(),
                source,
            )
            .with_coordination_effect(coordination)
        })
    }

    fn reobserve(&self) -> Result<(), FsError> {
        if observe_machine_local_file(
            &self.root,
            SOURCE_GUIDANCE_RECOVERY_PATH,
            ReadLimit::new(MAX_JOURNAL_BYTES),
        )? != self.journal_observation
        {
            return Err(journal::invalid("recovery journal changed after review"));
        }
        for file in &self.files {
            if observe_machine_local_file(&self.root, &file.path, ReadLimit::new(FILE_LIMIT))?
                != file.observed
            {
                return Err(collision(&file.path, "recovery file changed after review"));
            }
        }
        for (path, observed) in &self.directory_observations {
            if observe_directory(&self.root, path)? != *observed {
                return Err(collision(path, "recovery directory changed after review"));
            }
        }
        Ok(())
    }
}

fn matches_before(file: &super::journal::File, observed: &MachineLocalFileObservation) -> bool {
    match (&file.prior, &observed.prior, &observed.bytes) {
        (None, None, None) => true,
        (Some(prior), Some(state), Some(bytes)) => {
            prior.content.as_bytes() == bytes
                && prior.state.sha256 == state.sha256
                && prior.state.mode == state.mode
        }
        _ => false,
    }
}
fn matches_after(file: &super::journal::File, observed: &MachineLocalFileObservation) -> bool {
    match (&file.change, &observed.prior, &observed.bytes) {
        (Change::Delete, None, None) => true,
        (Change::Write { content, mode }, Some(state), Some(bytes)) => {
            content.as_bytes() == bytes
                && state.sha256 == ArtifactDigest::hash_bytes(content.as_bytes())
                && state.mode == *mode
        }
        _ => false,
    }
}
fn observe_directory(
    root: &ConfinedRoot,
    path: &str,
) -> Result<Option<MachineLocalDirectoryIdentity>, FsError> {
    Ok(root
        .location(path)?
        .directory_identity()?
        .map(|id| MachineLocalDirectoryIdentity {
            rel_path: path.to_owned(),
            device: FilesystemDeviceIdV1::new(id.device),
            inode: FilesystemInodeIdV1::new(id.inode),
        }))
}
fn change_identity(change: &SourceGuidanceFileChange) -> serde_json::Value {
    match change {
        SourceGuidanceFileChange::Keep => serde_json::json!({"kind":"keep"}),
        SourceGuidanceFileChange::Delete => serde_json::json!({"kind":"delete"}),
        SourceGuidanceFileChange::Write { content, mode } => {
            serde_json::json!({"kind":"write","sha256":ArtifactDigest::hash_bytes(content.as_bytes()),"mode":mode})
        }
    }
}
fn collision(path: &str, reason: &'static str) -> FsError {
    FsError::Collision(path.to_owned(), reason.to_owned())
}

pub(super) fn restore_file(
    root: &ConfinedRoot,
    file: &SourceGuidanceRecoveryFile,
) -> Result<MachineLocalFileObservation, FsError> {
    let location = root.location(&file.path)?;
    let prior = match (&file.observed.prior, &file.observed.bytes) {
        (Some(state), Some(bytes)) => Some(FileSnapshot {
            bytes: bytes.clone(),
            mode: state.mode.get(),
            identity: FileIdentity {
                device: state.identity.device.get(),
                inode: state.identity.inode.get(),
            },
        }),
        (None, None) => None,
        _ => {
            return Err(journal::invalid(
                "recovery observation presence is inconsistent",
            ))
        }
    };
    let mut backup = match &file.change {
        SourceGuidanceFileChange::Write { content, mode } => {
            Backup::write(location.clone(), prior, content.as_bytes(), mode.get())
        }
        SourceGuidanceFileChange::Delete => Backup::delete(location.clone(), prior),
        SourceGuidanceFileChange::Keep => return Ok(file.observed.clone()),
    };
    match &file.change {
        SourceGuidanceFileChange::Write { content, mode } => {
            if file.observed.prior.is_none() {
                location.write_atomic_noclobber(content.as_bytes(), mode.get())?;
            } else {
                location.write_atomic(content.as_bytes(), mode.get())?;
            }
        }
        SourceGuidanceFileChange::Delete => {
            let prior = file
                .observed
                .prior
                .as_ref()
                .ok_or_else(|| journal::invalid("recovery deletion has no observed file"))?;
            if !location.remove_file_if_identity(FileIdentity {
                device: prior.identity.device.get(),
                inode: prior.identity.inode.get(),
            })? {
                return Err(collision(
                    &file.path,
                    "recovery file disappeared before deletion",
                ));
            }
        }
        SourceGuidanceFileChange::Keep => {}
    }
    let verified = backup.verify_published()?;
    let mut expected = file.observed.clone();
    match (&file.change, verified) {
        (SourceGuidanceFileChange::Write { content, mode }, Some(id)) => {
            expected.bytes = Some(content.as_bytes().to_vec());
            expected.prior = Some(ExistingMachineLocalFileV1 {
                sha256: ArtifactDigest::hash_bytes(content.as_bytes()),
                mode: *mode,
                identity: MachineLocalFileIdentityV1 {
                    device: FilesystemDeviceIdV1::new(id.device),
                    inode: FilesystemInodeIdV1::new(id.inode),
                },
            });
        }
        (SourceGuidanceFileChange::Delete, None) => {
            expected.prior = None;
            expected.bytes = None;
        }
        _ => {
            return Err(journal::invalid(
                "verified recovery identity contradicts the selected change",
            ))
        }
    }
    Ok(expected)
}
