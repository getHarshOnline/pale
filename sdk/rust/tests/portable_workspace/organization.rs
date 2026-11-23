//! Actual owned organization façade behavior over localized compiler source bytes.

use pale_sdk::*;
#[path = "../support/organization_inputs.rs"]
mod inputs;

const BYTES: u32 = MAX_ORGANIZATION_COPY_BYTES;

#[path = "organization_node_queries.rs"]
mod node_queries;

#[path = "organization_correspondence.rs"]
mod correspondence;

fn section_parity<T: std::fmt::Debug + PartialEq>(
    mut query: impl FnMut(
        Option<&OrganizationDetailCursor>,
        u16,
    ) -> Result<OrganizationDetailPage, ApiError>,
    values: impl Fn(&OrganizationDetailRows) -> Vec<T>,
    seen: &mut std::collections::HashSet<std::mem::Discriminant<OrganizationDetailRows>>,
) {
    let whole = query(None, 256).unwrap();
    seen.insert(std::mem::discriminant(whole.rows()));
    let expected = values(whole.rows());
    assert_eq!(usize::from(whole.inspected()), expected.len());
    let mut actual = Vec::new();
    let mut after = None;
    for _ in 0..10_000 {
        let page = query(after.as_ref(), 1).unwrap();
        assert_eq!(
            std::mem::discriminant(page.rows()),
            std::mem::discriminant(whole.rows())
        );
        let current = values(page.rows());
        assert_eq!(usize::from(page.inspected()), current.len());
        assert!(current.len() <= 1);
        actual.extend(current);
        after = page.continuation().cloned();
        if after.is_none() {
            assert_eq!(actual, expected);
            return;
        }
    }
    panic!("actual section did not terminate");
}

#[test]
fn all_twenty_five_closed_sections_preserve_exact_values_across_tiny_pages() {
    let owner = compile(inputs::documents());
    let rows = entries(&owner, 1);
    let mut seen = std::collections::HashSet::new();
    for row in &rows {
        let record = row.handle();
        match row.row() {
            OrganizationRow::Scope { .. }
            | OrganizationRow::Organization { .. }
            | OrganizationRow::Group { .. }
            | OrganizationRow::DisciplineTree { .. }
            | OrganizationRow::Role { .. }
            | OrganizationRow::Object { .. } => {
                section_parity(
                    |a, limit| record.axis_resources(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::AxisResources(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
            }
            OrganizationRow::Leadership { .. } => {
                section_parity(
                    |a, limit| record.leadership_participants(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::LeadershipParticipants(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.leadership_scopes(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::LeadershipScopes(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
            }
            OrganizationRow::CommunicationActor { .. } => {
                section_parity(
                    |a, limit| record.communication_resolutions(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::CommunicationResolutions(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.iter().map(|v| v.local_node.clone()).collect()
                    },
                    &mut seen,
                );
                let page = record
                    .communication_resolutions(None, 256, 256, BYTES)
                    .unwrap();
                let OrganizationDetailRows::CommunicationResolutions(values) = page.rows() else {
                    panic!("wrong section");
                };
                assert!(!values.is_empty(), "nested coverage must not be vacuous");
                for value in values {
                    section_parity(
                        |a, limit| value.handle().authorized_closure(a, limit, limit, BYTES),
                        |rows| {
                            let OrganizationDetailRows::CommunicationClosure(values) = rows else {
                                panic!("wrong section variant");
                            };
                            values.clone()
                        },
                        &mut seen,
                    );
                }
            }
            OrganizationRow::GroupLens { .. } => {
                section_parity(
                    |a, limit| record.lens_standards(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::LensStandards(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.lens_conditions(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::LensConditions(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.lens_rights(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::LensRights(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
            }
            OrganizationRow::Service { .. } => {
                section_parity(
                    |a, limit| record.service_assignments(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::ServiceAssignments(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values
                            .iter()
                            .map(|v| (v.account.clone(), v.mandate.clone(), v.state, v.interval))
                            .collect()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.service_rights(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::ServiceRights(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.service_controllers(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::ServiceControllers(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.service_authorities(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::ServiceAuthorities(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
                let page = record.service_assignments(None, 256, 256, BYTES).unwrap();
                let OrganizationDetailRows::ServiceAssignments(values) = page.rows() else {
                    panic!("wrong section");
                };
                assert!(!values.is_empty(), "nested coverage must not be vacuous");
                for value in values {
                    section_parity(
                        |a, limit| value.handle().stewards(a, limit, limit, BYTES),
                        |rows| {
                            let OrganizationDetailRows::AssignmentStewards(values) = rows else {
                                panic!("wrong section variant");
                            };
                            values.clone()
                        },
                        &mut seen,
                    );
                    section_parity(
                        |a, limit| value.handle().pods(a, limit, limit, BYTES),
                        |rows| {
                            let OrganizationDetailRows::AssignmentPods(values) = rows else {
                                panic!("wrong section variant");
                            };
                            values.clone()
                        },
                        &mut seen,
                    );
                }
            }
            OrganizationRow::NativeProjection { .. } => {
                section_parity(
                    |a, limit| record.native_references(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::NativeReferences(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
            }
            OrganizationRow::Inspection { .. } => {
                section_parity(
                    |a, limit| record.inspection_coordinates(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::InspectionCoordinates(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.inspection_states(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::InspectionStates(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
            }
            OrganizationRow::Responsibility { .. } => {
                section_parity(
                    |a, limit| record.responsibility_selections(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::ResponsibilitySelections(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.responsibility_residences(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::ResponsibilityResidences(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.clone()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.responsibility_projections(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::ResponsibilityProjections(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values
                            .iter()
                            .map(|v| {
                                (
                                    v.projection_id.clone(),
                                    v.class,
                                    v.native_owner.clone(),
                                    v.state,
                                )
                            })
                            .collect()
                    },
                    &mut seen,
                );
                section_parity(
                    |a, limit| record.responsibility_genealogy(a, limit, limit, BYTES),
                    |rows| {
                        let OrganizationDetailRows::ResponsibilityGenealogy(values) = rows else {
                            panic!("wrong section variant");
                        };
                        values.iter().map(|v| v.link_id.clone()).collect()
                    },
                    &mut seen,
                );
                let page = record
                    .responsibility_projections(None, 256, 256, BYTES)
                    .unwrap();
                let OrganizationDetailRows::ResponsibilityProjections(values) = page.rows() else {
                    panic!("wrong section");
                };
                assert!(
                    !values.is_empty(),
                    "projection coverage must not be vacuous"
                );
                for value in values {
                    section_parity(
                        |a, limit| value.handle().native_references(a, limit, limit, BYTES),
                        |rows| {
                            let OrganizationDetailRows::ProjectionReferences(values) = rows else {
                                panic!("wrong section variant");
                            };
                            values.clone()
                        },
                        &mut seen,
                    );
                }
                let page = record
                    .responsibility_genealogy(None, 256, 256, BYTES)
                    .unwrap();
                let OrganizationDetailRows::ResponsibilityGenealogy(values) = page.rows() else {
                    panic!("wrong section");
                };
                assert!(!values.is_empty(), "genealogy coverage must not be vacuous");
                for value in values {
                    section_parity(
                        |a, limit| value.handle().responsibilities(a, limit, limit, BYTES),
                        |rows| {
                            let OrganizationDetailRows::GenealogyResponsibilities(values) = rows
                            else {
                                panic!("wrong section variant");
                            };
                            values.clone()
                        },
                        &mut seen,
                    );
                    section_parity(
                        |a, limit| value.handle().projections(a, limit, limit, BYTES),
                        |rows| {
                            let OrganizationDetailRows::GenealogyProjections(values) = rows else {
                                panic!("wrong section variant");
                            };
                            values.clone()
                        },
                        &mut seen,
                    );
                    section_parity(
                        |a, limit| value.handle().occurrences(a, limit, limit, BYTES),
                        |rows| {
                            let OrganizationDetailRows::GenealogyOccurrences(values) = rows else {
                                panic!("wrong section variant");
                            };
                            values.clone()
                        },
                        &mut seen,
                    );
                }
            }
            OrganizationRow::OccupantBinding { .. }
            | OrganizationRow::ProviderBinding { .. }
            | OrganizationRow::Relation { .. }
            | OrganizationRow::PopulationRule { .. }
            | OrganizationRow::DerivedRelation { .. } => {}
        }
    }
    assert_eq!(
        seen.len(),
        25,
        "all selected sections reached through actual admitted parents"
    );
}

fn compile(documents: Vec<(String, Vec<u8>)>) -> PortableWorkspaceCompilation {
    PaleApi::new()
        .unwrap()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            documents
                .into_iter()
                .map(|(label, bytes)| PortableWorkspaceDocument::new(label, bytes))
                .collect(),
        ))
        .expect("actual public SDK compiler must admit the localized fixture")
}

#[test]
fn explicit_relation_locality_survives_paging_without_claiming_native_existence() {
    let mut documents = inputs::documents();
    let (_, bytes) = documents
        .iter_mut()
        .find(|(label, _)| label == "organization.toml")
        .unwrap();
    let mut source: toml::Value = toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap();
    let mut native = source["relations"][0].clone();
    native["relation_id"] = "relation.unobserved".into();
    native["class"] = "native_presence".into();
    native["state"] = "unknown".into();
    native["to"] = toml::from_str::<toml::Value>(
        "kind = \"native_resource\"\nresource = \"external.unobserved\"",
    )
    .unwrap();
    source["relations"].as_array_mut().unwrap().push(native);
    *bytes = toml::to_string(&source).unwrap().into_bytes();
    let owner = compile(documents);
    let tiny = entries(&owner, 1);
    let whole = entries(&owner, 256);
    assert_eq!(
        tiny.iter().map(OrganizationEntry::row).collect::<Vec<_>>(),
        whole.iter().map(OrganizationEntry::row).collect::<Vec<_>>()
    );
    let mut seen = [false; 2];
    for entry in &tiny {
        let OrganizationRow::Relation {
            relation_id,
            locality,
            to,
            state,
            ..
        } = entry.row()
        else {
            continue;
        };
        assert!(entry.handle().belongs_to(&owner));
        if relation_id == "relation.unobserved" {
            assert_eq!(*locality, OrganizationRelationLocality::NativeBoundary);
            assert_eq!(*state, OrganizationFactState::Unknown);
            assert_eq!(
                *to,
                OrganizationNode::NativeResource("external.unobserved".into())
            );
            seen[1] = true;
        } else {
            assert_eq!(*locality, OrganizationRelationLocality::Internal);
            seen[0] = true;
        }
    }
    assert_eq!(seen, [true, true]);
}

fn entries(owner: &PortableWorkspaceCompilation, limit: u16) -> Vec<OrganizationEntry> {
    let mut result = Vec::new();
    let mut after = None;
    for _ in 0..10_000 {
        let page = owner
            .inspect_organization(after.as_ref(), limit, limit, BYTES)
            .unwrap();
        assert!(page.belongs_to(owner));
        assert!(page.inspected() <= limit);
        assert_eq!(usize::from(page.inspected()), page.entries().len());
        result.extend_from_slice(page.entries());
        after = page.continuation().cloned();
        if after.is_none() {
            return result;
        }
    }
    panic!("fixture did not terminate");
}

fn collect<T: Clone>(
    mut query: impl FnMut(Option<&OrganizationDetailCursor>) -> Result<OrganizationDetailPage, ApiError>,
    values: impl Fn(&OrganizationDetailRows) -> Vec<T>,
) -> Vec<T> {
    let mut result = Vec::new();
    let mut after = None;
    for _ in 0..10_000 {
        let page = query(after.as_ref()).unwrap();
        let current = values(page.rows());
        assert_eq!(usize::from(page.inspected()), current.len());
        assert!(current.len() <= 1);
        result.extend(current);
        after = page.continuation().cloned();
        if after.is_none() {
            return result;
        }
    }
    panic!("fixture detail did not terminate");
}

#[test]
fn all_eighteen_owned_families_preserve_tiny_page_order_and_independent_axes() {
    let owner = compile(inputs::documents());
    let tiny = entries(&owner, 1);
    let whole = entries(&owner, 256);
    assert_eq!(
        tiny.iter().map(OrganizationEntry::row).collect::<Vec<_>>(),
        whole.iter().map(OrganizationEntry::row).collect::<Vec<_>>()
    );
    let classes: std::collections::HashSet<_> = tiny
        .iter()
        .map(|entry| std::mem::discriminant(entry.row()))
        .collect();
    assert_eq!(
        classes.len(),
        18,
        "all18 must be admitted and projected, not just declared in an enum"
    );
    let role = tiny
        .iter()
        .find_map(|entry| match entry.row() {
            OrganizationRow::Role {
                role,
                organization,
                group: Some(group),
                discipline_tree: Some(tree),
                axes,
                ..
            } => Some((role, organization, group, tree, axes)),
            _ => None,
        })
        .unwrap();
    assert_eq!(role.0.namespace, role.1.namespace);
    assert_eq!(role.2.namespace, role.3.namespace);
    assert_ne!(role.2.group_id, role.3.tree_id);
    assert_ne!(role.4.owner, role.4.controller);
    assert_ne!(role.4.controller, role.4.authority);
    assert_eq!(role.4.disclosure, OrganizationDisclosure::Protected);
    let mut service_count = 0;
    for entry in &tiny {
        if let OrganizationRow::Service {
            contract,
            provider_completion,
            provider_acceptance,
            consumer_acceptance,
            native_read_back,
            pale_settlement,
            ..
        } = entry.row()
        {
            service_count += 1;
            assert_ne!(contract.provider_scope, contract.consumer_scope);
            let refs = [
                &provider_completion.evidence_ref,
                &provider_acceptance.evidence_ref,
                &consumer_acceptance.evidence_ref,
                &native_read_back.evidence_ref,
                &pale_settlement.evidence_ref,
            ];
            assert_eq!(
                refs.into_iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len(),
                5
            );
        }
    }
    assert!(service_count >= 2);
    assert!(tiny.iter().any(|entry| matches!(
        entry.row(),
        OrganizationRow::DisciplineTree {
            state: OrganizationFactState::Absent,
            ..
        }
    )));
    let derived: Vec<_> = tiny
        .iter()
        .filter_map(|entry| match entry.row() {
            OrganizationRow::DerivedRelation {
                source: OrganizationDerivedSource::LeadershipParticipation(topic),
                class,
                from,
                to,
                interval,
                ..
            } => Some((topic, class, from, to, interval)),
            _ => None,
        })
        .collect();
    assert_eq!(derived.len(), 4);
    assert!(derived
        .iter()
        .all(|edge| *edge.1 == OrganizationDerivedClass::LeadershipParticipation));
    assert!(derived
        .windows(2)
        .any(|pair| pair[0].0 == pair[1].0 && (pair[0].2 != pair[1].2 || pair[0].3 != pair[1].3)));
}

#[test]
fn retained_records_and_nested_parents_survive_all_original_page_and_compilation_drops() {
    let owner = compile(inputs::documents());
    let rows = entries(&owner, 1);
    let service = rows
        .iter()
        .find(|entry| matches!(entry.row(), OrganizationRow::Service { .. }))
        .unwrap()
        .handle()
        .clone();
    let assignments = collect(
        |after| service.service_assignments(after, 1, 1, BYTES),
        |values| {
            let OrganizationDetailRows::ServiceAssignments(values) = values else {
                panic!("wrong section");
            };
            values.clone()
        },
    );
    assert!(assignments.len() >= 2);
    let assignment = assignments[0].handle().clone();
    let initial = assignment.stewards(None, 1, 1, BYTES).unwrap();
    let after = initial.continuation().cloned().unwrap();
    drop(assignments);
    drop(initial);
    drop(rows);
    drop(owner);
    let remaining = assignment.stewards(Some(&after), 1, 1, BYTES).unwrap();
    assert!(matches!(
        remaining.rows(),
        OrganizationDetailRows::AssignmentStewards(_)
    ));
    let OrganizationDetailParent::Assignment(parent) = remaining.parent() else {
        panic!("nested parent custody lost");
    };
    assert!(matches!(
        parent.pods(None, 1, 1, BYTES).unwrap().rows(),
        OrganizationDetailRows::AssignmentPods(_)
    ));
    assert!(matches!(
        service.sources(None, 1, 1, BYTES).unwrap().mapping(),
        OrganizationSourceMapping::Named { .. }
    ));
}

#[test]
fn complete_owner_raw_limits_section_and_nested_member_mismatches_are_rejected() {
    let documents = inputs::documents();
    let owner = compile(documents.clone());
    let equal = compile(documents);
    assert_eq!(owner.semantic_ir_digest(), equal.semantic_ir_digest());
    let page = owner.inspect_organization(None, 1, 1, BYTES).unwrap();
    let after = page.continuation().unwrap();
    assert!(equal
        .inspect_organization(Some(after), 1, 1, BYTES)
        .is_err());
    assert!(owner
        .inspect_organization(Some(after), 1, 1, BYTES - 1)
        .is_err());
    assert!(owner
        .inspect_organization(Some(after), 0, 1, BYTES)
        .is_err());
    assert!(owner
        .inspect_organization(Some(after), 1, 257, BYTES)
        .is_err());
    assert!(owner.inspect_organization(Some(after), 1, 1, BYTES).is_ok());
    let rows = entries(&owner, 1);
    let service = rows
        .iter()
        .find(|entry| matches!(entry.row(), OrganizationRow::Service { .. }))
        .unwrap()
        .handle();
    assert!(!service.belongs_to(&equal));
    let assignments = service.service_assignments(None, 256, 256, BYTES).unwrap();
    let OrganizationDetailRows::ServiceAssignments(values) = assignments.rows() else {
        panic!("wrong section");
    };
    let first = values[0].handle();
    let second = values[1].handle();
    let page = first.stewards(None, 1, 1, BYTES).unwrap();
    let after = page.continuation().unwrap();
    assert!(second.stewards(Some(after), 1, 1, BYTES).is_err());
    assert!(first.pods(Some(after), 1, 1, BYTES).is_err());
    assert!(first.stewards(Some(after), 1, 1, BYTES - 1).is_err());
    assert!(first.stewards(Some(after), 1, 1, BYTES).is_ok());
    assert!(service.leadership_participants(None, 1, 1, BYTES).is_err());
}

#[test]
fn exact_copy_budget_boundaries_publish_no_partial_result_or_advanced_cursor() {
    let owner = compile(inputs::documents());
    let first = owner.inspect_organization(None, 1, 1, BYTES).unwrap();
    let exact = first.copied_string_bytes();
    assert!(exact > 1);
    assert!(owner.inspect_organization(None, 1, 1, exact - 1).is_err());
    let at = owner.inspect_organization(None, 1, 1, exact).unwrap();
    let above = owner.inspect_organization(None, 1, 1, exact + 1).unwrap();
    assert_eq!(at.entries()[0].row(), above.entries()[0].row());
    assert_eq!(at.copied_string_bytes(), exact);
    assert_eq!(above.copied_string_bytes(), exact);
    // A failed wrong-budget continuation cannot consume the actual token.
    let cursor = first.continuation().unwrap();
    let baseline = owner
        .inspect_organization(Some(cursor), 1, 1, BYTES)
        .unwrap();
    assert!(owner.inspect_organization(Some(cursor), 1, 1, 1).is_err());
    let retry = owner
        .inspect_organization(Some(cursor), 1, 1, BYTES)
        .unwrap();
    assert_eq!(baseline.entries()[0].row(), retry.entries()[0].row());
}

#[test]
fn unmapped_record_and_named_utf8_source_payload_have_distinct_exact_budgeted_meanings() {
    let mut documents = inputs::documents();
    let renamed = "organisation-é.toml";
    for (label, bytes) in &mut documents {
        if label == "organization.toml" {
            *label = renamed.into();
        }
        if label == "pale-workspace.toml" {
            let mut value: toml::Value =
                toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap();
            value["workspace"]["organizational_translations"] =
                toml::Value::Array(vec![toml::Value::String(renamed.into())]);
            *bytes = toml::to_string(&value).unwrap().into_bytes();
        }
    }
    let owner = compile(documents);
    let rows = entries(&owner, 1);
    let role = rows
        .iter()
        .find(|entry| matches!(entry.row(), OrganizationRow::Role { .. }))
        .unwrap()
        .handle();
    let binding = rows
        .iter()
        .find(|entry| matches!(entry.row(), OrganizationRow::OccupantBinding { .. }))
        .unwrap()
        .handle();
    assert!(matches!(
        binding.sources(None, 1, 1, BYTES).unwrap().mapping(),
        OrganizationSourceMapping::UnmappedRecord
    ));
    let source = role.sources(None, 256, 256, BYTES).unwrap();
    let OrganizationSourceMapping::Named { occurrences, .. } = source.mapping() else {
        panic!("role must have named occurrences");
    };
    assert!(!occurrences.is_empty());
    let expected: usize = occurrences
        .iter()
        .map(|occurrence| {
            let location = occurrence.location();
            location.source_label().len()
                + location
                    .path()
                    .iter()
                    .map(|step| match step {
                        AuthoringInstancePathSegment::Field(value) => value.len(),
                        AuthoringInstancePathSegment::Index(_) => 0,
                    })
                    .sum::<usize>()
        })
        .sum();
    assert!(occurrences
        .iter()
        .any(|occurrence| occurrence.location().source_label() == renamed));
    assert!(renamed.len() > renamed.chars().count());
    let exact = u32::try_from(expected).unwrap();
    assert_eq!(source.copied_string_bytes(), exact);
    assert!(role.sources(None, 256, 256, exact - 1).is_err());
    assert_eq!(
        role.sources(None, 256, 256, exact)
            .unwrap()
            .copied_string_bytes(),
        exact
    );
    assert_eq!(
        role.sources(None, 256, 256, exact + 1)
            .unwrap()
            .copied_string_bytes(),
        exact
    );
}

#[test]
fn genuine_copy_exhaustion_preserves_the_original_continuation_and_failure() {
    let mut documents = inputs::documents();
    let (_, bytes) = documents
        .iter_mut()
        .find(|(label, _)| label == "organization.toml")
        .unwrap();
    let mut source: toml::Value = toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap();
    assert!(source["scopes"].as_array().unwrap().len() >= 2);
    // Controller is independently declared. Preserve the scope owner because
    // actual child/container admission requires their owners to agree.
    source["scopes"][1]["axes"]["controller"] = toml::Value::String("x".repeat(255));
    *bytes = toml::to_string(&source).unwrap().into_bytes();
    let owner = compile(documents);
    let first = owner.inspect_organization(None, 1, 1, BYTES).unwrap();
    let second = owner
        .inspect_organization(first.continuation(), 1, 1, BYTES)
        .unwrap();
    assert!(matches!(
        first.entries()[0].row(),
        OrganizationRow::Scope { .. }
    ));
    assert!(matches!(
        second.entries()[0].row(),
        OrganizationRow::Scope { .. }
    ));
    let budget = first.copied_string_bytes();
    assert!(second.copied_string_bytes() > budget);
    let first = owner.inspect_organization(None, 1, 1, budget).unwrap();
    let cursor = first.continuation().unwrap();
    let retained = cursor.clone();
    for original in [cursor, &retained, cursor] {
        let failure = owner
            .inspect_organization(Some(original), 1, 1, budget)
            .unwrap_err();
        assert_eq!(
            failure.diagnostic().code(),
            "workspace_organization_copy_budget"
        );
    }
    // Raising a bound is a different query, not permission to reinterpret this
    // cursor. The exact original remains reusable but deterministically fails
    // on the same oversized record; an explicit new query can copy that record.
    assert_eq!(
        owner
            .inspect_organization(Some(cursor), 1, 1, BYTES)
            .unwrap_err()
            .diagnostic()
            .code(),
        "workspace_organization_custody_mismatch"
    );
    assert_eq!(
        second.copied_string_bytes(),
        owner
            .inspect_organization(
                owner
                    .inspect_organization(None, 1, 1, BYTES)
                    .unwrap()
                    .continuation(),
                1,
                1,
                BYTES,
            )
            .unwrap()
            .copied_string_bytes()
    );
}

#[test]
fn large_admitted_responsibility_headers_obey_the_copy_ceiling_and_exact_boundaries() {
    let mut documents = inputs::documents();
    let responsibility_index = documents
        .iter()
        .position(|(label, _)| label == "responsibility.toml")
        .unwrap();
    let organization_index = documents
        .iter()
        .position(|(label, _)| label == "organization.toml")
        .unwrap();
    let mut responsibility: toml::Value =
        toml::from_str(std::str::from_utf8(&documents[responsibility_index].1).unwrap()).unwrap();
    let mut organization: toml::Value =
        toml::from_str(std::str::from_utf8(&documents[organization_index].1).unwrap()).unwrap();
    let long = "z".repeat(255);
    for field in ["source_id", "owner", "evidence_ref"] {
        responsibility["provenance"][field] = toml::Value::String(long.clone());
        responsibility["placement"]["attribution"][field] = toml::Value::String(long.clone());
    }
    let namespace = responsibility["placement"]["value"]["service_contract"]["namespace"]
        .as_str()
        .unwrap()
        .to_owned();
    let contract_id = responsibility["placement"]["value"]["service_contract"]["contract_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let service = organization["services"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|service| {
            service["namespace"].as_str() == Some(&namespace)
                && service["contract_id"].as_str() == Some(&contract_id)
        })
        .unwrap();
    for (answer, service_field) in [
        ("provider_completion", "provider_completion"),
        ("provider_acceptance", "provider_acceptance"),
        ("consumer_acceptance", "consumer_acceptance"),
        ("native_result", "native_read_back"),
        ("pale_settlement", "pale_settlement"),
    ] {
        for field in ["source_id", "owner", "evidence_ref"] {
            responsibility[answer]["attribution"][field] = toml::Value::String(long.clone());
        }
        for field in ["owner", "evidence_ref"] {
            responsibility[answer]["value"]["requirement"][field] =
                toml::Value::String(long.clone());
            // Preserve the actual organization's exact independent requirement
            // join; admission must still verify it through the public compiler.
            service[service_field]["requirement"][field] = toml::Value::String(long.clone());
        }
    }
    documents[responsibility_index].1 = toml::to_string(&responsibility).unwrap().into_bytes();
    documents[organization_index].1 = toml::to_string(&organization).unwrap().into_bytes();
    let owner = compile(documents);
    let all = entries(&owner, 256);
    let target = all
        .iter()
        .find(|entry| matches!(entry.row(), OrganizationRow::Responsibility { .. }))
        .unwrap()
        .row();
    let mut after = None;
    let exact = loop {
        let page = owner
            .inspect_organization(after.as_ref(), 1, 1, BYTES)
            .unwrap();
        if page.entries().iter().any(|entry| entry.row() == target) {
            break page.copied_string_bytes();
        }
        after = Some(
            page.continuation()
                .expect("responsibility must be reached")
                .clone(),
        );
    };
    assert!(
        exact >= 31 * 255,
        "actual admitted maximum-length header components"
    );
    assert!(
        exact <= 38 * 255,
        "audited largest shallow header upper bound"
    );
    assert!(exact < BYTES);
    // Each new budget is a fresh traversal, preserving the raw-query binding.
    for budget in [exact - 1, exact, exact + 1] {
        let mut after = None;
        loop {
            match owner.inspect_organization(after.as_ref(), 1, 1, budget) {
                Err(error) => {
                    assert_eq!(budget, exact - 1);
                    assert_eq!(
                        error.diagnostic().code(),
                        "workspace_organization_copy_budget"
                    );
                    break;
                }
                Ok(page) if page.entries().iter().any(|entry| entry.row() == target) => {
                    assert!(budget >= exact);
                    assert_eq!(page.copied_string_bytes(), exact);
                    break;
                }
                Ok(page) => {
                    after = Some(
                        page.continuation()
                            .expect("must reach target or its budget error")
                            .clone(),
                    )
                }
            }
        }
    }
}

#[test]
fn equal_assignment_bodies_under_distinct_services_do_not_substitute_for_member_custody() {
    let mut documents = inputs::documents();
    let (_, bytes) = documents
        .iter_mut()
        .find(|(label, _)| label == "organization.toml")
        .unwrap();
    let mut source: toml::Value = toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap();
    let assignments = source["services"][0]["assignments"].clone();
    source["services"][1]["assignments"] = assignments;
    *bytes = toml::to_string(&source).unwrap().into_bytes();
    let owner = compile(documents);
    let rows = entries(&owner, 1);
    let services: Vec<_> = rows
        .iter()
        .filter(|entry| matches!(entry.row(), OrganizationRow::Service { .. }))
        .collect();
    assert_eq!(services.len(), 2);
    let first = services[0]
        .handle()
        .service_assignments(None, 1, 1, BYTES)
        .unwrap();
    let second = services[1]
        .handle()
        .service_assignments(None, 1, 1, BYTES)
        .unwrap();
    let OrganizationDetailRows::ServiceAssignments(first) = first.rows() else {
        panic!("wrong section");
    };
    let OrganizationDetailRows::ServiceAssignments(second) = second.rows() else {
        panic!("wrong section");
    };
    let (first, second) = (&first[0], &second[0]);
    assert_eq!(
        (&first.account, &first.mandate, first.state, first.interval),
        (
            &second.account,
            &second.mandate,
            second.state,
            second.interval
        )
    );
    let first_children = collect(
        |after| first.handle().stewards(after, 1, 1, BYTES),
        |rows| {
            let OrganizationDetailRows::AssignmentStewards(values) = rows else {
                panic!("wrong section");
            };
            values.clone()
        },
    );
    let second_children = collect(
        |after| second.handle().stewards(after, 1, 1, BYTES),
        |rows| {
            let OrganizationDetailRows::AssignmentStewards(values) = rows else {
                panic!("wrong section");
            };
            values.clone()
        },
    );
    assert_eq!(first_children, second_children);
    assert!(!first_children.is_empty());
    let page = first.handle().stewards(None, 1, 1, BYTES).unwrap();
    let cursor = page.continuation().unwrap();
    assert_eq!(
        second
            .handle()
            .stewards(Some(cursor), 1, 1, BYTES)
            .unwrap_err()
            .diagnostic()
            .code(),
        "workspace_organization_custody_mismatch"
    );
    assert!(first.handle().stewards(Some(cursor), 1, 1, BYTES).is_ok());
}
