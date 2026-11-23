//! Native child input checks over the actual protected parent and narrowed lease.
//! Lease retention is a prior fixture prerequisite, not a child-start transaction.

use super::*;
use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
enum Case {
    Valid,
    Malformed,
    WrongKind,
    WrongSchema,
    WrongDigest,
    HandlerDomainFailure,
}

pub(super) fn check(
    fixture: &Fixture<'_>,
    expected: &OperationObservationStateV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) {
    let custody = RefCell::new((port, anchor));
    for (case, suffix) in [
        (Case::Valid, "child-ingress-valid"),
        (Case::Malformed, "child-ingress-malformed"),
        (Case::WrongKind, "child-ingress-kind"),
        (Case::WrongSchema, "child-ingress-schema"),
        (Case::WrongDigest, "child-ingress-digest"),
        (Case::HandlerDomainFailure, "child-ingress-domain"),
    ] {
        let writes_after_lease = Cell::new(0);
        with_child_start(
            fixture,
            fixture::Admission::Native,
            |lease_ref, lease| {
                let mut borrowed = custody.borrow_mut();
                let (port, anchor) = &mut *borrowed;
                let coordinate = port.coordinate.clone();
                retain_operation_lease_body(*port, &coordinate, lease_ref, lease, 1 << 20).unwrap();
                writes_after_lease.set(port.connection.total_changes());
                prepare_parent(fixture, &[expected], port, anchor, || {}).unwrap()
            },
            suffix,
            |request| {
                match case {
                    Case::Malformed => request.body.payload["extra"] = serde_json::json!(true),
                    Case::WrongKind => {
                        request.body.payload = serde_json::json!({"kind":"weighted_ranking","payload":{
                            "weights":[{"id":"quality","weight_micros":1_000_000}],
                            "candidates":[{"id":"one","scores":[{"weight_id":"quality","value_micros":500_000}]}]
                        }})
                    }
                    Case::WrongSchema => {
                        request.body.schema_id = SchemaId::parse("example/unlike-input.v1").unwrap()
                    }
                    Case::WrongDigest => {
                        request.body.schema_sha256 = SchemaDigest::parse("0".repeat(64)).unwrap()
                    }
                    Case::HandlerDomainFailure => {
                        request.body.payload["payload"]["paths"] = serde_json::json!(["../outside"])
                    }
                    Case::Valid => {}
                }
                if !matches!(case, Case::Malformed) {
                    pale_contracts::validate_value(&request.body.payload, "action-input.v1")
                        .unwrap();
                }
            },
            |result, _, _, owners, signs| {
                match case {
                    Case::Malformed => {
                        assert!(matches!(
                            result,
                            Err(LocalOperationStartError::NativeAction(
                                NativeActionInvocationError::MalformedInput(_)
                            ))
                        ));
                        assert_eq!((owners, signs), (0, 0));
                    }
                    Case::WrongKind => {
                        assert!(
                            matches!(result, Err(LocalOperationStartError::NativeAction(NativeActionInvocationError::Action(error))) if error.code == ActionErrorCode::InputMismatch)
                        );
                        assert_eq!((owners, signs), (0, 0));
                    }
                    Case::WrongSchema | Case::WrongDigest => {
                        assert!(matches!(result, Err(LocalOperationStartError::Coordinate)));
                        assert_eq!((owners, signs), (0, 0));
                    }
                    Case::Valid | Case::HandlerDomainFailure => {
                        // A domain-invalid path is admitted here, not executed;
                        // the actual handler rejection remains in the parent journey.
                        drop(result.unwrap_or_else(|error| panic!("{case:?}: {error:?}")));
                        assert_eq!((owners, signs), (1, 2));
                    }
                }
                let mut borrowed = custody.borrow_mut();
                let (port, _) = &mut *borrowed;
                assert_eq!(
                    port.connection.total_changes(),
                    writes_after_lease.get(),
                    "{case:?}: child preparation cannot write after retained lease"
                );
                for prefix in ["child-request", "child-handle", "child-observation"] {
                    let reference = format!("{prefix}/{suffix}");
                    let records: i64 = port.connection.query_row(
                        "SELECT COUNT(*) FROM pale_deployment_control_record WHERE record_ref=?1", [&reference], |row| row.get(0),
                    ).unwrap();
                    let indexes: i64 = port.connection.query_row(
                        "SELECT COUNT(*) FROM pale_deployment_operation_index WHERE record_ref=?1", [&reference], |row| row.get(0),
                    ).unwrap();
                    assert_eq!(
                        (records, indexes),
                        (0, 0),
                        "{case:?}: no child locator or index before custody"
                    );
                }
            },
        );
    }

    let writes_after_lease = Cell::new(0);
    with_child_start(
        fixture,
        fixture::Admission::Delegate,
        |lease_ref, lease| {
            let mut borrowed = custody.borrow_mut();
            let (port, anchor) = &mut *borrowed;
            let coordinate = port.coordinate.clone();
            retain_operation_lease_body(*port, &coordinate, lease_ref, lease, 1 << 20).unwrap();
            writes_after_lease.set(port.connection.total_changes());
            prepare_parent(fixture, &[expected], port, anchor, || {}).unwrap()
        },
        "child-delegate",
        |_| {},
        |result, _, _, owners, signs| {
            drop(result.expect("narrow child Delegate is prepared"));
            assert_eq!((owners, signs), (1, 2));
            let mut borrowed = custody.borrow_mut();
            let (port, _) = &mut *borrowed;
            assert_eq!(port.connection.total_changes(), writes_after_lease.get());
        },
    );
}
