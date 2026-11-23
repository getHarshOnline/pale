//! Closed remote-history atoms and exact protocol-byte custody. No transport runs here.

use super::*;
use pale_core::records::{
    ArtifactDigest, ImmutableRecordReferenceV1, A2A_ROUTE_ADMISSION_V1_SCHEMA,
    A2A_TASK_BINDING_V1_SCHEMA, A2A_TASK_OBSERVATION_V1_SCHEMA,
};

/// Bounded immutable native evidence, separate from signed-record JCS.
/// Its coordinate hashes only the exact bytes. SSE metadata is independently
/// authenticated by the application checkpoint leaf and compared by custody.
#[derive(Clone, PartialEq, Eq)]
pub struct OperationA2aEvidence {
    coordinate: ControlRecordCoordinate,
    bytes: crate::ControlRecordBytes,
    sse: Option<(String, String)>,
}

impl std::fmt::Debug for OperationA2aEvidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationA2aEvidence")
            .field("coordinate", &self.coordinate)
            .field("bytes", &self.bytes)
            .finish_non_exhaustive()
    }
}

impl OperationA2aEvidence {
    /// Check the exact-byte digest and aggregate bytes before retaining input.
    /// SSE fields are `(event_id, event_type)`, not a raw HTTP/TLS transcript.
    ///
    /// # Errors
    /// Rejects empty, oversized, overflowed or digest-mismatched evidence.
    pub fn new(
        reference: ImmutableRecordReferenceV1,
        bytes: Vec<u8>,
        sse: Option<(String, String)>,
        maximum_bytes: usize,
    ) -> Result<Self, OperationCustodyCandidateError> {
        let length = sse.as_ref().map_or(Some(bytes.len()), |(id, kind)| {
            bytes
                .len()
                .checked_add(id.len())
                .and_then(|n| n.checked_add(kind.len()))
        });
        if length.is_none_or(|n| n == 0 || n > maximum_bytes)
            || ArtifactDigest::hash_bytes(&bytes) != reference.record_sha256
        {
            return Err(OperationCustodyCandidateError::A2aEvidence);
        }
        Ok(Self {
            coordinate: ControlRecordCoordinate::new(
                reference.record_ref,
                crate::ControlRecordDigest::parse(reference.record_sha256.as_str())
                    .map_err(|_| OperationCustodyCandidateError::A2aEvidence)?,
            ),
            bytes: crate::ControlRecordBytes::new(bytes)
                .map_err(|_| OperationCustodyCandidateError::A2aEvidence)?,
            sse,
        })
    }
    /// Exact immutable reference and raw-byte digest.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }
    /// Exact protocol bytes, never recanonicalized.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
    /// Optional inherited/reset SSE ID and event type.
    #[must_use]
    pub fn sse(&self) -> Option<(&str, &str)> {
        self.sse
            .as_ref()
            .map(|(id, kind)| (id.as_str(), kind.as_str()))
    }
    /// Aggregate admitted bytes including retained SSE metadata.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.bytes.as_slice().len()
            + self
                .sse
                .as_ref()
                .map_or(0, |(id, kind)| id.len() + kind.len())
    }
}

/// Explicit append-versus-retained dependencies for one remote-history atom.
/// Retained rows are compared, never silently inserted or rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationA2aDependencies {
    /// Raw rows absent before, and inserted with, the new signed atom.
    pub new_evidence: Vec<OperationA2aEvidence>,
    /// Exact raw rows authenticated under its predecessor checkpoint.
    pub retained_evidence: Vec<OperationA2aEvidence>,
    /// Exact signed parents which already have physical custody.
    pub retained_records: Vec<ControlRecord>,
    /// Exact physically retained route receiving its first association in this
    /// checkpoint domain. It must predate the physical transaction, but need not
    /// predate this domain's checkpoint. It is neither appended nor upserted.
    pub associated_route: Option<ControlRecord>,
}

impl OperationCustodyCandidate {
    /// Close one A2A send, binding, or observation atom with explicit evidence.
    ///
    /// `dependencies` contains new raw evidence, exact retained raw evidence,
    /// and exact retained signed parents. New rows must be absent and co-commit;
    /// retained rows must match bytes/metadata and predate the predecessor
    /// checkpoint. The application separately proves their checkpoint membership.
    /// A separately associated route instead predates the physical transaction;
    /// the application proves absence before inserting its first domain leaf.
    /// This constructor supplies no authority, evidence admission or send proof.
    ///
    /// # Errors
    /// Rejects a non-A2A transition, invalid record/index shape, aliased references,
    /// missing parents/evidence, or an unlike accumulator/checkpoint successor.
    pub fn new_a2a_custody(
        store: AuthorityStoreCoordinate,
        kind: OperationCustodyTransitionKind,
        records: Vec<OperationCustodyRecord>,
        mut indexes: Vec<OperationCustodyIndexMutation>,
        checkpoint: OperationCustodyCheckpoint,
        accumulator: OperationAccumulatorBatch,
        dependencies: OperationA2aDependencies,
    ) -> Result<Self, OperationCustodyCandidateError> {
        let OperationA2aDependencies {
            new_evidence: evidence,
            retained_evidence,
            retained_records: retained,
            associated_route,
        } = dependencies;
        let prior = match checkpoint.phase() {
            OperationCheckpointPhase::Successor { predecessor, .. } => predecessor,
            OperationCheckpointPhase::Initial => {
                return Err(OperationCustodyCandidateError::AccumulatorBinding);
            }
        };
        if accumulator.binding().store() != &store
            || accumulator.binding().successor() != checkpoint.successor()
            || !matches!(accumulator.binding().phase(), crate::AccumulatorCheckpointPhase::Successor(actual) if actual == prior)
        {
            return Err(OperationCustodyCandidateError::AccumulatorBinding);
        }
        let mut expected = match kind {
            OperationCustodyTransitionKind::PrepareA2aSend => {
                [OperationCustodyRecordKind::A2aSendPreparation]
                    .into_iter()
                    .collect::<BTreeSet<_>>()
            }
            OperationCustodyTransitionKind::PrepareA2aGetTask => {
                [OperationCustodyRecordKind::A2aGetTaskPreparation]
                    .into_iter()
                    .collect()
            }
            OperationCustodyTransitionKind::DisposeA2aGetTask => {
                [OperationCustodyRecordKind::A2aGetTaskLocalDisposition]
                    .into_iter()
                    .collect()
            }
            OperationCustodyTransitionKind::BindA2aTask => [
                OperationCustodyRecordKind::A2aTaskBinding,
                OperationCustodyRecordKind::A2aTaskObservation,
            ]
            .into_iter()
            .collect::<BTreeSet<_>>(),
            OperationCustodyTransitionKind::ObserveA2aTask => {
                [OperationCustodyRecordKind::A2aTaskObservation]
                    .into_iter()
                    .collect()
            }
            _ => return Err(OperationCustodyCandidateError::RecordShape),
        };
        if matches!(
            kind,
            OperationCustodyTransitionKind::PrepareA2aSend
                | OperationCustodyTransitionKind::BindA2aTask
        ) && records
            .iter()
            .any(|r| r.kind() == OperationCustodyRecordKind::A2aRouteAdmission)
        {
            expected.insert(OperationCustodyRecordKind::A2aRouteAdmission);
        }
        let actual = records
            .iter()
            .map(OperationCustodyRecord::kind)
            .collect::<BTreeSet<_>>();
        if actual != expected
            || actual.len() != records.len()
            || records
                .iter()
                .any(|r| r.record().schema_id().as_str() != r.kind().schema_id())
        {
            return Err(OperationCustodyCandidateError::RecordShape);
        }
        validate_indexes(kind, &records, &indexes, &retained)?;
        super::validate_record_references(&records, &indexes, &checkpoint)?;
        if associated_route.as_ref().is_some_and(|route| {
            !matches!(
                kind,
                OperationCustodyTransitionKind::PrepareA2aSend
                    | OperationCustodyTransitionKind::BindA2aTask
            ) || route.schema_id().as_str() != A2A_ROUTE_ADMISSION_V1_SCHEMA
                || actual.contains(&OperationCustodyRecordKind::A2aRouteAdmission)
        }) {
            return Err(OperationCustodyCandidateError::A2aEvidence);
        }
        let mut refs: BTreeSet<_> = records
            .iter()
            .map(|r| r.record().coordinate().record_ref())
            .chain([checkpoint.successor_record().coordinate().record_ref()])
            .chain(
                checkpoint
                    .predecessor_record()
                    .map(|record| record.coordinate().record_ref()),
            )
            .chain(indexes.iter().filter_map(|index| match index.expected() {
                OperationCustodyIndexPredecessor::Exact(record) => {
                    Some(record.coordinate().record_ref())
                }
                OperationCustodyIndexPredecessor::Absent => None,
            }))
            .collect();
        for record in retained.iter().chain(associated_route.iter()) {
            let unchanged_binding = indexes.iter().any(|index| matches!(index, OperationCustodyIndexMutation::A2aTaskBinding { expected:OperationCustodyIndexPredecessor::Exact(binding), .. } if binding == record));
            let terminal_get_task_head = kind == OperationCustodyTransitionKind::PrepareA2aGetTask
                && indexes.iter().any(|index| {
                    matches!(
                        index,
                        OperationCustodyIndexMutation::CurrentA2aGetTask {
                            expected: OperationCustodyIndexPredecessor::Exact(head),
                            ..
                        } if head == record
                            && record.schema_id().as_str() == A2A_TASK_OBSERVATION_V1_SCHEMA
                    )
                });
            if !refs.insert(record.coordinate().record_ref())
                && !unchanged_binding
                && !terminal_get_task_head
            {
                return Err(OperationCustodyCandidateError::DuplicateRecordReference);
            }
        }
        let evidence_count = evidence.len() + retained_evidence.len();
        if (matches!(
            kind,
            OperationCustodyTransitionKind::PrepareA2aSend
                | OperationCustodyTransitionKind::PrepareA2aGetTask
        ) && (evidence.len() != 1 || !retained_evidence.is_empty()))
            || (kind == OperationCustodyTransitionKind::DisposeA2aGetTask
                && (!evidence.is_empty() || !retained_evidence.is_empty()))
            || (!matches!(
                kind,
                OperationCustodyTransitionKind::PrepareA2aSend
                    | OperationCustodyTransitionKind::PrepareA2aGetTask
                    | OperationCustodyTransitionKind::DisposeA2aGetTask
            ) && !(1..=2).contains(&evidence_count))
        {
            return Err(OperationCustodyCandidateError::A2aEvidence);
        }
        for value in evidence.iter().chain(&retained_evidence) {
            if !refs.insert(value.coordinate().record_ref()) {
                return Err(OperationCustodyCandidateError::DuplicateRecordReference);
            }
        }
        let schemas: BTreeSet<_> = retained.iter().map(|r| r.schema_id().as_str()).collect();
        let mut parent_schemas = [OPERATION_HANDLE_V1_SCHEMA, OPERATION_OBSERVATION_V1_SCHEMA]
            .into_iter()
            .collect::<BTreeSet<_>>();
        if !actual.contains(&OperationCustodyRecordKind::A2aRouteAdmission)
            && associated_route.is_none()
        {
            parent_schemas.insert(A2A_ROUTE_ADMISSION_V1_SCHEMA);
        }
        if kind == OperationCustodyTransitionKind::ObserveA2aTask {
            parent_schemas.insert(A2A_TASK_BINDING_V1_SCHEMA);
        }
        if kind == OperationCustodyTransitionKind::PrepareA2aGetTask {
            parent_schemas.extend([
                A2A_TASK_BINDING_V1_SCHEMA,
                A2A_TASK_OBSERVATION_V1_SCHEMA,
                A2A_ROUTE_ADMISSION_V1_SCHEMA,
            ]);
        }
        if kind == OperationCustodyTransitionKind::DisposeA2aGetTask {
            parent_schemas.clear();
        }
        let duplicated_local_head = kind == OperationCustodyTransitionKind::PrepareA2aGetTask
            && retained.len() == schemas.len() + 1
            && retained
                .iter()
                .filter(|record| record.schema_id().as_str() == OPERATION_OBSERVATION_V1_SCHEMA)
                .count()
                == 2;
        if schemas != parent_schemas || (schemas.len() != retained.len() && !duplicated_local_head)
        {
            return Err(OperationCustodyCandidateError::A2aEvidence);
        }
        if let Some(OperationCustodyIndexMutation::A2aTaskBinding {
            expected: OperationCustodyIndexPredecessor::Exact(binding),
            ..
        }) = indexes
            .iter()
            .find(|i| i.role() == IndexRole::A2aTaskBinding)
        {
            if !retained.contains(binding) {
                return Err(OperationCustodyCandidateError::A2aEvidence);
            }
        }
        if kind == OperationCustodyTransitionKind::PrepareA2aSend {
            let preparation = records
                .iter()
                .find(|r| r.kind() == OperationCustodyRecordKind::A2aSendPreparation)
                .ok_or(OperationCustodyCandidateError::RecordShape)?
                .record()
                .coordinate();
            indexes.push(
                OperationCustodyIndexMutation::A2aSendPreparationCheckpoint {
                    key: OperationCustodyIndexKey::for_a2a_send_preparation_checkpoint(preparation),
                    successor: checkpoint.successor_record().coordinate().clone(),
                },
            );
        } else if matches!(
            kind,
            OperationCustodyTransitionKind::PrepareA2aGetTask
                | OperationCustodyTransitionKind::DisposeA2aGetTask
        ) {
            let private_record = records
                .first()
                .ok_or(OperationCustodyCandidateError::RecordShape)?
                .record()
                .coordinate();
            indexes.push(OperationCustodyIndexMutation::A2aGetTaskCheckpoint {
                key: OperationCustodyIndexKey::for_a2a_get_task_checkpoint(private_record),
                successor: checkpoint.successor_record().coordinate().clone(),
            });
        } else {
            let observation = records
                .iter()
                .find(|r| r.kind() == OperationCustodyRecordKind::A2aTaskObservation)
                .ok_or(OperationCustodyCandidateError::RecordShape)?
                .record()
                .coordinate();
            let key = indexes
                .iter()
                .find(|i| i.role() == IndexRole::A2aTaskBinding)
                .ok_or(OperationCustodyCandidateError::IndexShape)?
                .key()
                .clone();
            if kind == OperationCustodyTransitionKind::BindA2aTask {
                indexes.push(OperationCustodyIndexMutation::FirstA2aTaskObservation {
                    key,
                    successor: observation.clone(),
                });
            }
            indexes.push(
                OperationCustodyIndexMutation::A2aTaskObservationCheckpoint {
                    key: OperationCustodyIndexKey::for_observation(observation),
                    successor: checkpoint.successor_record().coordinate().clone(),
                },
            );
        }
        Ok(Self {
            store,
            kind,
            records: records.into_boxed_slice(),
            indexes: indexes.into_boxed_slice(),
            checkpoint: Box::new(checkpoint),
            accumulator: Box::new(accumulator),
            collection_journal: None,
            session_basis: None,
            a2a_evidence: evidence.into_boxed_slice(),
            a2a_retained_evidence: retained_evidence.into_boxed_slice(),
            a2a_retained_records: retained.into_boxed_slice(),
            a2a_associated_route: associated_route.map(Box::new),
        })
    }
    /// Raw rows which must be absent and inserted in this exact transaction.
    #[must_use]
    pub fn a2a_evidence(&self) -> &[OperationA2aEvidence] {
        &self.a2a_evidence
    }
    /// Exact raw dependencies already authenticated under the predecessor.
    #[must_use]
    pub fn a2a_retained_evidence(&self) -> &[OperationA2aEvidence] {
        &self.a2a_retained_evidence
    }
    /// Exact immutable parents which must already be retained before this atom.
    #[must_use]
    pub fn a2a_retained_records(&self) -> &[ControlRecord] {
        &self.a2a_retained_records
    }

    /// Exact existing route newly associated with this domain, not inserted again.
    #[must_use]
    pub fn a2a_associated_route(&self) -> Option<&ControlRecord> {
        self.a2a_associated_route.as_deref()
    }
}

fn validate_indexes(
    kind: OperationCustodyTransitionKind,
    records: &[OperationCustodyRecord],
    indexes: &[OperationCustodyIndexMutation],
    retained: &[ControlRecord],
) -> Result<(), OperationCustodyCandidateError> {
    if kind == OperationCustodyTransitionKind::PrepareA2aSend {
        if indexes.len() != 1 {
            return Err(OperationCustodyCandidateError::IndexShape);
        }
        let Some(preparation) = indexes
            .iter()
            .find(|index| index.role() == IndexRole::A2aSendPreparation)
        else {
            return Err(OperationCustodyCandidateError::IndexShape);
        };
        if !matches!(
            preparation.expected(),
            OperationCustodyIndexPredecessor::Absent
        ) || !records.iter().any(|record| {
            record.kind() == OperationCustodyRecordKind::A2aSendPreparation
                && record.record().coordinate() == preparation.successor()
        }) || retained
            .iter()
            .find(|record| record.schema_id().as_str() == OPERATION_OBSERVATION_V1_SCHEMA)
            .is_none_or(|dispatch| {
                preparation.key()
                    != &OperationCustodyIndexKey::for_a2a_send_preparation(dispatch.coordinate())
            })
        {
            return Err(OperationCustodyCandidateError::IndexPredecessor);
        }
        return Ok(());
    }
    if matches!(
        kind,
        OperationCustodyTransitionKind::PrepareA2aGetTask
            | OperationCustodyTransitionKind::DisposeA2aGetTask
    ) {
        if indexes.len() != 1 {
            return Err(OperationCustodyCandidateError::IndexShape);
        }
        let current = indexes
            .iter()
            .find(|index| index.role() == IndexRole::CurrentA2aGetTask)
            .ok_or(OperationCustodyCandidateError::IndexShape)?;
        let expected_valid = match (kind, current.expected()) {
            (
                OperationCustodyTransitionKind::PrepareA2aGetTask,
                OperationCustodyIndexPredecessor::Absent,
            ) => true,
            (
                OperationCustodyTransitionKind::PrepareA2aGetTask,
                OperationCustodyIndexPredecessor::Exact(record),
            ) => matches!(
                record.schema_id().as_str(),
                PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA
                    | A2A_TASK_OBSERVATION_V1_SCHEMA
            ),
            (
                OperationCustodyTransitionKind::DisposeA2aGetTask,
                OperationCustodyIndexPredecessor::Exact(record),
            ) => {
                record.schema_id().as_str() == PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA
            }
            _ => false,
        };
        let successor_kind = if kind == OperationCustodyTransitionKind::PrepareA2aGetTask {
            OperationCustodyRecordKind::A2aGetTaskPreparation
        } else {
            OperationCustodyRecordKind::A2aGetTaskLocalDisposition
        };
        if !expected_valid
            || !records.iter().any(|record| {
                record.kind() == successor_kind
                    && record.record().coordinate() == current.successor()
            })
        {
            return Err(OperationCustodyCandidateError::IndexPredecessor);
        }
        return Ok(());
    }
    let current_get_task = indexes
        .iter()
        .find(|index| index.role() == IndexRole::CurrentA2aGetTask);
    if indexes.len() != 2 + usize::from(current_get_task.is_some()) {
        return Err(OperationCustodyCandidateError::IndexShape);
    }
    let binding = indexes
        .iter()
        .find(|i| i.role() == IndexRole::A2aTaskBinding)
        .ok_or(OperationCustodyCandidateError::IndexShape)?;
    let observation = indexes
        .iter()
        .find(|i| i.role() == IndexRole::LatestA2aTaskObservation)
        .ok_or(OperationCustodyCandidateError::IndexShape)?;
    if binding.key() != observation.key() {
        return Err(OperationCustodyCandidateError::IndexKeyPair);
    }
    if let Some(current) = current_get_task {
        if kind != OperationCustodyTransitionKind::ObserveA2aTask
            || current.key() != binding.key()
            || !matches!(current.expected(), OperationCustodyIndexPredecessor::Exact(record)
                if record.schema_id().as_str() == PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA)
            || current.successor() != observation.successor()
        {
            return Err(OperationCustodyCandidateError::IndexPredecessor);
        }
    }
    let expected_schema =
        |index: &OperationCustodyIndexMutation, schema: &str, unchanged: bool| match index
            .expected()
        {
            OperationCustodyIndexPredecessor::Absent => {
                kind == OperationCustodyTransitionKind::BindA2aTask
            }
            OperationCustodyIndexPredecessor::Exact(record) => {
                kind == OperationCustodyTransitionKind::ObserveA2aTask
                    && record.schema_id().as_str() == schema
                    && (record.coordinate() == index.successor()) == unchanged
            }
        };
    if !expected_schema(binding, A2A_TASK_BINDING_V1_SCHEMA, true)
        || !expected_schema(observation, A2A_TASK_OBSERVATION_V1_SCHEMA, false)
    {
        return Err(OperationCustodyCandidateError::IndexPredecessor);
    }
    for (index, role) in [
        (binding, OperationCustodyRecordKind::A2aTaskBinding),
        (observation, OperationCustodyRecordKind::A2aTaskObservation),
    ] {
        if role == OperationCustodyRecordKind::A2aTaskBinding
            && kind == OperationCustodyTransitionKind::ObserveA2aTask
        {
            continue;
        }
        if !records
            .iter()
            .any(|r| r.kind() == role && r.record().coordinate() == index.successor())
        {
            return Err(OperationCustodyCandidateError::IndexSuccessor);
        }
    }
    Ok(())
}
