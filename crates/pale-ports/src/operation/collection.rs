//! Private physical collection-frontier binding, not consultation policy.

use std::sync::OnceLock;

use pale_core::{canon, records::*};
use schemars::transform::{RecursiveTransform, Transform};
use serde::{Deserialize, Serialize};

use crate::{
    AuthorityStoreCoordinate, BoundedAuthorityEntryBytes, ControlRecord, ControlRecordBytes,
    ControlRecordCoordinate, ControlRecordDigest, ControlTransactionPreparation,
    OperationCustodyIndexKey, OperationCustodyIndexQuery, PreparedAuthorityAppend,
};

/// Private physical encoding, deliberately absent from the public contract registry.
pub const PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA: &str =
    "pale/private-operation-collection-frontier.v1";

const PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_REFERENCE_DOMAIN: &str =
    "pale/private-operation-collection-frontier/v1";

// This is a custody projection of existing typed coordinates. In particular,
// it does not interpret the application-private journal entry as an admitted
// roster, or infer permission from the operation-handle coordinate.
#[derive(Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct CollectionFrontier {
    #[schemars(extend("const" = "PALE_OPERATION_COLLECTION_FRONTIER_V1"))]
    domain: String,
    control_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
    operation_handle_ref: ResidentRecordRefV1,
    operation_handle_sha256: ArtifactDigest,
    expected_transaction_sequence: ControlTransactionSequenceV1,
    committed_transaction_sequence: ControlTransactionSequenceV1,
    expected_journal_sequence: Option<JournalSequenceV1>,
    journal_sequence: JournalSequenceV1,
    journal_payload_sha256: ArtifactDigest,
}

/// Physical coordinates extracted from one admitted collection frontier.
///
/// This value identifies the journal entry to read. It does not admit the
/// consultation roster or grant permission to use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCollectionFrontierEvidence {
    frontier: ControlRecordCoordinate,
    operation_handle: ControlRecordCoordinate,
    journal_sequence: JournalSequenceV1,
    journal_payload_sha256: ArtifactDigest,
    committed_transaction_sequence: ControlTransactionSequenceV1,
}

impl OperationCollectionFrontierEvidence {
    /// Admit one private frontier for an exact store and operation handle.
    ///
    /// # Errors
    /// Rejects the wrong schema, store, handle, byte-derived reference, digest,
    /// canonical bytes, or bound.
    pub fn admit(
        frontier: &ControlRecord,
        store: &AuthorityStoreCoordinate,
        operation_handle: &ControlRecordCoordinate,
        maximum_frontier_bytes: usize,
    ) -> Result<Self, OperationCollectionJournalError> {
        let bytes = frontier.canonical_record_jcs().as_slice();
        if maximum_frontier_bytes == 0 || bytes.len() > maximum_frontier_bytes {
            return Err(OperationCollectionJournalError::BoundExceeded);
        }
        if frontier.schema_id().as_str() != PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA
            || frontier.schema_sha256() != &OperationCollectionJournal::schema_digest()?
        {
            return Err(OperationCollectionJournalError::FrontierSchema);
        }
        let frontier_digest = ArtifactDigest::hash_bytes(bytes);
        if frontier.coordinate().record_sha256().as_str() != frontier_digest.as_str() {
            return Err(OperationCollectionJournalError::FrontierDigest);
        }
        let expected_reference = collection_frontier_reference(&frontier_digest)?;
        if frontier.coordinate().record_ref() != &expected_reference {
            return Err(OperationCollectionJournalError::Reference);
        }
        let wire = CollectionFrontier::deserialize(
            canon::parse_strict(bytes).map_err(|_| OperationCollectionJournalError::Encoding)?,
        )
        .map_err(|_| OperationCollectionJournalError::Encoding)?;
        let canonical = canon::to_canonical_string(&wire)
            .map_err(|_| OperationCollectionJournalError::Encoding)?;
        let journal_advances = wire.expected_journal_sequence.map_or(
            wire.journal_sequence.get() == 1,
            |expected: JournalSequenceV1| {
                expected.checked_successor() == Some(wire.journal_sequence)
            },
        );
        if canonical.as_bytes() != bytes
            || wire.domain != "PALE_OPERATION_COLLECTION_FRONTIER_V1"
            || &wire.control_store_id != store.store_id()
            || &wire.control_incarnation_id != store.incarnation_id()
            || &wire.operation_handle_ref != operation_handle.record_ref()
            || wire.operation_handle_sha256.as_str() != operation_handle.record_sha256().as_str()
            || wire.expected_transaction_sequence.get().checked_add(1)
                != Some(wire.committed_transaction_sequence.get())
            || !journal_advances
        {
            return Err(OperationCollectionJournalError::Association);
        }
        Ok(Self {
            frontier: frontier.coordinate().clone(),
            operation_handle: operation_handle.clone(),
            journal_sequence: wire.journal_sequence,
            journal_payload_sha256: wire.journal_payload_sha256,
            committed_transaction_sequence: wire.committed_transaction_sequence,
        })
    }

    /// Borrow the exact frontier admitted from the private record bytes.
    #[must_use]
    pub const fn frontier(&self) -> &ControlRecordCoordinate {
        &self.frontier
    }

    /// Borrow the exact operation handle used during frontier admission.
    #[must_use]
    pub const fn operation_handle(&self) -> &ControlRecordCoordinate {
        &self.operation_handle
    }

    /// Return the exact entry in this operation handle’s collection journal.
    #[must_use]
    pub const fn journal_sequence(&self) -> JournalSequenceV1 {
        self.journal_sequence
    }

    /// Borrow the expected digest of the unframed journal payload.
    #[must_use]
    pub const fn journal_payload_sha256(&self) -> &ArtifactDigest {
        &self.journal_payload_sha256
    }

    /// Return the operation transaction that retained the frontier.
    #[must_use]
    pub const fn committed_transaction_sequence(&self) -> ControlTransactionSequenceV1 {
        self.committed_transaction_sequence
    }
}

/// Exact collection-head point read returned by the physical custody owner.
///
/// The application must compare the query with the requested operation handle
/// and the coordinate with the returned snapshot. This value reports what the
/// trusted port read atomically; it is not a signature or an independent
/// freshness proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCollectionHeadObservation {
    query: OperationCustodyIndexQuery,
    current_frontier: ControlRecordCoordinate,
}

impl OperationCollectionHeadObservation {
    /// Record the exact handle-scoped collection-head point read.
    #[must_use]
    pub fn for_handle(
        operation_handle: &ControlRecordCoordinate,
        current_frontier: ControlRecordCoordinate,
    ) -> Self {
        Self {
            query: OperationCustodyIndexQuery::CollectionHead(
                OperationCustodyIndexKey::for_collection_head(operation_handle),
            ),
            current_frontier,
        }
    }

    /// Borrow the exact private-index query answered by the port.
    #[must_use]
    pub const fn query(&self) -> &OperationCustodyIndexQuery {
        &self.query
    }

    /// Borrow the coordinate observed at the collection-head index.
    #[must_use]
    pub const fn current_frontier(&self) -> &ControlRecordCoordinate {
        &self.current_frontier
    }
}

/// Selected private collection bytes and the actual current head for their operation handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCollectionSnapshot {
    frontier: ControlRecord,
    evidence: OperationCollectionFrontierEvidence,
    head: OperationCollectionHeadObservation,
    journal_payload: BoundedAuthorityEntryBytes,
}

impl OperationCollectionSnapshot {
    /// Bind an admitted frontier and point-read head to its exact payload.
    ///
    /// # Errors
    /// Rejects unlike admitted/frontier coordinates or a payload whose digest
    /// differs from the frontier. The application separately verifies that the
    /// point-read query and current head match its requested operation.
    pub fn new(
        frontier: ControlRecord,
        evidence: &OperationCollectionFrontierEvidence,
        head: OperationCollectionHeadObservation,
        journal_payload: BoundedAuthorityEntryBytes,
    ) -> Result<Self, OperationCollectionJournalError> {
        let evidence_query = OperationCustodyIndexQuery::CollectionHead(
            OperationCustodyIndexKey::for_collection_head(evidence.operation_handle()),
        );
        if frontier.coordinate() != evidence.frontier() || head.query() != &evidence_query {
            return Err(OperationCollectionJournalError::Association);
        }
        if ArtifactDigest::hash_bytes(journal_payload.as_slice())
            != *evidence.journal_payload_sha256()
        {
            return Err(OperationCollectionJournalError::JournalPayload);
        }
        Ok(Self {
            frontier,
            evidence: evidence.clone(),
            head,
            journal_payload,
        })
    }

    /// Borrow the indexed physical frontier.
    #[must_use]
    pub const fn frontier(&self) -> &ControlRecord {
        &self.frontier
    }

    /// Borrow the admitted physical frontier coordinates.
    #[must_use]
    pub const fn evidence(&self) -> &OperationCollectionFrontierEvidence {
        &self.evidence
    }

    /// Borrow the atomic handle-scoped collection-head point observation.
    #[must_use]
    pub const fn head(&self) -> &OperationCollectionHeadObservation {
        &self.head
    }

    /// Borrow the exact application-owned collection bytes.
    #[must_use]
    pub const fn journal_payload(&self) -> &BoundedAuthorityEntryBytes {
        &self.journal_payload
    }
}

/// An exact private frontier and its mandatory, byte-bound journal append.
///
/// Construction proves only this physical binding and its byte-derived private
/// reference. Application admission separately joins the handle and roster to
/// the signed observation or its frozen stop frontier. This value grants no
/// collection, stopping, child release, or policy-satisfaction permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCollectionJournal {
    frontier: ControlRecord,
    operation_handle: ControlRecordCoordinate,
    transaction: ControlTransactionPreparation,
    append: PreparedAuthorityAppend,
    maximum_journal_payload_bytes: usize,
}

/// Refusal before a physical collection attachment exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OperationCollectionJournalError {
    /// A caller-supplied byte ceiling is zero or exceeded.
    #[error("collection custody exceeds its byte limit")]
    BoundExceeded,
    /// The journal and operation transaction name different physical stores.
    #[error("collection custody has unlike store coordinates")]
    Store,
    /// A retained frontier does not belong to the selected store or handle.
    #[error("collection frontier does not belong to the selected store and operation")]
    Association,
    /// A frontier does not use the reference derived from its exact canonical bytes.
    #[error("collection frontier reference differs from its canonical bytes")]
    Reference,
    /// Canonical physical bytes or their source-derived descriptor could not be encoded.
    #[error("collection custody could not be canonically encoded")]
    Encoding,
    /// The retained frontier uses unlike private schema metadata.
    #[error("collection frontier uses the wrong private schema")]
    FrontierSchema,
    /// The retained frontier bytes do not match their physical digest.
    #[error("collection frontier bytes differ from their physical digest")]
    FrontierDigest,
    /// The recovered collection-journal payload differs from its frontier digest.
    #[error("collection journal payload differs from its retained frontier")]
    JournalPayload,
}

impl OperationCollectionJournal {
    /// Derive the frontier from the exact append and physical transaction.
    /// No caller-supplied frontier reference, bytes, digest or schema identity
    /// are accepted.
    ///
    /// # Errors
    /// Rejects unlike stores, invalid byte limits or canonical encoding failure.
    pub fn prepare(
        operation_handle: &ControlRecordCoordinate,
        transaction: ControlTransactionPreparation,
        append: PreparedAuthorityAppend,
        maximum_frontier_bytes: usize,
        maximum_journal_payload_bytes: usize,
    ) -> Result<Self, OperationCollectionJournalError> {
        if maximum_frontier_bytes == 0
            || maximum_journal_payload_bytes == 0
            || append.entry().bytes().as_slice().len() > maximum_journal_payload_bytes
        {
            return Err(OperationCollectionJournalError::BoundExceeded);
        }
        if transaction.store() != append.store() {
            return Err(OperationCollectionJournalError::Store);
        }
        let wire = CollectionFrontier {
            domain: "PALE_OPERATION_COLLECTION_FRONTIER_V1".into(),
            control_store_id: transaction.store().store_id().clone(),
            control_incarnation_id: transaction.store().incarnation_id().clone(),
            operation_handle_ref: operation_handle.record_ref().clone(),
            operation_handle_sha256: ArtifactDigest::parse(
                operation_handle.record_sha256().as_str(),
            )
            .map_err(|_| OperationCollectionJournalError::Encoding)?,
            expected_transaction_sequence: transaction.expected(),
            committed_transaction_sequence: transaction.successor(),
            expected_journal_sequence: append.expected_head(),
            journal_sequence: append.entry().sequence(),
            journal_payload_sha256: ArtifactDigest::hash_bytes(append.entry().bytes().as_slice()),
        };
        // All input coordinates are already bounded value types; the only
        // variable payload was checked before hashing, and is not copied here.
        let bytes = canon::to_canonical_string(&wire)
            .map_err(|_| OperationCollectionJournalError::Encoding)?
            .into_bytes();
        if bytes.len() > maximum_frontier_bytes {
            return Err(OperationCollectionJournalError::BoundExceeded);
        }
        let frontier_digest = ArtifactDigest::hash_bytes(&bytes);
        let frontier_ref = collection_frontier_reference(&frontier_digest)?;
        if &frontier_ref == operation_handle.record_ref() {
            return Err(OperationCollectionJournalError::Reference);
        }
        let frontier = ControlRecord::new(
            ControlRecordCoordinate::new(
                frontier_ref,
                ControlRecordDigest::parse(frontier_digest.as_str())
                    .map_err(|_| OperationCollectionJournalError::Encoding)?,
            ),
            SchemaId::parse(PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA)
                .map_err(|_| OperationCollectionJournalError::Encoding)?,
            Self::schema_digest()?,
            ControlRecordBytes::new(bytes)
                .map_err(|_| OperationCollectionJournalError::Encoding)?,
        );
        Ok(Self {
            frontier,
            operation_handle: operation_handle.clone(),
            transaction,
            append,
            maximum_journal_payload_bytes,
        })
    }

    /// Borrow the exact derived record for the application's signed-reference join.
    #[must_use]
    pub const fn frontier(&self) -> &ControlRecord {
        &self.frontier
    }

    /// Borrow the exact operation handle used to derive its point index.
    #[must_use]
    pub const fn operation_handle(&self) -> &ControlRecordCoordinate {
        &self.operation_handle
    }

    /// Borrow the physical transaction that this pair must join.
    #[must_use]
    pub const fn transaction(&self) -> &ControlTransactionPreparation {
        &self.transaction
    }

    /// Borrow the only journal payload that can accompany this frontier.
    #[must_use]
    pub const fn append(&self) -> &PreparedAuthorityAppend {
        &self.append
    }

    /// Return the caller's original journal-read ceiling, without a fallback.
    #[must_use]
    pub const fn maximum_journal_payload_bytes(&self) -> usize {
        self.maximum_journal_payload_bytes
    }

    fn schema_digest() -> Result<SchemaDigest, OperationCollectionJournalError> {
        static DIGEST: OnceLock<Result<SchemaDigest, OperationCollectionJournalError>> =
            OnceLock::new();
        DIGEST
            .get_or_init(|| {
                let mut schema = schemars::schema_for!(CollectionFrontier);
                // Use Schemars' schema-aware traversal, not a copied JSON walker.
                // Editorial annotations do not change the physical encoding.
                RecursiveTransform(|schema: &mut schemars::Schema| {
                    schema.remove("description");
                    schema.remove("title");
                })
                .transform(&mut schema);
                let canonical = canon::to_canonical_string(&schema)
                    .map_err(|_| OperationCollectionJournalError::Encoding)?;
                SchemaDigest::parse(pale_core::sha256_hex(canonical.as_bytes()))
                    .map_err(|_| OperationCollectionJournalError::Encoding)
            })
            .clone()
    }
}

fn collection_frontier_reference(
    canonical_frontier_sha256: &ArtifactDigest,
) -> Result<ResidentRecordRefV1, OperationCollectionJournalError> {
    ResidentRecordRefV1::parse(format!(
        "{PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_REFERENCE_DOMAIN}/{}",
        canonical_frontier_sha256.as_str()
    ))
    .map_err(|_| OperationCollectionJournalError::Encoding)
}
