//! Actual compiled structural collection parity and nested continuation custody.

use super::super::tests::{compile, detail_responsibility, rows};
use super::*;
use crate::organization_fixtures as org;

fn collect<'a, T: Clone + 'a>(
    mut query: impl FnMut(
        Option<&OrganizationMemberCursor>,
        u16,
        u16,
    ) -> Result<OrganizationMemberPage<'a, T>, OrganizationInspectionError>,
    limit: u16,
) -> Vec<T> {
    let mut after = None;
    let mut result = Vec::new();
    for _ in 0..10_000 {
        let page = query(after.as_ref(), limit, limit).unwrap();
        assert_eq!(usize::from(page.inspected()), page.entries().len());
        assert!(page.inspected() <= limit);
        result.extend(page.entries().map(|entry| entry.value().clone()));
        after = page.continuation().cloned();
        if after.is_none() {
            return result;
        }
    }
    panic!("finite fixture section did not terminate");
}

#[test]
fn every_structural_section_matches_the_existing_compiled_collection() {
    let source = org::organizational_translation();
    let responsibility = detail_responsibility();
    let owner = compile(Some(&source), Some(&responsibility));
    for entry in rows(&owner, 1) {
        for limit in [1, 256] {
            match entry.record() {
                OrganizationInspectionRecord::Scope(v) => assert_eq!(
                    collect(|a, i, r| entry.axis_resources(a, i, r), limit),
                    v.axes.resources
                ),
                OrganizationInspectionRecord::Organization(v) => assert_eq!(
                    collect(|a, i, r| entry.axis_resources(a, i, r), limit),
                    v.axes.resources
                ),
                OrganizationInspectionRecord::Group(v) => assert_eq!(
                    collect(|a, i, r| entry.axis_resources(a, i, r), limit),
                    v.axes.resources
                ),
                OrganizationInspectionRecord::DisciplineTree(v) => assert_eq!(
                    collect(|a, i, r| entry.axis_resources(a, i, r), limit),
                    v.axes.resources
                ),
                OrganizationInspectionRecord::Role(v) => assert_eq!(
                    collect(|a, i, r| entry.axis_resources(a, i, r), limit),
                    v.axes.resources
                ),
                OrganizationInspectionRecord::Object(v) => assert_eq!(
                    collect(|a, i, r| entry.axis_resources(a, i, r), limit),
                    v.axes.resources
                ),
                OrganizationInspectionRecord::Leadership(v) => {
                    assert_eq!(
                        collect(|a, i, r| entry.leadership_participants(a, i, r), limit),
                        v.participants
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.leadership_scopes(a, i, r), limit),
                        v.affected_scopes
                    );
                }
                OrganizationInspectionRecord::CommunicationActor(v) => {
                    assert_eq!(
                        collect(|a, i, r| entry.communication_resolutions(a, i, r), limit),
                        v.local_resolutions
                    );
                    let page = entry.communication_resolutions(None, 256, 256).unwrap();
                    for member in page.entries() {
                        let handle = member.retain();
                        assert_eq!(
                            collect(|a, i, r| handle.authorized_closure(&owner, a, i, r), limit),
                            member.value().authorized_closure
                        );
                    }
                }
                OrganizationInspectionRecord::GroupLens(v) => {
                    assert_eq!(
                        collect(|a, i, r| entry.lens_standards(a, i, r), limit),
                        v.standards
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.lens_conditions(a, i, r), limit),
                        v.block_conditions
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.lens_rights(a, i, r), limit),
                        v.rights
                    );
                }
                OrganizationInspectionRecord::Service(v) => {
                    assert_eq!(
                        collect(|a, i, r| entry.service_assignments(a, i, r), limit),
                        v.assignments
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.service_rights(a, i, r), limit),
                        v.ceiling.rights.permitted
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.service_controllers(a, i, r), limit),
                        v.ceiling.controllers.permitted
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.service_authorities(a, i, r), limit),
                        v.ceiling.authorities.permitted
                    );
                    let page = entry.service_assignments(None, 256, 256).unwrap();
                    for member in page.entries() {
                        let handle = member.retain();
                        assert_eq!(
                            collect(|a, i, r| handle.stewards(&owner, a, i, r), limit),
                            member.value().stewards
                        );
                        assert_eq!(
                            collect(|a, i, r| handle.pods(&owner, a, i, r), limit),
                            member.value().pods
                        );
                    }
                }
                OrganizationInspectionRecord::NativeProjection(v) => assert_eq!(
                    collect(|a, i, r| entry.native_references(a, i, r), limit),
                    v.native_refs
                ),
                OrganizationInspectionRecord::Inspection(v) => {
                    assert_eq!(
                        collect(|a, i, r| entry.inspection_coordinates(a, i, r), limit),
                        v.coordinates
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.inspection_states(a, i, r), limit),
                        v.states
                    );
                }
                OrganizationInspectionRecord::Responsibility(v) => {
                    assert_eq!(
                        collect(|a, i, r| entry.responsibility_selections(a, i, r), limit),
                        v.structural_objects
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.responsibility_residences(a, i, r), limit),
                        v.residence
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.responsibility_projections(a, i, r), limit),
                        v.native_projections
                    );
                    assert_eq!(
                        collect(|a, i, r| entry.responsibility_genealogy(a, i, r), limit),
                        v.genealogy_links
                    );
                    let page = entry.responsibility_projections(None, 256, 256).unwrap();
                    for member in page.entries() {
                        let handle = member.retain();
                        assert_eq!(
                            collect(|a, i, r| handle.native_references(&owner, a, i, r), limit),
                            member.value().native_refs
                        );
                    }
                    let page = entry.responsibility_genealogy(None, 256, 256).unwrap();
                    for member in page.entries() {
                        let handle = member.retain();
                        assert_eq!(
                            collect(|a, i, r| handle.responsibilities(&owner, a, i, r), limit),
                            member.value().responsibilities
                        );
                        assert_eq!(
                            collect(|a, i, r| handle.projections(&owner, a, i, r), limit),
                            member.value().projections
                        );
                        assert_eq!(
                            collect(|a, i, r| handle.occurrences(&owner, a, i, r), limit),
                            member.value().occurrences
                        );
                    }
                }
                OrganizationInspectionRecord::OccupantBinding(_)
                | OrganizationInspectionRecord::ProviderBinding(_)
                | OrganizationInspectionRecord::PopulationRule(_)
                | OrganizationInspectionRecord::Relation(_)
                | OrganizationInspectionRecord::DerivedRelation(_) => {
                    assert!(matches!(
                        entry.axis_resources(None, limit, limit),
                        Err(OrganizationInspectionError::SectionUnavailable)
                    ));
                }
            }
        }
    }
}

#[test]
fn identical_assignment_values_under_different_services_cannot_exchange_continuations() {
    let mut source = org::organizational_translation();
    assert!(source.services.len() >= 2);
    source.services[1].assignments = source.services[0].assignments.clone();
    let owner = compile(Some(&source), None);
    let services: Vec<_> = rows(&owner, 1)
        .into_iter()
        .filter(|entry| matches!(entry.record(), OrganizationInspectionRecord::Service(_)))
        .collect();
    let first = services[0].service_assignments(None, 1, 1).unwrap();
    let second = services[1].service_assignments(None, 1, 1).unwrap();
    let first_entry = first.entries().next().unwrap();
    let second_entry = second.entries().next().unwrap();
    assert_eq!(
        first_entry.value(),
        second_entry.value(),
        "equal bodies are the hostile custody control"
    );
    let first = first_entry.retain();
    let second = second_entry.retain();
    let page = first.stewards(&owner, None, 1, 1).unwrap();
    let cursor = page.continuation().unwrap();
    assert!(matches!(
        second.stewards(&owner, Some(cursor), 1, 1),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    assert!(matches!(
        first.pods(&owner, Some(cursor), 1, 1),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    assert!(matches!(
        first.stewards(&owner, Some(cursor), 2, 1),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    assert!(first.stewards(&owner, Some(cursor), 1, 1).is_ok());
}

#[test]
fn equal_compilation_and_wrong_member_are_rejected_before_child_access() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    let equal = compile(Some(&source), None);
    assert_eq!(owner.semantic_ir_digest(), equal.semantic_ir_digest());
    let service = rows(&owner, 1)
        .into_iter()
        .find(|entry| matches!(entry.record(), OrganizationInspectionRecord::Service(_)))
        .unwrap();
    let page = service.service_assignments(None, 256, 256).unwrap();
    let mut members = page.entries();
    let first = members.next().unwrap().retain();
    let second = members.next().unwrap().retain();
    let page = first.stewards(&owner, None, 1, 1).unwrap();
    let after = page.continuation().unwrap();
    assert!(matches!(
        first.stewards(&equal, Some(after), 1, 1),
        Err(OrganizationInspectionError::RecordOwnerMismatch)
    ));
    assert!(matches!(
        second.stewards(&owner, Some(after), 1, 1),
        Err(OrganizationInspectionError::ContinuationMismatch)
    ));
    assert!(matches!(
        first.stewards(&owner, Some(after), 0, 1),
        Err(OrganizationInspectionError::InvalidLimit)
    ));
}

#[test]
fn terminal_empty_and_inapplicable_sections_have_distinct_results() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    let communication = rows(&owner, 1)
        .into_iter()
        .find(|entry| {
            matches!(
                entry.record(),
                OrganizationInspectionRecord::CommunicationActor(_)
            )
        })
        .unwrap();
    let page = communication
        .communication_resolutions(None, 256, 256)
        .unwrap();
    let empty = page
        .entries()
        .find(|member| member.value().authorized_closure.is_empty())
        .unwrap()
        .retain();
    let empty_page = empty.authorized_closure(&owner, None, 1, 1).unwrap();
    assert_eq!(empty_page.inspected(), 0);
    assert_eq!(empty_page.entries().len(), 0);
    assert!(empty_page.continuation().is_none());
    assert!(matches!(
        communication.service_assignments(None, 1, 1),
        Err(OrganizationInspectionError::SectionUnavailable)
    ));
    let nonempty = page
        .entries()
        .find(|member| !member.value().authorized_closure.is_empty())
        .unwrap()
        .retain();
    let one = nonempty.authorized_closure(&owner, None, 1, 1).unwrap();
    assert_eq!(one.inspected(), 1);
    let terminal = nonempty
        .authorized_closure(&owner, one.continuation(), 1, 1)
        .unwrap();
    assert_eq!(terminal.inspected(), 0);
    assert!(terminal.continuation().is_none());
}
