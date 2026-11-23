//! Owned endpoint/incident query behavior over the same admitted organization fixture.

use super::*;

#[test]
fn incident_service_pages_preserve_direction_requirements_and_owned_sources() {
    let owner = compile(inputs::documents());
    let all = entries(&owner, 256);
    for namespace in ["alpha", "beta", "gamma"] {
        let scope = OrganizationScopeRef {
            namespace: namespace.into(),
            scope_id: "scope".into(),
        };
        let expected: Vec<_> = all
            .iter()
            .filter(|entry| match entry.row() {
                OrganizationRow::Service { contract, .. } => {
                    contract.provider_scope == scope || contract.consumer_scope == scope
                }
                _ => false,
            })
            .map(OrganizationEntry::row)
            .collect();
        let mut found = Vec::new();
        let mut cursor = None;
        let mut inspected = 0;
        let mut complete = false;
        for _ in 0..16 {
            let page = owner
                .inspect_organization_incident_services(&scope, cursor.as_ref(), 1, 1, BYTES)
                .unwrap();
            assert!(page.belongs_to(&owner));
            assert_eq!(page.endpoint_lookup(), None);
            inspected += usize::from(page.inspected());
            for entry in page.entries() {
                assert!(entry.handle().belongs_to(&owner));
                assert!(matches!(
                    entry
                        .handle()
                        .sources(None, 256, 256, BYTES)
                        .unwrap()
                        .mapping(),
                    OrganizationSourceMapping::Named { .. }
                ));
                found.push(entry.clone());
            }
            if namespace == "gamma" && page.inspected() == 1 {
                assert!(page.entries().is_empty());
                assert!(page.continuation().is_some());
            }
            cursor = page.continuation().cloned();
            if cursor.is_none() {
                complete = true;
                break;
            }
        }
        assert!(complete);
        assert_eq!(inspected, 2);
        assert_eq!(
            found.iter().map(OrganizationEntry::row).collect::<Vec<_>>(),
            expected
        );
        let whole = owner
            .inspect_organization_incident_services(&scope, None, 256, 256, BYTES)
            .unwrap();
        assert_eq!(
            whole
                .entries()
                .iter()
                .map(OrganizationEntry::row)
                .collect::<Vec<_>>(),
            expected
        );
        for entry in found {
            let OrganizationRow::Service {
                contract,
                provider_completion,
                provider_acceptance,
                consumer_acceptance,
                native_read_back,
                pale_settlement,
                ..
            } = entry.row()
            else {
                panic!()
            };
            assert_ne!(contract.provider_scope, contract.consumer_scope);
            let evidence: std::collections::HashSet<_> = [
                provider_completion,
                provider_acceptance,
                consumer_acceptance,
                native_read_back,
                pale_settlement,
            ]
            .into_iter()
            .map(|v| &v.evidence_ref)
            .collect();
            assert_eq!(evidence.len(), 5);
        }
    }
}

#[test]
fn incident_services_reject_wrong_cursor_scope_and_copy_without_advancing() {
    let owner = compile(inputs::documents());
    let other = compile(inputs::documents());
    let scope = OrganizationScopeRef {
        namespace: "alpha".into(),
        scope_id: "scope".into(),
    };
    let node = OrganizationNode::Scope(scope.clone());
    let first = owner
        .inspect_organization_incident_services(&scope, None, 1, 1, BYTES)
        .unwrap();
    let cursor = first.continuation().unwrap();
    for result in [
        owner.inspect_organization(Some(cursor), 1, 1, BYTES),
        owner.inspect_organization_incident_relations(&node, Some(cursor), 1, 1, BYTES),
        owner.inspect_organization_endpoint(&node, Some(cursor), 1, 1, BYTES),
        other.inspect_organization_incident_services(&scope, Some(cursor), 1, 1, BYTES),
        owner.inspect_organization_incident_services(
            &OrganizationScopeRef {
                namespace: "beta".into(),
                scope_id: "scope".into(),
            },
            Some(cursor),
            1,
            1,
            BYTES,
        ),
        owner.inspect_organization_incident_services(&scope, Some(cursor), 1, 2, BYTES),
        owner.inspect_organization_incident_services(&scope, Some(cursor), 1, 1, BYTES - 1),
    ] {
        assert_eq!(
            result.unwrap_err().diagnostic().code(),
            "workspace_organization_custody_mismatch"
        );
    }
    for page in [
        owner.inspect_organization(None, 1, 1, BYTES).unwrap(),
        owner
            .inspect_organization_incident_relations(&node, None, 1, 1, BYTES)
            .unwrap(),
    ] {
        assert_eq!(
            owner
                .inspect_organization_incident_services(&scope, page.continuation(), 1, 1, BYTES)
                .unwrap_err()
                .diagnostic()
                .code(),
            "workspace_organization_custody_mismatch"
        );
    }
    assert!(owner
        .inspect_organization_incident_services(&scope, Some(cursor), 1, 1, BYTES)
        .is_ok());
    for _ in 0..2 {
        assert_eq!(
            owner
                .inspect_organization_incident_services(&scope, None, 1, 1, 1)
                .unwrap_err()
                .diagnostic()
                .code(),
            "workspace_organization_copy_budget"
        );
    }
    // Make the second existing contract's copied payload larger without changing
    // its full identity. A retained continuation must survive a later copy failure.
    let mut documents = inputs::documents();
    let document = documents
        .iter_mut()
        .find(|(label, _)| label == "organization.toml")
        .unwrap();
    let text = String::from_utf8(document.1.clone()).unwrap();
    let original = "contract_owner = \"owner.beta\"";
    assert_eq!(text.matches(original).count(), 1);
    document.1 = text
        .replace(
            original,
            &format!("contract_owner = \"owner.beta.{}\"", "x".repeat(128)),
        )
        .into_bytes();
    let larger = compile(documents);
    let initial = larger
        .inspect_organization_incident_services(&scope, None, 1, 1, BYTES)
        .unwrap();
    let bytes = initial.copied_string_bytes();
    let bounded = larger
        .inspect_organization_incident_services(&scope, None, 1, 1, bytes)
        .unwrap();
    let retained = bounded.continuation().unwrap();
    for _ in 0..2 {
        assert_eq!(
            larger
                .inspect_organization_incident_services(&scope, Some(retained), 1, 1, bytes)
                .unwrap_err()
                .diagnostic()
                .code(),
            "workspace_organization_copy_budget"
        );
    }
    assert!(bounded.entries()[0].handle().belongs_to(&larger));
    let invalid = OrganizationScopeRef {
        namespace: "not a namespace".into(),
        scope_id: "scope".into(),
    };
    assert_eq!(
        owner
            .inspect_organization_incident_services(&invalid, None, 1, 1, BYTES)
            .unwrap_err()
            .diagnostic()
            .code(),
        "workspace_organization_invalid_endpoint"
    );
}

fn query(
    owner: &PortableWorkspaceCompilation,
    node: &OrganizationNode,
    incident: bool,
    limit: u16,
) -> (
    Vec<OrganizationEntry>,
    usize,
    Option<OrganizationEndpointLookup>,
) {
    let mut after = None;
    let mut rows = Vec::new();
    let mut inspected = 0;
    for _ in 0..10_000 {
        let page = if incident {
            owner.inspect_organization_incident_relations(node, after.as_ref(), limit, limit, BYTES)
        } else {
            owner.inspect_organization_endpoint(node, after.as_ref(), limit, limit, BYTES)
        }
        .unwrap();
        assert!(page.belongs_to(owner));
        assert!(page.inspected() <= limit);
        assert!(page.entries().len() <= usize::from(limit));
        assert!(page
            .entries()
            .iter()
            .all(|entry| entry.handle().belongs_to(owner)));
        inspected += usize::from(page.inspected());
        rows.extend_from_slice(page.entries());
        after = page.continuation().cloned();
        if after.is_none() {
            return (rows, inspected, page.endpoint_lookup());
        }
    }
    panic!("bounded actual SDK query did not exhaust");
}

#[test]
fn all_nine_endpoint_variants_preserve_owned_values_and_external_reference_meaning() {
    let owner = compile(inputs::documents());
    let all = entries(&owner, 256);
    let mut kinds = std::collections::HashSet::new();
    for expected in &all {
        let node = match expected.row() {
            OrganizationRow::Scope { scope, .. } => OrganizationNode::Scope(scope.clone()),
            OrganizationRow::Organization { organization, .. } => {
                OrganizationNode::Organization(organization.clone())
            }
            OrganizationRow::Group { group, .. } => OrganizationNode::Group(group.clone()),
            OrganizationRow::DisciplineTree { tree, .. } => {
                OrganizationNode::DisciplineTree(tree.clone())
            }
            OrganizationRow::Role { role, .. } => OrganizationNode::Role(role.clone()),
            OrganizationRow::OccupantBinding { occupant, .. } => {
                OrganizationNode::Occupant(occupant.clone())
            }
            OrganizationRow::ProviderBinding { provider, .. } => {
                OrganizationNode::Provider(provider.clone())
            }
            OrganizationRow::Object { object_id, .. } => {
                OrganizationNode::Object(object_id.clone())
            }
            _ => continue,
        };
        kinds.insert(std::mem::discriminant(&node));
        let (tiny, visited, state) = query(&owner, &node, false, 1);
        let (whole, whole_visited, _) = query(&owner, &node, false, 256);
        assert_eq!(visited, whole_visited);
        assert_eq!(state, Some(OrganizationEndpointLookup::Found));
        assert!(tiny.iter().any(|actual| actual.row() == expected.row()));
        assert_eq!(
            tiny.iter().map(OrganizationEntry::row).collect::<Vec<_>>(),
            whole.iter().map(OrganizationEntry::row).collect::<Vec<_>>()
        );
    }
    let external = OrganizationNode::NativeResource("native.unobserved".into());
    kinds.insert(std::mem::discriminant(&external));
    assert_eq!(kinds.len(), 9);
    let (rows, visited, state) = query(&owner, &external, false, 1);
    assert!(rows.is_empty());
    assert_eq!(visited, 0);
    assert_eq!(state, Some(OrganizationEndpointLookup::ExternalReference));
}

#[test]
fn incident_results_match_the_complete_forward_relation_without_rederivation() {
    let owner = compile(inputs::documents());
    let all = entries(&owner, 256);
    let node = OrganizationNode::Role(OrganizationRoleRef {
        namespace: "alpha".into(),
        role_id: "accountable-a".into(),
    });
    let expected: Vec<_> = all
        .iter()
        .filter(|entry| match entry.row() {
            OrganizationRow::Relation { from, to, .. }
            | OrganizationRow::DerivedRelation { from, to, .. } => from == &node || to == &node,
            _ => false,
        })
        .map(OrganizationEntry::row)
        .collect();
    let (found, visits, state) = query(&owner, &node, true, 1);
    assert_eq!(state, None);
    assert_eq!(
        found.iter().map(OrganizationEntry::row).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(
        visits,
        all.iter()
            .filter(|entry| matches!(
                entry.row(),
                OrganizationRow::Relation { .. } | OrganizationRow::DerivedRelation { .. }
            ))
            .count()
    );
    assert!(found
        .iter()
        .any(|entry| matches!(entry.row(), OrganizationRow::DerivedRelation { .. })));
}

#[test]
fn incomplete_empty_lookup_is_not_absence_and_query_changes_reject_continuation() {
    let owner = compile(inputs::documents());
    let other = compile(inputs::documents());
    let node = OrganizationNode::Scope(OrganizationScopeRef {
        namespace: "alpha".into(),
        scope_id: "missing".into(),
    });
    let page = owner
        .inspect_organization_endpoint(&node, None, 1, 1, BYTES)
        .unwrap();
    assert!(page.organization_selected());
    assert!(page.entries().is_empty());
    assert_eq!(page.inspected(), 1);
    assert_eq!(
        page.endpoint_lookup(),
        Some(OrganizationEndpointLookup::SearchIncomplete)
    );
    let cursor = page.continuation().unwrap();
    let changed = OrganizationNode::Scope(OrganizationScopeRef {
        namespace: "beta".into(),
        scope_id: "missing".into(),
    });
    for result in [
        owner.inspect_organization(Some(cursor), 1, 1, BYTES),
        owner.inspect_organization_incident_relations(&node, Some(cursor), 1, 1, BYTES),
        owner.inspect_organization_endpoint(&changed, Some(cursor), 1, 1, BYTES),
        owner.inspect_organization_endpoint(&node, Some(cursor), 2, 1, BYTES),
        owner.inspect_organization_endpoint(&node, Some(cursor), 1, 2, BYTES),
        owner.inspect_organization_endpoint(&node, Some(cursor), 1, 1, BYTES - 1),
        other.inspect_organization_endpoint(&node, Some(cursor), 1, 1, BYTES),
    ] {
        assert_eq!(
            result.unwrap_err().diagnostic().code(),
            "workspace_organization_custody_mismatch"
        );
    }
    let (rows, _, state) = query(&owner, &node, false, 1);
    assert!(rows.is_empty());
    assert_eq!(state, Some(OrganizationEndpointLookup::NotFound));
}

#[test]
fn selector_parsing_and_copy_failure_use_canonical_errors_without_advancing_query() {
    let owner = compile(inputs::documents());
    let invalid = OrganizationNode::Role(OrganizationRoleRef {
        namespace: "not a namespace".into(),
        role_id: "x".into(),
    });
    assert_eq!(
        owner
            .inspect_organization_endpoint(&invalid, None, 1, 1, BYTES)
            .unwrap_err()
            .diagnostic()
            .code(),
        "workspace_organization_invalid_endpoint"
    );
    let node = OrganizationNode::Role(OrganizationRoleRef {
        namespace: "alpha".into(),
        role_id: "accountable-b".into(),
    });
    let first = owner
        .inspect_organization_endpoint(&node, None, 1, 1, 1)
        .unwrap();
    assert!(first.entries().is_empty());
    let cursor = first.continuation().unwrap();
    for _ in 0..2 {
        assert_eq!(
            owner
                .inspect_organization_endpoint(&node, Some(cursor), 1, 1, 1)
                .unwrap_err()
                .diagnostic()
                .code(),
            "workspace_organization_copy_budget"
        );
    }
    let (found, _, state) = query(&owner, &node, false, 1);
    assert_eq!(state, Some(OrganizationEndpointLookup::Found));
    assert_eq!(found.len(), 1);
}

#[test]
fn source_and_detail_handles_keep_the_same_owner_and_unmapped_binding_does_not_become_named() {
    let owner = compile(inputs::documents());
    let node = OrganizationNode::Role(OrganizationRoleRef {
        namespace: "alpha".into(),
        role_id: "accountable-a".into(),
    });
    let (found, _, _) = query(&owner, &node, false, 1);
    let role = found[0].handle();
    let source = role.sources(None, 256, 256, BYTES).unwrap();
    assert!(matches!(
        source.mapping(),
        OrganizationSourceMapping::Named { .. }
    ));
    assert!(matches!(
        role.axis_resources(None, 256, 256, BYTES).unwrap().rows(),
        OrganizationDetailRows::AxisResources(_)
    ));
    let all = entries(&owner, 256);
    let occupant = all
        .iter()
        .find_map(|entry| match entry.row() {
            OrganizationRow::OccupantBinding { occupant, .. } => Some(occupant.clone()),
            _ => None,
        })
        .unwrap();
    let (binding, _, _) = query(&owner, &OrganizationNode::Occupant(occupant), false, 1);
    assert!(matches!(
        binding[0]
            .handle()
            .sources(None, 1, 1, BYTES)
            .unwrap()
            .mapping(),
        OrganizationSourceMapping::UnmappedRecord
    ));
    drop(owner);
    assert!(matches!(
        role.sources(None, 256, 256, BYTES).unwrap().mapping(),
        OrganizationSourceMapping::Named { .. }
    ));
}
