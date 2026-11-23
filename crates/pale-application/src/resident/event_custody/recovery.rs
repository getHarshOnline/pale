//! Bounded recovery of pending Pale-derived publications.

use pale_core::canon;
use pale_core::records::{ArtifactDigest, ResidentRecordRefV1};
use pale_ports::{
    ControlRecord, ControlRecordCoordinate, ControlStoreError, EventCustodyPort,
    EventTransitionMemberKind, PaleDerivedOutboxCursor, PaleDerivedOutboxPageBoundary,
    PaleDerivedOutboxPageRead,
};
use serde::{Deserialize, Serialize};

use super::projection::pale_derived_outbox_key;
use super::record::RetentionDisposition;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PaleDerivedRetentionWire {
    publication_ref: ResidentRecordRefV1,
    publication_sha256: ArtifactDigest,
    disposition: RetentionDisposition,
}

/// One pending derived publication recovered under its exact retry identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPaleDerivedPublication {
    cursor: PaleDerivedOutboxCursor,
    retention_record: ControlRecordCoordinate,
    publication_ref: ResidentRecordRefV1,
    publication_sha256: ArtifactDigest,
}

impl PendingPaleDerivedPublication {
    /// Borrow the stable page cursor and application-derived publication key.
    #[must_use]
    pub const fn cursor(&self) -> &PaleDerivedOutboxCursor {
        &self.cursor
    }

    /// Borrow the exact private retention-record coordinate.
    #[must_use]
    pub const fn retention_record(&self) -> &ControlRecordCoordinate {
        &self.retention_record
    }

    /// Borrow the immutable publication reference used for idempotent exposure.
    #[must_use]
    pub const fn publication_ref(&self) -> &ResidentRecordRefV1 {
        &self.publication_ref
    }

    /// Borrow the exact publication digest used for idempotent exposure.
    #[must_use]
    pub const fn publication_sha256(&self) -> &ArtifactDigest {
        &self.publication_sha256
    }
}

/// Pending entries from one physical current-head page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPaleDerivedPublicationPage {
    entries: Box<[PendingPaleDerivedPublication]>,
    boundary: PaleDerivedOutboxPageBoundary,
}

impl PendingPaleDerivedPublicationPage {
    /// Borrow pending publications in stable cursor order.
    #[must_use]
    pub const fn entries(&self) -> &[PendingPaleDerivedPublication] {
        &self.entries
    }

    /// Borrow the physical page boundary, including pages containing only settled heads.
    #[must_use]
    pub const fn boundary(&self) -> &PaleDerivedOutboxPageBoundary {
        &self.boundary
    }
}

/// Recovery could not prove one exact pending Pale-derived publication page.
#[derive(Debug, thiserror::Error)]
pub enum EventCustodyRecoveryError {
    /// The existing custody owner could not complete the bounded physical read.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// A current head failed private schema, canonical-byte, digest, or key checks.
    #[error("Pale-derived outbox head is not an exact admitted retention record")]
    InvalidRetentionRecord,
}

/// Recover pending Pale-derived publications from one bounded current-head page.
///
/// A pending entry is retry evidence, not a native-effect grant. Repeated
/// sweeps retain the same publication reference and digest until settlement,
/// so the downstream owner can make exposure idempotent at that coordinate.
///
/// # Errors
///
/// Returns [`EventCustodyRecoveryError`] when the physical page is unavailable
/// or a head fails exact private-record admission.
pub fn read_pending_pale_derived_publications(
    store: &mut dyn EventCustodyPort,
    request: &PaleDerivedOutboxPageRead,
) -> Result<PendingPaleDerivedPublicationPage, EventCustodyRecoveryError> {
    let page = store.read_pale_derived_outbox_page(request)?;
    let mut pending = Vec::with_capacity(page.entries().len());
    for entry in page.entries() {
        let wire = admit_retention_record(entry.record(), entry.cursor())?;
        if matches!(wire.disposition, RetentionDisposition::Retained) {
            pending.push(PendingPaleDerivedPublication {
                cursor: entry.cursor().clone(),
                retention_record: entry.record().coordinate().clone(),
                publication_ref: wire.publication_ref,
                publication_sha256: wire.publication_sha256,
            });
        }
    }
    Ok(PendingPaleDerivedPublicationPage {
        entries: pending.into_boxed_slice(),
        boundary: page.boundary().clone(),
    })
}

fn admit_retention_record(
    record: &ControlRecord,
    cursor: &PaleDerivedOutboxCursor,
) -> Result<PaleDerivedRetentionWire, EventCustodyRecoveryError> {
    let invalid = || EventCustodyRecoveryError::InvalidRetentionRecord;
    let required_schema = EventTransitionMemberKind::PaleDerivedOutbox.required_schema_id();
    let bytes = record.canonical_record_jcs().as_slice();
    if record.schema_id().as_str() != required_schema
        || record.schema_sha256().as_str() != pale_core::sha256_hex(required_schema.as_bytes())
        || record.coordinate().record_sha256().as_str() != pale_core::sha256_hex(bytes)
    {
        return Err(invalid());
    }
    let value = canon::parse_strict(bytes).map_err(|_| invalid())?;
    let wire: PaleDerivedRetentionWire = serde_json::from_value(value).map_err(|_| invalid())?;
    let canonical = canon::to_canonical_string(&wire).map_err(|_| invalid())?;
    if canonical.as_bytes() != bytes
        || cursor.publication_key()
            != &pale_derived_outbox_key(&wire.publication_ref, &wire.publication_sha256)
                .map_err(|_| invalid())?
    {
        return Err(invalid());
    }
    Ok(wire)
}
