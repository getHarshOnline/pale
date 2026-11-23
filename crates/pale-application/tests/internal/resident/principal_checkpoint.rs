//! Principal profile admission and exact prepared-root association tests.
//!
//! The port below returns synthetic roots and bytes. These tests establish the
//! application boundary; native tests own actual Merkle computation and custody.

use super::*;
use crate::{principal_trust_test_support, resident};
use pale_ports::{
    AccumulatorMutation, AccumulatorPosition, ControlStoreError, ControlTransactionPort,
    ControlTransactionPreparation, PreparedPrincipalBootstrapAtom,
    PrincipalBootstrapCommitDisposition, PrincipalBootstrapDatabaseObservation,
    StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
};

#[path = "../../support/principal_checkpoint.rs"]
mod fixture;

#[derive(Clone, Copy, Default)]
enum Response {
    #[default]
    Exact,
    DifferentRequest,
    Limit,
}

#[derive(Default)]
struct Store {
    response: Response,
    request: Option<OperationAccumulatorPreparation>,
    transaction: Option<ControlTransactionSequenceV1>,
    transaction_reads: usize,
    trust_reads: std::collections::VecDeque<
        Result<Option<pale_ports::PrincipalTrustSnapshot>, ControlStoreError>,
    >,
    trust_requests: Vec<(pale_ports::PrincipalTrustStoreBinding, usize)>,
}

impl pale_ports::PrincipalTrustReadPort for Store {
    fn read_current_checkpoint(
        &mut self,
        binding: &pale_ports::PrincipalTrustStoreBinding,
        maximum_body_bytes: usize,
    ) -> Result<Option<pale_ports::PrincipalTrustSnapshot>, ControlStoreError> {
        self.trust_requests
            .push((binding.clone(), maximum_body_bytes));
        self.trust_reads
            .pop_front()
            .expect("finalization must select its trust read explicitly")
    }

    fn read_record(
        &mut self,
        _: &pale_ports::PrincipalTrustStoreBinding,
        _: &pale_ports::ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<pale_ports::ControlRecord>, ControlStoreError> {
        panic!("principal checkpoint tests do not read an individual trust record")
    }
}

impl ControlTransactionPort for Store {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        self.transaction_reads += 1;
        Ok(ControlTransactionPreparation::new(
            store.clone(),
            self.transaction
                .unwrap_or(ControlTransactionSequenceV1::new(1).unwrap()),
        )
        .unwrap())
    }
}

impl Store {
    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        self.request = Some(request.clone());
        if matches!(self.response, Response::Limit) {
            return Err(OperationAccumulatorPreparationError::Limit);
        }
        let mut returned = request.clone();
        if matches!(self.response, Response::DifferentRequest) {
            returned = OperationAccumulatorPreparation::new(
                request.basis().clone(),
                request.state().clone(),
                request.unresolved().clone(),
                request.limits(),
                request.implementation(),
                ControlTransactionPreparation::new(
                    request.basis().store().clone(),
                    ControlTransactionSequenceV1::new(request.transaction().expected().get() + 1)
                        .unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
        }
        let (old, next) = match returned.basis() {
            AccumulatorPreparationBasis::Initial(_) => (hash('0'), hash('a')),
            AccumulatorPreparationBasis::Successor(prior) => {
                (prior.state_root().clone(), hash('b'))
            }
        };
        Ok(PreparedOperationAccumulator::new(
            returned,
            StateAccumulatorBatch::new(
                old,
                next,
                vec![AccumulatorMutation::new(
                    AccumulatorPosition::Leaf([1; 32]),
                    None,
                    Some(vec![2; 32]),
                    32,
                )
                .unwrap()],
                1,
            )
            .unwrap(),
            UnresolvedCustodyAccumulatorBatch::new(hash('0'), hash('0'), Vec::new(), 1).unwrap(),
        )
        .unwrap())
    }
}

impl pale_ports::PrincipalCheckpointPreparationPort for Store {
    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        Store::prepare_accumulator(self, request)
    }
}

impl PrincipalBootstrapCustodyPort for Store {
    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        Store::prepare_accumulator(self, request)
    }

    fn compare_and_commit(
        &mut self,
        _: &PreparedPrincipalBootstrapAtom,
    ) -> Result<PrincipalBootstrapCommitDisposition, ControlStoreError> {
        panic!("checkpoint preparation and admission must not commit")
    }

    fn observe_exact(
        &mut self,
        _: &PreparedPrincipalBootstrapAtom,
    ) -> Result<PrincipalBootstrapDatabaseObservation, ControlStoreError> {
        panic!("checkpoint preparation and admission must not perform commit read-back")
    }
}

fn hash(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(byte.to_string().repeat(64)).unwrap()
}

#[path = "principal_checkpoint/finalization.rs"]
pub(crate) mod finalization;

#[test]
fn source_profile_is_exact_bounded_content_not_a_generic_profile_selector() {
    let content = SupportedPrincipalTrustCheckpointProfile::canonical_content().unwrap();
    let digest = SupportedPrincipalTrustCheckpointProfile::content_digest().unwrap();
    assert_eq!(
        digest.as_str(),
        "ef6ab18fe12de3c9f629e280a69a1f1be3d687601840609794475f093f9884d0"
    );
    let reference = ResidentRecordRefV1::parse("profile").unwrap();
    let admitted = SupportedPrincipalTrustCheckpointProfile::admit(
        reference.clone(),
        &digest,
        content.as_bytes(),
    )
    .unwrap();
    assert_eq!(admitted.digest, digest);
    assert_eq!(admitted.domains[0].as_str(), "principal-trust");
    let value = canon::parse_strict(content.as_bytes()).unwrap();
    assert_eq!(value["record_classes"].as_array().unwrap().len(), 4);
    assert_eq!(value["present_zero"], "reject-reserved-absence");
    for content in [
        content.replace("principal-trust-head", "principal-trust-history"),
        content.replace(
            "\"control_schema_version\":1",
            "\"control_schema_version\":2",
        ),
    ] {
        let value = canon::parse_strict(content.as_bytes()).unwrap();
        let digest = CheckpointProfileDigest::hash_canonical(&value).unwrap();
        assert!(matches!(
            SupportedPrincipalTrustCheckpointProfile::admit(
                reference.clone(),
                &digest,
                content.as_bytes()
            ),
            Err(PrincipalCheckpointError::UnsupportedProfile)
        ));
    }
    assert!(matches!(
        SupportedPrincipalTrustCheckpointProfile::admit(
            reference.clone(),
            &CheckpointProfileDigest::parse("f".repeat(64)).unwrap(),
            content.as_bytes()
        ),
        Err(PrincipalCheckpointError::ProfileDigest)
    ));
    assert!(matches!(
        SupportedPrincipalTrustCheckpointProfile::admit(
            reference.clone(),
            &digest,
            &vec![b' '; 4097]
        ),
        Err(PrincipalCheckpointError::UnsupportedProfile)
    ));
    assert!(matches!(
        SupportedPrincipalTrustCheckpointProfile::admit(reference, &digest, br#"{"x":1,"x":1}"#),
        Err(PrincipalCheckpointError::Canonicalize(_))
    ));
}

#[test]
fn initial_stage_requires_the_signer_to_match_the_retained_profile() {
    let profile = SupportedPrincipalTrustCheckpointProfile::admit(
        ResidentRecordRefV1::parse("profile").unwrap(),
        &SupportedPrincipalTrustCheckpointProfile::content_digest().unwrap(),
        SupportedPrincipalTrustCheckpointProfile::canonical_content()
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    let physical = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
    );
    let logical = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("principal-control-store-1").unwrap(),
        ControlIncarnationIdV1::parse("principal-control-incarnation-1").unwrap(),
    );
    let schema = SchemaDigest::parse("8".repeat(64)).unwrap();
    let mut selection = profile.bootstrap_profile(&physical, &logical, &schema);
    let authorities = principal_trust_test_support::admit_profile_authorities(
        &PrincipalBootstrapIdV1::parse("bootstrap-1").unwrap(),
        &selection,
    )
    .unwrap();
    let trust =
        principal_trust_test_support::initial_trust_preimage(&authorities, Some(&selection));
    let digest = PrincipalTrustCheckpointDigest::hash_canonical(&trust).unwrap();
    let different_profile = ResidentRecordRefV1::parse("different-profile").unwrap();
    selection.checkpoint_profile_ref = &different_profile;
    let result = authorities.admit_initial_trust_checkpoint(
        ResidentRecordRefV1::parse("trust-head").unwrap(),
        principal_trust_test_support::signed_wire(
            &trust,
            "canonical_trust_checkpoint_sha256",
            &digest,
        ),
        &trust.schema_sha256,
        selection,
        |_, _, _, _, _| Ok(()),
    );
    assert!(matches!(
        result,
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "principal bootstrap checkpoint authority",
            ..
        })
    ));
}

#[test]
fn admitted_records_prepare_two_scoped_leaves_and_retain_the_exact_batch_through_receipt() {
    let mut store = Store::default();
    let (prepared, checkpoint) = fixture::prepare(&mut store, |_| {}).unwrap();
    let request = store.request.as_ref().unwrap();
    assert_eq!(request.basis().store().store_id().as_str(), "trust-store-1");
    assert_eq!(
        request.basis().store().incarnation_id().as_str(),
        "trust-incarnation-1"
    );
    assert_eq!(
        checkpoint.control_store_id.as_str(),
        "principal-control-store-1"
    );
    assert_eq!(
        checkpoint.control_incarnation_id.as_str(),
        "principal-control-incarnation-1"
    );
    assert_eq!(request.state().changes().len(), 2);
    assert!(request.unresolved().changes().is_empty());
    assert_eq!(
        checkpoint
            .changed_leaf_keys
            .iter()
            .map(|key| key.record_class.as_str())
            .collect::<Vec<_>>(),
        ["checkpoint-signing-authority-head", "principal-trust-head"]
    );
    let expected_scope = canonical_hash(&serde_json::json!({
        "domain":"PALE_PRINCIPAL_TRUST_CHECKPOINT_SCOPE_V1",
        "control_store_id":"principal-control-store-1",
        "control_incarnation_id":"principal-control-incarnation-1",
        "checkpoint_domain_id":"principal-trust",
        "domain_epoch":1,
    }))
    .unwrap();
    let mut expected_keys = checkpoint
        .changed_leaf_keys
        .iter()
        .map(|key| {
            assert_eq!(key.scope_key_sha256, expected_scope);
            canonical_hash(&(KEY_DOMAIN, key)).unwrap()
        })
        .collect::<Vec<_>>();
    expected_keys.sort();
    assert_eq!(
        request
            .state()
            .changes()
            .iter()
            .map(|change| change.key().clone())
            .collect::<Vec<_>>(),
        expected_keys
    );
    assert!(request
        .state()
        .changes()
        .iter()
        .all(|change| change.predecessor().is_none() && change.successor().is_some()));
    let [batch] = prepared.candidate().accumulators() else {
        panic!("the supported profile prepares one initial domain")
    };
    assert_eq!(batch.binding().store(), request.basis().store());
    assert_eq!(batch.binding().transaction(), request.transaction());
    assert_eq!(
        batch.state().successor_root(),
        &checkpoint.state_root_sha256
    );
    assert_eq!(
        batch.unresolved().successor_root(),
        &checkpoint.unresolved_custody_root_sha256
    );
    assert_eq!(batch.binding().successor().domain_sequence().get(), 1);
    assert_eq!(
        batch.binding().successor().committed_transaction_sequence(),
        checkpoint.committed_transaction_sequence
    );
}

#[test]
fn signed_candidates_cannot_replace_prepared_meaning_or_native_correlation() {
    let changes: [fn(&mut ControlStateCheckpointPreimageV1); 9] = [
        |value| value.state_root_sha256 = hash('b'),
        |value| value.unresolved_custody_root_sha256 = hash('b'),
        |value| value.leaf_count = ControlLeafCountV1::new(3).unwrap(),
        |value| {
            value.changed_leaf_keys[0].logical_key = ControlLogicalKeyV1::parse("other").unwrap()
        },
        |value| {
            value.committed_transaction_sequence = ControlTransactionSequenceV1::new(3).unwrap()
        },
        |value| value.control_store_id = ControlStoreIdV1::parse("trust-store-1").unwrap(),
        |value| value.control_incarnation_id = ControlIncarnationIdV1::parse("other").unwrap(),
        |value| value.checkpoint_profile_ref = ResidentRecordRefV1::parse("other-profile").unwrap(),
        |value| {
            value.checkpoint_domain_id =
                ControlCheckpointDomainIdV1::parse("admin-authority").unwrap()
        },
    ];
    for change in changes {
        let mut store = Store::default();
        assert!(matches!(
            fixture::prepare(&mut store, change),
            Err(PrincipalCheckpointError::Admission(_))
        ));
        assert!(store.request.is_some());
    }
    let mut store = Store {
        response: Response::DifferentRequest,
        ..Store::default()
    };
    assert!(matches!(
        fixture::prepare(&mut store, |_| {}),
        Err(PrincipalCheckpointError::Association)
    ));
    let mut store = Store {
        response: Response::Limit,
        ..Store::default()
    };
    assert!(matches!(
        fixture::prepare(&mut store, |_| {}),
        Err(PrincipalCheckpointError::Native(
            OperationAccumulatorPreparationError::Limit
        ))
    ));
}
