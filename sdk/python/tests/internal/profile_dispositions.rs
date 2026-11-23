//! Exact shared profile-disposition projections and required variant reasons.

use pale_sdk::{
    EffectiveProfileNarrowingReason as Narrowing, EffectiveProfileOperationExposure as Exposure,
    EffectiveProfilePolicyBlock as Block, EffectiveProfileRequirementDisposition as Disposition,
};

use crate::{PyOperationExposure, PyRequirementDisposition};

#[test]
fn operation_exposure_preserves_all_three_owner_values() {
    for (source, expected) in [
        (Exposure::Unbound, PyOperationExposure::Unbound),
        (Exposure::SemanticOnly, PyOperationExposure::SemanticOnly),
        (
            Exposure::CapabilityBearing,
            PyOperationExposure::CapabilityBearing,
        ),
    ] {
        assert!(PyOperationExposure::from_api(source) == expected);
    }
}

#[test]
fn requirement_disposition_preserves_kind_and_required_reason() {
    use super::effective_profile_entries::{assert_value, Family};
    use pyo3::prelude::*;

    let mut sources = vec![Disposition::Satisfied, Disposition::ProviderBindingAbsent];
    sources.extend(
        [
            Narrowing::CapabilityNotPermitted,
            Narrowing::EffectNotPermitted,
            Narrowing::CapabilityAndEffectNotPermitted,
        ]
        .into_iter()
        .map(|reason| Disposition::Narrowed { reason }),
    );
    sources.extend(
        [
            Block::CapabilityForbidden,
            Block::EffectForbidden,
            Block::CapabilityAndEffectForbidden,
            Block::ProviderEffectMismatch,
            Block::RecipeContractMismatch,
        ]
        .into_iter()
        .map(|reason| Disposition::Blocked { reason }),
    );

    Python::initialize();
    Python::attach(|py| {
        for source in sources {
            let expected = serde_json::to_value(source).unwrap();
            let actual = PyRequirementDisposition::from_api(source)
                .into_pyobject(py)
                .unwrap();
            assert_value(&actual, &expected, Family::RequirementDisposition);
        }
    });
}
