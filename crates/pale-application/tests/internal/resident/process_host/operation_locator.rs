//! Exact operation-frame locator parsing laws.

use crate::resident::process_host::operation_locator::*;
use pale_ports::ResidentOperationFrameLocator;

#[test]
fn locator_enters_the_existing_typed_custody_coordinate() {
    let digest = "ab".repeat(32);
    let locator =
        ResidentOperationFrameLocator::new("operation-handle:fixture".to_owned(), digest.clone());
    let admitted = ResidentProgramOperationLocator::parse(&locator).expect("exact locator");
    assert_eq!(
        admitted.coordinate().record_ref().as_str(),
        "operation-handle:fixture"
    );
    assert_eq!(admitted.coordinate().record_sha256().as_str(), digest);
}

#[test]
fn malformed_reference_or_digest_never_enters_recovery() {
    let digest = "ab".repeat(32);
    let bad_reference = ResidentOperationFrameLocator::new("".to_owned(), digest);
    assert!(matches!(
        ResidentProgramOperationLocator::parse(&bad_reference),
        Err(ResidentProgramOperationLocatorError::RecordReference(_))
    ));

    let bad_digest = ResidentOperationFrameLocator::new(
        "operation-handle:fixture".to_owned(),
        "not-a-digest".to_owned(),
    );
    assert!(matches!(
        ResidentProgramOperationLocator::parse(&bad_digest),
        Err(ResidentProgramOperationLocatorError::HandleDigest(_))
    ));
}
