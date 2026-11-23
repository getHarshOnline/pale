//! Receipt admission and execution-workspace result aggregation.

mod receipt;
mod run;
mod settlement;

pub use receipt::{
    build_success_receipt_with_lineage, AdmittedSuccessReceipt, MaterializationLineageEvidence,
};
pub use run::{build_run_result, AdmittedRunResult, RunTargetEvidence};
pub use settlement::{
    decide_materialization_settlement, MaterializationSettlement, ReceiptPublicationOutcome,
};
