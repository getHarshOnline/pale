//! Actual finalization trees over a committed bootstrap, without finalization writes.

use super::*;
use crate::resident_fixture::{
    deployment_finalization_fixture::Fixture, principal_checkpoint_fixture as fixture,
};
use crate::SqlitePrincipalCheckpointPreparation;
use pale_application::resident::{
    AdmittedDeploymentFinalization, PreparedDeploymentFinalizationCheckpoint,
    SupportedPrincipalTrustCheckpointProfile,
};
use pale_core::records::{ControlStateCheckpointPreimageV1, ControlStateCheckpointV1};
use pale_ports::{
    AccumulatorPreparationLimits, AuthorityStoreCoordinate, ControlStoreError,
    ControlTransactionPort, ControlTransactionPreparation, OperationAccumulatorPreparation,
    OperationAccumulatorPreparationError, PreparedOperationAccumulator,
    PrincipalCheckpointPreparationPort, PrincipalTrustReadPort,
};

#[path = "finalization/predecessor.rs"]
mod predecessor;
use predecessor::Anchors;

struct RecordingPreparation<'a> {
    native: SqlitePrincipalCheckpointPreparation<'a>,
    request: Option<OperationAccumulatorPreparation>,
}

impl ControlTransactionPort for RecordingPreparation<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        self.native.prepare_transaction(store)
    }
}

impl PrincipalTrustReadPort for RecordingPreparation<'_> {
    fn read_record(
        &mut self,
        binding: &pale_ports::PrincipalTrustStoreBinding,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<pale_ports::ControlRecord>, ControlStoreError> {
        self.native
            .read_record(binding, coordinate, maximum_body_bytes)
    }

    fn read_current_checkpoint(
        &mut self,
        binding: &pale_ports::PrincipalTrustStoreBinding,
        maximum_body_bytes: usize,
    ) -> Result<Option<pale_ports::PrincipalTrustSnapshot>, ControlStoreError> {
        self.native
            .read_current_checkpoint(binding, maximum_body_bytes)
    }
}

impl PrincipalCheckpointPreparationPort for RecordingPreparation<'_> {
    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        self.request = Some(request.clone());
        self.native.prepare_accumulator(request)
    }
}

fn setup() -> (
    Connection,
    ControlStateCheckpointPreimageV1,
    fixture::BootstrapRecords,
    Fixture,
) {
    let mut connection = in_memory_store();
    let (bootstrap, control, records) = fixture::prepare_with_records(
        &mut SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        },
        |_| {},
    )
    .unwrap();
    let _committed = bootstrap
        .commit(&mut SqlitePrincipalBootstrapCustody {
            connection: &mut connection,
        })
        .unwrap();
    let finalization = Fixture::from_trust(
        &records.trust_ref,
        &records.trust,
        &records.signer_ref,
        &records.signer,
    );
    (connection, control, records, finalization)
}

fn prepare<'a, P: PrincipalCheckpointPreparationPort + PrincipalTrustReadPort + ?Sized>(
    finalization: &'a Fixture,
    prior_ref: &'a ResidentRecordRefV1,
    prior: &'a ControlStateCheckpointV1,
    port: &mut P,
) -> PreparedDeploymentFinalizationCheckpoint<'a> {
    let profile = SupportedPrincipalTrustCheckpointProfile::admit(
        prior.checkpoint_profile_ref().clone(),
        &SupportedPrincipalTrustCheckpointProfile::content_digest().unwrap(),
        SupportedPrincipalTrustCheckpointProfile::canonical_content()
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    profile
        .prepare_finalization(
            AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
            (prior_ref, prior),
            65_536,
            AccumulatorPreparationLimits::new(4, 4096, 4096, 4096, 262_144).unwrap(),
            port,
            &mut Anchors::new(finalization, prior_ref, prior),
        )
        .unwrap()
}

#[test]
fn repeated_native_preparation_has_stable_roots_and_no_database_writes() {
    let (mut connection, control, records, finalization) = setup();
    let prior = records.checkpoint(&control);
    let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
    let before = connection.total_changes();
    let mut port = RecordingPreparation {
        native: SqlitePrincipalCheckpointPreparation {
            connection: &mut connection,
        },
        request: None,
    };
    let first = prepare(&finalization, &prior_ref, &prior, &mut port);
    let request = port.request.clone().unwrap();
    let second = prepare(&finalization, &prior_ref, &prior, &mut port);
    assert_eq!(port.request.as_ref(), Some(&request));
    assert_eq!(first.state_root(), second.state_root());
    assert_ne!(first.state_root(), prior.state_root_sha256());
    assert_eq!(
        first.unresolved_root(),
        prior.unresolved_custody_root_sha256()
    );
    assert_eq!(first.leaf_count().get(), 5);
    assert_eq!(first.transaction_sequence().get(), 3);
    assert_eq!(first.changed_keys().len(), 4);
    let prepared_again = port.native.prepare_accumulator(&request).unwrap();
    assert_eq!(prepared_again.state_root(), first.state_root());
    let candidate = fixture::finalization_preimage(&first, &prior_ref, &prior, control.clone());
    let wire = crate::resident_fixture::deployment_finalization_fixture::changed_wire(
        &candidate,
        "canonical_control_checkpoint_sha256",
        |_| {},
    );
    let constructed = first
        .admit_checkpoint(
            ResidentRecordRefV1::parse("finalization-control-head").unwrap(),
            wire,
            &control.schema_sha256,
            &records.authority(&control),
            |_, _, _, _| Ok(()),
        )
        .unwrap();
    assert_eq!(
        constructed.accumulator().state().predecessor_root(),
        prior.state_root_sha256()
    );
    assert_eq!(
        constructed.accumulator().state().successor_root(),
        second.state_root()
    );
    assert!(constructed
        .accumulator()
        .state()
        .mutations()
        .iter()
        .any(|change| change.predecessor().is_some()));
    assert_eq!(connection.total_changes(), before);
    assert!(connection.is_autocommit(), "read snapshot must be released");
    let sequence: i64 = connection
        .query_row(
            "SELECT logical_transaction_sequence FROM pale_control_store_header WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(sequence, 2, "preparation does not reserve a transaction");
}

#[test]
fn stale_transaction_is_refused_without_reading_a_new_tree_as_the_old_one() {
    let (mut connection, control, records, finalization) = setup();
    let prior = records.checkpoint(&control);
    let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
    let request = {
        let mut port = RecordingPreparation {
            native: SqlitePrincipalCheckpointPreparation {
                connection: &mut connection,
            },
            request: None,
        };
        let _prepared = prepare(&finalization, &prior_ref, &prior, &mut port);
        port.request.unwrap()
    };
    connection.execute("UPDATE pale_control_store_header SET logical_transaction_sequence=logical_transaction_sequence+1 WHERE singleton=1", []).unwrap();
    let before = connection.total_changes();
    let result = SqlitePrincipalCheckpointPreparation {
        connection: &mut connection,
    }
    .prepare_accumulator(&request);
    assert!(
        matches!(result, Err(OperationAccumulatorPreparationError::Store(ref error)) if error.kind() == ControlStoreErrorKind::CandidateConflict)
    );
    assert_eq!(connection.total_changes(), before);
    assert!(connection.is_autocommit());
}

#[test]
fn corrupted_prior_tree_and_insufficient_work_budget_never_produce_roots() {
    let (mut connection, control, records, finalization) = setup();
    let prior = records.checkpoint(&control);
    let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
    let request = {
        let mut port = RecordingPreparation {
            native: SqlitePrincipalCheckpointPreparation {
                connection: &mut connection,
            },
            request: None,
        };
        let _prepared = prepare(&finalization, &prior_ref, &prior, &mut port);
        port.request.unwrap()
    };
    let limited = OperationAccumulatorPreparation::new(
        request.basis().clone(),
        request.state().clone(),
        request.unresolved().clone(),
        AccumulatorPreparationLimits::new(4, 1, 4096, 4096, 262_144).unwrap(),
        request.implementation(),
        request.transaction().clone(),
    )
    .unwrap();
    let before = connection.total_changes();
    let result = SqlitePrincipalCheckpointPreparation {
        connection: &mut connection,
    }
    .prepare_accumulator(&limited);
    assert!(matches!(
        result,
        Err(OperationAccumulatorPreparationError::Limit)
    ));
    assert_eq!(connection.total_changes(), before);
    connection
        .execute(
            "DELETE FROM pale_principal_control_accumulator_position",
            [],
        )
        .unwrap();
    let before = connection.total_changes();
    assert!(SqlitePrincipalCheckpointPreparation {
        connection: &mut connection
    }
    .prepare_accumulator(&request)
    .is_err());
    assert_eq!(connection.total_changes(), before);
    assert!(connection.is_autocommit());
}
