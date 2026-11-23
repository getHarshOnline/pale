//! Identity-bound preparation and recovery for bounded machine-local effects.
//!
//! Observation is write-free. Publication revalidates the admitted observation,
//! journals exact identities, and remains reversible until explicit completion.
//! Individual leaves are synchronized; a multi-effect sequence is not crash-atomic.

mod observation;
mod transaction;

pub use observation::{observe_machine_local_file, MachineLocalFileObservation};
pub use transaction::{
    prepare_machine_local_file, AppliedMachineLocalFile, MachineLocalFileIntent,
    PreparedMachineLocalFile,
};
