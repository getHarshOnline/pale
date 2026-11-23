//! Resolve exact native permission and disclosure before preparing a host response.

use pale_core::records::{CommonRequestEnvelopeV1, ImmutableRecordReferenceV1, UnixTimeV1};
use pale_ports::MachineFilesystemPort;

use super::{
    CorrelatedResidentHookOperation, ResidentApplication, ResidentHookNativeToolBinding,
    ResidentHookWritePolicyEvaluation,
};
use crate::hooks::{HookOutcome, HookOutput, RunPolicyFacts, RunPolicyPlan};

/// Exact request presented to the deployment's current permission owner.
///
/// The owner must verify effect permission, the selected provider's approval
/// requirement and any exact authorization required by the sealed run policy.
/// A transport key, compiled mapping or absence of a write violation is insufficient.
pub struct NativeHookPermissionRequest<'a> {
    /// Original caller-signed request already admitted by the resident.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Exact native occurrence and currently admitted operation/session pair.
    pub correlated: &'a CorrelatedResidentHookOperation<'a>,
    /// Unique compiled capability and effect within that operation's lease.
    pub binding: &'a ResidentHookNativeToolBinding<'a>,
    /// Sealed action-gate requirements; effect scope alone cannot satisfy these.
    pub run_policy: &'a pale_core::records::RunPolicyEntry,
    /// Fresh authority observation time selected by the resident.
    pub now: UnixTimeV1,
}

/// Current permission or disclosure could not be proved for this exact response.
#[derive(Debug, thiserror::Error)]
#[error("native hook response authority is unavailable or does not permit this request")]
pub struct NativeHookResponseAuthorityError;

/// Deployment-owned admission of existing permission and disclosure records.
///
/// Implementations resolve their evidence independently, verify its authenticity,
/// current authority and scope, and return its immutable reference. They must not
/// turn an arbitrary reference supplied by the caller into permission. The same
/// checks run again immediately before release; no default implementation exists.
pub trait NativeHookResponseAuthorityPort {
    /// Admit disclosure of the complete final frame, including signed operation
    /// metadata and permission/disclosure references, at the release boundary.
    ///
    /// # Errors
    /// Refuses any record or field outside this caller's current disclosure rights.
    fn admit_release(
        &mut self,
        request: &CommonRequestEnvelopeV1,
        response: &pale_ports::ResidentHookFrameResponse,
        now: UnixTimeV1,
    ) -> Result<(), NativeHookResponseAuthorityError>;

    /// Prove the exact native action's permission and required owner approval.
    ///
    /// # Errors
    /// Refuses missing, expired, revoked, mismatched or unproved authority.
    fn admit_action(
        &mut self,
        request: &NativeHookPermissionRequest<'_>,
    ) -> Result<ImmutableRecordReferenceV1, NativeHookResponseAuthorityError>;

    /// Prove disclosure of the complete response to this request's actual caller.
    ///
    /// This includes both output channels, current read/disclosure limits and
    /// any context retained by the selected operation. Permission to act does
    /// not by itself permit disclosing the response or another owner's records.
    ///
    /// # Errors
    /// Refuses a response whose disclosure is not currently proved.
    fn admit_disclosure(
        &mut self,
        request: &NativeHookPermissionRequest<'_>,
        response: &HookOutput,
    ) -> Result<ImmutableRecordReferenceV1, NativeHookResponseAuthorityError>;
}

/// Internal proof minted only after the selected permission owner admits the action.
pub(crate) struct NativeHookPermission {
    evidence: ImmutableRecordReferenceV1,
}

pub(in crate::resident) struct AdmittedNativeHookResponse {
    pub(in crate::resident) observed_at: UnixTimeV1,
    pub(in crate::resident) output: HookOutput,
    pub(in crate::resident) permission: ImmutableRecordReferenceV1,
    pub(in crate::resident) disclosure: ImmutableRecordReferenceV1,
}

impl CorrelatedResidentHookOperation<'_> {
    pub(in crate::resident) fn admit_response_policy(
        &self,
        request: &CommonRequestEnvelopeV1,
        application: &ResidentApplication<'_>,
        now: UnixTimeV1,
        filesystem: &dyn MachineFilesystemPort,
        authority: &mut dyn NativeHookResponseAuthorityPort,
    ) -> Result<AdmittedNativeHookResponse, NativeHookResponseAuthorityError> {
        let binding = self
            .admit_native_tool(application)
            .map_err(|_| NativeHookResponseAuthorityError)?;
        let admitted = self.session();
        let policy = admitted
            .request
            .select_hook_policy(application, &admitted.workspace_id, &admitted.session)
            .map_err(|_| NativeHookResponseAuthorityError)?;
        let original = super::ResidentNativeToolStart::admit(request.body())
            .map_err(|_| NativeHookResponseAuthorityError)?;
        if original.request().native_event_sha256() != admitted.request.native_event_sha256()
            || original.request().routing() != admitted.request.routing()
            || request.canonical_request_sha256().as_str()
                != self
                    .operation()
                    .request_coordinate()
                    .record_sha256()
                    .as_str()
        {
            return Err(NativeHookResponseAuthorityError);
        }
        let permission_request = NativeHookPermissionRequest {
            request,
            correlated: self,
            binding: &binding,
            run_policy: &policy.hook_context.run_policy,
            now,
        };
        let permission = NativeHookPermission {
            evidence: authority.admit_action(&permission_request)?,
        };
        let exact = crate::operation_admission::ExactToolAuthorization::from_resident_permission(
            &permission,
        );
        let outcome = RunPolicyPlan::evaluate(
            admitted.request.routing.event(),
            &policy.hook_context.run_policy,
            RunPolicyFacts {
                child_type: admitted.request.event.agent_type(),
                tool_name: admitted.request.event.tool_name(),
                tool_command: admitted.request.event.command(),
                exact_authorization: Some(&exact),
            },
        )
        .render(policy.injection.unwrap_or(""))
        .map_err(|_| NativeHookResponseAuthorityError)?;
        let boundary = self
            .admit_write_policy(application)
            .map_err(|_| NativeHookResponseAuthorityError)?
            .evaluate_write_boundary(filesystem);
        let outcome = match boundary {
            ResidentHookWritePolicyEvaluation::NoViolation => outcome,
            ResidentHookWritePolicyEvaluation::Deny { reason } => {
                outcome.merge(HookOutcome::Deny(reason))
            }
        };
        let output = outcome.into_output(policy.adapter, admitted.request.routing.event());
        let disclosure = authority.admit_disclosure(&permission_request, &output)?;
        Ok(AdmittedNativeHookResponse {
            observed_at: now,
            output,
            permission: permission.evidence,
            disclosure,
        })
    }
}
