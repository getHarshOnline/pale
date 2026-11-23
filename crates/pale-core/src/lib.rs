//! Core domain records and deterministic primitives for Pale agent surfaces.
//!
//! The engine, adapter and SDK crates build on this one, and the interfaces and
//! language bindings reach it through `pale-sdk`; it builds on no other Pale
//! crate. It owns wire records, hashing, RFC 8785 canonicalization, strict JSON
//! ingestion, and lexical path contracts. It is dependency-light and has no
//! ambient filesystem, process, or network access: the adapter crates own those
//! boundaries.

pub mod admission;
pub mod canon;
pub mod engine_wire;
pub mod managed;
pub mod paths;
pub mod platform;
pub mod records;

mod hash;

pub use hash::{is_sha256_hex, sha256_hex, sha256_reader};
pub use managed::{MANAGED_BEGIN, MANAGED_END};
pub use records::SURFACE_INTERFACE_PROTOCOL;
