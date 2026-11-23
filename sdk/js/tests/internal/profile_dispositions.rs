//! Exact shared disposition mappings, independent of native readiness.

use crate::{NarrowingReason, OperationExposure, PolicyBlock, RequirementDisposition};
use pale_sdk::{
    EffectiveProfileNarrowingReason as ApiNarrowingReason,
    EffectiveProfileOperationExposure as ApiOperationExposure,
    EffectiveProfilePolicyBlock as ApiPolicyBlock,
    EffectiveProfileRequirementDisposition as ApiRequirementDisposition,
};
use std::mem::discriminant;

#[test]
fn operation_exposure_preserves_all_three_configured_states() {
    for (source, expected) in [
        (ApiOperationExposure::Unbound, OperationExposure::Unbound),
        (
            ApiOperationExposure::SemanticOnly,
            OperationExposure::SemanticOnly,
        ),
        (
            ApiOperationExposure::CapabilityBearing,
            OperationExposure::CapabilityBearing,
        ),
    ] {
        assert_eq!(
            discriminant(&OperationExposure::from_api(source)),
            discriminant(&expected)
        );
    }
}

#[test]
fn complete_requirement_dispositions_keep_reasons_on_their_own_axis() {
    for (source, expected) in [
        (
            ApiRequirementDisposition::Satisfied,
            RequirementDisposition::Satisfied {},
        ),
        (
            ApiRequirementDisposition::ProviderBindingAbsent,
            RequirementDisposition::ProviderBindingAbsent {},
        ),
    ] {
        let projected = RequirementDisposition::from_api(source);
        assert_eq!(discriminant(&projected), discriminant(&expected));
    }
    for (source, expected) in [
        (
            ApiNarrowingReason::CapabilityNotPermitted,
            NarrowingReason::CapabilityNotPermitted,
        ),
        (
            ApiNarrowingReason::EffectNotPermitted,
            NarrowingReason::EffectNotPermitted,
        ),
        (
            ApiNarrowingReason::CapabilityAndEffectNotPermitted,
            NarrowingReason::CapabilityAndEffectNotPermitted,
        ),
    ] {
        let projected = RequirementDisposition::from_api(ApiRequirementDisposition::Narrowed {
            reason: source,
        });
        let RequirementDisposition::Narrowed { reason } = projected else {
            panic!("narrowing changed variant");
        };
        assert_eq!(discriminant(&reason), discriminant(&expected));
    }
    for (source, expected) in [
        (
            ApiPolicyBlock::CapabilityForbidden,
            PolicyBlock::CapabilityForbidden,
        ),
        (
            ApiPolicyBlock::EffectForbidden,
            PolicyBlock::EffectForbidden,
        ),
        (
            ApiPolicyBlock::CapabilityAndEffectForbidden,
            PolicyBlock::CapabilityAndEffectForbidden,
        ),
        (
            ApiPolicyBlock::ProviderEffectMismatch,
            PolicyBlock::ProviderEffectMismatch,
        ),
        (
            ApiPolicyBlock::RecipeContractMismatch,
            PolicyBlock::RecipeContractMismatch,
        ),
    ] {
        let projected =
            RequirementDisposition::from_api(ApiRequirementDisposition::Blocked { reason: source });
        let RequirementDisposition::Blocked { reason } = projected else {
            panic!("blocking changed variant");
        };
        assert_eq!(discriminant(&reason), discriminant(&expected));
    }
}
