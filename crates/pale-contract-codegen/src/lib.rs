//! Deterministic generation of the private Pale contract bundle.
//!
//! This crate builds on the wire records of `pale-core`. `pale-contracts` runs
//! it from its build script, and maintainers run its command line to
//! regenerate or verify the checked-in bundle.
//!
//! Rust wire types own portable JSON shape and structural constraints. The
//! descriptor manifest independently owns registration, lifecycle, canonical
//! resource identity, and the active schema map. This crate joins those two
//! authorities without repairing either one: any disagreement rejects the
//! complete generation before bytes reach a caller-selected directory.

mod manifest;
mod schema_catalog;
mod schema_projection;

pub use manifest::{
    generate_authoring_schemas, generate_bundle, read_descriptor_manifest, verify_bundle,
    write_bundle_atomically, BundleCoordinate, ContractBundle, DescriptorManifest,
    GeneratedSchemaDescriptor, GenerationError, BUNDLE_MANIFEST_NAME,
};
