//! Application-admitted publication using the existing native recovery engine.

use super::{
    journal::{Journal, Phase, Post, MAX_JOURNAL_BYTES},
    SOURCE_GUIDANCE_LOCK_PATH, SOURCE_GUIDANCE_RECOVERY_PATH,
};
use crate::confine::{CreatedDirectory, FileIdentity};
use crate::errors::FailureSet;
use crate::file_transaction::{recover, Backup, FileSnapshot};
use crate::{
    acquire_advisory_lock, observe_machine_local_file, AdvisoryLock, AdvisoryLockEffect,
    ConfinedRoot, FsError, ReadLimit,
};
use pale_core::records::{ArtifactDigest, ExistingMachineLocalFileV1};
use pale_ports::{
    AdmittedSourceGuidanceEffect, MachineLocalFileObservation, SourceGuidanceEffectFile,
    SourceGuidanceFileChange,
};
use std::{collections::BTreeMap, path::Path, time::Duration};

const TARGET_LIMIT: ReadLimit = ReadLimit::new(8 * 1024 * 1024);
const JOURNAL_LIMIT: ReadLimit = ReadLimit::new(MAX_JOURNAL_BYTES);

/// Native result of a confirmed source-guidance file transaction, not host use.
#[derive(Debug)]
pub struct SourceGuidanceWriteObservation {
    /// Exact application-reviewed proposal identity.
    pub plan_sha256: ArtifactDigest,
    /// Number of published writes/deletions, excluding unchanged checks.
    pub changed_files: usize,
    /// Persistent cooperating-lock effect, independent of target publication.
    pub coordination_effect: AdvisoryLockEffect,
}

/// Admission checked under the cooperating lock, before target mutation.
#[must_use = "publish the prepared source-guidance effect or discard it"]
pub struct PreparedSourceGuidanceWrite {
    run: Run,
}

/// Published files with exact in-process recovery and durable private custody.
#[must_use = "commit or roll back the published source-guidance write"]
pub struct AppliedSourceGuidanceWrite {
    run: Run,
}

/// What an unsuccessful publication establishes about guidance target files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceGuidancePublicationFailureState {
    /// No guidance leaf or ancestor mutation began; coordination is separate.
    NotStarted,
    /// Native recovery confirmed retained before-images and pruned created paths.
    /// Restored file inodes and the persistent lock may differ from the original.
    Restored,
    /// Publication, restoration or custody could not establish a settled result.
    OutcomeUnknown,
}

/// Publication failure with native restoration evidence, without flattening its cause.
#[derive(Debug, thiserror::Error)]
#[error("source-guidance publication failed ({state:?}): {source}")]
pub struct SourceGuidancePublicationFailure {
    state: SourceGuidancePublicationFailureState,
    #[source]
    source: FsError,
}
impl SourceGuidancePublicationFailure {
    /// Native target-file conclusion, distinct from persistent coordination effects.
    #[must_use]
    pub const fn state(&self) -> SourceGuidancePublicationFailureState {
        self.state
    }
    /// Retained filesystem cause for the owning SDK's diagnostic projection.
    #[must_use]
    pub const fn source_error(&self) -> &FsError {
        &self.source
    }
    /// Recover the original structured filesystem failure.
    #[must_use]
    pub fn into_source(self) -> FsError {
        self.source
    }
}

struct Run {
    effect: AdmittedSourceGuidanceEffect,
    root: ConfinedRoot,
    journal: Journal,
    last_journal: Option<MachineLocalFileObservation>,
    backups: Vec<Backup>,
    created: Vec<CreatedDirectory>,
    coordination: AdvisoryLockEffect,
    terminal: bool,
    _lock: AdvisoryLock,
}

impl std::fmt::Debug for PreparedSourceGuidanceWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedSourceGuidanceWrite")
            .field("plan", &self.run.effect.digest())
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for AppliedSourceGuidanceWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppliedSourceGuidanceWrite")
            .field("plan", &self.run.effect.digest())
            .field("changed_files", &self.run.backups.len())
            .finish_non_exhaustive()
    }
}

/// Acquire the native lock, refuse pending recovery and reobserve every baseline.
///
/// The persistent root-level lock can be created. No guidance file, parent or
/// recovery journal is changed by successful preparation. This is not a native
/// host grant, an adversarial-writer exclusion or a filesystem snapshot.
///
/// # Errors
/// Rejects changed roots/files/ancestors, unsafe paths, lock failures or a pending
/// recovery journal. Errors retain separately observed lock coordination effects.
pub fn prepare_source_guidance_write(
    effect: AdmittedSourceGuidanceEffect,
) -> Result<PreparedSourceGuidanceWrite, FsError> {
    let root = ConfinedRoot::open(Path::new(&effect.root().canonical_path))?;
    if root.machine_local_identity()? != *effect.root() {
        return Err(FsError::Collision(
            ".".into(),
            "source-guidance root identity changed".into(),
        ));
    }
    let (lock, coordination) = acquire_advisory_lock(
        &root,
        SOURCE_GUIDANCE_LOCK_PATH,
        Duration::from_secs(10),
        "source-guidance write",
    )
    .map_err(|failure| {
        let effect = failure.effect();
        failure.into_source().with_coordination_effect(effect)
    })?;
    let preparation = (|| {
        if observe_machine_local_file(&root, SOURCE_GUIDANCE_RECOVERY_PATH, JOURNAL_LIMIT)?
            .prior
            .is_some()
        {
            return Err(FsError::Collision(
                SOURCE_GUIDANCE_RECOVERY_PATH.into(),
                "unfinished source-guidance recovery must be reconciled first".into(),
            ));
        }
        for file in effect.files() {
            if observe_machine_local_file(&root, file.rel_path(), TARGET_LIMIT)? != *file.baseline()
            {
                return Err(FsError::Collision(
                    file.rel_path().into(),
                    "source-guidance baseline changed after review".into(),
                ));
            }
        }
        Journal::from_effect(&effect, &root)
    })();
    let journal = preparation.map_err(|error| error.with_coordination_effect(coordination))?;
    Ok(PreparedSourceGuidanceWrite {
        run: Run {
            effect,
            root,
            journal,
            last_journal: None,
            backups: Vec::new(),
            created: Vec::new(),
            coordination,
            terminal: false,
            _lock: lock,
        },
    })
}

impl PreparedSourceGuidanceWrite {
    /// Publish each application-ordered effect and verify the complete read-back.
    ///
    /// A synchronized private journal precedes target changes. The existing
    /// identity-bound recovery engine restores in-process failures; collisions
    /// preserve newer entries and retain recovery evidence. A crash is not an
    /// atomic rollback and a pending journal blocks subsequent normal writes.
    ///
    /// # Errors
    /// Returns the original failure after confirmed restoration, or structured
    /// restoration failures with the journal retained for explicit reconciliation.
    pub fn publish(self) -> Result<AppliedSourceGuidanceWrite, FsError> {
        self.publish_reported()
            .map_err(SourceGuidancePublicationFailure::into_source)
    }

    /// Publish with the native distinction between not-started, restored and uncertain.
    ///
    /// # Errors
    /// Returns a structured target-state conclusion and the intact filesystem cause.
    pub fn publish_reported(
        self,
    ) -> Result<AppliedSourceGuidanceWrite, SourceGuidancePublicationFailure> {
        let mut run = self.run;
        let result = run.publish();
        if let Err(source) = result {
            let started = !run.backups.is_empty() || !run.created.is_empty();
            let uncertain = source.effect_visibility() == crate::EffectVisibility::MayBeVisible;
            let restoration = run.restore();
            run.terminal = true;
            let (state, error) = match restoration {
                Ok(()) => (
                    if uncertain {
                        SourceGuidancePublicationFailureState::OutcomeUnknown
                    } else if started {
                        SourceGuidancePublicationFailureState::Restored
                    } else {
                        SourceGuidancePublicationFailureState::NotStarted
                    },
                    source,
                ),
                Err(failures) => (
                    SourceGuidancePublicationFailureState::OutcomeUnknown,
                    FsError::RestoreFailed {
                        source: Box::new(source),
                        failures,
                    },
                ),
            };
            return Err(SourceGuidancePublicationFailure {
                state,
                source: error.with_coordination_effect(run.coordination),
            });
        }
        Ok(AppliedSourceGuidanceWrite { run })
    }
}

impl AppliedSourceGuidanceWrite {
    /// Confirm commitment, remove the exact journal, and retain published files.
    ///
    /// # Errors
    /// A failure leaves the files in place and the journal for reconciliation;
    /// it never silently rolls back an uncertain commit decision.
    pub fn commit(mut self) -> Result<SourceGuidanceWriteObservation, FsError> {
        self.run.terminal = true;
        let finalization = (|| {
            self.run.read_back()?;
            if self.run.last_journal.is_some() {
                self.run.journal.phase = Phase::Committed;
                self.run.persist_journal()?;
                self.run.remove_journal()?;
            }
            Ok::<(), FsError>(())
        })();
        finalization.map_err(|source| {
            let error = if self.run.backups.is_empty() {
                source
            } else {
                crate::errors::effect_uncertain(
                    "commit source-guidance publication",
                    self.run.root.canonical_path(),
                    source,
                )
            };
            error.with_coordination_effect(self.run.coordination)
        })?;
        Ok(SourceGuidanceWriteObservation {
            plan_sha256: self.run.effect.digest().clone(),
            changed_files: self.run.backups.len(),
            coordination_effect: self.run.coordination,
        })
    }

    /// Restore exact prior bytes/modes only while published identities still match.
    ///
    /// # Errors
    /// Attempts the complete recovery set. Replacement files/directories are
    /// preserved, all failures retained, and the private journal left in place.
    pub fn rollback(mut self) -> Result<(), FsError> {
        let result = self.run.restore();
        self.run.terminal = true;
        result.map_err(|failures| {
            FsError::RecoveryFailed {
                operation: "roll back source guidance",
                failures,
            }
            .with_coordination_effect(self.run.coordination)
        })
    }
}

impl Run {
    fn persist_journal(&mut self) -> Result<(), FsError> {
        super::journal::persist(&self.root, &self.journal, &mut self.last_journal)
    }
    fn remove_journal(&mut self) -> Result<(), FsError> {
        super::journal::remove(&self.root, &mut self.last_journal)
    }

    fn expected(
        &self,
        file: &SourceGuidanceEffectFile,
    ) -> Result<MachineLocalFileObservation, FsError> {
        let created = self
            .created
            .iter()
            .map(|d| d.identity().map(|id| (id.rel_path.clone(), id)))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let mut expected = file.baseline().clone();
        let missing = std::mem::take(&mut expected.missing_parent_directories);
        for path in missing {
            if let Some(directory) = created.get(&path) {
                expected.retained_parent_directories.push(directory.clone());
            } else {
                expected.missing_parent_directories.push(path);
            }
        }
        Ok(expected)
    }

    fn check_before(&self, index: usize) -> Result<MachineLocalFileObservation, FsError> {
        let file = &self.effect.files()[index];
        let observed = observe_machine_local_file(&self.root, file.rel_path(), TARGET_LIMIT)?;
        if observed != self.expected(file)? {
            return Err(FsError::Collision(
                file.rel_path().into(),
                "source-guidance file or ancestor changed before publication".into(),
            ));
        }
        Ok(observed)
    }

    fn publish_file(&mut self, index: usize, journal_index: usize) -> Result<(), FsError> {
        let post = self.publish_leaf(index)?;
        self.journal.files[journal_index].published = Some(post);
        self.persist_journal()
    }

    fn publish_leaf(&mut self, index: usize) -> Result<Post, FsError> {
        self.check_before(index)?;
        let location = self.root.location(self.effect.files()[index].rel_path())?;
        if matches!(
            self.effect.files()[index].change(),
            SourceGuidanceFileChange::Write { .. }
        ) {
            let before = self.created.len();
            location.ensure_parent_tracked(&mut self.created)?;
            if self.created.len() != before {
                self.journal.created_directories = self
                    .created
                    .iter()
                    .map(CreatedDirectory::identity)
                    .collect::<Result<_, _>>()?;
                self.persist_journal()?;
            }
        }
        let observed = self.check_before(index)?;
        let prior = match (&observed.bytes, &observed.prior) {
            (Some(bytes), Some(p)) => Some(FileSnapshot {
                bytes: bytes.clone(),
                mode: p.mode.get(),
                identity: file_identity(p),
            }),
            (None, None) => None,
            _ => {
                return Err(FsError::InvalidState(
                    "source-guidance baseline presence contradicts bytes".into(),
                ))
            }
        };
        self.backups.try_reserve(1).map_err(|e| {
            crate::errors::data_error(
                "reserve source-guidance recovery entry",
                self.root.canonical_path(),
                e,
            )
        })?;
        match self.effect.files()[index].change() {
            SourceGuidanceFileChange::Keep => {
                return Err(FsError::InvalidState(
                    "unchanged file entered publication sequence".into(),
                ))
            }
            SourceGuidanceFileChange::Write { content, mode } => {
                let absent = prior.is_none();
                self.backups.push(Backup::write(
                    location.clone(),
                    prior,
                    content.as_bytes(),
                    mode.get(),
                ));
                if absent {
                    location.write_atomic_noclobber(content.as_bytes(), mode.get())?;
                } else {
                    location.write_atomic(content.as_bytes(), mode.get())?;
                }
            }
            SourceGuidanceFileChange::Delete => {
                let expected = observed.prior.as_ref().ok_or_else(|| {
                    FsError::InvalidState("admitted deletion has no prior file".into())
                })?;
                self.backups.push(Backup::delete(location.clone(), prior));
                if !location.remove_file_if_identity(file_identity(expected))? {
                    return Err(FsError::Collision(
                        location.relative().into(),
                        "source-guidance file disappeared before deletion".into(),
                    ));
                }
            }
        }
        let published = self
            .backups
            .last_mut()
            .ok_or_else(|| {
                FsError::InvalidState("source-guidance publication has no recovery entry".into())
            })?
            .verify_published()?;
        // Bind the journal to the exact identity verified by the shared recovery
        // engine. A second read must not replace that identity with a later edit.
        Ok(match (self.effect.files()[index].change(), published) {
            (SourceGuidanceFileChange::Write { content, mode }, Some(identity)) => Post::Present {
                state: ExistingMachineLocalFileV1 {
                    sha256: ArtifactDigest::hash_bytes(content.as_bytes()),
                    mode: *mode,
                    identity: pale_core::records::MachineLocalFileIdentityV1 {
                        device: pale_core::records::FilesystemDeviceIdV1::new(identity.device),
                        inode: pale_core::records::FilesystemInodeIdV1::new(identity.inode),
                    },
                },
            },
            (SourceGuidanceFileChange::Delete, None) => Post::Absent,
            _ => {
                return Err(FsError::InvalidState(
                    "source-guidance verified post identity contradicts the admitted effect".into(),
                ))
            }
        })
    }

    fn read_back(&self) -> Result<(), FsError> {
        let mut changed = 0;
        for file in self.effect.files() {
            let mut expected = self.expected(file)?;
            if !matches!(file.change(), SourceGuidanceFileChange::Keep) {
                let post = self.journal.files[changed]
                    .published
                    .as_ref()
                    .ok_or_else(|| {
                        FsError::InvalidState(
                            "source-guidance publication has no post identity".into(),
                        )
                    })?;
                expected.prior = match post {
                    Post::Absent => None,
                    Post::Present { state } => Some(state.clone()),
                };
                expected.bytes = match file.change() {
                    SourceGuidanceFileChange::Write { content, .. } => {
                        Some(content.as_bytes().to_vec())
                    }
                    _ => None,
                };
                changed += 1;
            }
            if observe_machine_local_file(&self.root, file.rel_path(), TARGET_LIMIT)? != expected {
                return Err(FsError::Collision(
                    file.rel_path().into(),
                    "complete source-guidance read-back differs from the published identities"
                        .into(),
                ));
            }
        }
        Ok(())
    }

    fn publish(&mut self) -> Result<(), FsError> {
        if !self.journal.files.is_empty() {
            self.persist_journal()?;
        }
        let mut changed = 0;
        for index in 0..self.effect.files().len() {
            if matches!(
                self.effect.files()[index].change(),
                SourceGuidanceFileChange::Keep
            ) {
                continue;
            }
            self.publish_file(index, changed)?;
            changed += 1;
        }
        self.read_back()?;
        if self.last_journal.is_some() {
            self.journal.phase = Phase::Published;
            self.persist_journal()?;
        }
        Ok(())
    }

    fn restore(&mut self) -> Result<(), FailureSet> {
        if self.last_journal.is_some() {
            self.journal.phase = Phase::Restoring;
            if let Err(error) = self.persist_journal() {
                if let Some(failures) = FailureSet::from_failures(vec![error]) {
                    return Err(failures);
                }
            }
        }
        recover(&self.backups, &self.created)?;
        let mut failures = Vec::new();
        if self.last_journal.is_some() {
            self.journal.phase = Phase::Restored;
            if let Err(error) = self.persist_journal().and_then(|()| self.remove_journal()) {
                failures.push(error);
            }
        }
        match FailureSet::from_failures(failures) {
            None => Ok(()),
            Some(failures) => Err(failures),
        }
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        if self.terminal {
            return;
        }
        let recovered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.restore()));
        if !matches!(recovered, Ok(Ok(()))) {
            eprintln!("pale-adapter-fs: source-guidance recovery incomplete; private journal retained, state UNKNOWN");
        }
    }
}

fn file_identity(prior: &ExistingMachineLocalFileV1) -> FileIdentity {
    FileIdentity {
        device: prior.identity.device.get(),
        inode: prior.identity.inode.get(),
    }
}

#[cfg(all(test, unix))]
#[path = "../../tests/internal/source_guidance.rs"]
mod tests;
