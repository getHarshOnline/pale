//! Package selection, closure, and governing-scope tests.

use pale_core::records::{
    AdmittedScopedOrderRelationV1, PackageDigest, PackageIdV1, PackageReleaseV1,
    PackageResourceExposureV1, PackageResourceIdV1, PackageResourceKindV1, PackageResourceRefV1,
    PackageResourceSelectorV1, PackageVersionV1, ResolvedGoverningContributionV1,
    ResolvedGoverningObligationV1, SemanticOrderPurposeV1, SemanticSelection,
};

use super::*;
use crate::packages::PackageDiagnosticCode;

fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.semantic").expect("valid Package id"),
        resource_id: PackageResourceIdV1::parse(id).expect("valid resource id"),
    }
}

fn usage(id: &str, kind: PackageResourceKindV1) -> PackageResourceUse {
    PackageResourceUse::new(resource(id), kind)
}

fn node(resource: PackageResource) -> AdmittedResourceNode {
    AdmittedResourceNode {
        page: None,
        exposure: PackageResourceExposureV1::Exported,
        resource,
    }
}

fn graph(permuted: bool, with_governing: bool) -> AdmittedPackageGraph {
    let doctrine = resource("doctrine");
    let operation = resource("operation");
    let role = resource("role");
    let stack = resource("stack");
    let obligation = resource("obligation");
    let contributor = resource("governing");
    let second_contributor = resource("governing-two");
    let rows = [
        (
            doctrine.clone(),
            node(PackageResource::Doctrine {
                description: "doctrine".to_string(),
                placement: pale_core::records::DoctrinePlacement::StandaloneJudgment,
                extends: Vec::new(),
                requires: Vec::new(),
                see_also: Vec::new(),
                official_sources: Vec::new(),
            }),
        ),
        (
            operation.clone(),
            node(PackageResource::Operation {
                description: "operation".to_string(),
                extends: Vec::new(),
                doctrine: vec![doctrine.clone()],
                roles: Vec::new(),
                outputs: Vec::new(),
                requirements: Vec::new(),
                capability_policy: None,
            }),
        ),
        (
            role.clone(),
            node(PackageResource::Role {
                description: "role".to_string(),
                extends: Vec::new(),
                inherited_lenses: Vec::new(),
                doctrine: vec![doctrine.clone()],
                stacks: Vec::new(),
            }),
        ),
        (
            stack.clone(),
            node(PackageResource::Stack {
                description: "stack".to_string(),
                extends: Vec::new(),
                doctrine: vec![doctrine.clone()],
                role_doctrine: Vec::new(),
                official_sources: Vec::new(),
            }),
        ),
        (
            obligation.clone(),
            node(PackageResource::Doctrine {
                description: "obligation".to_string(),
                placement: pale_core::records::DoctrinePlacement::StandaloneJudgment,
                extends: Vec::new(),
                requires: Vec::new(),
                see_also: Vec::new(),
                official_sources: Vec::new(),
            }),
        ),
        (contributor.clone(), node(PackageResource::GoverningOrder)),
        (
            second_contributor.clone(),
            node(PackageResource::GoverningOrder),
        ),
    ];
    let rows = rows.into_iter().map(|(resource, node)| {
        (
            PackageResourceUse::new(resource, resource_kind(&node.resource)),
            node,
        )
    });
    let resources = if permuted {
        rows.rev().collect()
    } else {
        rows.collect()
    };
    let bodies = [
        doctrine.clone(),
        operation.clone(),
        role.clone(),
        obligation.clone(),
    ]
    .into_iter()
    .map(|id| {
        let body = AdmittedMarkdown::from_text(id.clone(), "# Exact body\n");
        (id, body)
    })
    .collect();
    let contribution = |contributor: PackageResourceRefV1| {
        let mut obligations = vec![
            ResolvedGoverningObligationV1 {
                scope: PackageResourceUse::new(operation.clone(), PackageResourceKindV1::Operation),
                subject: PackageResourceUse::new(
                    operation.clone(),
                    PackageResourceKindV1::Operation,
                ),
                obligation: PackageResourceUse::new(
                    obligation.clone(),
                    PackageResourceKindV1::Doctrine,
                ),
                contributor: contributor.clone(),
            },
            ResolvedGoverningObligationV1 {
                scope: PackageResourceUse::new(role.clone(), PackageResourceKindV1::Role),
                subject: PackageResourceUse::new(role.clone(), PackageResourceKindV1::Role),
                obligation: PackageResourceUse::new(stack.clone(), PackageResourceKindV1::Stack),
                contributor: contributor.clone(),
            },
        ];
        if permuted {
            obligations.reverse();
        }
        (
            contributor,
            ResolvedGoverningContributionV1 {
                constraints: Vec::new(),
                obligations,
                exclusive_authority: Vec::new(),
                narrowing: Vec::new(),
                order: Vec::new(),
            },
        )
    };
    let governing = if with_governing {
        let rows = if permuted {
            vec![
                contribution(second_contributor.clone()),
                contribution(contributor.clone()),
            ]
        } else {
            vec![
                contribution(contributor.clone()),
                contribution(second_contributor.clone()),
            ]
        };
        rows.into_iter().collect()
    } else {
        BTreeMap::new()
    };
    AdmittedPackageGraph {
        releases: vec![PackageReleaseV1 {
            id: PackageIdV1::parse("example.semantic").expect("valid Package id"),
            version: PackageVersionV1::parse("1.0.0").expect("valid version"),
            content_digest: PackageDigest::hash_bytes(b"release"),
        }],
        resources,
        bodies,
        capability_policies: BTreeMap::new(),
        recipes: BTreeMap::new(),
        governing,
    }
}

fn repository_seeds(
    permuted: bool,
) -> BTreeMap<LogicalRepositoryIdV1, Vec<(PackageResourceRefV1, PackageResourceKindV1)>> {
    let mut alpha = vec![
        (resource("role"), PackageResourceKindV1::Role),
        (resource("doctrine"), PackageResourceKindV1::Doctrine),
    ];
    let mut beta = vec![
        (resource("operation"), PackageResourceKindV1::Operation),
        (resource("doctrine"), PackageResourceKindV1::Doctrine),
    ];
    if permuted {
        alpha.reverse();
        beta.reverse();
    }
    BTreeMap::from([
        (
            LogicalRepositoryIdV1::parse("repo-alpha").expect("valid repository id"),
            alpha,
        ),
        (
            LogicalRepositoryIdV1::parse("repo-beta").expect("valid repository id"),
            beta,
        ),
    ])
}

fn repository_order_graph(permuted: bool) -> AdmittedPackageGraph {
    let mut graph = graph(permuted, true);
    for id in ["alpha-before", "alpha-after", "beta-before", "beta-after"] {
        graph.resources.insert(
            usage(id, PackageResourceKindV1::PolicyFamily),
            node(PackageResource::PolicyFamily {
                title: id.to_string(),
                purpose: format!("purpose for {id}"),
            }),
        );
        let owner = resource(id);
        graph.bodies.insert(
            owner.clone(),
            AdmittedMarkdown::from_text(owner, &format!("# {id}\n")),
        );
    }
    let contributor = resource("governing");
    let contribution = graph
        .governing
        .get_mut(&contributor)
        .expect("governing fixture contribution");
    contribution.order.extend([
        AdmittedScopedOrderRelationV1 {
            scope: usage("role", PackageResourceKindV1::Role),
            purpose: SemanticOrderPurposeV1::Presentation,
            before: usage("alpha-before", PackageResourceKindV1::PolicyFamily),
            after: usage("alpha-after", PackageResourceKindV1::PolicyFamily),
            contributor: contributor.clone(),
        },
        AdmittedScopedOrderRelationV1 {
            scope: usage("operation", PackageResourceKindV1::Operation),
            purpose: SemanticOrderPurposeV1::Presentation,
            before: usage("beta-before", PackageResourceKindV1::PolicyFamily),
            after: usage("beta-after", PackageResourceKindV1::PolicyFamily),
            contributor,
        },
    ]);
    if permuted {
        contribution.order.reverse();
    }
    graph
}

fn two_hop_graph(permuted: bool) -> AdmittedPackageGraph {
    let mut graph = graph(permuted, true);
    let second_hop = resource("second-hop");
    graph.resources.insert(
        PackageResourceUse::new(second_hop.clone(), PackageResourceKindV1::Doctrine),
        node(PackageResource::Doctrine {
            description: "second hop".to_string(),
            placement: pale_core::records::DoctrinePlacement::StandaloneJudgment,
            extends: Vec::new(),
            requires: Vec::new(),
            see_also: Vec::new(),
            official_sources: Vec::new(),
        }),
    );
    graph.bodies.insert(
        second_hop.clone(),
        AdmittedMarkdown::from_text(second_hop.clone(), "# Second hop\n"),
    );
    for (contributor, contribution) in &mut graph.governing {
        contribution
            .obligations
            .push(ResolvedGoverningObligationV1 {
                scope: usage("stack", PackageResourceKindV1::Stack),
                subject: usage("stack", PackageResourceKindV1::Stack),
                obligation: PackageResourceUse::new(
                    second_hop.clone(),
                    PackageResourceKindV1::Doctrine,
                ),
                contributor: contributor.clone(),
            });
        if permuted {
            contribution.obligations.reverse();
        }
    }
    graph
}

fn exact(resource: PackageResourceRefV1) -> PackageResourceSelectorV1 {
    PackageResourceSelectorV1::Exact { resource }
}

#[test]
fn each_selection_family_closes_only_its_positive_dependencies() {
    let cases = [
        (
            PackageResourceKindV1::Operation,
            resource("operation"),
            BTreeSet::from([
                usage("operation", PackageResourceKindV1::Operation),
                usage("doctrine", PackageResourceKindV1::Doctrine),
            ]),
        ),
        (
            PackageResourceKindV1::Role,
            resource("role"),
            BTreeSet::from([
                usage("role", PackageResourceKindV1::Role),
                usage("doctrine", PackageResourceKindV1::Doctrine),
            ]),
        ),
        (
            PackageResourceKindV1::Stack,
            resource("stack"),
            BTreeSet::from([
                usage("stack", PackageResourceKindV1::Stack),
                usage("doctrine", PackageResourceKindV1::Doctrine),
            ]),
        ),
        (
            PackageResourceKindV1::Doctrine,
            resource("doctrine"),
            BTreeSet::from([usage("doctrine", PackageResourceKindV1::Doctrine)]),
        ),
    ];
    for (kind, seed, expected) in cases {
        let mut selection = SemanticSelection::default();
        match kind {
            PackageResourceKindV1::Operation => selection.operations.push(exact(seed)),
            PackageResourceKindV1::Role => selection.roles.push(exact(seed)),
            PackageResourceKindV1::Stack => selection.stacks.push(exact(seed)),
            PackageResourceKindV1::Doctrine => selection.doctrine.push(exact(seed)),
            _ => unreachable!("fixture covers the four occurrence selection fields"),
        }
        let sealed = graph(false, false)
            .select_and_seal(&selection, &BTreeMap::new())
            .unwrap();
        assert_eq!(
            sealed.resources().keys().cloned().collect::<BTreeSet<_>>(),
            expected
        );
    }
}

#[test]
fn optional_cycles_do_not_select_pages_or_activate_their_governing_rows() {
    let mut graph = graph(false, true);
    let PackageResource::Doctrine {
        extends,
        requires,
        see_also,
        ..
    } = &mut graph
        .resources
        .get_mut(&usage("doctrine", PackageResourceKindV1::Doctrine))
        .unwrap()
        .resource
    else {
        unreachable!()
    };
    extends.push(resource("obligation"));
    requires.push(resource("required"));
    see_also.push(resource("optional"));
    graph.resources.insert(
        usage("required", PackageResourceKindV1::Doctrine),
        node(PackageResource::Doctrine {
            description: "required".into(),
            placement: pale_core::records::DoctrinePlacement::StandaloneJudgment,
            extends: Vec::new(),
            requires: Vec::new(),
            see_also: vec![resource("doctrine")],
            official_sources: Vec::new(),
        }),
    );
    graph.bodies.insert(
        resource("required"),
        AdmittedMarkdown::from_text(resource("required"), "# Required\n"),
    );
    graph.resources.insert(
        usage("optional", PackageResourceKindV1::Doctrine),
        node(PackageResource::Doctrine {
            description: "optional".into(),
            placement: pale_core::records::DoctrinePlacement::StandaloneJudgment,
            extends: Vec::new(),
            requires: Vec::new(),
            see_also: vec![resource("doctrine")],
            official_sources: Vec::new(),
        }),
    );
    graph.bodies.insert(
        resource("optional"),
        AdmittedMarkdown::from_text(resource("optional"), "# Optional\n"),
    );
    for (contributor, contribution) in &mut graph.governing {
        contribution
            .obligations
            .push(ResolvedGoverningObligationV1 {
                scope: usage("optional", PackageResourceKindV1::Doctrine),
                subject: usage("optional", PackageResourceKindV1::Doctrine),
                obligation: usage("stack", PackageResourceKindV1::Stack),
                contributor: contributor.clone(),
            });
    }
    let selection = SemanticSelection {
        doctrine: vec![exact(resource("doctrine"))],
        ..SemanticSelection::default()
    };
    let selected = graph.select_and_seal(&selection, &BTreeMap::new()).unwrap();
    assert_eq!(
        selected
            .resources()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            usage("doctrine", PackageResourceKindV1::Doctrine),
            usage("obligation", PackageResourceKindV1::Doctrine),
            usage("required", PackageResourceKindV1::Doctrine)
        ])
    );
    assert!(selected.profile_governing_order().obligations().is_empty());
}

#[test]
fn package_scoped_wildcard_selects_only_exported_occurrences_of_its_field_kind() {
    let mut graph = graph(false, false);
    graph.resources.insert(
        usage("internal-operation", PackageResourceKindV1::Operation),
        AdmittedResourceNode {
            page: None,
            exposure: PackageResourceExposureV1::Internal,
            resource: PackageResource::Operation {
                description: "internal operation".to_string(),
                extends: Vec::new(),
                doctrine: Vec::new(),
                roles: Vec::new(),
                outputs: Vec::new(),
                requirements: Vec::new(),
                capability_policy: None,
            },
        },
    );
    let selection = SemanticSelection {
        operations: vec![PackageResourceSelectorV1::AllFromPackage {
            package_id: PackageIdV1::parse("example.semantic").expect("valid Package id"),
        }],
        ..SemanticSelection::default()
    };
    let sealed = graph.select_and_seal(&selection, &BTreeMap::new()).unwrap();
    assert!(sealed
        .resources()
        .contains_key(&usage("operation", PackageResourceKindV1::Operation)));
    assert!(sealed
        .resources()
        .contains_key(&usage("doctrine", PackageResourceKindV1::Doctrine)));
    assert!(!sealed.resources().contains_key(&usage(
        "internal-operation",
        PackageResourceKindV1::Operation
    )));
    assert!(!sealed
        .resources()
        .contains_key(&usage("role", PackageResourceKindV1::Role)));
}

#[test]
fn governing_relevance_composes_all_fragments_once_and_permutation_preserves_the_seal() {
    let selection = SemanticSelection {
        operations: vec![exact(resource("operation"))],
        ..SemanticSelection::default()
    };
    let first = graph(false, true)
        .select_and_seal(&selection, &BTreeMap::new())
        .unwrap();
    let second = graph(true, true)
        .select_and_seal(&selection, &BTreeMap::new())
        .unwrap();

    assert_eq!(first.digest(), second.digest());
    assert_eq!(first.profile_governing_order().obligations().len(), 1);
    assert_eq!(
        first.profile_governing_order().obligations()[0]
            .contributors
            .len(),
        2
    );
    assert!(first
        .resources()
        .contains_key(&usage("governing", PackageResourceKindV1::GoverningOrder)));
    assert!(first.resources().contains_key(&usage(
        "governing-two",
        PackageResourceKindV1::GoverningOrder
    )));
    assert!(first
        .resources()
        .contains_key(&usage("obligation", PackageResourceKindV1::Doctrine)));
    assert!(!first
        .resources()
        .contains_key(&usage("role", PackageResourceKindV1::Role)));
    assert!(!first
        .resources()
        .contains_key(&usage("stack", PackageResourceKindV1::Stack)));
    let expected_package_id = resource("operation").package_id;
    assert!(first
        .resources()
        .keys()
        .all(|resource| resource.resource.package_id == expected_package_id));
}

#[test]
fn governing_rows_activate_only_from_their_own_selected_scope() {
    let role_only = SemanticSelection {
        roles: vec![exact(resource("role"))],
        ..SemanticSelection::default()
    };
    let role_graph = graph(false, true)
        .select_and_seal(&role_only, &BTreeMap::new())
        .unwrap();
    assert_eq!(role_graph.profile_governing_order().obligations().len(), 1);
    assert_eq!(
        role_graph.profile_governing_order().obligations()[0].obligation,
        usage("stack", PackageResourceKindV1::Stack)
    );
    assert!(role_graph
        .resources()
        .contains_key(&usage("stack", PackageResourceKindV1::Stack)));
    assert!(!role_graph
        .resources()
        .contains_key(&usage("operation", PackageResourceKindV1::Operation)));
    assert!(!role_graph
        .resources()
        .contains_key(&usage("obligation", PackageResourceKindV1::Doctrine)));

    let both = SemanticSelection {
        operations: vec![exact(resource("operation"))],
        roles: vec![exact(resource("role"))],
        ..SemanticSelection::default()
    };
    let first = graph(false, true)
        .select_and_seal(&both, &BTreeMap::new())
        .unwrap();
    let permuted = graph(true, true)
        .select_and_seal(&both, &BTreeMap::new())
        .unwrap();
    assert_eq!(first.digest(), permuted.digest());
    assert_eq!(first.profile_governing_order().obligations().len(), 2);
    assert!(first
        .resources()
        .contains_key(&usage("obligation", PackageResourceKindV1::Doctrine)));
    assert!(first
        .resources()
        .contains_key(&usage("stack", PackageResourceKindV1::Stack)));
}

#[test]
fn repository_governing_fixed_points_are_causally_isolated_and_permutation_stable() {
    let first = repository_order_graph(false)
        .select_and_seal(&SemanticSelection::default(), &repository_seeds(false))
        .unwrap();
    let permuted = repository_order_graph(true)
        .select_and_seal(&SemanticSelection::default(), &repository_seeds(true))
        .unwrap();

    assert_eq!(first.digest(), permuted.digest());
    assert_eq!(first.profile_governing_order().obligations().len(), 2);
    assert_eq!(first.profile_governing_order().order().len(), 2);

    let alpha_id = LogicalRepositoryIdV1::parse("repo-alpha").expect("valid repository id");
    let beta_id = LogicalRepositoryIdV1::parse("repo-beta").expect("valid repository id");
    let alpha = first.repository_scope(&alpha_id).unwrap();
    assert!(alpha.contains(&usage("role", PackageResourceKindV1::Role)));
    assert!(alpha.contains(&usage("stack", PackageResourceKindV1::Stack)));
    assert!(alpha.contains(&usage("alpha-before", PackageResourceKindV1::PolicyFamily)));
    assert!(alpha.contains(&usage("alpha-after", PackageResourceKindV1::PolicyFamily)));
    assert!(!alpha.contains(&usage("operation", PackageResourceKindV1::Operation)));
    assert!(!alpha.contains(&usage("obligation", PackageResourceKindV1::Doctrine)));
    assert!(!alpha.contains(&usage("beta-before", PackageResourceKindV1::PolicyFamily)));
    assert!(!alpha.contains(&usage("beta-after", PackageResourceKindV1::PolicyFamily)));
    let alpha_order = first.repository_governing_order(&alpha_id).unwrap();
    assert_eq!(alpha_order.obligations().len(), 1);
    assert_eq!(
        alpha_order.obligations()[0].obligation,
        usage("stack", PackageResourceKindV1::Stack)
    );
    assert_eq!(alpha_order.order().len(), 1);
    assert_eq!(
        alpha_order.order()[0].before,
        usage("alpha-before", PackageResourceKindV1::PolicyFamily)
    );
    assert_eq!(
        alpha_order.order()[0].after,
        usage("alpha-after", PackageResourceKindV1::PolicyFamily)
    );

    let beta = first.repository_scope(&beta_id).unwrap();
    assert!(beta.contains(&usage("operation", PackageResourceKindV1::Operation)));
    assert!(beta.contains(&usage("obligation", PackageResourceKindV1::Doctrine)));
    assert!(beta.contains(&usage("beta-before", PackageResourceKindV1::PolicyFamily)));
    assert!(beta.contains(&usage("beta-after", PackageResourceKindV1::PolicyFamily)));
    assert!(!beta.contains(&usage("role", PackageResourceKindV1::Role)));
    assert!(!beta.contains(&usage("stack", PackageResourceKindV1::Stack)));
    assert!(!beta.contains(&usage("alpha-before", PackageResourceKindV1::PolicyFamily)));
    assert!(!beta.contains(&usage("alpha-after", PackageResourceKindV1::PolicyFamily)));
    let beta_order = first.repository_governing_order(&beta_id).unwrap();
    assert_eq!(beta_order.obligations().len(), 1);
    assert_eq!(
        beta_order.obligations()[0].obligation,
        usage("obligation", PackageResourceKindV1::Doctrine)
    );
    assert_eq!(beta_order.order().len(), 1);
    assert_eq!(
        beta_order.order()[0].before,
        usage("beta-before", PackageResourceKindV1::PolicyFamily)
    );
    assert_eq!(
        beta_order.order()[0].after,
        usage("beta-after", PackageResourceKindV1::PolicyFamily)
    );
}

#[test]
fn governing_closure_reaches_two_hops_but_never_infers_a_reverse_edge() {
    let role = SemanticSelection {
        roles: vec![exact(resource("role"))],
        ..SemanticSelection::default()
    };
    let first = two_hop_graph(false)
        .select_and_seal(&role, &BTreeMap::new())
        .unwrap();
    let permuted = two_hop_graph(true)
        .select_and_seal(&role, &BTreeMap::new())
        .unwrap();
    assert_eq!(first.digest(), permuted.digest());
    assert!(first
        .resources()
        .contains_key(&usage("stack", PackageResourceKindV1::Stack)));
    assert!(first
        .resources()
        .contains_key(&usage("second-hop", PackageResourceKindV1::Doctrine)));
    assert_eq!(first.profile_governing_order().obligations().len(), 2);

    let reverse = SemanticSelection {
        doctrine: vec![exact(resource("second-hop"))],
        ..SemanticSelection::default()
    };
    let reverse = two_hop_graph(false)
        .select_and_seal(&reverse, &BTreeMap::new())
        .unwrap();
    assert_eq!(reverse.resource_count(), 1);
    assert!(reverse
        .resources()
        .contains_key(&usage("second-hop", PackageResourceKindV1::Doctrine)));
    assert!(!reverse
        .resources()
        .contains_key(&usage("stack", PackageResourceKindV1::Stack)));
    assert!(reverse.profile_governing_order().obligations().is_empty());
}

#[test]
fn independent_selector_failures_return_one_typed_bounded_aggregate() {
    let selection = SemanticSelection {
        operations: vec![exact(resource("missing-operation"))],
        roles: vec![exact(resource("missing-role"))],
        ..SemanticSelection::default()
    };
    let PackageError::Diagnostics(diagnostics) = graph(false, false)
        .select_and_seal(&selection, &BTreeMap::new())
        .unwrap_err()
    else {
        panic!("independent selector failures must not collapse to first error");
    };
    assert_eq!(diagnostics.observed_len(), 2);
    assert!(diagnostics
        .entries()
        .iter()
        .all(|diagnostic| diagnostic.code() == PackageDiagnosticCode::ResourceSelector));
}
