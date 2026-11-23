//! Offline contract identity, registry, and structural validation for Pale.
//!
//! This crate builds on `pale-core` and, at build time, on
//! `pale-contract-codegen`. `pale-application` and `pale-sdk` validate
//! against its registry.
//!
//! Rust wire producers own portable shape and structural constraints. The
//! descriptor manifest independently owns registration and lifecycle identity;
//! the checked-in crate-local JSON Schemas are deterministic sealed projections
//! verified again during each build. Runtime validation performs no filesystem,
//! process, network, activation, or native-owner effect.
//!
//! # Basic usage
//!
//! ```
//! use pale_contracts::validate_value;
//!
//! let digest = serde_json::Value::String("a".repeat(64));
//! validate_value(&digest, "sha256-digest.v1")?;
//! # Ok::<(), pale_contracts::SchemaError>(())
//! ```

mod registry;

pub use registry::{
    admit_record_schema_identity, authoring_schema, embedded_schema, record_schema_identity,
    record_schema_identity_by_wire_id, schema_count, schema_descriptor, schema_registry_identity,
    validate_definition, validate_definition_record, validate_record, validate_value,
    validate_value_with_root_branch, AuthoringSchema, CompiledSchema, SchemaCompilationError,
    SchemaDescriptor, SchemaError, SchemaInstancePathSegment, SchemaViolation, SchemaViolations,
};
