//! Prepared publication, exact recovery, and receipt observations.

use std::fmt;

use pale_core::admission::AdmittedPlan;
use pale_core::records::{
    ArtifactDigest, MachineLocalEffectObservationV1, MachineLocalEffectTransitionV1,
    MachineLocalEffectV1, MachineLocalFileIdentityV1, MachineLocalPostFileV1,
    MachineLocalReceiptOperationV1,
};

use super::{observe_machine_local_file, MachineLocalFileObservation};
use crate::confine::{ConfinedLocation, ConfinedRoot, CreatedDirectory, ReadLimit};
use crate::errors::FsError;
use crate::file_transaction::{recover, Backup, FileSnapshot, PriorFile};

/// Desired post-state supplied while preparing an admitted machine-local effect.
///
/// Intent is content input, not authority; preparation also requires the opaque
/// admitted plan whose effect digest binds these bytes.
pub enum MachineLocalFileIntent {
    /// Publish exact bytes using the mode carried by the effect plan.
    Write(Vec<u8>),
    /// Remove the exact admitted prior file.
    Delete,
}

impl fmt::Debug for MachineLocalFileIntent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Write(bytes) => formatter
                .debug_struct("Write")
                .field("byte_count", &bytes.len())
                .finish(),
            Self::Delete => formatter.write_str("Delete"),
        }
    }
}

/// Admitted, write-free preparation for one exact machine-local file effect.
#[must_use = "publish the prepared local effect or discard it before mutation begins"]
pub struct PreparedMachineLocalFile {
    root: ConfinedRoot,
    location: ConfinedLocation,
    planned: MachineLocalEffectV1,
    prior: PriorFile,
    intent: MachineLocalFileIntent,
    read_limit: ReadLimit,
}

impl fmt::Debug for PreparedMachineLocalFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedMachineLocalFile")
            .field("kind", &self.planned.kind)
            .field("root", &"<redacted>")
            .field("rel_path", &self.planned.rel_path)
            .field("transition", &self.planned.transition)
            .finish_non_exhaustive()
    }
}

/// Published machine-local effect retaining exact in-process recovery state.
#[must_use = "commit or roll back the published machine-local effect"]
pub struct AppliedMachineLocalFile {
    observation: MachineLocalReceiptOperationV1,
    backups: Vec<Backup>,
    created_directories: Vec<CreatedDirectory>,
    terminal: bool,
}

impl fmt::Debug for AppliedMachineLocalFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppliedMachineLocalFile")
            .field("kind", &self.observation.kind)
            .field("root", &"<redacted>")
            .field("rel_path", &self.observation.rel_path)
            .field("observation", &self.observation.observation)
            .field("recovery_file_count", &self.backups.len())
            .field("recovery_directory_count", &self.created_directories.len())
            .field("terminal", &self.terminal)
            .finish_non_exhaustive()
    }
}

impl AppliedMachineLocalFile {
    /// Exact receipt input observed while the reversible transaction remains live.
    pub fn observation(&self) -> &MachineLocalReceiptOperationV1 {
        &self.observation
    }

    /// Keep the published file and disable its in-process rollback.
    ///
    /// This does not prove successful settlement. The owning settlement decision
    /// may retain the file after durable receipt publication or after uncertain
    /// publication that requires reconciliation.
    pub fn commit(mut self) {
        self.terminal = true;
    }

    /// Restore only entries that still match this transaction's published identities.
    ///
    /// # Errors
    ///
    /// Returns structured recovery failures when a file or created directory was
    /// replaced, cannot be observed, or cannot be restored. Replacement entries are
    /// preserved and the resulting state is unknown until explicitly reconciled.
    pub fn rollback(mut self) -> Result<(), FsError> {
        let result = recover(&self.backups, &self.created_directories).map_err(|failures| {
            FsError::RecoveryFailed {
                operation: "roll back machine-local effect",
                failures,
            }
        });
        self.terminal = true;
        result
    }
}

impl Drop for AppliedMachineLocalFile {
    fn drop(&mut self) {
        if self.terminal {
            return;
        }
        let backups = std::mem::take(&mut self.backups);
        let created = std::mem::take(&mut self.created_directories);
        let recovered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            recover(&backups, &created).is_ok()
        }));
        match recovered {
            Ok(true) => eprintln!(
                "pale-adapter-fs: machine-local transaction dropped unfinished; pre-state restored"
            ),
            Ok(false) => eprintln!(
                "pale-adapter-fs: machine-local transaction dropped unfinished; recovery failed (state UNKNOWN)"
            ),
            Err(_) => eprintln!(
                "pale-adapter-fs: machine-local transaction recovery panicked (state UNKNOWN)"
            ),
        }
    }
}

/// Prepare one exact effect carried by an admitted v1 plan.
///
/// The effect record and intent are data, not mutation authority. This function
/// performs no writes and returns a capability only when the effect is a member of
/// `admitted`; publication then revalidates every filesystem observation.
///
/// # Errors
///
/// Rejects root, path, baseline, parent-set, payload, mode, or action mismatch and
/// any unsafe or incomplete confined observation.
pub fn prepare_machine_local_file(
    root: ConfinedRoot,
    admitted: &AdmittedPlan,
    planned: &MachineLocalEffectV1,
    intent: MachineLocalFileIntent,
    limit: ReadLimit,
) -> Result<PreparedMachineLocalFile, FsError> {
    if !admitted
        .record()
        .machine_local_effects
        .iter()
        .any(|effect| effect == planned)
    {
        return Err(FsError::InvalidMachineLocalEffect(
            "effect is not part of the admitted materialization plan".into(),
        ));
    }
    let observed = observe_machine_local_file(&root, &planned.rel_path, limit)?;
    validate_observation(planned, &observed)?;
    validate_intent(planned, &intent)?;
    let prior = match (observed.bytes, observed.prior) {
        (Some(bytes), Some(prior)) => Some(FileSnapshot {
            bytes,
            mode: prior.mode.get(),
            identity: crate::confine::FileIdentity {
                device: prior.identity.device.get(),
                inode: prior.identity.inode.get(),
            },
        }),
        (None, None) => None,
        _ => {
            return Err(FsError::InvalidMachineLocalEffect(
                "observed bytes and prior file identity disagree".into(),
            ));
        }
    };
    Ok(PreparedMachineLocalFile {
        location: root.location(&planned.rel_path)?,
        root,
        planned: planned.clone(),
        prior,
        intent,
        read_limit: limit,
    })
}

impl PreparedMachineLocalFile {
    /// Publish the exact admitted effect and retain identity-bound recovery state.
    ///
    /// # Errors
    ///
    /// Rejects any change since preparation. Publication failure triggers complete
    /// in-process recovery; incomplete recovery is returned as [`FsError::RestoreFailed`].
    pub fn publish(self) -> Result<AppliedMachineLocalFile, FsError> {
        let current =
            observe_machine_local_file(&self.root, &self.planned.rel_path, self.read_limit)?;
        validate_observation(&self.planned, &current)?;
        if current.bytes.as_deref()
            != self
                .prior
                .as_ref()
                .map(|snapshot| snapshot.bytes.as_slice())
        {
            return Err(FsError::Collision(
                self.planned.rel_path.clone(),
                "machine-local bytes changed after preparation".into(),
            ));
        }

        let mut backups = Vec::new();
        backups.try_reserve_exact(1).map_err(|source| {
            crate::errors::data_error(
                "reserve machine-local recovery journal",
                self.location.display_path(),
                source,
            )
        })?;
        let mut created_directories = Vec::new();
        created_directories
            .try_reserve_exact(self.planned.transition.create_directories().len())
            .map_err(|source| {
                crate::errors::data_error(
                    "reserve machine-local directory recovery journal",
                    self.location.display_path(),
                    source,
                )
            })?;
        let publish = (|| -> Result<Option<crate::confine::FileIdentity>, FsError> {
            match &self.intent {
                MachineLocalFileIntent::Write(bytes) => {
                    self.location
                        .ensure_parent_tracked(&mut created_directories)?;
                    backups.push(Backup::write(
                        self.location.clone(),
                        self.prior.clone(),
                        bytes,
                        self.planned.transition.mode().get(),
                    ));
                    if !matches!(
                        self.planned.transition,
                        MachineLocalEffectTransitionV1::Unchanged { .. }
                    ) {
                        if self.planned.transition.prior().is_some() {
                            self.location
                                .write_atomic(bytes, self.planned.transition.mode().get())?;
                        } else {
                            self.location.write_atomic_noclobber(
                                bytes,
                                self.planned.transition.mode().get(),
                            )?;
                        }
                    }
                }
                MachineLocalFileIntent::Delete => {
                    backups.push(Backup::delete(self.location.clone(), self.prior.clone()));
                    if !self.location.remove_file()? {
                        return Err(FsError::Collision(
                            self.planned.rel_path.clone(),
                            "admitted machine-local file disappeared before removal".into(),
                        ));
                    }
                }
            }
            backups
                .last_mut()
                .ok_or_else(|| {
                    FsError::InvalidMachineLocalEffect(
                        "publication produced no recovery record".into(),
                    )
                })?
                .verify_published()
        })();
        let published_identity = match publish {
            Ok(identity) => identity,
            Err(source) => {
                return match recover(&backups, &created_directories) {
                    Ok(()) => Err(source),
                    Err(failures) => Err(FsError::RestoreFailed {
                        source: Box::new(source),
                        failures,
                    }),
                };
            }
        };

        let created = created_directories
            .iter()
            .map(|directory| directory.relative().to_string())
            .collect::<Vec<_>>();
        if created != self.planned.transition.create_directories() {
            let source = FsError::Collision(
                self.planned.rel_path.clone(),
                "created-directory set differs from the admitted machine-local effect".into(),
            );
            return match recover(&backups, &created_directories) {
                Ok(()) => Err(source),
                Err(failures) => Err(FsError::RestoreFailed {
                    source: Box::new(source),
                    failures,
                }),
            };
        }
        let observation = machine_local_receipt_observation(
            &self.planned.transition,
            published_identity,
            created,
        )?;
        Ok(AppliedMachineLocalFile {
            observation: MachineLocalReceiptOperationV1 {
                kind: self.planned.kind,
                root: self.planned.root,
                rel_path: self.planned.rel_path,
                observation,
                mode: self.planned.transition.mode(),
            },
            backups,
            created_directories,
            terminal: false,
        })
    }
}

fn validate_observation(
    planned: &MachineLocalEffectV1,
    observed: &MachineLocalFileObservation,
) -> Result<(), FsError> {
    if observed.root != planned.root
        || observed.prior.as_ref() != planned.transition.prior()
        || observed.retained_parent_directories != planned.retained_directories
        || observed.missing_parent_directories != planned.transition.create_directories()
    {
        return Err(FsError::Collision(
            planned.rel_path.clone(),
            "machine-local root, baseline, or parent state differs from the admitted plan".into(),
        ));
    }
    Ok(())
}

fn validate_intent(
    planned: &MachineLocalEffectV1,
    intent: &MachineLocalFileIntent,
) -> Result<(), FsError> {
    let valid = match (&planned.transition, intent) {
        (
            MachineLocalEffectTransitionV1::Create { post, .. }
            | MachineLocalEffectTransitionV1::Update { post, .. }
            | MachineLocalEffectTransitionV1::Splice { post, .. },
            MachineLocalFileIntent::Write(bytes),
        ) => post.sha256 == ArtifactDigest::hash_bytes(bytes),
        (
            MachineLocalEffectTransitionV1::Unchanged { state },
            MachineLocalFileIntent::Write(bytes),
        ) => state.sha256 == ArtifactDigest::hash_bytes(bytes),
        (MachineLocalEffectTransitionV1::Delete { .. }, MachineLocalFileIntent::Delete) => true,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(FsError::InvalidMachineLocalEffect(format!(
            "intent does not match `{}` action and payload",
            planned.rel_path
        )))
    }
}

fn machine_local_receipt_observation(
    transition: &MachineLocalEffectTransitionV1,
    published_identity: Option<crate::confine::FileIdentity>,
    created_directories: Vec<String>,
) -> Result<MachineLocalEffectObservationV1, FsError> {
    let post = |sha256: ArtifactDigest| {
        published_identity
            .map(|identity| MachineLocalPostFileV1 {
                sha256,
                identity: MachineLocalFileIdentityV1 {
                    device: pale_core::records::FilesystemDeviceIdV1::new(identity.device),
                    inode: pale_core::records::FilesystemInodeIdV1::new(identity.inode),
                },
            })
            .ok_or_else(|| {
                FsError::InvalidMachineLocalEffect(
                    "published file transition has no post-file identity".into(),
                )
            })
    };
    match transition {
        MachineLocalEffectTransitionV1::Create { post: planned, .. } => {
            Ok(MachineLocalEffectObservationV1::Create {
                post: post(planned.sha256.clone())?,
                created_directories,
            })
        }
        MachineLocalEffectTransitionV1::Update {
            prior,
            post: planned,
        } => Ok(MachineLocalEffectObservationV1::Update {
            pre_sha256: prior.sha256.clone(),
            post: post(planned.sha256.clone())?,
        }),
        MachineLocalEffectTransitionV1::Splice {
            prior,
            post: planned,
        } => Ok(MachineLocalEffectObservationV1::Splice {
            pre_sha256: prior.sha256.clone(),
            post: post(planned.sha256.clone())?,
        }),
        MachineLocalEffectTransitionV1::Unchanged { state } => {
            Ok(MachineLocalEffectObservationV1::Unchanged {
                state: post(state.sha256.clone())?,
            })
        }
        MachineLocalEffectTransitionV1::Delete { prior } => {
            if published_identity.is_some() {
                return Err(FsError::InvalidMachineLocalEffect(
                    "deleted file still has a post-file identity".into(),
                ));
            }
            Ok(MachineLocalEffectObservationV1::Delete {
                pre_sha256: prior.sha256.clone(),
            })
        }
    }
}
