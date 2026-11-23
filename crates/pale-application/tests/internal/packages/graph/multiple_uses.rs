//! Selection and visibility of distinct uses sharing one Package page.

use super::*;
use pale_core::records::{DoctrinePlacement, PackageIdV1, PackageResourceIdV1};

fn page() -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.shared").unwrap(),
        resource_id: PackageResourceIdV1::parse("review").unwrap(),
    }
}

fn usage(kind: PackageResourceKindV1) -> PackageResourceUse {
    PackageResourceUse::new(page(), kind)
}

fn ceiling() -> PackageResourceRefV1 {
    let mut resource = page();
    resource.resource_id = PackageResourceIdV1::parse("review-policy").unwrap();
    resource
}

fn graph(guide_exposure: PackageResourceExposureV1) -> AdmittedPackageGraph {
    let resources = [
        (
            usage(PackageResourceKindV1::Doctrine),
            AdmittedResourceNode {
                page: None,
                exposure: guide_exposure,
                resource: PackageResource::Doctrine {
                    description: "Review a change".into(),
                    placement: DoctrinePlacement::StandaloneJudgment,
                    extends: Vec::new(),
                    requires: Vec::new(),
                    see_also: Vec::new(),
                    official_sources: Vec::new(),
                },
            },
        ),
        (
            usage(PackageResourceKindV1::Operation),
            AdmittedResourceNode {
                page: None,
                exposure: PackageResourceExposureV1::Exported,
                resource: PackageResource::Operation {
                    description: "Review a change".into(),
                    extends: Vec::new(),
                    doctrine: vec![page()],
                    roles: vec![page()],
                    outputs: Vec::new(),
                    requirements: vec![AdmittedOperationRequirement {
                        id: CapabilityRequirementIdV1::parse("source").unwrap(),
                        description: "Read the source being reviewed.".into(),
                        capability: pale_core::records::CapabilityIdV1::parse("repository.read")
                            .unwrap(),
                        recipe: None,
                        effect_class: EffectClass::Read,
                        policy: AdmittedRequirementPolicy::RequiredBlock,
                        failure_posture: FailurePosture::FailClosed,
                    }],
                    capability_policy: Some(ceiling()),
                },
            },
        ),
        (
            usage(PackageResourceKindV1::Role),
            AdmittedResourceNode {
                page: None,
                exposure: PackageResourceExposureV1::Internal,
                resource: PackageResource::Role {
                    description: "Review a change".into(),
                    extends: Vec::new(),
                    inherited_lenses: Vec::new(),
                    doctrine: vec![page()],
                    stacks: Vec::new(),
                },
            },
        ),
        (
            PackageResourceUse::new(ceiling(), PackageResourceKindV1::CapabilityPolicy),
            AdmittedResourceNode {
                page: None,
                exposure: PackageResourceExposureV1::Internal,
                resource: PackageResource::CapabilityPolicy,
            },
        ),
    ]
    .into_iter()
    .collect();
    AdmittedPackageGraph {
        releases: Vec::new(),
        resources,
        bodies: BTreeMap::from([(
            page(),
            AdmittedMarkdown::from_text(page(), "# Review\n\nOne shared body.\n"),
        )]),
        capability_policies: BTreeMap::from([(
            ceiling(),
            CapabilityPolicy {
                permitted_capabilities: vec![pale_core::records::CapabilityIdV1::parse(
                    "repository.read",
                )
                .unwrap()],
                forbidden_capabilities: Vec::new(),
                permitted_effects: vec![EffectClass::Read],
                forbidden_effects: Vec::new(),
            },
        )]),
        recipes: BTreeMap::new(),
        governing: BTreeMap::new(),
    }
}

fn selection(kind: PackageResourceKindV1) -> SemanticSelection {
    let mut selection = SemanticSelection::default();
    let selector = PackageResourceSelectorV1::Exact { resource: page() };
    match kind {
        PackageResourceKindV1::Doctrine => selection.doctrine.push(selector),
        PackageResourceKindV1::Operation => selection.operations.push(selector),
        _ => unreachable!(),
    }
    selection
}

#[test]
fn selecting_a_guide_does_not_select_its_task_or_role() {
    let selected = graph(PackageResourceExposureV1::Exported)
        .select_and_seal(
            &selection(PackageResourceKindV1::Doctrine),
            &BTreeMap::new(),
        )
        .unwrap();
    assert_eq!(selected.resource_count(), 1);
    assert!(selected
        .resources()
        .contains_key(&usage(PackageResourceKindV1::Doctrine)));
    let parts = selected.into_resident_parts();
    assert_eq!(parts.bodies.len(), 1);
    assert!(parts.capability_policies.is_empty());
    assert!(parts.recipes.is_empty());
}

#[test]
fn selecting_a_task_closes_each_typed_dependency_and_retains_the_body_once() {
    let selected = graph(PackageResourceExposureV1::Internal)
        .select_and_seal(
            &selection(PackageResourceKindV1::Operation),
            &BTreeMap::new(),
        )
        .unwrap();
    assert_eq!(
        selected
            .resources()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            usage(PackageResourceKindV1::Doctrine),
            usage(PackageResourceKindV1::Operation),
            usage(PackageResourceKindV1::Role),
            PackageResourceUse::new(ceiling(), PackageResourceKindV1::CapabilityPolicy),
        ])
    );
    let parts = selected.into_resident_parts();
    assert_eq!(parts.bodies.len(), 1);
    assert_eq!(parts.capability_policies.len(), 1);
    assert!(parts.capability_policies.contains_key(&ceiling()));
    assert_eq!(
        parts.bodies[&page()].text(),
        "# Review\n\nOne shared body.\n"
    );
}

#[test]
fn exported_task_does_not_export_the_guides_selection() {
    let graph = graph(PackageResourceExposureV1::Internal);
    let error = graph
        .select_exact(
            &page(),
            PackageResourceKindV1::Doctrine,
            &mut BTreeSet::new(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        PackageError::InvalidResourceSelector {
            reason: ResourceSelectorViolation::InternalResource,
            ..
        }
    ));
    let mut diagnostics = PackageDiagnosticAccumulator::default();
    let mut selected = BTreeSet::new();
    graph.select_field(
        &[PackageResourceSelectorV1::AllFromPackage {
            package_id: page().package_id,
        }],
        PackageResourceKindV1::Doctrine,
        &mut selected,
        &mut diagnostics,
    );
    diagnostics.finish(()).unwrap();
    assert!(selected.is_empty());
}

#[test]
fn wrong_kind_is_distinct_from_a_missing_page() {
    let graph = graph(PackageResourceExposureV1::Exported);
    assert!(matches!(
        graph.select_exact(&page(), PackageResourceKindV1::Stack, &mut BTreeSet::new()),
        Err(PackageError::InvalidResourceSelector {
            reason: ResourceSelectorViolation::WrongKind,
            ..
        })
    ));
    let mut missing = page();
    missing.resource_id = PackageResourceIdV1::parse("missing").unwrap();
    assert!(matches!(
        graph.select_exact(&missing, PackageResourceKindV1::Stack, &mut BTreeSet::new()),
        Err(PackageError::InvalidResourceSelector {
            reason: ResourceSelectorViolation::MissingResource,
            ..
        })
    ));
}

#[test]
fn governing_rows_activate_only_for_the_selected_use_of_a_shared_page() {
    let mut graph = graph(PackageResourceExposureV1::Exported);
    let mut contributor = page();
    contributor.resource_id = PackageResourceIdV1::parse("governing").unwrap();
    graph.resources.insert(
        PackageResourceUse::new(contributor.clone(), PackageResourceKindV1::GoverningOrder),
        AdmittedResourceNode {
            page: None,
            exposure: PackageResourceExposureV1::Internal,
            resource: PackageResource::GoverningOrder,
        },
    );
    graph.governing.insert(
        contributor.clone(),
        ResolvedGoverningContributionV1 {
            constraints: Vec::new(),
            obligations: vec![ResolvedGoverningObligationV1 {
                scope: usage(PackageResourceKindV1::Operation),
                subject: usage(PackageResourceKindV1::Operation),
                obligation: usage(PackageResourceKindV1::Role),
                contributor,
            }],
            exclusive_authority: Vec::new(),
            narrowing: Vec::new(),
            order: Vec::new(),
        },
    );
    let guide = graph
        .clone()
        .select_and_seal(
            &selection(PackageResourceKindV1::Doctrine),
            &BTreeMap::new(),
        )
        .unwrap();
    assert_eq!(guide.resource_count(), 1);
    assert!(guide.profile_governing_order().obligations().is_empty());

    let task = graph
        .select_and_seal(
            &selection(PackageResourceKindV1::Operation),
            &BTreeMap::new(),
        )
        .unwrap();
    let obligations = task.profile_governing_order().obligations();
    assert_eq!(obligations.len(), 1);
    assert_eq!(
        obligations[0].scope,
        usage(PackageResourceKindV1::Operation)
    );
    assert_eq!(
        obligations[0].obligation,
        usage(PackageResourceKindV1::Role)
    );
}
