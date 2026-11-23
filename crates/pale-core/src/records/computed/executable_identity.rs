//! Executable and interface identity records.
//!
//! Decoded values are claims. A consuming boundary binds them to an observed
//! executable and successful identity probe before treating them as evidence of
//! byte identity or interface compatibility. They do not report process health,
//! resident liveness, activation, or readiness.

use serde::{Deserialize, Serialize};

use crate::platform::DurablePublication;
use crate::records::SchemaId;

/// Protocol token carried by [`SurfaceExecutableIdentityV1::protocol`] and
/// checked before using the selected executable.
///
/// Matching this token is necessary compatibility evidence, not installation or
/// activation proof.
pub const SURFACE_INTERFACE_PROTOCOL: &str = "pale.agent-surface-interface.v1";

/// Machine-readable interface claim compiled into a Pale executable.
///
/// When emitted by the running executable, the mechanism is an implementation
/// fact rather than a hardware promise. Deserialization alone does not establish
/// the emitter's identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SurfaceExecutableIdentityV1 {
    /// Stable v1 wire identity.
    #[schemars(
        with = "String",
        extend("const" = "pale/surface-executable-identity.v1")
    )]
    pub schema: SchemaId,
    /// SHA-256 of the exact JSON Schema bytes selected for this record.
    pub schema_sha256: crate::records::SchemaDigest,
    /// Executable-identity schema major.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Pale release version reported by the executable.
    pub version: String,
    /// Command and record protocol implemented by the executable.
    pub protocol: String,
    /// Operating-system identity compiled into the executable.
    pub os: String,
    /// CPU architecture identity compiled into the executable.
    pub arch: String,
    /// Platform-specific durability primitive used for atomic publication.
    pub durable_publication: DurablePublication,
}
