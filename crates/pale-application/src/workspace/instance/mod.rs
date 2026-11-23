//! Machine-bound instance resolution and filesystem-backed verification.

mod authority;
mod executable_identity;
mod harnesses;
mod resolution;

pub(crate) use authority::{verify_durable_evidence_root, verify_reading_order};
pub use authority::{
    verify_materialization_lineage_binding, verify_materialization_lineage_root,
    verify_session_state_root,
};
pub(crate) use executable_identity::verify_instance_executable_binding;
pub use executable_identity::verify_process_executable_binding;
pub use harnesses::verify_native_harness;
pub(crate) use harnesses::verify_native_harnesses;
pub(crate) use resolution::resolve_instance;
