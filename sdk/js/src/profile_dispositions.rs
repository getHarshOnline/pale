//! Project configured operation exposure and requirement outcomes into Node.
//!
//! These shared values preserve the SDK's narrowing and blocking reasons.
//! They describe configuration, not observed native readiness.

use napi_derive::napi;
use pale_sdk::{
    EffectiveProfileNarrowingReason, EffectiveProfileOperationExposure,
    EffectiveProfilePolicyBlock, EffectiveProfileRequirementDisposition,
};

/// Exact three-state operation exposure.
#[napi(js_name = "OperationExposure")]
pub enum OperationExposure {
    /// No private operation binding exists.
    Unbound,
    /// A binding exists with zero Package-owned requirements.
    SemanticOnly,
    /// A binding exists with Package-owned requirements.
    CapabilityBearing,
}

impl OperationExposure {
    pub(crate) fn from_api(value: EffectiveProfileOperationExposure) -> Self {
        match value {
            EffectiveProfileOperationExposure::Unbound => Self::Unbound,
            EffectiveProfileOperationExposure::SemanticOnly => Self::SemanticOnly,
            EffectiveProfileOperationExposure::CapabilityBearing => Self::CapabilityBearing,
        }
    }
}

/// Positive-set reason for narrowing.
#[napi(js_name = "NarrowingReason")]
pub enum NarrowingReason {
    /// Capability omitted.
    CapabilityNotPermitted,
    /// Effect omitted.
    EffectNotPermitted,
    /// Capability and effect omitted.
    CapabilityAndEffectNotPermitted,
}

impl NarrowingReason {
    fn from_api(value: EffectiveProfileNarrowingReason) -> Self {
        match value {
            EffectiveProfileNarrowingReason::CapabilityNotPermitted => Self::CapabilityNotPermitted,
            EffectiveProfileNarrowingReason::EffectNotPermitted => Self::EffectNotPermitted,
            EffectiveProfileNarrowingReason::CapabilityAndEffectNotPermitted => {
                Self::CapabilityAndEffectNotPermitted
            }
        }
    }
}

/// Exact policy dimension blocking a requirement.
#[napi(js_name = "PolicyBlock")]
pub enum PolicyBlock {
    /// Capability explicitly forbidden.
    CapabilityForbidden,
    /// Effect explicitly forbidden.
    EffectForbidden,
    /// Capability and effect explicitly forbidden.
    CapabilityAndEffectForbidden,
    /// Provider effect differs from the requirement.
    ProviderEffectMismatch,
    /// Provider lacks the required recipe contract.
    RecipeContractMismatch,
}

impl PolicyBlock {
    fn from_api(value: EffectiveProfilePolicyBlock) -> Self {
        match value {
            EffectiveProfilePolicyBlock::CapabilityForbidden => Self::CapabilityForbidden,
            EffectiveProfilePolicyBlock::EffectForbidden => Self::EffectForbidden,
            EffectiveProfilePolicyBlock::CapabilityAndEffectForbidden => {
                Self::CapabilityAndEffectForbidden
            }
            EffectiveProfilePolicyBlock::ProviderEffectMismatch => Self::ProviderEffectMismatch,
            EffectiveProfilePolicyBlock::RecipeContractMismatch => Self::RecipeContractMismatch,
        }
    }
}

/// Portable requirement outcome with the reason required by its literal kind.
/// Configuration does not establish native readiness or authorize effects.
#[napi(
    discriminant = "kind",
    discriminant_case = "kebab-case",
    object_from_js = false
)]
pub enum RequirementDisposition {
    /// A private provider satisfies the portable requirement.
    Satisfied {},
    /// No private provider binding exists.
    ProviderBindingAbsent {},
    /// A positive permitted set omitted the occurrence.
    Narrowed {
        /// Exact omitted policy dimension.
        reason: NarrowingReason,
    },
    /// Explicit policy or a present provider contradicts the requirement.
    Blocked {
        /// Exact blocking dimension.
        reason: PolicyBlock,
    },
}

impl RequirementDisposition {
    pub(crate) fn from_api(value: EffectiveProfileRequirementDisposition) -> Self {
        match value {
            EffectiveProfileRequirementDisposition::Satisfied => Self::Satisfied {},
            EffectiveProfileRequirementDisposition::ProviderBindingAbsent => {
                Self::ProviderBindingAbsent {}
            }
            EffectiveProfileRequirementDisposition::Narrowed { reason } => Self::Narrowed {
                reason: NarrowingReason::from_api(reason),
            },
            EffectiveProfileRequirementDisposition::Blocked { reason } => Self::Blocked {
                reason: PolicyBlock::from_api(reason),
            },
        }
    }
}
