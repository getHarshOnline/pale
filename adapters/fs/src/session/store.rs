//! Confined locking, retention, and publication for expiring records.

use std::path::Path;
use std::time::Duration;

use pale_core::records::{ArtifactDigest, ExpiringRecordContract};
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::envelope::{decode, encode, Decoded, Freshness, UnixTimestamp};
use super::PublicationClass;
use crate::confine::{
    ConfinedEntryKind, ConfinedLocation, ConfinedRoot, DirectoryLimit, ReadLimit,
};
use crate::errors::{invariant, FailureSet, FsError};

/// Owner-only mode required for every expiring record.
const MODE: u32 = 0o600;
/// Maximum encoded envelope and payload bytes in one atomic publication.
const MAX_RECORD_BYTES: usize = 1 << 20;
/// Read ceiling matching [`MAX_RECORD_BYTES`].
const RECORD_READ_LIMIT: ReadLimit = ReadLimit::new(MAX_RECORD_BYTES);
/// Maximum entries enumerated before an expiring-record prune pass.
const RECORD_DIRECTORY_LIMIT: DirectoryLimit = DirectoryLimit::new(4_096);
/// Maximum wait for an authority-bearing record transition lock.
const LOCK_TIMEOUT: Duration = Duration::from_millis(500);
/// Maximum opportunistic wait for a pruning lock.
const PRUNE_LOCK_TIMEOUT: Duration = Duration::from_millis(5);

fn effect_uncertain(
    operation: &'static str,
    location: &ConfinedLocation,
    source: FsError,
) -> FsError {
    FsError::EffectUncertain {
        operation,
        path: location.display_path(),
        source: Box::new(source),
    }
}

fn read_bounded(
    location: &ConfinedLocation,
    publication: PublicationClass,
) -> Result<Option<Vec<u8>>, FsError> {
    let Some(existing) = location.read_file(RECORD_READ_LIMIT)? else {
        return Ok(None);
    };
    if existing.mode != MODE {
        return Err(FsError::Collision(
            location.relative().to_string(),
            format!(
                "{} mode is {:#o}; expected owner-only {MODE:#o}",
                publication.vocabulary().record_name,
                existing.mode
            ),
        ));
    }
    Ok(Some(existing.bytes))
}

/// Shared mechanism behind the two semantically distinct public record stores.
pub(super) struct ExpiringRecordStore {
    root: ConfinedRoot,
    state: ConfinedLocation,
    lock: ConfinedLocation,
    ttl: Duration,
    publication: PublicationClass,
    record_binding: ArtifactDigest,
    contract: ExpiringRecordContract,
}

impl ExpiringRecordStore {
    /// Open and bind one record path to a retained root and publication class.
    pub(super) fn new(
        root: &Path,
        state_rel: &str,
        ttl: Duration,
        publication: PublicationClass,
        contract: ExpiringRecordContract,
    ) -> Result<Self, FsError> {
        Self::from_root(
            ConfinedRoot::open(root)?,
            state_rel,
            ttl,
            publication,
            contract,
        )
    }

    fn from_root(
        root: ConfinedRoot,
        state_rel: &str,
        ttl: Duration,
        publication: PublicationClass,
        contract: ExpiringRecordContract,
    ) -> Result<Self, FsError> {
        if ttl.is_zero() {
            return Err(invariant(
                publication.vocabulary().create_operation,
                state_rel,
                "retention must be greater than zero",
            ));
        }
        let state_path = Path::new(state_rel);
        if state_path.extension().and_then(|value| value.to_str()) != Some("json") {
            return Err(FsError::UnexpectedExpiringRecordEntry {
                path: state_path.to_path_buf(),
                expected: "a .json expiring record",
            });
        }
        let parent = state_path
            .parent()
            .and_then(Path::to_str)
            .filter(|parent| !parent.is_empty())
            .ok_or_else(|| FsError::Escape(state_rel.to_string()))?;
        let shard = &pale_core::sha256_hex(state_rel.as_bytes())[..2];
        let state = root.location(state_rel)?;
        let lock = root.location(&format!("{parent}/.locks/{shard}.lock"))?;
        state.validate_ancestors()?;
        lock.validate_ancestors()?;
        let binding_preimage = pale_core::canon::to_canonical_string(&(
            publication.record_class(),
            crate::materialization_state::target_root_sha256(root.canonical_path()),
            state_rel,
            &contract,
        ))
        .map_err(|source| {
            crate::errors::data_error(
                publication.vocabulary().create_operation,
                state.display_path(),
                source,
            )
        })?;
        let record_binding = ArtifactDigest::hash_bytes(binding_preimage.as_bytes());
        Ok(Self {
            root,
            state,
            lock,
            ttl,
            publication,
            record_binding,
            contract,
        })
    }

    fn decode_existing<T: Serialize + DeserializeOwned>(
        &self,
    ) -> Result<Option<Decoded<T>>, FsError> {
        self.root.verify_path_identity()?;
        let Some(bytes) = read_bounded(&self.state, self.publication)? else {
            return Ok(None);
        };
        let decoded = decode::<T>(
            &bytes,
            &self.state.display_path(),
            self.publication,
            &self.record_binding,
            &self.contract,
        )?;
        self.root.verify_path_identity()?;
        Ok(Some(decoded))
    }

    fn freshness(&self, updated: UnixTimestamp) -> Result<Freshness, FsError> {
        let freshness = UnixTimestamp::now()?.freshness(updated, self.ttl);
        if freshness == Freshness::FutureDated {
            return Err(invariant(
                self.publication.vocabulary().decode_timestamp_operation,
                self.state.display_path(),
                format!(
                    "{} is future-dated or the system clock moved backward; temporal status is unknown",
                    self.publication.vocabulary().record_name
                ),
            ));
        }
        Ok(freshness)
    }

    fn load_existing<T: Serialize + DeserializeOwned>(&self) -> Result<Option<(u64, T)>, FsError> {
        let Some(record) = self.decode_existing()? else {
            return Ok(None);
        };
        if self.freshness(record.updated)? == Freshness::Fresh {
            Ok(Some((record.publication_generation, record.state)))
        } else {
            Ok(None)
        }
    }

    fn load_for_update<T: Serialize + DeserializeOwned>(
        &self,
    ) -> Result<(u64, Option<T>), FsError> {
        let Some(record) = self.decode_existing()? else {
            return Ok((0, None));
        };
        let state = (self.freshness(record.updated)? == Freshness::Fresh).then_some(record.state);
        Ok((record.publication_generation, state))
    }

    /// Load fresh state or the domain default when the record is absent or expired.
    pub(super) fn load<T: Serialize + DeserializeOwned + Default>(
        &self,
    ) -> Result<(u64, T), FsError> {
        Ok(self.load_existing()?.unwrap_or_else(|| (0, T::default())))
    }

    /// Load fresh state or invoke the domain initializer when no live record exists.
    pub(super) fn load_or_initialize<T: Serialize + DeserializeOwned>(
        &self,
        initialize: impl FnOnce() -> T,
    ) -> Result<(u64, T), FsError> {
        Ok(self.load_existing()?.unwrap_or_else(|| (0, initialize())))
    }

    /// Initialize when absent or expired, then durably publish one transition.
    pub(super) fn update_or_initialize<T, R>(
        &self,
        initialize: impl FnOnce() -> T,
        transition: impl FnOnce(&mut T) -> R,
    ) -> Result<R, FsError>
    where
        T: Serialize + DeserializeOwned,
    {
        self.update_if_with(initialize, |state| Some(transition(state)))?
            .ok_or_else(|| {
                invariant(
                    self.publication.vocabulary().update_operation,
                    self.state.display_path(),
                    "unconditional transition was not applied",
                )
            })
    }

    /// Initialize when absent or expired and publish only an accepted transition.
    pub(super) fn update_if_or_initialize<T, R>(
        &self,
        initialize: impl FnOnce() -> T,
        transition: impl FnOnce(&mut T) -> Option<R>,
    ) -> Result<Option<R>, FsError>
    where
        T: Serialize + DeserializeOwned,
    {
        self.update_if_with(initialize, transition)
    }

    /// Publish only when the locked transition returns an output.
    pub(super) fn update_if<T, R>(
        &self,
        transition: impl FnOnce(&mut T) -> Option<R>,
    ) -> Result<Option<R>, FsError>
    where
        T: Serialize + DeserializeOwned + Default,
    {
        self.update_if_with(T::default, transition)
    }

    fn update_if_with<T, R>(
        &self,
        initialize: impl FnOnce() -> T,
        transition: impl FnOnce(&mut T) -> Option<R>,
    ) -> Result<Option<R>, FsError>
    where
        T: Serialize + DeserializeOwned,
    {
        self.root.verify_path_identity()?;
        self.state.ensure_parent()?;
        let (_guard, coordination_effect) = crate::lock::acquire_location(
            &self.lock,
            LOCK_TIMEOUT,
            self.publication.vocabulary().record_name,
        )
        .map_err(|failure| {
            let effect = failure.effect();
            failure.into_source().with_coordination_effect(effect)
        })?;

        let update = (|| -> Result<Option<R>, FsError> {
            let (publication_generation, existing) = self.load_for_update()?;
            let mut state = existing.unwrap_or_else(initialize);
            let Some(out) = transition(&mut state) else {
                return Ok(None);
            };
            let next_publication_generation =
                publication_generation.checked_add(1).ok_or_else(|| {
                    invariant(
                        self.publication.vocabulary().update_operation,
                        self.state.display_path(),
                        "publication generation exhausted",
                    )
                })?;
            let bytes = encode(
                next_publication_generation,
                UnixTimestamp::now()?,
                state,
                &self.state.display_path(),
                self.publication,
                &self.record_binding,
                &self.contract,
            )?;
            if bytes.len() > MAX_RECORD_BYTES {
                return Err(FsError::RecordSizeLimit {
                    path: self.state.relative().to_string(),
                    max_bytes: MAX_RECORD_BYTES,
                });
            }
            match self.publication {
                PublicationClass::SessionContinuity => {
                    self.state.write_session_state(&bytes, MODE)?;
                }
                PublicationClass::DurableEvidence => self.state.write_atomic(&bytes, MODE)?,
            }
            self.root.verify_path_identity().map_err(|source| {
                effect_uncertain(
                    self.publication.vocabulary().update_operation,
                    &self.state,
                    source,
                )
            })?;
            Ok(Some(out))
        })();
        update.map_err(|source| source.with_coordination_effect(coordination_effect))
    }

    /// Remove this exact record under its shard lock.
    pub(super) fn remove(&self) -> Result<bool, FsError> {
        self.root.verify_path_identity()?;
        let (_guard, coordination_effect) = crate::lock::acquire_location(
            &self.lock,
            LOCK_TIMEOUT,
            self.publication.vocabulary().record_name,
        )
        .map_err(|failure| {
            let effect = failure.effect();
            failure.into_source().with_coordination_effect(effect)
        })?;
        let removal = (|| -> Result<bool, FsError> {
            let removed = self.state.remove_file()?;
            if removed {
                self.root.verify_path_identity().map_err(|source| {
                    effect_uncertain("remove expiring record", &self.state, source)
                })?;
            } else {
                self.root.verify_path_identity()?;
            }
            Ok(removed)
        })();
        removal.map_err(|source| source.with_coordination_effect(coordination_effect))
    }

    /// Validate the sibling grammar, then remove at most `limit` expired records.
    pub(super) fn prune_expired_siblings(&self, limit: usize) -> Result<usize, FsError> {
        if limit == 0 {
            return Ok(0);
        }
        self.root.verify_path_identity()?;
        let parent = Path::new(self.state.relative())
            .parent()
            .and_then(Path::to_str)
            .filter(|parent| !parent.is_empty())
            .ok_or_else(|| FsError::Escape(self.state.relative().to_string()))?;
        let directory = self.root.location(parent)?;
        let candidates = record_candidates(&directory)?;
        let mut removed = 0;
        let mut failures = Vec::new();
        for name in candidates {
            if removed >= limit {
                break;
            }
            let relative = format!("{parent}/{name}");
            let candidate = Self::from_root(
                self.root.clone(),
                &relative,
                self.ttl,
                self.publication,
                self.contract.clone(),
            )?;
            match candidate.remove_if_expired() {
                Ok(true) => removed += 1,
                Ok(false) => {}
                Err(error) => failures.push(error),
            }
        }
        if let Err(source) = self.root.verify_path_identity() {
            failures.push(if removed > 0 {
                effect_uncertain(
                    self.publication.vocabulary().cleanup_operation,
                    &directory,
                    source,
                )
            } else {
                source
            });
        }
        match FailureSet::from_failures(failures) {
            Some(failures) => Err(FsError::BatchFailed {
                operation: self.publication.vocabulary().cleanup_operation,
                failures,
            }),
            None => Ok(removed),
        }
    }

    fn remove_if_expired(&self) -> Result<bool, FsError> {
        self.root.verify_path_identity()?;
        let (_guard, coordination_effect) = crate::lock::acquire_location(
            &self.lock,
            PRUNE_LOCK_TIMEOUT,
            self.publication.vocabulary().record_name,
        )
        .map_err(|failure| {
            let effect = failure.effect();
            failure.into_source().with_coordination_effect(effect)
        })?;
        let removal = (|| -> Result<bool, FsError> {
            let Some(bytes) = read_bounded(&self.state, self.publication)? else {
                return Ok(false);
            };
            let record = decode::<serde_json::Value>(
                &bytes,
                &self.state.display_path(),
                self.publication,
                &self.record_binding,
                &self.contract,
            )?;
            if self.freshness(record.updated)? == Freshness::Fresh {
                return Ok(false);
            }
            let removed = self.state.remove_file()?;
            if removed {
                self.root.verify_path_identity().map_err(|source| {
                    effect_uncertain("prune expired record", &self.state, source)
                })?;
            }
            Ok(removed)
        })();
        removal.map_err(|source| source.with_coordination_effect(coordination_effect))
    }
}

fn record_candidates(directory: &ConfinedLocation) -> Result<Vec<String>, FsError> {
    let mut candidates = Vec::new();
    for entry in directory
        .read_directory(RECORD_DIRECTORY_LIMIT)?
        .unwrap_or_default()
    {
        match (entry.name.as_str(), entry.kind) {
            (".locks", ConfinedEntryKind::Directory) => {}
            (name, ConfinedEntryKind::File) if name.ends_with(".json") => {
                candidates.push(entry.name);
            }
            _ => {
                return Err(FsError::UnexpectedExpiringRecordEntry {
                    path: directory.display_path().join(entry.name),
                    expected: "a .json record or the .locks directory",
                });
            }
        }
    }
    Ok(candidates)
}
