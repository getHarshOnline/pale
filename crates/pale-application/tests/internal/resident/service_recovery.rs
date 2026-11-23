//! Physical resident-service trust recovery tests.

use super::*;

impl ResidentServiceTrustFrontier {
    pub(in crate::resident) fn from_recovered_fixture(
        frontier: RecoveredCurrentPrincipalTrustFrontier,
    ) -> Self {
        Self(frontier)
    }
}
use crate::{principal_trust_test_support, resident::record_projection};
use pale_core::records::*;
use pale_ports::*;
use std::{cell::RefCell, rc::Rc};

struct Fixture {
    bootstrap: AdmittedPrincipalBootstrapV1,
    checkpoint_schema: SchemaDigest,
    checkpoint_snapshot: PrincipalTrustCheckpointSetSnapshot,
    binding: PrincipalTrustStoreBinding,
    root_key: Ed25519PublicKeyV1,
}

fn fixture() -> Fixture {
    let (fixture, _) = principal_trust_test_support::admitted_bootstrap_fixture();
    let view = fixture.admitted.atom_view();
    let (checkpoint_ref, checkpoint) = view.trust_checkpoint_entry();
    let coordinate = record_projection::coordinate(
        checkpoint_ref,
        checkpoint.canonical_trust_checkpoint_sha256().as_str(),
    )
    .expect("checkpoint coordinate");
    let checkpoint_record =
        record_projection::record(coordinate.clone(), checkpoint).expect("checkpoint record");
    let binding = PrincipalTrustStoreBinding::new(
        checkpoint.trust_store_id().clone(),
        checkpoint.trust_store_incarnation_id().clone(),
        checkpoint.principal_id().clone(),
    );
    let checkpoint_snapshot = PrincipalTrustCheckpointSetSnapshot::new(
        binding.clone(),
        PrincipalTrustCheckpointHead::new(
            checkpoint.principal_id().clone(),
            *checkpoint.checkpoint_epoch(),
            coordinate,
        ),
        ControlTransactionSequenceV1::new(1).expect("transaction"),
        vec![PrincipalTrustCheckpointRecord::new(
            ControlTransactionSequenceV1::new(1).expect("transaction"),
            checkpoint_record,
        )]
        .into_boxed_slice(),
    )
    .expect("checkpoint snapshot");
    let root_key = view
        .authorities()
        .genesis()
        .principal_record()
        .root_public_key()
        .clone();
    Fixture {
        bootstrap: fixture.admitted,
        checkpoint_schema: fixture.trust_preimage.schema_sha256,
        checkpoint_snapshot,
        binding,
        root_key,
    }
}

struct CheckpointReader {
    expected_binding: PrincipalTrustStoreBinding,
    expected_limits: PrincipalTrustCheckpointSetReadLimits,
    snapshot: Option<PrincipalTrustCheckpointSetSnapshot>,
    calls: Rc<RefCell<Vec<&'static str>>>,
}

impl PrincipalTrustCheckpointSetReadPort for CheckpointReader {
    fn read_checkpoint_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: PrincipalTrustCheckpointSetReadLimits,
    ) -> Result<Option<PrincipalTrustCheckpointSetSnapshot>, ControlStoreError> {
        assert_eq!(binding, &self.expected_binding);
        assert_eq!(limits, self.expected_limits);
        self.calls.borrow_mut().push("checkpoints");
        Ok(self.snapshot.take())
    }
}

struct EmptyProducerReader {
    expected_binding: PrincipalTrustStoreBinding,
    expected_page_items: usize,
    calls: Rc<RefCell<Vec<&'static str>>>,
}

impl PrincipalProducerIncarnationReadPort for EmptyProducerReader {
    fn read_by_epoch(
        &mut self,
        _: &PrincipalTrustStoreBinding,
        _: &ProducerEpochV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        panic!("empty recovery uses head pages")
    }

    fn read_head(
        &mut self,
        _: &PrincipalTrustStoreBinding,
        _: &ProducerIdV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        panic!("empty recovery uses head pages")
    }

    fn read_by_allocation(
        &mut self,
        _: &PrincipalTrustSnapshot,
        _: &ControlRecordCoordinate,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        panic!("empty recovery has no allocation lineage")
    }

    fn read_heads_page(
        &mut self,
        request: &PrincipalProducerIncarnationHeadPageRead,
    ) -> Result<PrincipalProducerIncarnationHeadPage, ControlStoreError> {
        assert_eq!(request.binding(), &self.expected_binding);
        assert_eq!(request.maximum_items(), self.expected_page_items);
        assert!(request.after().is_none());
        self.calls.borrow_mut().push("producers");
        Ok(PrincipalProducerIncarnationHeadPage::new(
            request,
            vec![],
            PrincipalProducerIncarnationHeadPageBoundary::Complete,
        )
        .expect("empty producer page"))
    }
}

struct FinalizationReader {
    expected_binding: PrincipalTrustStoreBinding,
    expected_limits: DeploymentFinalizationSetReadLimits,
    calls: Rc<RefCell<Vec<&'static str>>>,
}

impl DeploymentFinalizationSetReadPort for FinalizationReader {
    fn read_finalization_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: DeploymentFinalizationSetReadLimits,
    ) -> Result<Option<DeploymentFinalizationSetSnapshot>, ControlStoreError> {
        assert_eq!(binding, &self.expected_binding);
        assert_eq!(limits, self.expected_limits);
        self.calls.borrow_mut().push("finalizations");
        Ok(None)
    }
}

struct NoProducerRecords;

impl PrincipalProducerIncarnationRecoveryAdmission for NoProducerRecords {
    fn admit_allocation(
        &mut self,
        _: ProducerIncarnationWireV1,
        _: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationV1, InvalidResidentRecordV1> {
        panic!("empty producer frontier has no allocation")
    }

    fn admit_retirement(
        &mut self,
        _: ProducerIncarnationRetirementWireV1,
        _: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationRetirementV1, InvalidResidentRecordV1> {
        panic!("empty producer frontier has no retirement")
    }
}

struct HistoricalKeys(Ed25519PublicKeyV1);

impl ResidentServiceHistoricalKeyPort for HistoricalKeys {
    fn resolve_historical_principal_key(
        &mut self,
        _: &ResidentPrincipalIdV1,
        _: &ResidentKeyIdV1,
        _: PrincipalKeyEpochV1,
    ) -> ResidentServiceHistoricalKeyResolution {
        ResidentServiceHistoricalKeyResolution::Resolved(self.0.clone())
    }
}

struct AcceptSignatures;

impl ResidentServiceTrustSignaturePort for AcceptSignatures {
    fn verify_historical_checkpoint_signature(
        &mut self,
        _: &[u8],
        _: &Ed25519PublicKeyV1,
        _: &PrincipalTrustCheckpointPreimageV1,
        _: &PrincipalTrustCheckpointDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }
}

struct NoDeploymentAuthorities;

impl ResidentServiceDeploymentAuthorityAdmissionPort for NoDeploymentAuthorities {
    fn admit_historical_deployment_finalization(
        &mut self,
        _: &ControlRecord,
        _: &ControlRecord,
        _: &ControlRecord,
    ) -> Result<ResidentServiceDeploymentAuthorityRecords, InvalidResidentRecordV1> {
        panic!("initial trust frontier has no deployment finalization")
    }
}

fn recover(
    fixture: Fixture,
    checkpoint_snapshot: Option<PrincipalTrustCheckpointSetSnapshot>,
    maximum_producer_page_items: usize,
    calls: Rc<RefCell<Vec<&'static str>>>,
) -> Result<ResidentServiceTrustFrontier, ResidentServiceTrustRecoveryError> {
    let checkpoint_limits = PrincipalTrustCheckpointSetReadLimits::new(1, 65_536);
    let finalization_limits = DeploymentFinalizationSetReadLimits::new(1, 65_536, 128);
    let mut checkpoints = CheckpointReader {
        expected_binding: fixture.binding.clone(),
        expected_limits: checkpoint_limits,
        snapshot: checkpoint_snapshot,
        calls: Rc::clone(&calls),
    };
    let mut producers = EmptyProducerReader {
        expected_binding: fixture.binding.clone(),
        expected_page_items: maximum_producer_page_items,
        calls: Rc::clone(&calls),
    };
    let mut finalizations = FinalizationReader {
        expected_binding: fixture.binding.clone(),
        expected_limits: finalization_limits,
        calls,
    };
    let mut producer_admission = NoProducerRecords;
    let mut historical_keys = HistoricalKeys(fixture.root_key);
    let mut checkpoint_signatures = AcceptSignatures;
    let mut deployment_authorities = NoDeploymentAuthorities;
    ResidentServiceTrustFrontier::recover_from_physical(
        ResidentServicePhysicalTrustRecoveryInput {
            bootstrap: fixture.bootstrap,
            checkpoint_schema_sha256: fixture.checkpoint_schema,
            checkpoint_limits,
            maximum_producer_page_items,
            finalization_limits,
        },
        ResidentServiceTrustPhysicalPorts {
            checkpoints: &mut checkpoints,
            producers: &mut producers,
            finalizations: &mut finalizations,
        },
        &mut producer_admission,
        ResidentServiceTrustEvidencePorts {
            historical_keys: &mut historical_keys,
            checkpoint_signatures: &mut checkpoint_signatures,
            deployment_authorities: &mut deployment_authorities,
        },
    )
}

#[test]
fn physical_recovery_uses_the_bootstrap_binding_limits_and_required_read_order() {
    let fixture = fixture();
    let snapshot = fixture.checkpoint_snapshot.clone();
    let calls = Rc::new(RefCell::new(Vec::new()));
    recover(fixture, Some(snapshot), 4, Rc::clone(&calls)).expect("physical trust recovery");
    assert_eq!(
        calls.borrow().as_slice(),
        ["checkpoints", "producers", "finalizations"]
    );
}

#[test]
fn physical_recovery_rejects_a_substituted_checkpoint_store() {
    let fixture = fixture();
    let original = fixture.checkpoint_snapshot.clone();
    let substituted_binding = PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("substituted-trust-store").expect("store"),
        fixture.binding.trust_store_incarnation_id().clone(),
        fixture.binding.principal_id().clone(),
    );
    let substituted = PrincipalTrustCheckpointSetSnapshot::new(
        substituted_binding,
        original.through().clone(),
        original.observed_through(),
        original.checkpoint_records().to_vec().into_boxed_slice(),
    )
    .expect("structurally valid substituted physical snapshot");
    let error = match recover(
        fixture,
        Some(substituted),
        4,
        Rc::new(RefCell::new(Vec::new())),
    ) {
        Ok(_) => panic!("store substitution must fail"),
        Err(error) => error,
    };
    assert_eq!(error, ResidentServiceTrustRecoveryError::CheckpointChain);
}

#[test]
fn physical_recovery_rejects_zero_producer_bounds_before_finalization_reads() {
    let fixture = fixture();
    let snapshot = fixture.checkpoint_snapshot.clone();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let error = match recover(fixture, Some(snapshot), 0, Rc::clone(&calls)) {
        Ok(_) => panic!("zero producer bound must fail"),
        Err(error) => error,
    };
    assert_eq!(error, ResidentServiceTrustRecoveryError::ProducerFrontier);
    assert_eq!(calls.borrow().as_slice(), ["checkpoints"]);
}

#[test]
fn physical_recovery_requires_checkpoint_evidence_before_other_reads() {
    let fixture = fixture();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let error = match recover(fixture, None, 4, Rc::clone(&calls)) {
        Ok(_) => panic!("missing checkpoint evidence must fail"),
        Err(error) => error,
    };
    assert_eq!(
        error,
        ResidentServiceTrustRecoveryError::CheckpointUnavailable
    );
    assert_eq!(calls.borrow().as_slice(), ["checkpoints"]);
}
