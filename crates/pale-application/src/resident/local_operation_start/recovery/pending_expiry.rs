//! Repair a committed expiry or initial collection in a root's checkpoint domain.

use super::*;
use crate::resident::{
    operation_observation_custody::read_history, read_operation_observation_for_handle,
    recover_committed_consultation_expiry, recover_committed_pre_dispatch_expiry,
    CommittedObservationCheckpointRecovery, ConsultationCollection, ConsultationCollectionLimits,
    HistoricalConsultationAdmission, OperationControlRecoveryAdmission,
    OperationObservationTransition, OperationStartRecoveryEvidence,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchor, ControlDomainAntiRollbackAnchor,
    OperationAccumulatorCheckpoint, OperationAccumulatorMembership,
};

/// One cold-start repair stopped without granting a usable operation.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum PendingRootObservationError {
    #[error(transparent)]
    Identity(#[from] LocalOperationRecoveryError),
    #[error(transparent)]
    History(#[from] OperationObservationReadmissionError),
    #[error(transparent)]
    Projection(#[from] crate::resident::OperationObservationCustodyProjectionError),
    #[error(transparent)]
    Diagnostic(#[from] crate::diagnostic::DiagnosticAdmissionError),
    #[error(transparent)]
    Collection(#[from] crate::resident::ConsultationCollectionError),
    #[error(transparent)]
    Anchor(#[from] Box<crate::resident::OperationObservationCustodyAnchorFailure>),
}

/// Repair expiry or initial collection custody in a root's checkpoint domain.
/// Consult collection repair requires explicit historical collection admission.
/// All authority callbacks retain their historical admission contracts. This
/// fixed two-checkpoint proof and bounded observation walk never scan the store,
/// sign, rewrite custody, or release a response. Parent depth is explicitly
/// bounded; other mutations remain with their separate recovery owners.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn recover_pending_root_observation(
    input: LocalOperationRecoveryContext<'_>,
    source: (
        &CommonRequestEnvelopeV1,
        &OperationStartRecoveryEvidence,
        usize,
    ),
    limits: AccumulatorPreparationLimits,
    custody: &mut dyn OperationCustodyPort,
    sessions: &dyn SessionLeaseCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admission: &mut OperationControlRecoveryAdmission<'_>,
    historical_collection: Option<(
        ConsultationCollectionLimits,
        &mut HistoricalConsultationAdmission<'_>,
    )>,
) -> Result<(), PendingRootObservationError> {
    let (request, start, maximum_parent_depth) = source;
    input.verify_limits()?;
    let hint: OperationHandleWireV1 = serde_json::from_value(
        canon::parse_strict(start.handle().canonical_record_jcs().as_slice())
            .map_err(LocalOperationRecoveryError::from)?,
    )
    .map_err(LocalOperationRecoveryError::from)?;
    // Only the root source owns repair of its shared domain. Skipping this
    // mutation attempt still requires ordinary protected recovery below it.
    if hint.parent_operation().is_some() {
        return Ok(());
    }
    let selection = input
        .profile
        .select(input.store, request)
        .map_err(LocalOperationRecoveryError::from)?;
    if !selection.matches_authority(input.checkpoint_authority) || start.store() != input.store {
        return Err(LocalOperationRecoveryError::Association.into());
    }
    let domain = selection
        .initial_basis()
        .map_err(LocalOperationRecoveryError::from)?
        .domain()
        .clone();
    input.checkpoint_limits.require_work(1, 0, 1)?;
    let (head, next_record) = custody
        .read_checkpoint_head(
            input.store,
            &domain,
            input
                .maximum_record_bytes
                .min(input.checkpoint_limits.remaining_bytes(0)?),
        )
        .map_err(LocalOperationRecoveryError::from)?
        .ok_or(LocalOperationRecoveryError::Missing)?;
    let expected_anchor = AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
        input.store.store_id().clone(),
        input.store.incarnation_id().clone(),
        domain,
        head.domain_epoch(),
        head.domain_sequence(),
        head.coordinate().clone(),
    ));
    let retained_anchor = anchor
        .read(&expected_anchor.key())
        .map_err(LocalOperationRecoveryError::from)?
        .ok_or(LocalOperationRecoveryError::Missing)?;
    if retained_anchor == expected_anchor {
        return Ok(());
    }
    input.checkpoint_limits.require_work(
        2,
        next_record.canonical_record_jcs().as_slice().len(),
        2,
    )?;
    let next = admit_checkpoint(
        &next_record,
        input,
        &mut *admission.verify_checkpoint_signature,
    )?;
    if head.coordinate() != next_record.coordinate()
        || head.checkpoint_domain_id() != next.checkpoint_domain_id()
        || head.domain_epoch() != *next.domain_epoch()
        || head.domain_sequence() != *next.domain_sequence()
        || head.committed_transaction_sequence() != *next.committed_transaction_sequence()
    {
        return Err(LocalOperationRecoveryError::Association.into());
    }
    let prior_coordinate = record_projection::coordinate(
        next.previous_control_checkpoint_ref()
            .as_ref()
            .ok_or(LocalOperationRecoveryError::Association)?,
        next.previous_control_checkpoint_sha256()
            .as_ref()
            .ok_or(LocalOperationRecoveryError::Association)?
            .as_str(),
    )
    .map_err(|_| LocalOperationRecoveryError::Association)?;
    let prior_limit = input.maximum_record_bytes.min(
        input
            .checkpoint_limits
            .remaining_bytes(next_record.canonical_record_jcs().as_slice().len())?,
    );
    let prior_record = custody
        .read_operation_record(input.store, &prior_coordinate, prior_limit)
        .map_err(LocalOperationRecoveryError::from)?
        .ok_or(LocalOperationRecoveryError::Missing)?;
    input.checkpoint_limits.require_work(
        2,
        next_record
            .canonical_record_jcs()
            .as_slice()
            .len()
            .checked_add(prior_record.canonical_record_jcs().as_slice().len())
            .ok_or(LocalOperationRecoveryError::Bounds)?,
        2,
    )?;
    let checkpoint_schema =
        pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID)
            .map_err(LocalOperationRecoveryError::from)?;
    check_record(
        &prior_record,
        &checkpoint_schema.schema,
        &checkpoint_schema.schema_sha256,
        prior_limit,
    )?;
    let prior_wire = serde_json::from_value(
        canon::parse_strict(prior_record.canonical_record_jcs().as_slice())
            .map_err(LocalOperationRecoveryError::from)?,
    )
    .map_err(LocalOperationRecoveryError::from)?;
    let prior = (admission.admit_checkpoint)(prior_coordinate.record_ref(), prior_wire)
        .map_err(LocalOperationRecoveryError::from)?;
    if prior_record.coordinate().record_sha256().as_str()
        != prior.canonical_control_checkpoint_sha256().as_str()
        || prior_record.canonical_record_jcs().as_slice()
            != canon::to_canonical_string(&prior)
                .map_err(LocalOperationRecoveryError::from)?
                .as_bytes()
    {
        return Err(LocalOperationRecoveryError::Association.into());
    }
    next.verify_successor_of(prior_coordinate.record_ref(), &prior)
        .map_err(LocalOperationRecoveryError::from)?;
    let prior_anchor = AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
        input.store.store_id().clone(),
        input.store.incarnation_id().clone(),
        prior.checkpoint_domain_id().clone(),
        *prior.domain_epoch(),
        *prior.domain_sequence(),
        prior_coordinate.clone(),
    ));
    if prior_record.coordinate() != &prior_coordinate || retained_anchor != prior_anchor {
        return Err(LocalOperationRecoveryError::Association.into());
    }
    let (handle, lease, _) = identity::admit_handle(
        input,
        request,
        None,
        start.handle(),
        custody,
        sessions,
        (
            &mut *admission.admit_handle_authority,
            &mut *admission.verify_handle_signature,
        ),
    )?;
    if !matches!(
        handle.operation_class(),
        OperationClassV1::Invoke | OperationClassV1::Consult
    ) || handle.parent_operation().is_some()
    {
        return Err(LocalOperationRecoveryError::Association.into());
    }
    selection
        .verify(input.store, &handle, &next)
        .map_err(LocalOperationRecoveryError::from)?;
    selection
        .verify(input.store, &handle, &prior)
        .map_err(LocalOperationRecoveryError::from)?;
    let selected = super::pending_expiry_child::select_expiry_identity(
        input,
        super::pending_expiry_child::RootIdentity {
            request,
            start,
            handle: &handle,
            lease: &lease,
            maximum_parent_depth,
        },
        &next,
        custody,
        sessions,
        admission,
    )?;
    let super::pending_expiry_child::ExpiryIdentity {
        start: target_start,
        handle,
        selection,
        leaves,
    } = selected;
    let start = target_start.as_ref().unwrap_or(start);
    selection
        .verify(input.store, &handle, &prior)
        .map_err(LocalOperationRecoveryError::from)?;
    let evidence = read_operation_observation_for_handle(
        custody,
        input.store,
        start.handle().coordinate().record_ref(),
        &handle,
        input.maximum_record_bytes,
    )
    .map_err(LocalOperationRecoveryError::from)?
    .ok_or(LocalOperationRecoveryError::Missing)?;
    if evidence.checkpoint() != &next_record {
        return Err(LocalOperationRecoveryError::Association.into());
    }
    let (predecessor, observation) = admit_observation_history(
        input,
        start,
        &handle,
        evidence.observation().clone(),
        custody,
        admission,
    )?;
    let membership = OperationAccumulatorMembership::new(
        OperationAccumulatorCheckpoint::new(
            input.store.clone(),
            next.checkpoint_profile_ref().clone(),
            next.checkpoint_profile_sha256().clone(),
            head,
            next.state_root_sha256().clone(),
            next.unresolved_custody_root_sha256().clone(),
        ),
        leaves,
        input.membership_limits,
        selection.implementation(),
    )
    .map_err(LocalOperationRecoveryError::from)?;
    let verified = custody
        .verify_accumulator_membership(&membership)
        .map_err(LocalOperationRecoveryError::from)?;
    if verified.request() != &membership {
        return Err(LocalOperationRecoveryError::Association.into());
    }
    if matches!(
        observation.1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::CollectingConsultation { .. }
        }
    ) {
        if handle.operation_class() != &OperationClassV1::Consult
            || handle.parent_operation().is_some()
        {
            return Err(LocalOperationRecoveryError::Association.into());
        }
        let (collection_limits, admit_historical) =
            historical_collection.ok_or(LocalOperationRecoveryError::Association)?;
        let resolve = |_: &OperationObservationV1| Ok(input.store.clone());
        let parent = (start.handle().coordinate().record_ref(), &handle);
        let opened = ConsultationCollection::recover_historical(
            parent,
            &observation.1,
            collection_limits,
            &resolve,
            custody,
            admit_historical,
        )?;
        let pending = crate::resident::consultation::recover_committed_initial_collection(
            &opened,
            OperationObservationTransition::new(
                parent.0,
                parent.1,
                &predecessor.0,
                &predecessor.1,
                &observation.0,
                &observation.1,
            ),
            CommittedObservationCheckpointRecovery {
                selection,
                predecessor: (prior_coordinate.record_ref(), &prior),
                successor: (next_record.coordinate().record_ref(), &next),
                limits,
            },
            collection_limits,
            &resolve,
            custody,
        )?;
        pending.advance_anchor(anchor).map_err(Box::new)?;
        return Ok(());
    }
    if matches!(
        observation.1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::ConsultationCollected { .. }
        }
    ) {
        if handle.operation_class() != &OperationClassV1::Consult
            || handle.parent_operation().is_some()
        {
            return Err(LocalOperationRecoveryError::Association.into());
        }
        let (collection_limits, admit_historical) =
            historical_collection.ok_or(LocalOperationRecoveryError::Association)?;
        let resolve = |_: &OperationObservationV1| Ok(input.store.clone());
        let parent = (start.handle().coordinate().record_ref(), &handle);
        let previous = ConsultationCollection::recover_historical(
            parent,
            &predecessor.1,
            collection_limits,
            &resolve,
            custody,
            &mut *admit_historical,
        )?;
        let stopped = ConsultationCollection::recover_historical(
            parent,
            &observation.1,
            collection_limits,
            &resolve,
            custody,
            &mut *admit_historical,
        )?;
        let pending = recover_committed_consultation_expiry(
            &previous,
            &stopped,
            OperationObservationTransition::new(
                parent.0,
                parent.1,
                &predecessor.0,
                &predecessor.1,
                &observation.0,
                &observation.1,
            ),
            CommittedObservationCheckpointRecovery {
                selection,
                predecessor: (prior_coordinate.record_ref(), &prior),
                successor: (next_record.coordinate().record_ref(), &next),
                limits,
            },
            collection_limits,
            &resolve,
            custody,
        )?;
        pending.advance_anchor(anchor).map_err(Box::new)?;
        return Ok(());
    }
    let OperationObservationStateV1::Expired {
        diagnostic: reference,
    } = observation.1.state()
    else {
        return Err(LocalOperationRecoveryError::Association.into());
    };
    let coordinate =
        record_projection::coordinate(&reference.record_ref, reference.record_sha256.as_str())
            .map_err(|_| LocalOperationRecoveryError::Association)?;
    let record = custody
        .read_operation_record(input.store, &coordinate, input.maximum_record_bytes)
        .map_err(LocalOperationRecoveryError::from)?
        .ok_or(LocalOperationRecoveryError::Missing)?;
    if record.coordinate() != &coordinate {
        return Err(LocalOperationRecoveryError::Association.into());
    }
    let diagnostic = crate::diagnostic::AdmittedDiagnostic::from_bytes(
        record.canonical_record_jcs().as_slice(),
    )?;
    let pending = recover_committed_pre_dispatch_expiry(
        OperationObservationTransition::new(
            start.handle().coordinate().record_ref(),
            &handle,
            &predecessor.0,
            &predecessor.1,
            &observation.0,
            &observation.1,
        ),
        &diagnostic,
        CommittedObservationCheckpointRecovery {
            selection,
            predecessor: (prior_coordinate.record_ref(), &prior),
            successor: (next_record.coordinate().record_ref(), &next),
            limits,
        },
        &|_: &OperationObservationV1| Ok(input.store.clone()),
        custody,
    )?;
    pending.advance_anchor(anchor).map_err(Box::new)?;
    Ok(())
}

type CheckpointSignatureAdmission<'a> = dyn FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

fn admit_checkpoint(
    record: &ControlRecord,
    input: LocalOperationRecoveryContext<'_>,
    signature: &mut CheckpointSignatureAdmission<'_>,
) -> Result<ControlStateCheckpointV1, LocalOperationRecoveryError> {
    let schema = pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID)?;
    check_record(
        record,
        &schema.schema,
        &schema.schema_sha256,
        input.maximum_record_bytes,
    )?;
    let value = ControlStateCheckpointV1::admit_signed_wire(
        serde_json::from_value(canon::parse_strict(
            record.canonical_record_jcs().as_slice(),
        )?)?,
        &schema.schema_sha256,
        input.checkpoint_authority,
        signature,
    )?;
    if record.coordinate().record_sha256().as_str()
        != value.canonical_control_checkpoint_sha256().as_str()
        || record.canonical_record_jcs().as_slice()
            != canon::to_canonical_string(&value)?.as_bytes()
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    Ok(value)
}

type Observation = (ResidentRecordRefV1, OperationObservationV1);

fn admit_observation_history(
    input: LocalOperationRecoveryContext<'_>,
    start: &OperationStartRecoveryEvidence,
    handle: &OperationHandleV1,
    record: ControlRecord,
    custody: &mut dyn OperationCustodyPort,
    admission: &mut OperationControlRecoveryAdmission<'_>,
) -> Result<(Observation, Observation), PendingRootObservationError> {
    let history = read_history(custody, input.store, record, input.observation_limits)?;
    let schema = pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID)
        .map_err(LocalOperationRecoveryError::from)?;
    let mut predecessor = None;
    let mut current: Option<Observation> = None;
    for (record, wire) in history.into_iter().rev() {
        let signing =
            (admission.resolve_observation_signer)(record.coordinate().record_ref(), &wire)
                .map_err(LocalOperationRecoveryError::from)?;
        let observation = OperationObservationV1::admit_contextual_wire(
            wire,
            &OperationObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema.schema_sha256,
                    signing_authority: &signing,
                },
                handle_ref: start.handle().coordinate().record_ref(),
                handle,
                predecessor: current
                    .as_ref()
                    .map(|(reference, value)| (reference, value)),
            },
            &mut *admission.admit_observation_state,
            &mut *admission.admit_rotation,
            &mut *admission.verify_observation_signature,
        )
        .map_err(LocalOperationRecoveryError::from)?;
        if record.coordinate().record_sha256().as_str()
            != observation
                .canonical_operation_observation_sha256()
                .as_str()
            || record.canonical_record_jcs().as_slice()
                != canon::to_canonical_string(&observation)
                    .map_err(LocalOperationRecoveryError::from)?
                    .as_bytes()
            || (current.is_none() && &record != start.initial_observation())
        {
            return Err(LocalOperationRecoveryError::Association.into());
        }
        predecessor = current;
        current = Some((record.coordinate().record_ref().clone(), observation));
    }
    Ok((
        predecessor.ok_or(LocalOperationRecoveryError::Missing)?,
        current.ok_or(LocalOperationRecoveryError::Missing)?,
    ))
}
