//! Reusable public-record organizational fixtures for integration tests.

use pale_core::records::*;

use super::responsibility_fixtures::{fixture_id as id, policy_ceiling as policy};

/// Build a namespace-qualified scope reference.
pub(super) fn scope(namespace: &str, local: &str) -> LogicalScopeRefV1 {
    LogicalScopeRefV1 {
        namespace: id(namespace),
        scope_id: id(local),
    }
}

/// Build a namespace-qualified organization reference.
pub(super) fn organization(namespace: &str, local: &str) -> LogicalOrganizationRefV1 {
    LogicalOrganizationRefV1 {
        namespace: id(namespace),
        organization_id: id(local),
    }
}

/// Build a namespace-qualified group reference.
pub(super) fn group(namespace: &str, local: &str) -> OrganizationalGroupRefV1 {
    OrganizationalGroupRefV1 {
        namespace: id(namespace),
        group_id: id(local),
    }
}

/// Build a namespace-qualified tree reference.
pub(super) fn tree(namespace: &str, local: &str) -> DisciplineTreeRefV1 {
    DisciplineTreeRefV1 {
        namespace: id(namespace),
        tree_id: id(local),
    }
}

/// Build a namespace-qualified role reference.
pub(super) fn role(namespace: &str, local: &str) -> OrganizationalRoleRefV1 {
    OrganizationalRoleRefV1 {
        namespace: id(namespace),
        role_id: id(local),
    }
}

/// Build one valid interval.
pub(super) fn interval() -> EffectiveIntervalV1 {
    EffectiveIntervalV1 {
        start: 1,
        end_exclusive: Some(10),
    }
}

/// Build independent authority axes under one explicit owner.
pub(super) fn axes_owned(owner: &str, name: &str) -> OrganizationalAuthorityAxesV1 {
    OrganizationalAuthorityAxesV1 {
        owner: id(&format!("owner.{owner}")),
        controller: id(&format!("controller.{name}")),
        authority: id(&format!("authority.{name}")),
        resources: vec![id(&format!("resource.{name}"))],
        disclosure: DisclosureCeilingV1::Protected,
    }
}

/// Return the complete initial-v1 group-lens right set in canonical order.
pub(super) fn group_lens_rights() -> Vec<GroupLensRightV1> {
    vec![
        GroupLensRightV1::Standards,
        GroupLensRightV1::Review,
        GroupLensRightV1::BoundedBlock,
        GroupLensRightV1::Escalation,
    ]
}

/// Build a complete neutral organizational translation fixture.
pub(super) fn organizational_translation() -> OrganizationalTranslationV1 {
    let scope_a = scope("alpha", "scope");
    let scope_b = scope("beta", "scope");
    let scope_c = scope("gamma", "scope");
    let org_a = organization("alpha", "organization");
    let org_b = organization("beta", "organization");
    let group_a = group("alpha", "group");
    let tree_primary = tree("alpha", "primary-tree");
    let tree_leader = tree("alpha", "leader-tree");
    let tree_beta_primary = tree("beta", "primary-tree");
    let tree_beta_supporting = tree("beta", "supporting-tree");
    let tree_absent = tree("beta", "optional-tree");
    let role_a = role("alpha", "accountable-a");
    let role_b = role("alpha", "accountable-b");
    let role_c = role("beta", "reviewer");
    let provider: OrganizationalProviderId = id("provider.communication");
    let requirement = |owner: &str, class: &str| ResponsibilityEvidenceRequirementV1 {
        owner: id(owner),
        evidence_ref: id(class),
    };
    let service = |contract: &str,
                   provider_scope: LogicalScopeRefV1,
                   consumer_scope: LogicalScopeRefV1,
                   owner: &str| DirectionalServiceContractV1 {
        contract_id: id(contract),
        namespace: id(owner),
        contract_owner: id(&format!("owner.{owner}")),
        provider_scope,
        consumer_scope,
        provider_completion: ProviderCompletionRequirementV1 {
            requirement: requirement(owner, &format!("evidence.{owner}.completion")),
        },
        provider_acceptance: ProviderAcceptanceRequirementV1 {
            requirement: requirement(owner, &format!("evidence.{owner}.provider-acceptance")),
        },
        consumer_acceptance: ConsumerAcceptanceRequirementV1 {
            requirement: requirement(owner, &format!("evidence.{owner}.consumer-acceptance")),
        },
        native_read_back: NativeReadBackRequirementV1 {
            requirement: requirement(owner, &format!("evidence.{owner}.native-read-back")),
        },
        pale_settlement: PaleSettlementRequirementV1 {
            requirement: requirement(owner, &format!("evidence.{owner}.settlement")),
        },
        assignments: vec![
            ServiceAssignmentV1 {
                account: id(&format!("account.{owner}.one")),
                mandate: id(&format!("mandate.{owner}.one")),
                stewards: vec![id("steward.shared"), id(&format!("steward.{owner}"))],
                pods: vec![id("pod.shared"), id(&format!("pod.{owner}"))],
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            ServiceAssignmentV1 {
                account: id(&format!("account.{owner}.two")),
                mandate: id(&format!("mandate.{owner}.two")),
                stewards: vec![id("steward.shared")],
                pods: vec![id("pod.shared")],
                state: EffectiveFactStateV1::Stale,
                interval: interval(),
            },
        ],
        ceiling: policy(owner),
        state: EffectiveFactStateV1::Present,
        interval: interval(),
    };
    OrganizationalTranslationV1 {
        schema: ORGANIZATIONAL_TRANSLATION_V1_SCHEMA.into(),
        schema_version: ORGANIZATIONAL_TRANSLATION_V1_SCHEMA_VERSION,
        scopes: vec![
            LogicalScopeV1 {
                scope: scope_a.clone(),
                axes: axes_owned("alpha", "alpha-scope"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            LogicalScopeV1 {
                scope: scope_b.clone(),
                axes: axes_owned("beta", "beta-scope"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            LogicalScopeV1 {
                scope: scope_c.clone(),
                axes: axes_owned("gamma", "gamma-scope"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
        ],
        organizations: vec![
            LogicalOrganizationV1 {
                organization: org_a.clone(),
                scope: scope_a.clone(),
                axes: axes_owned("alpha", "alpha-organization"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            LogicalOrganizationV1 {
                organization: org_b.clone(),
                scope: scope_b.clone(),
                axes: axes_owned("beta", "beta-organization"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
        ],
        groups: vec![OrganizationalGroupV1 {
            group: group_a.clone(),
            organization: org_a.clone(),
            axes: axes_owned("alpha", "alpha-group"),
            state: EffectiveFactStateV1::Present,
            interval: interval(),
        }],
        discipline_trees: vec![
            DisciplineTreeV1 {
                tree: tree_primary.clone(),
                organization: org_a.clone(),
                axes: axes_owned("alpha", "primary-tree"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            DisciplineTreeV1 {
                tree: tree_leader.clone(),
                organization: org_a.clone(),
                axes: axes_owned("alpha", "leader-tree"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            DisciplineTreeV1 {
                tree: tree_beta_primary.clone(),
                organization: org_b.clone(),
                axes: axes_owned("beta", "primary-tree"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            DisciplineTreeV1 {
                tree: tree_beta_supporting.clone(),
                organization: org_b.clone(),
                axes: axes_owned("beta", "supporting-tree"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            DisciplineTreeV1 {
                tree: tree_absent.clone(),
                organization: org_b.clone(),
                axes: axes_owned("beta", "optional-tree"),
                state: EffectiveFactStateV1::Absent,
                interval: interval(),
            },
        ],
        roles: vec![
            OrganizationalRoleV1 {
                role: role_a.clone(),
                organization: org_a.clone(),
                group: Some(group_a.clone()),
                discipline_tree: Some(tree_primary.clone()),
                role_class: id("role-class.accountable"),
                axes: axes_owned("alpha", "role-a"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            OrganizationalRoleV1 {
                role: role_b.clone(),
                organization: org_a.clone(),
                group: Some(group_a.clone()),
                discipline_tree: Some(tree_leader.clone()),
                role_class: id("role-class.accountable"),
                axes: axes_owned("alpha", "role-b"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            OrganizationalRoleV1 {
                role: role_c.clone(),
                organization: org_b.clone(),
                group: None,
                discipline_tree: None,
                role_class: id("role-class.reviewer"),
                axes: axes_owned("beta", "role-c"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
        ],
        occupant_bindings: vec![OrganizationalOccupantBindingV1 {
            role: role_a.clone(),
            occupant: id("occupant.alpha"),
            issuer: id("issuer.alpha"),
            state: EffectiveFactStateV1::Present,
            interval: interval(),
        }],
        provider_bindings: vec![OrganizationalProviderBindingV1 {
            role: role_b.clone(),
            provider: provider.clone(),
            issuer: id("issuer.provider"),
            state: EffectiveFactStateV1::Present,
            interval: interval(),
        }],
        communication_actor_bindings: vec![CommunicationActorBindingV1 {
            provider: provider.clone(),
            handle: id("actor.canonical"),
            local_resolutions: vec![
                CommunicationActorLocalResolutionV1 {
                    local_node: OrganizationalNodeRefV1::Role {
                        role: role_a.clone(),
                    },
                    authorized_closure: vec![OrganizationalNodeRefV1::Role {
                        role: role_b.clone(),
                    }],
                },
                CommunicationActorLocalResolutionV1 {
                    local_node: OrganizationalNodeRefV1::Role {
                        role: role_c.clone(),
                    },
                    authorized_closure: Vec::new(),
                },
            ],
            issuer: id("issuer.communication"),
            state: EffectiveFactStateV1::Present,
            interval: interval(),
        }],
        leadership: vec![LeadershipParticipationV1 {
            topic: id("topic.shared"),
            participants: vec![role_a.clone(), role_c.clone()],
            affected_scopes: vec![scope_a.clone(), scope_b.clone()],
            issuer: id("issuer.leadership"),
            disclosure: DisclosureCeilingV1::Protected,
            state: EffectiveFactStateV1::Present,
            interval: interval(),
        }],
        relations: vec![
            OrganizationalRelationV1 {
                relation_id: id("relation.coordinates"),
                class: OrganizationalRelationClassV1::Coordinates,
                from: OrganizationalNodeRefV1::Group { group: group_a },
                to: OrganizationalNodeRefV1::DisciplineTree {
                    tree: tree_primary.clone(),
                },
                issuer: id("issuer.coordinates"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            OrganizationalRelationV1 {
                relation_id: id("relation.leader"),
                class: OrganizationalRelationClassV1::Leader,
                from: OrganizationalNodeRefV1::DisciplineTree {
                    tree: tree_leader.clone(),
                },
                to: OrganizationalNodeRefV1::DisciplineTree {
                    tree: tree_primary.clone(),
                },
                issuer: id("issuer.leader"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            OrganizationalRelationV1 {
                relation_id: id("relation.collaboration"),
                class: OrganizationalRelationClassV1::Collaboration,
                from: OrganizationalNodeRefV1::Role {
                    role: role_a.clone(),
                },
                to: OrganizationalNodeRefV1::Role {
                    role: role_b.clone(),
                },
                issuer: id("issuer.collaboration"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            OrganizationalRelationV1 {
                relation_id: id("relation.supporting"),
                class: OrganizationalRelationClassV1::Supporting,
                from: OrganizationalNodeRefV1::DisciplineTree {
                    tree: tree_beta_supporting,
                },
                to: OrganizationalNodeRefV1::DisciplineTree {
                    tree: tree_beta_primary,
                },
                issuer: id("issuer.supporting"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            OrganizationalRelationV1 {
                relation_id: id("relation.atomic-decision"),
                class: OrganizationalRelationClassV1::Collaboration,
                from: OrganizationalNodeRefV1::Role {
                    role: role_a.clone(),
                },
                to: OrganizationalNodeRefV1::Object {
                    object: id("object.atomic-system"),
                },
                issuer: id("issuer.atomic-decision"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            OrganizationalRelationV1 {
                relation_id: id("relation.atomic-native-gate"),
                class: OrganizationalRelationClassV1::Acceptance,
                from: OrganizationalNodeRefV1::Role {
                    role: role_b.clone(),
                },
                to: OrganizationalNodeRefV1::Object {
                    object: id("object.atomic-system"),
                },
                issuer: id("issuer.atomic-native-gate"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
        ],
        objects: vec![
            OrganizationalObjectV1 {
                object_id: id("object.named-scope"),
                namespace: id("alpha"),
                scope: scope_a.clone(),
                organization: org_a.clone(),
                object_class: id("object-class.named-scope"),
                axes: axes_owned("alpha", "object-named"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            OrganizationalObjectV1 {
                object_id: id("object.atomic-system"),
                namespace: id("alpha"),
                scope: scope_a.clone(),
                organization: org_a.clone(),
                object_class: id("object-class.atomic-system"),
                axes: axes_owned("alpha", "object-atomic"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
        ],
        population_rules: vec![
            ObjectRolePopulationRuleV1 {
                object_class: id("object-class.named-scope"),
                accountable_role_class: id("role-class.named-scope-owner"),
                issuer: id("issuer.population"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            ObjectRolePopulationRuleV1 {
                object_class: id("object-class.atomic-system"),
                accountable_role_class: id("role-class.atomic-owner"),
                issuer: id("issuer.population"),
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
        ],
        group_lenses: vec![GroupLensAuthorityProfileV1 {
            profile_id: id("lens.quality"),
            holder: role_a.clone(),
            alignment_relation: id("relation.collaboration"),
            lens: id("lens-class.quality"),
            review_owner: Some(role_b.clone()),
            standards: vec![id("standard.quality")],
            block_conditions: vec![id("condition.quality-gate")],
            escalation_resolver: Some(role_b.clone()),
            rights: group_lens_rights(),
            issuer: id("issuer.lens"),
            state: EffectiveFactStateV1::Present,
            interval: interval(),
        }],
        services: vec![
            service("service.shared", scope_a.clone(), scope_b.clone(), "alpha"),
            service("service.shared", scope_b.clone(), scope_a.clone(), "beta"),
        ],
        native_projections: vec![
            OrganizationalNativeProjectionV1 {
                projection_id: id("projection.alpha.work"),
                subject: OrganizationalNodeRefV1::Organization {
                    organization: org_a,
                },
                class: ResponsibilityProjectionClassV1::AccountableWork,
                native_owner: id("native-owner.alpha"),
                native_refs: vec![id("native.alpha.one"), id("native.alpha.two")],
                disclosure: DisclosureCeilingV1::Protected,
                state: EffectiveFactStateV1::Present,
                interval: interval(),
            },
            OrganizationalNativeProjectionV1 {
                projection_id: id("projection.beta.relationship"),
                subject: OrganizationalNodeRefV1::Organization {
                    organization: org_b,
                },
                class: ResponsibilityProjectionClassV1::Relationship,
                native_owner: id("native-owner.beta"),
                native_refs: Vec::new(),
                disclosure: DisclosureCeilingV1::Private,
                state: EffectiveFactStateV1::Stale,
                interval: interval(),
            },
        ],
        inspection: Some(OrganizationalInspectionV1 {
            inspection_id: id("inspection.complete"),
            principal: id("principal.viewer"),
            disclosure: DisclosureCeilingV1::Protected,
            view_provider: None,
            coordinates: InspectionCoordinateClassV1::all().to_vec(),
            states: EffectiveFactStateV1::all().to_vec(),
        }),
    }
}
