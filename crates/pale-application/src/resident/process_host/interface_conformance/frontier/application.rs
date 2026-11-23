//! Real resident state prepared for the interface conformance frontier.
//!
//! The setup uses ordinary generation admission, lifecycle commits, ambient
//! session admission, checkpointing, and cold recovery. Its physical custody is
//! memory only and is valid only for the callback that owns the final service.

use std::collections::BTreeSet;

use pale_core::records::*;
use pale_ports::*;

use crate::resident::*;

use super::{
    captured_generation::{captured_generation, CapturedGenerationFixture},
    lifecycle::{
        ConformanceGenerationSource, ConformanceHistorical, ConformanceLifecycle, EmptyHistorical,
    },
    principal::admitted_bootstrap,
    session::{ConformanceSessionFixture, ProvingAuthority},
    storage::{ConformanceAuthorityStore, UnavailableEventCustody},
};

fn limits() -> AuthorityRecoveryReadLimits {
    AuthorityRecoveryReadLimits::new(1_000_000, 1_000_000, 8, 64)
        .expect("bounded conformance recovery")
}

fn reconstruction_parameters(epoch: u64) -> ResidentReconstructionParameters {
    ResidentReconstructionParameters::new(
        pale_kernel::AdaptiveNodeSetPolicy::new(32, 2_500).expect("bounded conformance generation"),
        4,
        4,
        WakeEpochV1::new(epoch),
    )
}

/// Fully committed in-memory resident source for one outer-interface proof.
pub(in super::super) struct ConformanceResidentState {
    pub(in super::super) store_owner: ConformanceAuthorityStore,
    pub(in super::super) store: AuthorityStoreCoordinate,
    pub(in super::super) generation: AdmittedCompiledGeneration,
    pub(in super::super) profile: EffectiveProjectProfileV1,
    pub(in super::super) lifecycle: ConformanceLifecycle,
    pub(in super::super) historical: ConformanceHistorical,
    pub(in super::super) session: ConformanceSessionFixture,
    pub(in super::super) trust_ref: ResidentRecordRefV1,
    pub(in super::super) trust: PrincipalTrustCheckpointV1,
    reconstructions: Vec<ResidentGenerationReconstructionInput>,
    pub(in super::super) expected_peer: LocalIpcPeerCredentials,
}

impl ConformanceResidentState {
    pub(in super::super) fn prepare() -> Self {
        let CapturedGenerationFixture {
            generation,
            profile,
            initial_compilation,
            initial_materializations,
            reconstructions,
        } = captured_generation();
        let lifecycle = ConformanceLifecycle::initial(generation.record());
        let bootstrap = admitted_bootstrap();
        let view = bootstrap.atom_view();
        let (trust_ref, trust) = view.trust_checkpoint_entry();
        let trust_ref = trust_ref.clone();
        let trust = trust.clone();
        let store = AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("control-store-1").expect("control store"),
            ControlIncarnationIdV1::parse("control-incarnation-1")
                .expect("control-store incarnation"),
        );
        let session = ConformanceSessionFixture::construct(&trust_ref, &trust, &store);
        let store_owner = ConformanceAuthorityStore::default();
        store_owner.bind(store.clone());

        let mut empty_source = ConformanceGenerationSource::empty();
        let mut events = UnavailableEventCustody;
        let opened = ResidentApplication::cold_open(
            &store_owner,
            &store_owner,
            &mut events,
            limits().maximum_entry_bytes(),
            ResidentColdOpenInput::new(
                store.clone(),
                limits(),
                limits().maximum_entry_bytes(),
                RecoveryReferenceKnowledge::Known(BTreeSet::new()),
                &EmptyHistorical,
                &mut empty_source,
                reconstruction_parameters(1),
            ),
        )
        .expect("empty conformance resident cold-opens");
        let ResidentColdOpenOutcome::Ready(mut application) = opened else {
            panic!("empty conformance resident must be writable")
        };
        let admitted = application
            .admit_generation(
                initial_compilation,
                initial_materializations,
                pale_kernel::AdaptiveNodeSetPolicy::new(32, 2_500)
                    .expect("bounded conformance generation"),
            )
            .expect("captured generation admits into the resident");
        assert_eq!(
            admitted,
            generation.record().compiled_generation_digest().clone()
        );
        let publication_receipt = application
            .publish(lifecycle.published.clone())
            .expect("generation publication commits");
        let advertisement_receipt = application
            .advertise(lifecycle.advertised.clone())
            .expect("generation advertisement commits");
        let activation_receipt = application
            .activate(lifecycle.active.clone(), lifecycle.fence.clone())
            .expect("generation activation commits");
        let session_receipt = session.with_admitted_session(
            generation.record(),
            &profile,
            &lifecycle.active,
            &lifecycle.fence,
            &trust_ref,
            &trust,
            |admitted, current, _| {
                let (_, receipt) = application
                    .admit_session(
                        SessionAdmissionLeaseWireV1::from(admitted.lease()),
                        current,
                        &mut ProvingAuthority,
                    )
                    .expect("ambient session commits into the resident");
                receipt
            },
        );
        let setup_sequences = [
            publication_receipt.journal_sequence().get(),
            advertisement_receipt.journal_sequence().get(),
            activation_receipt.journal_sequence().get(),
            session_receipt.journal_sequence().get(),
        ];
        assert!(
            setup_sequences.windows(2).all(|pair| pair[0] < pair[1]),
            "resident setup receipts form one monotone commit sequence"
        );
        application
            .checkpoint()
            .expect("resident recovery checkpoint commits");
        drop(application);

        let historical = ConformanceHistorical::new(
            generation.record().clone(),
            profile.clone(),
            lifecycle.clone(),
            session.signing.clone(),
        );
        Self {
            store_owner,
            store,
            generation,
            profile,
            lifecycle,
            historical,
            session,
            trust_ref,
            trust,
            reconstructions,
            expected_peer: LocalIpcPeerCredentials::new(501, 20, Some(8_081)),
        }
    }

    pub(in super::super) fn take_generation_source(&mut self) -> ConformanceGenerationSource {
        ConformanceGenerationSource::new(
            self.reconstructions
                .pop()
                .expect("the conformance resident has one bounded reconstruction per service"),
        )
    }
}
