//! Algebraic laws for operation-scoped provider readiness.

use pale_core::records::{
    ArtifactDigest, CapabilityIdV1, CapabilityRequirementIdV1, CompiledGenerationDigest,
    EffectClass, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1, ResidentRecordRefV1,
    UnixTimeV1,
};

use super::{super::*, admission::admitted_success_fixture};

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn capability(id: &str) -> CapabilityIdV1 {
    CapabilityIdV1::parse(id).expect("capability id")
}

fn generation(byte: char) -> CompiledGenerationDigest {
    CompiledGenerationDigest::parse(sha(byte)).expect("generation digest")
}

fn operation(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("synthetic-package").expect("package id"),
        resource_id: PackageResourceIdV1::parse(id).expect("operation id"),
    }
}

fn requirement(id: &str) -> CapabilityRequirementIdV1 {
    CapabilityRequirementIdV1::parse(id).expect("requirement id")
}

fn exact_dependencies() -> ProviderInvocationComponent<ProviderInvocationDependencySet> {
    ProviderInvocationComponent::Satisfied(
        ProviderInvocationDependencySet::new(vec![ProviderInvocationDependencyCoordinate::new(
            ResidentRecordRefV1::parse("synthetic-dependency").expect("dependency ref"),
            ArtifactDigest::parse(sha('1')).expect("dependency digest"),
        )])
        .expect("consistent dependency set"),
    )
}

fn readiness_for(
    operation_id: &str,
    generation_seed: char,
    valid_until: u64,
    checked_at: u64,
) -> ProviderInvocationOperationReadiness {
    let (observation, binding) =
        admitted_success_fixture(operation_id, generation_seed, valid_until);
    let coordinate = observation.coordinate().clone();
    let capability = ProviderInvocationCapabilityReadiness::from_admitted_success(
        requirement("read-owner-state"),
        EffectClass::Read,
        &binding,
        [EffectClass::Read],
        exact_dependencies(),
        &observation,
        UnixTimeV1::new(checked_at).expect("check time"),
    )
    .expect("same-coordinate readiness admits");
    ProviderInvocationOperationReadiness::derive(
        coordinate.compiled_generation_digest().clone(),
        coordinate.operation().clone(),
        ProviderInvocationOperationExposure::CapabilityBearing(
            ProviderInvocationCapabilitySet::new(vec![capability]).expect("capability set"),
        ),
    )
    .expect("requirement belongs to the exact operation")
}

#[test]
fn readiness_meet_is_commutative_associative_idempotent_and_has_ready_identity() {
    use ProviderInvocationReadinessState::{Blocked, Ready, Stale, Unavailable, Unknown};
    let states = [Ready, Unknown, Stale, Unavailable, Blocked];
    for a in states {
        assert_eq!(a.meet(a), a);
        assert_eq!(a.meet(Ready), a);
        for b in states {
            assert_eq!(a.meet(b), b.meet(a));
            for c in states {
                assert_eq!(a.meet(b).meet(c), a.meet(b.meet(c)));
            }
        }
    }
}

#[test]
fn semantic_only_is_ready_without_any_provider_coordinate() {
    let readiness = ProviderInvocationOperationReadiness::derive(
        generation('a'),
        operation("semantic-only"),
        ProviderInvocationOperationExposure::SemanticOnly,
    )
    .expect("semantic-only has no provider requirement join");

    assert_eq!(readiness.state(), ProviderInvocationReadinessState::Ready);
    assert!(matches!(
        readiness.exposure(),
        ProviderInvocationOperationExposure::SemanticOnly
    ));
}

#[test]
fn unbound_and_provider_binding_absent_remain_distinct() {
    let generation = generation('a');
    let operation = operation("coordinate-owner");
    let unbound = ProviderInvocationOperationReadiness::derive(
        generation.clone(),
        operation.clone(),
        ProviderInvocationOperationExposure::Unbound,
    )
    .expect("unbound has no requirement join");
    let absent = ProviderInvocationCapabilityReadiness::from_portable_disposition(
        generation.clone(),
        operation.clone(),
        requirement("missing-provider"),
        capability("synthetic.read"),
        crate::compile::EffectiveProfileRequirementDisposition::ProviderBindingAbsent,
    )
    .expect("absent provider is a valid portable disposition");
    let capability_bearing = ProviderInvocationOperationReadiness::derive(
        generation,
        operation,
        ProviderInvocationOperationExposure::CapabilityBearing(
            ProviderInvocationCapabilitySet::new(vec![absent]).expect("requirement set"),
        ),
    )
    .expect("requirement coordinate matches operation");

    assert!(matches!(
        unbound.exposure(),
        ProviderInvocationOperationExposure::Unbound
    ));
    assert!(matches!(
        capability_bearing.exposure(),
        ProviderInvocationOperationExposure::CapabilityBearing(_)
    ));
    assert_eq!(
        unbound.state(),
        ProviderInvocationReadinessState::Unavailable
    );
    assert_eq!(
        capability_bearing.state(),
        ProviderInvocationReadinessState::Unavailable
    );
}

#[test]
fn stale_evidence_closes_only_its_exact_operation() {
    let stale = readiness_for("stale-operation", 'a', 20, 21);
    let sibling = readiness_for("independent-operation", 'a', 20, 20);

    assert_eq!(stale.state(), ProviderInvocationReadinessState::Stale);
    assert_eq!(sibling.state(), ProviderInvocationReadinessState::Ready);
    assert_ne!(stale.operation(), sibling.operation());
}

#[test]
fn unlike_binding_and_required_effect_substitutions_cannot_construct_ready() {
    let (observation, exact_binding) = admitted_success_fixture("protected-operation", 'a', 20);
    let other_binding = crate::provider_binding_fixtures::provider_binding_with_coordinates(
        crate::composition_source_fixtures::participant("synthetic-participant"),
        "other-binding",
        "other-executable",
    );
    let binding_error = ProviderInvocationCapabilityReadiness::from_admitted_success(
        requirement("read-owner-state"),
        EffectClass::Read,
        &other_binding,
        [EffectClass::Read],
        exact_dependencies(),
        &observation,
        UnixTimeV1::new(20).expect("check time"),
    )
    .expect_err("another binding cannot inherit positive observation evidence");
    assert_eq!(
        binding_error,
        ProviderInvocationReadinessError::ProviderCoordinateMismatch
    );

    let effect_error = ProviderInvocationCapabilityReadiness::from_admitted_success(
        requirement("publish-owner-state"),
        EffectClass::Publish,
        &exact_binding,
        [EffectClass::Read],
        exact_dependencies(),
        &observation,
        UnixTimeV1::new(20).expect("check time"),
    )
    .expect_err("required publish effect is outside read-only ceiling");
    assert_eq!(
        effect_error,
        ProviderInvocationReadinessError::RequiredEffectOutsideCeiling
    );
}

#[test]
fn requirement_generation_and_operation_cannot_be_transposed() {
    let (observation, binding) = admitted_success_fixture("owner-operation", 'a', 20);
    let capability = ProviderInvocationCapabilityReadiness::from_admitted_success(
        requirement("read-owner-state"),
        EffectClass::Read,
        &binding,
        [EffectClass::Read],
        exact_dependencies(),
        &observation,
        UnixTimeV1::new(20).expect("check time"),
    )
    .expect("exact requirement readiness");
    let error = ProviderInvocationOperationReadiness::derive(
        CompiledGenerationDigest::parse(sha('b')).expect("other generation"),
        PackageResourceRefV1 {
            package_id: observation.coordinate().operation().package_id.clone(),
            resource_id: pale_core::records::PackageResourceIdV1::parse("other-operation")
                .expect("other operation"),
        },
        ProviderInvocationOperationExposure::CapabilityBearing(
            ProviderInvocationCapabilitySet::new(vec![capability]).expect("capability set"),
        ),
    )
    .expect_err("requirement cannot move between operation/generation coordinates");
    assert_eq!(
        error,
        ProviderInvocationReadinessError::RequirementCoordinateMismatch
    );
}

#[test]
fn dependency_reference_rejects_unlike_bytes_and_deduplicates_exact_repeats() {
    let reference = ResidentRecordRefV1::parse("synthetic-dependency").expect("dependency ref");
    let first = ProviderInvocationDependencyCoordinate::new(
        reference.clone(),
        ArtifactDigest::parse(sha('1')).expect("first digest"),
    );
    let exact_repeat = first.clone();
    let conflict = ProviderInvocationDependencyCoordinate::new(
        reference,
        ArtifactDigest::parse(sha('2')).expect("second digest"),
    );

    let exact = ProviderInvocationDependencySet::new(vec![first.clone(), exact_repeat])
        .expect("exact repeats deduplicate");
    assert_eq!(exact.as_slice(), &[first]);
    assert_eq!(
        ProviderInvocationDependencySet::new(vec![exact.as_slice()[0].clone(), conflict])
            .expect_err("same reference cannot identify unlike bytes"),
        ProviderInvocationReadinessError::DependencyReferenceConflict
    );
}

#[test]
fn admitted_implementation_retains_every_non_substitutable_coordinate_axis() {
    let (observation, _) = admitted_success_fixture("coordinate-operation", 'a', 20);
    let coordinate = observation.coordinate().implementation();

    assert_eq!(coordinate.artifacts().source, "source-a");
    assert_eq!(coordinate.artifacts().build, "build-a");
    assert_eq!(coordinate.artifacts().executable, "executable-a");
    assert_eq!(coordinate.protocol().configuration, "configuration-a");
    assert_eq!(coordinate.protocol().protocol, "protocol-a");
}
