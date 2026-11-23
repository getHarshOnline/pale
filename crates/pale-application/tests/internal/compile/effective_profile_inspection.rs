//! Law tests for application-owned effective-profile classification.

use pale_core::records::{
    Approval, CapabilityBinding, CapabilityIdV1, CapabilityNarrowingV1, CapabilityProvider,
    CapabilityProviderBinding, CapabilityRequirementIdV1, EffectClass, FailurePosture,
    LogicalRepositoryIdV1, OperationSelection, PackageIdV1, PackageResourceIdV1,
    PackageResourceRefV1, ResolvedCapabilityRequirementPolicyV1,
};

use super::{
    EffectiveProfileNarrowingReason, EffectiveProfileOperationExposure,
    EffectiveProfileOperationInspection, EffectiveProfilePolicyBlock,
    EffectiveProfileProviderDisposition, EffectiveProfileRequirementDisposition,
    EffectiveProfileRequirementInspection,
};
use crate::semantic::{
    ProviderUnavailability, ResolvedCapability, ResolvedOperation, ResolvedOperationMechanics,
    ResolvedOperationRecord, ResolvedProviderSelection,
};

fn requirement(provider: ResolvedProviderSelection) -> ResolvedCapability {
    ResolvedCapability {
        requirement_id: CapabilityRequirementIdV1::parse("inspect")
            .expect("valid requirement identity"),
        description: "Inspect one exact portable disposition.".into(),
        capability_id: CapabilityIdV1::parse("inspection").expect("valid capability identity"),
        recipe: None,
        provider,
        effect_class: EffectClass::Read,
        policy: ResolvedCapabilityRequirementPolicyV1::RequiredBlock,
        failure_posture: FailurePosture::FailClosed,
    }
}

fn inspect<'a>(
    requirement: &'a ResolvedCapability,
    narrowing: Option<&'a CapabilityNarrowingV1>,
) -> EffectiveProfileRequirementInspection<'a> {
    static REPOSITORY: std::sync::LazyLock<LogicalRepositoryIdV1> =
        std::sync::LazyLock::new(|| LogicalRepositoryIdV1::parse("fixture").unwrap());
    static OPERATION: std::sync::LazyLock<PackageResourceRefV1> =
        std::sync::LazyLock::new(|| PackageResourceRefV1 {
            package_id: PackageIdV1::parse("engine").unwrap(),
            resource_id: PackageResourceIdV1::parse("op-inspect").unwrap(),
        });
    EffectiveProfileRequirementInspection {
        requirement,
        capability_narrowing: narrowing,
        repository_id: &REPOSITORY,
        operation_id: &OPERATION,
    }
}

fn narrowing(
    permitted_capability: bool,
    forbidden_capability: bool,
    permitted_effect: bool,
    forbidden_effect: bool,
) -> CapabilityNarrowingV1 {
    CapabilityNarrowingV1 {
        permitted_capabilities: permitted_capability
            .then(|| CapabilityIdV1::parse("inspection").expect("valid capability identity"))
            .into_iter()
            .collect(),
        forbidden_capabilities: forbidden_capability
            .then(|| CapabilityIdV1::parse("inspection").expect("valid capability identity"))
            .into_iter()
            .collect(),
        permitted_effects: permitted_effect
            .then_some(EffectClass::Read)
            .into_iter()
            .collect(),
        forbidden_effects: forbidden_effect
            .then_some(EffectClass::Read)
            .into_iter()
            .collect(),
    }
}

#[test]
fn requirement_disposition_does_not_collapse_provider_or_native_state() {
    let bound = requirement(ResolvedProviderSelection::Bound(CapabilityBinding {
        effect_class: EffectClass::Read,
        approval: Approval::None,
        tool_contract: None,
        implementation: CapabilityProviderBinding::NativeRust {
            target: "inspection".into(),
        },
        note: None,
    }));
    assert_eq!(
        inspect(&bound, None).disposition(),
        EffectiveProfileRequirementDisposition::Satisfied
    );
    assert_eq!(
        inspect(&bound, None).provider(),
        EffectiveProfileProviderDisposition::Bound(CapabilityProvider::NativeRust)
    );

    let cases = [
        (
            ProviderUnavailability::MissingBinding,
            EffectiveProfileRequirementDisposition::ProviderBindingAbsent,
        ),
        (
            ProviderUnavailability::EffectMismatch,
            EffectiveProfileRequirementDisposition::PolicyBlocked(
                EffectiveProfilePolicyBlock::ProviderEffectMismatch,
            ),
        ),
        (
            ProviderUnavailability::RecipeContractMismatch,
            EffectiveProfileRequirementDisposition::PolicyBlocked(
                EffectiveProfilePolicyBlock::RecipeContractMismatch,
            ),
        ),
    ];
    for (unavailability, expected) in cases {
        let unavailable = requirement(ResolvedProviderSelection::Unbound(unavailability));
        assert_eq!(inspect(&unavailable, None).disposition(), expected);
        assert_eq!(
            inspect(&unavailable, None).provider(),
            EffectiveProfileProviderDisposition::Unbound(unavailability)
        );
    }
}

#[test]
fn narrowing_preserves_positive_absence_and_explicit_forbidden_relations() {
    let unavailable = requirement(ResolvedProviderSelection::Unbound(
        ProviderUnavailability::ExcludedByNarrowing,
    ));
    let cases = [
        (
            narrowing(false, false, true, false),
            EffectiveProfileRequirementDisposition::Narrowed(
                EffectiveProfileNarrowingReason::CapabilityNotPermitted,
            ),
        ),
        (
            narrowing(true, false, false, false),
            EffectiveProfileRequirementDisposition::Narrowed(
                EffectiveProfileNarrowingReason::EffectNotPermitted,
            ),
        ),
        (
            narrowing(false, false, false, false),
            EffectiveProfileRequirementDisposition::Narrowed(
                EffectiveProfileNarrowingReason::CapabilityAndEffectNotPermitted,
            ),
        ),
        (
            narrowing(true, true, true, false),
            EffectiveProfileRequirementDisposition::PolicyBlocked(
                EffectiveProfilePolicyBlock::CapabilityForbidden,
            ),
        ),
        (
            narrowing(true, false, true, true),
            EffectiveProfileRequirementDisposition::PolicyBlocked(
                EffectiveProfilePolicyBlock::EffectForbidden,
            ),
        ),
        (
            narrowing(true, true, true, true),
            EffectiveProfileRequirementDisposition::PolicyBlocked(
                EffectiveProfilePolicyBlock::CapabilityAndEffectForbidden,
            ),
        ),
    ];

    for (narrowing, expected) in cases {
        let inspection = inspect(&unavailable, Some(&narrowing));
        assert_eq!(inspection.disposition(), expected);
        assert_eq!(
            inspection.provider(),
            EffectiveProfileProviderDisposition::Unbound(
                ProviderUnavailability::ExcludedByNarrowing
            )
        );
    }
}

fn operation(mechanics: ResolvedOperationMechanics) -> ResolvedOperation {
    ResolvedOperation {
        record: ResolvedOperationRecord {
            id: PackageResourceRefV1 {
                package_id: PackageIdV1::parse("engine").expect("valid Package identity"),
                resource_id: PackageResourceIdV1::parse("op-inspect")
                    .expect("valid operation identity"),
            },
            description: "Inspect one profile.".into(),
            roles: Vec::new(),
            outputs: Vec::new(),
        },
        body: String::new().into(),
        required_doctrine: String::new().into(),
        body_sources: Vec::new(),
        doctrine_ids: Vec::new(),
        mechanics,
    }
}

#[test]
fn operation_exposure_is_three_state_and_requirement_cardinality_is_independent() {
    let required = requirement(ResolvedProviderSelection::Unbound(
        ProviderUnavailability::MissingBinding,
    ));
    let operations = [
        (
            operation(ResolvedOperationMechanics::Unbound),
            EffectiveProfileOperationExposure::Unbound,
            0,
        ),
        (
            operation(ResolvedOperationMechanics::SemanticOnly {
                selection: OperationSelection::Internal,
            }),
            EffectiveProfileOperationExposure::SemanticOnly,
            0,
        ),
        (
            operation(ResolvedOperationMechanics::CapabilityBearing {
                selection: OperationSelection::Internal,
                capabilities: vec![required],
            }),
            EffectiveProfileOperationExposure::CapabilityBearing,
            1,
        ),
    ];

    let repository_id = LogicalRepositoryIdV1::parse("fixture").unwrap();
    for (operation, exposure, requirement_count) in &operations {
        let inspection = EffectiveProfileOperationInspection {
            operation,
            capability_narrowing: None,
            repository_id: &repository_id,
        };
        assert_eq!(inspection.exposure(), *exposure);
        assert_eq!(inspection.requirements().len(), *requirement_count);
    }
}
