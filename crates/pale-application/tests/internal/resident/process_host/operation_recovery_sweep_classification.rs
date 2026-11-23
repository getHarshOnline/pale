//! Test the explicit owner classification for cold recovery.

use super::*;

#[test]
fn unsupported_operation_owners_are_explicit() {
    assert_eq!(
        class_deferred_owner(OperationClassV1::Consult, false),
        Some(ResidentProcessOperationRecoveryDeferred::Consult)
    );
    assert_eq!(
        class_deferred_owner(OperationClassV1::Delegate, false),
        Some(ResidentProcessOperationRecoveryDeferred::Delegate)
    );
    assert_eq!(
        class_deferred_owner(OperationClassV1::Invoke, true),
        Some(ResidentProcessOperationRecoveryDeferred::Delegate)
    );
    assert_eq!(
        a2a_deferred_owner(),
        ResidentProcessOperationRecoveryDeferred::A2a
    );
    assert_eq!(class_deferred_owner(OperationClassV1::Invoke, false), None);
    assert_eq!(
        deferred_owner(RestartSemanticOwner::A2a, OperationClassV1::Delegate, true),
        Some(ResidentProcessOperationRecoveryDeferred::A2a)
    );
    assert_eq!(
        deferred_owner(
            RestartSemanticOwner::Ordinary,
            OperationClassV1::Delegate,
            true
        ),
        Some(ResidentProcessOperationRecoveryDeferred::Delegate)
    );
}
