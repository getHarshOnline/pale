//! In-memory custody and writer evidence for interface conformance.

use super::evidence::principal;

pub(super) mod anchor {
    //! Callback-scoped anti-rollback state for the conformance frontier.
    //!
    //! The map applies the real prepared-transition classifier. It is memory only;
    //! it does not provide an independent failure domain or survive process loss.

    use std::collections::BTreeMap;

    use pale_ports::*;

    #[derive(Default)]
    pub(in super::super) struct ConformanceAntiRollbackAnchors {
        values: BTreeMap<AntiRollbackAnchorKey, AntiRollbackAnchor>,
        pub(in super::super) advances: usize,
    }

    impl AntiRollbackAnchorPort for ConformanceAntiRollbackAnchors {
        fn read(
            &mut self,
            key: &AntiRollbackAnchorKey,
        ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
            Ok(self.values.get(key).cloned())
        }

        fn compare_and_advance(
            &mut self,
            transition: &PreparedAntiRollbackAnchorTransition,
        ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
            match transition.classify_observed(self.values.get(transition.key())) {
                AntiRollbackAnchorObservation::Missing if transition.expected().is_none() => {
                    self.values
                        .insert(transition.key().clone(), transition.next().clone());
                    self.advances += 1;
                    Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
                }
                AntiRollbackAnchorObservation::ExactPredecessor => {
                    self.values
                        .insert(transition.key().clone(), transition.next().clone());
                    self.advances += 1;
                    Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
                }
                AntiRollbackAnchorObservation::ExactSuccessor => {
                    Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay)
                }
                other => Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(other)),
            }
        }

        fn observe(
            &mut self,
            transition: &PreparedAntiRollbackAnchorTransition,
        ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
            Ok(transition.classify_observed(self.values.get(transition.key())))
        }
    }
}

pub(super) mod custody {
    //! In-memory operation custody for the interface conformance frontier.
    //!
    //! This owner exercises the application-defined custody candidates, indexes,
    //! checkpoints, and accumulator requests. Its roots are deterministic test
    //! models. It has no crash boundary and does not qualify durable media.

    use pale_core::records::*;
    use pale_ports::*;

    /// Exact operation custody retained during one callback-scoped journey.
    pub(in super::super) struct ConformanceOperationCustody {
        store: AuthorityStoreCoordinate,
        history: Vec<OperationCustodyCandidate>,
        retained_leases: Vec<ControlRecord>,
    }

    impl ConformanceOperationCustody {
        pub(in super::super) fn new(store: AuthorityStoreCoordinate) -> Self {
            Self {
                store,
                history: Vec::new(),
                retained_leases: Vec::new(),
            }
        }

        fn retained_record(&self, coordinate: &ControlRecordCoordinate) -> Option<ControlRecord> {
            self.history
                .iter()
                .rev()
                .flat_map(candidate_records)
                .chain(self.retained_leases.iter())
                .find(|record| record.coordinate() == coordinate)
                .cloned()
        }

        fn indexed_record(&self, query: &OperationCustodyIndexQuery) -> Option<ControlRecord> {
            self.history.iter().rev().find_map(|candidate| {
                let coordinate = candidate
                    .indexes()
                    .iter()
                    .find_map(|mutation| matching_index_successor(mutation, query))?;
                candidate_records(candidate)
                    .find(|record| record.coordinate() == coordinate)
                    .cloned()
            })
        }
    }

    fn candidate_records(
        candidate: &OperationCustodyCandidate,
    ) -> impl Iterator<Item = &ControlRecord> {
        candidate
            .records()
            .iter()
            .map(OperationCustodyRecord::record)
            .chain(candidate.a2a_retained_records())
            .chain(candidate.a2a_associated_route())
            .chain(std::iter::once(candidate.checkpoint().successor_record()))
            .chain(candidate.checkpoint().predecessor_record())
    }

    fn matching_index_successor<'a>(
        mutation: &'a OperationCustodyIndexMutation,
        query: &OperationCustodyIndexQuery,
    ) -> Option<&'a ControlRecordCoordinate> {
        use OperationCustodyIndexMutation as Mutation;
        use OperationCustodyIndexQuery as Query;

        match (mutation, query) {
            (Mutation::NativeOutcome { key, successor, .. }, Query::NativeOutcome(expected))
            | (
                Mutation::A2aSendPreparation { key, successor, .. },
                Query::A2aSendPreparation(expected),
            )
            | (
                Mutation::A2aSendPreparationCheckpoint { key, successor },
                Query::A2aSendPreparationCheckpoint(expected),
            )
            | (Mutation::A2aTaskBinding { key, successor, .. }, Query::A2aTaskBinding(expected))
            | (
                Mutation::FirstA2aTaskObservation { key, successor },
                Query::FirstA2aTaskObservation(expected),
            )
            | (
                Mutation::LatestA2aTaskObservation { key, successor, .. },
                Query::LatestA2aTaskObservation(expected),
            )
            | (
                Mutation::A2aTaskObservationCheckpoint { key, successor },
                Query::A2aTaskObservationCheckpoint(expected),
            )
            | (Mutation::RequestForHandle { key, successor }, Query::RequestForHandle(expected))
            | (Mutation::StartObservation { key, successor }, Query::StartObservation(expected))
            | (
                Mutation::ObservationCheckpoint { key, successor },
                Query::ObservationCheckpoint(expected),
            )
            | (
                Mutation::RequestEvidence { key, successor, .. },
                Query::RequestEvidence(expected),
            )
            | (
                Mutation::RequestOperation { key, successor, .. },
                Query::RequestOperation(expected),
            )
            | (
                Mutation::OperationHandle { key, successor, .. },
                Query::OperationHandle(expected),
            )
            | (
                Mutation::ParentChildMembership { key, successor, .. },
                Query::ParentChildMembership(expected),
            )
            | (Mutation::CollectionHead { key, successor, .. }, Query::CollectionHead(expected))
            | (
                Mutation::LatestObservation { key, successor, .. },
                Query::LatestObservation(expected),
            ) if key == expected => Some(successor),
            _ => None,
        }
    }

    fn collection_snapshot_error(_: OperationCollectionJournalError) -> ControlStoreError {
        ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
    }

    impl ControlTransactionPort for ConformanceOperationCustody {
        fn prepare_transaction(
            &mut self,
            store: &AuthorityStoreCoordinate,
        ) -> Result<ControlTransactionPreparation, ControlStoreError> {
            if store != &self.store {
                return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
            }
            let expected = self.history.last().map_or_else(
                || ControlTransactionSequenceV1::new(1).expect("initial transaction"),
                |candidate| {
                    candidate
                        .checkpoint()
                        .successor()
                        .committed_transaction_sequence()
                },
            );
            ControlTransactionPreparation::new(store.clone(), expected)
                .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
        }
    }

    impl OperationCustodyPort for ConformanceOperationCustody {
        fn read_collection_snapshot(
            &mut self,
            store: &AuthorityStoreCoordinate,
            handle: &ControlRecordCoordinate,
            maximum_frontier_bytes: usize,
            maximum_journal_payload_bytes: usize,
        ) -> Result<Option<OperationCollectionSnapshot>, ControlStoreError> {
            if store != &self.store {
                return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
            }
            if maximum_frontier_bytes == 0 || maximum_journal_payload_bytes == 0 {
                return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
            }
            let query = OperationCustodyIndexQuery::CollectionHead(
                OperationCustodyIndexKey::for_collection_head(handle),
            );
            let Some((candidate, coordinate)) = self.history.iter().rev().find_map(|candidate| {
                candidate.indexes().iter().find_map(|mutation| {
                    matching_index_successor(mutation, &query)
                        .map(|coordinate| (candidate, coordinate.clone()))
                })
            }) else {
                return Ok(None);
            };
            let journal = candidate
                .collection_journal()
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            if journal.frontier().coordinate() != &coordinate {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            let frontier = candidate_records(candidate)
                .find(|record| record.coordinate() == &coordinate)
                .cloned()
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            if &frontier != journal.frontier() {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            if frontier.canonical_record_jcs().as_slice().len() > maximum_frontier_bytes
                || journal.append().entry().bytes().as_slice().len() > maximum_journal_payload_bytes
            {
                return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
            }
            let evidence = OperationCollectionFrontierEvidence::admit(
                &frontier,
                store,
                handle,
                maximum_frontier_bytes,
            )
            .map_err(collection_snapshot_error)?;
            let head = OperationCollectionHeadObservation::for_handle(handle, coordinate);
            OperationCollectionSnapshot::new(
                frontier,
                &evidence,
                head,
                journal.append().entry().bytes().clone(),
            )
            .map(Some)
            .map_err(collection_snapshot_error)
        }

        fn read_operation_session_basis(
            &mut self,
            store: &AuthorityStoreCoordinate,
            handle: &ControlRecordCoordinate,
            maximum_bytes: usize,
        ) -> Result<Option<OperationSessionBasis>, ControlStoreError> {
            if store != &self.store {
                return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
            }
            let basis = self
                .history
                .iter()
                .rev()
                .filter_map(OperationCustodyCandidate::session_basis)
                .find(|basis| basis.handle() == handle)
                .cloned();
            if basis
                .as_ref()
                .is_some_and(|value| value.canonical_basis_jcs().len() > maximum_bytes)
            {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            Ok(basis)
        }

        fn read_a2a_task_bindings(
            &mut self,
            _: &AuthorityStoreCoordinate,
            _: &OperationA2aBindingScope,
            _: Option<&OperationA2aBindingCursor>,
            _: usize,
            _: usize,
        ) -> Result<OperationA2aBindings, ControlStoreError> {
            Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ))
        }

        fn read_publication_sources(
            &mut self,
            _: &AuthorityStoreCoordinate,
            _: Option<&OperationPublicationCursor>,
            _: usize,
            _: usize,
        ) -> Result<OperationPublicationSources, OperationPublicationReadError> {
            Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into())
        }

        fn read_checkpoint_head(
            &mut self,
            store: &AuthorityStoreCoordinate,
            domain: &ControlCheckpointDomainIdV1,
            maximum_bytes: usize,
        ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
            if store != &self.store {
                return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
            }
            let result = self.history.iter().rev().find_map(|candidate| {
                let checkpoint = candidate.checkpoint();
                (checkpoint.successor().checkpoint_domain_id() == domain).then(|| {
                    (
                        checkpoint.successor().clone(),
                        checkpoint.successor_record().clone(),
                    )
                })
            });
            if result.as_ref().is_some_and(|(_, record)| {
                record.canonical_record_jcs().as_slice().len() > maximum_bytes
            }) {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            Ok(result)
        }

        fn verify_accumulator_membership(
            &mut self,
            request: &OperationAccumulatorMembership,
        ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError>
        {
            Ok(VerifiedOperationAccumulatorMembership::new(request.clone()))
        }

        fn prepare_accumulator(
            &mut self,
            request: &OperationAccumulatorPreparation,
        ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
            let marker = u8::try_from(request.transaction().successor().get())
                .map_err(|_| OperationAccumulatorPreparationError::Limit)?;
            let (state_predecessor, unresolved_predecessor, prior_value) = match request.basis() {
                AccumulatorPreparationBasis::Initial(_) => (
                    ArtifactDigest::parse("0".repeat(64)).expect("closed initial root"),
                    ArtifactDigest::parse("0".repeat(64)).expect("closed initial root"),
                    None,
                ),
                AccumulatorPreparationBasis::Successor(prior) => (
                    prior.state_root().clone(),
                    prior.unresolved_root().clone(),
                    Some(vec![marker.saturating_sub(1); 32]),
                ),
            };
            let next_root = ArtifactDigest::hash_bytes(&[marker; 32]);
            let state = StateAccumulatorBatch::new(
                state_predecessor,
                next_root,
                vec![AccumulatorMutation::new(
                    AccumulatorPosition::Leaf([marker; 32]),
                    prior_value,
                    Some(vec![marker; 32]),
                    32,
                )
                .map_err(|_| OperationAccumulatorPreparationError::Limit)?],
                1,
            )
            .map_err(|_| OperationAccumulatorPreparationError::Limit)?;
            let unresolved = UnresolvedCustodyAccumulatorBatch::new(
                unresolved_predecessor.clone(),
                unresolved_predecessor,
                Vec::new(),
                1,
            )
            .map_err(|_| OperationAccumulatorPreparationError::Limit)?;
            PreparedOperationAccumulator::new(request.clone(), state, unresolved)
                .map_err(|_| OperationAccumulatorPreparationError::Limit)
        }

        fn compare_and_retain_operation_lease(
            &mut self,
            store: &AuthorityStoreCoordinate,
            record: &ControlRecord,
        ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
            if store != &self.store {
                return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
            }
            if self
                .retained_leases
                .iter()
                .any(|existing| existing == record)
            {
                return Ok(OperationCustodyCommitOutcome::ExactReplay);
            }
            self.retained_leases.push(record.clone());
            Ok(OperationCustodyCommitOutcome::Committed)
        }

        fn read_operation_record(
            &mut self,
            store: &AuthorityStoreCoordinate,
            coordinate: &ControlRecordCoordinate,
            maximum_bytes: usize,
        ) -> Result<Option<ControlRecord>, ControlStoreError> {
            if store != &self.store {
                return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
            }
            let record = self.retained_record(coordinate);
            if record.as_ref().is_some_and(|record| {
                record.canonical_record_jcs().as_slice().len() > maximum_bytes
            }) {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            Ok(record)
        }

        fn compare_and_commit(
            &mut self,
            candidate: &OperationCustodyCandidate,
        ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
            if self.history.last() == Some(candidate) {
                return Ok(OperationCustodyCommitOutcome::ExactReplay);
            }
            self.history.push(candidate.clone());
            Ok(OperationCustodyCommitOutcome::Committed)
        }

        fn observe_exact(
            &mut self,
            candidate: &OperationCustodyCandidate,
        ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
            if self.history.last() == Some(candidate) {
                return Ok(OperationCustodyReadBackOutcome::ExactSuccessor);
            }
            let predecessor_matches = match candidate.checkpoint().predecessor_record() {
                None => self.history.is_empty(),
                Some(predecessor) => self.history.last().is_some_and(|current| {
                    current.checkpoint().successor_record().coordinate() == predecessor.coordinate()
                }),
            };
            Ok(if predecessor_matches {
                OperationCustodyReadBackOutcome::Predecessor
            } else {
                OperationCustodyReadBackOutcome::ConflictingOrPartial
            })
        }

        fn read_index(
            &mut self,
            store: &AuthorityStoreCoordinate,
            query: &OperationCustodyIndexQuery,
            maximum_record_bytes: usize,
        ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
            if store != &self.store {
                return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
            }
            self.indexed_record(query).map_or_else(
                || Ok(OperationCustodyIndexObservation::absent(query.clone())),
                |record| {
                    if record.canonical_record_jcs().as_slice().len() > maximum_record_bytes {
                        return Err(ControlStoreError::new(
                            ControlStoreErrorKind::RecoveryRequired,
                        ));
                    }
                    OperationCustodyIndexObservation::exact(query.clone(), record).map_err(|_| {
                        ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
                    })
                },
            )
        }
    }
}

pub(super) mod storage {
    //! Bounded in-memory custody for the interface conformance journey.

    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use pale_core::records::{JournalSequenceV1, SessionAdmissionLeaseDigest};
    use pale_ports::*;

    /// Minimal durable owner used to cold-open the real resident application.
    ///
    /// The owner preserves exact append, replay, paging, checkpoint, and session
    /// body semantics in memory. It models neither a crash boundary nor durable
    /// media and therefore cannot qualify a production control store.
    #[derive(Default)]
    pub(in super::super) struct ConformanceAuthorityStore {
        store: Mutex<Option<AuthorityStoreCoordinate>>,
        entries: Mutex<Vec<SequencedAuthorityEntry>>,
        checkpoints: Mutex<Vec<StoredRecoveryCheckpoint>>,
        sessions: Mutex<BTreeMap<SessionAdmissionLeaseDigest, SessionLeaseBody>>,
    }

    impl ConformanceAuthorityStore {
        pub(in super::super) fn bind(&self, store: AuthorityStoreCoordinate) {
            *self.store.lock().expect("conformance store binding") = Some(store);
        }

        fn admits(&self, store: &AuthorityStoreCoordinate) -> bool {
            self.store
                .lock()
                .expect("conformance store binding")
                .as_ref()
                == Some(store)
        }
    }

    impl DurableAuthorityStorePort for ConformanceAuthorityStore {
        fn compare_and_append(
            &self,
            append: &PreparedAuthorityAppend,
        ) -> Result<(), DurableAuthorityStoreError> {
            if !self.admits(append.store()) {
                return Err(DurableAuthorityStoreErrorKind::Unavailable.into());
            }
            let mut entries = self.entries.lock().expect("conformance journal");
            if entries.last().map(SequencedAuthorityEntry::sequence) != append.expected_head() {
                return Err(DurableAuthorityStoreErrorKind::Conflict.into());
            }
            entries.push(append.entry().clone());
            Ok(())
        }

        fn compare_and_store_checkpoint(
            &self,
            checkpoint: &PreparedRecoveryCheckpoint,
        ) -> Result<(), DurableAuthorityStoreError> {
            if !self.admits(checkpoint.store()) {
                return Err(DurableAuthorityStoreErrorKind::Unavailable.into());
            }
            let head = self
                .entries
                .lock()
                .expect("conformance journal")
                .last()
                .map(SequencedAuthorityEntry::sequence);
            if head != Some(checkpoint.expected_head()) {
                return Err(DurableAuthorityStoreErrorKind::Conflict.into());
            }
            self.checkpoints
                .lock()
                .expect("conformance checkpoints")
                .insert(0, checkpoint.checkpoint().clone());
            Ok(())
        }

        fn read_recovery_checkpoints(
            &self,
            _: &RecoveryCheckpointRead,
        ) -> Result<Vec<RecoveryCheckpointCandidate>, DurableAuthorityStoreError> {
            Ok(self
                .checkpoints
                .lock()
                .expect("conformance checkpoints")
                .iter()
                .cloned()
                .map(RecoveryCheckpointCandidate::Trusted)
                .collect())
        }

        fn read_journal_head(
            &self,
            store: &AuthorityStoreCoordinate,
        ) -> Result<Option<JournalSequenceV1>, DurableAuthorityStoreError> {
            if !self.admits(store) {
                return Err(DurableAuthorityStoreErrorKind::Unavailable.into());
            }
            Ok(self
                .entries
                .lock()
                .expect("conformance journal")
                .last()
                .map(SequencedAuthorityEntry::sequence))
        }

        fn read_journal_page(
            &self,
            request: &JournalTailPageRead,
        ) -> Result<JournalTailPage, DurableAuthorityStoreError> {
            if !self.admits(request.store()) {
                return Err(DurableAuthorityStoreErrorKind::Unavailable.into());
            }
            let entries = self.entries.lock().expect("conformance journal");
            let page = entries
                .iter()
                .filter(|entry| {
                    request.after().is_none_or(|after| entry.sequence() > after)
                        && entry.sequence() <= request.through()
                })
                .take(request.maximum_page_entries())
                .cloned()
                .collect::<Vec<_>>();
            let boundary = if page
                .last()
                .is_some_and(|entry| entry.sequence() == request.through())
            {
                JournalTailPageBoundary::Complete
            } else if let Some(resume_after) = page.last().map(SequencedAuthorityEntry::sequence) {
                JournalTailPageBoundary::More { resume_after }
            } else {
                return Err(DurableAuthorityStoreErrorKind::Corrupt(
                    AuthorityByteClass::JournalEntry,
                )
                .into());
            };
            JournalTailPage::new(request, page, boundary)
        }
    }

    impl SessionLeaseCustodyPort for ConformanceAuthorityStore {
        fn compare_and_retain(
            &self,
            body: &SessionLeaseBody,
        ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
            if !self.admits(body.store()) {
                return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
            }
            let mut sessions = self.sessions.lock().expect("conformance sessions");
            match sessions.get(body.digest()) {
                None => {
                    sessions.insert(body.digest().clone(), body.clone());
                    Ok(SessionLeaseBodyRetention::Retained)
                }
                Some(existing) if existing == body => Ok(SessionLeaseBodyRetention::ExactReplay),
                Some(_) => Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                )),
            }
        }

        fn read(
            &self,
            request: &SessionLeaseBodyRead,
        ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
            if !self.admits(request.store()) {
                return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
            }
            let body = self
                .sessions
                .lock()
                .expect("conformance sessions")
                .get(request.digest())
                .cloned();
            if body.as_ref().is_some_and(|body| {
                body.canonical_wire_jcs().as_slice().len() > request.maximum_body_bytes()
            }) {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            Ok(body)
        }
    }

    pub(in super::super) struct UnavailableEventCustody;

    fn unavailable<T>() -> Result<T, ControlStoreError> {
        Err(ControlStoreError::new(ControlStoreErrorKind::Storage))
    }

    impl EventCustodyPort for UnavailableEventCustody {
        fn read_committed_members(
            &mut self,
            _: &EventCustodyHistoryRead,
        ) -> Result<EventCustodyHistoryObservation, ControlStoreError> {
            unavailable()
        }

        fn compare_and_commit(
            &mut self,
            _: &EventCustodyTransition,
            _: usize,
        ) -> Result<EventCustodyCommitOutcome, ControlStoreError> {
            unavailable()
        }

        fn observe_exact(
            &mut self,
            _: &EventCustodyTransition,
            _: usize,
        ) -> Result<EventCustodyReadBackOutcome, ControlStoreError> {
            unavailable()
        }

        fn read_index(
            &mut self,
            _: &AuthorityStoreCoordinate,
            _: &EventIndexQuery,
            _: usize,
        ) -> Result<EventIndexObservation, ControlStoreError> {
            unavailable()
        }

        fn read_record(
            &mut self,
            _: &AuthorityStoreCoordinate,
            _: &ControlRecordCoordinate,
            _: usize,
        ) -> Result<EventRecordObservation, ControlStoreError> {
            unavailable()
        }

        fn read_inbox_page(
            &mut self,
            _: &EventInboxPageRead,
        ) -> Result<EventInboxPage, ControlStoreError> {
            unavailable()
        }

        fn read_pale_derived_outbox_page(
            &mut self,
            _: &PaleDerivedOutboxPageRead,
        ) -> Result<PaleDerivedOutboxPage, ControlStoreError> {
            unavailable()
        }
    }
}

pub(super) mod writer {
    //! Deterministic current-writer evidence for the in-memory frontier.

    use pale_core::records::*;
    use pale_ports::*;

    use crate::resident::{
        record_projection, ResidentServiceCurrentWriterAdmissionPort,
        ResidentServiceCurrentWriterEvidence,
    };

    use super::principal::{record_ref, signed_wire};

    fn schema(id: &str) -> SchemaDigest {
        pale_contracts::record_schema_identity(id)
            .expect("registered writer schema")
            .schema_sha256
    }

    fn sha(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    /// Held process-local exclusion for the deterministic principal.
    pub(in super::super) struct ConformanceWriterGuard(pub(in super::super) ResidentPrincipalIdV1);

    impl PrincipalWriterGuardPort for ConformanceWriterGuard {
        fn principal_id(&self) -> &ResidentPrincipalIdV1 {
            &self.0
        }
    }

    pub(in super::super) struct ConformanceWriterAdmission {
        writer: PrincipalResidentWriterFenceV1,
        checkpoint: ControlStateCheckpointV1,
    }

    impl ConformanceWriterAdmission {
        pub(in super::super) fn construct() -> (Self, PrincipalResidentWriterSnapshot) {
            let writer_schema = schema(PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA_ID);
            let writer_preimage = PrincipalResidentWriterFencePreimageV1 {
                schema_id: SchemaId::parse(PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA)
                    .expect("writer schema"),
                schema_sha256: writer_schema.clone(),
                writer_fence_id: PrincipalResidentWriterFenceIdV1::parse("conformance-writer")
                    .expect("writer fence"),
                principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
                writer_generation: PrincipalResidentWriterGenerationV1::new(1)
                    .expect("writer generation"),
                previous_writer_fence_ref: None,
                previous_writer_fence_sha256: None,
                prior_terminal_receipt_ref: None,
                prior_terminal_receipt_sha256: None,
                takeover_receipt_ref: None,
                takeover_receipt_sha256: None,
                primary_node_id: ResidentNodeIdV1::parse("conformance-node").expect("node"),
                process_instance_id: ResidentProcessInstanceIdV1::parse("conformance-process")
                    .expect("process"),
                boot_incarnation_id: ResidentBootIncarnationIdV1::parse("conformance-boot")
                    .expect("boot"),
                trust_store_id: ControlStoreIdV1::parse("trust-store-1").expect("trust store"),
                trust_store_incarnation_id: ControlIncarnationIdV1::parse("trust-incarnation-1")
                    .expect("trust incarnation"),
                opened_deployment_store_heads_root_sha256: ArtifactDigest::hash_bytes(
                    b"interface conformance opened deployments",
                ),
                principal_session_floor_epoch: PrincipalSessionFloorEpochV1::new(1)
                    .expect("session floor"),
                state: PrincipalResidentWriterFenceStateV1::Active,
                acquired_at: UnixTimeV1::new(50).expect("writer time"),
                retired_at: None,
                issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1")
                    .expect("principal"),
                issuer_key_id: ResidentKeyIdV1::parse("root-key-1").expect("root key"),
                issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("root epoch"),
            };
            let writer_digest =
                PrincipalResidentWriterFenceDigest::hash_canonical(&writer_preimage)
                    .expect("writer digest");
            let principal = writer_preimage.principal_id.clone();
            let key = writer_preimage.issuer_key_id.clone();
            let writer = PrincipalResidentWriterFenceV1::admit_initial_active_wire(
                signed_wire(
                    &writer_preimage,
                    "canonical_principal_resident_writer_fence_sha256",
                    &writer_digest,
                ),
                &PrincipalResidentWriterFenceHeadAdmissionStateV1::Unallocated,
                &writer_schema,
                &PrincipalKeyAdmissionContextV1::new(
                    &principal,
                    &key,
                    PrincipalKeyEpochV1::new(1).expect("root epoch"),
                ),
                |_, _, _, _| Ok(()),
            )
            .expect("writer fence admits");

            let checkpoint_schema = schema(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let checkpoint_preimage = ControlStateCheckpointPreimageV1 {
                schema_id: SchemaId::parse(CONTROL_STATE_CHECKPOINT_V1_SCHEMA)
                    .expect("checkpoint schema"),
                schema_sha256: checkpoint_schema.clone(),
                control_checkpoint_id: ControlCheckpointIdV1::parse("writer-checkpoint-1")
                    .expect("writer checkpoint"),
                control_store_id: writer.trust_store_id().clone(),
                control_incarnation_id: writer.trust_store_incarnation_id().clone(),
                checkpoint_domain_id: ControlCheckpointDomainIdV1::parse(
                    "principal-writer:principal-1",
                )
                .expect("writer domain"),
                domain_epoch: ControlDomainEpochV1::new(1).expect("domain epoch"),
                domain_sequence: ControlDomainSequenceV1::new(1).expect("domain sequence"),
                previous_control_checkpoint_ref: None,
                previous_control_checkpoint_sha256: None,
                checkpoint_profile_ref: record_ref("writer-checkpoint-profile"),
                checkpoint_profile_sha256: CheckpointProfileDigest::parse(sha('2'))
                    .expect("checkpoint profile"),
                control_schema_version: 1,
                committed_transaction_sequence: ControlTransactionSequenceV1::new(1)
                    .expect("transaction"),
                state_root_sha256: ArtifactDigest::hash_bytes(b"interface writer state"),
                leaf_count: ControlLeafCountV1::new(1).expect("leaf count"),
                changed_leaf_keys: vec![ChangedControlLeafKeyV1 {
                    record_class: ControlRecordClassV1::parse("principal-resident-writer")
                        .expect("record class"),
                    scope_key_sha256: ArtifactDigest::hash_bytes(b"principal-1"),
                    logical_key: ControlLogicalKeyV1::parse("writer-1").expect("writer key"),
                }],
                unresolved_custody_root_sha256: ArtifactDigest::hash_bytes(
                    b"no unresolved custody",
                ),
                issued_at: UnixTimeV1::new(51).expect("checkpoint time"),
                checkpoint_signing_authority_ref: record_ref("writer-checkpoint-authority"),
                checkpoint_signing_authority_sha256: CheckpointSigningAuthorityDigest::parse(sha(
                    '3',
                ))
                .expect("checkpoint authority"),
                issuer_principal_id: principal.clone(),
                issuer_key_id: ResidentKeyIdV1::parse("checkpoint-key-1").expect("checkpoint key"),
                issuer_key_epoch: SigningKeyEpochV1::new(1).expect("checkpoint key epoch"),
            };
            let checkpoint_digest =
                ControlStateCheckpointDigest::hash_canonical(&checkpoint_preimage)
                    .expect("checkpoint digest");
            let checkpoint = ControlStateCheckpointV1::admit_initial_wire(
                signed_wire(
                    &checkpoint_preimage,
                    "canonical_control_checkpoint_sha256",
                    &checkpoint_digest,
                ),
                &checkpoint_schema,
                &ControlCheckpointSigningAuthorityAdmissionContextV1 {
                    authority_ref: &checkpoint_preimage.checkpoint_signing_authority_ref,
                    authority_sha256: &checkpoint_preimage.checkpoint_signing_authority_sha256,
                    principal_id: &checkpoint_preimage.issuer_principal_id,
                    control_store_id: &checkpoint_preimage.control_store_id,
                    control_incarnation_id: &checkpoint_preimage.control_incarnation_id,
                    checkpoint_domain_id: &checkpoint_preimage.checkpoint_domain_id,
                    checkpoint_profile_ref: &checkpoint_preimage.checkpoint_profile_ref,
                    checkpoint_profile_sha256: &checkpoint_preimage.checkpoint_profile_sha256,
                    subject_key_id: &checkpoint_preimage.issuer_key_id,
                    subject_key_epoch: checkpoint_preimage.issuer_key_epoch,
                },
                |_, _, _, _| Ok(()),
            )
            .expect("writer checkpoint admits");

            let writer_ref = record_ref("current-principal-writer");
            let checkpoint_ref = record_ref("current-principal-writer-checkpoint");
            let writer_coordinate = record_projection::coordinate(
                &writer_ref,
                writer
                    .canonical_principal_resident_writer_fence_sha256()
                    .as_str(),
            )
            .expect("writer coordinate");
            let checkpoint_coordinate = record_projection::coordinate(
                &checkpoint_ref,
                checkpoint.canonical_control_checkpoint_sha256().as_str(),
            )
            .expect("checkpoint coordinate");
            let snapshot = PrincipalResidentWriterSnapshot::new(
                PrincipalTrustStoreBinding::new(
                    writer.trust_store_id().clone(),
                    writer.trust_store_incarnation_id().clone(),
                    writer.principal_id().clone(),
                ),
                PrincipalResidentWriterHead::new(
                    writer.writer_fence_id().clone(),
                    writer.principal_id().clone(),
                    *writer.writer_generation(),
                    *writer.state(),
                    writer_coordinate.clone(),
                    ControlCheckpointHead::new(
                        checkpoint.checkpoint_domain_id().clone(),
                        *checkpoint.domain_epoch(),
                        *checkpoint.domain_sequence(),
                        *checkpoint.committed_transaction_sequence(),
                        checkpoint_coordinate.clone(),
                    ),
                ),
                record_projection::record(writer_coordinate, &writer).expect("writer record"),
                record_projection::record(checkpoint_coordinate, &checkpoint)
                    .expect("checkpoint record"),
            )
            .expect("writer snapshot closes");
            (Self { writer, checkpoint }, snapshot)
        }
    }

    impl ResidentServiceCurrentWriterAdmissionPort for ConformanceWriterAdmission {
        fn admit_current_writer(
            &mut self,
            _: &PrincipalResidentWriterSnapshot,
        ) -> Result<ResidentServiceCurrentWriterEvidence, InvalidResidentRecordV1> {
            Ok(ResidentServiceCurrentWriterEvidence {
                writer: self.writer.clone(),
                checkpoint: self.checkpoint.clone(),
            })
        }
    }
}

pub(super) use anchor::*;
pub(super) use custody::*;
pub(super) use storage::*;
