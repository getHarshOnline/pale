//! Bounded discovery behavior for committed operation starts.

use super::*;
use pale_core::records::{
    ControlIncarnationIdV1, ControlStoreIdV1, ControlTransactionSequenceV1, ResidentRecordRefV1,
    SessionAdmissionLeaseDigest,
};
use pale_ports::{
    ControlRecordCoordinate, ControlRecordDigest, ControlStoreErrorKind, OperationSessionBasis,
    OperationStartSources,
};

impl LocalOperationStartDiscoveryAttempt {
    pub(in crate::resident) const fn is_exhausted(&self) -> bool {
        self.exhausted
    }
}

fn recovery() -> ControlStoreError {
    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired)
}

fn store(label: &str) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse(format!("{label}-store")).unwrap(),
        ControlIncarnationIdV1::parse(format!("{label}-incarnation")).unwrap(),
    )
}

fn coordinate(label: &str, fill: u8) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(label).unwrap(),
        ControlRecordDigest::parse(char::from(fill).to_string().repeat(64)).unwrap(),
    )
}

fn source(transaction: u64, label: &str) -> OperationStartSource {
    OperationStartSource::new(
        ControlTransactionSequenceV1::new(transaction).unwrap(),
        OperationSessionBasis::new(
            coordinate(&format!("{label}/handle"), b'a'),
            coordinate(&format!("{label}/request"), b'b'),
            SessionAdmissionLeaseDigest::parse("c".repeat(64)).unwrap(),
            format!("{{\"source\":\"{label}\"}}").into_bytes(),
            1024,
        )
        .unwrap(),
    )
}

fn cursor(
    store: &AuthorityStoreCoordinate,
    through: u64,
    source: &OperationStartSource,
) -> OperationStartCursor {
    OperationStartCursor::new(
        store.clone(),
        ControlTransactionSequenceV1::new(through).unwrap(),
        source.transaction(),
        source.session_basis().handle().clone(),
        source.session_basis().request().clone(),
        source.session_basis().session_lease_digest().clone(),
        source.session_basis().basis_sha256().clone(),
    )
    .unwrap()
}

#[test]
fn one_source_turn_advances_only_after_disposition() {
    let selected = store("bounded-turn");
    let first = source(4, "first");
    let expected_cursor = cursor(&selected, 9, &first);
    let attempt = LocalOperationStartDiscoveryAttempt::new(selected.clone());
    let turn = discover_local_operation_start_with(attempt.clone(), 1024, |store, prior, bytes| {
        assert_eq!(store, &selected);
        assert!(prior.is_none());
        assert_eq!(bytes, 1024);
        Ok(OperationStartSources::new(
            ControlTransactionSequenceV1::new(9).unwrap(),
            vec![first],
            Some(expected_cursor.clone()),
        ))
    })
    .unwrap();
    let LocalOperationStartDiscoveryTurn::Pending(pending) = turn else {
        panic!("one source must remain pending")
    };
    assert_eq!(pending.source().transaction().get(), 4);
    assert_eq!(attempt.cursor, None);
    let advanced = pending.advance();
    assert!(!advanced.is_exhausted());
    assert_eq!(advanced.cursor, Some(expected_cursor));
}

#[test]
fn frozen_prefix_is_reused_and_final_source_exhausts_without_another_read() {
    let selected = store("frozen-prefix");
    let first = source(4, "first");
    let second = source(7, "second");
    let first_cursor = cursor(&selected, 7, &first);
    let first_turn = discover_local_operation_start_with(
        LocalOperationStartDiscoveryAttempt::new(selected.clone()),
        1024,
        |_, _, _| {
            Ok(OperationStartSources::new(
                ControlTransactionSequenceV1::new(7).unwrap(),
                vec![first],
                Some(first_cursor),
            ))
        },
    )
    .unwrap();
    let LocalOperationStartDiscoveryTurn::Pending(first_pending) = first_turn else {
        panic!("first source")
    };
    let second_turn =
        discover_local_operation_start_with(first_pending.advance(), 1024, |_, prior, _| {
            assert_eq!(prior.unwrap().through().get(), 7);
            Ok(OperationStartSources::new(
                ControlTransactionSequenceV1::new(7).unwrap(),
                vec![second],
                None,
            ))
        })
        .unwrap();
    let LocalOperationStartDiscoveryTurn::Pending(second_pending) = second_turn else {
        panic!("second source")
    };
    let exhausted = second_pending.advance();
    assert!(exhausted.is_exhausted());
    let no_read = discover_local_operation_start_with(exhausted, 1024, |_, _, _| {
        panic!("an exhausted frozen prefix must perform no store read")
    })
    .unwrap();
    assert!(matches!(
        no_read,
        LocalOperationStartDiscoveryTurn::Exhausted(_)
    ));
}

#[test]
fn hostile_pages_and_invalid_budgets_fail_without_advancing() {
    let selected = store("hostile-page");
    let attempt = LocalOperationStartDiscoveryAttempt::new(selected);
    assert!(matches!(
        discover_local_operation_start_with(attempt.clone(), 0, |_, _, _| unreachable!()),
        Err(LocalOperationStartDiscoveryError::Bounds)
    ));
    let result = discover_local_operation_start_with(attempt, 1024, |_, _, _| {
        Ok(OperationStartSources::new(
            ControlTransactionSequenceV1::new(5).unwrap(),
            vec![source(1, "one"), source(2, "two")],
            None,
        ))
    });
    assert!(matches!(
        result,
        Err(LocalOperationStartDiscoveryError::Page)
    ));
}

#[test]
fn changed_prefix_and_misdirected_continuation_fail_closed() {
    let selected = store("changed-prefix");
    let first = source(1, "one");
    let prior = cursor(&selected, 3, &first);
    let attempt = LocalOperationStartDiscoveryAttempt {
        store: selected.clone(),
        cursor: Some(prior),
        exhausted: false,
    };
    let result = discover_local_operation_start_with(attempt.clone(), 1024, |_, _, _| {
        Ok(OperationStartSources::new(
            ControlTransactionSequenceV1::new(4).unwrap(),
            vec![source(2, "two")],
            None,
        ))
    });
    assert!(matches!(
        result,
        Err(LocalOperationStartDiscoveryError::Page)
    ));

    let returned = source(2, "two");
    let wrong = cursor(&selected, 3, &source(2, "other"));
    let result = discover_local_operation_start_with(attempt, 1024, |_, _, _| {
        Ok(OperationStartSources::new(
            ControlTransactionSequenceV1::new(3).unwrap(),
            vec![returned],
            Some(wrong),
        ))
    });
    assert!(matches!(
        result,
        Err(LocalOperationStartDiscoveryError::Page)
    ));
}

#[test]
fn read_errors_preserve_their_storage_classification() {
    let attempt = LocalOperationStartDiscoveryAttempt::new(store("read-error"));
    let result = discover_local_operation_start_with(attempt, 1024, |_, _, _| {
        Err(OperationStartReadError::Storage(recovery()))
    });
    assert!(matches!(
        result,
        Err(LocalOperationStartDiscoveryError::Read(_))
    ));
}
