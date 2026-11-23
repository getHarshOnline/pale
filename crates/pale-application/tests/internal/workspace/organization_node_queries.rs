//! Compiler-backed endpoint and incident-page laws, without a second source fixture.

use super::super::tests::{compile, rows};
use super::*;
use crate::organization_fixtures as org;

fn collect<'a>(
    owner: &'a PortableWorkspaceUniverse,
    node: &OrganizationalNodeRefV1,
    incident: bool,
    limit: u16,
) -> (
    Vec<OrganizationInspectionEntry<'a>>,
    usize,
    Option<OrganizationEndpointLookup>,
) {
    let mut after = None;
    let mut found = Vec::new();
    let mut inspected = 0;
    for _ in 0..10_000 {
        let page = if incident {
            owner.inspect_organization_incident_relations(node, after.as_ref(), limit, limit)
        } else {
            owner.inspect_organization_endpoint(node, after.as_ref(), limit, limit)
        }
        .unwrap();
        assert!(page.inspected() <= limit);
        assert!(page.entries().len() <= usize::from(limit));
        inspected += usize::from(page.inspected());
        found.extend_from_slice(page.entries());
        after = page.continuation().cloned();
        if after.is_none() {
            return (found, inspected, page.endpoint_lookup());
        }
    }
    panic!("bounded fixture query did not exhaust");
}

fn value(record: OrganizationInspectionRecord<'_>) -> serde_json::Value {
    match record {
        OrganizationInspectionRecord::Scope(v) => serde_json::to_value(v),
        OrganizationInspectionRecord::Role(v) => serde_json::to_value(v),
        OrganizationInspectionRecord::OccupantBinding(v) => serde_json::to_value(v),
        OrganizationInspectionRecord::Relation(v) => serde_json::to_value(v),
        OrganizationInspectionRecord::DerivedRelation(v) => serde_json::to_value(v),
        OrganizationInspectionRecord::Service(v) => serde_json::to_value(v),
        _ => panic!("unexpected fixture result family"),
    }
    .unwrap()
}

#[test]
fn incident_services_keep_full_parallel_contracts_and_charge_sparse_scope_scans() {
    let mut source = org::organizational_translation();
    let mut parallel = source.services[0].clone();
    parallel.contract_id = "service.parallel".parse().unwrap();
    source.services.push(parallel);
    let owner = compile(Some(&source), None);
    let all = rows(&owner, 256);
    for scope in &source.scopes {
        let expected: Vec<_> = all
            .iter()
            .filter(|entry| match entry.record() {
                OrganizationInspectionRecord::Service(v) => {
                    v.provider_scope == scope.scope || v.consumer_scope == scope.scope
                }
                _ => false,
            })
            .map(|entry| value(entry.record()))
            .collect();
        let mut found = Vec::new();
        let mut after = None;
        let mut inspected = 0;
        let mut complete = false;
        for _ in 0..16 {
            let page = owner
                .inspect_organization_incident_services(&scope.scope, after.as_ref(), 1, 1)
                .unwrap();
            assert_eq!(page.endpoint_lookup(), None);
            inspected += usize::from(page.inspected());
            found.extend(page.entries().iter().map(|entry| value(entry.record())));
            if expected.is_empty() && page.inspected() == 1 {
                assert!(page.entries().is_empty());
                assert!(page.continuation().is_some());
            }
            after = page.continuation().cloned();
            if after.is_none() {
                complete = true;
                break;
            }
        }
        assert!(complete);
        assert_eq!(inspected, source.services.len());
        assert_eq!(found, expected);
        let whole = owner
            .inspect_organization_incident_services(&scope.scope, None, 256, 256)
            .unwrap();
        assert_eq!(
            whole
                .entries()
                .iter()
                .map(|entry| value(entry.record()))
                .collect::<Vec<_>>(),
            expected
        );
    }
    let matched = owner
        .inspect_organization_incident_services(&source.services[0].provider_scope, None, 256, 256)
        .unwrap();
    assert_eq!(
        matched.entries().len(),
        3,
        "Each contract appears once despite shared endpoints"
    );
}

#[test]
fn incident_service_continuation_keeps_scope_query_owner_and_limit_custody() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    let other = compile(Some(&source), None);
    let scope = &source.services[0].provider_scope;
    let node = OrganizationalNodeRefV1::Scope {
        scope: scope.clone(),
    };
    let page = owner
        .inspect_organization_incident_services(scope, None, 1, 1)
        .unwrap();
    let cursor = page.continuation().unwrap();
    for result in [
        owner.inspect_organization(Some(cursor), 1, 1),
        owner.inspect_organization_endpoint(&node, Some(cursor), 1, 1),
        owner.inspect_organization_incident_relations(&node, Some(cursor), 1, 1),
        owner.inspect_organization_incident_services(
            &org::scope("gamma", "scope"),
            Some(cursor),
            1,
            1,
        ),
        other.inspect_organization_incident_services(scope, Some(cursor), 1, 1),
        owner.inspect_organization_incident_services(scope, Some(cursor), 2, 1),
    ] {
        assert!(matches!(
            result,
            Err(OrganizationInspectionError::ContinuationMismatch)
        ));
    }
    for first in [
        owner.inspect_organization(None, 1, 1).unwrap(),
        owner
            .inspect_organization_incident_relations(&node, None, 1, 1)
            .unwrap(),
    ] {
        assert!(matches!(
            owner.inspect_organization_incident_services(scope, first.continuation(), 1, 1),
            Err(OrganizationInspectionError::ContinuationMismatch)
        ));
    }
    assert!(owner
        .inspect_organization_incident_services(scope, Some(cursor), 1, 1)
        .is_ok());
    let absent = compile(None, None);
    let page = absent
        .inspect_organization_incident_services(scope, None, 1, 1)
        .unwrap();
    assert!(!page.organization_selected());
    assert_eq!(page.inspected(), 0);
    assert!(page.entries().is_empty());
    assert!(page.continuation().is_none());
    assert_eq!(page.endpoint_lookup(), None);
}

#[test]
fn sparse_exact_scope_queries_charge_nonmatches_and_distinguish_search_from_absence() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    for scope in &source.scopes {
        let node = OrganizationalNodeRefV1::Scope {
            scope: scope.scope.clone(),
        };
        let (found, visits, state) = collect(&owner, &node, false, 1);
        assert_eq!(visits, source.scopes.len());
        assert_eq!(state, Some(OrganizationEndpointLookup::Found));
        assert_eq!(found.len(), 1);
        assert_eq!(
            value(found[0].record()),
            serde_json::to_value(scope).unwrap()
        );
    }
    let absent = OrganizationalNodeRefV1::Scope {
        scope: org::scope("alpha", "unselected-scope"),
    };
    let first = owner
        .inspect_organization_endpoint(&absent, None, 1, 1)
        .unwrap();
    assert_eq!(first.inspected(), 1);
    assert!(first.entries().is_empty());
    assert!(first.continuation().is_some());
    assert_eq!(
        first.endpoint_lookup(),
        Some(OrganizationEndpointLookup::SearchIncomplete)
    );
    let (found, visits, state) = collect(&owner, &absent, false, 1);
    assert!(found.is_empty());
    assert_eq!(visits, source.scopes.len());
    assert_eq!(state, Some(OrganizationEndpointLookup::NotFound));
}

#[test]
fn selection_external_reference_and_negative_fact_state_are_independent() {
    let absent_owner = compile(None, None);
    let local = OrganizationalNodeRefV1::Role {
        role: org::role("alpha", "accountable-a"),
    };
    let absent = absent_owner
        .inspect_organization_endpoint(&local, None, 1, 1)
        .unwrap();
    assert!(!absent.organization_selected());
    assert_eq!(absent.inspected(), 0);
    assert_eq!(
        absent.endpoint_lookup(),
        Some(OrganizationEndpointLookup::NotFound)
    );
    let mut source = org::organizational_translation();
    source.roles[0].state = EffectiveFactStateV1::Absent;
    let node = OrganizationalNodeRefV1::Role {
        role: source.roles[0].role.clone(),
    };
    let owner = compile(Some(&source), None);
    let (found, _, state) = collect(&owner, &node, false, 1);
    assert_eq!(state, Some(OrganizationEndpointLookup::Found));
    assert!(
        matches!(found[0].record(), OrganizationInspectionRecord::Role(v) if v.state == EffectiveFactStateV1::Absent)
    );
    let external = OrganizationalNodeRefV1::NativeResource {
        resource: "native.not-observed".parse().unwrap(),
    };
    let page = owner
        .inspect_organization_endpoint(&external, None, 1, 1)
        .unwrap();
    assert!(page.organization_selected());
    assert_eq!(page.inspected(), 0);
    assert!(page.entries().is_empty());
    assert!(page.continuation().is_none());
    assert_eq!(
        page.endpoint_lookup(),
        Some(OrganizationEndpointLookup::ExternalReference)
    );
}

#[test]
fn repeated_occupant_witnesses_preserve_roles_intervals_and_unmapped_record_sources() {
    let mut source = org::organizational_translation();
    let original = source.occupant_bindings[0].clone();
    let mut later = original.clone();
    later.interval = EffectiveIntervalV1 {
        start: 11,
        end_exclusive: Some(20),
    };
    let mut other_role = original.clone();
    other_role.role = org::role("alpha", "accountable-b");
    source.occupant_bindings.extend([later, other_role]);
    let node = OrganizationalNodeRefV1::Occupant {
        occupant: original.occupant.clone(),
    };
    let owner = compile(Some(&source), None);
    let (tiny, inspected, state) = collect(&owner, &node, false, 1);
    let (whole, _, _) = collect(&owner, &node, false, 256);
    assert_eq!(tiny.len(), 3);
    assert_eq!(inspected, source.occupant_bindings.len());
    assert_eq!(state, Some(OrganizationEndpointLookup::Found));
    assert_eq!(
        tiny.iter().map(|v| value(v.record())).collect::<Vec<_>>(),
        whole.iter().map(|v| value(v.record())).collect::<Vec<_>>()
    );
    for entry in tiny {
        assert!(matches!(
            entry.sources(None, 1, 1).unwrap(),
            OrganizationSources::UnmappedRecord
        ));
    }
}

#[test]
fn incident_pages_match_forward_records_and_preserve_self_loops_parallel_and_derived_edges() {
    let mut source = org::organizational_translation();
    let mut relation = source
        .relations
        .iter()
        .find(|v| v.class == OrganizationalRelationClassV1::Collaboration)
        .unwrap()
        .clone();
    let node = relation.from.clone();
    relation.relation_id = "relation.parallel".parse().unwrap();
    source.relations.push(relation.clone());
    relation.relation_id = "relation.self".parse().unwrap();
    relation.to = node.clone();
    source.relations.push(relation);
    let owner = compile(Some(&source), None);
    let reference = rows(&owner, 256)
        .into_iter()
        .filter(|entry| match entry.record() {
            OrganizationInspectionRecord::Relation(v) => v.from == node || v.to == node,
            OrganizationInspectionRecord::DerivedRelation(v) => v.from == node || v.to == node,
            _ => false,
        })
        .map(|entry| value(entry.record()))
        .collect::<Vec<_>>();
    let (actual, visits, state) = collect(&owner, &node, true, 1);
    assert_eq!(state, None);
    let all = rows(&owner, 256);
    assert_eq!(
        visits,
        all.iter()
            .filter(|entry| matches!(
                entry.record(),
                OrganizationInspectionRecord::Relation(_)
                    | OrganizationInspectionRecord::DerivedRelation(_)
            ))
            .count()
    );
    assert_eq!(
        actual
            .iter()
            .map(|entry| value(entry.record()))
            .collect::<Vec<_>>(),
        reference
    );
    assert_eq!(
        actual
            .iter()
            .filter(
                |entry| matches!(entry.record(), OrganizationInspectionRecord::Relation(v)
        if v.relation_id.as_str() == "relation.self")
            )
            .count(),
        1
    );
    assert!(actual.iter().any(|entry| matches!(
        entry.record(),
        OrganizationInspectionRecord::DerivedRelation(_)
    )));
}

#[test]
fn external_incident_evidence_and_empty_neighborhood_do_not_claim_endpoint_existence() {
    let mut source = org::organizational_translation();
    let external = OrganizationalNodeRefV1::NativeResource {
        resource: "native.unobserved".parse().unwrap(),
    };
    let mut presence = source.relations[0].clone();
    presence.relation_id = "relation.native-presence".parse().unwrap();
    presence.class = OrganizationalRelationClassV1::NativePresence;
    presence.to = external.clone();
    source.relations.push(presence);
    let owner = compile(Some(&source), None);
    let (relations, _, state) = collect(&owner, &external, true, 1);
    assert_eq!(state, None);
    assert!(!relations.is_empty());
    assert!(relations.iter().all(|entry| match entry.record() {
        OrganizationInspectionRecord::Relation(v) => v.from == external || v.to == external,
        OrganizationInspectionRecord::DerivedRelation(v) => v.from == external || v.to == external,
        _ => false,
    }));
    let (_, _, state) = collect(&owner, &external, false, 1);
    assert_eq!(state, Some(OrganizationEndpointLookup::ExternalReference));
    let unknown = OrganizationalNodeRefV1::Role {
        role: org::role("alpha", "unknown-role"),
    };
    let (relations, _, state) = collect(&owner, &unknown, true, 1);
    assert!(relations.is_empty());
    assert_eq!(state, None);
    let (_, _, state) = collect(&owner, &unknown, false, 1);
    assert_eq!(state, Some(OrganizationEndpointLookup::NotFound));
}

#[test]
fn query_endpoint_limits_and_exact_compilation_all_fence_continuation() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    let other = compile(Some(&source), None);
    let node = OrganizationalNodeRefV1::Role {
        role: source.roles[0].role.clone(),
    };
    let different = OrganizationalNodeRefV1::Role {
        role: org::role("alpha", "missing"),
    };
    let first = owner
        .inspect_organization_endpoint(&node, None, 1, 1)
        .unwrap();
    let after = first.continuation().unwrap();
    for result in [
        owner.inspect_organization(Some(after), 1, 1),
        owner.inspect_organization_incident_relations(&node, Some(after), 1, 1),
        owner.inspect_organization_endpoint(&different, Some(after), 1, 1),
        owner.inspect_organization_endpoint(&node, Some(after), 2, 1),
        owner.inspect_organization_endpoint(&node, Some(after), 1, 2),
        other.inspect_organization_endpoint(&node, Some(after), 1, 1),
    ] {
        assert!(matches!(
            result,
            Err(OrganizationInspectionError::ContinuationMismatch)
        ));
    }
    assert!(matches!(
        owner.inspect_organization_endpoint(&node, Some(after), 0, 1),
        Err(OrganizationInspectionError::InvalidLimit)
    ));
    assert!(owner
        .inspect_organization_endpoint(&node, Some(after), 1, 1)
        .is_ok());
}

#[test]
fn found_record_handles_reuse_exact_original_source_and_cumulative_found_survives_terminal_page() {
    let mut source = org::organizational_translation();
    source.roles.reverse();
    let node = OrganizationalNodeRefV1::Role {
        role: source.roles[0].role.clone(),
    };
    let owner = compile(Some(&source), None);
    let (found, _, state) = collect(&owner, &node, false, 1);
    assert_eq!(state, Some(OrganizationEndpointLookup::Found));
    let retained = found[0].retain();
    let resolved = owner.resolve_organization_record(&retained).unwrap();
    let mut after = None;
    let mut definition_paths = Vec::new();
    let mut exhausted = false;
    for _ in 0..10_000 {
        let OrganizationSources::Named { page, continuation } =
            resolved.sources(after.as_ref(), 1, 1).unwrap()
        else {
            panic!()
        };
        definition_paths.extend(
            page.occurrences()
                .iter()
                .filter(|v| v.role() == crate::workspace::AuthoringOccurrenceRole::Definition)
                .map(|v| v.path().to_vec()),
        );
        after = continuation;
        if after.is_none() {
            exhausted = true;
            break;
        }
    }
    assert!(exhausted, "source query must exhaust its actual fixture");
    assert_eq!(definition_paths.len(), 1);
    assert_eq!(
        definition_paths[0][1],
        crate::workspace::AuthoringInstancePathSegment::Index(0)
    );
    let other = compile(Some(&source), None);
    assert!(matches!(
        other.resolve_organization_record(&retained),
        Err(OrganizationInspectionError::RecordOwnerMismatch)
    ));
}
