//! Pure settlement decision for receipt-coupled materialization.
//!
//! Native adapters observe publication and execute commit or rollback. This
//! module owns the proof law connecting that observation to the only lawful
//! settlement action, so CLI and later native leaves cannot reinterpret receipt
//! uncertainty independently.

/// Observation established after attempting immutable receipt publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptPublicationOutcome {
    /// The exact receipt is durably available.
    Published,
    /// The exact receipt is proven absent.
    Absent,
    /// The immutable slot contains a different receipt or required mode.
    Collision,
    /// Receipt visibility or durability could not be established.
    Uncertain,
}

/// Only lawful target disposition for one receipt-publication outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterializationSettlement {
    /// Commit prepared target effects because exact success evidence exists.
    Commit,
    /// Restore every prepared effect before claiming a failed-no-effect result.
    RollbackNoEffect,
    /// Retain the post-state because the receipt may already be visible.
    RetainForReconciliation,
}

/// Select settlement from the observed immutable-receipt state.
///
/// The transition is total and deterministic:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/materialization-settlement.display.mathml"
))]
///
/// Retention is not success. It preserves the only state that can remain
/// consistent with a receipt that might already be externally observable.
#[must_use]
pub const fn decide_materialization_settlement(
    outcome: ReceiptPublicationOutcome,
) -> MaterializationSettlement {
    match outcome {
        ReceiptPublicationOutcome::Published => MaterializationSettlement::Commit,
        ReceiptPublicationOutcome::Absent | ReceiptPublicationOutcome::Collision => {
            MaterializationSettlement::RollbackNoEffect
        }
        ReceiptPublicationOutcome::Uncertain => MaterializationSettlement::RetainForReconciliation,
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/plan/settlement.rs"]
mod tests;
