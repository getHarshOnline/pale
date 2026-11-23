//! Explicit effects and invariant failures for session transitions.

use pale_core::records::{ModeAxis, PackageResourceRefV1};

use super::verification::VerificationRequirement;

/// The explicit outcome of applying one state action.
#[must_use = "state transitions return the exact effect an adapter must persist or report"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateEffect {
    /// A run mode was activated on one independent axis.
    Activated {
        /// Independent mode dimension updated.
        axis: ModeAxis,
        /// Newly active mode identity.
        mode: PackageResourceRefV1,
    },
    /// Explicit overrides were cleared and the declared defaults restored.
    DefaultsRestored,
    /// A first-touch key was recorded for the first time.
    FirstTouch {
        /// Bounded normalized or digest-substituted path key.
        key: String,
    },
    /// A bounded audit entry was captured this session.
    Captured {
        /// Bounded entry retained in session recency order.
        entry: String,
    },
    /// A write created a verification epoch.
    VerificationPending {
        /// Newly allocated write epoch.
        epoch: u64,
        /// Strength required to discharge this epoch.
        requirement: VerificationRequirement,
    },
    /// A successful exact verifier discharged the observed epoch.
    VerificationCleared {
        /// Exact observed epoch that was discharged.
        epoch: u64,
    },
    /// Completion guidance was emitted once for the observed epoch.
    VerificationPrompted {
        /// Exact observed epoch for which guidance was emitted.
        epoch: u64,
    },
    /// Compaction armed a context refresh for the next eligible prompt.
    RebaselinePending {
        /// Newly allocated compaction epoch.
        epoch: u64,
    },
    /// The prompt bound to this epoch consumed the pending refresh.
    RebaselineCleared {
        /// Exact compaction epoch consumed by this prompt.
        epoch: u64,
    },
}

/// A persisted-state invariant or monotonic transition could not be satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SessionStateError {
    /// Persisted state did not carry the exact generated schema identity.
    #[error("session-state schema identity is not admitted")]
    SchemaIdentityInvalid,
    /// Active-mode state contains an invalid identity, duplicate axis, or excess entry.
    #[error("active-mode state is invalid or exceeds its bound")]
    ActiveModesBound,
    /// First-touch state exceeds its count or text ceiling.
    #[error("first-touch state exceeds its bound")]
    FirstTouchBound,
    /// Capture state is duplicated or exceeds its count or text ceiling.
    #[error("capture state is invalid or exceeds its bound")]
    CapturesBound,
    /// Operation lease identity or authorization is invalid or oversized.
    #[error("operation lease is invalid or exceeds its bound")]
    OperationLeaseBound,
    /// A sticky operation failure cannot be replaced within the same lease.
    #[error("operation lease already carries a sticky failure")]
    OperationFailureSticky,
    /// No newer operation activation epoch can be represented.
    #[error("operation activation epoch overflow")]
    OperationEpochOverflow,
    /// A replacement lease does not preserve or advance the current activation epoch.
    #[error("operation lease replacement has an invalid activation epoch")]
    OperationEpochInvalid,
    /// No newer verification epoch can be represented.
    #[error("verification epoch overflow")]
    VerificationEpochOverflow,
    /// Persisted verification debt does not match the latest allocated epoch.
    #[error("verification debt is inconsistent with its epoch")]
    VerificationStateInvalid,
    /// No newer post-compaction refresh epoch can be represented.
    #[error("rebaseline epoch overflow")]
    RebaselineEpochOverflow,
    /// Persisted rebaseline debt does not match the latest allocated epoch.
    #[error("rebaseline debt is inconsistent with its epoch")]
    RebaselineStateInvalid,
}
