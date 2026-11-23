//! External compile proof for public Consult service composition.

use pale_application::resident::{
    AmbientSessionAuthorityPort, ResidentFrameService, ResidentServiceAssemblyError,
    ResidentServiceCallerSignaturePort, ResidentServiceCancellationInputPort,
    ResidentServiceConsultInputPort, ResidentServiceConsultPolicyPort,
    ResidentServiceConsultStopInput, ResidentServiceCurrentRequestEvidencePort,
    ResidentServiceCurrentWriterAdmissionPort, ResidentServiceInvokeConsultObserveCancelOwners,
    ResidentServiceInvokeCryptographyPort, ResidentServiceInvokeInputPort,
    ResidentServiceInvokePolicyPort, ResidentServiceObserveInputPort, ResidentServiceProcessInput,
};
use pale_ports::{CryptographicRandomSourcePort, PrincipalWriterGuardPort, WallClockSourcePort};

#[test]
fn consult_composition_uses_only_public_ports() {
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
        ConsultInputs,
        CancellationInputs,
        ObserveInputs,
    >(
        process: ResidentServiceProcessInput<'service, 'service, WriterGuard, WriterAdmission>,
        owners: ResidentServiceInvokeConsultObserveCancelOwners<
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
            ConsultInputs,
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
        Policy: ResidentServiceInvokePolicyPort + ResidentServiceConsultPolicyPort + 'service,
        Policy::StopInput: ResidentServiceConsultStopInput + 'service,
        Cryptography: ResidentServiceInvokeCryptographyPort + 'service,
        InvokeInputs: ResidentServiceInvokeInputPort + 'service,
        ConsultInputs: ResidentServiceConsultInputPort + 'service,
        CancellationInputs: ResidentServiceCancellationInputPort<'service> + 'service,
        ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
    {
        ResidentFrameService::recover_invoke_consult_observe_cancel(process, owners)
    }

    // Compiling this generic body from an external crate proves that the public
    // owner and builder do not require a private intent or custody type.
}
