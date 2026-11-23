//! Existing operation checkpoint and custody phases for remote-history atoms.

use super::*;
use crate::resident::{
    checkpoint_profile::{
        self, a2a_evidence_leaf, a2a_record_leaf, A2aCustodyLeaf, OperationLeafProjectionError,
    },
    CompletedOperationObservationCustody, OperationCheckpointSelection,
    PreparedOperationObservationCustody,
};

/// Prepared exact roots and retained dependency proofs; still no custody claim.
#[must_use = "sign the exact checkpoint and complete database plus independent anchor custody"]
pub struct PreparedA2aTaskCheckpoint<'a> {
    append: PreparedA2aTaskAppend<'a>,
    selection: OperationCheckpointSelection,
    predecessor_ref: ResidentRecordRefV1,
    predecessor: ControlStateCheckpointV1,
    changed_keys: Vec<ChangedControlLeafKeyV1>,
    leaf_count: ControlLeafCountV1,
    accumulator: PreparedOperationAccumulator,
}

impl<'a> PreparedA2aTaskAppend<'a> {
    /// Prove retained dependencies and prepare new leaves under the same checkpoint.
    ///
    /// The historical local-observation proof must name this exact anchored
    /// predecessor. Retained route, binding and raw evidence membership is checked
    /// before preparation; the same checkpoint and physical transaction are fenced
    /// again at commitment. Unresolved custody remains unchanged. Neither this
    /// method nor a successful proof grants network access or public disclosure.
    /// A physically retained route receiving its first domain association instead
    /// proves that leaf absent; its exact physical provenance is checked at commit.
    ///
    /// # Errors
    /// Rejects unlike selection/proof, stale checkpoint, missing retained leaves,
    /// bound exhaustion, corrupt native state or substituted preparation output.
    pub fn prepare_checkpoint(
        self,
        selection: OperationCheckpointSelection,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &ControlStateCheckpointV1,
        limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedA2aTaskCheckpoint<'a>, A2aTaskCustodyError> {
        self.prepare_checkpoint_with_custody(
            selection,
            predecessor_ref,
            predecessor,
            None,
            limits,
            membership_limits,
            custody,
        )
    }

    /// Prepare a successor from an already completed same-process checkpoint.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_checkpoint_after_custody(
        self,
        selection: OperationCheckpointSelection,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &ControlStateCheckpointV1,
        completed: &CompletedOperationObservationCustody,
        limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedA2aTaskCheckpoint<'a>, A2aTaskCustodyError> {
        self.prepare_checkpoint_with_custody(
            selection,
            predecessor_ref,
            predecessor,
            Some(completed),
            limits,
            membership_limits,
            custody,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_checkpoint_with_custody(
        mut self,
        selection: OperationCheckpointSelection,
        predecessor_ref: &ResidentRecordRefV1,
        predecessor: &ControlStateCheckpointV1,
        completed: Option<&CompletedOperationObservationCustody>,
        limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedA2aTaskCheckpoint<'a>, A2aTaskCustodyError> {
        selection
            .verify(&self.store, self.handle, predecessor)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let predecessor_is_exact = completed.map_or_else(
            || {
                self.operation_custody.current.record == *predecessor
                    && self.operation_custody.current.reference() == predecessor_ref
            },
            |completed| {
                let candidate = completed.candidate();
                let head = candidate.checkpoint().successor();
                candidate.store() == &self.store
                    && head.coordinate().record_ref() == predecessor_ref
                    && head.coordinate().record_sha256().as_str()
                        == predecessor.canonical_control_checkpoint_sha256().as_str()
            },
        );
        if !predecessor_is_exact {
            return Err(A2aTaskCustodyError::Evidence);
        }
        let prior = OperationAccumulatorCheckpoint::new(
            self.store.clone(),
            predecessor.checkpoint_profile_ref().clone(),
            predecessor.checkpoint_profile_sha256().clone(),
            head(predecessor_ref, predecessor)?,
            predecessor.state_root_sha256().clone(),
            predecessor.unresolved_custody_root_sha256().clone(),
        );
        let key = binding_key(self.handle, self.binding.task_binding_id())?;
        let mut changes = Vec::new();
        let mut changed_keys = Vec::new();
        let mut retained = Vec::new();
        let mut inserted = 0u64;
        for record in &self.records {
            let (class, logical) = match record.kind() {
                OperationCustodyRecordKind::A2aTaskBinding => ("a2a-task-binding", key.as_str()),
                OperationCustodyRecordKind::A2aTaskObservation => {
                    ("a2a-task-observation", key.as_str())
                }
                OperationCustodyRecordKind::A2aRouteAdmission => (
                    "a2a-route-admission",
                    record.record().coordinate().record_ref().as_str(),
                ),
                _ => return Err(A2aTaskCustodyError::Evidence),
            };
            let leaf = a2a_record_leaf(
                &self.store,
                predecessor,
                class,
                logical,
                record.record().coordinate(),
            )
            .map_err(leaf_error)?;
            let old = if record.kind() == OperationCustodyRecordKind::A2aTaskObservation {
                self.predecessor
                    .map(|(reference, observation)| {
                        let coordinate = record_projection::coordinate(
                            reference,
                            observation.canonical_a2a_task_observation_sha256().as_str(),
                        )
                        .map_err(OperationObservationCustodyProjectionError::from)?;
                        a2a_record_leaf(&self.store, predecessor, class, logical, &coordinate)
                            .map(|v| v.value)
                            .map_err(leaf_error)
                    })
                    .transpose()?
            } else {
                None
            };
            if old.is_none() {
                inserted += 1;
            }
            push_change(leaf, old, &mut changes, &mut changed_keys)?;
        }
        for record in &self.dependencies.retained_records {
            let (class, logical) = match record.schema_id().as_str() {
                A2A_TASK_BINDING_V1_SCHEMA => ("a2a-task-binding", key.as_str()),
                A2A_ROUTE_ADMISSION_V1_SCHEMA => (
                    "a2a-route-admission",
                    record.coordinate().record_ref().as_str(),
                ),
                // The existing recovered operation proof owns these historical
                // parents; do not require an old dispatch to be today's head.
                OPERATION_HANDLE_V1_SCHEMA | OPERATION_OBSERVATION_V1_SCHEMA => continue,
                _ => return Err(A2aTaskCustodyError::Evidence),
            };
            let leaf = a2a_record_leaf(
                &self.store,
                predecessor,
                class,
                logical,
                record.coordinate(),
            )
            .map_err(leaf_error)?;
            retained.push(AccumulatorMembershipLeaf::new(leaf.hashed_key, leaf.value)?);
        }
        if let Some(route) = &self.dependencies.associated_route {
            let leaf = a2a_record_leaf(
                &self.store,
                predecessor,
                "a2a-route-admission",
                route.coordinate().record_ref().as_str(),
                route.coordinate(),
            )
            .map_err(leaf_error)?;
            push_change(leaf, None, &mut changes, &mut changed_keys)?;
            inserted += 1;
        }
        for evidence in &self.dependencies.new_evidence {
            let leaf = a2a_evidence_leaf(&self.store, predecessor, evidence).map_err(leaf_error)?;
            push_change(leaf, None, &mut changes, &mut changed_keys)?;
            inserted += 1;
        }
        for evidence in &self.dependencies.retained_evidence {
            let leaf = a2a_evidence_leaf(&self.store, predecessor, evidence).map_err(leaf_error)?;
            retained.push(AccumulatorMembershipLeaf::new(leaf.hashed_key, leaf.value)?);
        }
        if let Some((key, preparation)) = &self.get_task_preparation {
            let successor = self
                .records
                .iter()
                .find(|record| record.kind() == OperationCustodyRecordKind::A2aTaskObservation)
                .ok_or(A2aTaskCustodyError::Evidence)?
                .record()
                .coordinate()
                .clone();
            let leaf = a2a_record_leaf(
                &self.store,
                predecessor,
                "a2a-get-task-current",
                key.as_str(),
                &successor,
            )
            .map_err(leaf_error)?;
            let old = a2a_record_leaf(
                &self.store,
                predecessor,
                "a2a-get-task-current",
                key.as_str(),
                preparation.coordinate(),
            )
            .map_err(leaf_error)?
            .value;
            push_change(leaf, Some(old), &mut changes, &mut changed_keys)?;
            self.indexes
                .push(OperationCustodyIndexMutation::CurrentA2aGetTask {
                    key: key.clone(),
                    expected: OperationCustodyIndexPredecessor::Exact(preparation.clone()),
                    successor,
                });
        }
        if !retained.is_empty() {
            retained.sort_by(|a, b| a.key().cmp(b.key()));
            let request = OperationAccumulatorMembership::new(
                prior.clone(),
                retained,
                membership_limits,
                selection.implementation(),
            )?;
            let verified = custody
                .verify_accumulator_membership(&request)
                .map_err(OperationObservationCustodyProjectionError::from)?;
            if verified.request() != &request {
                return Err(A2aTaskCustodyError::Evidence);
            }
        }
        changes.sort_by(|a, b| a.key().cmp(b.key()));
        changed_keys.sort();
        let leaf_count = ControlLeafCountV1::new(
            predecessor
                .leaf_count()
                .get()
                .checked_add(inserted)
                .ok_or(A2aTaskCustodyError::Evidence)?,
        )?;
        let state = StateLeafChanges::new(changes, limits.maximum_leaf_changes())
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let unresolved =
            UnresolvedCustodyLeafChanges::new(Vec::new(), limits.maximum_leaf_changes())
                .map_err(OperationObservationCustodyProjectionError::from)?;
        let transaction = custody
            .prepare_transaction(&self.store)
            .map_err(OperationAccumulatorPreparationError::from)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let request = OperationAccumulatorPreparation::new(
            AccumulatorPreparationBasis::Successor(prior),
            state,
            unresolved,
            limits,
            selection.implementation(),
            transaction,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let accumulator = custody
            .prepare_accumulator(&request)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        if accumulator.request() != &request {
            return Err(A2aTaskCustodyError::Evidence);
        }
        Ok(PreparedA2aTaskCheckpoint {
            append: self,
            selection,
            predecessor_ref: predecessor_ref.clone(),
            predecessor: predecessor.clone(),
            changed_keys,
            leaf_count,
            accumulator,
        })
    }
}

impl PreparedA2aTaskCheckpoint<'_> {
    /// Proposed physical transaction, not a durable receipt.
    #[must_use]
    pub fn transaction_sequence(&self) -> ControlTransactionSequenceV1 {
        self.accumulator.request().transaction().successor()
    }
    /// Exact computed full-state root.
    #[must_use]
    pub fn state_root(&self) -> &ArtifactDigest {
        self.accumulator.state_root()
    }
    /// Unchanged unresolved-custody root.
    #[must_use]
    pub fn unresolved_root(&self) -> &ArtifactDigest {
        self.accumulator.unresolved_root()
    }
    /// Exact sorted changed logical keys.
    #[must_use]
    pub fn changed_keys(&self) -> &[ChangedControlLeafKeyV1] {
        &self.changed_keys
    }
    /// Exact retained leaf count after the selected insertions/replacement.
    #[must_use]
    pub const fn leaf_count(&self) -> ControlLeafCountV1 {
        self.leaf_count
    }

    /// Sign this prepared checkpoint and enter the existing commit/anchor phases.
    ///
    /// All root, count, profile, chronology and transaction joins precede signing.
    /// The result exposes physical custody phases, not remote or local completion.
    ///
    /// # Errors
    /// Rejects unlike checkpoint fields, authority, signing or closed atom inputs.
    pub fn issue_checkpoint<Sign>(
        self,
        successor_ref: &ResidentRecordRefV1,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<PreparedOperationObservationCustody, A2aTaskCustodyError>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.issue_checkpoint_with_value(successor_ref, preimage, schema, authority, sign)
            .map(|(physical, _)| physical)
    }

    pub(in crate::resident) fn issue_checkpoint_with_value<Sign>(
        self,
        successor_ref: &ResidentRecordRefV1,
        preimage: ControlStateCheckpointPreimageV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<
        (
            PreparedOperationObservationCustody,
            ControlStateCheckpointV1,
        ),
        A2aTaskCustodyError,
    >
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let basis = self.accumulator.basis();
        if !self.selection.matches_authority(authority)
            || authority.principal_id != self.append.handle.issuer_principal_id()
            || &preimage.control_store_id != basis.store().store_id()
            || &preimage.control_incarnation_id != basis.store().incarnation_id()
            || &preimage.checkpoint_domain_id != basis.domain()
            || preimage.domain_epoch != basis.epoch()
            || preimage.committed_transaction_sequence != self.transaction_sequence()
            || &preimage.checkpoint_profile_ref != basis.profile_ref()
            || &preimage.checkpoint_profile_sha256 != basis.profile_sha256()
            || preimage.control_schema_version != checkpoint_profile::CONTROL_SCHEMA_VERSION
            || &preimage.state_root_sha256 != self.accumulator.state_root()
            || &preimage.unresolved_custody_root_sha256 != self.accumulator.unresolved_root()
            || preimage.changed_leaf_keys != self.changed_keys
            || preimage.leaf_count != self.leaf_count
            || preimage.issued_at < *self.append.observation.observed_at()
        {
            return Err(A2aTaskCustodyError::Evidence);
        }
        let successor = ControlStateCheckpointV1::issue_successor_contextual(
            preimage,
            &self.predecessor_ref,
            &self.predecessor,
            schema,
            authority,
            sign,
        )?;
        let successor_head = head(successor_ref, &successor)?;
        let checkpoint = OperationCustodyCheckpoint::new(
            OperationCheckpointPhase::Successor {
                predecessor: head(&self.predecessor_ref, &self.predecessor)?,
                predecessor_record: Box::new(project(
                    &self.predecessor_ref,
                    self.predecessor
                        .canonical_control_checkpoint_sha256()
                        .as_str(),
                    &self.predecessor,
                    self.append.maximum_record_bytes,
                )?),
            },
            successor_head.clone(),
            project(
                successor_ref,
                successor.canonical_control_checkpoint_sha256().as_str(),
                &successor,
                self.append.maximum_record_bytes,
            )?,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        let batch = self
            .accumulator
            .bind_successor(successor_head)
            .map_err(OperationObservationCustodyProjectionError::from)?;
        let candidate = OperationCustodyCandidate::new_a2a_custody(
            self.append.store,
            self.append.kind,
            self.append.records,
            self.append.indexes,
            checkpoint,
            batch,
            self.append.dependencies,
        )
        .map_err(OperationObservationCustodyProjectionError::from)?;
        use crate::resident::operation_custody::{
            prepare_operation_observation_candidate, OperationObservationCandidatePreparationError,
        };
        let physical =
            prepare_operation_observation_candidate(candidate).map_err(|error| match error {
                OperationObservationCandidatePreparationError::NotObservation => {
                    A2aTaskCustodyError::Evidence
                }
                OperationObservationCandidatePreparationError::Anchor(error) => {
                    OperationObservationCustodyProjectionError::from(error).into()
                }
            })?;
        Ok((physical, successor))
    }
}

fn head(
    reference: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlCheckpointHead, A2aTaskCustodyError> {
    Ok(ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        record_projection::coordinate(
            reference,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )
        .map_err(OperationObservationCustodyProjectionError::from)?,
    ))
}

fn push_change(
    leaf: A2aCustodyLeaf,
    old: Option<ArtifactDigest>,
    changes: &mut Vec<AccumulatorLeafChange>,
    keys: &mut Vec<ChangedControlLeafKeyV1>,
) -> Result<(), A2aTaskCustodyError> {
    changes.push(
        AccumulatorLeafChange::new(leaf.hashed_key, old, Some(leaf.value))
            .map_err(OperationObservationCustodyProjectionError::from)?,
    );
    keys.push(leaf.key);
    Ok(())
}
fn leaf_error(error: OperationLeafProjectionError) -> A2aTaskCustodyError {
    match error {
        OperationLeafProjectionError::Store => {
            OperationObservationCustodyProjectionError::StoreScope
        }
        OperationLeafProjectionError::Identity(error) => error.into(),
        OperationLeafProjectionError::Canonicalize(error) => error.into(),
    }
    .into()
}
