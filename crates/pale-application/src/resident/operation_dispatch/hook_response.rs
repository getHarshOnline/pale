//! Retain an exact native response before one current-authority release.

use super::*;

pub(in crate::resident) mod continuation;
pub(in crate::resident) mod recovery;
use super::checkpoint_authority::RetainedCheckpointAuthority;
use crate::resident::hook::response_policy::AdmittedNativeHookResponse;
use crate::resident::{
    CorrelatedResidentHookOperation, NativeHookResponseAuthorityPort, ResidentApplication,
};
pub(in crate::resident) use continuation::NativeHookResponseContinuation;
use continuation::{phase_append, ResponsePhase, ResponsePhaseInputs};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, ControlRecordBytes,
    ControlRecordCoordinate, ControlRecordDigest, MachineFilesystemPort, OperationCustodyPort,
    ResidentHookFrameResponse, ResidentHookResponseRecord, ResidentOperationFrameRecords,
    WallClockSourcePort,
};

/// Deployment-selected inputs for durable native response preparation.
/// Allocate record and checkpoint identities once for the original occurrence;
/// lend the same identities, schemas, signing selection and bounds on continuation.
/// Current permission and disclosure still require fresh admission on each turn.
/// No default credential, permission owner, signer or filesystem is selected.
pub struct NativeHookResponseInputs<'a> {
    /// Current action approval and disclosure owner, rechecked before dispatch and release.
    pub authority: &'a mut dyn NativeHookResponseAuthorityPort,
    /// Filesystem observations used by the selected write boundary.
    pub filesystem: &'a dyn MachineFilesystemPort,
    /// Clock sampled at every custody boundary.
    pub clock: &'a dyn WallClockSourcePort,
    /// Exact schema and runtime signing authority for observations.
    pub signing: SignedResidentAdmissionContextV1<'a>,
    /// Checkpoint schema resolved independently of the returned records.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Current admitted checkpoint authority for this deployment.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Resolve each observation to its selected authority store.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    /// Finite accumulator work limits.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Maximum bytes in response and custody records.
    pub maximum_record_bytes: usize,
    /// Fresh immutable private response reference.
    pub response_ref: ResidentRecordRefV1,
    /// Fresh preparation observation reference.
    pub preparation_ref: ResidentRecordRefV1,
    /// Fresh dispatch observation reference.
    pub dispatch_ref: ResidentRecordRefV1,
    /// Fresh attempt identity, retained by both phases.
    pub attempt_id: CapabilityAttemptIdV1,
    /// Fresh preparation checkpoint identity.
    pub preparation_checkpoint: OperationCheckpointInput,
    /// Fresh dispatch checkpoint identity.
    pub dispatch_checkpoint: OperationCheckpointInput,
    /// Context already proved by the existing context-binding owner, when required.
    pub context_binding: Option<&'a CompletedOperationContextBinding<'a>>,
    /// Admit a runtime signing-authority rotation through its existing owner.
    pub admit_rotation: &'a mut HookResponseRotation<'a>,
    /// Sign the exact observation prepared by the application.
    pub sign_observation: &'a mut HookResponseObservationSigner<'a>,
    /// Sign the exact checkpoint prepared by the application.
    pub sign_checkpoint: &'a mut HookResponseCheckpointSigner<'a>,
}

/// Historical authority rotation admission, independent of response permission.
pub type HookResponseRotation<'a> = dyn FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
/// Physical observation signer; it cannot select the response or its parents.
pub type HookResponseObservationSigner<'a> = dyn FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;
/// Physical checkpoint signer; it cannot replace the prepared mutation.
pub type HookResponseCheckpointSigner<'a> = dyn FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;

pub(in crate::resident) struct NativeHookResponseTurn<'a> {
    pub correlated: &'a CorrelatedResidentHookOperation<'a>,
    pub request: &'a CommonRequestEnvelopeV1,
    pub application: &'a ResidentApplication<'a>,
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    pub start: Option<NativeHookResponseStart<'a>>,
    pub custody: &'a mut dyn OperationCustodyPort,
    pub anchor: &'a mut dyn AntiRollbackAnchorPort,
}

pub(in crate::resident) struct NativeHookResponseStart<'a> {
    pub predecessor: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    pub checkpoint: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
}

/// A stopped append must be reconciled using its exact retained phase.
#[derive(Debug)]
pub(in crate::resident) enum NativeHookResponseFailure {
    Admission(OperationAttemptError),
    Authority,
    Release,
    Custody,
}

impl From<OperationAttemptError> for NativeHookResponseFailure {
    fn from(value: OperationAttemptError) -> Self {
        Self::Admission(value)
    }
}

/// The final callback re-reads current request/session authority and evaluates
/// the same selected response policy. It runs after dispatch custody, under the
/// process's exclusive operation owner, before any response is returned.
pub(in crate::resident) fn prepare_and_release_native_hook_response<Authority, Refresh>(
    turn: NativeHookResponseTurn<'_>,
    inputs: NativeHookResponseInputs<'_>,
    retained: &mut Option<NativeHookResponseContinuation>,
    authority: &mut Authority,
    refresh: Refresh,
) -> Result<ResidentHookFrameResponse, NativeHookResponseFailure>
where
    Authority: AmbientSessionAuthorityPort,
    Refresh: FnOnce(
        &mut Authority,
        &mut dyn NativeHookResponseAuthorityPort,
        &dyn MachineFilesystemPort,
    ) -> Result<AdmittedNativeHookResponse, NativeHookResponseFailure>,
{
    let operation = turn.correlated.operation();
    let (predecessor, floor) = if let Some(state) = retained.as_ref() {
        if state.returned || !state.matches(&turn, &inputs) {
            return Err(NativeHookResponseFailure::Release);
        }
        let latest = state.dispatch.as_ref().unwrap_or(&state.preparation);
        (
            state.preparation.predecessor.clone(),
            *latest.observation.1.observed_at(),
        )
    } else {
        let start = turn
            .start
            .as_ref()
            .ok_or(OperationAttemptError::Association)?;
        (
            (start.predecessor.0.clone(), start.predecessor.1.clone()),
            *start.checkpoint.1.issued_at(),
        )
    };
    let now = sample_time(inputs.clock, turn.current.now.max(floor))?;
    let session = turn.correlated.session().session();
    let generation = turn
        .application
        .generation(session.lease().compiled_generation_digest())
        .ok_or(OperationAttemptError::Association)?;
    let context = OperationAttemptContext {
        operation,
        request: turn.request,
        session,
        current: turn.current,
        lease: operation.historical_operation_lease(),
        context_binding: inputs.context_binding,
        generation,
        selection_ref: inputs.response_ref.clone(),
        maximum_selection_bytes: inputs.maximum_record_bytes,
        observation_ref: inputs.preparation_ref.clone(),
        observation: OperationObservationAdmissionContextV1 {
            signed: inputs.signing,
            handle_ref: operation.handle().0,
            handle: operation.handle().1,
            predecessor: Some((&predecessor.0, &predecessor.1)),
        },
    };
    context.validate_current_at(&turn.current.at_time(now), authority)?;
    context.require_context_ready()?;
    signing::validate_runtime_signer(
        inputs.signing.signing_authority,
        turn.current,
        OPERATION_OBSERVATION_V1_SCHEMA,
    )?;
    if retained.is_none() {
        let start = turn
            .start
            .as_ref()
            .ok_or(OperationAttemptError::Association)?;
        let references = [
            &inputs.response_ref,
            &inputs.preparation_ref,
            &inputs.dispatch_ref,
            &inputs.preparation_checkpoint.reference,
            &inputs.dispatch_checkpoint.reference,
        ];
        for (index, reference) in references.iter().enumerate() {
            if references[..index].contains(reference)
                || *reference == operation.handle().0
                || *reference == operation.handle().1.operation_lease_ref()
                || *reference == operation.request_coordinate().record_ref()
                || *reference == start.predecessor.0
                || *reference == start.checkpoint.0
            {
                return Err(OperationAttemptError::Association.into());
            }
        }
        let selected = turn
            .correlated
            .admit_response_policy(
                turn.request,
                turn.application,
                now,
                inputs.filesystem,
                inputs.authority,
            )
            .map_err(|_| NativeHookResponseFailure::Authority)?;
        let response = ResidentHookResponseRecord {
            schema_id: SchemaId::parse(pale_ports::PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA)
                .map_err(|_| OperationAttemptError::Encoding)?,
            handle_sha256: operation
                .handle()
                .1
                .canonical_operation_handle_sha256()
                .clone(),
            native_request_sha256: turn.request.body().payload_sha256.clone(),
            permission: selected.permission,
            disclosure: selected.disclosure,
            stdout: selected.output.stdout,
            stderr: selected.output.stderr,
            exit_code: selected.output.exit_code,
        };
        let response_canonical = pale_core::canon::to_canonical_string(&response)
            .map_err(|_| OperationAttemptError::Encoding)?;
        if response_canonical.len() > inputs.maximum_record_bytes {
            return Err(OperationAttemptError::BoundExceeded.into());
        }
        let record = response_record(inputs.response_ref.clone(), &response, &response_canonical)?;
        let now = sample_time(inputs.clock, now)?;
        context.validate_time_at(&turn.current.at_time(now))?;
        let preparation = signing::issue_observation(
            &context.observation,
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::DispatchPrepared {
                    attempt_id: inputs.attempt_id.clone(),
                    evidence: response_evidence(&record),
                },
            },
            now,
            &mut *inputs.admit_rotation,
            &mut *inputs.sign_observation,
        )?;
        // The occurrence owner retains this value before any projection or physical I/O.
        *retained = Some(NativeHookResponseContinuation {
            request: turn.request.clone(),
            response,
            response_canonical,
            record,
            preparation: ResponsePhase::new(
                start.predecessor,
                (inputs.preparation_ref.clone(), preparation),
                start.checkpoint,
                copy_checkpoint_input(&inputs.preparation_checkpoint),
            ),
            dispatch: None,
            dispatch_ref: inputs.dispatch_ref.clone(),
            dispatch_checkpoint: copy_checkpoint_input(&inputs.dispatch_checkpoint),
            attempt_id: inputs.attempt_id.clone(),
            returned: false,
            signing_authority: inputs.signing.signing_authority.clone(),
            observation_schema: inputs.signing.expected_schema_sha256.clone(),
            checkpoint_schema: inputs.checkpoint_schema.clone(),
            checkpoint_authority: RetainedCheckpointAuthority::new(inputs.checkpoint_authority),
            limits: inputs.accumulator_limits,
            maximum_record_bytes: inputs.maximum_record_bytes,
        });
    }
    let state = retained
        .as_mut()
        .expect("the response was retained before custody");
    state.preparation.complete(
        operation,
        Some(&state.record),
        &mut ResponsePhaseInputs {
            checkpoint_schema: inputs.checkpoint_schema,
            checkpoint_authority: inputs.checkpoint_authority,
            resolve_store: inputs.resolve_store,
            accumulator_limits: inputs.accumulator_limits,
            sign_checkpoint: &mut *inputs.sign_checkpoint,
        },
        turn.custody,
        turn.anchor,
    )?;
    phase_append(
        operation,
        &state.preparation.predecessor,
        &state.preparation.observation,
        Some(&state.record),
        inputs.resolve_store,
    )
    .map_err(|_| NativeHookResponseFailure::Custody)?
    .verify_completed_custody(
        state
            .preparation
            .completed
            .as_ref()
            .expect("preparation custody completed"),
    )
    .map_err(|_| NativeHookResponseFailure::Custody)?;
    if state.dispatch.is_none() {
        let now = sample_time(inputs.clock, now)?;
        context.validate_time_at(&turn.current.at_time(now))?;
        let selected = turn
            .correlated
            .admit_response_policy(
                turn.request,
                turn.application,
                now,
                inputs.filesystem,
                inputs.authority,
            )
            .map_err(|_| NativeHookResponseFailure::Release)?;
        if !same_response(&selected, &state.response) {
            return Err(NativeHookResponseFailure::Release);
        }
        let now = sample_time(inputs.clock, now)?;
        context.validate_time_at(&turn.current.at_time(now))?;
        let dispatch = signing::issue_observation(
            &OperationObservationAdmissionContextV1 {
                signed: inputs.signing,
                handle_ref: operation.handle().0,
                handle: operation.handle().1,
                predecessor: Some((
                    &state.preparation.observation.0,
                    &state.preparation.observation.1,
                )),
            },
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched {
                    attempt_id: state.attempt_id.clone(),
                    evidence: response_evidence(&state.record),
                },
            },
            now,
            &mut *inputs.admit_rotation,
            &mut *inputs.sign_observation,
        )?;
        state.dispatch = Some(ResponsePhase::new(
            (
                &state.preparation.observation.0,
                &state.preparation.observation.1,
            ),
            (state.dispatch_ref.clone(), dispatch),
            (
                &state.preparation.identity.reference,
                state
                    .preparation
                    .checkpoint
                    .as_ref()
                    .expect("preparation checkpoint retained"),
            ),
            copy_checkpoint_input(&state.dispatch_checkpoint),
        ));
    }
    let dispatch = state
        .dispatch
        .as_mut()
        .expect("signed dispatch was retained");
    dispatch.complete(
        operation,
        None,
        &mut ResponsePhaseInputs {
            checkpoint_schema: inputs.checkpoint_schema,
            checkpoint_authority: inputs.checkpoint_authority,
            resolve_store: inputs.resolve_store,
            accumulator_limits: inputs.accumulator_limits,
            sign_checkpoint: &mut *inputs.sign_checkpoint,
        },
        turn.custody,
        turn.anchor,
    )?;
    let current = refresh(authority, inputs.authority, inputs.filesystem)
        .map_err(|_| NativeHookResponseFailure::Release)?;
    if current.observed_at < *dispatch.observation.1.observed_at()
        || !same_response(&current, &state.response)
    {
        return Err(NativeHookResponseFailure::Release);
    }
    let now = sample_time(inputs.clock, current.observed_at)
        .map_err(|_| NativeHookResponseFailure::Release)?;
    context
        .validate_time_at(&turn.current.at_time(now))
        .map_err(|_| NativeHookResponseFailure::Release)?;
    let released = ResidentHookFrameResponse::Prepared {
        records: ResidentOperationFrameRecords::new(
            operation.handle().0.as_str().to_owned(),
            pale_core::canon::to_canonical_string(operation.handle().1)
                .map_err(|_| NativeHookResponseFailure::Release)?,
            dispatch.observation.0.as_str().to_owned(),
            pale_core::canon::to_canonical_string(&dispatch.observation.1)
                .map_err(|_| NativeHookResponseFailure::Release)?,
        ),
        response_canonical: state.response_canonical.clone(),
    };
    inputs
        .authority
        .admit_release(turn.request, &released, now)
        .map_err(|_| NativeHookResponseFailure::Release)?;
    phase_append(
        operation,
        &dispatch.predecessor,
        &dispatch.observation,
        None,
        inputs.resolve_store,
    )
    .map_err(|_| NativeHookResponseFailure::Release)?
    .verify_fresh_dispatch_custody(
        dispatch
            .completed
            .as_ref()
            .expect("dispatch custody completed"),
        turn.custody,
        turn.anchor,
        inputs.maximum_record_bytes,
    )
    .map_err(|_| NativeHookResponseFailure::Release)?;
    let now = sample_time(inputs.clock, now).map_err(|_| NativeHookResponseFailure::Release)?;
    context
        .validate_time_at(&turn.current.at_time(now))
        .map_err(|_| NativeHookResponseFailure::Release)?;
    state.returned = true;
    Ok(released)
}

fn same_response(
    selected: &AdmittedNativeHookResponse,
    response: &ResidentHookResponseRecord,
) -> bool {
    selected.output.stdout == response.stdout
        && selected.output.stderr == response.stderr
        && selected.output.exit_code == response.exit_code
        && selected.permission == response.permission
        && selected.disclosure == response.disclosure
}

fn copy_checkpoint_input(input: &OperationCheckpointInput) -> OperationCheckpointInput {
    OperationCheckpointInput {
        reference: input.reference.clone(),
        id: input.id.clone(),
    }
}

fn response_evidence(record: &ControlRecord) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::hash_bytes(record.canonical_record_jcs().as_slice()),
    }
}

pub(in crate::resident) fn response_record(
    reference: ResidentRecordRefV1,
    response: &ResidentHookResponseRecord,
    canonical: &str,
) -> Result<ControlRecord, OperationAttemptError> {
    let schema =
        pale_core::canon::to_canonical_string(&schemars::schema_for!(ResidentHookResponseRecord))
            .map_err(|_| OperationAttemptError::Encoding)?;
    Ok(ControlRecord::new(
        ControlRecordCoordinate::new(
            reference,
            ControlRecordDigest::parse(pale_core::sha256_hex(canonical.as_bytes()))
                .map_err(|_| OperationAttemptError::Encoding)?,
        ),
        response.schema_id.clone(),
        SchemaDigest::parse(pale_core::sha256_hex(schema.as_bytes()))
            .map_err(|_| OperationAttemptError::Encoding)?,
        ControlRecordBytes::new(canonical.as_bytes().to_vec())
            .map_err(|_| OperationAttemptError::Encoding)?,
    ))
}
