//! Native private recovery format, not a public runtime or Package contract.

use super::SOURCE_GUIDANCE_RECOVERY_PATH;
use crate::errors::data_error;
use crate::{observe_machine_local_file, ConfinedRoot, FsError, ReadLimit};
const JOURNAL_LIMIT: ReadLimit = ReadLimit::new(MAX_JOURNAL_BYTES);
use pale_core::records::{
    ArtifactDigest, ExistingMachineLocalFileV1, MachineLocalDirectoryIdentity, UnixFileMode,
};
use pale_ports::{
    AdmittedSourceGuidanceEffect, SourceGuidanceFileChange, MAX_SOURCE_GUIDANCE_TRANSACTION_FILES,
};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

/// Encoded journal ceiling, separate from the application's payload-byte budget.
pub(super) const MAX_JOURNAL_BYTES: usize = 128 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
enum Format {
    #[serde(rename = "pale-private-source-guidance-recovery")]
    Recovery,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Writing,
    Published,
    Restoring,
    Committed,
    Restored,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prior {
    pub(super) content: String,
    pub(super) state: ExistingMachineLocalFileV1,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Change {
    Write { content: String, mode: UnixFileMode },
    Delete,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Post {
    Absent,
    Present { state: ExistingMachineLocalFileV1 },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct File {
    pub(super) path: String,
    pub(super) prior: Option<Prior>,
    pub(super) change: Change,
    pub(super) published: Option<Post>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Journal {
    format: Format,
    version: u32,
    root_sha256: ArtifactDigest,
    plan_sha256: ArtifactDigest,
    pub(super) owner_sha256: ArtifactDigest,
    pub(super) phase: Phase,
    pub(super) files: Vec<File>,
    pub(super) retained_directories: Vec<MachineLocalDirectoryIdentity>,
    pub(super) created_directories: Vec<MachineLocalDirectoryIdentity>,
}

impl Journal {
    pub(super) fn from_effect(
        effect: &AdmittedSourceGuidanceEffect,
        root: &ConfinedRoot,
    ) -> Result<Self, FsError> {
        if effect.files().len() > MAX_SOURCE_GUIDANCE_TRANSACTION_FILES {
            return Err(invalid("source-guidance transaction file limit exceeded"));
        }
        let mut files = Vec::new();
        let mut retained = std::collections::BTreeMap::new();
        for file in effect.files() {
            let change = match file.change() {
                SourceGuidanceFileChange::Keep => continue,
                SourceGuidanceFileChange::Write { content, mode } => Change::Write {
                    content: content.clone(),
                    mode: *mode,
                },
                SourceGuidanceFileChange::Delete => Change::Delete,
            };
            let prior = match (&file.baseline().bytes, &file.baseline().prior) {
                (Some(bytes), Some(state)) => Some(Prior {
                    content: std::str::from_utf8(bytes)
                        .map_err(|error| {
                            data_error(
                                "capture source-guidance recovery text",
                                root.canonical_path(),
                                error,
                            )
                        })?
                        .to_owned(),
                    state: state.clone(),
                }),
                (None, None) => None,
                _ => {
                    return Err(FsError::InvalidState(
                        "admitted source-guidance presence contradicts bytes".into(),
                    ))
                }
            };
            for directory in &file.baseline().retained_parent_directories {
                retained.insert(directory.rel_path.clone(), directory.clone());
            }
            files.push(File {
                path: file.rel_path().to_owned(),
                prior,
                change,
                published: None,
            });
        }
        let journal = Self {
            format: Format::Recovery,
            version: 1,
            root_sha256: root_digest(root)?,
            plan_sha256: effect.digest().clone(),
            owner_sha256: effect.owner_sha256().clone(),
            phase: Phase::Writing,
            files,
            retained_directories: retained.into_values().collect(),
            created_directories: Vec::new(),
        };
        journal.encode(root)?;
        Ok(journal)
    }

    pub(super) fn decode(bytes: &[u8], root: &ConfinedRoot) -> Result<Self, FsError> {
        use std::collections::BTreeSet;
        if bytes.len() > MAX_JOURNAL_BYTES {
            return Err(invalid("recovery journal byte limit exceeded"));
        }
        let journal: Self = serde_json::from_slice(bytes).map_err(|error| {
            data_error(
                "decode private source-guidance recovery journal",
                root.canonical_path(),
                error,
            )
        })?;
        if journal.version != 1
            || journal.root_sha256 != root_digest(root)?
            || journal.files.is_empty()
            || journal.files.len() > MAX_SOURCE_GUIDANCE_TRANSACTION_FILES
        {
            return Err(invalid("recovery version, root or file inventory differs"));
        }
        let mut paths = BTreeSet::new();
        let mut ancestors = BTreeSet::new();
        let mut payload = 0_usize;
        let mut uncheckpointed = false;
        for file in &journal.files {
            if !paths.insert(file.path.clone())
                || file.path.len() > pale_core::paths::MAX_CONFINED_RELATIVE_PATH_BYTES
                || pale_core::paths::ConfinedRelativePath::parse(&file.path).is_none()
                || matches!(
                    file.path.as_str(),
                    super::SOURCE_GUIDANCE_LOCK_PATH | super::SOURCE_GUIDANCE_RECOVERY_PATH
                )
            {
                return Err(invalid(
                    "recovery contains invalid, reserved or repeated file paths",
                ));
            }
            let mut parent = std::path::Path::new(&file.path).parent();
            while let Some(path) = parent.filter(|p| !p.as_os_str().is_empty()) {
                ancestors.insert(
                    path.to_str()
                        .ok_or_else(|| invalid("recovery parent is not UTF-8"))?
                        .to_owned(),
                );
                parent = path.parent();
            }
            if let Some(prior) = &file.prior {
                if prior.content.len() > 8 * 1024 * 1024
                    || ArtifactDigest::hash_bytes(prior.content.as_bytes()) != prior.state.sha256
                {
                    return Err(invalid("recovery before-image differs from its identity"));
                }
                payload = payload.saturating_add(prior.content.len());
            }
            match &file.change {
                Change::Write { content, mode } => {
                    if content.len() > 8 * 1024 * 1024 {
                        return Err(invalid("recovery after-image exceeds its byte limit"));
                    }
                    payload = payload.saturating_add(content.len());
                    if let Some(post) = &file.published {
                        if !matches!(post, Post::Present { state } if state.sha256 == ArtifactDigest::hash_bytes(content.as_bytes()) && state.mode == *mode)
                        {
                            return Err(invalid("recovery checkpoint contradicts its write"));
                        }
                    }
                }
                Change::Delete => {
                    if file.prior.is_none()
                        || file
                            .published
                            .as_ref()
                            .is_some_and(|p| !matches!(p, Post::Absent))
                    {
                        return Err(invalid("recovery checkpoint contradicts its deletion"));
                    }
                }
            }
            if payload > 64 * 1024 * 1024 {
                return Err(invalid("recovery payload budget exceeded"));
            }
            if file.published.is_none() {
                uncheckpointed = true;
            } else if uncheckpointed {
                return Err(invalid("recovery checkpoints are not a prefix"));
            }
        }
        if matches!(journal.phase, Phase::Published | Phase::Committed) && uncheckpointed {
            return Err(invalid(
                "complete recovery phase has incomplete checkpoints",
            ));
        }
        let mut directories = BTreeSet::new();
        if journal
            .retained_directories
            .len()
            .saturating_add(journal.created_directories.len())
            > 16384
        {
            return Err(invalid("recovery directory budget exceeded"));
        }
        for directory in journal
            .retained_directories
            .iter()
            .chain(&journal.created_directories)
        {
            if !ancestors.contains(&directory.rel_path)
                || !directories.insert(directory.rel_path.clone())
            {
                return Err(invalid(
                    "recovery directory is duplicated or unrelated to its files",
                ));
            }
        }
        let retained = journal
            .retained_directories
            .iter()
            .map(|d| d.rel_path.as_str())
            .collect::<BTreeSet<_>>();
        for file in &journal.files {
            if file.prior.is_some() {
                let mut parent = std::path::Path::new(&file.path).parent();
                while let Some(path) = parent.filter(|p| !p.as_os_str().is_empty()) {
                    if !retained.contains(
                        path.to_str()
                            .ok_or_else(|| invalid("invalid recovery parent"))?,
                    ) {
                        return Err(invalid(
                            "recovery before-image has an incomplete parent inventory",
                        ));
                    }
                    parent = path.parent();
                }
            }
        }
        Ok(journal)
    }

    pub(super) fn encode(&self, root: &ConfinedRoot) -> Result<Vec<u8>, FsError> {
        if self.files.len() > MAX_SOURCE_GUIDANCE_TRANSACTION_FILES {
            return Err(invalid("recovery journal file limit exceeded"));
        }
        let mut target = BoundedBytes(Vec::new());
        serde_json::to_writer(&mut target, self).map_err(|error| {
            data_error(
                "encode bounded source-guidance recovery journal",
                root.canonical_path(),
                error,
            )
        })?;
        Ok(target.0)
    }
}

pub(super) fn invalid(detail: &'static str) -> FsError {
    FsError::InvalidState(detail.into())
}

fn root_digest(root: &ConfinedRoot) -> Result<ArtifactDigest, FsError> {
    let bytes = pale_core::canon::to_canonical_string(&serde_json::json!({
        "domain":"pale.private-source-guidance-recovery-root.v1", "root":root.machine_local_identity()?
    })).map_err(|e| data_error("encode recovery root identity", root.canonical_path(), e))?;
    Ok(ArtifactDigest::hash_bytes(bytes.as_bytes()))
}

struct BoundedBytes(Vec<u8>);
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_JOURNAL_BYTES.saturating_sub(self.0.len()) {
            return Err(io::Error::other(
                "source-guidance recovery journal byte limit exceeded",
            ));
        }
        self.0.try_reserve(bytes.len()).map_err(io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn persist(
    root: &ConfinedRoot,
    journal: &Journal,
    last: &mut Option<pale_ports::MachineLocalFileObservation>,
) -> Result<(), FsError> {
    let bytes = journal.encode(root)?;
    let location = root.location(SOURCE_GUIDANCE_RECOVERY_PATH)?;
    if let Some(expected) = &*last {
        if observe_machine_local_file(root, SOURCE_GUIDANCE_RECOVERY_PATH, JOURNAL_LIMIT)?
            != *expected
        {
            return Err(FsError::Collision(
                SOURCE_GUIDANCE_RECOVERY_PATH.into(),
                "recovery journal was replaced or edited".into(),
            ));
        }
        location.write_atomic(&bytes, 0o600)?;
    } else {
        location.write_atomic_noclobber(&bytes, 0o600)?;
    }
    let observed = observe_machine_local_file(root, SOURCE_GUIDANCE_RECOVERY_PATH, JOURNAL_LIMIT)?;
    if observed.bytes.as_deref() != Some(bytes.as_slice())
        || observed
            .prior
            .as_ref()
            .is_none_or(|p| p.mode.get() != 0o600)
    {
        return Err(FsError::Collision(
            SOURCE_GUIDANCE_RECOVERY_PATH.into(),
            "recovery journal read-back differs".into(),
        ));
    }
    *last = Some(observed);
    Ok(())
}

pub(super) fn remove(
    root: &ConfinedRoot,
    last: &mut Option<pale_ports::MachineLocalFileObservation>,
) -> Result<(), FsError> {
    let Some(expected) = &*last else {
        return Ok(());
    };
    if observe_machine_local_file(root, SOURCE_GUIDANCE_RECOVERY_PATH, JOURNAL_LIMIT)? != *expected
    {
        return Err(FsError::Collision(
            SOURCE_GUIDANCE_RECOVERY_PATH.into(),
            "recovery journal changed before cleanup".into(),
        ));
    }
    let prior = expected
        .prior
        .as_ref()
        .ok_or_else(|| FsError::InvalidState("retained recovery journal has no identity".into()))?;
    if !root
        .location(SOURCE_GUIDANCE_RECOVERY_PATH)?
        .remove_file_if_identity(crate::confine::FileIdentity {
            device: prior.identity.device.get(),
            inode: prior.identity.inode.get(),
        })?
    {
        return Err(FsError::Collision(
            SOURCE_GUIDANCE_RECOVERY_PATH.into(),
            "recovery journal disappeared before cleanup".into(),
        ));
    }
    *last = None;
    Ok(())
}
