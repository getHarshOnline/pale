//! Diagnostic contract classification at the embedded-registry boundary.

use super::*;

#[test]
fn diagnostic_contract_failure_distinguishes_input_from_registry_invariants() {
    let invalid = diagnostic_contract_failure(
        &pale_contracts::SchemaError::IdentityMismatch("diagnostic.v1".to_string()),
        "invalid_fixture",
        "invalid fixture",
    );
    assert_eq!(invalid.code, "invalid_fixture");
    assert_eq!(invalid.stage, DiagnosticStageV1::ContractValidation);
    assert_eq!(invalid.disposition, DiagnosticDispositionV1::InvalidInput);

    let internal = diagnostic_contract_failure(
        &pale_contracts::SchemaError::UnknownId("diagnostic.v1".to_string()),
        "invalid_fixture",
        "invalid fixture",
    );
    assert_eq!(internal.code, "diagnostic_contract_invariant");
    assert_eq!(internal.stage, DiagnosticStageV1::Internal);
    assert_eq!(
        internal.disposition,
        DiagnosticDispositionV1::InternalFailure
    );
}
