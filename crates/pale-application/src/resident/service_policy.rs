//! Admitted policy for the public resident frame service.
//!
//! A deployment constructs this value before it accepts a frame. The
//! constructor joins an explicit service policy to one operation retained by
//! an admitted compiled generation. Request bytes cannot construct or modify
//! the projection.

use pale_core::records::{
    ActivationFenceDigest, BoundedResidentTextV1, CapabilityIdV1, CommonRequestFederationClaimV1,
    CommonRequestRouteV1, CommonRequestScopeV1, ExecutionOwnerFenceV1, ExpectedPreconditionV1,
    PackageResourceRefV1, RequestCallerV1, ResidentRecordRefV1, ResidentWorkspaceIdV1, SchemaId,
    SigningAuthorityBindingV1,
};

use super::{
    process_host::current_request::{
        ResidentRetainedCommonRequestPolicy, ResidentRetainedRequestExecutionFence,
    },
    AdmittedCompiledGeneration,
};

/// Exact operation and request policy selected before resident ingress.
///
/// The caller supplies deployment policy, not an incoming request. The
/// constructor proves that the selected operation belongs to `generation` and
/// binds the resulting projection to that generation's digest.
pub struct ResidentServicePolicyInput {
    /// Repository that owns the selected operation.
    pub repository_id: pale_core::records::LogicalRepositoryIdV1,
    /// Contract used by the request body.
    pub body_schema_id: SchemaId,
    /// Digest of the exact request-body contract selected by deployment policy.
    pub body_schema_sha256: pale_core::records::SchemaDigest,
    /// Caller identity and signer coordinate admitted by deployment policy.
    pub caller: RequestCallerV1,
    /// Exact authority regime for the caller signature.
    pub signing_authority: SigningAuthorityBindingV1,
    /// Exact admitted route and contract-set coordinate.
    pub route: CommonRequestRouteV1,
    /// Optional execution-owner fence retained by the service.
    pub execution_owner_fence: Option<ResidentServiceExecutionOwnerFence>,
    /// Optional admitted federation coordinate.
    pub federation_claim: Option<CommonRequestFederationClaimV1>,
    /// Workspace governed by this policy.
    pub workspace_id: ResidentWorkspaceIdV1,
    /// Capability admitted for this frame journey.
    pub capability_id: CapabilityIdV1,
    /// Operation retained by the admitted generation.
    pub operation: PackageResourceRefV1,
    /// Exact request scope selected by deployment policy.
    pub scope: CommonRequestScopeV1,
    /// Closed risk classification selected by deployment policy.
    pub risk_class: BoundedResidentTextV1,
    /// Canonically ordered resource constraints.
    pub resource_constraints: Vec<BoundedResidentTextV1>,
    /// Optional immutable state precondition.
    pub expected_precondition: Option<ExpectedPreconditionV1>,
    /// Maximum lifetime accepted for one caller-selected request window.
    pub maximum_request_lifetime_seconds: u64,
    /// Optional activation fence required by this service policy.
    pub fence_token: Option<ActivationFenceDigest>,
}

/// Exact admitted execution fence retained by service policy.
pub struct ResidentServiceExecutionOwnerFence {
    /// Immutable fence-record reference.
    pub fence_ref: ResidentRecordRefV1,
    /// Admitted fence body.
    pub fence: ExecutionOwnerFenceV1,
}

/// An admitted, generation-bound policy for resident frame handling.
///
/// The fields remain opaque so callers cannot reuse the projection as a
/// request template or alter policy between admission and frame handling.
#[derive(Clone)]
pub struct ResidentServicePolicyProjection {
    policy: ResidentRetainedCommonRequestPolicy,
}

/// Why a resident service policy could not be admitted.
#[derive(Debug, thiserror::Error)]
pub enum ResidentServicePolicyError {
    /// The selected operation is absent from the admitted generation.
    #[error("resident service operation is absent from the admitted generation")]
    OperationUnavailable,
    /// The selected operation has no admitted private binding.
    #[error("resident service operation is not bound in the admitted generation")]
    OperationUnbound,
    /// The public request-envelope contract is absent from the embedded bundle.
    #[error(transparent)]
    Contract(#[from] pale_contracts::SchemaError),
    /// The request lifetime ceiling is zero.
    #[error("resident service request lifetime must be positive")]
    RequestLifetime,
}

impl ResidentServicePolicyProjection {
    /// Admit one deployment policy against an exact compiled generation.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is absent or unbound, the public
    /// request-envelope contract is unavailable, or the request lifetime
    /// ceiling is zero.
    pub fn admit(
        generation: &AdmittedCompiledGeneration,
        input: ResidentServicePolicyInput,
    ) -> Result<Self, ResidentServicePolicyError> {
        let operation = generation
            .operation(&input.repository_id, &input.operation)
            .ok_or(ResidentServicePolicyError::OperationUnavailable)?;
        if !operation.mechanics.is_bound() {
            return Err(ResidentServicePolicyError::OperationUnbound);
        }
        if input.maximum_request_lifetime_seconds == 0 {
            return Err(ResidentServicePolicyError::RequestLifetime);
        }
        let envelope = pale_contracts::record_schema_identity(
            pale_core::records::COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID,
        )?;
        Ok(Self {
            policy: ResidentRetainedCommonRequestPolicy {
                expected_schema_sha256: envelope.schema_sha256,
                expected_body_schema_id: input.body_schema_id,
                expected_body_schema_sha256: input.body_schema_sha256,
                caller: input.caller,
                signing_authority: input.signing_authority,
                route: input.route,
                operation_target: None,
                execution_owner_fence: input.execution_owner_fence.map(|value| {
                    ResidentRetainedRequestExecutionFence {
                        fence_ref: value.fence_ref,
                        fence: value.fence,
                    }
                }),
                federation_claim: input.federation_claim,
                workspace_id: input.workspace_id,
                capability_id: input.capability_id,
                operation: input.operation,
                scope: input.scope,
                risk_class: input.risk_class,
                resource_constraints: input.resource_constraints,
                expected_precondition: input.expected_precondition,
                maximum_request_lifetime_seconds: input.maximum_request_lifetime_seconds,
                generation: Some(generation.record().compiled_generation_digest().clone()),
                fence_token: input.fence_token,
            },
        })
    }

    pub(in crate::resident) fn into_inner(self) -> ResidentRetainedCommonRequestPolicy {
        self.policy
    }
}
