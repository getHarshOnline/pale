//! External compile proof for public Delegate service composition.

use pale_application::resident::{
    AmbientSessionAuthorityPort, ResidentFrameService, ResidentServiceAssemblyError,
    ResidentServiceCallerSignaturePort, ResidentServiceCancellationInputPort,
    ResidentServiceConsultInputPort, ResidentServiceConsultPolicyPort,
    ResidentServiceConsultStopInput, ResidentServiceCurrentRequestEvidencePort,
    ResidentServiceCurrentWriterAdmissionPort, ResidentServiceDelegateInputPort,
    ResidentServiceDelegatePolicyPort, ResidentServiceInvokeConsultDelegateObserveCancelOwners,
    ResidentServiceInvokeCryptographyPort, ResidentServiceInvokeInputPort,
    ResidentServiceInvokePolicyPort, ResidentServiceObserveInputPort, ResidentServiceProcessInput,
    ResidentServicePublicationInputPort,
};
use pale_ports::{CryptographicRandomSourcePort, PrincipalWriterGuardPort, WallClockSourcePort};

#[test]
fn delegate_composition_uses_only_public_ports() {
    #[allow(dead_code)]
    trait RecoverFromPublicParts<'service, WriterGuard, WriterAdmission> {
        fn recover_from_public_parts(
            self,
            process: ResidentServiceProcessInput<'service, 'service, WriterGuard, WriterAdmission>,
        ) -> Result<ResidentFrameService<'service>, ResidentServiceAssemblyError>;
    }

    impl<
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
            DelegateInputs,
            PublicationInputs,
            CancellationInputs,
            ObserveInputs,
        > RecoverFromPublicParts<'service, WriterGuard, WriterAdmission>
        for ResidentServiceInvokeConsultDelegateObserveCancelOwners<
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
            DelegateInputs,
            PublicationInputs,
            CancellationInputs,
            ObserveInputs,
        >
    where
        WriterGuard: PrincipalWriterGuardPort + 'service,
        WriterAdmission: ResidentServiceCurrentWriterAdmissionPort,
        Evidence: ResidentServiceCurrentRequestEvidencePort + 'service,
        Clock: WallClockSourcePort + 'service,
        SessionAuthority: AmbientSessionAuthorityPort + 'service,
        CallerSignature: ResidentServiceCallerSignaturePort + 'service,
        Random: CryptographicRandomSourcePort + 'service,
        OperationAuthority: AmbientSessionAuthorityPort + 'service,
        Policy: ResidentServiceInvokePolicyPort
            + ResidentServiceConsultPolicyPort
            + ResidentServiceDelegatePolicyPort
            + 'service,
        Policy::StopInput: ResidentServiceConsultStopInput + 'service,
        Cryptography: ResidentServiceInvokeCryptographyPort + 'service,
        InvokeInputs: ResidentServiceInvokeInputPort + 'service,
        ConsultInputs: ResidentServiceConsultInputPort + 'service,
        DelegateInputs: ResidentServiceDelegateInputPort<'service> + 'service,
        PublicationInputs: ResidentServicePublicationInputPort + 'service,
        CancellationInputs: ResidentServiceCancellationInputPort<'service> + 'service,
        ObserveInputs: ResidentServiceObserveInputPort<'service> + 'service,
    {
        fn recover_from_public_parts(
            self,
            process: ResidentServiceProcessInput<'service, 'service, WriterGuard, WriterAdmission>,
        ) -> Result<ResidentFrameService<'service>, ResidentServiceAssemblyError> {
            self.recover(process)
        }
    }

    // This external generic body proves that assembly needs no private restart
    // phase or private A2A recovery trait.
}
