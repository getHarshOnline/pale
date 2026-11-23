//! Actual admitted records, full-key comparison and independent source custody.

use super::super::tests::{compile, detail_responsibility, rows};
use super::*;
use crate::organization_fixtures as org;

fn finish<'a>(
    other: &'a PortableWorkspaceUniverse,
    selected_owner: &'a PortableWorkspaceUniverse,
    selected: &OrganizationRecordHandle,
    limit: u16,
) -> OrganizationCorrespondencePage<'a> {
    let mut after = None;
    for _ in 0..10_000 {
        let page = other
            .correspond_organization_record(selected_owner, selected, after.as_ref(), limit)
            .unwrap();
        assert!(page.inspected() <= limit);
        match page.state() {
            OrganizationCorrespondenceState::Searching(next) => after = Some(next.clone()),
            _ => return page,
        }
    }
    panic!("finite admitted family did not terminate");
}

#[test]
fn all_eighteen_families_retain_exact_equal_records_in_both_directions() {
    let source = org::organizational_translation();
    let responsibility = detail_responsibility();
    let left = compile(Some(&source), Some(&responsibility));
    let right = compile(Some(&source), Some(&responsibility));
    let mut families = std::collections::HashSet::new();
    for (selected_owner, other_owner) in [(&left, &right), (&right, &left)] {
        for selected in rows(selected_owner, 256) {
            families.insert(std::mem::discriminant(&selected.record()));
            let handle = selected.retain();
            let page = finish(other_owner, selected_owner, &handle, 1);
            let OrganizationCorrespondenceState::Matched { other, values } = page.state() else {
                panic!("key missing");
            };
            assert_eq!(*values, PortableWorkspaceValueComparison::Equal);
            assert_eq!(selected.record(), other.record());
            assert!(selected_owner
                .resolve_organization_record(&other.retain())
                .is_err());
            assert!(other_owner
                .resolve_organization_record(&other.retain())
                .is_ok());
        }
    }
    assert_eq!(families.len(), 18);
}

#[test]
fn same_record_joins_independent_pages_not_equal_values_or_other_positions() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    let reconstructed = compile(Some(&source), None);
    let first = rows(&owner, 1);
    let second = rows(&owner, 256);
    let different_owner = rows(&reconstructed, 256);
    assert_eq!(
        owner.semantic_ir_digest(),
        reconstructed.semantic_ir_digest()
    );
    assert!(first.len() > 2);
    for ((left, right), foreign) in first.iter().zip(&second).zip(&different_owner) {
        assert!(left.retain().same_record(&right.retain()));
        assert!(right.retain().same_record(&left.retain()));
        assert_eq!(left.record(), foreign.record());
        assert!(!left.retain().same_record(&foreign.retain()));
    }
    assert!(!first[0].retain().same_record(&first[1].retain()));
    let other_family = first
        .iter()
        .find(|entry| matches!(entry.record(), OrganizationInspectionRecord::Role(_)))
        .unwrap();
    assert!(!first[0].retain().same_record(&other_family.retain()));
}

#[test]
fn reordered_equal_records_keep_fresh_original_source_positions() {
    let source = org::organizational_translation();
    let mut reordered = source.clone();
    reordered.roles.reverse();
    let left = compile(Some(&source), None);
    let right = compile(Some(&reordered), None);
    let selected = rows(&left, 256)
        .into_iter()
        .find(|r| matches!(r.record(), OrganizationInspectionRecord::Role(_)))
        .unwrap();
    let result = finish(&right, &left, &selected.retain(), 1);
    let OrganizationCorrespondenceState::Matched { other, values } = result.state() else {
        panic!("role missing");
    };
    assert_eq!(*values, PortableWorkspaceValueComparison::Equal);
    let OrganizationSources::Named { page: before, .. } = selected.sources(None, 256, 256).unwrap()
    else {
        panic!("source missing");
    };
    let OrganizationSources::Named { page: after, .. } = other.sources(None, 256, 256).unwrap()
    else {
        panic!("source missing");
    };
    let definition = |page: &crate::workspace::AuthoringOccurrencePage<'_>| {
        page.occurrences()
            .iter()
            .find(|occurrence| {
                occurrence.role() == crate::workspace::AuthoringOccurrenceRole::Definition
            })
            .expect("role definition missing")
            .path()
            .to_vec()
    };
    assert_ne!(definition(&before), definition(&after));
}

#[test]
fn full_nested_values_change_without_changing_the_record_key_or_collection_count() {
    let source = org::organizational_translation();
    let mut changed = source.clone();
    assert!(!changed.roles[0].axes.resources.is_empty());
    changed.roles[0].axes.resources[0] = "resource.changed".parse().unwrap();
    let left = compile(Some(&source), None);
    let right = compile(Some(&changed), None);
    let selected = rows(&left, 256).into_iter().find(|r| matches!(r.record(), OrganizationInspectionRecord::Role(v) if v.role == source.roles[0].role)).unwrap();
    let result = finish(&right, &left, &selected.retain(), 256);
    assert!(matches!(
        result.state(),
        OrganizationCorrespondenceState::Matched {
            values: PortableWorkspaceValueComparison::Changed,
            ..
        }
    ));
}

#[test]
fn changed_binding_intervals_are_removed_and_added_not_a_changed_shared_binding() {
    let source = org::organizational_translation();
    let mut changed = source.clone();
    changed.occupant_bindings[0].interval.start += 1;
    changed.provider_bindings[0].interval.start += 1;
    let left = compile(Some(&source), None);
    let right = compile(Some(&changed), None);
    for (selected_owner, other_owner, expected) in
        [(&left, &right, &source), (&right, &left, &changed)]
    {
        for selected in rows(selected_owner, 256)
            .into_iter()
            .filter(|r| match r.record() {
                OrganizationInspectionRecord::OccupantBinding(v) => {
                    key::occupant(v) == key::occupant(&expected.occupant_bindings[0])
                }
                OrganizationInspectionRecord::ProviderBinding(v) => {
                    key::provider(v) == key::provider(&expected.provider_bindings[0])
                }
                _ => false,
            })
        {
            let result = finish(other_owner, selected_owner, &selected.retain(), 1);
            assert!(matches!(
                result.state(),
                OrganizationCorrespondenceState::Unmatched
            ));
        }
    }
}

#[test]
fn a_stable_explicit_relation_id_preserves_correspondence_when_an_endpoint_changes() {
    let source = org::organizational_translation();
    let mut changed = source.clone();
    let relation = &mut changed.relations[2];
    std::mem::swap(&mut relation.from, &mut relation.to);
    let pale_core::records::OrganizationalNodeRefV1::Role { role } = &relation.from else {
        panic!("collaboration source must remain a role");
    };
    changed.group_lenses[0].holder = role.clone();
    assert_ne!(source.relations[2].to, changed.relations[2].to);
    let left = compile(Some(&source), None);
    let right = compile(Some(&changed), None);
    let selected = rows(&left, 256).into_iter().find(|r| matches!(r.record(), OrganizationInspectionRecord::Relation(v) if v.relation_id == source.relations[2].relation_id)).unwrap();
    let result = finish(&right, &left, &selected.retain(), 1);
    assert!(matches!(
        result.state(),
        OrganizationCorrespondenceState::Matched {
            values: PortableWorkspaceValueComparison::Changed,
            ..
        }
    ));
}

#[test]
fn derived_identity_preserves_parallel_endpoints_and_original_canonical_order() {
    let source = org::organizational_translation();
    let owner = compile(Some(&source), None);
    let derived: Vec<_> = rows(&owner, 256)
        .into_iter()
        .filter_map(|r| match r.record() {
            OrganizationInspectionRecord::DerivedRelation(v) => Some(v.clone()),
            _ => None,
        })
        .collect();
    assert!(derived.len() > 1);
    let mut original = derived.clone();
    original.reverse();
    original.sort();
    assert_eq!(original, derived);
    let base = &derived[0];
    let mut value_change = base.clone();
    value_change.issuer = "issuer.changed".parse().unwrap();
    assert_eq!(key::derived(base), key::derived(&value_change));
    assert_ne!(base, &value_change);
    let mut different = base.clone();
    different.interval.start += 1;
    assert_ne!(key::derived(base), key::derived(&different));
    assert!(derived.iter().enumerate().all(|(i, a)| derived[i + 1..]
        .iter()
        .all(|b| key::derived(a) != key::derived(b))));
}

#[test]
fn limits_and_cursor_custody_precede_search_and_complete_absence_requires_exhaustion() {
    let source = org::organizational_translation();
    let selected_owner = compile(Some(&source), None);
    let other = compile(Some(&source), None);
    let equal_bytes = compile(Some(&source), None);
    let scopes: Vec<_> = rows(&selected_owner, 256)
        .into_iter()
        .filter(|r| matches!(r.record(), OrganizationInspectionRecord::Scope(_)))
        .collect();
    assert!(scopes.len() > 1);
    let selected = scopes.last().unwrap().retain();
    let first = other
        .correspond_organization_record(&selected_owner, &selected, None, 1)
        .unwrap();
    let OrganizationCorrespondenceState::Searching(cursor) = first.state() else {
        panic!("must charge earlier scope");
    };
    assert_eq!(first.inspected(), 1);
    assert!(equal_bytes
        .correspond_organization_record(&selected_owner, &selected, Some(cursor), 1)
        .is_err());
    assert!(other
        .correspond_organization_record(&selected_owner, &scopes[0].retain(), Some(cursor), 1)
        .is_err());
    assert!(other
        .correspond_organization_record(&selected_owner, &selected, Some(cursor), 2)
        .is_err());
    assert!(other
        .correspond_organization_record(&equal_bytes, &selected, None, 1)
        .is_err());
    assert!(other
        .correspond_organization_record(&selected_owner, &selected, None, 0)
        .is_err());
    assert!(other
        .correspond_organization_record(&selected_owner, &selected, Some(cursor), 1)
        .is_ok());
    let empty = compile(None, None);
    let absent = finish(&empty, &selected_owner, &selected, 1);
    assert_eq!(absent.inspected(), 0);
    assert!(matches!(
        absent.state(),
        OrganizationCorrespondenceState::Unmatched
    ));
}

#[test]
fn derived_values_follow_their_original_binding_issuer_without_collapsing_parallel_edges() {
    let source = org::organizational_translation();
    let mut changed = source.clone();
    changed.occupant_bindings[0].issuer = "issuer.changed".parse().unwrap();
    let left = compile(Some(&source), None);
    let right = compile(Some(&changed), None);
    let mut changed_count = 0;
    let mut same_topic_edges = std::collections::BTreeSet::new();
    for selected in rows(&left, 256)
        .into_iter()
        .filter(|r| matches!(r.record(), OrganizationInspectionRecord::DerivedRelation(_)))
    {
        let result = finish(&right, &left, &selected.retain(), 1);
        let OrganizationCorrespondenceState::Matched { other, values } = result.state() else {
            panic!("existing derived key lost");
        };
        if *values == PortableWorkspaceValueComparison::Changed {
            changed_count += 1;
        }
        if let OrganizationInspectionRecord::DerivedRelation(v) = other.record() {
            same_topic_edges.insert((
                v.source.clone(),
                v.class,
                v.from.clone(),
                v.to.clone(),
                v.interval.clone(),
            ));
        }
    }
    assert_eq!(changed_count, 1);
    assert!(same_topic_edges.len() > 1);
}
