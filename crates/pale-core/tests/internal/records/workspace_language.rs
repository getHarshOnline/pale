//! Checks the bounded workspace policy algebra, authority conflicts, and portable identities.

use super::{
    DisclosureCeilingV1, WorkspaceAuthorityCeilingV1, WorkspaceAuthorityScopeId,
    WorkspaceControllerCeilingV1, WorkspaceControllerScopeId, WorkspaceDeliveryV1,
    WorkspaceEffectCeilingV1, WorkspaceExclusiveAuthorityClaimV1, WorkspaceId,
    WorkspaceObligationId, WorkspacePolicyCeilingV1, WorkspacePolicyCombinationError,
    WorkspacePropagationV1, WorkspaceResourceId, WorkspaceRightId, WorkspaceRightsCeilingV1,
    WorkspaceVisibilityV1, MAX_WORKSPACE_AUTHORITY_CLAIMS, MAX_WORKSPACE_OBLIGATIONS,
};

fn policy_space() -> Vec<WorkspacePolicyCeilingV1> {
    let mut values = Vec::new();
    for disclosure in [
        DisclosureCeilingV1::Private,
        DisclosureCeilingV1::Protected,
        DisclosureCeilingV1::Public,
    ] {
        for effect in [
            WorkspaceEffectCeilingV1::None,
            WorkspaceEffectCeilingV1::Read,
            WorkspaceEffectCeilingV1::Mutate,
        ] {
            for model_use_permitted in [false, true] {
                for retention_seconds in [0, 1, crate::canon::JSON_SAFE_INTEGER_MAX] {
                    values.push(WorkspacePolicyCeilingV1 {
                        disclosure,
                        effect,
                        model_use_permitted,
                        retention_seconds,
                        rights: WorkspaceRightsCeilingV1 { permitted: vec![] },
                        controllers: WorkspaceControllerCeilingV1 { permitted: vec![] },
                        authorities: WorkspaceAuthorityCeilingV1 { permitted: vec![] },
                    });
                }
            }
        }
    }
    values
}

fn obligation(value: &str) -> WorkspaceObligationId {
    value.parse().expect("test obligation identity is valid")
}

fn claim(scope: &str, resolver: &str) -> WorkspaceExclusiveAuthorityClaimV1 {
    WorkspaceExclusiveAuthorityClaimV1 {
        scope: scope.parse().expect("test authority scope is valid"),
        authority_resolver: resolver.parse().expect("test resolver identity is valid"),
    }
}

#[test]
fn portable_id_rejects_the_first_value_beyond_the_wire_bound() {
    assert!(WorkspaceId::parse(format!("a{}", "b".repeat(254))).is_ok());
    assert!(WorkspaceId::parse(format!("a{}", "b".repeat(255))).is_err());
    let parsed: WorkspaceId = "workspace".parse().unwrap();
    assert_eq!(parsed.as_ref(), "workspace");
    assert_eq!(WorkspaceId::try_from("workspace").unwrap(), parsed);
    assert_eq!(
        WorkspaceId::try_from("workspace".to_string()).unwrap(),
        parsed
    );
}

#[test]
fn retention_rejects_the_first_integer_outside_canonical_json() {
    let value = serde_json::json!({
        "disclosure": "private",
        "effect": "none",
        "model_use_permitted": false,
        "retention_seconds": crate::canon::JSON_SAFE_INTEGER_MAX + 1
        ,"rights":{"permitted":[]},"controllers":{"permitted":[]},"authorities":{"permitted":[]}
    });
    assert!(serde_json::from_value::<WorkspacePolicyCeilingV1>(value).is_err());

    let schema = serde_json::to_value(schemars::schema_for!(WorkspacePolicyCeilingV1))
        .expect("workspace policy schema is JSON");
    assert_eq!(
        schema.pointer("/properties/retention_seconds/maximum"),
        Some(&serde_json::json!(crate::canon::JSON_SAFE_INTEGER_MAX))
    );
}

#[test]
fn policy_meet_obeys_the_declared_product_order() {
    let values = policy_space();
    let identity = WorkspacePolicyCeilingV1 {
        disclosure: DisclosureCeilingV1::Public,
        effect: WorkspaceEffectCeilingV1::Mutate,
        model_use_permitted: true,
        retention_seconds: crate::canon::JSON_SAFE_INTEGER_MAX,
        rights: WorkspaceRightsCeilingV1 { permitted: vec![] },
        controllers: WorkspaceControllerCeilingV1 { permitted: vec![] },
        authorities: WorkspaceAuthorityCeilingV1 { permitted: vec![] },
    };

    for left in &values {
        assert_eq!(left.meet(left), *left);
        assert_eq!(left.meet(&identity), *left);
        for right in &values {
            let meet = left.meet(right);
            assert_eq!(meet, right.meet(left));
            assert_eq!(left.is_narrower_or_equal(right), meet == *left);
            assert!(meet.is_narrower_or_equal(left));
            assert!(meet.is_narrower_or_equal(right));
            for third in &values {
                assert_eq!(left.meet(right).meet(third), left.meet(&right.meet(third)));
            }
        }
    }
}

#[test]
fn independent_set_axes_form_product_meets() {
    let broad = WorkspacePolicyCeilingV1 {
        disclosure: DisclosureCeilingV1::Public,
        effect: WorkspaceEffectCeilingV1::Mutate,
        model_use_permitted: true,
        retention_seconds: 100,
        rights: WorkspaceRightsCeilingV1 {
            permitted: vec![
                WorkspaceRightId::parse("read").unwrap(),
                WorkspaceRightId::parse("derive").unwrap(),
            ],
        },
        controllers: WorkspaceControllerCeilingV1 {
            permitted: vec![
                WorkspaceControllerScopeId::parse("controller-a").unwrap(),
                WorkspaceControllerScopeId::parse("controller-b").unwrap(),
            ],
        },
        authorities: WorkspaceAuthorityCeilingV1 {
            permitted: vec![
                WorkspaceAuthorityScopeId::parse("authority-a").unwrap(),
                WorkspaceAuthorityScopeId::parse("authority-b").unwrap(),
            ],
        },
    };
    let narrow = WorkspacePolicyCeilingV1 {
        rights: WorkspaceRightsCeilingV1 {
            permitted: vec![WorkspaceRightId::parse("read").unwrap()],
        },
        controllers: WorkspaceControllerCeilingV1 {
            permitted: vec![WorkspaceControllerScopeId::parse("controller-a").unwrap()],
        },
        authorities: WorkspaceAuthorityCeilingV1 {
            permitted: vec![WorkspaceAuthorityScopeId::parse("authority-b").unwrap()],
        },
        ..broad.clone()
    };
    assert_eq!(broad.meet(&narrow), narrow);
    assert!(narrow.is_narrower_or_equal(&broad));
    assert!(!broad.is_narrower_or_equal(&narrow));
    let empty = WorkspacePolicyCeilingV1 {
        rights: WorkspaceRightsCeilingV1 { permitted: vec![] },
        controllers: WorkspaceControllerCeilingV1 { permitted: vec![] },
        authorities: WorkspaceAuthorityCeilingV1 { permitted: vec![] },
        ..broad.clone()
    };
    assert_eq!(broad.meet(&empty), empty);
    assert_eq!(
        broad.meet(&narrow).meet(&empty),
        broad.meet(&narrow.meet(&empty))
    );
    assert_eq!(broad.meet(&narrow), narrow.meet(&broad));
    assert_eq!(narrow.meet(&narrow), narrow);
}

#[test]
fn access_orders_only_preserve_or_narrow_authority() {
    let visibilities = [
        WorkspaceVisibilityV1::Private,
        WorkspaceVisibilityV1::Protected,
        WorkspaceVisibilityV1::Public,
    ];
    for left in visibilities {
        assert_eq!(left.meet(left), left);
        for right in visibilities {
            assert_eq!(left.meet(right), right.meet(left));
            assert!(left.meet(right).is_narrower_or_equal(left));
            assert!(left.meet(right).is_narrower_or_equal(right));
            for third in visibilities {
                assert_eq!(left.meet(right).meet(third), left.meet(right.meet(third)));
            }
        }
    }

    for proposed in [WorkspaceDeliveryV1::Interface, WorkspaceDeliveryV1::Source] {
        for base in [WorkspaceDeliveryV1::Interface, WorkspaceDeliveryV1::Source] {
            assert_eq!(
                proposed.is_narrower_or_equal(base),
                proposed == WorkspaceDeliveryV1::Interface || base == WorkspaceDeliveryV1::Source
            );
        }
    }

    for left in [
        WorkspacePropagationV1::Prevent,
        WorkspacePropagationV1::Transitive,
    ] {
        assert_eq!(left.meet(left), left);
        for right in [
            WorkspacePropagationV1::Prevent,
            WorkspacePropagationV1::Transitive,
        ] {
            assert_eq!(left.meet(right), right.meet(left));
            assert!(left.meet(right).is_narrower_or_equal(left));
            assert!(left.meet(right).is_narrower_or_equal(right));
            for third in [
                WorkspacePropagationV1::Prevent,
                WorkspacePropagationV1::Transitive,
            ] {
                assert_eq!(left.meet(right).meet(third), left.meet(right.meet(third)));
            }
        }
    }
}

#[test]
fn obligations_accumulate_as_a_canonical_set_union() {
    let left = vec![obligation("redact"), obligation("retain")];
    let right = vec![obligation("redact"), obligation("review")];
    let third = vec![obligation("audit")];
    let empty = Vec::new();

    assert_eq!(
        WorkspaceObligationId::try_union_sets(&left, &empty).unwrap(),
        left
    );
    assert_eq!(
        WorkspaceObligationId::try_union_sets(&left, &left).unwrap(),
        left
    );
    assert_eq!(
        WorkspaceObligationId::try_union_sets(&left, &right).unwrap(),
        WorkspaceObligationId::try_union_sets(&right, &left).unwrap()
    );
    let left_then_third = WorkspaceObligationId::try_union_sets(&left, &right).unwrap();
    let right_then_third = WorkspaceObligationId::try_union_sets(&right, &third).unwrap();
    assert_eq!(
        WorkspaceObligationId::try_union_sets(&left_then_third, &third).unwrap(),
        WorkspaceObligationId::try_union_sets(&left, &right_then_third).unwrap()
    );
}

#[test]
fn authority_combination_is_canonical_and_conflict_preserving() {
    let left = vec![claim("scope.alpha", "resolver.alpha")];
    let right = vec![claim("scope.beta", "resolver.beta")];
    let third = vec![claim("scope.gamma", "resolver.gamma")];
    let empty = Vec::new();

    assert_eq!(
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&left, &empty).unwrap(),
        left
    );
    assert_eq!(
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&left, &left).unwrap(),
        left
    );
    assert_eq!(
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&left, &right).unwrap(),
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&right, &left).unwrap()
    );
    let left_then_right =
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&left, &right).unwrap();
    let right_then_third =
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&right, &third).unwrap();
    assert_eq!(
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&left_then_right, &third).unwrap(),
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&left, &right_then_third).unwrap()
    );

    let conflict = vec![WorkspaceExclusiveAuthorityClaimV1 {
        scope: WorkspaceAuthorityScopeId::try_from("scope.alpha").unwrap(),
        authority_resolver: WorkspaceResourceId::try_from("resolver.other").unwrap(),
    }];
    assert_eq!(
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&left, &conflict).unwrap_err(),
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&conflict, &left).unwrap_err()
    );
}

#[test]
fn policy_accumulation_accepts_its_bound_and_rejects_the_next_identity() {
    let obligations = (0..MAX_WORKSPACE_OBLIGATIONS)
        .map(|index| obligation(&format!("o{index}")))
        .collect::<Vec<_>>();
    assert_eq!(
        WorkspaceObligationId::try_union_sets(&[], &obligations)
            .unwrap()
            .len(),
        MAX_WORKSPACE_OBLIGATIONS
    );
    let obligation_error =
        WorkspaceObligationId::try_union_sets(&obligations, &[obligation("overflow")]).unwrap_err();
    assert!(matches!(
        obligation_error,
        WorkspacePolicyCombinationError::ObligationLimit {
            observed,
            limit: MAX_WORKSPACE_OBLIGATIONS
        } if observed == MAX_WORKSPACE_OBLIGATIONS + 1
    ));

    let claims = (0..MAX_WORKSPACE_AUTHORITY_CLAIMS)
        .map(|index| claim(&format!("s{index}"), &format!("r{index}")))
        .collect::<Vec<_>>();
    assert_eq!(
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&[], &claims)
            .unwrap()
            .len(),
        MAX_WORKSPACE_AUTHORITY_CLAIMS
    );
    let authority_error = WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(
        &claims,
        &[claim("overflow", "resolver.overflow")],
    )
    .unwrap_err();
    assert!(matches!(
        authority_error,
        WorkspacePolicyCombinationError::AuthorityLimit {
            observed,
            limit: MAX_WORKSPACE_AUTHORITY_CLAIMS
        } if observed == MAX_WORKSPACE_AUTHORITY_CLAIMS + 1
    ));
}
