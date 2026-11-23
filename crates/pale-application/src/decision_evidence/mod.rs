//! A tamper-evident ledger of the control decisions Pale returned to a coding
//! agent's hooks.
//!
//! Each entry is chained to the one before it by hash, so a later change to
//! history is detectable, and every entry is bounded in size. The ledger builds on
//! `bounded_text`, `operation_admission` and `resident`; no other module in this
//! crate builds on it.

mod current;
mod error;
mod identity;
mod observation;
mod validation;
mod wire;

pub use current::DecisionEvidenceLedger;
pub use error::DecisionEvidenceError;
pub use identity::DecisionLedgerIdentity;
pub use observation::DecisionEvidenceObservation;

// The two v1 identities and these ceilings are one persisted-record contract:
// replay and append validation must enforce the same bounded canonical preimage.
/// Wire identity of the hash-chained ledger record.
pub(super) const SCHEMA_V1: &str = "pale/decision-evidence-ledger.v1";
/// Wire identity of one ledger entry inside [`DecisionEvidenceLedger`].
pub(super) const ENTRY_SCHEMA_V1: &str = "pale/decision-evidence.v1";
/// Maximum UTF-8 bytes retained for one decision reason.
pub(super) const MAX_REASON_BYTES: usize = 1_024;
/// Maximum UTF-8 bytes retained for one native tool identity.
pub(super) const MAX_TOOL_BYTES: usize = 256;
