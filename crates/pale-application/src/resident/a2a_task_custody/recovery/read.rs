//! Bounded immutable reads, before contextual owner admission.

use super::*;

pub(super) struct History {
    pub(super) binding: (ControlRecord, A2aTaskBindingWireV1),
    pub(super) route: (ControlRecord, A2aRouteAdmissionWireV1),
    /// Selected head first; revision one last.
    pub(super) revisions: Vec<(ControlRecord, A2aTaskObservationWireV1)>,
    pub(super) checkpoint: ControlRecord,
    pub(super) selected_observation: ControlRecordCoordinate,
}

impl History {
    pub(super) fn read(
        input: &A2aTaskRecoveryInput<'_>,
        port: &mut dyn OperationCustodyPort,
    ) -> Result<Self, A2aTaskRecoveryError> {
        let (_, handle) = input.operation.handle();
        let key = binding_key(handle, input.task_binding_id)?;
        let mut budget = Bodies {
            remaining: input.limits.maximum_history_bytes,
            maximum: input.limits.maximum_record_bytes,
        };
        let binding_record = budget.index(
            port,
            input.store,
            OperationCustodyIndexQuery::A2aTaskBinding(key.clone()),
        )?;
        let binding: A2aTaskBindingWireV1 = decode(&binding_record, A2A_TASK_BINDING_V1_SCHEMA_ID)?;
        if binding.task_binding_id() != input.task_binding_id {
            return Err(A2aTaskRecoveryError::Association);
        }
        let route_coordinate = coordinate(
            binding.route_admission_ref(),
            binding.route_admission_sha256().as_str(),
        )?;
        let route_record = budget.record(port, input.store, &route_coordinate)?;
        let route = decode(&route_record, A2A_ROUTE_ADMISSION_V1_SCHEMA_ID)?;
        let first = budget.index(
            port,
            input.store,
            OperationCustodyIndexQuery::FirstA2aTaskObservation(key.clone()),
        )?;
        let mut record = budget.index(
            port,
            input.store,
            OperationCustodyIndexQuery::LatestA2aTaskObservation(key),
        )?;
        let selected_observation = match input.observation {
            A2aTaskObservationSelection::SelectedHead => record.coordinate().clone(),
            A2aTaskObservationSelection::HistoryMember { reference, digest } => {
                coordinate(reference, digest.as_str())?
            }
        };
        let checkpoint = budget.index(
            port,
            input.store,
            OperationCustodyIndexQuery::A2aTaskObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(record.coordinate()),
            ),
        )?;
        let mut revisions = Vec::new();
        let mut expected = None;
        let mut selected_found = false;
        loop {
            if revisions.len() == input.limits.maximum_revisions {
                return Err(A2aTaskRecoveryError::Bounds);
            }
            let wire: A2aTaskObservationWireV1 =
                decode(&record, A2A_TASK_OBSERVATION_V1_SCHEMA_ID)?;
            let revision = wire.revision().get();
            if expected.is_some_and(|value| value != revision) {
                return Err(A2aTaskRecoveryError::Lineage);
            }
            let previous = match (revision, wire.predecessor_ref(), wire.predecessor_sha256()) {
                (1, None, None) => None,
                (n, Some(reference), Some(digest)) if n > 1 => {
                    Some(coordinate(reference, digest.as_str())?)
                }
                _ => return Err(A2aTaskRecoveryError::Lineage),
            };
            if previous.is_none() && record != first {
                return Err(A2aTaskRecoveryError::Association);
            }
            selected_found |= record.coordinate() == &selected_observation;
            revisions.push((record, wire));
            let Some(previous) = previous else {
                break;
            };
            if revisions.len() == input.limits.maximum_revisions {
                return Err(A2aTaskRecoveryError::Bounds);
            }
            expected = Some(revision - 1);
            record = budget.record(port, input.store, &previous)?;
        }
        if !selected_found {
            return Err(A2aTaskRecoveryError::Missing);
        }
        Ok(Self {
            binding: (binding_record, binding),
            route: (route_record, route),
            revisions,
            checkpoint,
            selected_observation,
        })
    }
}

/// Each physical body read is charged, including repeated first/head reads.
struct Bodies {
    remaining: usize,
    maximum: usize,
}
impl Bodies {
    fn limit(&self) -> Result<usize, A2aTaskRecoveryError> {
        if self.remaining == 0 {
            return Err(A2aTaskRecoveryError::Bounds);
        }
        Ok(self.maximum.min(self.remaining))
    }
    fn charge(&mut self, record: ControlRecord) -> Result<ControlRecord, A2aTaskRecoveryError> {
        let length = record.canonical_record_jcs().as_slice().len();
        if length > self.maximum {
            return Err(A2aTaskRecoveryError::Bounds);
        }
        self.remaining = self
            .remaining
            .checked_sub(length)
            .ok_or(A2aTaskRecoveryError::Bounds)?;
        Ok(record)
    }
    fn index(
        &mut self,
        port: &mut dyn OperationCustodyPort,
        store: &AuthorityStoreCoordinate,
        query: OperationCustodyIndexQuery,
    ) -> Result<ControlRecord, A2aTaskRecoveryError> {
        let record =
            read_index(port, store, query, self.limit()?)?.ok_or(A2aTaskRecoveryError::Missing)?;
        self.charge(record)
    }
    fn record(
        &mut self,
        port: &mut dyn OperationCustodyPort,
        store: &AuthorityStoreCoordinate,
        selected: &ControlRecordCoordinate,
    ) -> Result<ControlRecord, A2aTaskRecoveryError> {
        let record = port
            .read_operation_record(store, selected, self.limit()?)?
            .ok_or(A2aTaskRecoveryError::Missing)?;
        if record.coordinate() != selected {
            return Err(A2aTaskRecoveryError::Association);
        }
        self.charge(record)
    }
}

pub(super) struct Evidence(pub(super) BTreeMap<ControlRecordCoordinate, OperationA2aEvidence>);
impl Evidence {
    pub(super) fn read(
        history: &History,
        input: &A2aTaskRecoveryInput<'_>,
        port: &mut dyn OperationCustodyPort,
    ) -> Result<Self, A2aTaskRecoveryError> {
        let mut requested = BTreeMap::new();
        let mut add = |reference: &ImmutableRecordReferenceV1| -> Result<(), A2aTaskRecoveryError> {
            let coord = coordinate(&reference.record_ref, reference.record_sha256.as_str())?;
            if let Some(previous) = requested.get(coord.record_ref()) {
                if previous != &coord {
                    return Err(A2aTaskRecoveryError::Association);
                }
            } else {
                if requested.len() == input.limits.maximum_evidence_records {
                    return Err(A2aTaskRecoveryError::Bounds);
                }
                requested.insert(coord.record_ref().clone(), coord);
            }
            Ok(())
        };
        add(history.binding.1.request_evidence())?;
        add(history.binding.1.initial_task_evidence())?;
        for (_, wire) in &history.revisions {
            for reference in source_references(wire.source()) {
                add(reference)?;
            }
        }
        let mut remaining = input.limits.maximum_evidence_bytes;
        let mut rows = BTreeMap::new();
        for coord in requested.into_values() {
            if remaining == 0 {
                return Err(A2aTaskRecoveryError::Bounds);
            }
            let evidence = port
                .read_a2a_evidence(input.store, &coord, remaining)?
                .ok_or(A2aTaskRecoveryError::Missing)?;
            if evidence.coordinate() != &coord {
                return Err(A2aTaskRecoveryError::Association);
            }
            remaining = remaining
                .checked_sub(evidence.byte_len())
                .ok_or(A2aTaskRecoveryError::Bounds)?;
            rows.insert(coord, evidence);
        }
        Ok(Self(rows))
    }
    pub(super) fn select(
        &self,
        refs: &[&ImmutableRecordReferenceV1],
    ) -> Result<Vec<&OperationA2aEvidence>, A2aTaskRecoveryError> {
        refs.iter()
            .map(|reference| {
                let coord = coordinate(&reference.record_ref, reference.record_sha256.as_str())?;
                self.0.get(&coord).ok_or(A2aTaskRecoveryError::Missing)
            })
            .collect()
    }
}

pub(super) fn coordinate(
    reference: &ResidentRecordRefV1,
    digest: &str,
) -> Result<ControlRecordCoordinate, A2aTaskRecoveryError> {
    record_projection::coordinate(reference, digest)
        .map_err(OperationObservationCustodyProjectionError::from)
        .map_err(A2aTaskCustodyError::from)
        .map_err(Into::into)
}

pub(super) fn decode<T: serde::de::DeserializeOwned>(
    record: &ControlRecord,
    schema: &str,
) -> Result<T, A2aTaskRecoveryError> {
    let expected = pale_contracts::record_schema_identity(schema)?;
    if record.schema_id() != &expected.schema || record.schema_sha256() != &expected.schema_sha256 {
        return Err(A2aTaskRecoveryError::Association);
    }
    Ok(serde_json::from_value(canon::parse_strict(
        record.canonical_record_jcs().as_slice(),
    )?)?)
}

pub(super) fn check_admitted(
    record: &ControlRecord,
    value: &impl serde::Serialize,
    digest: &str,
) -> Result<(), A2aTaskRecoveryError> {
    if record.coordinate().record_sha256().as_str() != digest
        || canon::to_canonical_string(value)?.as_bytes() != record.canonical_record_jcs().as_slice()
    {
        return Err(A2aTaskRecoveryError::Association);
    }
    Ok(())
}
