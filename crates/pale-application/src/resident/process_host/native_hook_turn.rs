//! Resume an exact native occurrence through current authority and head custody.

use super::*;
use crate::resident::operation_dispatch::hook_response::{
    prepare_and_release_native_hook_response, NativeHookResponseFailure, NativeHookResponseStart,
    NativeHookResponseTurn,
};
use crate::resident::AuthenticatedResidentHookRequest;

impl<WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
    ResidentProgram<'_, WriterGuard, PolicyEvaluator, TurnOwner, StopInput>
where
    TurnOwner: ResidentProgramTurnOwner<WriterGuard, PolicyEvaluator, StopInput>,
{
    pub(in crate::resident) fn admit_native_hook(
        &mut self,
        peer: LocalIpcPeerCredentials,
        request: &AuthenticatedResidentHookRequest,
    ) -> Result<pale_ports::ResidentHookFrameResponse, ResidentOperationFrameFailure> {
        self.turn_owner.admit_native_hook_turn(
            &mut self.process,
            &self.deployment_authorities,
            peer,
            request,
        )
    }
}

/// Correlate the native occurrence, durably prepare and dispatch its response,
/// then recheck current authority before returning the exact admitted frame.
pub(super) fn admit_hook_turn<
    WriterGuard,
    PeerWitness,
    Peer,
    Evidence,
    Clock,
    SessionAuthority,
    Signature,
    Random,
    OperationAuthority,
    Cryptography,
    Recovery,
    ConsultRecovery,
>(
    owner: &mut ResidentProgramOperationTurnOwner<
        '_,
        '_,
        PeerWitness,
        Peer,
        Evidence,
        Clock,
        SessionAuthority,
        Signature,
        Random,
        OperationAuthority,
        Cryptography,
        Recovery,
        ConsultRecovery,
    >,
    process: &mut ResidentProcessHost<'_, WriterGuard>,
    deployment_authorities: &[ResidentProgramDeploymentAuthority],
    peer: LocalIpcPeerCredentials,
    request: &AuthenticatedResidentHookRequest,
) -> Result<pale_ports::ResidentHookFrameResponse, ResidentOperationFrameFailure>
where
    WriterGuard: PrincipalWriterGuardPort,
    Peer: ResidentProgramPeerAdmissionPort<Witness = PeerWitness>,
    Evidence: ResidentCurrentRequestEvidencePort<PeerWitness>,
    Clock: pale_ports::WallClockSourcePort,
    SessionAuthority: crate::resident::AmbientSessionAuthorityPort,
    Signature: ResidentProgramCallerSignaturePort,
    OperationAuthority: crate::resident::AmbientSessionAuthorityPort,
    Recovery: ResidentProgramRootRecoveryInputPort,
{
    use ResidentOperationFrameFailure as Failure;
    if owner.admitted.is_some() {
        return Err(Failure::ReplayConflict);
    }
    let peer = owner
        .peer
        .admit_hook_peer(peer)
        .map_err(|_| Failure::AdmissionDenied)?;
    let hooks = owner
        .native_hooks
        .as_mut()
        .ok_or(Failure::AdmissionDenied)?;
    // Match the captured key, occurrence, route and original bytes before reading
    // protected evidence. The retained signature is a claim to re-admit now.
    let original = hooks.original_request(request)?;
    let bytes =
        pale_core::canon::to_canonical_string(original).map_err(|_| Failure::InvalidRequest)?;
    let admitted = owner.current.admit_bytes(
        process,
        deployment_authorities,
        &peer,
        bytes.as_bytes(),
        ResidentProgramRequestPolicyKind::Operation(
            operation_request_host::ResidentOperationRequestKind::StartInvoke,
        ),
        None,
    )?;
    owner
        .current
        .signature
        .admit_native_caller_key(admitted.request().caller(), &request.caller_key())
        .map_err(|_| Failure::AdmissionDenied)?;
    let correlated =
        admitted.with_native_context(process, owner.current.historical, |context| {
            let correlated = hooks.continue_hook(
                request,
                native_hook_start::NativeHookStartContext {
                    application: context.application,
                    session: context.session,
                    historical: context.historical,
                    current: context.current,
                },
                &mut owner.operation_authority,
            )?;
            correlated
                .admit_native_tool(context.application)
                .map_err(|_| Failure::AdmissionDenied)?;
            Ok::<_, Failure>(correlated)
        })??;
    let locator =
        operation_locator::ResidentProgramOperationLocator::from_issued(correlated.operation())
            .map_err(|_| Failure::AdmissionDenied)?;
    let selector = admitted.selector();
    let mut matching = owner
        .deployments
        .iter_mut()
        .filter(|entry| entry.matches(selector));
    let selected = matching.next().ok_or(Failure::ConfigurationUnavailable)?;
    if matching.next().is_some() {
        return Err(Failure::ConfigurationUnavailable);
    }
    let response = hooks.response_mut(request)?;
    let recovered = if response.is_none() {
        Some(owner.recovery.with_root_recovery(
            selector,
            admitted.request(),
            Some(&locator),
            |restart, mut admission, _, _| {
                let access = selected.access();
                let recovered = crate::resident::recover_operation_control(
                    locator.recovery_input(
                        restart.operation,
                        restart.maximum_parent_depth,
                        restart.current_head,
                    ),
                    &mut *access.custody,
                    access.sessions,
                    &mut *access.anchor,
                    reborrow_operation_recovery_admission(&mut admission),
                )
                .map_err(|_| Failure::ResidentUnavailable)?;
                locator
                    .admit_recovered(&recovered)
                    .map_err(|_| Failure::ReplayConflict)?;
                verify_current_head(
                    admitted.request(),
                    correlated.operation(),
                    &recovered,
                    access.custody,
                    access.anchor,
                    restart.operation.maximum_record_bytes,
                )?;
                Ok::<_, Failure>(recovered)
            },
        )??)
    } else {
        None
    };
    let result = owner.recovery.with_native_hook_response_inputs(
        selector,
        admitted.request(),
        |inputs| {
            admitted.with_native_context(process, owner.current.historical, |context| {
                let access = selected.access();
                prepare_and_release_native_hook_response(
                    NativeHookResponseTurn {
                        correlated: &correlated,
                        request: admitted.request(),
                        application: context.application,
                        current: context.current,
                        start: recovered.as_ref().map(|recovered| NativeHookResponseStart {
                            predecessor: recovered.observation().selected(),
                            checkpoint: recovered.observation().current_checkpoint(),
                        }),
                        custody: access.custody,
                        anchor: access.anchor,
                    },
                    inputs,
                    response,
                    &mut owner.operation_authority,
                    |operation_authority, response_authority, filesystem| {
                        use NativeHookResponseFailure as ResponseFailure;
                        let policy = ResidentProgramRequestPolicyKind::Operation(
                            operation_request_host::ResidentOperationRequestKind::StartInvoke,
                        );
                        let fresh = owner
                            .current
                            .admit_bytes(
                                process,
                                deployment_authorities,
                                &peer,
                                bytes.as_bytes(),
                                policy,
                                None,
                            )
                            .map_err(|_| ResponseFailure::Authority)?;
                        if fresh.selector() != selector || fresh.request() != admitted.request() {
                            return Err(ResponseFailure::Authority);
                        }
                        owner
                            .current
                            .signature
                            .admit_native_caller_key(
                                fresh.request().caller(),
                                &request.caller_key(),
                            )
                            .map_err(|_| ResponseFailure::Authority)?;
                        fresh
                            .with_native_context(
                                process,
                                owner.current.historical,
                                |fresh_context| {
                                    correlated
                                        .operation()
                                        .revalidate_session(
                                            fresh_context.current,
                                            operation_authority,
                                        )
                                        .map_err(|_| ResponseFailure::Authority)?;
                                    correlated
                                        .admit_response_policy(
                                            fresh.request(),
                                            fresh_context.application,
                                            fresh_context.current.now,
                                            filesystem,
                                            response_authority,
                                        )
                                        .map_err(|_| ResponseFailure::Authority)
                                },
                            )
                            .map_err(|_| ResponseFailure::Authority)?
                    },
                )
            })
        },
    )??;
    match result {
        Ok(response) => Ok(response),
        Err(NativeHookResponseFailure::Custody) => Err(Failure::ResidentUnavailable),
        Err(NativeHookResponseFailure::Admission(error)) => {
            let _ = error;
            Err(Failure::AdmissionDenied)
        }
        Err(NativeHookResponseFailure::Authority) => Err(Failure::AdmissionDenied),
        Err(NativeHookResponseFailure::Release) => Err(Failure::ResidentUnavailable),
    }
}

pub(in crate::resident) fn verify_current_head(
    request: &CommonRequestEnvelopeV1,
    operation: &crate::resident::IssuedLocalOperation,
    recovered: &crate::resident::RecoveredOperationControl,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_record_bytes: usize,
) -> Result<(), ResidentOperationFrameFailure> {
    if recovered.operation().request() != request
        || recovered.operation().operation().handle() != operation.handle()
    {
        return Err(ResidentOperationFrameFailure::ReplayConflict);
    }
    crate::resident::operation_dispatch::verify_recovered_operation_head_custody(
        operation,
        recovered.observation(),
        custody,
        anchor,
        maximum_record_bytes,
    )
    .map_err(|_| ResidentOperationFrameFailure::ResidentUnavailable)
}
