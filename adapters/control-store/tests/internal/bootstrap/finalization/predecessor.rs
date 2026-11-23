//! Real trust reads and tree preparation with controlled, read-only anchor responses.

use super::*;
use pale_application::resident::PrincipalCheckpointError;
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorKey, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    ControlDomainAntiRollbackAnchor, ControlRecordCoordinate, ControlRecordDigest,
    PreparedAntiRollbackAnchorTransition, PrincipalTrustMinimumAnchor,
};

pub(super) struct Anchors {
    values: [AntiRollbackAnchor; 2],
    reads: usize,
    missing_at: Option<usize>,
}

impl Anchors {
    pub(super) fn new(
        finalization: &Fixture,
        prior_ref: &ResidentRecordRefV1,
        prior: &ControlStateCheckpointV1,
    ) -> Self {
        let trust = &finalization.prior;
        Self {
            values: [
                PrincipalTrustMinimumAnchor::new(
                    trust.principal_id().clone(),
                    *trust.checkpoint_epoch(),
                    ControlRecordCoordinate::new(
                        finalization.prior_ref.clone(),
                        ControlRecordDigest::parse(
                            trust.canonical_trust_checkpoint_sha256().as_str(),
                        )
                        .unwrap(),
                    ),
                )
                .into(),
                ControlDomainAntiRollbackAnchor::new(
                    prior.control_store_id().clone(),
                    prior.control_incarnation_id().clone(),
                    prior.checkpoint_domain_id().clone(),
                    *prior.domain_epoch(),
                    *prior.domain_sequence(),
                    ControlRecordCoordinate::new(
                        prior_ref.clone(),
                        ControlRecordDigest::parse(
                            prior.canonical_control_checkpoint_sha256().as_str(),
                        )
                        .unwrap(),
                    ),
                )
                .into(),
            ],
            reads: 0,
            missing_at: None,
        }
    }
}

impl AntiRollbackAnchorPort for Anchors {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        let position = self.reads;
        self.reads += 1;
        assert!(position < 4, "bounded before/after checks");
        assert_eq!(key, &self.values[position % 2].key());
        Ok((self.missing_at != Some(position)).then(|| self.values[position % 2].clone()))
    }
    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("read-only preparation must not advance an anchor")
    }
    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("preparation does not propose an anchor transition")
    }
}

fn selected_profile(prior: &ControlStateCheckpointV1) -> SupportedPrincipalTrustCheckpointProfile {
    SupportedPrincipalTrustCheckpointProfile::admit(
        prior.checkpoint_profile_ref().clone(),
        &SupportedPrincipalTrustCheckpointProfile::content_digest().unwrap(),
        SupportedPrincipalTrustCheckpointProfile::canonical_content()
            .unwrap()
            .as_bytes(),
    )
    .unwrap()
}

#[test]
fn missing_anchor_before_or_after_real_tree_preparation_never_returns_a_candidate() {
    for missing_at in 0..4 {
        let (mut connection, control, records, finalization) = setup();
        let prior = records.checkpoint(&control);
        let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
        let mut anchors = Anchors::new(&finalization, &prior_ref, &prior);
        anchors.missing_at = Some(missing_at);
        let before = connection.total_changes();
        let mut port = RecordingPreparation {
            native: SqlitePrincipalCheckpointPreparation {
                connection: &mut connection,
            },
            request: None,
        };
        let result = selected_profile(&prior).prepare_finalization(
            AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
            (&prior_ref, &prior),
            65_536,
            AccumulatorPreparationLimits::new(4, 4096, 4096, 4096, 262_144).unwrap(),
            &mut port,
            &mut anchors,
        );
        if missing_at % 2 == 0 {
            assert!(matches!(
                result,
                Err(PrincipalCheckpointError::TrustMinimum)
            ));
        } else {
            assert!(matches!(
                result,
                Err(PrincipalCheckpointError::ControlAnchor)
            ));
        }
        assert_eq!(port.request.is_some(), missing_at >= 2);
        assert_eq!(anchors.reads, missing_at + 1);
        assert_eq!(connection.total_changes(), before);
        assert!(connection.is_autocommit());
    }
}

#[test]
fn altered_physical_trust_cannot_be_promoted_by_matching_independent_anchors() {
    for fault in 0..4 {
        let (mut connection, control, records, finalization) = setup();
        let prior = records.checkpoint(&control);
        let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
        match fault {
            0 => {
                connection
                    .execute("DELETE FROM pale_principal_trust_checkpoint_head", [])
                    .unwrap();
            }
            1 => {
                connection.execute(
                    "UPDATE pale_principal_trust_record SET canonical_record_jcs=?1 WHERE record_ref=?2",
                    rusqlite::params![b"{}".as_slice(), records.trust_ref.as_str()],
                ).unwrap();
            }
            2 => {
                connection.execute("UPDATE pale_principal_trust_checkpoint_head SET checkpoint_epoch=checkpoint_epoch+1", []).unwrap();
            }
            _ => {
                connection.execute(
                    "UPDATE pale_principal_trust_record SET schema_sha256=?1 WHERE record_ref=?2",
                    rusqlite::params!["f".repeat(64), records.trust_ref.as_str()],
                ).unwrap();
            }
        }
        let before = connection.total_changes();
        let mut anchors = Anchors::new(&finalization, &prior_ref, &prior);
        let mut port = RecordingPreparation {
            native: SqlitePrincipalCheckpointPreparation {
                connection: &mut connection,
            },
            request: None,
        };
        let result = selected_profile(&prior).prepare_finalization(
            AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
            (&prior_ref, &prior),
            65_536,
            AccumulatorPreparationLimits::new(4, 4096, 4096, 4096, 262_144).unwrap(),
            &mut port,
            &mut anchors,
        );
        assert!(
            matches!(result, Err(PrincipalCheckpointError::CurrentTrust)),
            "fault {fault}"
        );
        assert!(port.request.is_none());
        assert_eq!(anchors.reads, 0);
        assert_eq!(connection.total_changes(), before);
        assert!(connection.is_autocommit());
    }
}

struct ChangedAfterPreparation<'a>(SqlitePrincipalCheckpointPreparation<'a>);

impl ControlTransactionPort for ChangedAfterPreparation<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        self.0.prepare_transaction(store)
    }
}

impl PrincipalTrustReadPort for ChangedAfterPreparation<'_> {
    fn read_record(
        &mut self,
        binding: &pale_ports::PrincipalTrustStoreBinding,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<pale_ports::ControlRecord>, ControlStoreError> {
        self.0.read_record(binding, coordinate, maximum_body_bytes)
    }

    fn read_current_checkpoint(
        &mut self,
        binding: &pale_ports::PrincipalTrustStoreBinding,
        maximum_body_bytes: usize,
    ) -> Result<Option<pale_ports::PrincipalTrustSnapshot>, ControlStoreError> {
        self.0.read_current_checkpoint(binding, maximum_body_bytes)
    }
}

impl PrincipalCheckpointPreparationPort for ChangedAfterPreparation<'_> {
    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        let prepared = self.0.prepare_accumulator(request)?;
        // Simulate an intervening change after the real read transaction ended.
        // This is controlled interleaving, not concurrent-writer qualification.
        assert!(self.0.connection.is_autocommit());
        self.0.connection.execute(
            "UPDATE pale_principal_trust_checkpoint_head SET checkpoint_epoch=checkpoint_epoch+1", [],
        ).unwrap();
        Ok(prepared)
    }
}

#[test]
fn trust_change_after_released_native_snapshot_is_reported_not_reserved_away() {
    let (mut connection, control, records, finalization) = setup();
    let prior = records.checkpoint(&control);
    let prior_ref = ResidentRecordRefV1::parse("source-principal-control-head").unwrap();
    let before = connection.total_changes();
    let mut anchors = Anchors::new(&finalization, &prior_ref, &prior);
    let result = selected_profile(&prior).prepare_finalization(
        AdmittedDeploymentFinalization::admit(finalization.candidate()).unwrap(),
        (&prior_ref, &prior),
        65_536,
        AccumulatorPreparationLimits::new(4, 4096, 4096, 4096, 262_144).unwrap(),
        &mut ChangedAfterPreparation(SqlitePrincipalCheckpointPreparation {
            connection: &mut connection,
        }),
        &mut anchors,
    );
    assert!(matches!(
        result,
        Err(PrincipalCheckpointError::CurrentTrust)
    ));
    assert_eq!(anchors.reads, 2);
    assert_eq!(
        connection.total_changes(),
        before + 1,
        "only the explicitly simulated change writes"
    );
    assert!(connection.is_autocommit());
}
