//! Match a correlated native event to one compiled capability within its lease.

use pale_core::records::{
    CapabilityIdV1, CapabilityProviderBinding, CapabilityRequirementIdV1, EffectClass,
};

use super::{CorrelatedResidentHookOperation, ResidentApplication};
use crate::resident::hook::{AdmittedResidentHookWritePolicy, ResidentHookPolicyAdmissionError};
use crate::semantic::ResolvedCapability;

impl CorrelatedResidentHookOperation<'_> {
    /// Select the sealed write boundary within this operation's native mapping.
    ///
    /// The session, operation lease and uniquely matched capability must all
    /// include Write before this policy can find no violation for a proven
    /// write. A successful evaluation still grants no native permission and
    /// does not establish current operation progress or durable evidence.
    ///
    /// # Errors
    /// Rejects an unavailable or ambiguous native mapping, or a policy that
    /// does not match the admitted session's exact generation and route.
    pub fn admit_write_policy<'a>(
        &'a self,
        application: &'a ResidentApplication<'_>,
    ) -> Result<AdmittedResidentHookWritePolicy<'a>, ResidentHookOperationWritePolicyError> {
        let binding = self.admit_native_tool(application)?;
        let mut policy = self.admitted.admit_write_policy(application)?;
        policy.admits_write &= binding.effect_class() == EffectClass::Write;
        Ok(policy)
    }

    /// Match one native tool to this operation's exact compiled provider mapping.
    ///
    /// Matching uses the native tool name, typed input predicates and sealed
    /// command constraints. The model-facing name grants no alias. The selected
    /// capability must match the handle's original request binding, and its
    /// capability and effect must remain within the narrower operation lease.
    /// This proves a compiled association, not provider liveness, current
    /// operation progress, filesystem permission, disclosure or a native effect.
    ///
    /// # Errors
    /// Rejects a missing retained generation or operation, absent/unavailable
    /// provider mapping, a mismatched tool or more than one matching requirement.
    pub fn admit_native_tool<'a>(
        &'a self,
        application: &'a ResidentApplication<'_>,
    ) -> Result<ResidentHookNativeToolBinding<'a>, ResidentHookNativeToolBindingError> {
        use ResidentHookNativeToolBindingError as Error;
        let session = self.admitted.session().lease();
        let lease = self.operation.historical_operation_lease();
        let requested_capability = self.operation.handle().1.request_binding().capability_id();
        let generation = application
            .generation(session.compiled_generation_digest())
            .ok_or(Error::MissingGeneration)?;
        let operation = generation
            .operation(session.repository_id(), lease.operation())
            .ok_or(Error::MissingOperation)?;
        let request = self.admitted.request;
        let event = request.event();
        let name = event.tool_name().ok_or(Error::Unavailable)?;
        let provider = request.routing().identity().provider_binding();
        let mut matching = operation.capabilities().iter().filter(|requirement| {
            if &requirement.capability_id != requested_capability
                || !lease.capabilities().contains(&requirement.capability_id)
                || !lease.effects().contains(&requirement.effect_class)
            {
                return false;
            }
            let Some(binding) = requirement.provider.binding() else {
                return false;
            };
            if binding.effect_class != requirement.effect_class {
                return false;
            }
            let CapabilityProviderBinding::HostNative { provider_tools } = &binding.implementation
            else {
                return false;
            };
            provider_tools.iter().any(|selected| {
                &selected.provider_binding == provider
                    && selected.tools.iter().any(|tool| {
                        crate::host_tool::matches_pre_tool(
                            tool,
                            name,
                            event.tool_input(),
                            event.command(),
                        )
                    })
            })
        });
        let requirement = matching.next().ok_or(Error::Unavailable)?;
        if matching.next().is_some() {
            return Err(Error::Ambiguous);
        }
        Ok(ResidentHookNativeToolBinding { requirement })
    }
}

/// One compiled native-tool association inside a correlated operation's lease.
///
/// This value contains no response or dispatch permission. The operation owner
/// must still check current progress and the selected effect policy before work.
pub struct ResidentHookNativeToolBinding<'a> {
    requirement: &'a ResolvedCapability,
}

impl ResidentHookNativeToolBinding<'_> {
    /// Borrow the unique sealed requirement, including its provider approval policy.
    #[must_use]
    pub const fn requirement(&self) -> &ResolvedCapability {
        self.requirement
    }

    /// Return the unique Package requirement matched by the native event.
    #[must_use]
    pub const fn requirement_id(&self) -> &CapabilityRequirementIdV1 {
        &self.requirement.requirement_id
    }

    /// Return the capability narrowed by the operation lease.
    #[must_use]
    pub const fn capability_id(&self) -> &CapabilityIdV1 {
        &self.requirement.capability_id
    }

    /// Return the matching declared effect; this does not permit that effect.
    #[must_use]
    pub const fn effect_class(&self) -> EffectClass {
        self.requirement.effect_class
    }
}

impl std::fmt::Debug for ResidentHookNativeToolBinding<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ResidentHookNativeToolBinding(<compiled association>)")
    }
}

/// The retained operation cannot uniquely account for this native tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResidentHookNativeToolBindingError {
    /// The session's admitted generation is no longer retained.
    #[error("native hook generation is unavailable")]
    MissingGeneration,
    /// The issued operation is absent from that exact repository and generation.
    #[error("native hook operation is unavailable")]
    MissingOperation,
    /// No selected provider/tool mapping fits within the operation lease.
    #[error("native tool has no matching operation capability")]
    Unavailable,
    /// Multiple capability requirements match the same native occurrence.
    #[error("native tool matches more than one operation capability")]
    Ambiguous,
}

/// A correlated operation could not select its native write policy.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResidentHookOperationWritePolicyError {
    /// The native event has no unique compiled capability within its lease.
    #[error(transparent)]
    NativeTool(#[from] ResidentHookNativeToolBindingError),
    /// The sealed policy does not match the admitted session and event.
    #[error(transparent)]
    Policy(#[from] ResidentHookPolicyAdmissionError),
}
