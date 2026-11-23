//! Physical producer-incarnation transition closure tests.

use super::*;
use crate::{ControlRecordBytes, ControlRecordDigest};

use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, PrincipalTrustCheckpointEpochV1, ResidentRecordRefV1,
    SchemaDigest, SchemaId,
};

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record ref"),
        ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
            .expect("record digest"),
    )
}

fn record(reference: &str, byte: char) -> ControlRecord {
    record_with_schema(reference, byte, "pale/test.v1")
}

fn record_with_schema(reference: &str, byte: char, schema: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse(schema).expect("schema"),
        SchemaDigest::parse("a".repeat(64)).expect("schema digest"),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes())
            .expect("record bytes"),
    )
}

fn binding() -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("trust-store-1").expect("store"),
        ControlIncarnationIdV1::parse("trust-incarnation-1").expect("incarnation"),
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
    )
}

fn trust_head(epoch: u64) -> PrincipalTrustCheckpointHead {
    PrincipalTrustCheckpointHead::new(
        binding().principal_id().clone(),
        PrincipalTrustCheckpointEpochV1::new(epoch).expect("trust epoch"),
        coordinate(&format!("trust-{epoch}"), digit(epoch)),
    )
}

fn snapshot() -> PrincipalTrustSnapshot {
    PrincipalTrustSnapshot::new(binding(), trust_head(1), record("trust-1", '1'))
        .expect("producer recovery snapshot")
}

fn checkpoint(sequence: u64) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("producer:producer-1").expect("domain"),
        ControlDomainEpochV1::new(1).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        pale_core::records::ControlTransactionSequenceV1::new(sequence + 20).unwrap(),
        coordinate(&format!("control-{sequence}"), digit(sequence + 4)),
    )
}

fn digit(value: u64) -> char {
    match value {
        0..=9 => char::from(b'0' + u8::try_from(value).expect("single decimal digit")),
        _ => panic!("test coordinate exceeds one digit"),
    }
}

fn producer_head(
    epoch: char,
    state: ProducerIncarnationHeadStateV1,
    retirement: Option<ControlRecordCoordinate>,
) -> PrincipalProducerIncarnationHead {
    PrincipalProducerIncarnationHead::new(
        binding().principal_id().clone(),
        ProducerIdV1::parse("producer-1").expect("producer"),
        ProducerEpochV1::parse(std::iter::repeat_n(epoch, 32).collect::<String>())
            .expect("producer epoch"),
        state,
        coordinate(&format!("allocation-{epoch}"), epoch),
        retirement,
    )
    .expect("producer head")
}

fn expected_frontier(
    producer: Option<(PrincipalProducerIncarnationHead, Vec<ControlRecord>)>,
) -> PrincipalProducerIncarnationExpectedFrontier {
    PrincipalProducerIncarnationExpectedFrontier::new(
        producer,
        (trust_head(1), record("trust-1", '1')),
        (checkpoint(1), record("control-1", '5')),
    )
}

fn next_frontier(
    producer: PrincipalProducerIncarnationHead,
) -> PrincipalProducerIncarnationSuccessorFrontier {
    PrincipalProducerIncarnationSuccessorFrontier::new(producer, trust_head(2), checkpoint(2))
}

#[test]
fn all_three_producer_transition_shapes_close_exactly() {
    let initial = producer_head('a', ProducerIncarnationHeadStateV1::Active, None);
    PreparedPrincipalProducerIncarnationTransition::new(
        binding(),
        PrincipalProducerIncarnationTransitionKind::InitialAllocation,
        expected_frontier(None),
        vec![
            record("allocation-a", 'a'),
            record("trust-2", '2'),
            record("control-2", '6'),
        ],
        next_frontier(initial.clone()),
    )
    .expect("initial allocation");

    let retirement_coordinate = coordinate("retirement-a", 'b');
    let retired = producer_head(
        'a',
        ProducerIncarnationHeadStateV1::Retired,
        Some(retirement_coordinate),
    );
    PreparedPrincipalProducerIncarnationTransition::new(
        binding(),
        PrincipalProducerIncarnationTransitionKind::Retirement,
        expected_frontier(Some((initial, vec![record("allocation-a", 'a')]))),
        vec![
            record("retirement-a", 'b'),
            record("trust-2", '2'),
            record("control-2", '6'),
        ],
        next_frontier(retired.clone()),
    )
    .expect("retirement");

    let successor = producer_head('c', ProducerIncarnationHeadStateV1::Active, None);
    PreparedPrincipalProducerIncarnationTransition::new(
        binding(),
        PrincipalProducerIncarnationTransitionKind::SuccessorAllocation,
        expected_frontier(Some((
            retired,
            vec![record("allocation-a", 'a'), record("retirement-a", 'b')],
        ))),
        vec![
            record("allocation-c", 'c'),
            record("trust-2", '2'),
            record("control-2", '6'),
        ],
        next_frontier(successor),
    )
    .expect("successor allocation");
}

#[test]
fn producer_transition_rejects_partial_predecessor_and_skipped_frontiers() {
    let retirement_coordinate = coordinate("retirement-a", 'b');
    let retired = producer_head(
        'a',
        ProducerIncarnationHeadStateV1::Retired,
        Some(retirement_coordinate),
    );
    let successor = producer_head('c', ProducerIncarnationHeadStateV1::Active, None);
    let partial = PreparedPrincipalProducerIncarnationTransition::new(
        binding(),
        PrincipalProducerIncarnationTransitionKind::SuccessorAllocation,
        expected_frontier(Some((retired.clone(), vec![record("allocation-a", 'a')]))),
        vec![
            record("allocation-c", 'c'),
            record("trust-2", '2'),
            record("control-2", '6'),
        ],
        next_frontier(successor.clone()),
    );
    assert_eq!(
        partial,
        Err(PrincipalProducerIncarnationCandidateError::ProducerHead)
    );

    let skipped =
        PrincipalProducerIncarnationSuccessorFrontier::new(successor, trust_head(3), checkpoint(2));
    let rejected = PreparedPrincipalProducerIncarnationTransition::new(
        binding(),
        PrincipalProducerIncarnationTransitionKind::SuccessorAllocation,
        expected_frontier(Some((
            retired,
            vec![record("allocation-a", 'a'), record("retirement-a", 'b')],
        ))),
        vec![
            record("allocation-c", 'c'),
            record("trust-3", '3'),
            record("control-2", '6'),
        ],
        skipped,
    );
    assert_eq!(
        rejected,
        Err(PrincipalProducerIncarnationCandidateError::TrustCheckpointClosure)
    );
}

#[test]
fn producer_point_reads_close_active_and_retired_record_sets() {
    let active = producer_head('a', ProducerIncarnationHeadStateV1::Active, None);
    let active_records = PrincipalProducerIncarnationRecordSet::new(
        active.clone(),
        None,
        record_with_schema("allocation-a", 'a', PRODUCER_INCARNATION_V1_SCHEMA),
        None,
    )
    .expect("active incarnation records");
    assert_eq!(active_records.head(), &active);
    assert!(active_records.retirement().is_none());

    let retirement_coordinate = coordinate("retirement-a", 'b');
    let retired = producer_head(
        'a',
        ProducerIncarnationHeadStateV1::Retired,
        Some(retirement_coordinate),
    );
    let retired_records = PrincipalProducerIncarnationRecordSet::new(
        retired.clone(),
        None,
        record_with_schema("allocation-a", 'a', PRODUCER_INCARNATION_V1_SCHEMA),
        Some(record_with_schema(
            "retirement-a",
            'b',
            PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA,
        )),
    )
    .expect("retired incarnation records");
    assert_eq!(retired_records.head(), &retired);
    assert_eq!(
        retired_records.head().state(),
        ProducerIncarnationHeadStateV1::Retired
    );
    assert!(retired_records.retirement().is_some());
}

#[test]
fn producer_point_reads_reject_wrong_schema_or_retirement_branch() {
    let active = producer_head('a', ProducerIncarnationHeadStateV1::Active, None);
    assert_eq!(
        PrincipalProducerIncarnationRecordSet::new(
            active.clone(),
            None,
            record("allocation-a", 'a'),
            None,
        ),
        Err(PrincipalProducerIncarnationReadError::Allocation)
    );
    assert_eq!(
        PrincipalProducerIncarnationRecordSet::new(
            active,
            None,
            record_with_schema("allocation-a", 'a', PRODUCER_INCARNATION_V1_SCHEMA),
            Some(record_with_schema(
                "retirement-a",
                'b',
                PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA,
            )),
        ),
        Err(PrincipalProducerIncarnationReadError::Retirement)
    );
}

#[test]
fn producer_head_pages_enforce_principal_order_bound_and_continuation() {
    let request =
        PrincipalProducerIncarnationHeadPageRead::new(snapshot(), None, 1).expect("page request");
    let head = producer_head('a', ProducerIncarnationHeadStateV1::Active, None);
    let records = PrincipalProducerIncarnationRecordSet::new(
        head,
        None,
        record_with_schema("allocation-a", 'a', PRODUCER_INCARNATION_V1_SCHEMA),
        None,
    )
    .expect("record set");
    let cursor = PrincipalProducerIncarnationHeadCursor::new(
        ProducerIdV1::parse("producer-1").expect("producer"),
    );
    let entry = PrincipalProducerIncarnationHeadPageEntry::new(cursor.clone(), records)
        .expect("matching page entry");
    let page = PrincipalProducerIncarnationHeadPage::new(
        &request,
        vec![entry.clone()],
        PrincipalProducerIncarnationHeadPageBoundary::More {
            resume_after: cursor.clone(),
        },
    )
    .expect("bounded page");
    assert_eq!(page.request(), &request);
    assert_eq!(page.through(), request.through());
    assert_eq!(page.entries(), &[entry]);

    let repeated_request =
        PrincipalProducerIncarnationHeadPageRead::new(snapshot(), Some(cursor.clone()), 1)
            .expect("continuation request");
    let repeated = PrincipalProducerIncarnationHeadPage::new(
        &repeated_request,
        vec![page.entries()[0].clone()],
        PrincipalProducerIncarnationHeadPageBoundary::Complete,
    );
    assert_eq!(
        repeated,
        Err(PrincipalProducerIncarnationHeadPageError::NonAscending)
    );

    let wrong_continuation = PrincipalProducerIncarnationHeadPage::new(
        &request,
        page.entries().to_vec(),
        PrincipalProducerIncarnationHeadPageBoundary::More {
            resume_after: PrincipalProducerIncarnationHeadCursor::new(
                ProducerIdV1::parse("producer-2").expect("producer"),
            ),
        },
    );
    assert_eq!(
        wrong_continuation,
        Err(PrincipalProducerIncarnationHeadPageError::Continuation)
    );
}

#[test]
fn producer_head_page_request_rejects_zero_bound_and_foreign_checkpoint() {
    assert_eq!(
        PrincipalProducerIncarnationHeadPageRead::new(snapshot(), None, 0,),
        Err(PrincipalProducerIncarnationHeadPageError::ZeroBound)
    );
    assert_eq!(
        PrincipalProducerIncarnationHeadPageRead::new(
            snapshot(),
            None,
            MAX_RESIDENT_SET_MEMBERS + 1,
        ),
        Err(PrincipalProducerIncarnationHeadPageError::BoundExceeded)
    );
}
