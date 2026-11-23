//! Faults alter confirmations only; all operation state comes from real `SQLite`.

use super::*;

pub(super) struct Custody<'a, 'connection> {
    pub(super) inner: &'a mut SqliteOperationCustody<'connection>,
    pub(super) case: Case,
    pub(super) commits: usize,
    pub(super) last: Option<OperationCustodyCandidate>,
}

impl ControlTransactionPort for Custody<'_, '_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        self.inner.prepare_transaction(store)
    }
}

impl OperationCustodyPort for Custody<'_, '_> {
    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &pale_ports::OperationA2aBindingScope,
        cursor: Option<&pale_ports::OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationA2aBindings, ControlStoreError> {
        self.inner.read_a2a_task_bindings(
            store,
            scope,
            cursor,
            maximum_bindings,
            maximum_body_bytes,
        )
    }

    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationPublicationSources, OperationPublicationReadError> {
        self.inner
            .read_publication_sources(store, cursor, maximum_sources, maximum_body_bytes)
    }
    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &ControlCheckpointDomainIdV1,
        maximum_record_bytes: usize,
    ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        self.inner
            .read_checkpoint_head(store, domain, maximum_record_bytes)
    }
    fn verify_accumulator_membership(
        &mut self,
        request: &OperationAccumulatorMembership,
    ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
        self.inner.verify_accumulator_membership(request)
    }
    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        self.inner.prepare_accumulator(request)
    }
    fn compare_and_retain_operation_lease(
        &mut self,
        store: &AuthorityStoreCoordinate,
        record: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.inner.compare_and_retain_operation_lease(store, record)
    }
    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        self.inner
            .read_operation_record(store, coordinate, maximum_body_bytes)
    }
    fn compare_and_commit(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.commits += 1;
        self.last = Some(candidate.clone());
        let actual = self.inner.compare_and_commit(candidate)?;
        if (self.commits == 2
            && matches!(
                self.case,
                Case::DispatchLostConfirmation | Case::DispatchUnknownReadBack
            ))
            || (self.commits == 3 && matches!(self.case, Case::OutcomeUnknownReadBack))
        {
            assert_eq!(actual, OperationCustodyCommitOutcome::Committed);
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::OutcomeUnknown,
            ));
        }
        Ok(actual)
    }
    fn observe_exact(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        if (self.commits == 2 && matches!(self.case, Case::DispatchUnknownReadBack))
            || (self.commits == 3 && matches!(self.case, Case::OutcomeUnknownReadBack))
        {
            return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
        }
        self.inner.observe_exact(candidate)
    }
    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        self.inner.read_index(store, query, maximum_record_bytes)
    }
}

pub(super) struct Anchor<'a> {
    pub(super) inner: &'a mut IndependentAnchor,
    pub(super) case: Case,
    pub(super) advances: usize,
}

impl AntiRollbackAnchorPort for Anchor<'_> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.inner.read(key)
    }
    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.inner.observe(transition)
    }
    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.advances += 1;
        if matches!(
            (self.advances, self.case),
            (1, Case::PreparationAnchorFailure)
                | (2, Case::DispatchAnchorFailure)
                | (3, Case::OutcomeAnchorFailure)
                | (4, Case::TerminalAnchorFailure)
        ) {
            return Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ));
        }
        self.inner.compare_and_advance(transition)
    }
}
