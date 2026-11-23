//! Admitted initial producer setup shared by application and native custody tests.
//!
//! Signature and checkpoint-content witnesses are controlled test inputs. This
//! fixture exercises public admission and preparation, not a production signer.

use super::principal_trust_test_support::{admitted_bootstrap_fixture, AdmittedBootstrapFixture};
use super::producer_record_fixture::{
    admit_allocation, control_successor, producer_preimage, trust_successor,
};
use super::resident::*;
use pale_core::records::*;
use pale_ports::*;

pub(crate) struct InitialProducerFixture {
    pub(crate) fixture: AdmittedBootstrapFixture,
    pub(crate) allocation_ref: ResidentRecordRefV1,
    pub(crate) allocation: ProducerIncarnationV1,
    pub(crate) next_trust_ref: ResidentRecordRefV1,
    pub(crate) next_trust: PrincipalTrustCheckpointV1,
    pub(crate) next_control_ref: ResidentRecordRefV1,
    pub(crate) next_control: ControlStateCheckpointV1,
    pub(crate) empty_frontier: RecoveredPrincipalProducerIncarnationFrontier,
}

impl InitialProducerFixture {
    pub(crate) fn new() -> (Self, Vec<ControlStateCheckpointPreimageV1>) {
        let (fixture, control_preimages) = admitted_bootstrap_fixture();
        let atom = fixture.admitted.atom_view();
        let authority_ref = atom.authorities().producer_authority_ref();
        let authority = atom.authorities().producer_authority();
        let allocation_ref = ResidentRecordRefV1::parse("allocation-a").expect("allocation ref");
        let allocation_preimage = producer_preimage(authority_ref, authority, 'a', 50);
        let allocation = admit_allocation(
            &allocation_preimage,
            authority_ref,
            authority,
            &ProducerIncarnationHeadAdmissionStateV1::Unallocated,
        );
        let next_heads = vec![
            ProducerIncarnationHeadV1::active(&allocation_ref, &allocation)
                .expect("active semantic head"),
        ];
        let (trust_ref, trust) = atom.trust_checkpoint_entry();
        let (next_trust_ref, next_trust) = trust_successor(
            fixture.trust_preimage.clone(),
            trust_ref,
            trust,
            "trust-checkpoint-2",
            producer_incarnation_heads_root_sha256(&next_heads).expect("next producer root"),
            |_| {},
        );
        let (control_ref, control) = atom
            .initial_control_checkpoints()
            .next()
            .expect("initial control checkpoint");
        let (next_control_ref, next_control) = control_successor(
            control_preimages[0].clone(),
            control_ref,
            control,
            "control-checkpoint-producer-2",
        );
        let empty_frontier = recover_principal_producer_incarnation_frontier(
            &mut EmptyHistory,
            PrincipalTrustStoreBinding::new(
                ControlStoreIdV1::parse("trust-store-1").expect("trust store"),
                ControlIncarnationIdV1::parse("trust-incarnation-1").expect("trust incarnation"),
                ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            ),
            CurrentPrincipalTrustHead {
                checkpoint_ref: trust_ref,
                checkpoint: trust,
            },
            1,
            &mut EmptyHistory,
        )
        .expect("empty history agrees with the admitted bootstrap root");
        (
            Self {
                fixture,
                allocation_ref,
                allocation,
                next_trust_ref,
                next_trust,
                next_control_ref,
                next_control,
                empty_frontier,
            },
            control_preimages,
        )
    }

    pub(crate) fn prepare(&self) -> PreparedPrincipalProducerIncarnationCustody {
        let atom = self.fixture.admitted.atom_view();
        let (trust_ref, trust) = atom.trust_checkpoint_entry();
        let (control_ref, control) = atom
            .initial_control_checkpoints()
            .next()
            .expect("initial checkpoint");
        let verifier = |construction: &PrincipalProducerCheckpointConstruction<'_, '_>| {
            assert_initial_construction(construction, &self.allocation_ref);
            Ok(())
        };
        prepare_initial_principal_producer_incarnation(
            AdmittedProducerIncarnation::new(&self.allocation_ref, &self.allocation),
            PrincipalProducerIncarnationProjectionContext::new(
                &self.empty_frontier,
                AdmittedPrincipalTrustCheckpointTransition::new(
                    trust_ref,
                    trust,
                    &self.next_trust_ref,
                    &self.next_trust,
                )
                .expect("initial trust transition"),
                AdmittedProducerControlCheckpointTransition::new(
                    control_ref,
                    control,
                    &self.next_control_ref,
                    &self.next_control,
                    &verifier,
                ),
            )
            .expect("initial projection context"),
        )
        .expect("initial public projection")
    }
}

pub(crate) fn assert_initial_construction(
    construction: &PrincipalProducerCheckpointConstruction<'_, '_>,
    allocation_ref: &ResidentRecordRefV1,
) {
    assert_eq!(
        construction.transition_kind(),
        PrincipalProducerIncarnationTransitionKind::InitialAllocation
    );
    assert!(construction.predecessor_allocation().is_none());
    assert_eq!(
        construction
            .appended_allocation()
            .expect("appended allocation")
            .0,
        allocation_ref
    );
}

/// The bootstrap root is empty; any request to admit a stored record is a bug.
struct EmptyHistory;

impl PrincipalProducerIncarnationReadPort for EmptyHistory {
    fn read_by_epoch(
        &mut self,
        _: &PrincipalTrustStoreBinding,
        _: &ProducerEpochV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        panic!("empty recovery uses bounded head pages")
    }
    fn read_head(
        &mut self,
        _: &PrincipalTrustStoreBinding,
        _: &ProducerIdV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        panic!("empty recovery uses bounded head pages")
    }
    fn read_by_allocation(
        &mut self,
        _: &PrincipalTrustSnapshot,
        _: &ControlRecordCoordinate,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError> {
        panic!("empty history has no allocation lineage")
    }
    fn read_heads_page(
        &mut self,
        request: &PrincipalProducerIncarnationHeadPageRead,
    ) -> Result<PrincipalProducerIncarnationHeadPage, ControlStoreError> {
        assert!(request.after().is_none(), "one complete empty page");
        Ok(PrincipalProducerIncarnationHeadPage::new(
            request,
            vec![],
            PrincipalProducerIncarnationHeadPageBoundary::Complete,
        )
        .expect("empty page"))
    }
}

impl PrincipalProducerIncarnationRecoveryAdmission for EmptyHistory {
    fn admit_allocation(
        &mut self,
        _: ProducerIncarnationWireV1,
        _: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationV1, InvalidResidentRecordV1> {
        panic!("empty history cannot admit an allocation")
    }
    fn admit_retirement(
        &mut self,
        _: ProducerIncarnationRetirementWireV1,
        _: &ProducerIncarnationHeadAdmissionStateV1<'_>,
    ) -> Result<ProducerIncarnationRetirementV1, InvalidResidentRecordV1> {
        panic!("empty history cannot admit a retirement")
    }
}
