//! Shared checkpoint custody for operation-service observation phases.

use super::*;
use crate::resident::{
    CompletedOperationObservationCustody, OperationObservationCustodyAnchorFailure,
    OperationObservationCustodyCommitFailure,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, OperationCustodyPort, WallClockSourcePort,
};

/// Independently allocated identity of one new operation checkpoint.
pub struct OperationCheckpointInput {
    /// Fresh immutable record reference.
    pub reference: ResidentRecordRefV1,
    /// Fresh checkpoint identity, distinct from its reference.
    pub id: ControlCheckpointIdV1,
}

/// A stopped operation append retaining the existing physical retry rules.
#[derive(Debug)]
pub enum OperationAppendFailure {
    /// Clock, signing or existing operation admission failed before an append.
    Operation(OperationAttemptError),
    /// Root preparation, checkpoint issuance or projection failed before commitment.
    Projection(OperationObservationCustodyProjectionError),
    /// The database outcome is classified by its existing custody owner.
    Commit {
        /// Exact already-issued checkpoint; not proof of commitment.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Owns the candidate; only its existing checked extraction permits retry.
        failure: OperationObservationCustodyCommitFailure,
    },
    /// Database custody exists, but the independent anchor is not established.
    Anchor {
        /// Exact already-issued checkpoint; not proof of completed anchoring.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
        /// Owns the pending phase and its original uncertainty classification.
        failure: OperationObservationCustodyAnchorFailure,
    },
}

/// One signed successor stopped while entering exact checkpoint custody.
#[must_use = "continue this exact custody phase or preserve it"]
pub(in crate::resident) struct OwnedOperationObservationCustodyFailure {
    preparation: Option<Box<OwnedObservationCheckpointPreparation>>,
    failure: OperationAppendFailure,
}

struct OwnedObservationCheckpointPreparation {
    transition: super::PreparedOwnedOperationObservation,
    prior_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    checkpoint: OperationCheckpointInput,
}

/// Retain one detached signed successor through checkpoint, database, and anchor custody.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn retain_owned_operation_observation<Sign>(
    transition: super::PreparedOwnedOperationObservation,
    prior_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    checkpoint: OperationCheckpointInput,
    now: UnixTimeV1,
    schema: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    limits: AccumulatorPreparationLimits,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign: Sign,
) -> Result<CompletedOperationObservationCustody, OwnedOperationObservationCustodyFailure>
where
    Sign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let append = match transition.prepare_append(resolve_store) {
        Ok(append) => append,
        Err(reason) => {
            return Err(OwnedOperationObservationCustodyFailure {
                preparation: Some(Box::new(OwnedObservationCheckpointPreparation {
                    transition,
                    prior_checkpoint,
                    checkpoint,
                })),
                failure: OperationAppendFailure::Projection(reason),
            });
        }
    };
    let append_checkpoint_input = OperationCheckpointInput {
        reference: checkpoint.reference.clone(),
        id: checkpoint.id.clone(),
    };
    match append_checkpoint(
        append,
        transition.operation(),
        (&prior_checkpoint.0, &prior_checkpoint.1),
        append_checkpoint_input,
        now,
        schema,
        authority,
        limits,
        custody,
        anchor,
        sign,
    ) {
        Ok((completed, _)) => Ok(completed),
        Err(
            failure @ (OperationAppendFailure::Commit { .. }
            | OperationAppendFailure::Anchor { .. }),
        ) => Err(OwnedOperationObservationCustodyFailure {
            preparation: None,
            failure,
        }),
        Err(failure) => Err(OwnedOperationObservationCustodyFailure {
            preparation: Some(Box::new(OwnedObservationCheckpointPreparation {
                transition,
                prior_checkpoint,
                checkpoint,
            })),
            failure,
        }),
    }
}

/// Continue one exact detached successor without re-signing an issued checkpoint.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn continue_owned_operation_observation_custody<Sign>(
    pending: OwnedOperationObservationCustodyFailure,
    now: UnixTimeV1,
    schema: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    limits: AccumulatorPreparationLimits,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign: Sign,
) -> Result<CompletedOperationObservationCustody, OwnedOperationObservationCustodyFailure>
where
    Sign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let OwnedOperationObservationCustodyFailure {
        preparation,
        failure,
    } = pending;
    match failure {
        failure @ (OperationAppendFailure::Commit { .. }
        | OperationAppendFailure::Anchor { .. }) => failure
            .continue_custody(custody, anchor)
            .map(|(completed, _)| completed)
            .map_err(|failure| OwnedOperationObservationCustodyFailure {
                preparation: None,
                failure,
            }),
        _ => {
            let preparation = preparation.expect("pre-checkpoint failure retains its inputs");
            retain_owned_operation_observation(
                preparation.transition,
                preparation.prior_checkpoint,
                preparation.checkpoint,
                now,
                schema,
                authority,
                resolve_store,
                limits,
                custody,
                anchor,
                sign,
            )
        }
    }
}

impl From<OperationAttemptError> for OperationAppendFailure {
    fn from(value: OperationAttemptError) -> Self {
        Self::Operation(value)
    }
}

impl From<OperationObservationCustodyProjectionError> for OperationAppendFailure {
    fn from(value: OperationObservationCustodyProjectionError) -> Self {
        Self::Projection(value)
    }
}

impl OperationAppendFailure {
    /// Continue only an already-issued database or anchor custody phase.
    ///
    /// This method never rebuilds or re-signs the observation or checkpoint.
    /// It returns the unchanged phase when exact read-back cannot authorize the
    /// next physical step.
    pub(in crate::resident) fn continue_custody(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<
        (
            CompletedOperationObservationCustody,
            (ResidentRecordRefV1, ControlStateCheckpointV1),
        ),
        Self,
    > {
        let (checkpoint, pending) = match self {
            Self::Commit {
                checkpoint,
                failure,
            } => {
                let pending = match failure.reconcile_exact(custody) {
                    Ok(pending) => pending,
                    Err(failure) => match failure.try_into_retryable() {
                        Ok(prepared) => match prepared.commit(custody, anchor) {
                            Ok(pending) => pending,
                            Err(failure) => {
                                return Err(Self::Commit {
                                    checkpoint,
                                    failure,
                                });
                            }
                        },
                        Err(failure) => {
                            return Err(Self::Commit {
                                checkpoint,
                                failure,
                            });
                        }
                    },
                };
                (checkpoint, pending)
            }
            Self::Anchor {
                checkpoint,
                failure,
            } => match failure.continue_after_failed_observation(anchor) {
                Ok(completed) => return Ok((completed, *checkpoint)),
                Err(failure) => match failure.try_into_retryable() {
                    Ok(pending) => (checkpoint, pending),
                    Err(failure) => {
                        return Err(Self::Anchor {
                            checkpoint,
                            failure,
                        });
                    }
                },
            },
            Self::Operation(_) | Self::Projection(_) => return Err(self),
        };
        match pending.advance_anchor(anchor) {
            Ok(completed) => Ok((completed, *checkpoint)),
            Err(failure) => Err(Self::Anchor {
                checkpoint,
                failure,
            }),
        }
    }
}

pub(crate) fn sample_time(
    clock: &dyn WallClockSourcePort,
    floor: UnixTimeV1,
) -> Result<UnixTimeV1, OperationAttemptError> {
    let now = clock.observe_unix_time()?;
    if now < floor {
        return Err(OperationAttemptError::TimeOrder);
    }
    Ok(now)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_checkpoint<Sign>(
    append: PreparedOperationObservationAppend<'_>,
    operation: &IssuedLocalOperation,
    prior: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    identity: OperationCheckpointInput,
    now: UnixTimeV1,
    schema: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    limits: AccumulatorPreparationLimits,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign: Sign,
) -> Result<
    (
        CompletedOperationObservationCustody,
        (ResidentRecordRefV1, ControlStateCheckpointV1),
    ),
    OperationAppendFailure,
>
where
    Sign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    append_checkpoint_for_selection(
        append,
        operation.checkpoint_selection(),
        prior,
        identity,
        now,
        schema,
        authority,
        limits,
        custody,
        anchor,
        sign,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn append_checkpoint_for_selection<Sign>(
    append: PreparedOperationObservationAppend<'_>,
    selection: crate::resident::checkpoint_profile::OperationCheckpointSelection,
    prior: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    identity: OperationCheckpointInput,
    now: UnixTimeV1,
    schema: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    limits: AccumulatorPreparationLimits,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign: Sign,
) -> Result<
    (
        CompletedOperationObservationCustody,
        (ResidentRecordRefV1, ControlStateCheckpointV1),
    ),
    OperationAppendFailure,
>
where
    Sign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let prepared = append.prepare_checkpoint(selection, prior.0, prior.1, limits, custody)?;
    complete_prepared_checkpoint(
        prepared, prior, identity, now, schema, authority, custody, anchor, sign,
    )
}

/// Issue, commit, and independently anchor an already prepared observation checkpoint.
///
/// Preparation remains with the semantic owner that assembled the exact append.
/// This helper only preserves the shared physical ordering: checkpoint issuance,
/// database commitment, then independent anchor advancement.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn complete_prepared_checkpoint<Sign>(
    prepared: crate::resident::PreparedOperationObservationCheckpoint<'_>,
    prior: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    identity: OperationCheckpointInput,
    now: UnixTimeV1,
    schema: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign: Sign,
) -> Result<
    (
        CompletedOperationObservationCustody,
        (ResidentRecordRefV1, ControlStateCheckpointV1),
    ),
    OperationAppendFailure,
>
where
    Sign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let preimage =
        observation_checkpoint_preimage(&prepared, prior, &identity, now, schema, authority)?;
    let (physical, checkpoint) =
        prepared.issue_checkpoint(&identity.reference, preimage, schema, authority, sign)?;
    let checkpoint = (identity.reference, checkpoint);
    commit_prepared_checkpoint(physical, checkpoint, custody, anchor)
}

/// Commit and anchor one already prepared custody/checkpoint pair.
///
/// Semantic preparation, checkpoint construction, and signing happen before
/// this mechanical tail. It cannot rebuild or re-sign either value.
pub(in crate::resident) fn commit_prepared_checkpoint(
    physical: crate::resident::PreparedOperationObservationCustody,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<
    (
        CompletedOperationObservationCustody,
        (ResidentRecordRefV1, ControlStateCheckpointV1),
    ),
    OperationAppendFailure,
> {
    let pending = match physical.commit(custody, anchor) {
        Ok(value) => value,
        Err(failure) => {
            return Err(OperationAppendFailure::Commit {
                checkpoint: Box::new(checkpoint),
                failure,
            });
        }
    };
    match pending.advance_anchor(anchor) {
        Ok(completed) => Ok((completed, checkpoint)),
        Err(failure) => Err(OperationAppendFailure::Anchor {
            checkpoint: Box::new(checkpoint),
            failure,
        }),
    }
}

pub(in crate::resident) fn observation_checkpoint_preimage(
    prepared: &crate::resident::PreparedOperationObservationCheckpoint<'_>,
    prior: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    identity: &OperationCheckpointInput,
    now: UnixTimeV1,
    schema: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) -> Result<ControlStateCheckpointPreimageV1, OperationAttemptError> {
    let sequence = prior
        .1
        .domain_sequence()
        .get()
        .checked_add(1)
        .ok_or(OperationAttemptError::Association)?;
    Ok(ControlStateCheckpointPreimageV1 {
        schema_id: prior.1.schema_id().clone(),
        schema_sha256: schema.clone(),
        control_checkpoint_id: identity.id.clone(),
        control_store_id: authority.control_store_id.clone(),
        control_incarnation_id: authority.control_incarnation_id.clone(),
        checkpoint_domain_id: authority.checkpoint_domain_id.clone(),
        domain_epoch: *prior.1.domain_epoch(),
        domain_sequence: ControlDomainSequenceV1::new(sequence)?,
        previous_control_checkpoint_ref: Some(prior.0.clone()),
        previous_control_checkpoint_sha256: Some(
            prior.1.canonical_control_checkpoint_sha256().clone(),
        ),
        checkpoint_profile_ref: authority.checkpoint_profile_ref.clone(),
        checkpoint_profile_sha256: authority.checkpoint_profile_sha256.clone(),
        control_schema_version: *prior.1.control_schema_version(),
        committed_transaction_sequence: prepared.transaction_sequence(),
        state_root_sha256: prepared.state_root().clone(),
        leaf_count: prepared.leaf_count(),
        changed_leaf_keys: prepared.changed_keys().to_vec(),
        unresolved_custody_root_sha256: prepared.unresolved_root().clone(),
        issued_at: now,
        checkpoint_signing_authority_ref: authority.authority_ref.clone(),
        checkpoint_signing_authority_sha256: authority.authority_sha256.clone(),
        issuer_principal_id: authority.principal_id.clone(),
        issuer_key_id: authority.subject_key_id.clone(),
        issuer_key_epoch: authority.subject_key_epoch,
    })
}
