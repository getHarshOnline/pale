//! Bounded expiring records shared across native-harness processes.
//!
//! The public types distinguish recoverable session continuity from evidence that
//! must use the adapter's strongest declared publication path. Envelope evolution
//! and filesystem retention mechanics remain separate internal owners.

mod envelope;
mod store;

use std::path::Path;
use std::time::Duration;

use pale_core::engine_wire::ExpiringRecordClassV1;
use pale_core::records::ExpiringRecordContract;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::errors::FsError;
use store::ExpiringRecordStore;

#[derive(Clone, Copy, Debug)]
/// Semantic record class selecting envelope identity and publication strength.
enum PublicationClass {
    SessionContinuity,
    DurableEvidence,
}

/// Stable operator-facing vocabulary owned by one [`PublicationClass`].
struct RecordVocabulary {
    record_name: &'static str,
    create_operation: &'static str,
    parse_operation: &'static str,
    decode_operation: &'static str,
    decode_timestamp_operation: &'static str,
    update_operation: &'static str,
    encode_operation: &'static str,
    cleanup_operation: &'static str,
}

/// Operator vocabulary for recoverable session-continuity records.
const SESSION_VOCABULARY: RecordVocabulary = RecordVocabulary {
    record_name: "session state",
    create_operation: "create session store",
    parse_operation: "parse session state",
    decode_operation: "decode session state",
    decode_timestamp_operation: "decode session-state timestamp",
    update_operation: "update session state",
    encode_operation: "encode session state",
    cleanup_operation: "prune expired session states",
};

/// Operator vocabulary distinguishing durable evidence from session continuity.
const EVIDENCE_VOCABULARY: RecordVocabulary = RecordVocabulary {
    record_name: "durable evidence",
    create_operation: "create durable-evidence store",
    parse_operation: "parse durable evidence",
    decode_operation: "decode durable evidence",
    decode_timestamp_operation: "decode durable-evidence timestamp",
    update_operation: "update durable evidence",
    encode_operation: "encode durable evidence",
    cleanup_operation: "prune expired durable evidence",
};

impl PublicationClass {
    const fn record_class(self) -> &'static str {
        match self {
            Self::SessionContinuity => "session-continuity",
            Self::DurableEvidence => "durable-evidence",
        }
    }

    const fn wire_class(self) -> ExpiringRecordClassV1 {
        match self {
            Self::SessionContinuity => ExpiringRecordClassV1::SessionContinuity,
            Self::DurableEvidence => ExpiringRecordClassV1::DurableEvidence,
        }
    }

    const fn vocabulary(self) -> &'static RecordVocabulary {
        match self {
            Self::SessionContinuity => &SESSION_VOCABULARY,
            Self::DurableEvidence => &EVIDENCE_VOCABULARY,
        }
    }
}

/// Expiring cross-process continuity state for one native-harness session.
///
/// Reads observe complete old-or-new records. Conditional updates serialize the
/// full read-modify-publish transition under a bounded advisory lock. This state
/// is recoverable continuity, not durable decision evidence. Current envelopes
/// bind the publication class, retained root, and relative record path. The store
/// validates that envelope; `T`'s deserializer and transition own domain rules.
pub struct SessionStore(ExpiringRecordStore);

impl SessionStore {
    /// Open one owner-only `.json` session record beneath a confined root.
    ///
    /// `state_rel` must name a file below a non-empty parent and `ttl` must be
    /// nonzero. The retained root descriptor governs every later operation.
    ///
    /// # Errors
    ///
    /// Rejects an unsafe root or record path, a non-JSON leaf, zero retention, and
    /// pre-existing unsafe ancestors.
    pub fn new(
        root: &Path,
        state_rel: &str,
        ttl: Duration,
        contract: ExpiringRecordContract,
    ) -> Result<Self, FsError> {
        ExpiringRecordStore::new(
            root,
            state_rel,
            ttl,
            PublicationClass::SessionContinuity,
            contract,
        )
        .map(Self)
    }

    /// Read the current publication generation and state, or publication generation zero plus [`Default`]
    /// when the record is absent or expired.
    ///
    /// A future timestamp is ambiguous clock evidence and fails closed; it is never
    /// converted into an empty session because that could erase a live guard.
    ///
    /// # Errors
    ///
    /// Rejects oversized, non-owner-only, malformed, non-canonical, misbound, or
    /// future-dated records and any incomplete confined read.
    pub fn load<T: Serialize + DeserializeOwned + Default>(&self) -> Result<(u64, T), FsError> {
        self.0.load()
    }

    /// Publish a session transition only when it returns [`Some`].
    ///
    /// [`None`] leaves bytes and publication generation unchanged. The closure runs under the
    /// bounded lock and must not perform external effects that publication failure
    /// could leave behind.
    ///
    /// # Errors
    ///
    /// Reports confinement, lock, current-record admission, publication generation, encoding,
    /// size, publication, durability, and post-publication identity failures.
    pub fn update_if<T, R>(
        &self,
        transition: impl FnOnce(&mut T) -> Option<R>,
    ) -> Result<Option<R>, FsError>
    where
        T: Serialize + DeserializeOwned + Default,
    {
        self.0.update_if(transition)
    }

    /// Remove this session record under its shard lock. Absence is success;
    /// post-unlink uncertainty is reported as [`FsError::EffectUncertain`].
    ///
    /// # Errors
    ///
    /// Reports lock, confinement, entry-type, unlink, synchronization, and root
    /// identity failures. An uncertain error may follow a visible removal.
    pub fn remove(&self) -> Result<bool, FsError> {
        self.0.remove()
    }

    /// Remove at most `limit` expired siblings after validating the complete
    /// session-directory grammar. A failure can follow earlier removals and is
    /// returned with the batch's structured failure set.
    ///
    /// # Errors
    ///
    /// Rejects foreign directory entries and unsafe, malformed, misbound, or
    /// future-dated records. A batch error can retain failures after earlier
    /// siblings were removed.
    pub fn prune_expired_siblings(&self, limit: usize) -> Result<usize, FsError> {
        self.0.prune_expired_siblings(limit)
    }
}

/// Bounded expiring evidence that requires durable publication before success.
///
/// This remains a replaceable snapshot record, not an append-only event stream.
/// Identity-bound domains initialize explicitly; domains where absence is an empty
/// record may opt into the [`Default`] transition methods. Current envelopes bind
/// the publication class, retained root, and relative record path; unbound
/// envelopes and future-dated evidence fail closed. The store validates that
/// envelope; `T`'s deserializer and transition own domain rules.
pub struct DurableEvidenceStore(ExpiringRecordStore);

impl DurableEvidenceStore {
    /// Open one owner-only `.json` evidence record beneath a confined root.
    ///
    /// `state_rel` must name a file below a non-empty parent and `ttl` must be
    /// nonzero. The retained root descriptor governs every later operation.
    ///
    /// # Errors
    ///
    /// Rejects an unsafe root or record path, a non-JSON leaf, zero retention, and
    /// pre-existing unsafe ancestors.
    pub fn new(
        root: &Path,
        state_rel: &str,
        ttl: Duration,
        contract: ExpiringRecordContract,
    ) -> Result<Self, FsError> {
        ExpiringRecordStore::new(
            root,
            state_rel,
            ttl,
            PublicationClass::DurableEvidence,
            contract,
        )
        .map(Self)
    }

    /// Read current evidence, or publication generation zero plus [`Default`] when absent or
    /// expired and the caller's domain defines that state as empty. Future-dated
    /// evidence is uncertainty and returns an error rather than becoming default.
    ///
    /// # Errors
    ///
    /// Rejects oversized, non-owner-only, malformed, non-canonical, unbound,
    /// misbound, or future-dated evidence and any incomplete confined read.
    pub fn load_or_default<T: Serialize + DeserializeOwned + Default>(
        &self,
    ) -> Result<(u64, T), FsError> {
        self.0.load()
    }

    /// Read current evidence or initialize an identity-bound empty value without publishing it.
    ///
    /// This is the read counterpart to [`Self::update_or_initialize`]. The
    /// initializer runs only when the record is absent or expired.
    ///
    /// # Errors
    ///
    /// Rejects the same malformed, unsafe, misbound, or future-dated evidence as
    /// [`Self::load_or_default`].
    pub fn load_or_initialize<T: Serialize + DeserializeOwned>(
        &self,
        initialize: impl FnOnce() -> T,
    ) -> Result<(u64, T), FsError> {
        self.0.load_or_initialize(initialize)
    }

    /// Begin from [`Default`] when absent or expired, then conditionally publish one
    /// evidence transition. Replacing an expired record advances its persisted
    /// generation; [`None`] leaves bytes and publication generation unchanged.
    ///
    /// # Errors
    ///
    /// Reports confinement, lock, current-record admission, publication generation, encoding,
    /// size, publication, durability, and post-publication identity failures.
    pub fn update_if_or_default<T, R>(
        &self,
        transition: impl FnOnce(&mut T) -> Option<R>,
    ) -> Result<Option<R>, FsError>
    where
        T: Serialize + DeserializeOwned + Default,
    {
        self.0.update_if(transition)
    }

    /// Initialize an identity-bound record when absent or expired and publish
    /// only when the transition returns [`Some`].
    ///
    /// [`None`] leaves bytes and publication generation unchanged. Both closures
    /// must remain bounded and side-effect-free.
    ///
    /// # Errors
    ///
    /// Reports current-record admission, confinement, encoding, publication,
    /// durability, and post-publication identity failures.
    pub fn update_if_or_initialize<T, R>(
        &self,
        initialize: impl FnOnce() -> T,
        transition: impl FnOnce(&mut T) -> Option<R>,
    ) -> Result<Option<R>, FsError>
    where
        T: Serialize + DeserializeOwned,
    {
        self.0.update_if_or_initialize(initialize, transition)
    }

    /// Initialize an identity-bound record when absent or expired, then publish
    /// one unconditional transition under the bounded lock. Replacing an expired
    /// record advances its publication generation.
    ///
    /// Both closures must be bounded and side-effect-free because encoding or
    /// publication can still fail after the transition returns.
    ///
    /// # Errors
    ///
    /// Reports confinement, lock, current-record admission, publication generation, encoding,
    /// size, publication, durability, and post-publication identity failures.
    pub fn update_or_initialize<T, R>(
        &self,
        initialize: impl FnOnce() -> T,
        transition: impl FnOnce(&mut T) -> R,
    ) -> Result<R, FsError>
    where
        T: Serialize + DeserializeOwned,
    {
        self.0.update_or_initialize(initialize, transition)
    }

    /// Remove at most `limit` expired siblings after validating the complete
    /// evidence-directory grammar. A failure can follow earlier removals and is
    /// returned with the batch's structured failure set.
    ///
    /// # Errors
    ///
    /// Rejects foreign directory entries and unsafe, malformed, unbound, misbound,
    /// or future-dated evidence. A batch error can retain failures after earlier
    /// siblings were removed.
    pub fn prune_expired_siblings(&self, limit: usize) -> Result<usize, FsError> {
        self.0.prune_expired_siblings(limit)
    }
}
