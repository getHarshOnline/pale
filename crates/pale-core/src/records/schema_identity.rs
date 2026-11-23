//! Distinct identities for wire records, descriptors, and active schema sets.
//!
//! A [`SchemaId`] names the contract carried by a record. It is deliberately
//! not interchangeable with a descriptor registry key, raw schema digest,
//! manifest digest, or [`SchemaRegistryIdentity`].

use serde::{Deserialize, Deserializer, Serialize};

use super::{
    CompilationInputDigest, OneShotCompilationDigest, SchemaDigest, SchemaManifestDigest,
    SchemaSetDigest,
};

/// Maximum admitted byte length of an untrusted schema identity.
///
/// The bound applies before the identity enters maps, diagnostics, or
/// generated metadata and is part of the wire grammar.
const MAX_SCHEMA_ID_BYTES: usize = 256;

/// A wire schema identity is malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "schema identity must be 1..={MAX_SCHEMA_ID_BYTES} ASCII bytes, begin and end with an \
     ASCII alphanumeric character, and contain only lowercase alphanumerics, '.', '-', '_', or '/'"
)]
pub struct InvalidSchemaId;

/// Canonical wire identity carried by one record.
///
/// This is deliberately distinct from the shorter descriptor-registry key used
/// to select an embedded JSON Schema. Admission always begins with the expected
/// registry key and then compares this value with that descriptor's exact
/// generated `wire_schema_id`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct SchemaId(
    #[schemars(
        length(min = 1, max = 256),
        regex(pattern = r"^[a-z0-9](?:[a-z0-9._/-]*[a-z0-9])?$")
    )]
    String,
);

impl SchemaId {
    /// Validate a wire schema identity received across a trust boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use pale_core::records::SchemaId;
    ///
    /// let id = SchemaId::parse("pale/example.v1")?;
    /// assert_eq!(id.as_str(), "pale/example.v1");
    /// assert!(SchemaId::parse("Pale/example.v1").is_err());
    /// # Ok::<(), pale_core::records::InvalidSchemaId>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`InvalidSchemaId`] for an empty, oversized, non-ASCII, uppercase,
    /// control-bearing, or otherwise non-canonical value.
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidSchemaId> {
        let value = value.into();
        let bytes = value.as_bytes();
        let valid_edge = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
        if bytes.is_empty()
            || bytes.len() > MAX_SCHEMA_ID_BYTES
            || !valid_edge(bytes[0])
            || !valid_edge(bytes[bytes.len() - 1])
            || !bytes.iter().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'-' | b'_' | b'/')
            })
        {
            return Err(InvalidSchemaId);
        }
        Ok(Self(value))
    }

    /// Borrow the canonical wire spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for SchemaId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for SchemaId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for SchemaId {
    type Error = InvalidSchemaId;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for SchemaId {
    type Error = InvalidSchemaId;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

/// Schema-identity claim carried by a standalone persisted or cross-process record.
///
/// Owning records declare these two typed members directly and may construct this
/// value for admission only when the canonical descriptor declares one exact wire
/// schema identity. The helper is intentionally not a Serde-flatten contract:
/// strict outer records retain responsibility for their exact member set. Shape
/// validation never substitutes for comparing both members with the descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordSchemaIdentity {
    /// Claimed canonical wire schema ID serialized as the record's `schema` member.
    pub schema: SchemaId,
    /// Claimed digest of the raw JSON Schema bytes selected by the descriptor.
    pub schema_sha256: SchemaDigest,
}

/// Coordinate pair for a descriptor manifest and its active raw-schema set.
///
/// The set digest alone does not bind descriptor authority, lifecycle,
/// persistence, compatibility, or migration metadata, so the two values always
/// travel as one typed pair at compilation-root boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaRegistryIdentity {
    /// RFC 8785 canonical descriptor-manifest digest.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Domain-separated active raw-schema-set digest.
    pub schema_set_sha256: SchemaSetDigest,
}

/// Contract coordinate required to admit materialization state and lineage.
///
/// The application binds the exact record schemas and registry selected by one
/// compilation. Filesystem adapters consume this coordinate while observing
/// persisted bytes; they do not select schemas or mint compilation identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationContractIdentity {
    /// Exact schema identity required for repository-local ownership state.
    pub state_schema: RecordSchemaIdentity,
    /// Exact schema identity required for machine-local ownership lineage.
    pub lineage_schema: RecordSchemaIdentity,
    /// Registry coordinate that admits both record schemas.
    pub schema_registry: SchemaRegistryIdentity,
    /// Complete one-shot compilation identity bound into both records.
    pub one_shot_compilation_digest: OneShotCompilationDigest,
}

impl MaterializationContractIdentity {
    /// Bind exact state, lineage, registry, and compilation coordinates.
    #[must_use]
    pub fn new(
        state_schema: RecordSchemaIdentity,
        lineage_schema: RecordSchemaIdentity,
        schema_registry: SchemaRegistryIdentity,
        one_shot_compilation_digest: OneShotCompilationDigest,
    ) -> Self {
        Self {
            state_schema,
            lineage_schema,
            schema_registry,
            one_shot_compilation_digest,
        }
    }
}

/// Contract coordinates carried for one expiring cross-process record.
///
/// The adapter owns confinement, publication and retention. Its caller owns the
/// selected envelope and payload contracts plus the compilation inputs under
/// which those bytes may be interpreted. Keeping all four coordinates together
/// prevents an old payload from being decoded merely because its path survived
/// a schema or selected-package-content change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpiringRecordContract {
    /// Standalone schema identity of the adapter-owned persistence envelope.
    pub envelope: RecordSchemaIdentity,
    /// Standalone schema identity of the domain payload carried in the `state` member.
    pub payload: RecordSchemaIdentity,
    /// Descriptor-manifest and raw-schema-set identity of the compiling runtime.
    pub schema_registry: SchemaRegistryIdentity,
    /// Non-circular identity of the exact pre-render compilation inputs.
    pub compilation_input_digest: CompilationInputDigest,
}
