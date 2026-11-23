//! Public comparison directions, nested values and complete source-handle custody.

use super::*;

fn finish(
    comparison: &PortableWorkspaceComparison,
    side: OrganizationComparisonSide,
    record: &OrganizationRecordHandle,
    limit: u16,
) -> OrganizationCorrespondence {
    let mut after = None;
    for _ in 0..10_000 {
        let result = comparison
            .correspond_organization_record(side, record, after.as_ref(), limit, BYTES)
            .unwrap();
        assert!(result.belongs_to(comparison));
        assert_eq!(result.selected_side(), side);
        assert!(result.inspected() <= limit);
        match result.state() {
            OrganizationCorrespondenceState::Searching(next) => after = Some(next.clone()),
            _ => return result,
        }
    }
    panic!("bounded correspondence did not terminate");
}

#[test]
fn eighteen_record_families_retain_both_source_owners_and_reverse_direction() {
    let prior = compile(inputs::documents());
    let current = compile(inputs::documents());
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let mut families = std::collections::HashSet::new();
    for (side, selected_owner, other_owner) in [
        (OrganizationComparisonSide::Prior, &prior, &current),
        (OrganizationComparisonSide::Current, &current, &prior),
    ] {
        for selected in entries(selected_owner, 256) {
            families.insert(std::mem::discriminant(selected.row()));
            let result = finish(&comparison, side, selected.handle(), 1);
            assert!(result.selected().belongs_to(selected_owner));
            let OrganizationCorrespondenceState::Matched { other, values } = result.state() else {
                panic!("missing exact record");
            };
            assert_eq!(*values, PortableWorkspaceValueComparison::Equal);
            assert_eq!(other.row(), selected.row());
            assert!(other.handle().belongs_to(other_owner));
            assert!(!other.handle().belongs_to(selected_owner));
            // Both native source-query outcomes remain available, including the
            // explicit unmapped disposition for full binding/derived identities.
            assert_eq!(
                std::mem::discriminant(
                    result
                        .selected()
                        .sources(None, 256, 256, BYTES)
                        .unwrap()
                        .mapping()
                ),
                std::mem::discriminant(
                    other
                        .handle()
                        .sources(None, 256, 256, BYTES)
                        .unwrap()
                        .mapping()
                )
            );
        }
    }
    assert_eq!(families.len(), 18);
}

#[test]
fn same_record_reuses_exact_counterpart_handles_without_equating_generations() {
    let prior = compile(inputs::documents());
    let current = compile(inputs::documents());
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let first = entries(&prior, 1);
    let repeated = entries(&prior, 256);
    let other_generation = entries(&current, 256);
    assert_eq!(prior.semantic_ir_digest(), current.semantic_ir_digest());
    for ((selected, same), other) in first.iter().zip(&repeated).zip(&other_generation) {
        assert!(selected.handle().same_record(same.handle()));
        assert!(same.handle().same_record(selected.handle()));
        assert_eq!(selected.row(), other.row());
        assert!(!selected.handle().same_record(other.handle()));
        let result = finish(
            &comparison,
            OrganizationComparisonSide::Prior,
            selected.handle(),
            256,
        );
        let OrganizationCorrespondenceState::Matched {
            other: counterpart, ..
        } = result.state()
        else {
            panic!("same-key counterpart missing");
        };
        assert!(counterpart.handle().same_record(other.handle()));
        assert!(!counterpart.handle().same_record(selected.handle()));
    }
    assert!(!first[0].handle().same_record(first[1].handle()));
    let other_family = first
        .iter()
        .find(|entry| matches!(entry.row(), OrganizationRow::Role { .. }))
        .unwrap();
    assert!(!first[0].handle().same_record(other_family.handle()));
}

#[test]
fn unchanged_nested_row_counts_do_not_hide_changed_canonical_values() {
    let documents = inputs::documents();
    let mut changed = documents.clone();
    let (_, bytes) = changed
        .iter_mut()
        .find(|(name, _)| name == "organization.toml")
        .unwrap();
    let mut source: toml::Value = toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap();
    let roles = source["roles"].as_array_mut().unwrap();
    let resources = roles[0]["axes"]["resources"].as_array_mut().unwrap();
    assert!(!resources.is_empty());
    resources[0] = toml::Value::String("resource.changed".into());
    *bytes = toml::to_string(&source).unwrap().into_bytes();
    let prior = compile(documents);
    let current = compile(changed);
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let mut changed_count = 0;
    for selected in entries(&prior, 256)
        .into_iter()
        .filter(|entry| matches!(entry.row(), OrganizationRow::Role { .. }))
    {
        let result = finish(
            &comparison,
            OrganizationComparisonSide::Prior,
            selected.handle(),
            256,
        );
        let OrganizationCorrespondenceState::Matched { other, values } = result.state() else {
            panic!("role disappeared");
        };
        if *values == PortableWorkspaceValueComparison::Changed {
            changed_count += 1;
            assert_eq!(
                selected.row(),
                other.row(),
                "shallow count row is deliberately not value equality"
            );
            let before = selected
                .handle()
                .axis_resources(None, 256, 256, BYTES)
                .unwrap();
            let after = other
                .handle()
                .axis_resources(None, 256, 256, BYTES)
                .unwrap();
            let OrganizationDetailRows::AxisResources(before) = before.rows() else {
                panic!("wrong section");
            };
            let OrganizationDetailRows::AxisResources(after) = after.rows() else {
                panic!("wrong section");
            };
            assert_ne!(before, after);
        }
    }
    assert_eq!(changed_count, 1);
}

#[test]
fn exact_pair_side_selected_record_and_raw_bounds_cannot_be_substituted() {
    let prior = compile(inputs::documents());
    let current = compile(inputs::documents());
    let duplicate = compile(inputs::documents());
    let api = PaleApi::new().unwrap();
    let comparison = api.compare_portable_workspaces(&prior, &current).unwrap();
    let other_pair = api.compare_portable_workspaces(&prior, &duplicate).unwrap();
    let reversed = api.compare_portable_workspaces(&current, &prior).unwrap();
    let scopes: Vec<_> = entries(&prior, 256)
        .into_iter()
        .filter(|row| matches!(row.row(), OrganizationRow::Scope { .. }))
        .collect();
    let selected = scopes.last().unwrap().handle();
    let first = comparison
        .correspond_organization_record(OrganizationComparisonSide::Prior, selected, None, 1, BYTES)
        .unwrap();
    let OrganizationCorrespondenceState::Searching(cursor) = first.state() else {
        panic!("expected nonmatch visit");
    };
    for (pair, side, record, visits, bytes) in [
        (
            &other_pair,
            OrganizationComparisonSide::Prior,
            selected,
            1,
            BYTES,
        ),
        (
            &reversed,
            OrganizationComparisonSide::Current,
            selected,
            1,
            BYTES,
        ),
        (
            &comparison,
            OrganizationComparisonSide::Current,
            selected,
            1,
            BYTES,
        ),
        (
            &comparison,
            OrganizationComparisonSide::Prior,
            scopes[0].handle(),
            1,
            BYTES,
        ),
        (
            &comparison,
            OrganizationComparisonSide::Prior,
            selected,
            2,
            BYTES,
        ),
        (
            &comparison,
            OrganizationComparisonSide::Prior,
            selected,
            1,
            BYTES - 1,
        ),
    ] {
        assert!(pair
            .correspond_organization_record(side, record, Some(cursor), visits, bytes)
            .is_err());
    }
    assert!(comparison
        .correspond_organization_record(
            OrganizationComparisonSide::Prior,
            selected,
            Some(cursor),
            1,
            BYTES
        )
        .is_ok());
    assert!(comparison
        .correspond_organization_record(OrganizationComparisonSide::Prior, selected, None, 0, BYTES)
        .is_err());
    assert!(comparison
        .correspond_organization_record(OrganizationComparisonSide::Prior, selected, None, 1, 0)
        .is_err());
}

#[test]
fn copy_failure_does_not_consume_the_input_handle_or_cursor() {
    let prior = compile(inputs::documents());
    let current = compile(inputs::documents());
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let scopes: Vec<_> = entries(&prior, 256)
        .into_iter()
        .filter(|row| matches!(row.row(), OrganizationRow::Scope { .. }))
        .collect();
    let selected = scopes[1].handle();
    let page = comparison
        .correspond_organization_record(OrganizationComparisonSide::Prior, selected, None, 1, 1)
        .unwrap();
    let OrganizationCorrespondenceState::Searching(cursor) = page.state() else {
        panic!("expected nonmatch visit");
    };
    for _ in 0..2 {
        let result = comparison.correspond_organization_record(
            OrganizationComparisonSide::Prior,
            selected,
            Some(cursor),
            1,
            1,
        );
        assert!(
            result.is_err(),
            "matching row exceeds one-byte output budget"
        );
    }
    assert!(selected.sources(None, 256, 256, BYTES).is_ok());
    assert!(matches!(
        finish(
            &comparison,
            OrganizationComparisonSide::Prior,
            selected,
            256
        )
        .state(),
        OrganizationCorrespondenceState::Matched { .. }
    ));
}

#[test]
fn source_reordering_preserves_values_but_not_original_occurrence_positions() {
    let documents = inputs::documents();
    let mut reordered = documents.clone();
    let (_, bytes) = reordered
        .iter_mut()
        .find(|(name, _)| name == "organization.toml")
        .unwrap();
    let mut source: toml::Value = toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap();
    source["roles"].as_array_mut().unwrap().reverse();
    *bytes = toml::to_string(&source).unwrap().into_bytes();
    let prior = compile(documents);
    let current = compile(reordered);
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let selected = entries(&prior, 256)
        .into_iter()
        .find(|entry| matches!(entry.row(), OrganizationRow::Role { .. }))
        .unwrap();
    let result = finish(
        &comparison,
        OrganizationComparisonSide::Prior,
        selected.handle(),
        1,
    );
    let OrganizationCorrespondenceState::Matched { other, values } = result.state() else {
        panic!("role disappeared");
    };
    assert_eq!(*values, PortableWorkspaceValueComparison::Equal);
    let before = selected.handle().sources(None, 256, 256, BYTES).unwrap();
    let after = other.handle().sources(None, 256, 256, BYTES).unwrap();
    let OrganizationSourceMapping::Named {
        occurrences: before,
        ..
    } = before.mapping()
    else {
        panic!("named source missing");
    };
    let OrganizationSourceMapping::Named {
        occurrences: after, ..
    } = after.mapping()
    else {
        panic!("named source missing");
    };
    let definition = |occurrences: &[pale_sdk::AuthoringOccurrence]| {
        occurrences
            .iter()
            .find(|occurrence| occurrence.role() == pale_sdk::AuthoringOccurrenceRole::Definition)
            .expect("role definition missing")
            .location()
            .path()
            .to_vec()
    };
    assert_ne!(definition(before), definition(after));
}

#[test]
fn interval_changes_remain_removed_and_added_in_each_comparison_direction() {
    let documents = inputs::documents();
    let mut changed = documents.clone();
    let (_, bytes) = changed
        .iter_mut()
        .find(|(name, _)| name == "organization.toml")
        .unwrap();
    let mut source: toml::Value = toml::from_str(std::str::from_utf8(bytes).unwrap()).unwrap();
    source["occupant_bindings"][0]["interval"]["start"] = toml::Value::Integer(2);
    *bytes = toml::to_string(&source).unwrap().into_bytes();
    let prior = compile(documents);
    let current = compile(changed);
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    for (side, owner) in [
        (OrganizationComparisonSide::Prior, &prior),
        (OrganizationComparisonSide::Current, &current),
    ] {
        let selected = entries(owner, 256)
            .into_iter()
            .find(|entry| matches!(entry.row(), OrganizationRow::OccupantBinding { .. }))
            .unwrap();
        let first = comparison
            .correspond_organization_record(side, selected.handle(), None, 1, BYTES)
            .unwrap();
        let OrganizationCorrespondenceState::Searching(cursor) = first.state() else {
            panic!("nonmatch must consume a visit before absence");
        };
        assert_eq!(first.inspected(), 1);
        assert_eq!(first.copied_string_bytes(), 0);
        let terminal = comparison
            .correspond_organization_record(side, selected.handle(), Some(cursor), 1, BYTES)
            .unwrap();
        assert!(matches!(
            terminal.state(),
            OrganizationCorrespondenceState::Unmatched
        ));
        assert_eq!(terminal.inspected(), 0);
        assert!(matches!(
            terminal
                .selected()
                .sources(None, 256, 256, BYTES)
                .unwrap()
                .mapping(),
            OrganizationSourceMapping::UnmappedRecord
        ));
    }
}
