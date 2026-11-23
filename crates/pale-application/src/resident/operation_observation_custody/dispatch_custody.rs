//! Compare an exact completed append before a separate native dispatch decision.

use pale_core::records::{OperationActivePhaseV1, OperationObservationStateV1};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorError, AntiRollbackAnchorPort,
    ControlDomainAntiRollbackAnchor, ControlStoreError, OperationCustodyRecordKind,
};

use super::*;
use crate::resident::{CompletedOperationObservationCustody, OperationCustodyDatabaseEvidence};

/// A completed append is not the exact newly committed dispatch still selected.
#[derive(Debug, thiserror::Error)]
pub enum OperationDispatchCustodyError {
    /// The completed transaction belongs to a different prepared append.
    #[error("completed custody does not match the prepared observation")]
    Association,
    /// Existing or uncertain database commitment cannot release another call.
    #[error("dispatch requires a new database commit, not replay or recovered uncertainty")]
    NotNewCommit,
    /// Selected observation or current checkpoint no longer matches the append.
    #[error("the completed dispatch is no longer the exact selected database head")]
    HeadChanged,
    /// Independent retained state is absent, different or ahead of this checkpoint.
    #[error("the independent anchor is not the exact completed checkpoint")]
    AnchorChanged,
    /// The bounded observation lookup failed or returned inconsistent evidence.
    #[error(transparent)]
    Recovery(#[from] OperationRecoveryError),
    /// The current checkpoint could not be read from its owning store.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// The independent anchor owner could not answer the exact read.
    #[error(transparent)]
    Anchor(#[from] AntiRollbackAnchorError),
}

impl PreparedOperationObservationAppend<'_> {
    /// Compare the complete append with historical database-and-anchor evidence.
    /// Replay is valid history here; this comparison never permits execution.
    pub(in crate::resident) fn verify_completed_custody(
        &self,
        completed: &CompletedOperationObservationCustody,
    ) -> Result<(), OperationDispatchCustodyError> {
        let candidate = completed.candidate();
        // The closed candidate owner admits exactly one primary latest-index
        // mutation and derives its checkpoint index. Compare that primary
        // mutation, including predecessor bytes, rather than just its key.
        let records_match = candidate.records().len() == 1 + self.associated_records.len()
            && candidate.record(OperationCustodyRecordKind::OperationObservation)
                == Some(&self.successor_record)
            && self
                .associated_records
                .iter()
                .all(|record| candidate.record(record.kind()) == Some(record.record()));
        if candidate.kind() != OperationCustodyTransitionKind::Observe
            || candidate.store() != &self.store
            || !records_match
            || !candidate
                .indexes()
                .contains(&self.latest_observation_mutation)
        {
            return Err(OperationDispatchCustodyError::Association);
        }
        Ok(())
    }

    /// Check a newly committed dispatch against selected storage and its anchor.
    ///
    /// Uses at most four bounded database reads and one anchor read; no writes
    /// occur. A newer checkpoint is refused, not treated as inferred ancestry.
    /// These sequential reads are not an atomic snapshot or a promise about
    /// state after return. The caller retains exclusive owner composition and
    /// separately checks current session, lease, context and disclosure authority.
    pub(in crate::resident) fn verify_fresh_dispatch_custody(
        &self,
        completed: &CompletedOperationObservationCustody,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        maximum_record_bytes: usize,
    ) -> Result<(), OperationDispatchCustodyError> {
        self.verify_completed_custody(completed)?;
        if !matches!(
            self.transition.successor.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched { .. }
            }
        ) {
            return Err(OperationDispatchCustodyError::Association);
        }
        if completed.database_evidence() != OperationCustodyDatabaseEvidence::New {
            return Err(OperationDispatchCustodyError::NotNewCommit);
        }
        let candidate = completed.candidate();
        let selected = read_operation_observation_for_handle(
            custody,
            &self.store,
            self.transition.handle_ref,
            self.transition.handle,
            maximum_record_bytes,
        )?
        .ok_or(OperationDispatchCustodyError::HeadChanged)?;
        let checkpoint = candidate.checkpoint();
        if selected.observation() != &self.successor_record
            || selected.checkpoint() != checkpoint.successor_record()
        {
            return Err(OperationDispatchCustodyError::HeadChanged);
        }
        let current = custody.read_checkpoint_head(
            &self.store,
            checkpoint.successor().checkpoint_domain_id(),
            maximum_record_bytes,
        )?;
        if !current.as_ref().is_some_and(|(head, record)| {
            head == checkpoint.successor()
                && record == checkpoint.successor_record()
                && record.canonical_record_jcs().as_slice().len() <= maximum_record_bytes
        }) {
            return Err(OperationDispatchCustodyError::HeadChanged);
        }
        let head = checkpoint.successor();
        let expected = AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
            self.store.store_id().clone(),
            self.store.incarnation_id().clone(),
            head.checkpoint_domain_id().clone(),
            head.domain_epoch(),
            head.domain_sequence(),
            head.coordinate().clone(),
        ));
        if anchor.read(&expected.key())?.as_ref() != Some(&expected) {
            return Err(OperationDispatchCustodyError::AnchorChanged);
        }
        Ok(())
    }
}
