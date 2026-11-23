//! Strict versioned envelope encoding for expiring records.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pale_core::canon;
use pale_core::engine_wire::ExpiringRecordEnvelopeV1;
use pale_core::records::{ArtifactDigest, ExpiringRecordContract};
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::PublicationClass;
use crate::errors::{data_error, invariant, FsError};

/// Exact persisted-envelope version admitted by this initial-v1 adapter.
const SCHEMA_VERSION: u32 = 1;

/// Domain state plus the temporal metadata authenticated by its envelope.
pub(super) struct Decoded<T> {
    /// Monotonic publication generation retained across expiry.
    pub(super) publication_generation: u64,
    /// Wall-clock publication time used only for bounded retention.
    pub(super) updated: UnixTimestamp,
    /// Caller-owned domain state.
    pub(super) state: T,
}

/// Validated Unix wall-clock timestamp with nanosecond precision.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct UnixTimestamp {
    seconds: u64,
    subsec_nanos: u32,
}

/// Retention classification relative to one captured wall clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Freshness {
    /// The record remains inside its declared retention window.
    Fresh,
    /// The record reached or exceeded its declared retention window.
    Expired,
    /// Ordering cannot be established because the record is ahead of the clock.
    FutureDated,
}

impl UnixTimestamp {
    /// Capture the current Unix wall clock.
    pub(super) fn now() -> Result<Self, FsError> {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|source| data_error("read system clock", "expiring-record", source))?;
        Ok(Self {
            seconds: duration.as_secs(),
            subsec_nanos: duration.subsec_nanos(),
        })
    }

    fn from_persisted<T>(
        persisted: &ExpiringRecordEnvelopeV1<T>,
        path: &Path,
        publication: PublicationClass,
    ) -> Result<Self, FsError> {
        if persisted.updated_subsec_nanos >= 1_000_000_000 {
            return Err(invariant(
                publication.vocabulary().decode_timestamp_operation,
                path,
                "subsecond nanoseconds are outside their valid range",
            ));
        }
        Ok(Self {
            seconds: persisted.updated_unix_seconds,
            subsec_nanos: persisted.updated_subsec_nanos,
        })
    }

    /// Classify `earlier` against this timestamp and an exact expiry boundary.
    pub(super) fn freshness(self, earlier: Self, ttl: Duration) -> Freshness {
        if self < earlier {
            return Freshness::FutureDated;
        }
        let seconds = self.seconds - earlier.seconds;
        let (seconds, nanos) = if self.subsec_nanos >= earlier.subsec_nanos {
            (seconds, self.subsec_nanos - earlier.subsec_nanos)
        } else {
            (
                seconds.saturating_sub(1),
                1_000_000_000 + self.subsec_nanos - earlier.subsec_nanos,
            )
        };
        if Duration::new(seconds, nanos) >= ttl {
            Freshness::Expired
        } else {
            Freshness::Fresh
        }
    }
}

/// Strictly decode and authenticate one record envelope for its exact store binding.
pub(super) fn decode<T: Serialize + DeserializeOwned>(
    bytes: &[u8],
    path: &Path,
    publication: PublicationClass,
    expected_binding: &ArtifactDigest,
    contract: &ExpiringRecordContract,
) -> Result<Decoded<T>, FsError> {
    let vocabulary = publication.vocabulary();
    let value = canon::parse_strict(bytes)
        .map_err(|source| data_error(vocabulary.parse_operation, path, source))?;
    let persisted: ExpiringRecordEnvelopeV1<T> = serde_json::from_value(value)
        .map_err(|source| data_error(vocabulary.decode_operation, path, source))?;
    if persisted.schema_version == SCHEMA_VERSION {
        if persisted.schema != contract.envelope.schema
            || persisted.schema_sha256 != contract.envelope.schema_sha256
            || persisted.schema_manifest_sha256 != contract.schema_registry.schema_manifest_sha256
            || persisted.schema_set_sha256 != contract.schema_registry.schema_set_sha256
            || persisted.compilation_input_digest != contract.compilation_input_digest
            || persisted.payload_schema != contract.payload.schema
            || persisted.payload_schema_sha256 != contract.payload.schema_sha256
        {
            return Err(invariant(
                vocabulary.decode_operation,
                path,
                "record contract identity does not match the admitted compilation inputs",
            ));
        }
        if persisted.record_class != publication.wire_class() {
            return Err(invariant(
                vocabulary.decode_operation,
                path,
                "record publication class does not match this store",
            ));
        }
        if &persisted.record_binding != expected_binding {
            return Err(invariant(
                vocabulary.decode_operation,
                path,
                "record binding does not match this confined root and path",
            ));
        }
        let canonical = canon::to_canonical_string(&persisted)
            .map_err(|source| data_error(vocabulary.decode_operation, path, source))?;
        if canonical.as_bytes() != bytes {
            return Err(invariant(
                vocabulary.decode_operation,
                path,
                "record bytes are not canonical",
            ));
        }
    } else {
        return Err(invariant(
            vocabulary.decode_operation,
            path,
            format!("unsupported schema version {}", persisted.schema_version),
        ));
    }
    let updated = UnixTimestamp::from_persisted(&persisted, path, publication)?;
    Ok(Decoded {
        publication_generation: persisted.publication_generation,
        updated,
        state: persisted.state,
    })
}

/// Canonically encode one current-version record envelope.
pub(super) fn encode<T: Serialize>(
    publication_generation: u64,
    updated: UnixTimestamp,
    state: T,
    path: &Path,
    publication: PublicationClass,
    record_binding: &ArtifactDigest,
    contract: &ExpiringRecordContract,
) -> Result<Vec<u8>, FsError> {
    let persisted = ExpiringRecordEnvelopeV1 {
        schema: contract.envelope.schema.clone(),
        schema_sha256: contract.envelope.schema_sha256.clone(),
        schema_version: SCHEMA_VERSION,
        schema_manifest_sha256: contract.schema_registry.schema_manifest_sha256.clone(),
        schema_set_sha256: contract.schema_registry.schema_set_sha256.clone(),
        compilation_input_digest: contract.compilation_input_digest.clone(),
        payload_schema: contract.payload.schema.clone(),
        payload_schema_sha256: contract.payload.schema_sha256.clone(),
        record_class: publication.wire_class(),
        record_binding: record_binding.clone(),
        publication_generation,
        updated_unix_seconds: updated.seconds,
        updated_subsec_nanos: updated.subsec_nanos,
        state,
    };
    canon::to_canonical_string(&persisted)
        .map(String::into_bytes)
        .map_err(|source| data_error(publication.vocabulary().encode_operation, path, source))
}
