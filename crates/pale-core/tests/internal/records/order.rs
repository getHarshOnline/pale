//! Governing algebra contract tests.

use super::*;

fn resource(package_id: &str, resource_id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: crate::records::PackageIdV1::parse(package_id)
            .expect("synthetic Package id must be valid"),
        resource_id: crate::records::PackageResourceIdV1::parse(resource_id)
            .expect("synthetic resource id must be valid"),
    }
}

fn usage(package_id: &str, resource_id: &str) -> PackageResourceUseV1 {
    PackageResourceUseV1::new(
        resource(package_id, resource_id),
        crate::records::PackageResourceKindV1::Doctrine,
    )
}

fn capability(id: &str) -> CapabilityIdV1 {
    CapabilityIdV1::parse(id).expect("synthetic capability id must be valid")
}

fn ceiling(permitted: &[&str], forbidden: &[&str]) -> AdmittedCapabilityPolicyV1 {
    AdmittedCapabilityPolicyV1 {
        permitted_capabilities: permitted.iter().map(|id| capability(id)).collect(),
        forbidden_capabilities: forbidden.iter().map(|id| capability(id)).collect(),
        permitted_effects: BTreeSet::new(),
        forbidden_effects: BTreeSet::new(),
    }
}

fn policy(permitted: &[&str], forbidden: &[&str]) -> CapabilityPolicy {
    CapabilityPolicy {
        permitted_capabilities: permitted.iter().map(|id| capability(id)).collect(),
        forbidden_capabilities: forbidden.iter().map(|id| capability(id)).collect(),
        permitted_effects: Vec::new(),
        forbidden_effects: Vec::new(),
    }
}

fn constraint(subject: &str, policy_id: &str, contributor: &str) -> ResolvedGoverningConstraintV1 {
    ResolvedGoverningConstraintV1 {
        scope: usage("example.workflow", "scope"),
        subject: usage("example.workflow", subject),
        policy: resource("example.policy", policy_id),
        contributor: resource("example.policy", contributor),
    }
}

fn empty_contribution() -> ResolvedGoverningContributionV1 {
    ResolvedGoverningContributionV1 {
        constraints: Vec::new(),
        obligations: Vec::new(),
        exclusive_authority: Vec::new(),
        narrowing: Vec::new(),
        order: Vec::new(),
    }
}

#[test]
fn order_purpose_does_not_collapse_into_one_sequence() {
    let link = |resource_id| GoverningResourceLinkV1::Local {
        kind: None,
        resource_id: crate::records::PackageResourceIdV1::parse(resource_id)
            .expect("synthetic resource id must be valid"),
    };
    let before = link("research");
    let after = link("review");
    let scope = link("publish");
    let reasoning = ScopedOrderRelationV1 {
        scope: scope.clone(),
        purpose: SemanticOrderPurposeV1::Reasoning,
        before: before.clone(),
        after: after.clone(),
    };
    let presentation = ScopedOrderRelationV1 {
        scope,
        purpose: SemanticOrderPurposeV1::Presentation,
        before,
        after,
    };

    assert_ne!(reasoning, presentation);
    assert_eq!(
        serde_json::to_value(&reasoning).expect("relation must serialize")["purpose"],
        "reasoning"
    );
    assert_eq!(
        serde_json::to_value(&presentation).expect("relation must serialize")["purpose"],
        "presentation"
    );
}

#[test]
fn ceiling_meet_obeys_semilattice_laws() {
    let a = ceiling(&["read", "write", "publish"], &["publish"]);
    let b = ceiling(&["read", "write"], &["write"]);
    let c = ceiling(&["read", "network"], &[]);

    assert_eq!(a.meet(&b), b.meet(&a));
    assert_eq!(a.meet(&b).meet(&c), a.meet(&b.meet(&c)));
    assert_eq!(a.meet(&a), a.normalized());
    let meet = a.meet(&b);
    assert_eq!(
        meet.permitted_capabilities,
        BTreeSet::from([capability("read")])
    );
    assert!(meet
        .permitted_capabilities
        .is_subset(&a.normalized().permitted_capabilities));
    assert!(meet
        .permitted_capabilities
        .is_subset(&b.normalized().permitted_capabilities));
    assert!(meet
        .forbidden_capabilities
        .is_superset(&a.forbidden_capabilities));
    assert!(meet
        .forbidden_capabilities
        .is_superset(&b.forbidden_capabilities));
}

#[test]
fn global_composition_meets_all_selected_package_contributions() {
    let broad = resource("example.policy", "broad");
    let narrow = resource("example.policy", "narrow");
    let policies = BTreeMap::from([
        (broad, policy(&["read", "write"], &[])),
        (narrow, policy(&["read"], &[])),
    ]);
    let first = ResolvedGoverningContributionV1 {
        constraints: vec![constraint("subject", "broad", "governing-a")],
        ..empty_contribution()
    };
    let second = ResolvedGoverningContributionV1 {
        constraints: vec![constraint("subject", "narrow", "governing-b")],
        ..empty_contribution()
    };

    let forward =
        ResolvedGoverningContributionV1::compose([first.clone(), second.clone()], &policies)
            .expect("selected Package contributions must compose");
    let reverse = ResolvedGoverningContributionV1::compose([second, first], &policies)
        .expect("composition order must have no semantic force");

    assert_eq!(forward, reverse);
    assert_eq!(forward.ceilings().len(), 1);
    assert_eq!(
        forward.ceilings()[0].policy.permitted_capabilities,
        BTreeSet::from([capability("read")])
    );
    assert_eq!(forward.ceilings()[0].contributors.len(), 2);
    assert_eq!(forward.ceilings()[0].policies.len(), 2);
}

#[test]
fn obligation_accumulation_retains_all_contributors() {
    let contribution = |contributor| ResolvedGoverningContributionV1 {
        obligations: vec![ResolvedGoverningObligationV1 {
            scope: usage("example.workflow", "scope"),
            subject: usage("example.workflow", "subject"),
            obligation: usage("example.policy", "cite-evidence"),
            contributor: resource("example.policy", contributor),
        }],
        ..empty_contribution()
    };
    let admitted = ResolvedGoverningContributionV1::compose(
        [contribution("policy-a"), contribution("policy-b")],
        &BTreeMap::new(),
    )
    .expect("obligations must compose");
    assert_eq!(admitted.obligations().len(), 1);
    assert_eq!(admitted.obligations()[0].contributors.len(), 2);
}

#[test]
fn exclusive_authority_conflict_is_order_independent() {
    let claim = |claimant| AdmittedExclusiveAuthorityClaimV1 {
        scope: usage("example.workflow", "scope"),
        authority: usage("example.authority", "release"),
        claimant: resource("example.claimants", claimant),
    };
    let first = ResolvedGoverningContributionV1 {
        exclusive_authority: vec![claim("a")],
        ..empty_contribution()
    };
    let second = ResolvedGoverningContributionV1 {
        exclusive_authority: vec![claim("b")],
        ..empty_contribution()
    };
    let forward =
        ResolvedGoverningContributionV1::compose([first.clone(), second.clone()], &BTreeMap::new())
            .expect_err("conflict must fail");
    let reverse = ResolvedGoverningContributionV1::compose([second, first], &BTreeMap::new())
        .expect_err("conflict must fail");
    assert_eq!(forward, reverse);
    assert!(matches!(
        forward,
        GoverningAlgebraErrorV1::ExclusiveAuthorityConflict { claimants, .. }
            if claimants == BTreeSet::from([
                resource("example.claimants", "a"),
                resource("example.claimants", "b"),
            ])
    ));
}

#[test]
fn purpose_scoped_partial_order_cycles_fail() {
    let scope = usage("example.workflow", "scope");
    let a = usage("example.graph", "a");
    let b = usage("example.graph", "b");
    let contributor = resource("example.policy", "governing");
    let invalid = ResolvedGoverningContributionV1 {
        order: vec![
            AdmittedScopedOrderRelationV1 {
                scope: scope.clone(),
                purpose: SemanticOrderPurposeV1::Composition,
                before: a.clone(),
                after: b.clone(),
                contributor: contributor.clone(),
            },
            AdmittedScopedOrderRelationV1 {
                scope,
                purpose: SemanticOrderPurposeV1::Composition,
                before: b.clone(),
                after: a.clone(),
                contributor,
            },
        ],
        ..empty_contribution()
    };
    let error = ResolvedGoverningContributionV1::compose([invalid], &BTreeMap::new())
        .expect_err("purpose-scoped cycle must fail");
    assert!(matches!(
        error,
        GoverningAlgebraErrorV1::PartialOrderCycle {
            purpose: SemanticOrderPurposeV1::Composition,
            ref members,
            ref edges,
            ..
        } if members == &vec![a.clone(), b.clone(), a.clone()]
            && edges.len() == 2
    ));
}

#[test]
fn different_order_purposes_remain_independent_and_select_nothing() {
    let scope = usage("example.workflow", "scope");
    let a = usage("example.graph", "a");
    let b = usage("example.graph", "b");
    let contributor = resource("example.policy", "governing");
    let contribution = ResolvedGoverningContributionV1 {
        order: vec![
            AdmittedScopedOrderRelationV1 {
                scope: scope.clone(),
                purpose: SemanticOrderPurposeV1::Reasoning,
                before: a.clone(),
                after: b.clone(),
                contributor: contributor.clone(),
            },
            AdmittedScopedOrderRelationV1 {
                scope,
                purpose: SemanticOrderPurposeV1::Presentation,
                before: b,
                after: a,
                contributor,
            },
        ],
        ..empty_contribution()
    };
    let admitted = ResolvedGoverningContributionV1::compose([contribution], &BTreeMap::new())
        .expect("opposite edges with different purposes are independent");
    assert_eq!(admitted.order().len(), 2);
    assert!(admitted.ceilings().is_empty());
    assert!(admitted.obligations().is_empty());
}

#[test]
fn narrowing_closure_propagates_policy_provenance_and_obligations() {
    let policies = BTreeMap::from([(
        resource("example.policy", "ceiling"),
        policy(&["read"], &[]),
    )]);
    let scope = usage("example.workflow", "scope");
    let broad = usage("example.workflow", "broad");
    let middle = usage("example.workflow", "middle");
    let leaf = usage("example.workflow", "leaf");
    let contributor = resource("example.policy", "governing");
    let contribution = ResolvedGoverningContributionV1 {
        constraints: vec![constraint("broad", "ceiling", "governing")],
        obligations: vec![ResolvedGoverningObligationV1 {
            scope: scope.clone(),
            subject: broad.clone(),
            obligation: usage("example.policy", "cite-evidence"),
            contributor: contributor.clone(),
        }],
        narrowing: vec![
            ResolvedGoverningNarrowingV1 {
                scope: scope.clone(),
                narrower: leaf.clone(),
                broader: middle.clone(),
                contributor: contributor.clone(),
            },
            ResolvedGoverningNarrowingV1 {
                scope: scope.clone(),
                narrower: middle,
                broader: broad,
                contributor,
            },
        ],
        ..empty_contribution()
    };
    let admitted = ResolvedGoverningContributionV1::compose([contribution], &policies)
        .expect("finite acyclic narrowing must reach closure");
    let leaf_ceiling = admitted
        .ceilings()
        .iter()
        .find(|ceiling| ceiling.subject == leaf)
        .expect("transitive closure must carry the broader policy to the leaf");
    assert_eq!(
        leaf_ceiling.policy.permitted_capabilities,
        BTreeSet::from([capability("read")])
    );
    assert!(admitted.obligations().iter().any(|obligation| {
        obligation.subject == leaf
            && obligation.obligation == usage("example.policy", "cite-evidence")
    }));
    admitted
        .validate()
        .expect("sealed closure must remain independently validatable");

    let mut incomplete = admitted;
    incomplete
        .obligations
        .retain(|obligation| obligation.subject != leaf);
    assert!(matches!(
        incomplete.validate(),
        Err(GoverningAlgebraErrorV1::IncompleteNarrowingClosure { .. })
    ));
}

#[test]
fn narrowing_propagates_obligations_without_a_capability_ceiling() {
    let scope = usage("example.workflow", "scope");
    let broad = usage("example.workflow", "broad");
    let narrow = usage("example.workflow", "narrow");
    let contributor = resource("example.policy", "governing");
    let contribution = ResolvedGoverningContributionV1 {
        obligations: vec![ResolvedGoverningObligationV1 {
            scope: scope.clone(),
            subject: broad.clone(),
            obligation: usage("example.policy", "cite-evidence"),
            contributor: contributor.clone(),
        }],
        narrowing: vec![ResolvedGoverningNarrowingV1 {
            scope,
            narrower: narrow.clone(),
            broader: broad,
            contributor,
        }],
        ..empty_contribution()
    };
    let admitted = ResolvedGoverningContributionV1::compose([contribution], &BTreeMap::new())
        .expect("obligations compose independently of capability ceilings");
    assert!(admitted
        .obligations()
        .iter()
        .any(|obligation| obligation.subject == narrow));
}

#[test]
fn narrowing_closure_stops_at_the_effective_relation_bound() {
    let scope = usage("example.workflow", "scope");
    let broad = usage("example.workflow", "broad");
    let contributor = resource("example.policy", "governing");
    let obligations = (0..MAX_GOVERNING_RELATIONS)
        .map(|index| ResolvedGoverningObligationV1 {
            scope: scope.clone(),
            subject: broad.clone(),
            obligation: usage("example.obligation", &format!("item-{index}")),
            contributor: contributor.clone(),
        })
        .collect();
    let contribution = ResolvedGoverningContributionV1 {
        obligations,
        narrowing: vec![ResolvedGoverningNarrowingV1 {
            scope,
            narrower: usage("example.workflow", "narrow"),
            broader: broad,
            contributor,
        }],
        ..empty_contribution()
    };
    assert!(matches!(
        ResolvedGoverningContributionV1::compose([contribution], &BTreeMap::new()),
        Err(GoverningAlgebraErrorV1::RelationTooLarge {
            relation: "accumulated-obligation",
            actual,
            maximum: MAX_GOVERNING_RELATIONS,
        }) if actual == MAX_GOVERNING_RELATIONS + 1
    ));
}

#[test]
fn narrowing_cycle_returns_a_stable_exact_witness() {
    let scope = usage("example.workflow", "scope");
    let a = usage("example.graph", "a");
    let b = usage("example.graph", "b");
    let c = usage("example.graph", "c");
    let d = usage("example.graph", "d");
    let contributor = resource("example.policy", "governing");
    let contribution = ResolvedGoverningContributionV1 {
        narrowing: vec![
            ResolvedGoverningNarrowingV1 {
                scope: scope.clone(),
                narrower: b.clone(),
                broader: a.clone(),
                contributor: contributor.clone(),
            },
            ResolvedGoverningNarrowingV1 {
                scope: scope.clone(),
                narrower: a.clone(),
                broader: b.clone(),
                contributor: contributor.clone(),
            },
            ResolvedGoverningNarrowingV1 {
                scope,
                narrower: d,
                broader: c,
                contributor,
            },
        ],
        ..empty_contribution()
    };
    let error = ResolvedGoverningContributionV1::compose([contribution], &BTreeMap::new())
        .expect_err("narrowing cycle must fail");
    assert!(matches!(
        error,
        GoverningAlgebraErrorV1::NarrowingCycle {
            ref members,
            ref edges,
            ..
        } if members == &vec![a.clone(), b.clone(), a.clone()]
            && edges.len() == 2
            && edges.iter().all(|edge| {
                (edge.broader == a && edge.narrower == b)
                    || (edge.broader == b && edge.narrower == a)
            })
    ));
}

#[test]
fn narrowing_rejects_a_specialization_that_widens_its_broader_policy() {
    let policies = BTreeMap::from([
        (
            resource("example.policy", "broad-policy"),
            policy(&["read"], &[]),
        ),
        (
            resource("example.policy", "narrow-policy"),
            policy(&["read", "write"], &[]),
        ),
    ]);
    let edge = ResolvedGoverningNarrowingV1 {
        scope: usage("example.workflow", "scope"),
        narrower: usage("example.workflow", "narrow"),
        broader: usage("example.workflow", "broad"),
        contributor: resource("example.policy", "governing"),
    };
    let contribution = ResolvedGoverningContributionV1 {
        constraints: vec![
            constraint("broad", "broad-policy", "governing"),
            constraint("narrow", "narrow-policy", "governing"),
        ],
        narrowing: vec![edge.clone()],
        ..empty_contribution()
    };
    assert!(matches!(
        ResolvedGoverningContributionV1::compose([contribution], &policies),
        Err(GoverningAlgebraErrorV1::NarrowingWidens {
            edge: rejected,
            capabilities,
            effects,
        }) if rejected.as_ref() == &edge
            && capabilities == BTreeSet::from([capability("write")])
            && effects.is_empty()
    ));
}

#[test]
fn duplicate_or_contradictory_policy_axes_are_rejected() {
    for invalid in [policy(&["read", "read"], &[]), policy(&["read"], &["read"])] {
        let policy_ref = resource("example.policy", "invalid");
        let contribution = ResolvedGoverningContributionV1 {
            constraints: vec![constraint("subject", "invalid", "governing")],
            ..empty_contribution()
        };
        assert!(matches!(
            ResolvedGoverningContributionV1::compose(
                [contribution],
                &BTreeMap::from([(policy_ref.clone(), invalid)]),
            ),
            Err(GoverningAlgebraErrorV1::InvalidCapabilityPolicy { policy })
                if policy == policy_ref
        ));
    }
}

#[test]
fn private_narrowing_uses_the_same_meet_and_cannot_widen() {
    let package = CapabilityPolicy {
        permitted_capabilities: vec![capability("read"), capability("write")],
        forbidden_capabilities: Vec::new(),
        permitted_effects: vec![EffectClass::Read, EffectClass::Write],
        forbidden_effects: Vec::new(),
    };
    let narrowing = CapabilityNarrowingV1 {
        permitted_capabilities: vec![capability("read")],
        forbidden_capabilities: vec![capability("write")],
        permitted_effects: vec![EffectClass::Read],
        forbidden_effects: vec![EffectClass::Write],
    };
    let effective =
        meet_capability_narrowing(&package, &narrowing).expect("strict narrowing must compose");
    assert!(effective
        .permitted_capabilities
        .contains(&capability("read")));

    let mut widening = narrowing;
    widening.permitted_capabilities = vec![capability("publish")];
    assert!(matches!(
        meet_capability_narrowing(&package, &widening),
        Err(GoverningAlgebraErrorV1::InvalidCapabilityNarrowing)
    ));
}

#[test]
fn shared_page_uses_keep_separate_scopes_and_ceiling_subjects() {
    use crate::records::PackageResourceKindV1 as Kind;
    let guide = usage("example.workflow", "shared");
    let role = PackageResourceUseV1::new(guide.resource.clone(), Kind::Role);
    let a = usage("example.workflow", "a");
    let b = usage("example.workflow", "b");
    let contributor = resource("example.policy", "governing");
    let policy_ref = resource("example.policy", "ceiling");
    let contribution = ResolvedGoverningContributionV1 {
        constraints: [guide.clone(), role.clone()]
            .into_iter()
            .map(|subject| ResolvedGoverningConstraintV1 {
                scope: role.clone(),
                subject,
                policy: policy_ref.clone(),
                contributor: contributor.clone(),
            })
            .collect(),
        order: vec![
            AdmittedScopedOrderRelationV1 {
                scope: guide.clone(),
                purpose: SemanticOrderPurposeV1::Reasoning,
                before: a.clone(),
                after: b.clone(),
                contributor: contributor.clone(),
            },
            AdmittedScopedOrderRelationV1 {
                scope: role.clone(),
                purpose: SemanticOrderPurposeV1::Reasoning,
                before: b,
                after: a,
                contributor,
            },
        ],
        ..empty_contribution()
    };
    let admitted = ResolvedGoverningContributionV1::compose(
        [contribution],
        &BTreeMap::from([(policy_ref, policy(&["read"], &[]))]),
    )
    .expect("opposite edges in different page uses are not a cycle");
    assert_eq!(admitted.ceilings().len(), 2);
    assert_eq!(admitted.order().len(), 2);
    let encoded = serde_json::to_value(&admitted).unwrap();
    let decoded: AdmittedGoverningOrderV1 = serde_json::from_value(encoded).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded, admitted);
    assert_eq!(
        decoded
            .order()
            .iter()
            .map(|edge| edge.scope.clone())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([guide, role])
    );
}
