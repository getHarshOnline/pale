//! Restore exact expiry custody after losing the process's pending phase.

use super::*;
use crate::resident::CommittedObservationCheckpointRecovery;

/// Reconstruct a stored pre-dispatch expiry without signing or committing again.
///
/// The caller must independently admit the handle, both observations, diagnostic
/// and both checkpoints from bounded retained records. This owner checks the
/// expiry state and its diagnostic association, reconstructs the original append,
/// and returns only the independently pending anchor. It neither restores an
/// executable continuation nor establishes evidence that a host enforced refusal.
///
/// # Errors
/// Rejects a dispatched predecessor, wrong diagnostic, invalid expiry chronology,
/// mismatched checkpoint changes, incomplete physical custody or exceeded bounds.
pub fn recover_committed_pre_dispatch_expiry<'a>(
    transition: OperationObservationTransition<'a>,
    diagnostic: &AdmittedDiagnostic,
    checkpoints: CommittedObservationCheckpointRecovery<'a>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    custody: &mut dyn OperationCustodyPort,
) -> Result<
    DatabaseCommittedOperationObservationAnchorPending,
    OperationObservationCustodyProjectionError,
> {
    let append = prepare_operation_observation_append(transition, resolve_store)?;
    let handle = append.handle().1;
    let predecessor = append.predecessor().1;
    let successor = append.successor().1;
    let OperationObservationStateV1::Expired {
        diagnostic: reference,
    } = successor.state()
    else {
        return Err(OperationObservationCustodyProjectionError::Lineage);
    };
    if !matches!(
        handle.operation_class(),
        OperationClassV1::Invoke | OperationClassV1::Delegate
    ) || !matches!(
        predecessor.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving
                | OperationActivePhaseV1::ContextBound { .. }
                | OperationActivePhaseV1::ProviderResolved { .. }
                | OperationActivePhaseV1::DispatchPrepared { .. }
        }
    ) || successor.observed_at() < handle.expires_at()
        || successor.observed_at() > handle.retain_until()
        || successor.observed_at() < checkpoints.predecessor.1.issued_at()
    {
        return Err(OperationObservationCustodyProjectionError::Lineage);
    }
    let (digest, record) =
        crate::resident::record_projection::diagnostic_record(&reference.record_ref, diagnostic)?;
    if digest != reference.record_sha256 {
        return Err(OperationObservationCustodyProjectionError::Lineage);
    }
    append
        .with_associated_record(OperationCustodyRecord::Diagnostic(record))
        .recover_committed_checkpoint(checkpoints, custody)
}
