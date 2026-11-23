//! Borrowed coordinates that bind a decision ledger to one native activity path.

use std::fmt;

use pale_core::engine_wire::DecisionActivityV1;
use pale_core::records::{
    CompilationInputDigest, ExecutionPathSelectionV1, LogicalRepositoryIdV1, ProfileDigest,
};

/// Exact path-bound identity of one [`DecisionEvidenceLedger`](super::DecisionEvidenceLedger).
///
/// Every field participates in ledger admission; changing any coordinate creates
/// a different evidence stream rather than a continuation of the old one.
#[derive(Clone, Copy)]
pub struct DecisionLedgerIdentity<'a> {
    /// Exact compilation inputs that produced the admitted hook context.
    pub compilation_input_digest: &'a CompilationInputDigest,
    /// Portable profile identity binding the canonical project and execution workspace.
    pub profile_digest: &'a ProfileDigest,
    /// Canonical repository selected inside the digested profile.
    pub repository_id: &'a LogicalRepositoryIdV1,
    /// Exact neutral path selected for the native activity.
    pub execution_path: &'a ExecutionPathSelectionV1,
    /// Strongest exposed native activity, or an explicit visibility limitation.
    pub activity: &'a DecisionActivityV1,
}

impl fmt::Debug for DecisionLedgerIdentity<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecisionLedgerIdentity")
            .field("compilation_input_digest", self.compilation_input_digest)
            .field("profile_digest", self.profile_digest)
            .field("repository_id", &self.repository_id)
            .field("execution_path", self.execution_path)
            .field("activity", &self.activity.observed())
            .finish()
    }
}
