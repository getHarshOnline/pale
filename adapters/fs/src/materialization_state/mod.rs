//! Read materialization records and verify planned ownership changes.

mod read;
mod transition;

pub(crate) use read::{lineage_relative, read_trusted_from_roots};
pub use read::{read_materialization_state, target_root_sha256};
pub(crate) use transition::verify_transition;
