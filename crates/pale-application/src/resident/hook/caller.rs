//! Join one selected connection key to an exact admitted native session.

use pale_core::records::{
    CapabilitySessionGrantDigest, HarnessActivityIdentity, NativeSessionCorrelationV1,
    ResidentRecordRefV1, SessionAdmissionIdV1, SessionAdmissionLeaseDigest, SessionGrantIdV1,
};

use super::{
    AdmittedAmbientSession, AdmittedResidentHookSession, AmbientSessionAdmissionContext,
    AmbientSessionAuthorityPort, GenerationLifecycleRecoveryResolver, ResidentApplication,
    ResidentHookIngressRequest, ResidentHookSessionAdmissionError,
    ResidentHookSessionAdmissionErrorKind as Failure,
};

/// A deployment-selected connection key bound to one exact admitted session.
///
/// Composition installs this binding after selecting the connection credential
/// and admitting the session. Construction authenticates no connection and grants
/// no work. The binding preserves the original lease and grant; it cannot follow
/// a replacement session merely because the native host reuses its activity ID.
/// Recovered sessions still need current authority admission on every use.
pub struct ResidentHookCallerBinding {
    caller_key: [u8; 32],
    activity: HarnessActivityIdentity,
    session_id: SessionAdmissionIdV1,
    lease_digest: SessionAdmissionLeaseDigest,
    grant_ref: ResidentRecordRefV1,
    grant_id: SessionGrantIdV1,
    grant_digest: CapabilitySessionGrantDigest,
}

impl ResidentHookCallerBinding {
    /// Bind an explicitly selected credential to this admitted native session.
    ///
    /// # Errors
    /// Rejects a session whose native activity was not observed. The caller must
    /// establish that correlation through session admission, not fill it from a
    /// hook's untrusted claim.
    pub fn new(
        caller_key: [u8; 32],
        session: &AdmittedAmbientSession,
    ) -> Result<Self, ResidentHookSessionAdmissionError> {
        let NativeSessionCorrelationV1::Observed {
            native_session_identity,
        } = session.lease().native_session_correlation()
        else {
            return Err(Failure::UnobservedCallerActivity.into());
        };
        Ok(Self {
            caller_key,
            activity: native_session_identity.clone(),
            session_id: session.lease().session_admission_id().clone(),
            lease_digest: session.lease().session_admission_lease_digest().clone(),
            grant_ref: session.grant_ref().clone(),
            grant_id: session.grant_id().clone(),
            grant_digest: session.grant_sha256().clone(),
        })
    }

    /// Bind the exact request decoded from an authenticated application carrier.
    ///
    /// `caller_key` is the credential the carrier that supplied this request
    /// authenticated. It and the event's native activity must both match before
    /// protected session custody is read. Kernel user IDs and a parsed event
    /// cannot substitute for this authenticated caller key.
    ///
    /// # Errors
    /// Rejects absent or different authentication, or an absent, malformed or
    /// different native activity. Success proves no current session permission.
    pub(in crate::resident) fn admit_authenticated_request<'binding, 'request>(
        &'binding self,
        caller_key: Option<[u8; 32]>,
        request: &'request ResidentHookIngressRequest,
    ) -> Result<CallerBoundResidentHookRequest<'binding, 'request>, ResidentHookSessionAdmissionError>
    {
        if caller_key != Some(self.caller_key) {
            return Err(Failure::CallerSubstitution.into());
        }
        let activity = HarnessActivityIdentity::try_from_host(
            request.event().session_id(),
            request.event().turn_id(),
        )
        .map_err(Failure::InvalidNativeIdentity)?;
        if activity.as_ref() != Some(&self.activity) {
            return Err(Failure::CallerActivitySubstitution.into());
        }
        Ok(CallerBoundResidentHookRequest {
            binding: self,
            request,
        })
    }

    fn matches(&self, session: &AdmittedAmbientSession) -> bool {
        session.lease().session_admission_id() == &self.session_id
            && session.lease().session_admission_lease_digest() == &self.lease_digest
            && session.grant_ref() == &self.grant_ref
            && session.grant_id() == &self.grant_id
            && session.grant_sha256() == &self.grant_digest
    }
}

/// A hook request joined to its selected connection key and native activity.
///
/// Current session recovery and authority checks still precede policy lookup.
#[must_use = "a caller-bound hook request must reach current session admission or be rejected"]
pub struct CallerBoundResidentHookRequest<'binding, 'request> {
    binding: &'binding ResidentHookCallerBinding,
    request: &'request ResidentHookIngressRequest,
}

impl<'request> CallerBoundResidentHookRequest<'_, 'request> {
    /// Recover the exact bound session and recheck current authority and routing.
    ///
    /// # Errors
    /// Preserves failures from native-session recovery and rejects a replacement
    /// lease or grant. Success creates no operation and permits no native effect.
    pub fn admit_native_session<P: AmbientSessionAuthorityPort>(
        &self,
        application: &ResidentApplication<'_>,
        historical: &dyn GenerationLifecycleRecoveryResolver,
        context: &AmbientSessionAdmissionContext<'_>,
        authority: &mut P,
    ) -> Result<AdmittedResidentHookSession<'request>, ResidentHookSessionAdmissionError> {
        let admitted =
            self.request
                .admit_native_session(application, historical, context, authority)?;
        if !self.binding.matches(admitted.session()) {
            return Err(Failure::CallerSessionSubstitution.into());
        }
        Ok(admitted)
    }
}

impl std::fmt::Debug for CallerBoundResidentHookRequest<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CallerBoundResidentHookRequest(<bound>)")
    }
}
