//! Test-only semantic admission around the production collection reader.

use crate::resident::*;
use pale_core::records::OperationObservationV1;
use pale_ports::OperationCustodyPort;

/// Read a collection with fixture policy after the caller admits its observation.
///
/// The production reader still checks stored bytes, bounds and exact association.
/// This callback supplies fixture policy, not production authorisation.
///
/// # Errors
/// Returns the production reader's refusal unchanged.
pub fn recover_collection_fixture(
    operation: &IssuedLocalOperation,
    observation: &OperationObservationV1,
    limits: ConsultationCollectionLimits,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
) -> Result<Option<RecoveredConsultationCollection>, ConsultationCollectionError> {
    ConsultationCollection::recover_current(
        operation,
        observation,
        limits,
        resolve_store,
        custody,
        |_| Ok(()),
    )
}
