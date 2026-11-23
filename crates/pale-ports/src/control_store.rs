//! Neutral physical primitives shared by control-store custody ports.
//!
//! Application services admit semantic records and transitions before this
//! boundary. These types retain only exact immutable record bytes, physical
//! coordinates, and hard-domain checkpoint heads needed for atomic custody.
//! They do not infer a principal, deployment, effect, signer, or authority from
//! storage location.

use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlTransactionSequenceV1, InvalidResidentRecordV1, ResidentRecordRefV1, SchemaDigest,
    SchemaId,
};

/// Proposed next mutation of one exact control-store incarnation.
///
/// Construction checks only the positive bounded integer relationship. The
/// preparation is neither a reservation nor evidence that the expected head is
/// current. A custody owner must compare the complete preparation inside its
/// actual mutation transaction. Only successful durable commitment establishes
/// the successor as a committed transaction; signing alone does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlTransactionPreparation {
    store: crate::AuthorityStoreCoordinate,
    expected: ControlTransactionSequenceV1,
    successor: ControlTransactionSequenceV1,
}

impl ControlTransactionPreparation {
    /// Bind a store coordinate to its structurally checked next transaction.
    ///
    /// # Errors
    /// Rejects exhaustion of the existing control-coordinate integer domain.
    pub fn new(
        store: crate::AuthorityStoreCoordinate,
        expected: ControlTransactionSequenceV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        // The admitted coordinate is at most 2^53-1, so the u64 addition cannot
        // overflow. The existing owner rejects a successor outside its domain.
        let successor = ControlTransactionSequenceV1::new(expected.get() + 1)?;
        Ok(Self {
            store,
            expected,
            successor,
        })
    }

    /// Borrow the exact store and incarnation, independent of filesystem path.
    #[must_use]
    pub const fn store(&self) -> &crate::AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the transaction head that actual commitment must compare.
    #[must_use]
    pub const fn expected(&self) -> ControlTransactionSequenceV1 {
        self.expected
    }

    /// Return the proposed successor, not a durable receipt.
    #[must_use]
    pub const fn successor(&self) -> ControlTransactionSequenceV1 {
        self.successor
    }
}

/// Bounded transaction-frontier read shared by checkpoint custody owners.
///
/// Every logical mutation in the selected physical store participates in the
/// same store-and-incarnation sequence. This read neither advances the sequence
/// nor retains a write lock, admits authority, or protects against rollback.
/// An unrelated successful write may make the result stale before commitment.
pub trait ControlTransactionPort {
    /// Read the exact positive store head and form its checked successor.
    ///
    /// # Errors
    /// Rejects unavailable or corrupt storage, a different store incarnation,
    /// or exhaustion. A missing head is not an uninitialized counter to create.
    fn prepare_transaction(
        &mut self,
        store: &crate::AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError>;
}

/// Exact physical head of one hard control-checkpoint domain.
///
/// The checkpoint remains a separately admitted signed record. This projection
/// retains only the coordinates required to compare and advance its physical
/// custody atomically with the domain mutation it closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlCheckpointHead {
    checkpoint_domain_id: ControlCheckpointDomainIdV1,
    domain_epoch: ControlDomainEpochV1,
    domain_sequence: ControlDomainSequenceV1,
    committed_transaction_sequence: ControlTransactionSequenceV1,
    coordinate: ControlRecordCoordinate,
}

impl ControlCheckpointHead {
    /// Bind one admitted checkpoint to its physical domain frontier.
    #[must_use]
    pub const fn new(
        checkpoint_domain_id: ControlCheckpointDomainIdV1,
        domain_epoch: ControlDomainEpochV1,
        domain_sequence: ControlDomainSequenceV1,
        committed_transaction_sequence: ControlTransactionSequenceV1,
        coordinate: ControlRecordCoordinate,
    ) -> Self {
        Self {
            checkpoint_domain_id,
            domain_epoch,
            domain_sequence,
            committed_transaction_sequence,
            coordinate,
        }
    }

    /// Borrow the exact hard-conflict domain.
    #[must_use]
    pub const fn checkpoint_domain_id(&self) -> &ControlCheckpointDomainIdV1 {
        &self.checkpoint_domain_id
    }

    /// Return the checkpoint domain epoch.
    #[must_use]
    pub const fn domain_epoch(&self) -> ControlDomainEpochV1 {
        self.domain_epoch
    }

    /// Return the positive sequence within [`Self::domain_epoch`].
    #[must_use]
    pub const fn domain_sequence(&self) -> ControlDomainSequenceV1 {
        self.domain_sequence
    }

    /// Return the separately admitted checkpoint's claimed store transaction.
    /// Native custody must bind this claim to the actual atomic insertion stamp;
    /// the domain sequence and this coordinate have different owners.
    #[must_use]
    pub const fn committed_transaction_sequence(&self) -> ControlTransactionSequenceV1 {
        self.committed_transaction_sequence
    }

    /// Borrow the immutable checkpoint record coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Test whether this head is the exact successor of `predecessor`.
    ///
    /// A successor preserves domain and epoch, advances sequence exactly once,
    /// and names a different immutable checkpoint record.
    #[must_use]
    pub fn is_exact_successor_of(&self, predecessor: &Self) -> bool {
        predecessor.checkpoint_domain_id == self.checkpoint_domain_id
            && predecessor.domain_epoch == self.domain_epoch
            && predecessor.domain_sequence.checked_successor() == Some(self.domain_sequence)
            && predecessor.committed_transaction_sequence.get()
                <= self.committed_transaction_sequence.get()
            && predecessor.coordinate != self.coordinate
    }
}

/// Lowercase SHA-256 spelling used to compare heterogeneous control records.
///
/// Each semantic family retains its distinct digest type before this physical
/// boundary. This wrapper erases only the family type, not the bytes or role.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct ControlRecordDigest(String);

impl<'de> serde::Deserialize<'de> for ControlRecordDigest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

impl ControlRecordDigest {
    /// Admit one exact lowercase hexadecimal SHA-256 spelling.
    ///
    /// # Errors
    ///
    /// Returns [`ControlRecordError::InvalidDigest`] unless `value` has exactly
    /// 64 lowercase hexadecimal bytes.
    pub fn parse(value: impl Into<String>) -> Result<Self, ControlRecordError> {
        let value = value.into();
        if pale_core::is_sha256_hex(&value) {
            Ok(Self(value))
        } else {
            Err(ControlRecordError::InvalidDigest)
        }
    }

    /// Borrow the exact lowercase hexadecimal spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Exact immutable reference and heterogeneous digest retained by a store.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlRecordCoordinate {
    record_ref: ResidentRecordRefV1,
    record_sha256: ControlRecordDigest,
}

impl ControlRecordCoordinate {
    /// Pair one immutable reference with the digest supplied by its record family.
    #[must_use]
    pub const fn new(record_ref: ResidentRecordRefV1, record_sha256: ControlRecordDigest) -> Self {
        Self {
            record_ref,
            record_sha256,
        }
    }

    /// Borrow the immutable record reference.
    #[must_use]
    pub const fn record_ref(&self) -> &ResidentRecordRefV1 {
        &self.record_ref
    }

    /// Borrow the heterogeneous physical comparison digest.
    #[must_use]
    pub const fn record_sha256(&self) -> &ControlRecordDigest {
        &self.record_sha256
    }
}

/// Non-empty opaque record bytes retained for physical custody or recovery.
///
/// Construction proves neither canonicality nor signed-record admission; those
/// checks belong to the application owner before custody or recovered use.
///
/// Debug formatting reports only byte length: retained records may contain
/// private request payloads. Explicit byte access remains available through
/// [`Self::as_slice`] for custody and contextual admission.
#[derive(Clone, PartialEq, Eq)]
pub struct ControlRecordBytes(Vec<u8>);

impl std::fmt::Debug for ControlRecordBytes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ControlRecordBytes")
            .field("byte_len", &self.0.len())
            .finish_non_exhaustive()
    }
}

impl ControlRecordBytes {
    /// Retain one non-empty byte string without interpreting it.
    ///
    /// # Errors
    ///
    /// Returns [`ControlRecordError::EmptyBytes`] for an empty value.
    /// Canonicality and signed-record admission remain upstream.
    pub fn new(bytes: Vec<u8>) -> Result<Self, ControlRecordError> {
        if bytes.is_empty() {
            Err(ControlRecordError::EmptyBytes)
        } else {
            Ok(Self(bytes))
        }
    }

    /// Borrow the complete retained opaque bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

/// One record's coordinates, schema metadata, and bytes for physical custody.
///
/// Callers must admit prepared records upstream and re-admit recovered fields
/// before semantic use. This container only retains the supplied fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRecord {
    coordinate: ControlRecordCoordinate,
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    canonical_record_jcs: ControlRecordBytes,
}

impl ControlRecord {
    /// Bind identity, selected schema, and retained bytes as one physical row.
    #[must_use]
    pub const fn new(
        coordinate: ControlRecordCoordinate,
        schema_id: SchemaId,
        schema_sha256: SchemaDigest,
        canonical_record_jcs: ControlRecordBytes,
    ) -> Self {
        Self {
            coordinate,
            schema_id,
            schema_sha256,
            canonical_record_jcs,
        }
    }

    /// Borrow the immutable reference/digest pair.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Borrow the signed record's exact schema identity.
    #[must_use]
    pub const fn schema_id(&self) -> &SchemaId {
        &self.schema_id
    }

    /// Borrow the exact selected schema digest.
    #[must_use]
    pub const fn schema_sha256(&self) -> &SchemaDigest {
        &self.schema_sha256
    }

    /// Borrow the complete retained record bytes.
    #[must_use]
    pub const fn canonical_record_jcs(&self) -> &ControlRecordBytes {
        &self.canonical_record_jcs
    }
}

/// Physical record construction failed before a custody candidate existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ControlRecordError {
    /// A heterogeneous record digest is not exact lowercase SHA-256.
    #[error("control record digest is not lowercase SHA-256")]
    InvalidDigest,
    /// The retained record bytes are empty.
    #[error("control record bytes are empty")]
    EmptyBytes,
}

/// Stable error class shared by physical control-store adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlStoreErrorKind {
    /// The requested read or write exceeds its caller-supplied byte limit.
    BoundExceeded,
    /// Another complete candidate owns the selected head.
    CandidateConflict,
    /// Partial, corrupt, or wrongly bound physical state needs explicit recovery.
    RecoveryRequired,
    /// A commit failure proves neither success nor rollback.
    OutcomeUnknown,
    /// A valid logical transaction head has no representable successor.
    /// Exact readback and replay do not require a successor and remain possible.
    TransactionExhausted,
    /// The physical store was unavailable or failed before commit ambiguity.
    Storage,
}

/// Typed physical-store failure preserving its native source when one exists.
#[derive(Debug, thiserror::Error)]
#[error("control store failure ({kind:?})")]
pub struct ControlStoreError {
    kind: ControlStoreErrorKind,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ControlStoreError {
    /// Construct a source-free physical classification.
    #[must_use]
    pub fn new(kind: ControlStoreErrorKind) -> Self {
        Self { kind, source: None }
    }

    /// Retain one adapter-native source behind a stable physical classification.
    pub fn with_source<E>(kind: ControlStoreErrorKind, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind,
            source: Some(Box::new(source)),
        }
    }

    /// Return the stable physical failure class.
    #[must_use]
    pub const fn kind(&self) -> ControlStoreErrorKind {
        self.kind
    }
}

#[cfg(test)]
#[path = "../tests/internal/control_store.rs"]
mod tests;
