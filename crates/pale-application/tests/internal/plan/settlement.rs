//! Laws for receipt-coupled materialization settlement.

use super::*;

#[test]
fn every_receipt_observation_has_one_non_substitutable_settlement() {
    assert_eq!(
        decide_materialization_settlement(ReceiptPublicationOutcome::Published),
        MaterializationSettlement::Commit
    );
    for outcome in [
        ReceiptPublicationOutcome::Absent,
        ReceiptPublicationOutcome::Collision,
    ] {
        assert_eq!(
            decide_materialization_settlement(outcome),
            MaterializationSettlement::RollbackNoEffect
        );
    }
    assert_eq!(
        decide_materialization_settlement(ReceiptPublicationOutcome::Uncertain),
        MaterializationSettlement::RetainForReconciliation
    );
}
