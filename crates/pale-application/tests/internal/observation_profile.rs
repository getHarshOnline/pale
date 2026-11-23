//! Primary-row association and raw-work bounds, independent of SDK section layout.

use std::cell::Cell;

use pale_core::records::{
    CapabilityIdV1, CapabilityRequirementIdV1, ControlIncarnationIdV1, ControlStoreIdV1,
    ExperienceCompositionPlanDigest, InstanceDigest, LogicalRepositoryIdV1, PackageIdV1,
    PackageResourceIdV1, PackageResourceRefV1, ProfileDigest,
};
use pale_ports::AuthorityStoreCoordinate;

use super::*;

fn requirement(
    repository: &str,
    operation: &str,
    local_requirement: &str,
    capability: &str,
) -> ObservationSubject {
    ObservationSubject::CapabilityRequirement {
        repository_id: LogicalRepositoryIdV1::parse(repository).unwrap(),
        operation_id: PackageResourceRefV1 {
            package_id: PackageIdV1::parse("fixture").unwrap(),
            resource_id: PackageResourceIdV1::parse(operation).unwrap(),
        },
        requirement_id: CapabilityRequirementIdV1::parse(local_requirement).unwrap(),
        capability_id: CapabilityIdV1::parse(capability).unwrap(),
    }
}

#[test]
fn full_requirement_key_and_raw_visits_survive_filtered_windows() {
    let exact = requirement("website", "publish", "required", "publication");
    let other_repository = requirement("notes", "publish", "required", "publication");
    let other_operation = requirement("website", "review", "required", "publication");
    let other_requirement = requirement("website", "publish", "optional", "publication");
    let other_capability = requirement("website", "publish", "required", "inspection");
    let rows = [
        None,
        Some(&other_repository),
        Some(&other_operation),
        Some(&other_requirement),
        Some(&other_capability),
        Some(&exact),
        Some(&exact),
    ];
    let visited = Cell::new(0);
    let first = select_rows(
        &exact,
        rows.into_iter().inspect(|_| visited.set(visited.get() + 1)),
        5,
        1,
    );
    assert_eq!(first.mapping, ObservationProfileMapping::PrimaryRows);
    assert!(first.selected.is_empty());
    assert_eq!((first.inspected, visited.get()), (5, 5));
    let second = select_rows(
        &exact,
        rows[5..]
            .iter()
            .copied()
            .inspect(|_| visited.set(visited.get() + 1)),
        5,
        1,
    );
    assert_eq!(second.selected, vec![0]);
    assert_eq!((second.inspected, visited.get()), (1, 6));
    let third = select_rows(&exact, rows[6..].iter().copied(), 5, 5);
    assert_eq!(third.selected, vec![0]);
    assert_eq!(third.inspected, 1);
}

#[test]
fn no_primary_mapping_does_not_acquire_a_row_from_equal_provenance() {
    let subject = ObservationSubject::ResidentProcess;
    let selected = select_rows(&subject, [Some(&subject), None], 2, 2);
    assert_eq!(selected.mapping, ObservationProfileMapping::NoPrimaryRows);
    assert!(selected.selected.is_empty());
    assert_eq!(selected.inspected, 2);
}

#[test]
fn profile_association_requires_all_three_generation_coordinates() {
    let digest = |seed: char| std::iter::repeat_n(seed, 64).collect::<String>();
    let profile = ProfileDigest::parse(digest('1')).unwrap();
    let instance = InstanceDigest::parse(digest('2')).unwrap();
    let composition = ExperienceCompositionPlanDigest::parse(digest('3')).unwrap();
    let generation = ObservationGeneration::EffectiveProfile {
        profile_digest: profile.clone(),
        instance_digest: instance.clone(),
        composition_plan_digest: composition.clone(),
    };

    assert!(profile_coordinates_match(
        &generation,
        &profile,
        &instance,
        &composition
    ));
    assert!(!profile_coordinates_match(
        &generation,
        &ProfileDigest::parse(digest('4')).unwrap(),
        &instance,
        &composition
    ));
    assert!(!profile_coordinates_match(
        &generation,
        &profile,
        &InstanceDigest::parse(digest('5')).unwrap(),
        &composition
    ));
    assert!(!profile_coordinates_match(
        &generation,
        &profile,
        &instance,
        &ExperienceCompositionPlanDigest::parse(digest('6')).unwrap()
    ));

    let resident = ObservationGeneration::Resident {
        authority_store: AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("control-store").unwrap(),
            ControlIncarnationIdV1::parse("incarnation").unwrap(),
        ),
        active_generation: None,
    };
    assert!(!profile_coordinates_match(
        &resident,
        &profile,
        &instance,
        &composition
    ));
}
