//! Typed refusals for decision-ledger admission and monotonic extension.

use pale_core::canon;

/// Structural refusal while loading or extending decision evidence.
#[derive(Debug, thiserror::Error)]
pub enum DecisionEvidenceError {
    /// Persisted evidence does not satisfy its canonical JSON schema.
    #[error("decision-evidence schema validation failed: {source}")]
    Schema {
        /// Canonical schema failure without the untrusted evidence payload.
        #[source]
        source: pale_contracts::SchemaError,
    },
    /// An existing ledger was opened through a different path-bound identity.
    #[error("decision-evidence identity does not match its path binding")]
    IdentityMismatch,
    /// Persisted evidence violates its bounds or sequence order.
    #[error("decision-evidence record is malformed or outside its bounds")]
    Malformed,
    /// A retained entry or predecessor link differs from its computed digest.
    #[error("decision-evidence digest chain is invalid")]
    BrokenChain,
    /// Claimed effect is not possible for the selected terminal harness and event.
    #[error("decision-evidence disposition does not match its host event")]
    DispositionMismatch,
    /// A stage names no exact predecessor or changes its decision coordinates.
    #[error("decision-evidence stage does not match its exact predecessor")]
    StageMismatch,
    /// The bounded view already retains this exact stage transition.
    #[error("decision-evidence stage is already retained")]
    StageReplay,
    /// The activity-local sequence counter cannot be advanced without wrapping.
    #[error("decision-evidence sequence overflow")]
    SequenceOverflow,
    /// Canonicalization of a decision-evidence digest preimage failed.
    #[error("decision-evidence digest failed: {source}")]
    Digest {
        /// Canonicalization failure without the untrusted evidence payload.
        #[source]
        source: canon::CanonError,
    },
}
