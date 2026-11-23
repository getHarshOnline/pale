//! Behavioral tests for resident local-IPC values.

use super::*;

#[test]
fn terminal_cancellation_disposition_uses_one_unambiguous_spelling() {
    let encoded = pale_core::canon::to_canonical_string(
        &ResidentOperationFrameCancellationDisposition::TerminalAlreadyEstablished,
    )
    .expect("the closed cancellation disposition serializes");

    assert_eq!(encoded, "\"terminal_already_established\"");
}
