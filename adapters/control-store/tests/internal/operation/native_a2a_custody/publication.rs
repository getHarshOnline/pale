//! Physical source discovery over actual contextual A2A custody. No Event is
//! constructed or published; remote lifecycle meaning stays with its owner.

use super::*;

fn read(
    port: &mut SqliteOperationCustody<'_>,
    cursor: Option<&OperationPublicationCursor>,
    count: usize,
    bytes: usize,
) -> Result<OperationPublicationSources, OperationPublicationReadError> {
    let coordinate = port.coordinate.clone();
    let writes = port.connection.total_changes();
    let result = port.read_publication_sources(&coordinate, cursor, count, bytes);
    assert_eq!(
        port.connection.total_changes(),
        writes,
        "discovery is read-only"
    );
    result
}

pub(super) fn all(port: &mut SqliteOperationCustody<'_>) -> Vec<OperationPublicationSource> {
    let page = read(
        port,
        None,
        MAX_OPERATION_PUBLICATION_SOURCES,
        MAX_OPERATION_PUBLICATION_BODY_BYTES,
    )
    .unwrap();
    assert!(page.continuation.is_none(), "bounded fixture fits one page");
    assert!(page.sources.windows(2).all(|pair| (
        pair[0].transaction().get(),
        pair[0].observation().coordinate().record_ref().as_str()
    ) < (
        pair[1].transaction().get(),
        pair[1].observation().coordinate().record_ref().as_str()
    )));
    page.sources
}

pub(super) fn assert_candidate(
    port: &mut SqliteOperationCustody<'_>,
    candidate: &OperationCustodyCandidate,
) {
    let expected = candidate
        .record(OperationCustodyRecordKind::A2aTaskObservation)
        .unwrap();
    let page = all(port);
    let matching: Vec<_> = page
        .iter()
        .filter(|source| source.observation().coordinate() == expected.coordinate())
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "replay cannot duplicate an immutable source"
    );
    let source = matching[0];
    assert_eq!(
        source.kind(),
        OperationPublicationSourceKind::A2aTaskObservation
    );
    assert_eq!(source.observation(), expected);
    assert_eq!(
        source.checkpoint(),
        candidate.checkpoint().successor_record()
    );
    assert_eq!(
        source.transaction(),
        candidate
            .checkpoint()
            .successor()
            .committed_transaction_sequence()
    );
    assert!(page
        .iter()
        .any(|source| source.kind() == OperationPublicationSourceKind::OperationObservation));
}

pub(super) struct Prefix {
    sources: Vec<OperationPublicationSource>,
    cursor: OperationPublicationCursor,
}

impl Prefix {
    pub(super) fn capture(port: &mut SqliteOperationCustody<'_>) -> Self {
        let sources = all(port);
        assert!(sources
            .iter()
            .any(|source| source.kind() == OperationPublicationSourceKind::A2aTaskObservation));
        let first = read(port, None, 1, MAX_OPERATION_PUBLICATION_BODY_BYTES).unwrap();
        assert_eq!(first.sources, sources[..1]);
        Self {
            sources,
            cursor: first.continuation.unwrap(),
        }
    }

    pub(super) fn verify(self, port: &mut SqliteOperationCustody<'_>) {
        assert!(
            all(port).len() > self.sources.len(),
            "both families gained later revisions"
        );
        let first = self.sources[0].observation();
        // A continuation must seek past already returned bodies, even if those
        // bytes would exceed every legal page budget on a fresh traversal.
        port.connection.execute(
            "UPDATE pale_deployment_control_record SET canonical_record_jcs=zeroblob(?1) WHERE record_ref=?2",
            rusqlite::params![i64::try_from(MAX_OPERATION_PUBLICATION_BODY_BYTES + 1).unwrap(), first.coordinate().record_ref().as_str()],
        ).unwrap();
        assert!(matches!(
            read(port, None, 1, MAX_OPERATION_PUBLICATION_BODY_BYTES),
            Err(OperationPublicationReadError::Limit)
        ));
        let mut remaining = Vec::new();
        let mut cursor = Some(self.cursor.clone());
        for _ in 0..self.sources.len() {
            let page = read(
                port,
                cursor.as_ref(),
                1,
                MAX_OPERATION_PUBLICATION_BODY_BYTES,
            )
            .unwrap();
            if let Some(next) = &page.continuation {
                assert_eq!(next.through(), self.cursor.through());
            }
            remaining.extend(page.sources);
            cursor = page.continuation;
            if cursor.is_none() {
                break;
            }
        }
        assert!(cursor.is_none(), "bounded continuation makes progress");
        assert_eq!(
            remaining,
            self.sources[1..],
            "captured prefix excludes later local and remote commits"
        );
        port.connection.execute(
            "UPDATE pale_deployment_control_record SET canonical_record_jcs=?1 WHERE record_ref=?2",
            rusqlite::params![first.canonical_record_jcs().as_slice(), first.coordinate().record_ref().as_str()],
        ).unwrap();
    }
}

pub(super) fn pages_and_bounds(port: &mut SqliteOperationCustody<'_>) {
    let expected = all(port);
    let bytes: usize = expected
        .iter()
        .map(|source| {
            source.observation().canonical_record_jcs().as_slice().len()
                + source.checkpoint().canonical_record_jcs().as_slice().len()
        })
        .sum();
    assert_eq!(
        read(port, None, expected.len(), bytes).unwrap().sources,
        expected
    );
    assert!(matches!(
        read(port, None, expected.len(), bytes - 1),
        Err(OperationPublicationReadError::Limit)
    ));
    for count in [1, 2, 3] {
        let mut found = Vec::new();
        let mut cursor = None;
        for _ in 0..expected.len() {
            let page = read(port, cursor.as_ref(), count, bytes).unwrap();
            assert!(page.sources.len() <= count);
            found.extend(page.sources);
            cursor = page.continuation;
            if cursor.is_none() {
                break;
            }
        }
        assert!(cursor.is_none());
        assert_eq!(found, expected, "mixed source pagination count={count}");
    }
}

pub(super) fn corrupt_checkpoint(
    port: &mut SqliteOperationCustody<'_>,
    candidate: &OperationCustodyCandidate,
) {
    let record = candidate
        .record(OperationCustodyRecordKind::A2aTaskObservation)
        .unwrap();
    let key = OperationCustodyIndexKey::for_observation(record.coordinate());
    let checkpoint = candidate.checkpoint().successor_record();
    let sources = all(port);
    let earlier = sources
        .iter()
        .find(|source| {
            source.kind() == OperationPublicationSourceKind::OperationObservation
                && source.transaction().get()
                    < candidate
                        .checkpoint()
                        .successor()
                        .committed_transaction_sequence()
                        .get()
        })
        .unwrap()
        .checkpoint();
    for (reference, digest) in [
        (
            earlier.coordinate().record_ref(),
            earlier.coordinate().record_sha256(),
        ),
        (
            record.coordinate().record_ref(),
            record.coordinate().record_sha256(),
        ),
    ] {
        port.connection.execute(
            "UPDATE pale_deployment_operation_index SET record_ref=?1, record_sha256=?2 WHERE index_role='a2a_task_observation_checkpoint' AND index_key=?3",
            rusqlite::params![reference.as_str(), digest.as_str(), key.as_str()],
        ).unwrap();
        assert!(
            matches!(read(port, None, MAX_OPERATION_PUBLICATION_SOURCES, MAX_OPERATION_PUBLICATION_BODY_BYTES),
            Err(OperationPublicationReadError::Storage(error)) if error.kind() == ControlStoreErrorKind::RecoveryRequired)
        );
    }
    port.connection.execute("DELETE FROM pale_deployment_operation_index WHERE index_role='a2a_task_observation_checkpoint' AND index_key=?1", [key.as_str()]).unwrap();
    port.connection.execute(
        "INSERT INTO pale_deployment_operation_index(index_role,index_key,record_ref,record_sha256) VALUES ('observation_checkpoint',?1,?2,?3)",
        rusqlite::params![key.as_str(), checkpoint.coordinate().record_ref().as_str(), checkpoint.coordinate().record_sha256().as_str()],
    ).unwrap();
    assert!(
        matches!(read(port, None, MAX_OPERATION_PUBLICATION_SOURCES, MAX_OPERATION_PUBLICATION_BODY_BYTES),
        Err(OperationPublicationReadError::Storage(error)) if error.kind() == ControlStoreErrorKind::RecoveryRequired)
    );
    port.connection.execute("DELETE FROM pale_deployment_operation_index WHERE index_role='observation_checkpoint' AND index_key=?1", [key.as_str()]).unwrap();
    port.connection.execute(
        "INSERT INTO pale_deployment_operation_index(index_role,index_key,record_ref,record_sha256) VALUES ('a2a_task_observation_checkpoint',?1,?2,?3)",
        rusqlite::params![key.as_str(), checkpoint.coordinate().record_ref().as_str(), checkpoint.coordinate().record_sha256().as_str()],
    ).unwrap();
    assert_eq!(
        all(port),
        sources,
        "restore exact owned fixture after committed corruption"
    );
}
