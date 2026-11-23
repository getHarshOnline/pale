//! Typed publication/outbox closure, with the existing model admission fixtures.

use super::*;
use pale_ports::EventTransitionMemberKind as M;

fn publications(
    maximum_publication_bytes: usize,
    retention_reference: &str,
) -> [Result<PreparedEventCustodyTransition, EventCustodyProjectionError>; 3] {
    let source_event = event("source-event", "source-revision");
    let destination_event = event("destination-event", "destination-revision");
    let source_event_ref = record_ref("source-event-record");
    let destination_event_ref = record_ref("destination-event-record");
    let source = admitted_occurrence(
        "source",
        &source_event_ref,
        source_event.canonical_event_sha256(),
    );
    let destination = admitted_occurrence(
        "destination",
        &destination_event_ref,
        destination_event.canonical_event_sha256(),
    );
    let source_ref = record_ref("source-occurrence-record");
    let destination_ref = record_ref("destination-occurrence-record");
    let edge = admitted_edge("edge", &source_ref, &source, &destination_ref, &destination);
    let frontier = admitted_frontier(
        "frontier",
        "subject",
        ReconciliationDimensionStatusV1::Pending,
    );
    let publication_ref = record_ref("publication-record");
    let retention_ref = record_ref(retention_reference);
    [
        prepare(EventCustodyTransitionInput::ProjectInteractionOccurrence {
            event: AdmittedEventRecord::new(&source_event_ref, &source_event),
            occurrence: AdmittedEventRecord::new(&publication_ref, &source),
            retention_record_ref: &retention_ref,
            maximum_publication_bytes,
        }),
        prepare(EventCustodyTransitionInput::ProjectGenealogyEdge {
            source: AdmittedEventRecord::new(&source_ref, &source),
            destination: AdmittedEventRecord::new(&destination_ref, &destination),
            edge: AdmittedEventRecord::new(&publication_ref, &edge),
            retention_record_ref: &retention_ref,
            maximum_publication_bytes,
        }),
        prepare(EventCustodyTransitionInput::AdvanceReconciliationFrontier {
            predecessor: None,
            successor: AdmittedEventRecord::new(&publication_ref, &frontier),
            retention_record_ref: &retention_ref,
            maximum_publication_bytes,
        }),
    ]
}

#[test]
fn all_typed_publications_retain_the_exact_family_coordinate_and_one_exposure() {
    for (result, family) in publications(16_384, "publication-outbox").into_iter().zip([
        M::InteractionOccurrence,
        M::GenealogyEdge,
        M::ReconciliationFrontier,
    ]) {
        let prepared = result.expect("typed publication");
        let candidate = prepared.candidate();
        assert_eq!(candidate.members().len(), 2);
        assert_eq!(candidate.index_mutations().len(), 2);
        let publication = candidate
            .members()
            .iter()
            .find(|member| member.kind() == family)
            .unwrap()
            .record();
        let coordinate = publication.coordinate().clone();
        let canonical_bytes = publication.canonical_record_jcs().as_slice().to_vec();
        let (request, page) = derived_recovery_page(&prepared);
        let mut store =
            FakeEventStore::with_commit(CommitStep::New, EventCustodyReadBackOutcome::Predecessor)
                .with_derived_page(page);
        let mut completed = prepared.commit(&mut store, 2).expect("atomic custody");
        assert_eq!(
            completed.take_continuation().map(|value| value.kind()),
            Some(EventCustodyContinuationKind::ExposePaleDerivedPublication)
        );
        assert_eq!(completed.take_continuation(), None);
        let pending = read_pending_pale_derived_publications(&mut store, &request).unwrap();
        assert_eq!(pending.entries().len(), 1);
        let recovered = &pending.entries()[0];
        assert_eq!(recovered.publication_ref(), coordinate.record_ref());
        assert_eq!(
            recovered.publication_sha256().as_str(),
            coordinate.record_sha256().as_str()
        );
        assert_ne!(
            recovered.publication_sha256().as_str(),
            pale_core::sha256_hex(&canonical_bytes)
        );
    }
}

#[test]
fn publication_retention_cap_is_exact_for_each_typed_family() {
    for (index, prepared) in publications(16_384, "publication-outbox")
        .into_iter()
        .enumerate()
    {
        let prepared = prepared.unwrap();
        let bytes: usize = prepared
            .candidate()
            .members()
            .iter()
            .map(|member| member.record().canonical_record_jcs().as_slice().len())
            .sum();
        assert!(publications(bytes, "publication-outbox")
            .into_iter()
            .nth(index)
            .unwrap()
            .is_ok());
        assert!(matches!(
            publications(bytes - 1, "publication-outbox")
                .into_iter()
                .nth(index)
                .unwrap(),
            Err(EventCustodyProjectionError::PublicationBytes)
        ));
    }
    for rejected in publications(0, "publication-outbox") {
        assert!(matches!(
            rejected,
            Err(EventCustodyProjectionError::PublicationBytes)
        ));
    }
}

#[test]
fn publication_and_retention_cannot_share_an_immutable_reference() {
    for rejected in publications(16_384, "publication-record") {
        assert!(matches!(
            rejected,
            Err(EventCustodyProjectionError::Lineage)
        ));
    }
}

#[test]
fn mismatched_event_cannot_acquire_occurrence_publication_custody() {
    let original = event("original", "revision-1");
    let different = event("different", "revision-2");
    let event_ref = record_ref("event-record");
    let occurrence =
        admitted_occurrence("occurrence", &event_ref, original.canonical_event_sha256());
    assert!(matches!(
        prepare(EventCustodyTransitionInput::ProjectInteractionOccurrence {
            event: AdmittedEventRecord::new(&event_ref, &different),
            occurrence: AdmittedEventRecord::new(&record_ref("occurrence-record"), &occurrence),
            retention_record_ref: &record_ref("outbox-record"),
            maximum_publication_bytes: 16_384,
        }),
        Err(EventCustodyProjectionError::Lineage)
    ));
}

#[test]
fn derived_projection_point_reads_distinguish_new_exact_conflict_and_current() {
    let store_coordinate = authority_store();
    let source_event = event("classification-source", "revision-1");
    let destination_event = event("classification-destination", "revision-1");
    let source_event_ref = record_ref("classification-source-event");
    let destination_event_ref = record_ref("classification-destination-event");
    let occurrence_ref = record_ref("classification-occurrence");
    let destination_ref = record_ref("classification-destination-occurrence");
    let source = admitted_occurrence(
        "classification-source",
        &source_event_ref,
        source_event.canonical_event_sha256(),
    );
    let destination = admitted_occurrence(
        "classification-destination",
        &destination_event_ref,
        destination_event.canonical_event_sha256(),
    );
    let occurrence_input = EventCustodyTransitionInput::ProjectInteractionOccurrence {
        event: AdmittedEventRecord::new(&source_event_ref, &source_event),
        occurrence: AdmittedEventRecord::new(&occurrence_ref, &source),
        retention_record_ref: &record_ref("classification-occurrence-outbox"),
        maximum_publication_bytes: 16_384,
    };
    let prepared = prepare(occurrence_input).unwrap();
    let occurrence_record = prepared
        .candidate()
        .members()
        .iter()
        .find(|member| member.kind() == M::InteractionOccurrence)
        .unwrap()
        .record()
        .clone();
    let mut point_store =
        FakeEventStore::with_commit(CommitStep::New, EventCustodyReadBackOutcome::Predecessor);
    assert!(matches!(
        classify_interaction_occurrence(
            &mut point_store,
            &store_coordinate,
            AdmittedEventRecord::new(&occurrence_ref, &source),
            16 * 1024,
        )
        .unwrap(),
        DerivedEventRecordDisposition::New
    ));
    point_store.index_record = Some(occurrence_record.clone());
    assert!(matches!(
        classify_interaction_occurrence(
            &mut point_store,
            &store_coordinate,
            AdmittedEventRecord::new(&occurrence_ref, &source),
            16 * 1024,
        )
        .unwrap(),
        DerivedEventRecordDisposition::ExactReplay { .. }
    ));
    let unlike_source = admitted_occurrence(
        "classification-source-unlike",
        &source_event_ref,
        source_event.canonical_event_sha256(),
    );
    assert!(matches!(
        classify_interaction_occurrence(
            &mut point_store,
            &store_coordinate,
            AdmittedEventRecord::new(&occurrence_ref, &unlike_source),
            16 * 1024,
        )
        .unwrap(),
        DerivedEventRecordDisposition::Conflict { .. }
    ));

    let edge_ref = record_ref("classification-edge");
    let edge = admitted_edge(
        "classification-edge",
        &occurrence_ref,
        &source,
        &destination_ref,
        &destination,
    );
    let edge_prepared = prepare(EventCustodyTransitionInput::ProjectGenealogyEdge {
        source: AdmittedEventRecord::new(&occurrence_ref, &source),
        destination: AdmittedEventRecord::new(&destination_ref, &destination),
        edge: AdmittedEventRecord::new(&edge_ref, &edge),
        retention_record_ref: &record_ref("classification-edge-outbox"),
        maximum_publication_bytes: 16_384,
    })
    .unwrap();
    point_store.index_record = Some(
        edge_prepared
            .candidate()
            .members()
            .iter()
            .find(|member| member.kind() == M::GenealogyEdge)
            .unwrap()
            .record()
            .clone(),
    );
    assert!(matches!(
        classify_genealogy_edge(
            &mut point_store,
            &store_coordinate,
            AdmittedEventRecord::new(&edge_ref, &edge),
            16 * 1024,
        )
        .unwrap(),
        DerivedEventRecordDisposition::ExactReplay { .. }
    ));

    let frontier_ref = record_ref("classification-frontier");
    let pending = admitted_frontier(
        "classification-frontier-pending",
        "classification-subject",
        ReconciliationDimensionStatusV1::Pending,
    );
    let frontier_prepared = prepare(EventCustodyTransitionInput::AdvanceReconciliationFrontier {
        predecessor: None,
        successor: AdmittedEventRecord::new(&frontier_ref, &pending),
        retention_record_ref: &record_ref("classification-frontier-outbox"),
        maximum_publication_bytes: 16_384,
    })
    .unwrap();
    point_store.index_record = Some(
        frontier_prepared
            .candidate()
            .members()
            .iter()
            .find(|member| member.kind() == M::ReconciliationFrontier)
            .unwrap()
            .record()
            .clone(),
    );
    assert!(matches!(
        classify_reconciliation_frontier(
            &mut point_store,
            &store_coordinate,
            AdmittedEventRecord::new(&frontier_ref, &pending),
            16 * 1024,
        )
        .unwrap(),
        ReconciliationFrontierDisposition::ExactReplay { .. }
    ));
    let advanced = admitted_frontier(
        "classification-frontier-advanced",
        "classification-subject",
        ReconciliationDimensionStatusV1::Complete,
    );
    assert!(matches!(
        classify_reconciliation_frontier(
            &mut point_store,
            &store_coordinate,
            AdmittedEventRecord::new(&frontier_ref, &advanced),
            16 * 1024,
        )
        .unwrap(),
        ReconciliationFrontierDisposition::Current { .. }
    ));
}

#[test]
fn exact_atomic_retention_is_the_predecessor_of_settlement() {
    for prepared in publications(16_384, "publication-outbox") {
        let prepared = prepared.unwrap();
        let (request, page) = derived_recovery_page(&prepared);
        let mut store =
            FakeEventStore::with_commit(CommitStep::New, EventCustodyReadBackOutcome::Predecessor)
                .with_derived_page(page);
        prepared.commit(&mut store, 2).expect("publication commit");
        let pending = read_pending_pale_derived_publications(&mut store, &request).unwrap();
        let publication = &pending.entries()[0];
        let prior = PaleDerivedPublicationRetention::retained(
            publication.retention_record().record_ref().clone(),
            publication.publication_ref().clone(),
            publication.publication_sha256().clone(),
        );
        let next = PaleDerivedPublicationRetention::settled(
            record_ref("settled-outbox"),
            publication.publication_ref().clone(),
            publication.publication_sha256().clone(),
            record_ref("acknowledgement"),
            artifact('e'),
        );
        let settled = prepare(EventCustodyTransitionInput::SettlePaleDerivedPublication {
            predecessor: &prior,
            successor: &next,
        })
        .unwrap();
        let EventIndexPredecessor::Exact(expected) =
            settled.candidate().index_mutations()[0].expected()
        else {
            panic!("exact retained predecessor")
        };
        assert_eq!(expected.coordinate(), publication.retention_record());
        let (request, page) = derived_recovery_page(&settled);
        store.derived_page = Some(page);
        store.commits.push_back(CommitStep::New);
        let mut completed = settled.commit(&mut store, 2).unwrap();
        assert_eq!(completed.take_continuation(), None);
        assert!(read_pending_pale_derived_publications(&mut store, &request)
            .unwrap()
            .entries()
            .is_empty());
    }
}
