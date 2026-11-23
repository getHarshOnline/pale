//! External compile proof for public resident service composition.

use pale_application::resident::{
    with_recovered_resident_service_process, AmbientSessionAuthorityPort, ResidentFrameService,
    ResidentServiceAssemblyError, ResidentServiceCallerSignaturePort,
    ResidentServiceCancellationInputPort, ResidentServiceCurrentRequestEvidencePort,
    ResidentServiceCurrentWriterAdmissionPort, ResidentServiceInvokeCryptographyPort,
    ResidentServiceInvokeInputPort, ResidentServiceInvokeObserveCancelOwners,
    ResidentServiceInvokePolicyPort, ResidentServiceObserveInputPort,
    ResidentServicePhysicalProcessRecoveryError, ResidentServicePhysicalProcessRecoveryInput,
    ResidentServiceProcessInput,
};
use pale_ports::{CryptographicRandomSourcePort, PrincipalWriterGuardPort, WallClockSourcePort};

#[test]
fn invoke_observe_cancel_composition_uses_only_public_ports() {
    #[allow(dead_code, clippy::too_many_arguments)]
    fn recover_from_public_parts<
        'service,
        WriterGuard,
        WriterAdmission,
        Evidence,
        Clock,
        SessionAuthority,
        CallerSignature,
        Random,
        OperationAuthority,
        Policy,
        Cryptography,
        InvokeInputs,
        CancellationInputs,
        ObserveInputs,
    >(
        process: ResidentServiceProcessInput<'service, 'service, WriterGuard, WriterAdmission>,
        owners: ResidentServiceInvokeObserveCancelOwners<
            'service,
            Evidence,
            Clock,
            SessionAuthority,
            CallerSignature,
            Random,
            OperationAuthority,
            Policy,
            Cryptography,
            InvokeInputs,
            CancellationInputs,
            ObserveInputs,
        >,
    ) -> Result<ResidentFrameService<'service>, ResidentServiceAssemblyError>
    where
        WriterGuard: PrincipalWriterGuardPort + 'service,
        WriterAdmission: ResidentServiceCurrentWriterAdmissionPort,
        Evidence: ResidentServiceCurrentRequestEvidencePort + 'service,
        Clock: WallClockSourcePort + 'service,
        SessionAuthority: AmbientSessionAuthorityPort + 'service,
        CallerSignature: ResidentServiceCallerSignaturePort + 'service,
        Random: CryptographicRandomSourcePort + 'service,
        OperationAuthority: AmbientSessionAuthorityPort + 'service,
        Policy: ResidentServiceInvokePolicyPort + 'service,
        Cryptography: ResidentServiceInvokeCryptographyPort + 'service,
        InvokeInputs: ResidentServiceInvokeInputPort + 'service,
        CancellationInputs: ResidentServiceCancellationInputPort<'service> + 'service,
        ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
    {
        let ResidentServiceInvokeObserveCancelOwners {
            peer_admission,
            maximum_request_bytes,
            request_policies,
            historical,
            evidence,
            clock,
            session_authority,
            caller_signature,
            random,
            operation_authority,
            policy,
            cryptography,
            invoke_inputs,
            cancel_inputs,
            observe_inputs,
        } = owners;
        ResidentFrameService::recover_invoke_observe_cancel(
            process,
            ResidentServiceInvokeObserveCancelOwners {
                peer_admission,
                maximum_request_bytes,
                request_policies,
                historical,
                evidence,
                clock,
                session_authority,
                caller_signature,
                random,
                operation_authority,
                policy,
                cryptography,
                invoke_inputs,
                cancel_inputs,
                observe_inputs,
            },
        )
    }

    // The generic function body is compiled as an external crate. That is the
    // proof: no private phase owner or private trait appears in construction.
}

#[test]
fn physical_process_recovery_lends_only_a_scoped_public_input() {
    #[allow(dead_code)]
    fn recover_from_public_parts<'physical, WriterGuard, WriterAdmission, Output, UseProcess>(
        input: ResidentServicePhysicalProcessRecoveryInput<'physical, WriterGuard, WriterAdmission>,
        use_process: UseProcess,
    ) -> Result<Output, ResidentServicePhysicalProcessRecoveryError>
    where
        WriterGuard: PrincipalWriterGuardPort,
        UseProcess: for<'evidence> FnOnce(
            ResidentServiceProcessInput<'evidence, 'physical, WriterGuard, WriterAdmission>,
        ) -> Output,
    {
        with_recovered_resident_service_process(input, use_process)
    }

    // This generic body compiles as an external crate. The callback can consume
    // the process input, but it cannot return a borrow of the locally recovered
    // writer snapshot or trust frontier.
}
