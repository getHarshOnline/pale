//! Checked principal bootstrap fixture tests.

use super::principal::*;

const INPUTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/resident/invoke-conformance.v1/principal/bootstrap-authority-inputs.json"
));

#[test]
fn checked_bootstrap_inputs_pass_every_admission_stage() {
    let inputs = fixture_inputs_from_str(INPUTS).expect("fixture JSON");
    admit_bootstrap(inputs).expect("checked bootstrap inputs admit");
}

#[test]
fn changed_principal_digest_is_rejected() {
    let tampered = INPUTS.replacen(
        "495a9a7888beb01e6a893bd974b6606147dc2a00b0bf52ad1c9662c6d5b977d0",
        "0000000000000000000000000000000000000000000000000000000000000000",
        1,
    );
    let inputs = fixture_inputs_from_str(&tampered).expect("tampered fixture remains JSON");
    assert!(admit_bootstrap(inputs).is_err());
}
