//! Test-only mutation of the private operation session basis.

use super::*;

/// Replace only the retained Consult intent in a test-owned private basis.
pub(in crate::resident) fn with_test_consultation_start_intent(
    stored: &OperationSessionBasis,
    intent: ConsultationStartIntent,
) -> OperationSessionBasis {
    let mut body: Body = serde_json::from_value(
        canon::parse_strict(stored.canonical_basis_jcs()).expect("test basis remains strict JSON"),
    )
    .expect("test basis retains the private body shape");
    body.consultation_start_intent = Some(intent);
    OperationSessionBasis::new(
        stored.handle().clone(),
        stored.request().clone(),
        stored.session_lease_digest().clone(),
        canon::to_canonical_string(&body)
            .expect("test basis remains canonical")
            .into_bytes(),
        MAX_OPERATION_SESSION_BASIS_BYTES,
    )
    .expect("test basis remains bounded")
}
