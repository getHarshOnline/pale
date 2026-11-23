//! Resume collection opening after every planned child reached protected custody.

use super::*;

pub(super) enum RecoveredStart {
    Existing(Box<ResidentProcessOperationRestartDisposition>),
    OpenCollection(Box<consult_start::ResidentProcessConsultationCollectionPending>),
}

/// Exact Start replay may recover the saved child prefix before a collection exists.
/// An absent child may have been signed before custody was interrupted. Its suffix
/// remains unavailable until signer-owned evidence resolves issuance; this path
/// never replaces it by running the roster policy again.
pub(super) fn resume<WriterGuard>(
    process: &ResidentProcessHost<'_, WriterGuard>,
    selector: &ResidentDeploymentAuthoritySelector,
    access: ResidentDeploymentOperationAccess<'_>,
    context: crate::resident::LocalOperationRecoveryContext<'_>,
    maximum_parent_depth: usize,
    admission: OperationControlRecoveryAdmission<'_>,
    disposition: Box<ResidentProcessOperationRestartDisposition>,
) -> Result<RecoveredStart, ResidentOperationFrameFailure> {
    let ResidentProcessOperationRestartDisposition::OperationResolution(resolution) = *disposition
    else {
        return Ok(RecoveredStart::Existing(disposition));
    };
    if resolution.phase != restart::ResidentProcessOperationResolutionPhase::Resolving
        || resolution.start.selector != *selector
    {
        return Err(ResidentOperationFrameFailure::InvalidRequest);
    }
    let (parent, _) = resolution.start.control.into_parts();
    let started = process
        .rebuild_consultation_start_from_protected_intent(
            selector,
            access.store,
            parent.into_operation(),
            context,
            maximum_parent_depth,
            access.custody,
            access.sessions,
            access.anchor,
            admission,
        )
        .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)?;
    match started.into_initialization() {
        consult_start::ResidentProcessConsultationInitialization::OpenCollection(pending) => {
            Ok(RecoveredStart::OpenCollection(Box::new(pending)))
        }
        consult_start::ResidentProcessConsultationInitialization::StartChild(_) => {
            Err(ResidentOperationFrameFailure::ResidentUnavailable)
        }
    }
}
