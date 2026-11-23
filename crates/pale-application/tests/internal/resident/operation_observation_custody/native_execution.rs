//! Application-phase custody using the existing recording ports and finite tree model.
//! These helpers do not prove native Merkle roots, signatures or durable storage.

use super::*;
use crate::local_operation_start_fixture::checkpoint_preimage_at_store;
use crate::resident::{CompletedOperationObservationCustody, IssuedLocalOperation};
use pale_ports::{
    AccumulatorMutation, AccumulatorPosition, AccumulatorPreparationLimits, AntiRollbackAnchorKey,
    ControlTransactionPreparation, OperationAccumulatorPreparation, OperationCollectionSnapshot,
    OperationCustodyIndexMutation, OperationCustodyIndexObservation, OperationCustodyRecordKind,
    OperationPublicationReadError, OperationPublicationSources, PreparedOperationAccumulator,
    StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

struct ServiceStore {
    transaction: ControlTransactionSequenceV1,
    handle_record: Option<ControlRecord>,
    lease_record: Option<ControlRecord>,
    retained_records: Vec<ControlRecord>,
    history: Vec<OperationCustodyCandidate>,
    latest_candidate: Option<OperationCustodyCandidate>,
    preparations: usize,
    read_backs: usize,
    commits: usize,
    conflicts_on_commit: Vec<usize>,
    unknown_exact_commits: Vec<usize>,
    unavailable_read_backs: Vec<usize>,
    uncertain_exact: bool,
    fail_a2a_preflight_once: bool,
    conflict_active: bool,
    competing_head: Rc<RefCell<Option<ControlRecord>>>,
    collection_snapshot: Option<OperationCollectionSnapshot>,
}

impl ServiceStore {
    fn retained_record(
        &self,
        coordinate: &pale_ports::ControlRecordCoordinate,
    ) -> Option<ControlRecord> {
        self.history
            .iter()
            .find_map(|candidate| {
                candidate
                    .records()
                    .iter()
                    .map(OperationCustodyRecord::record)
                    .chain(candidate.a2a_retained_records())
                    .chain(candidate.a2a_associated_route())
                    .chain(std::iter::once(candidate.checkpoint().successor_record()))
                    .chain(candidate.checkpoint().predecessor_record())
                    .find(|record| record.coordinate() == coordinate)
                    .cloned()
            })
            .or_else(|| {
                self.retained_records
                    .iter()
                    .find(|record| record.coordinate() == coordinate)
                    .cloned()
            })
            .or_else(|| {
                self.lease_record
                    .as_ref()
                    .filter(|record| record.coordinate() == coordinate)
                    .cloned()
            })
    }

    fn indexed_record(
        &self,
        query: &pale_ports::OperationCustodyIndexQuery,
    ) -> Option<ControlRecord> {
        self.history
            .iter()
            .rev()
            .find_map(|candidate| super::candidate_indexed_record(candidate, query))
    }
}

impl pale_ports::ControlTransactionPort for ServiceStore {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        ControlTransactionPreparation::new(store.clone(), self.transaction)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
    }
}

impl OperationCustodyPort for ServiceStore {
    fn read_collection_snapshot(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum_frontier_bytes: usize,
        maximum_journal_payload_bytes: usize,
    ) -> Result<Option<OperationCollectionSnapshot>, ControlStoreError> {
        let Some(snapshot) = self.collection_snapshot.as_ref() else {
            return Ok(None);
        };
        if snapshot.evidence().operation_handle() != handle
            || snapshot.frontier().canonical_record_jcs().as_slice().len() > maximum_frontier_bytes
            || snapshot.journal_payload().as_slice().len() > maximum_journal_payload_bytes
            || self
                .history
                .iter()
                .any(|candidate| candidate.store() != store)
        {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        Ok(Some(snapshot.clone()))
    }

    fn read_operation_starts(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&pale_ports::OperationStartCursor>,
        maximum_sources: usize,
        maximum_session_basis_bytes: usize,
    ) -> Result<pale_ports::OperationStartSources, pale_ports::OperationStartReadError> {
        if maximum_sources == 0 || maximum_session_basis_bytes == 0 {
            return Err(pale_ports::OperationStartReadError::Limit);
        }
        let through = cursor.map_or_else(
            || {
                self.history
                    .last()
                    .map(|candidate| {
                        candidate
                            .checkpoint()
                            .successor()
                            .committed_transaction_sequence()
                    })
                    .unwrap_or(self.transaction)
            },
            pale_ports::OperationStartCursor::through,
        );
        if cursor.is_some_and(|cursor| cursor.store() != store) {
            return Err(pale_ports::OperationStartReadError::Storage(
                ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired),
            ));
        }
        let starts = self
            .history
            .iter()
            .filter_map(|candidate| {
                let basis = candidate.session_basis()?;
                let transaction = candidate
                    .checkpoint()
                    .successor()
                    .committed_transaction_sequence();
                (transaction.get() <= through.get()).then_some((transaction, basis))
            })
            .collect::<Vec<_>>();
        let after = match cursor {
            None => 0,
            Some(cursor) => starts
                .iter()
                .position(|(transaction, basis)| {
                    *transaction == cursor.after_transaction()
                        && basis.handle() == cursor.handle()
                        && basis.request() == cursor.request()
                        && basis.session_lease_digest() == cursor.session_lease_digest()
                        && basis.basis_sha256() == cursor.session_basis_digest()
                })
                .map(|position| position + 1)
                .ok_or_else(|| {
                    pale_ports::OperationStartReadError::Storage(ControlStoreError::new(
                        ControlStoreErrorKind::RecoveryRequired,
                    ))
                })?,
        };
        let Some((transaction, basis)) = starts.get(after) else {
            return Ok(pale_ports::OperationStartSources::new(
                through,
                Vec::new(),
                None,
            ));
        };
        if basis.canonical_basis_jcs().len() > maximum_session_basis_bytes {
            return Err(pale_ports::OperationStartReadError::Limit);
        }
        let source = pale_ports::OperationStartSource::new(*transaction, (*basis).clone());
        let continuation = (after + 1 < starts.len())
            .then(|| {
                pale_ports::OperationStartCursor::new(
                    store.clone(),
                    through,
                    *transaction,
                    basis.handle().clone(),
                    basis.request().clone(),
                    basis.session_lease_digest().clone(),
                    basis.basis_sha256().clone(),
                )
            })
            .transpose()
            .map_err(pale_ports::OperationStartReadError::Storage)?;
        Ok(pale_ports::OperationStartSources::new(
            through,
            vec![source],
            continuation,
        ))
    }

    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &pale_ports::OperationA2aBindingScope,
        cursor: Option<&pale_ports::OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationA2aBindings, ControlStoreError> {
        if maximum_bindings == 0
            || maximum_body_bytes == 0
            || cursor.is_some_and(|cursor| cursor.store() != store || cursor.scope() != scope)
        {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        let through = cursor.map_or_else(
            || {
                self.history
                    .last()
                    .map(|candidate| {
                        candidate
                            .checkpoint()
                            .successor()
                            .committed_transaction_sequence()
                    })
                    .unwrap_or(self.transaction)
            },
            pale_ports::OperationA2aBindingCursor::through,
        );
        let mut sources = self
            .history
            .iter()
            .filter_map(|candidate| {
                let transaction = candidate
                    .checkpoint()
                    .successor()
                    .committed_transaction_sequence();
                if transaction.get() > through.get() {
                    return None;
                }
                candidate.indexes().iter().find_map(|index| {
                    let pale_ports::OperationCustodyIndexMutation::A2aTaskBinding {
                        operation_key,
                        key,
                        successor,
                        ..
                    } = index
                    else {
                        return None;
                    };
                    if operation_key != scope.operation_key() {
                        return None;
                    }
                    let binding = candidate
                        .record(OperationCustodyRecordKind::A2aTaskBinding)
                        .filter(|record| record.coordinate() == successor)?
                        .clone();
                    pale_ports::OperationA2aBindingSource::new(key.clone(), transaction, binding)
                        .ok()
                })
            })
            .filter(|source| {
                cursor.is_none_or(|cursor| {
                    (
                        source.transaction().get(),
                        source.binding().coordinate().record_ref().as_str(),
                    ) > (
                        cursor.after_transaction().get(),
                        cursor.after_reference().as_str(),
                    )
                })
            })
            .collect::<Vec<_>>();
        sources.sort_by(|left, right| {
            (
                left.transaction().get(),
                left.binding().coordinate().record_ref().as_str(),
            )
                .cmp(&(
                    right.transaction().get(),
                    right.binding().coordinate().record_ref().as_str(),
                ))
        });
        let mut returned = Vec::new();
        let mut returned_bytes = 0_usize;
        for source in sources.iter().take(maximum_bindings) {
            returned_bytes = returned_bytes
                .checked_add(source.binding().canonical_record_jcs().as_slice().len())
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            if returned_bytes > maximum_body_bytes {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
            returned.push(source.clone());
        }
        let continuation = if sources.len() > returned.len() {
            let last = returned
                .last()
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))?;
            Some(pale_ports::OperationA2aBindingCursor::new(
                store.clone(),
                scope.clone(),
                through,
                last.transaction(),
                last.binding().coordinate().record_ref().clone(),
            )?)
        } else {
            None
        };
        Ok(pale_ports::OperationA2aBindings {
            bindings: returned,
            continuation,
        })
    }

    fn read_publication_sources(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: Option<&pale_ports::OperationPublicationCursor>,
        _: usize,
        _: usize,
    ) -> Result<OperationPublicationSources, OperationPublicationReadError> {
        panic!("provider-dispatch retention does not read publication sources")
    }

    fn read_operation_session_basis(
        &mut self,
        _: &AuthorityStoreCoordinate,
        handle: &pale_ports::ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<pale_ports::OperationSessionBasis>, ControlStoreError> {
        Ok(self
            .history
            .iter()
            .find(|candidate| {
                candidate
                    .record(OperationCustodyRecordKind::OperationHandle)
                    .is_some_and(|record| record.coordinate() == handle)
            })
            .and_then(OperationCustodyCandidate::session_basis)
            .cloned())
    }

    fn read_checkpoint_head(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<Option<(pale_ports::ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        Ok(self.history.last().map(|candidate| {
            (
                candidate.checkpoint().successor().clone(),
                candidate.checkpoint().successor_record().clone(),
            )
        }))
    }

    fn verify_accumulator_membership(
        &mut self,
        request: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        Ok(pale_ports::VerifiedOperationAccumulatorMembership::new(
            request.clone(),
        ))
    }

    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, pale_ports::OperationAccumulatorPreparationError>
    {
        self.preparations += 1;
        let pale_ports::AccumulatorPreparationBasis::Successor(predecessor) = request.basis()
        else {
            panic!("provider dispatch always extends an admitted checkpoint")
        };
        let marker = u8::try_from(request.transaction().successor().get())
            .expect("modeled transaction fits one marker byte");
        let next_root = ArtifactDigest::hash_bytes(&[marker; 32]);
        let state = StateAccumulatorBatch::new(
            predecessor.state_root().clone(),
            next_root,
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([marker; 32]),
                Some(vec![marker; 32]),
                Some(vec![marker + 1; 32]),
                32,
            )
            .expect("modeled state mutation")],
            1,
        )
        .expect("modeled state batch");
        let unresolved = UnresolvedCustodyAccumulatorBatch::new(
            predecessor.unresolved_root().clone(),
            predecessor.unresolved_root().clone(),
            vec![],
            1,
        )
        .expect("unchanged unresolved tree");
        PreparedOperationAccumulator::new(request.clone(), state, unresolved)
            .map_err(|_| pale_ports::OperationAccumulatorPreparationError::Limit)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("provider-dispatch retention does not retain a new lease")
    }

    fn read_operation_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        let record = self.retained_record(coordinate);
        if let Some(record) = &record {
            assert!(record.canonical_record_jcs().as_slice().len() <= maximum);
        }
        Ok(record)
    }

    fn read_a2a_evidence(
        &mut self,
        _: &AuthorityStoreCoordinate,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum: usize,
    ) -> Result<Option<pale_ports::OperationA2aEvidence>, ControlStoreError> {
        let evidence = self
            .history
            .iter()
            .rev()
            .flat_map(OperationCustodyCandidate::a2a_evidence)
            .find(|evidence| evidence.coordinate() == coordinate)
            .cloned();
        if let Some(evidence) = &evidence {
            if evidence.byte_len() > maximum {
                return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
            }
        }
        Ok(evidence)
    }

    fn read_a2a_evidence_inserted_at(
        &mut self,
        _: &AuthorityStoreCoordinate,
        transaction: ControlTransactionSequenceV1,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<Vec<pale_ports::OperationA2aEvidence>, ControlStoreError> {
        let evidence = self
            .history
            .iter()
            .find(|candidate| {
                candidate
                    .checkpoint()
                    .successor()
                    .committed_transaction_sequence()
                    == transaction
            })
            .map(OperationCustodyCandidate::a2a_evidence)
            .unwrap_or_default();
        let byte_len = evidence.iter().try_fold(0_usize, |total, item| {
            total
                .checked_add(item.byte_len())
                .ok_or_else(|| ControlStoreError::new(ControlStoreErrorKind::BoundExceeded))
        })?;
        if evidence.len() > maximum_items || byte_len > maximum_bytes {
            return Err(ControlStoreError::new(ControlStoreErrorKind::BoundExceeded));
        }
        Ok(evidence.to_vec())
    }

    fn compare_and_commit(
        &mut self,
        candidate: &pale_ports::OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.commits += 1;
        if self.unknown_exact_commits.contains(&self.commits) {
            self.transaction = candidate
                .checkpoint()
                .successor()
                .committed_transaction_sequence();
            self.latest_candidate = Some(candidate.clone());
            self.history.push(candidate.clone());
            self.uncertain_exact = true;
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::OutcomeUnknown,
            ));
        }
        if self.conflicts_on_commit.contains(&self.commits) {
            self.transaction = candidate
                .checkpoint()
                .successor()
                .committed_transaction_sequence();
            self.conflict_active = true;
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::CandidateConflict,
            ));
        }
        self.transaction = candidate
            .checkpoint()
            .successor()
            .committed_transaction_sequence();
        self.latest_candidate = Some(candidate.clone());
        self.history.push(candidate.clone());
        self.conflict_active = false;
        Ok(OperationCustodyCommitOutcome::Committed)
    }

    fn observe_exact(
        &mut self,
        _: &pale_ports::OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        self.read_backs += 1;
        if self.unavailable_read_backs.contains(&self.read_backs) {
            return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
        }
        if std::mem::take(&mut self.uncertain_exact) {
            Ok(OperationCustodyReadBackOutcome::ExactSuccessor)
        } else {
            Ok(OperationCustodyReadBackOutcome::Predecessor)
        }
    }

    fn read_index(
        &mut self,
        _: &AuthorityStoreCoordinate,
        query: &pale_ports::OperationCustodyIndexQuery,
        _: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        if self.fail_a2a_preflight_once
            && matches!(
                query,
                pale_ports::OperationCustodyIndexQuery::A2aSendPreparation(_)
                    | pale_ports::OperationCustodyIndexQuery::A2aSendPreparationCheckpoint(_)
            )
        {
            self.fail_a2a_preflight_once = false;
            return Err(ControlStoreError::new(ControlStoreErrorKind::Storage));
        }
        let initial = self.history.get(1).or_else(|| self.history.first());
        let latest = self.history.last();
        if !(self.conflict_active
            && matches!(
                query,
                pale_ports::OperationCustodyIndexQuery::LatestObservation(_)
            ))
        {
            if let Some(record) = self.indexed_record(query) {
                return OperationCustodyIndexObservation::exact(query.clone(), record)
                    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired));
            }
        }
        let a2a_successor = self.history.iter().rev().find_map(|candidate| {
            candidate
                .indexes()
                .iter()
                .find_map(|index| match (query, index) {
                    (
                        pale_ports::OperationCustodyIndexQuery::A2aTaskBinding(expected),
                        OperationCustodyIndexMutation::A2aTaskBinding { key, successor, .. },
                    )
                    | (
                        pale_ports::OperationCustodyIndexQuery::FirstA2aTaskObservation(expected),
                        OperationCustodyIndexMutation::FirstA2aTaskObservation { key, successor },
                    )
                    | (
                        pale_ports::OperationCustodyIndexQuery::LatestA2aTaskObservation(expected),
                        OperationCustodyIndexMutation::LatestA2aTaskObservation {
                            key,
                            successor,
                            ..
                        },
                    )
                    | (
                        pale_ports::OperationCustodyIndexQuery::A2aTaskObservationCheckpoint(
                            expected,
                        ),
                        OperationCustodyIndexMutation::A2aTaskObservationCheckpoint {
                            key,
                            successor,
                        },
                    ) if key == expected => Some(successor.clone()),
                    (
                        pale_ports::OperationCustodyIndexQuery::CurrentA2aGetTask(expected),
                        OperationCustodyIndexMutation::CurrentA2aGetTask { key, successor, .. },
                    )
                    | (
                        pale_ports::OperationCustodyIndexQuery::A2aGetTaskCheckpoint(expected),
                        OperationCustodyIndexMutation::A2aGetTaskCheckpoint { key, successor },
                    ) if key == expected => Some(successor.clone()),
                    _ => None,
                })
        });
        let record = match query {
            pale_ports::OperationCustodyIndexQuery::A2aTaskBinding(_)
            | pale_ports::OperationCustodyIndexQuery::FirstA2aTaskObservation(_)
            | pale_ports::OperationCustodyIndexQuery::LatestA2aTaskObservation(_)
            | pale_ports::OperationCustodyIndexQuery::A2aTaskObservationCheckpoint(_)
            | pale_ports::OperationCustodyIndexQuery::CurrentA2aGetTask(_)
            | pale_ports::OperationCustodyIndexQuery::A2aGetTaskCheckpoint(_) => a2a_successor
                .and_then(|coordinate| {
                    self.history.iter().find_map(|candidate| {
                        candidate
                            .records()
                            .iter()
                            .map(OperationCustodyRecord::record)
                            .chain(std::iter::once(candidate.checkpoint().successor_record()))
                            .chain(candidate.checkpoint().predecessor_record())
                            .find(|record| record.coordinate() == &coordinate)
                            .cloned()
                    })
                }),
            pale_ports::OperationCustodyIndexQuery::OperationHandle(_) => {
                self.handle_record.clone().or_else(|| {
                    initial.and_then(|candidate| {
                        candidate
                            .record(OperationCustodyRecordKind::OperationHandle)
                            .cloned()
                    })
                })
            }
            pale_ports::OperationCustodyIndexQuery::LatestObservation(_)
                if self.conflict_active =>
            {
                self.competing_head.borrow().clone().or_else(|| {
                    latest
                        .and_then(|candidate| {
                            candidate.record(OperationCustodyRecordKind::OperationObservation)
                        })
                        .cloned()
                })
            }
            pale_ports::OperationCustodyIndexQuery::LatestObservation(_) => latest.and_then(|_| {
                self.history.iter().rev().find_map(|candidate| {
                    candidate
                        .a2a_retained_records()
                        .iter()
                        .find(|record| {
                            record.schema_id().as_str()
                                == pale_core::records::OPERATION_OBSERVATION_V1_SCHEMA
                        })
                        .cloned()
                        .or_else(|| {
                            candidate
                                .record(OperationCustodyRecordKind::OperationObservation)
                                .cloned()
                        })
                })
            }),
            pale_ports::OperationCustodyIndexQuery::RequestForHandle(_)
            | pale_ports::OperationCustodyIndexQuery::RequestEvidence(_) => initial
                .and_then(|candidate| candidate.record(OperationCustodyRecordKind::CommonRequest))
                .cloned(),
            pale_ports::OperationCustodyIndexQuery::RequestOperation(_) => initial
                .and_then(|candidate| candidate.record(OperationCustodyRecordKind::OperationHandle))
                .cloned(),
            pale_ports::OperationCustodyIndexQuery::StartObservation(_) => initial
                .and_then(|candidate| {
                    candidate.record(OperationCustodyRecordKind::OperationObservation)
                })
                .cloned(),
            pale_ports::OperationCustodyIndexQuery::ObservationCheckpoint(key) => self
                .history
                .iter()
                .find(|candidate| {
                    candidate
                        .record(OperationCustodyRecordKind::OperationObservation)
                        .is_some_and(|observation| {
                            pale_ports::OperationCustodyIndexKey::for_observation(
                                observation.coordinate(),
                            ) == *key
                        })
                })
                .map(|candidate| candidate.checkpoint().successor_record().clone())
                .or_else(|| {
                    self.history.iter().find_map(|candidate| {
                        candidate
                            .a2a_retained_records()
                            .iter()
                            .find(|record| {
                                pale_ports::OperationCustodyIndexKey::for_observation(
                                    record.coordinate(),
                                ) == *key
                            })
                            .and_then(|_| candidate.checkpoint().predecessor_record().cloned())
                    })
                })
                .or_else(|| {
                    latest.map(|candidate| candidate.checkpoint().successor_record().clone())
                }),
            pale_ports::OperationCustodyIndexQuery::A2aSendPreparation(_) => {
                self.history.iter().rev().find_map(|candidate| {
                    candidate
                        .record(OperationCustodyRecordKind::A2aSendPreparation)
                        .cloned()
                })
            }
            pale_ports::OperationCustodyIndexQuery::A2aSendPreparationCheckpoint(_) => self
                .history
                .iter()
                .rev()
                .find(|candidate| {
                    candidate
                        .record(OperationCustodyRecordKind::A2aSendPreparation)
                        .is_some()
                })
                .map(|candidate| candidate.checkpoint().successor_record().clone()),
            _ => panic!("provider-dispatch release uses only operation observation indexes"),
        };
        record.map_or_else(
            || Ok(OperationCustodyIndexObservation::absent(query.clone())),
            |record| {
                OperationCustodyIndexObservation::exact(query.clone(), record)
                    .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
            },
        )
    }
}

struct ServiceAnchor {
    current: Option<AntiRollbackAnchor>,
    unavailable_advances: Vec<usize>,
    unknown_exact_advances: Vec<usize>,
    unknown_failed_observation_advances: Vec<usize>,
    fail_next_ambiguous_observation: bool,
    later_observations_active: bool,
    later_observations: VecDeque<ServiceAnchorLaterObservation>,
    observations: usize,
    advances: usize,
}

/// Scripted later evidence after an ambiguous advance could not be observed.
#[derive(Clone, Copy)]
pub(in crate::resident) enum ServiceAnchorLaterObservation {
    ExactSuccessor,
    ExactPredecessor,
    Missing,
    Unavailable,
}

impl AntiRollbackAnchorPort for ServiceAnchor {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        Ok(self
            .current
            .as_ref()
            .filter(|anchor| &anchor.key() == key)
            .cloned())
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.advances += 1;
        if self.unavailable_advances.contains(&self.advances) {
            return Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ));
        }
        self.current = Some(transition.next().clone());
        if self
            .unknown_failed_observation_advances
            .contains(&self.advances)
        {
            self.fail_next_ambiguous_observation = true;
            return Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::OutcomeUnknown,
            ));
        }
        if self.unknown_exact_advances.contains(&self.advances) {
            return Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::OutcomeUnknown,
            ));
        }
        Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations += 1;
        if std::mem::take(&mut self.fail_next_ambiguous_observation) {
            self.later_observations_active = true;
            return Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ));
        }
        if self.later_observations_active {
            if let Some(observation) = self.later_observations.pop_front() {
                return match observation {
                    ServiceAnchorLaterObservation::ExactSuccessor => {
                        Ok(AntiRollbackAnchorObservation::ExactSuccessor)
                    }
                    ServiceAnchorLaterObservation::ExactPredecessor => {
                        Ok(AntiRollbackAnchorObservation::ExactPredecessor)
                    }
                    ServiceAnchorLaterObservation::Missing => {
                        Ok(AntiRollbackAnchorObservation::Missing)
                    }
                    ServiceAnchorLaterObservation::Unavailable => Err(
                        AntiRollbackAnchorError::new(AntiRollbackAnchorErrorKind::Unavailable),
                    ),
                };
            }
        }
        Ok(if self.current.as_ref() == Some(transition.next()) {
            AntiRollbackAnchorObservation::ExactSuccessor
        } else {
            AntiRollbackAnchorObservation::ExactPredecessor
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn with_service_custody_and_handle<R>(
    initial: &ControlStateCheckpointV1,
    initial_candidate: Option<&OperationCustodyCandidate>,
    lease_record: Option<ControlRecord>,
    expected_appends: usize,
    expected_anchor_advances: usize,
    fail_anchor_advance: bool,
    conflicts_on_commit: &[usize],
    handle_record: Option<ControlRecord>,
    competing_head: Rc<RefCell<Option<ControlRecord>>>,
    run: impl FnOnce(&mut dyn OperationCustodyPort, &mut dyn AntiRollbackAnchorPort) -> R,
) -> R {
    with_service_custody_history_and_handle(
        initial,
        initial_candidate,
        lease_record,
        Vec::new(),
        expected_appends,
        expected_anchor_advances,
        fail_anchor_advance,
        conflicts_on_commit,
        handle_record,
        competing_head,
        run,
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn with_service_custody_history_and_handle<R>(
    initial: &ControlStateCheckpointV1,
    initial_candidate: Option<&OperationCustodyCandidate>,
    lease_record: Option<ControlRecord>,
    retained_records: Vec<ControlRecord>,
    expected_appends: usize,
    expected_anchor_advances: usize,
    fail_anchor_advance: bool,
    conflicts_on_commit: &[usize],
    handle_record: Option<ControlRecord>,
    competing_head: Rc<RefCell<Option<ControlRecord>>>,
    run: impl FnOnce(&mut dyn OperationCustodyPort, &mut dyn AntiRollbackAnchorPort) -> R,
) -> R {
    let unavailable_advances = fail_anchor_advance
        .then_some(1)
        .into_iter()
        .collect::<Vec<_>>();
    with_service_custody_faults(
        initial,
        initial_candidate,
        Vec::new(),
        lease_record,
        retained_records,
        expected_appends,
        expected_appends,
        expected_appends,
        expected_anchor_advances,
        expected_appends,
        conflicts_on_commit,
        &[],
        &[],
        &unavailable_advances,
        &[],
        &[],
        Vec::new(),
        false,
        handle_record,
        competing_head,
        run,
    )
}

/// Run the resident custody model with phase-specific uncertainty injection.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn with_service_custody_faults<R>(
    initial: &ControlStateCheckpointV1,
    initial_candidate: Option<&OperationCustodyCandidate>,
    preceding_candidates: Vec<OperationCustodyCandidate>,
    lease_record: Option<ControlRecord>,
    retained_records: Vec<ControlRecord>,
    expected_preparations: usize,
    expected_commits: usize,
    expected_read_backs: usize,
    expected_anchor_advances: usize,
    expected_anchor_observations: usize,
    conflicts_on_commit: &[usize],
    unknown_exact_commits: &[usize],
    unavailable_read_backs: &[usize],
    unavailable_advances: &[usize],
    unknown_exact_advances: &[usize],
    unknown_failed_observation_advances: &[usize],
    later_observations: Vec<ServiceAnchorLaterObservation>,
    fail_a2a_preflight_once: bool,
    handle_record: Option<ControlRecord>,
    competing_head: Rc<RefCell<Option<ControlRecord>>>,
    run: impl FnOnce(&mut dyn OperationCustodyPort, &mut dyn AntiRollbackAnchorPort) -> R,
) -> R {
    with_service_custody_faults_and_collection(
        initial,
        initial_candidate,
        preceding_candidates,
        lease_record,
        retained_records,
        expected_preparations,
        expected_commits,
        expected_read_backs,
        expected_anchor_advances,
        expected_anchor_observations,
        conflicts_on_commit,
        unknown_exact_commits,
        unavailable_read_backs,
        unavailable_advances,
        unknown_exact_advances,
        unknown_failed_observation_advances,
        later_observations,
        fail_a2a_preflight_once,
        handle_record,
        competing_head,
        None,
        run,
    )
}

/// Run the custody model with one exact private Consult collection snapshot.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn with_service_custody_faults_and_collection<R>(
    initial: &ControlStateCheckpointV1,
    initial_candidate: Option<&OperationCustodyCandidate>,
    mut preceding_candidates: Vec<OperationCustodyCandidate>,
    lease_record: Option<ControlRecord>,
    retained_records: Vec<ControlRecord>,
    expected_preparations: usize,
    expected_commits: usize,
    expected_read_backs: usize,
    expected_anchor_advances: usize,
    expected_anchor_observations: usize,
    conflicts_on_commit: &[usize],
    unknown_exact_commits: &[usize],
    unavailable_read_backs: &[usize],
    unavailable_advances: &[usize],
    unknown_exact_advances: &[usize],
    unknown_failed_observation_advances: &[usize],
    later_observations: Vec<ServiceAnchorLaterObservation>,
    fail_a2a_preflight_once: bool,
    handle_record: Option<ControlRecord>,
    competing_head: Rc<RefCell<Option<ControlRecord>>>,
    collection_snapshot: Option<OperationCollectionSnapshot>,
    run: impl FnOnce(&mut dyn OperationCustodyPort, &mut dyn AntiRollbackAnchorPort) -> R,
) -> R {
    preceding_candidates.extend(initial_candidate.into_iter().cloned());
    let retained_anchor = preceding_candidates.last().map(|candidate| {
        let head = candidate.checkpoint().successor();
        AntiRollbackAnchor::from(pale_ports::ControlDomainAntiRollbackAnchor::new(
            candidate.store().store_id().clone(),
            candidate.store().incarnation_id().clone(),
            head.checkpoint_domain_id().clone(),
            head.domain_epoch(),
            head.domain_sequence(),
            head.coordinate().clone(),
        ))
    });
    let mut store = ServiceStore {
        transaction: *initial.committed_transaction_sequence(),
        handle_record,
        lease_record,
        retained_records,
        history: preceding_candidates,
        latest_candidate: None,
        preparations: 0,
        read_backs: 0,
        commits: 0,
        conflicts_on_commit: conflicts_on_commit.to_vec(),
        unknown_exact_commits: unknown_exact_commits.to_vec(),
        unavailable_read_backs: unavailable_read_backs.to_vec(),
        uncertain_exact: false,
        fail_a2a_preflight_once,
        conflict_active: false,
        competing_head,
        collection_snapshot,
    };
    let mut anchor = ServiceAnchor {
        current: retained_anchor,
        unavailable_advances: unavailable_advances.to_vec(),
        unknown_exact_advances: unknown_exact_advances.to_vec(),
        unknown_failed_observation_advances: unknown_failed_observation_advances.to_vec(),
        fail_next_ambiguous_observation: false,
        later_observations_active: false,
        later_observations: later_observations.into(),
        observations: 0,
        advances: 0,
    };
    let result = run(&mut store, &mut anchor);
    assert_eq!(store.preparations, expected_preparations);
    assert_eq!(store.read_backs, expected_read_backs);
    assert_eq!(store.commits, expected_commits);
    assert_eq!(anchor.observations, expected_anchor_observations);
    assert_eq!(anchor.advances, expected_anchor_advances);
    result
}

pub(in crate::resident) fn complete(
    append: PreparedOperationObservationAppend<'_>,
    request: &CommonRequestEnvelopeV1,
    predecessor: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    database: OperationCustodyDatabaseEvidence,
    observed_at: u64,
) -> (
    CompletedOperationObservationCustody,
    ControlStateCheckpointV1,
) {
    let prior = predecessor.1;
    let store_coordinate = AuthorityStoreCoordinate::new(
        prior.control_store_id().clone(),
        prior.control_incarnation_id().clone(),
    );
    complete_with_selection(
        append,
        crate::local_operation_start_fixture::profile()
            .select(&store_coordinate, request)
            .unwrap(),
        predecessor,
        database,
        observed_at,
    )
}

pub(in crate::resident) fn complete_with_selection(
    append: PreparedOperationObservationAppend<'_>,
    selection: crate::resident::checkpoint_profile::OperationCheckpointSelection,
    predecessor: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    database: OperationCustodyDatabaseEvidence,
    observed_at: u64,
) -> (
    CompletedOperationObservationCustody,
    ControlStateCheckpointV1,
) {
    let (prior_ref, prior) = predecessor;
    let sequence = prior.domain_sequence().get() + 1;
    let root = ArtifactDigest::hash_bytes(format!("finite model root {sequence}").as_bytes());
    let store_coordinate = AuthorityStoreCoordinate::new(
        prior.control_store_id().clone(),
        prior.control_incarnation_id().clone(),
    );
    let mut port = checkpoint_preparation::PreparationOnly::at_store(
        |request: &pale_ports::OperationAccumulatorPreparation| {
            // Explicit representation model, not an implementation of the native tree.
            let state = StateAccumulatorBatch::new(
                prior.state_root_sha256().clone(),
                root.clone(),
                vec![AccumulatorMutation::new(
                    AccumulatorPosition::Leaf([1; 32]),
                    Some(vec![3; 32]),
                    Some(vec![4; 32]),
                    32,
                )
                .unwrap()],
                1,
            )
            .unwrap();
            let unresolved = UnresolvedCustodyAccumulatorBatch::new(
                prior.unresolved_custody_root_sha256().clone(),
                prior.unresolved_custody_root_sha256().clone(),
                vec![],
                1,
            )
            .unwrap();
            Ok(PreparedOperationAccumulator::new(request.clone(), state, unresolved).unwrap())
        },
        *prior.committed_transaction_sequence(),
        store_coordinate.clone(),
    );
    let prepared = append
        .prepare_checkpoint(
            selection,
            prior_ref,
            prior,
            AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536).unwrap(),
            &mut port,
        )
        .unwrap();
    let reference = record_ref(&format!("operation/checkpoint-{sequence}"));
    let mut preimage = checkpoint_preimage_at_store(
        sequence,
        Some((
            prior_ref.clone(),
            prior.canonical_control_checkpoint_sha256().clone(),
        )),
        &store_coordinate,
    );
    preimage.checkpoint_domain_id = prior.checkpoint_domain_id().clone();
    preimage.checkpoint_profile_ref = prior.checkpoint_profile_ref().clone();
    preimage.checkpoint_profile_sha256 = prior.checkpoint_profile_sha256().clone();
    preimage.committed_transaction_sequence =
        ControlTransactionSequenceV1::new(prior.committed_transaction_sequence().get() + 1)
            .unwrap();
    preimage.state_root_sha256 = prepared.state_root().clone();
    preimage.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    preimage.leaf_count = *prior.leaf_count();
    preimage.changed_leaf_keys = vec![prepared.observation_key().clone()];
    preimage.issued_at = UnixTimeV1::new(observed_at).unwrap();
    let (custody, checkpoint) = prepared
        .issue_checkpoint(
            &reference,
            preimage.clone(),
            &preimage.schema_sha256,
            &checkpoint_authority(&preimage),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    let mut store = RecordingStore::committing(match database {
        OperationCustodyDatabaseEvidence::ExactReplay => OperationCustodyCommitOutcome::ExactReplay,
        _ => OperationCustodyCommitOutcome::Committed,
    });
    if database == OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit {
        store.commit_result = Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        )));
        store
            .read_back_results
            .push_back(Ok(OperationCustodyReadBackOutcome::ExactSuccessor));
    }
    let pending = commit_observation(custody, &mut store).unwrap();
    assert_eq!(pending.database_evidence(), database);
    let mut anchor = RecordingAnchor::advancing();
    if database == OperationCustodyDatabaseEvidence::ExactReplay {
        anchor.observation_result = Some(Ok(AntiRollbackAnchorObservation::ExactSuccessor));
    }
    let completed = pending.advance_anchor(&mut anchor).unwrap();
    assert_eq!(
        completed
            .candidate()
            .checkpoint()
            .successor()
            .coordinate()
            .record_ref(),
        &reference
    );
    assert_eq!(
        completed
            .candidate()
            .checkpoint()
            .successor_record()
            .canonical_record_jcs()
            .as_slice(),
        pale_core::canon::to_canonical_string(&checkpoint)
            .unwrap()
            .as_bytes()
    );
    (completed, checkpoint)
}

struct ReadAnchor(Option<AntiRollbackAnchor>);

impl AntiRollbackAnchorPort for ReadAnchor {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        let value = self.0.take().expect("one anchor read at most");
        assert_eq!(key, &value.key());
        Ok(Some(value))
    }
    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("execution guard cannot advance an anchor")
    }
    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("execution guard reads the retained anchor only")
    }
}

pub(in crate::resident) fn with_reads<R>(
    operation: &IssuedLocalOperation,
    completed: CompletedOperationObservationCustody,
    shifted: bool,
    run: impl FnOnce(
        CompletedOperationObservationCustody,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
    ) -> R,
) -> R {
    let candidate = completed.candidate();
    let checkpoint = candidate.checkpoint();
    let (handle_ref, handle) = operation.handle();
    let key = operation_handle_key(handle.operation_handle_id()).unwrap();
    let handle_record = record_projection::record(
        record_projection::coordinate(
            handle_ref,
            handle.canonical_operation_handle_sha256().as_str(),
        )
        .unwrap(),
        handle,
    )
    .unwrap();
    let observation = candidate
        .records()
        .iter()
        .find_map(|record| match record {
            OperationCustodyRecord::OperationObservation(value) => Some(value.clone()),
            _ => None,
        })
        .unwrap();
    let reads = [
        (
            OperationCustodyIndexQuery::OperationHandle(key.clone()),
            handle_record,
        ),
        (
            OperationCustodyIndexQuery::LatestObservation(key),
            observation.clone(),
        ),
        (
            OperationCustodyIndexQuery::ObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(observation.coordinate()),
            ),
            checkpoint.successor_record().clone(),
        ),
    ]
    .into_iter()
    .map(|(query, record)| OperationCustodyIndexObservation::exact(query, record).unwrap())
    .collect();
    let mut store = RecordingStore::recovering(reads);
    store.checkpoint_reads.push_back(Ok((!shifted).then(|| {
        (
            checkpoint.successor().clone(),
            checkpoint.successor_record().clone(),
        )
    })));
    let head = checkpoint.successor();
    let mut anchor = ReadAnchor(Some(AntiRollbackAnchor::from(
        ControlDomainAntiRollbackAnchor::new(
            candidate.store().store_id().clone(),
            candidate.store().incarnation_id().clone(),
            head.checkpoint_domain_id().clone(),
            head.domain_epoch(),
            head.domain_sequence(),
            head.coordinate().clone(),
        ),
    )));
    let result = run(completed, &mut store, &mut anchor);
    assert_eq!(
        (store.commits, store.read_backs),
        (0, 0),
        "execution inspection is read-only"
    );
    result
}
