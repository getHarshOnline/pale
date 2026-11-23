//! The borrowed operation view uses the existing unadmitted deployment store.
//! Start and checkpoint preparation use actual application/native owners;
//! signature, authority and independent-anchor fixtures remain declared models.
//! Physical read-back below is not authenticated attachment or continuation.

use crate::authority::DeploymentControlSqliteStore;
use crate::location::{ControlStorageRoot, ControlStoreScope, DeploymentStoreScope};
use crate::resident_fixture::local_operation_start_fixture::StartAdmission;
use crate::resident_fixture::local_operation_start_fixture::{
    checkpoint_authority, checkpoint_preimage, record_ref, signature, store_coordinate, with_start,
};
use crate::store::{initialize_unadmitted_store, ControlStoreIdentity};
use crate::test_support::{self as support, RetainedAntiRollbackAnchors};
use pale_application::resident::{
    read_operation_start_for_request, retain_operation_lease_body,
    SupportedOperationCheckpointProfile,
};
use pale_core::records::*;
use pale_ports::{
    AccumulatorPreparationLimits, AuthorityStoreCoordinate, BoundedAuthorityEntryBytes,
    ControlRecordCoordinate, ControlRecordDigest, ControlStoreErrorKind, ControlTransactionPort,
    DurableAuthorityStorePort, OperationCustodyCommitOutcome, OperationCustodyPort,
    OperationCustodyReadBackOutcome, OperationCustodyRecord, OperationCustodyRecordKind,
    OperationPublicationReadError, PreparedAuthorityAppend, SequencedAuthorityEntry,
};

use super::local_recovery::with_generation;
use super::SqliteOperationCustody;

fn append_journal(
    store: &DeploymentControlSqliteStore,
    coordinate: &AuthorityStoreCoordinate,
    previous: Option<JournalSequenceV1>,
    sequence: u64,
) {
    // This opaque authority frame tests physical transaction ordering only.
    let append = PreparedAuthorityAppend::new(
        coordinate.clone(),
        previous,
        SequencedAuthorityEntry::new(
            JournalSequenceV1::new(sequence).unwrap(),
            BoundedAuthorityEntryBytes::new(b"view-ordering".to_vec(), 64).unwrap(),
        ),
    )
    .unwrap();
    store.compare_and_append(&append).unwrap();
}

#[test]
fn borrowed_operation_view_shares_transaction_owner_and_reopens_exact_start() {
    with_generation(|generation, effective| {
        with_start(
            generation,
            effective,
            "borrowed-store-view",
            |_| {},
            StartAdmission::AcceptedOwnerModel,
            |prepared, request, owner_calls, record_signatures, retained| {
                assert_eq!((owner_calls, record_signatures), (1, 2));
                let prepared = prepared.unwrap();
                let directory = support::private_tempdir();
                let binding = retained.current.deployment_binding;
                let scope = ControlStoreScope::Deployment(DeploymentStoreScope::new(
                    ResidentWorkspaceIdV1::parse(
                        binding.deployment_terms().workspace_id().as_str(),
                    )
                    .unwrap(),
                    retained.current.deployment_binding_ref.clone(),
                    ArtifactDigest::parse(binding.canonical_binding_sha256().as_str()).unwrap(),
                ));
                let location = ControlStorageRoot::retain(directory.path())
                    .unwrap()
                    .prepare_store_location(scope.clone())
                    .unwrap();
                let coordinate = store_coordinate();
                initialize_unadmitted_store(
                    &location,
                    ControlStoreIdentity::new(
                        scope,
                        coordinate.store_id().clone(),
                        coordinate.incarnation_id().clone(),
                    ),
                )
                .unwrap();
                let store = DeploymentControlSqliteStore::open_unadmitted_candidate(
                    &location,
                    coordinate.clone(),
                )
                .unwrap();
                let mut port = &store;
                let initial = port.prepare_transaction(&coordinate).unwrap();
                let before = store.record_custody_connection().unwrap().total_changes();
                retain_operation_lease_body(
                    &mut port,
                    &coordinate,
                    &record_ref("start-lease"),
                    retained.lease,
                    1 << 20,
                )
                .unwrap();
                assert_eq!(
                    port.prepare_transaction(&coordinate).unwrap().expected(),
                    initial.successor()
                );
                assert!(
                    store.record_custody_connection().unwrap().total_changes() > before,
                    "operation writes occurred on the existing connection, not another opener"
                );

                let before_journal = port.prepare_transaction(&coordinate).unwrap();
                append_journal(&store, &coordinate, None, 1);
                let after_journal = port.prepare_transaction(&coordinate).unwrap();
                assert_eq!(after_journal.expected(), before_journal.successor());
                let prepared = prepared
                    .prepare_checkpoint(
                        AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000)
                            .unwrap(),
                        &mut port,
                    )
                    .unwrap();
                assert_eq!(prepared.transaction_sequence(), after_journal.successor());
                let mut checkpoint = checkpoint_preimage(1, None);
                checkpoint.committed_transaction_sequence = prepared.transaction_sequence();
                checkpoint.issued_at = UnixTimeV1::new(61).unwrap();
                checkpoint.checkpoint_profile_sha256 =
                    SupportedOperationCheckpointProfile::content_digest().unwrap();
                checkpoint.checkpoint_domain_id =
                    SupportedOperationCheckpointProfile::conflict_domain(request).unwrap();
                checkpoint.state_root_sha256 = prepared.state_root().clone();
                checkpoint.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
                checkpoint.changed_leaf_keys = prepared.changed_keys().to_vec();
                checkpoint.leaf_count = ControlLeafCountV1::new(4).unwrap();
                let (physical, pending) = prepared
                    .issue_checkpoint(
                        &record_ref("operation/checkpoint-1"),
                        checkpoint.clone(),
                        &checkpoint.schema_sha256,
                        &checkpoint_authority(&checkpoint),
                        |domain, _, _| {
                            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                            Ok(signature())
                        },
                    )
                    .unwrap();
                let mut anchor = RetainedAntiRollbackAnchors::default();
                let database = physical.commit(&mut port, &mut anchor, 1 << 20).unwrap();
                let candidate = database.candidate().clone();
                let issued = pending
                    .finish(database.advance_anchor(&mut anchor).unwrap())
                    .unwrap_or_else(|_| panic!("exact committed and anchored start"));
                assert_eq!(
                    issued.handle().0,
                    candidate
                        .record(OperationCustodyRecordKind::OperationHandle)
                        .unwrap()
                        .coordinate()
                        .record_ref()
                );
                assert_eq!(
                    port.prepare_transaction(&coordinate).unwrap().expected(),
                    checkpoint.committed_transaction_sequence
                );
                let before_reads = store.record_custody_connection().unwrap().total_changes();
                assert_eq!(
                    port.compare_and_commit(&candidate).unwrap(),
                    OperationCustodyCommitOutcome::ExactReplay
                );
                assert_eq!(
                    port.observe_exact(&candidate).unwrap(),
                    OperationCustodyReadBackOutcome::ExactSuccessor
                );
                for record in candidate
                    .records()
                    .iter()
                    .map(OperationCustodyRecord::record)
                {
                    let borrowed = port
                        .read_operation_record(&coordinate, record.coordinate(), 1 << 20)
                        .unwrap();
                    assert_eq!(borrowed.as_ref(), Some(record));
                    let mut connection = store.record_custody_connection().unwrap();
                    let direct = SqliteOperationCustody {
                        connection: &mut connection,
                        coordinate: coordinate.clone(),
                    }
                    .read_operation_record(&coordinate, record.coordinate(), 1 << 20)
                    .unwrap();
                    assert_eq!(borrowed, direct);
                    assert_eq!(
                        crate::store::transaction::record_stamp(
                            &connection,
                            crate::ControlStoreKind::DeploymentControl,
                            record.coordinate()
                        )
                        .unwrap(),
                        Some(checkpoint.committed_transaction_sequence)
                    );
                }
                assert_eq!(
                    store.record_custody_connection().unwrap().total_changes(),
                    before_reads
                );
                let before_second_journal = port.prepare_transaction(&coordinate).unwrap();
                append_journal(
                    &store,
                    &coordinate,
                    Some(JournalSequenceV1::new(1).unwrap()),
                    2,
                );
                assert_eq!(
                    port.prepare_transaction(&coordinate).unwrap().expected(),
                    before_second_journal.successor()
                );
                drop(issued);
                drop(store);

                let reopened = DeploymentControlSqliteStore::open_unadmitted_candidate(
                    &location,
                    coordinate.clone(),
                )
                .unwrap();
                let mut port = &reopened;
                let before_reads = reopened
                    .record_custody_connection()
                    .unwrap()
                    .total_changes();
                assert_eq!(
                    port.observe_exact(&candidate).unwrap(),
                    OperationCustodyReadBackOutcome::ExactSuccessor
                );
                let evidence =
                    read_operation_start_for_request(&mut port, &coordinate, request, 1 << 20)
                        .unwrap()
                        .unwrap();
                assert_eq!(
                    evidence.request(),
                    candidate
                        .record(OperationCustodyRecordKind::CommonRequest)
                        .unwrap()
                );
                assert_eq!(
                    evidence.handle(),
                    candidate
                        .record(OperationCustodyRecordKind::OperationHandle)
                        .unwrap()
                );
                assert_eq!(
                    evidence.initial_observation(),
                    candidate
                        .record(OperationCustodyRecordKind::OperationObservation)
                        .unwrap()
                );
                assert_eq!(
                    evidence.checkpoint(),
                    candidate.checkpoint().successor_record()
                );
                assert_eq!(
                    port.read_checkpoint_head(
                        &coordinate,
                        &checkpoint.checkpoint_domain_id,
                        1 << 20
                    )
                    .unwrap(),
                    Some((
                        candidate.checkpoint().successor().clone(),
                        candidate.checkpoint().successor_record().clone()
                    ))
                );
                let page = port
                    .read_publication_sources(&coordinate, None, 2, 1 << 20)
                    .unwrap();
                assert_eq!(page.sources.len(), 1);
                assert_eq!(
                    page.sources[0].observation(),
                    evidence.initial_observation()
                );
                assert_eq!(page.sources[0].checkpoint(), evidence.checkpoint());
                assert_eq!(
                    page.sources[0].transaction(),
                    checkpoint.committed_transaction_sequence
                );
                assert!(page.continuation.is_none());
                assert_eq!(
                    reopened.read_journal_head(&coordinate).unwrap(),
                    Some(JournalSequenceV1::new(2).unwrap())
                );
                assert_eq!(
                    reopened
                        .record_custody_connection()
                        .unwrap()
                        .total_changes(),
                    before_reads,
                    "recovery and source discovery perform no writes"
                );
            },
        );
    });
}

#[test]
fn borrowed_operation_view_rejects_foreign_store_and_incarnation() {
    let directory = support::private_tempdir();
    let (location, identity, _) = support::initialize(
        &directory,
        crate::ControlStoreKind::DeploymentControl,
        "operation-view-binding",
    );
    let coordinate = AuthorityStoreCoordinate::new(
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    );
    let store =
        DeploymentControlSqliteStore::open_unadmitted_candidate(&location, coordinate.clone())
            .unwrap();
    let mut port = &store;
    let record = ControlRecordCoordinate::new(
        record_ref("absent"),
        ControlRecordDigest::parse(ArtifactDigest::hash_bytes(b"absent").as_str()).unwrap(),
    );
    let before = store.record_custody_connection().unwrap().total_changes();
    for foreign in [
        AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("foreign-store").unwrap(),
            coordinate.incarnation_id().clone(),
        ),
        AuthorityStoreCoordinate::new(
            coordinate.store_id().clone(),
            ControlIncarnationIdV1::parse("foreign-incarnation").unwrap(),
        ),
    ] {
        assert_eq!(
            port.prepare_transaction(&foreign).unwrap_err().kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(
            port.read_operation_record(&foreign, &record, 1024)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert!(
            matches!(port.read_publication_sources(&foreign, None, 1, 1024), Err(OperationPublicationReadError::Storage(error)) if error.kind() == ControlStoreErrorKind::RecoveryRequired)
        );
    }
    assert!(port
        .read_operation_record(&coordinate, &record, 1024)
        .unwrap()
        .is_none());
    assert_eq!(
        store.record_custody_connection().unwrap().total_changes(),
        before
    );
}
