//! Responsibility-derivation and heterogeneous-organization law tests.
//!
//! These tests prove source-grammar validation and deterministic pure lowering
//! only. Admitted closure, locks, generations, and materialized-view behavior
//! are B5 concerns and are deliberately not claimed here.

use std::collections::BTreeSet;

use pale_core::records::*;

use crate::workspace::{lower_organizational_translation_v1, lower_responsibility_derivations_v1};

use crate::{organization_composition_fixtures, organization_fixtures, responsibility_fixtures};

use organization_composition_fixtures::{
    axes, empty_organizational_translation as empty_organization_fixture,
};
use organization_fixtures::{
    axes_owned, group_lens_rights, interval, organization,
    organizational_translation as organization_fixture, role, scope, tree,
};

use responsibility_fixtures::{
    directional_service_contract, fixture_id as id, omitted_answer as omitted,
    policy_ceiling as policy, responsibility_derivation as derivation, supplied_answer as supplied,
};

#[test]
fn b3_all_seventeen_questions_are_required_and_omissions_are_attributable() {
    let fixture = derivation("complete", 0, false);
    let fields = [
        "responsibility",
        "native_owner",
        "trigger",
        "placement",
        "structural_objects",
        "request_framer",
        "provider_acceptance",
        "consumer_acceptance",
        "native_result",
        "lawful_silence",
        "publisher_module",
        "adopter_module",
        "ceilings",
        "incumbent",
        "readiness",
        "safe_failure",
        "direct_path",
    ];
    let encoded = serde_json::to_value(fixture).unwrap();
    for field in fields {
        let mut missing = encoded.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<ResponsibilityDerivationV1>(missing).is_err(),
            "{field}"
        );
    }
    let omitted_answer: AttributableAnswerV1<ResponsibilityDirectPathV1> = omitted("direct");
    assert!(omitted_answer.is_omitted());
    assert_eq!(omitted_answer.attribution().owner, id("owner.direct"));
}

#[test]
fn b3_nine_sanitized_transfer_shapes_compile_with_sparse_many_to_many_links() {
    let mut publication = derivation("publication", 3, true);
    publication.residence = vec![
        ResponsibilityResidenceV1 {
            class: ResponsibilityResidenceClassV1::NativeApplication,
            resource: id("resource.publication.native_application"),
            owner: id("native-owner.publication"),
        },
        ResponsibilityResidenceV1 {
            class: ResponsibilityResidenceClassV1::Package,
            resource: id("residence.publication.package"),
            owner: id("owner.publication.package"),
        },
        ResponsibilityResidenceV1 {
            class: ResponsibilityResidenceClassV1::Reference,
            resource: id("residence.publication.reference"),
            owner: id("owner.publication.reference"),
        },
    ];
    publication.native_projections[0].class = ResponsibilityProjectionClassV1::NativeApplication;
    publication.native_projections[0].native_owner = id("native-owner.publication");
    publication.native_projections[0].native_refs =
        vec![id("resource.publication.native_application")];

    let mut growth = derivation("growth", 2, true);
    growth.publisher_module = omitted("growth-publisher");
    growth.adopter_module = omitted("growth-adopter");
    growth.residence[0].class = ResponsibilityResidenceClassV1::Configuration;
    growth.lawful_silence = supplied(
        "growth-silence",
        ResponsibilitySilenceBoundaryV1 {
            silent_surfaces: vec![
                ResponsibilitySilenceClassV1::Communication,
                ResponsibilitySilenceClassV1::RelationshipState,
            ],
        },
    );

    let mut finance = derivation("finance", 1, true);
    finance.lifecycle = ResponsibilityDerivationLifecycleV1::EvidenceGated;
    finance.safe_failure = supplied("finance-failure", ResponsibilitySafeFailureV1::Deny);

    let mut product = derivation("product", 2, false);
    product.residence[0].class = ResponsibilityResidenceClassV1::Package;
    product.residence[0].resource = id("resource.product.package");
    product
        .structural_objects
        .iter_mut()
        .find(|selection| selection.kind == StructuralObjectKindV1::Package)
        .expect("Package question exists")
        .selection = supplied("product-package", id("resource.product.package"));

    let mut research = derivation("research", 1, false);
    research.trigger = supplied(
        "research-trigger",
        ResponsibilityTriggerV1::RecurringCadence {
            cadence: id("cadence.research"),
        },
    );
    research.lifecycle = ResponsibilityDerivationLifecycleV1::ImplementationOpen;
    research.residence[0].class = ResponsibilityResidenceClassV1::Package;
    research.residence[0].resource = id("resource.research.method-package");
    research
        .structural_objects
        .iter_mut()
        .find(|selection| selection.kind == StructuralObjectKindV1::Package)
        .expect("Package question exists")
        .selection = supplied(
        "research-method-package",
        id("resource.research.method-package"),
    );

    let mut signal = derivation("signal", 0, true);
    signal.lifecycle = ResponsibilityDerivationLifecycleV1::Candidate;
    signal.lawful_silence = supplied(
        "signal-silence",
        ResponsibilitySilenceBoundaryV1 {
            silent_surfaces: Vec::new(),
        },
    );

    let mut client = derivation("arms-length-client", 3, true);
    client.residence[0].class = ResponsibilityResidenceClassV1::Reference;
    client.incumbent = omitted("client-incumbent");

    let mut direct_native = derivation("direct-native-app", 0, false);
    direct_native.publisher_module = omitted("direct-native-publisher");
    direct_native.adopter_module = omitted("direct-native-adopter");
    direct_native.lifecycle = ResponsibilityDerivationLifecycleV1::VerifiedCurrent;

    let mut sovereign = derivation("sovereign-new-principal", 1, false);
    sovereign.lifecycle = ResponsibilityDerivationLifecycleV1::OwnerDesiredConfiguration;
    sovereign.incumbent = omitted("sovereign-incumbent");
    if let AttributableAnswerV1::Supplied { value, .. } = &mut sovereign.native_owner {
        value.owner_class = ResponsibilityOwnerClassV1::Principal;
    }

    let mut fixtures = vec![
        publication,
        growth,
        finance,
        product,
        research,
        signal,
        client,
        direct_native,
        sovereign,
    ];
    let responsibilities = fixtures
        .iter()
        .map(|item| item.derivation_id.clone())
        .collect::<Vec<_>>();
    let projections = fixtures
        .iter()
        .flat_map(|item| {
            item.native_projections
                .iter()
                .map(|item| item.projection_id.clone())
        })
        .collect::<Vec<_>>();
    fixtures
        .iter_mut()
        .find(|item| item.derivation_id == id("responsibility.publication"))
        .expect("publication fixture")
        .genealogy_links
        .push(ResponsibilityGenealogyLinkV1 {
            link_id: id("genealogy.cross-case"),
            responsibilities,
            projections,
            occurrences: vec![id("occurrence.request"), id("occurrence.acceptance")],
        });
    let lowered = lower_responsibility_derivations_v1(fixtures).unwrap();
    assert_eq!(lowered.derivations.len(), 9);
    assert!(lowered
        .derivations
        .iter()
        .any(|item| item.native_projections.is_empty()));
    assert!(lowered
        .derivations
        .iter()
        .any(|item| item.native_projections.len() > 1));
    let link = lowered
        .derivations
        .iter()
        .flat_map(|item| &item.genealogy_links)
        .find(|link| link.link_id == id("genealogy.cross-case"))
        .expect("many-to-many link is retained");
    assert_eq!(link.responsibilities.len(), 9);
    assert!(link.projections.len() > 9);
    let publication = lowered
        .derivations
        .iter()
        .find(|item| item.derivation_id == id("responsibility.publication"))
        .unwrap();
    assert_eq!(publication.residence.len(), 3);
    assert!(publication.residence.iter().any(|residence| {
        residence.class == ResponsibilityResidenceClassV1::NativeApplication
            && residence.resource == id("resource.publication.native_application")
    }));
    assert!(publication.native_projections.iter().any(|projection| {
        projection.class == ResponsibilityProjectionClassV1::NativeApplication
            || projection.class == ResponsibilityProjectionClassV1::AccountableWork
    }));
    assert!(publication.structural_objects.iter().any(|selection| {
        selection.kind == StructuralObjectKindV1::NativeApplication
            && matches!(
                &selection.selection,
                AttributableAnswerV1::Supplied { value, .. }
                    if value == &id("resource.publication.native_application")
            )
    }));
    assert!(!publication.direct_path.is_omitted());
    assert!(!publication.publisher_module.is_omitted());
    assert!(!publication.adopter_module.is_omitted());
    let product = lowered
        .derivations
        .iter()
        .find(|item| item.derivation_id == id("responsibility.product"))
        .unwrap();
    assert!(!product.publisher_module.is_omitted());
    assert!(!product.adopter_module.is_omitted());
    assert!(!product.direct_path.is_omitted());
    assert!(product.residence.iter().any(|residence| {
        residence.class == ResponsibilityResidenceClassV1::Package
            && residence.resource == id("resource.product.package")
    }));
    assert!(product.structural_objects.iter().any(|selection| {
        selection.kind == StructuralObjectKindV1::Package
            && matches!(
                &selection.selection,
                AttributableAnswerV1::Supplied { value, .. }
                    if value == &id("resource.product.package")
            )
    }));
    assert!(!product.native_projections.is_empty());
    let research = lowered
        .derivations
        .iter()
        .find(|item| item.derivation_id == id("responsibility.research"))
        .unwrap();
    assert!(!research.publisher_module.is_omitted());
    assert!(!research.adopter_module.is_omitted());
    assert!(!research.direct_path.is_omitted());
    assert!(research.residence.iter().any(|residence| {
        residence.class == ResponsibilityResidenceClassV1::Package
            && residence.resource == id("resource.research.method-package")
    }));
    assert!(research.structural_objects.iter().any(|selection| {
        selection.kind == StructuralObjectKindV1::Package
            && matches!(
                &selection.selection,
                AttributableAnswerV1::Supplied { value, .. }
                    if value == &id("resource.research.method-package")
            )
    }));
    assert!(!research.native_projections.is_empty());
    let direct_native = lowered
        .derivations
        .iter()
        .find(|item| item.derivation_id == id("responsibility.direct-native-app"))
        .unwrap();
    assert!(direct_native.publisher_module.is_omitted());
    assert!(direct_native.adopter_module.is_omitted());
    assert!(!direct_native.direct_path.is_omitted());
    assert!(direct_native.native_projections.is_empty());
    let signal = lowered
        .derivations
        .iter()
        .find(|item| item.derivation_id == id("responsibility.signal"))
        .unwrap();
    let AttributableAnswerV1::Supplied { value: silence, .. } = &signal.lawful_silence else {
        panic!("signal silence is explicitly supplied")
    };
    assert!(silence.silent_surfaces.is_empty());
    assert!(lowered.derivations.iter().all(|item| {
        item.structural_objects
            .iter()
            .find(|selection| selection.kind == StructuralObjectKindV1::LogicalOrganization)
            .is_some_and(|selection| selection.selection.is_omitted())
    }));
}

#[test]
fn b3_directional_service_requires_one_exact_non_reflexive_contract_coordinate() {
    let valid = derivation("service-join", 0, true);
    let contract = directional_service_contract("service-join");
    let structural = valid
        .structural_objects
        .iter()
        .find(|selection| selection.kind == StructuralObjectKindV1::DirectionalServiceContract)
        .expect("directional contract selection");
    assert!(matches!(
        &structural.selection,
        AttributableAnswerV1::Supplied { value, .. } if value == &contract.resource_id()
    ));
    lower_responsibility_derivations_v1(vec![valid.clone()]).unwrap();

    let mut mismatched = valid.clone();
    let selection = mismatched
        .structural_objects
        .iter_mut()
        .find(|selection| selection.kind == StructuralObjectKindV1::DirectionalServiceContract)
        .expect("directional contract selection");
    selection.selection = supplied("mismatch", id("other:service"));
    assert!(matches!(
        lower_responsibility_derivations_v1(vec![mismatched]),
        Err(
            crate::composition::ResponsibilityDerivationError::DirectionalServiceContractMismatch { .. }
        )
    ));

    let mut provider_substitution = valid.clone();
    let AttributableAnswerV1::Supplied {
        value: ResponsibilityPlacementV1::DirectionalService { service_contract },
        ..
    } = &mut provider_substitution.placement
    else {
        panic!("directional placement")
    };
    service_contract.provider_scope = organization_fixtures::scope("substitute", "scope");
    assert_ne!(service_contract, &contract);
    assert!(matches!(
        lower_responsibility_derivations_v1(vec![provider_substitution]),
        Err(
            crate::composition::ResponsibilityDerivationError::DirectionalServiceContractMismatch { .. }
        )
    ));

    let mut consumer_substitution = valid.clone();
    let AttributableAnswerV1::Supplied {
        value: ResponsibilityPlacementV1::DirectionalService { service_contract },
        ..
    } = &mut consumer_substitution.placement
    else {
        panic!("directional placement")
    };
    service_contract.consumer_scope = organization_fixtures::scope("substitute", "scope");
    assert_ne!(service_contract, &contract);
    assert!(matches!(
        lower_responsibility_derivations_v1(vec![consumer_substitution]),
        Err(
            crate::composition::ResponsibilityDerivationError::DirectionalServiceContractMismatch { .. }
        )
    ));

    let mut reversed = valid.clone();
    let AttributableAnswerV1::Supplied {
        value: ResponsibilityPlacementV1::DirectionalService { service_contract },
        ..
    } = &mut reversed.placement
    else {
        panic!("directional placement")
    };
    std::mem::swap(
        &mut service_contract.provider_scope,
        &mut service_contract.consumer_scope,
    );
    assert_ne!(service_contract, &contract);
    assert!(matches!(
        lower_responsibility_derivations_v1(vec![reversed]),
        Err(
            crate::composition::ResponsibilityDerivationError::DirectionalServiceContractMismatch { .. }
        )
    ));

    let mut reflexive = valid;
    let AttributableAnswerV1::Supplied {
        value: ResponsibilityPlacementV1::DirectionalService { service_contract },
        ..
    } = &mut reflexive.placement
    else {
        panic!("directional placement")
    };
    service_contract.consumer_scope = service_contract.provider_scope.clone();
    assert!(matches!(
        lower_responsibility_derivations_v1(vec![reflexive]),
        Err(crate::composition::ResponsibilityDerivationError::ReflexiveDirectionalService { .. })
    ));
}

#[test]
fn b3_structural_selection_never_mints_an_organization() {
    let lowered = lower_responsibility_derivations_v1(vec![derivation("local", 0, false)]).unwrap();
    let organization = lowered.derivations[0]
        .structural_objects
        .iter()
        .find(|item| item.kind == StructuralObjectKindV1::LogicalOrganization)
        .unwrap();
    assert!(organization.selection.is_omitted());
}

#[test]
fn b3_acceptance_and_settlement_coordinates_remain_independently_typed() {
    let lowered = lower_responsibility_derivations_v1(vec![derivation("seams", 1, true)]).unwrap();
    let record = &lowered.derivations[0];
    let encoded = serde_json::to_value(record).unwrap();
    let names = [
        "provider_completion",
        "provider_acceptance",
        "consumer_acceptance",
        "native_result",
        "pale_settlement",
    ];
    let refs = names.map(|name| {
        encoded[name]["value"]["requirement"]["evidence_ref"]
            .as_str()
            .expect("evidence identity serializes as text")
    });
    assert_eq!(refs.into_iter().collect::<BTreeSet<_>>().len(), 5);
}

#[test]
fn b3_candidate_only_inventory_has_exact_token_boundaries() {
    assert!(candidate_only_workspace_language_identifier_token_v1(
        RESPONSIBILITY_DERIVATION_V1_SCHEMA
    )
    .is_none());
    for token in CANDIDATE_ONLY_WORKSPACE_LANGUAGE_IDENTIFIER_TOKENS_V1 {
        let surface = format!("kernel/{token}-abi");
        assert_eq!(
            candidate_only_workspace_language_identifier_token_v1(&surface),
            Some(token)
        );
    }
    assert!(candidate_only_workspace_language_identifier_token_v1("conceptual-model").is_none());
}

#[test]
fn b3_policy_meet_and_effective_state_laws_are_typed_and_deterministic() {
    let left = ResponsibilityCeilingsV1 {
        policy: policy("left"),
        resources: vec![id("resource.left"), id("resource.shared")],
    };
    let mut right_policy = policy("right");
    right_policy.disclosure = DisclosureCeilingV1::Private;
    let right = ResponsibilityCeilingsV1 {
        policy: right_policy,
        resources: vec![id("resource.right"), id("resource.shared")],
    };
    assert_eq!(left.meet(&right), right.meet(&left));
    assert_eq!(left.meet(&left), left);
    assert_eq!(left.meet(&right).resources, vec![id("resource.shared")]);
    assert_eq!(
        EffectiveFactStateV1::Unknown
            .try_meet(EffectiveFactStateV1::Denied)
            .unwrap(),
        EffectiveFactStateV1::Unknown
    );
    assert!(EffectiveFactStateV1::Absent
        .try_meet(EffectiveFactStateV1::Denied)
        .is_err());
}

#[test]
fn b3_duplicate_diagnostic_names_the_actual_collision() {
    let first = derivation("alpha-unique", 0, false);
    let duplicate = derivation("zeta-duplicate", 0, false);
    let error =
        lower_responsibility_derivations_v1(vec![duplicate.clone(), first, duplicate]).unwrap_err();
    assert!(matches!(
        error,
        crate::composition::ResponsibilityDerivationError::Duplicate {
            derivation,
            kind: "derivation identity"
        } if derivation == id("responsibility.zeta-duplicate")
    ));
}

#[test]
fn b3_all_nested_and_residence_bounds_reject_the_first_value_over() {
    let fixture = derivation("nested-boundary", 0, false);
    assert!(super::responsibility::enforce_nested_max(
        MAX_RESPONSIBILITY_NESTED_MEMBERS,
        MAX_RESPONSIBILITY_NESTED_MEMBERS,
        &fixture,
        "test boundary",
    )
    .is_ok());
    assert!(super::responsibility::enforce_nested_max(
        MAX_RESPONSIBILITY_NESTED_MEMBERS + 1,
        MAX_RESPONSIBILITY_NESTED_MEMBERS,
        &fixture,
        "test boundary",
    )
    .is_err());

    let mut derivation_family = (0..MAX_RESPONSIBILITY_DERIVATIONS)
        .map(|index| derivation(&format!("derivation-boundary-{index}"), 0, false))
        .collect::<Vec<_>>();
    assert!(lower_responsibility_derivations_v1(derivation_family.clone()).is_ok());
    derivation_family.push(derivation("derivation-boundary-first-over", 0, false));
    assert!(lower_responsibility_derivations_v1(derivation_family).is_err());

    let mut invalid = fixture.clone();
    invalid
        .structural_objects
        .push(invalid.structural_objects[0].clone());
    assert!(lower_responsibility_derivations_v1(vec![invalid]).is_err());

    let mut invalid = fixture.clone();
    let AttributableAnswerV1::Supplied { value, .. } = &mut invalid.lawful_silence else {
        panic!("supplied silence")
    };
    value.silent_surfaces = vec![
        ResponsibilitySilenceClassV1::Communication,
        ResponsibilitySilenceClassV1::AccountableWork,
        ResponsibilitySilenceClassV1::RelationshipState,
        ResponsibilitySilenceClassV1::Communication,
    ];
    assert!(lower_responsibility_derivations_v1(vec![invalid]).is_err());

    let mut invalid = fixture.clone();
    let AttributableAnswerV1::Supplied { value, .. } = &mut invalid.ceilings else {
        panic!("supplied ceilings")
    };
    value.resources = (0..=MAX_RESPONSIBILITY_NESTED_MEMBERS)
        .map(|index| id(&format!("resource.boundary.r{index}")))
        .collect();
    assert!(lower_responsibility_derivations_v1(vec![invalid]).is_err());

    let mut invalid = fixture.clone();
    let AttributableAnswerV1::Supplied { value, .. } = &mut invalid.ceilings else {
        panic!("supplied ceilings")
    };
    value.policy.rights.permitted = (0..=MAX_WORKSPACE_POLICY_MEMBERS)
        .map(|index| id(&format!("right.boundary.r{index}")))
        .collect();
    assert!(matches!(
        lower_responsibility_derivations_v1(vec![invalid]),
        Err(crate::composition::ResponsibilityDerivationError::InvalidPolicyCeiling { .. })
    ));

    let mut duplicate_policy = fixture.clone();
    let AttributableAnswerV1::Supplied { value, .. } = &mut duplicate_policy.ceilings else {
        panic!("supplied ceilings")
    };
    value.policy.controllers.permitted = vec![id("controller.repeated"), id("controller.repeated")];
    assert!(matches!(
        lower_responsibility_derivations_v1(vec![duplicate_policy]),
        Err(crate::composition::ResponsibilityDerivationError::InvalidPolicyCeiling { .. })
    ));

    let mut invalid = fixture.clone();
    invalid.residence.clear();
    assert!(lower_responsibility_derivations_v1(vec![invalid]).is_err());
    let mut exact = fixture.clone();
    exact.residence = (0..MAX_RESPONSIBILITY_RESIDENCES)
        .map(|index| ResponsibilityResidenceV1 {
            class: ResponsibilityResidenceClassV1::Reference,
            resource: id(&format!("residence.boundary.r{index}")),
            owner: id("owner.boundary"),
        })
        .collect();
    assert!(lower_responsibility_derivations_v1(vec![exact.clone()]).is_ok());
    exact.residence.push(ResponsibilityResidenceV1 {
        class: ResponsibilityResidenceClassV1::Reference,
        resource: id("residence.boundary.first-over"),
        owner: id("owner.boundary"),
    });
    assert!(lower_responsibility_derivations_v1(vec![exact]).is_err());

    let mut projection_over = fixture.clone();
    projection_over.native_projections = vec![ResponsibilityNativeProjectionV1 {
        projection_id: id("projection.boundary"),
        class: ResponsibilityProjectionClassV1::Context,
        native_owner: id("owner.boundary"),
        native_refs: (0..=MAX_RESPONSIBILITY_NESTED_MEMBERS)
            .map(|index| id(&format!("native.boundary.r{index}")))
            .collect(),
        state: EffectiveFactStateV1::Present,
    }];
    assert!(lower_responsibility_derivations_v1(vec![projection_over]).is_err());

    for family in ["responsibilities", "projections", "occurrences"] {
        let mut invalid = fixture.clone();
        invalid.genealogy_links = vec![ResponsibilityGenealogyLinkV1 {
            link_id: id(&format!("genealogy.over.{family}")),
            responsibilities: if family == "responsibilities" {
                vec![invalid.derivation_id.clone(); MAX_RESPONSIBILITY_NESTED_MEMBERS + 1]
            } else {
                vec![invalid.derivation_id.clone()]
            },
            projections: if family == "projections" {
                vec![id("projection.missing"); MAX_RESPONSIBILITY_NESTED_MEMBERS + 1]
            } else {
                Vec::new()
            },
            occurrences: if family == "occurrences" {
                vec![id("occurrence.boundary"); MAX_RESPONSIBILITY_NESTED_MEMBERS + 1]
            } else {
                vec![id("occurrence.boundary")]
            },
        }];
        assert!(lower_responsibility_derivations_v1(vec![invalid]).is_err());
    }
    for empty_family in ["responsibilities", "occurrences"] {
        let mut invalid = fixture.clone();
        invalid.genealogy_links = vec![ResponsibilityGenealogyLinkV1 {
            link_id: id(&format!("genealogy.empty.{empty_family}")),
            responsibilities: if empty_family == "responsibilities" {
                Vec::new()
            } else {
                vec![invalid.derivation_id.clone()]
            },
            projections: Vec::new(),
            occurrences: if empty_family == "occurrences" {
                Vec::new()
            } else {
                vec![id("occurrence.boundary")]
            },
        }];
        assert!(lower_responsibility_derivations_v1(vec![invalid]).is_err());
    }

    let mut projection_family = fixture.clone();
    projection_family.native_projections = (0..MAX_RESPONSIBILITY_NATIVE_PROJECTIONS)
        .map(|index| ResponsibilityNativeProjectionV1 {
            projection_id: id(&format!("projection.family.p{index}")),
            class: ResponsibilityProjectionClassV1::Context,
            native_owner: id("owner.boundary"),
            native_refs: Vec::new(),
            state: EffectiveFactStateV1::Present,
        })
        .collect();
    assert!(lower_responsibility_derivations_v1(vec![projection_family.clone()]).is_ok());
    projection_family
        .native_projections
        .push(ResponsibilityNativeProjectionV1 {
            projection_id: id("projection.family.first-over"),
            class: ResponsibilityProjectionClassV1::Context,
            native_owner: id("owner.boundary"),
            native_refs: Vec::new(),
            state: EffectiveFactStateV1::Present,
        });
    assert!(lower_responsibility_derivations_v1(vec![projection_family]).is_err());

    let mut genealogy_family = fixture;
    genealogy_family.genealogy_links = (0..MAX_RESPONSIBILITY_GENEALOGY_LINKS)
        .map(|index| ResponsibilityGenealogyLinkV1 {
            link_id: id(&format!("genealogy.family.g{index}")),
            responsibilities: vec![genealogy_family.derivation_id.clone()],
            projections: Vec::new(),
            occurrences: vec![id(&format!("occurrence.family.o{index}"))],
        })
        .collect();
    assert!(lower_responsibility_derivations_v1(vec![genealogy_family.clone()]).is_ok());
    genealogy_family
        .genealogy_links
        .push(ResponsibilityGenealogyLinkV1 {
            link_id: id("genealogy.family.first-over"),
            responsibilities: vec![genealogy_family.derivation_id.clone()],
            projections: Vec::new(),
            occurrences: vec![id("occurrence.family.first-over")],
        });
    assert!(lower_responsibility_derivations_v1(vec![genealogy_family]).is_err());
}

#[test]
fn b4_every_nested_bound_accepts_its_boundary_and_rejects_first_over() {
    for (kind, maximum) in [
        ("nested", MAX_ORGANIZATIONAL_NESTED_MEMBERS),
        ("lens rights", MAX_GROUP_LENS_RIGHTS),
        ("inspection coordinates", MAX_INSPECTION_COORDINATES),
        ("inspection states", MAX_INSPECTION_STATES),
    ] {
        assert!(super::organization::enforce_range(kind, maximum, 0, maximum).is_ok());
        assert!(super::organization::enforce_range(kind, maximum + 1, 0, maximum).is_err());
    }
    assert!(super::organization::enforce_range(
        "required nested",
        0,
        1,
        MAX_ORGANIZATIONAL_NESTED_MEMBERS,
    )
    .is_err());

    let mut invalid = organization_fixture();
    invalid.scopes[0].axes.resources =
        vec![id("resource.over"); MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
    assert!(lower_organizational_translation_v1(invalid).is_err());

    let mut invalid = organization_fixture();
    let resolution = invalid.communication_actor_bindings[0].local_resolutions[0].clone();
    invalid.communication_actor_bindings[0].local_resolutions =
        vec![resolution; MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
    assert!(lower_organizational_translation_v1(invalid).is_err());

    let mut invalid = organization_fixture();
    let node = invalid.communication_actor_bindings[0].local_resolutions[0]
        .local_node
        .clone();
    invalid.communication_actor_bindings[0].local_resolutions[0].authorized_closure =
        vec![node; MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
    assert!(lower_organizational_translation_v1(invalid).is_err());

    for family in ["participants", "scopes"] {
        let mut invalid = organization_fixture();
        if family == "participants" {
            invalid.leadership[0].participants =
                vec![role("alpha", "accountable-a"); MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
        } else {
            invalid.leadership[0].affected_scopes =
                vec![scope("alpha", "scope"); MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
        }
        assert!(lower_organizational_translation_v1(invalid).is_err());
    }
    let mut invalid = organization_fixture();
    invalid.leadership[0].affected_scopes.clear();
    assert!(lower_organizational_translation_v1(invalid).is_err());

    for family in ["standards", "blocks"] {
        let mut invalid = organization_fixture();
        if family == "standards" {
            invalid.group_lenses[0].standards =
                vec![id("standard.over"); MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
        } else {
            invalid.group_lenses[0].block_conditions =
                vec![id("condition.over"); MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
        }
        assert!(lower_organizational_translation_v1(invalid).is_err());
    }
    let mut invalid = organization_fixture();
    invalid.group_lenses[0].rights = vec![GroupLensRightV1::Review; MAX_GROUP_LENS_RIGHTS + 1];
    assert!(lower_organizational_translation_v1(invalid).is_err());

    let mut invalid = organization_fixture();
    let assignment = invalid.services[0].assignments[0].clone();
    invalid.services[0].assignments = vec![assignment; MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
    assert!(lower_organizational_translation_v1(invalid).is_err());
    for family in ["stewards", "pods"] {
        let mut invalid = organization_fixture();
        if family == "stewards" {
            invalid.services[0].assignments[0].stewards =
                vec![id("steward.over"); MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
        } else {
            invalid.services[0].assignments[0].pods =
                vec![id("pod.over"); MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
        }
        assert!(lower_organizational_translation_v1(invalid).is_err());
    }

    let mut invalid = organization_fixture();
    invalid.services[0].ceiling.authorities.permitted =
        vec![id("authority.over"); MAX_WORKSPACE_POLICY_MEMBERS + 1];
    assert!(matches!(
        lower_organizational_translation_v1(invalid),
        Err(
            crate::composition::OrganizationalTranslationError::InvalidPolicyCeiling {
                source: InvalidWorkspacePolicyCeilingV1::Limit {
                    axis: WorkspaceCeilingSetAxisV1::Authority,
                    ..
                },
                ..
            }
        )
    ));

    let mut invalid = organization_fixture();
    invalid.services[0]
        .ceiling
        .rights
        .permitted
        .push(id("right.alpha"));
    assert!(matches!(
        lower_organizational_translation_v1(invalid),
        Err(
            crate::composition::OrganizationalTranslationError::InvalidPolicyCeiling {
                source: InvalidWorkspacePolicyCeilingV1::Duplicate {
                    axis: WorkspaceCeilingSetAxisV1::Rights,
                    ..
                },
                ..
            }
        )
    ));

    let mut invalid = organization_fixture();
    invalid.native_projections[0].native_refs =
        vec![id("native.over"); MAX_ORGANIZATIONAL_NESTED_MEMBERS + 1];
    assert!(lower_organizational_translation_v1(invalid).is_err());

    let mut invalid = organization_fixture();
    invalid.inspection.as_mut().unwrap().coordinates =
        vec![InspectionCoordinateClassV1::GroupTree; MAX_INSPECTION_COORDINATES + 1];
    assert!(lower_organizational_translation_v1(invalid).is_err());
    let mut invalid = organization_fixture();
    invalid.inspection.as_mut().unwrap().states =
        vec![EffectiveFactStateV1::Unknown; MAX_INSPECTION_STATES + 1];
    assert!(lower_organizational_translation_v1(invalid).is_err());
    let mut invalid = organization_fixture();
    invalid.inspection.as_mut().unwrap().coordinates.clear();
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_derived_relation_bound_counts_dedicated_facts_before_expansion() {
    let mut boundary = organization_fixture();
    boundary.leadership.clear();
    for index in 3..63 {
        boundary.roles.push(OrganizationalRoleV1 {
            role: role("alpha", &format!("leadership-role-{index}")),
            organization: organization("alpha", "organization"),
            group: None,
            discipline_tree: None,
            role_class: id("role-class.participant"),
            axes: axes_owned("alpha", &format!("leadership-role-{index}")),
            state: EffectiveFactStateV1::Present,
            interval: interval(),
        });
    }
    for index in 3..65 {
        boundary.scopes.push(LogicalScopeV1 {
            scope: scope(&format!("seam-{index}"), "scope"),
            axes: axes_owned(&format!("seam-{index}"), &format!("seam-{index}-scope")),
            state: EffectiveFactStateV1::Present,
            interval: interval(),
        });
    }
    boundary.leadership.push(LeadershipParticipationV1 {
        topic: id("topic.boundary"),
        participants: boundary
            .roles
            .iter()
            .take(62)
            .map(|item| item.role.clone())
            .collect(),
        affected_scopes: boundary
            .scopes
            .iter()
            .take(65)
            .map(|item| item.scope.clone())
            .collect(),
        issuer: id("issuer.boundary"),
        disclosure: DisclosureCeilingV1::Protected,
        state: EffectiveFactStateV1::Present,
        interval: interval(),
    });
    assert!(lower_organizational_translation_v1(boundary.clone()).is_ok());
    boundary.leadership[0]
        .participants
        .push(role("alpha", "leadership-role-62"));
    assert!(matches!(
        lower_organizational_translation_v1(boundary),
        Err(crate::composition::OrganizationalTranslationError::Limit {
            kind: "derived relation"
        })
    ));
}

#[test]
fn b4_case_01_identical_shapes_retain_distinct_axes_and_projections() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let [left, right] = &lowered.source.organizations[..] else {
        panic!("two organizations")
    };
    assert_ne!(left.organization.namespace, right.organization.namespace);
    assert_ne!(left.axes.owner, right.axes.owner);
    assert_ne!(left.axes.controller, right.axes.controller);
    assert_ne!(left.axes.resources, right.axes.resources);
    assert_ne!(
        lowered.source.native_projections[0].projection_id,
        lowered.source.native_projections[1].projection_id
    );
    let mut invalid = organization_fixture();
    invalid.organizations.push(invalid.organizations[0].clone());
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_02_trees_may_differ_or_be_explicitly_absent() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let absent_trees = lowered
        .source
        .discipline_trees
        .iter()
        .filter(|tree| tree.state == EffectiveFactStateV1::Absent)
        .map(|tree| &tree.tree)
        .collect::<BTreeSet<_>>();
    assert_eq!(absent_trees.len(), 1);
    assert!(lowered.source.roles.iter().all(|role| {
        role.discipline_tree
            .as_ref()
            .is_none_or(|tree| !absent_trees.contains(tree))
    }));
    assert!(lowered.source.roles.iter().any(|record_role| {
        record_role.role == role("beta", "reviewer") && record_role.discipline_tree.is_none()
    }));
    let leader = lowered
        .source
        .relations
        .iter()
        .find(|relation| relation.class == OrganizationalRelationClassV1::Leader)
        .expect("alpha vertical selects its leader relation");
    let supporting = lowered
        .source
        .relations
        .iter()
        .find(|relation| relation.class == OrganizationalRelationClassV1::Supporting)
        .expect("beta vertical selects its supporting relation");
    assert_ne!(leader.from, supporting.from);
    assert_ne!(leader.to, supporting.to);
    let mut invalid = organization_fixture();
    invalid.discipline_trees[0].axes.owner = id("owner.beta");
    assert!(matches!(
        lower_organizational_translation_v1(invalid),
        Err(crate::composition::OrganizationalTranslationError::CrossOwnerContainment { .. })
    ));
}

#[test]
fn b4_case_03_coordination_does_not_transfer_native_authority() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let before = axes_owned("alpha", "primary-tree").authority;
    let tree_record = lowered
        .source
        .discipline_trees
        .iter()
        .find(|tree| tree.tree.tree_id == id("primary-tree"))
        .unwrap();
    assert_eq!(tree_record.axes.authority, before);
    let mut invalid = organization_fixture();
    invalid.relations[0].to = OrganizationalNodeRefV1::DisciplineTree {
        tree: tree("beta", "optional-tree"),
    };
    assert!(matches!(
        lower_organizational_translation_v1(invalid),
        Err(crate::composition::OrganizationalTranslationError::NonLocalRelation { .. })
    ));
}

#[test]
fn b4_case_04_native_work_survives_without_a_delivery_structure() {
    let lowered = lower_organizational_translation_v1(empty_organization_fixture()).unwrap();
    assert!(lowered.source.organizations.is_empty());
    let mut invalid = empty_organization_fixture();
    invalid.organizations.push(LogicalOrganizationV1 {
        organization: organization("orphan", "organization"),
        scope: scope("orphan", "scope"),
        axes: axes_owned("orphan", "organization"),
        state: EffectiveFactStateV1::Present,
        interval: interval(),
    });
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_05_one_shape_can_be_internal_and_another_directional() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    assert!(!lowered.source.discipline_trees.is_empty());
    assert_eq!(lowered.source.services.len(), 2);
    let mut invalid = organization_fixture();
    invalid.services[0].consumer_scope = invalid.services[0].provider_scope.clone();
    assert!(matches!(
        lower_organizational_translation_v1(invalid),
        Err(crate::composition::OrganizationalTranslationError::ReflexiveService { .. })
    ));
}

#[test]
fn b4_case_06_leadership_is_topic_scoped_and_nonambient() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let leadership = &lowered.source.leadership[0];
    assert_eq!(leadership.affected_scopes.len(), 2);
    assert!(!leadership
        .affected_scopes
        .contains(&scope("gamma", "scope")));
    assert_eq!(
        lowered
            .derived_relations
            .iter()
            .filter(
                |edge| edge.class == OrganizationalDerivedRelationClassV1::LeadershipParticipation
            )
            .count(),
        4
    );
    let mut invalid = organization_fixture();
    invalid.leadership[0].participants.clear();
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_07_generic_services_do_not_merge_owners_or_contracts() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    assert_eq!(
        lowered.source.services[0].contract_id,
        lowered.source.services[1].contract_id
    );
    assert_ne!(
        lowered.source.services[0].namespace,
        lowered.source.services[1].namespace
    );
    assert_ne!(
        lowered.source.services[0].contract_owner,
        lowered.source.services[1].contract_owner
    );
    let mut invalid = organization_fixture();
    invalid.services.push(invalid.services[0].clone());
    let error = lower_organizational_translation_v1(invalid).unwrap_err();
    assert!(error.to_string().contains("alpha:service.shared"));
}

#[test]
fn b4_case_08_service_assignments_are_many_to_many_without_primary() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let assignments = &lowered.source.services[0].assignments;
    assert_eq!(assignments.len(), 2);
    assert!(assignments
        .iter()
        .all(|item| item.stewards.contains(&id("steward.shared"))));
    assert!(assignments
        .iter()
        .all(|item| item.pods.contains(&id("pod.shared"))));
    let mut invalid = organization_fixture();
    invalid.services[0].assignments[0]
        .stewards
        .push(id("steward.shared"));
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_09_protected_projection_preserves_denial_without_interior_traversal() {
    let mut fixture = organization_fixture();
    fixture.native_projections[1].state = EffectiveFactStateV1::Denied;
    let lowered = lower_organizational_translation_v1(fixture).unwrap();
    let projection = lowered
        .source
        .native_projections
        .iter()
        .find(|item| item.projection_id == id("projection.beta.relationship"))
        .unwrap();
    assert_eq!(projection.state, EffectiveFactStateV1::Denied);
    assert!(projection.native_refs.is_empty());
    let mut invalid = organization_fixture();
    invalid.native_projections[0]
        .native_refs
        .push(id("native.alpha.one"));
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_10_adding_a_scope_changes_only_the_explicit_source_family() {
    let baseline = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let mut extended = organization_fixture();
    extended.scopes.push(LogicalScopeV1 {
        scope: scope("delta", "scope"),
        axes: axes("delta-scope"),
        state: EffectiveFactStateV1::Present,
        interval: interval(),
    });
    let extended = lower_organizational_translation_v1(extended).unwrap();
    assert_eq!(baseline.source.organizations, extended.source.organizations);
    assert_eq!(
        baseline.source.scopes.len() + 1,
        extended.source.scopes.len()
    );
    let mut invalid = organization_fixture();
    invalid.organizations[0].axes.owner = id("owner.beta");
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_11_native_projections_are_optional_multiple_and_independently_stale() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    assert_eq!(lowered.source.native_projections.len(), 2);
    assert!(lowered
        .source
        .native_projections
        .iter()
        .any(|item| item.state == EffectiveFactStateV1::Stale));
    let empty = lower_organizational_translation_v1(empty_organization_fixture()).unwrap();
    assert!(empty.source.native_projections.is_empty());
    let mut invalid = organization_fixture();
    invalid
        .native_projections
        .push(invalid.native_projections[0].clone());
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_12_equal_handles_preserve_distinct_authored_local_resolutions() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let binding = &lowered.source.communication_actor_bindings[0];
    assert_eq!(binding.local_resolutions.len(), 2);
    assert_eq!(binding.local_resolutions[0].authorized_closure.len(), 1);
    assert!(binding.local_resolutions[1].authorized_closure.is_empty());
    let mut invalid = organization_fixture();
    invalid.communication_actor_bindings[0].local_resolutions[0]
        .authorized_closure
        .push(OrganizationalNodeRefV1::Role {
            role: role("unknown", "role"),
        });
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_13_observation_projection_creates_no_ruling_assignment_or_structure() {
    let baseline = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let mut observed = organization_fixture();
    observed
        .native_projections
        .push(OrganizationalNativeProjectionV1 {
            projection_id: id("projection.gamma.observation"),
            subject: OrganizationalNodeRefV1::Scope {
                scope: scope("gamma", "scope"),
            },
            class: ResponsibilityProjectionClassV1::Context,
            native_owner: id("native-owner.gamma"),
            native_refs: vec![id("native.gamma.observation")],
            disclosure: DisclosureCeilingV1::Private,
            state: EffectiveFactStateV1::Present,
            interval: interval(),
        });
    let observed = lower_organizational_translation_v1(observed).unwrap();
    assert_eq!(baseline.source.organizations, observed.source.organizations);
    assert_eq!(baseline.source.roles, observed.source.roles);
    assert_eq!(baseline.source.relations, observed.source.relations);
    assert_eq!(
        baseline.source.population_rules,
        observed.source.population_rules
    );
    assert_eq!(baseline.derived_relations, observed.derived_relations);
    assert_eq!(
        baseline.source.native_projections.len() + 1,
        observed.source.native_projections.len()
    );
}

#[test]
fn b4_derived_occupancy_cannot_be_authored_through_relation_grammar() {
    let authored_occupancy = serde_json::json!({
        "relation_id": "relation.authored-occupancy",
        "class": "occupancy",
        "from": {"kind": "role", "role": {"namespace": "alpha", "role_id": "accountable-a"}},
        "to": {"kind": "occupant", "occupant": "occupant.alpha"},
        "issuer": "issuer.alpha",
        "state": "present",
        "interval": {"start": 1, "end_exclusive": 10}
    });
    assert!(serde_json::from_value::<OrganizationalRelationV1>(authored_occupancy).is_err());
}

#[test]
fn b4_case_14_direct_native_paths_need_no_optional_organization_or_projection() {
    let lowered = lower_organizational_translation_v1(empty_organization_fixture()).unwrap();
    assert!(lowered.source.scopes.is_empty());
    assert!(lowered.source.native_projections.is_empty());
    let mut invalid = empty_organization_fixture();
    invalid
        .native_projections
        .push(OrganizationalNativeProjectionV1 {
            projection_id: id("projection.orphan"),
            subject: OrganizationalNodeRefV1::Role {
                role: role("orphan", "role"),
            },
            class: ResponsibilityProjectionClassV1::NativeApplication,
            native_owner: id("owner.orphan"),
            native_refs: Vec::new(),
            disclosure: DisclosureCeilingV1::Private,
            state: EffectiveFactStateV1::Unknown,
            interval: interval(),
        });
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_15_cross_owner_consequence_states_are_separate() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let service = &lowered.source.services[0];
    let refs = [
        &service.provider_completion.requirement.evidence_ref,
        &service.provider_acceptance.requirement.evidence_ref,
        &service.consumer_acceptance.requirement.evidence_ref,
        &service.native_read_back.requirement.evidence_ref,
        &service.pale_settlement.requirement.evidence_ref,
    ];
    assert_eq!(refs.into_iter().collect::<BTreeSet<_>>().len(), 5);
    let mut changed = organization_fixture();
    changed.services[0]
        .provider_acceptance
        .requirement
        .evidence_ref = id("evidence.changed");
    let changed = lower_organizational_translation_v1(changed).unwrap();
    let service = &changed.source.services[0];
    assert_ne!(
        service.provider_acceptance.requirement.evidence_ref,
        service.consumer_acceptance.requirement.evidence_ref
    );
}

#[test]
fn b4_case_16_object_population_is_class_typed_and_not_label_inferred() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    assert_eq!(lowered.source.objects.len(), 2);
    assert_eq!(lowered.source.population_rules.len(), 2);
    assert_ne!(
        lowered.source.population_rules[0].object_class,
        lowered.source.population_rules[1].object_class
    );
    let mut invalid = organization_fixture();
    invalid
        .population_rules
        .push(invalid.population_rules[0].clone());
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_case_17_accountable_collaboration_merges_neither_identity_nor_rights() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let relation = lowered
        .source
        .relations
        .iter()
        .find(|item| item.class == OrganizationalRelationClassV1::Collaboration)
        .unwrap();
    assert_ne!(relation.from, relation.to);
    assert_ne!(
        lowered.source.roles[0].axes.authority,
        lowered.source.roles[1].axes.authority
    );
    assert_ne!(
        lowered.source.roles[0].axes.controller,
        lowered.source.roles[1].axes.controller
    );
    let atomic_edges = lowered
        .source
        .relations
        .iter()
        .filter(|relation| {
            relation.to
                == OrganizationalNodeRefV1::Object {
                    object: id("object.atomic-system"),
                }
        })
        .collect::<Vec<_>>();
    assert_eq!(atomic_edges.len(), 2);
    assert_eq!(
        atomic_edges
            .iter()
            .map(|relation| &relation.from)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    assert_eq!(
        atomic_edges
            .iter()
            .map(|relation| relation.class)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            OrganizationalRelationClassV1::Acceptance,
            OrganizationalRelationClassV1::Collaboration,
        ])
    );
    for class in [
        OrganizationalRelationClassV1::Coordinates,
        OrganizationalRelationClassV1::Primary,
        OrganizationalRelationClassV1::Leader,
        OrganizationalRelationClassV1::Supporting,
        OrganizationalRelationClassV1::PeopleManagement,
        OrganizationalRelationClassV1::RoadmapCoordination,
        OrganizationalRelationClassV1::DisciplineJudgment,
        OrganizationalRelationClassV1::Standards,
        OrganizationalRelationClassV1::Review,
        OrganizationalRelationClassV1::BoundedBlock,
        OrganizationalRelationClassV1::Escalation,
        OrganizationalRelationClassV1::Acceptance,
        OrganizationalRelationClassV1::Collaboration,
    ] {
        assert_eq!(
            class.locality(),
            OrganizationalRelationLocalityV1::Internal,
            "{class}"
        );
    }
    assert_eq!(
        OrganizationalRelationClassV1::NativePresence.locality(),
        OrganizationalRelationLocalityV1::NativeBoundary
    );
    let mut cross_owner = organization_fixture();
    cross_owner.relations[2].to = OrganizationalNodeRefV1::Role {
        role: role("beta", "reviewer"),
    };
    assert!(matches!(
        lower_organizational_translation_v1(cross_owner),
        Err(crate::composition::OrganizationalTranslationError::NonLocalRelation { .. })
    ));
}

#[test]
fn b4_case_18_group_lens_is_bounded_and_not_line_management() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let lens = &lowered.source.group_lenses[0];
    assert_eq!(lens.holder, role("alpha", "accountable-a"));
    assert_eq!(lens.alignment_relation, id("relation.collaboration"));
    assert_eq!(lens.rights, group_lens_rights());
    assert_eq!(lens.block_conditions, [id("condition.quality-gate")]);
    assert_eq!(
        lens.escalation_resolver,
        Some(role("alpha", "accountable-b"))
    );
    assert!(!lowered
        .source
        .relations
        .iter()
        .any(|item| item.class == OrganizationalRelationClassV1::PeopleManagement));
    let mut missing_support = organization_fixture();
    missing_support.group_lenses[0].review_owner = None;
    assert!(lower_organizational_translation_v1(missing_support).is_err());
    let mut unexpected_support = organization_fixture();
    unexpected_support.group_lenses[0]
        .rights
        .retain(|right| *right != GroupLensRightV1::BoundedBlock);
    assert!(lower_organizational_translation_v1(unexpected_support).is_err());
    let mut foreign_holder = organization_fixture();
    foreign_holder.group_lenses[0].holder = role("beta", "reviewer");
    assert!(matches!(
        lower_organizational_translation_v1(foreign_holder),
        Err(
            crate::composition::OrganizationalTranslationError::InvalidGroupLensBinding {
                kind: "holder-to-relation",
                ..
            }
        )
    ));
    let mut management_alignment = organization_fixture();
    management_alignment.relations[2].class = OrganizationalRelationClassV1::PeopleManagement;
    assert!(matches!(
        lower_organizational_translation_v1(management_alignment),
        Err(
            crate::composition::OrganizationalTranslationError::InvalidGroupLensBinding {
                kind: "holder-to-relation",
                ..
            }
        )
    ));
}

#[test]
fn b4_case_19_full_matrix_preserves_every_negative_state_without_authority() {
    let lowered = lower_organizational_translation_v1(organization_fixture()).unwrap();
    let inspection = lowered.source.inspection.unwrap();
    assert_eq!(inspection.coordinates, InspectionCoordinateClassV1::all());
    assert_eq!(inspection.states, EffectiveFactStateV1::all());
    assert!(inspection.view_provider.is_none());
    let mut invalid = organization_fixture();
    invalid.inspection.as_mut().unwrap().states.pop();
    assert!(lower_organizational_translation_v1(invalid).is_err());
}

#[test]
fn b4_rejects_invalid_intervals_relations_and_duplicate_lens_rights() {
    let mut invalid = organization_fixture();
    invalid.roles[0].interval.end_exclusive = Some(1);
    assert!(lower_organizational_translation_v1(invalid).is_err());

    let mut invalid = organization_fixture();
    invalid.relations[0].from = OrganizationalNodeRefV1::Object {
        object: id("object.atomic-system"),
    };
    assert!(lower_organizational_translation_v1(invalid).is_err());

    let mut invalid = organization_fixture();
    invalid.group_lenses[0]
        .rights
        .push(GroupLensRightV1::Review);
    assert!(lower_organizational_translation_v1(invalid).is_err());
}
