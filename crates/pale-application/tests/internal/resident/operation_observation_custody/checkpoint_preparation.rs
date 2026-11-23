//! Application request/result association, not native Merkle-proof qualification.

use super::*;
use crate::local_operation_start_fixture::StartAdmission;
use crate::resident::checkpoint_profile::operation_observation_change;
use crate::resident::{OperationCheckpointProfileError, SupportedOperationCheckpointProfile};
use pale_ports::{
    AccumulatorPreparationBasis, AccumulatorPreparationLimits, OperationAccumulatorCheckpoint,
    OperationAccumulatorPreparation, OperationAccumulatorPreparationError,
    PreparedOperationAccumulator,
};

#[test]
fn supported_profile_accepts_its_source_content_and_canonical_formatting_only() {
    let source = SupportedOperationCheckpointProfile::canonical_content().unwrap();
    let digest = SupportedOperationCheckpointProfile::content_digest().unwrap();
    let value = pale_core::canon::parse_strict(source.as_bytes()).unwrap();
    assert_eq!(
        CheckpointProfileDigest::hash_canonical(&value).unwrap(),
        digest
    );
    let pretty = serde_json::to_string_pretty(&value).unwrap();
    assert_ne!(source, pretty);
    for bytes in [source.as_bytes(), pretty.as_bytes()] {
        let admitted = SupportedOperationCheckpointProfile::admit(
            record_ref("operation/checkpoint-profile"),
            &digest,
            bytes,
        )
        .expect("exact source-owned content accepts either formatting");
        admitted
            .select(&store_coordinate(), &profile_request())
            .unwrap()
            .verify(
                &store_coordinate(),
                &profile_fixture().handle,
                &profile_checkpoints().predecessor,
            )
            .expect("actual selected encoding and coordinates");
    }
    let duplicate = format!("{{\"content\":\"duplicate\",{}", &source[1..]);
    assert!(matches!(
        SupportedOperationCheckpointProfile::admit(
            record_ref("operation/checkpoint-profile"),
            &digest,
            duplicate.as_bytes(),
        ),
        Err(OperationCheckpointProfileError::Canonicalize(_))
    ));
    assert!(matches!(
        SupportedOperationCheckpointProfile::admit(
            record_ref("operation/checkpoint-profile"),
            &digest,
            &vec![b' '; 4097],
        ),
        Err(OperationCheckpointProfileError::Unsupported)
    ));
    assert!(matches!(
        SupportedOperationCheckpointProfile::admit(
            record_ref("operation/checkpoint-profile"),
            &CheckpointProfileDigest::parse("0".repeat(64)).unwrap(),
            source.as_bytes(),
        ),
        Err(OperationCheckpointProfileError::Digest)
    ));
}

#[test]
fn previous_root_only_profile_and_its_signer_hash_are_not_supported() {
    let mut old = pale_core::canon::parse_strict(
        SupportedOperationCheckpointProfile::canonical_content()
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    old["conflict_domain"] = serde_json::json!("operation-request-index-identity-v1");
    old["supported_mutation"] = serde_json::json!(
        "initial-root-request-handle-observation-and-retained-observation-replacement"
    );
    let old_digest = CheckpointProfileDigest::hash_canonical(&old).unwrap();
    assert_ne!(
        old_digest,
        SupportedOperationCheckpointProfile::content_digest().unwrap()
    );
    assert!(matches!(
        SupportedOperationCheckpointProfile::admit(
            record_ref("operation/checkpoint-profile"),
            &old_digest,
            pale_core::canon::to_canonical_string(&old)
                .unwrap()
                .as_bytes()
        ),
        Err(OperationCheckpointProfileError::Unsupported)
    ));
    let mut authority_preimage = profile_checkpoint(1, None);
    authority_preimage.checkpoint_profile_sha256 = old_digest;
    assert!(!selection().matches_authority(&checkpoint_authority(&authority_preimage)));
    // Historical qualification receipts retain their old content identity. They
    // do not authorize construction or signatures under the corrected profile.
}

#[test]
fn profile_without_original_session_basis_cannot_supply_current_signing_authority() {
    let mut old = pale_core::canon::parse_strict(
        SupportedOperationCheckpointProfile::canonical_content()
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    let fields = old.as_object_mut().unwrap();
    assert!(fields.remove("session_basis_value_encoding").is_some());
    assert!(fields.remove("session_basis_shape_sha256").is_some());
    old["supported_mutation"] = serde_json::json!(
        "initial-root-and-inherited-child-request-handle-observation-insertion-retained-observation-replacement-and-a2a-route-binding-evidence-insertion-task-history-replacement"
    );
    let old_digest = CheckpointProfileDigest::hash_canonical(&old).unwrap();
    assert_ne!(
        old_digest,
        SupportedOperationCheckpointProfile::content_digest().unwrap()
    );
    assert!(matches!(
        SupportedOperationCheckpointProfile::admit(
            record_ref("operation/checkpoint-profile"),
            &old_digest,
            pale_core::canon::to_canonical_string(&old)
                .unwrap()
                .as_bytes(),
        ),
        Err(OperationCheckpointProfileError::Unsupported)
    ));
    let mut signer = profile_checkpoint(1, None);
    signer.checkpoint_profile_sha256 = old_digest;
    assert!(!selection().matches_authority(&checkpoint_authority(&signer)));
}

#[test]
fn child_selection_inherits_domain_but_binds_its_own_exact_request_and_profile() {
    let parent = selection();
    let child_request =
        crate::effect_case_lineage_fixture::EffectFixture::new("child-profile").request;
    let child = parent.child(&profile(), &child_request).unwrap();
    assert_eq!(
        child.initial_basis().unwrap(),
        parent.initial_basis().unwrap()
    );
    assert_ne!(
        SupportedOperationCheckpointProfile::conflict_domain(&child_request).unwrap(),
        *child.initial_basis().unwrap().domain()
    );
    let child_handle = OperationFixture::with_binding(
        "1",
        "child-operation",
        120,
        child_request.local_request_binding().unwrap(),
    );
    child
        .verify(
            &store_coordinate(),
            &child_handle.handle,
            &profile_checkpoints().predecessor,
        )
        .unwrap();
    assert!(matches!(
        child.verify(
            &store_coordinate(),
            &profile_fixture().handle,
            &profile_checkpoints().predecessor
        ),
        Err(OperationCheckpointProfileError::Association)
    ));
    let renamed = SupportedOperationCheckpointProfile::admit(
        record_ref("other-profile"),
        &SupportedOperationCheckpointProfile::content_digest().unwrap(),
        SupportedOperationCheckpointProfile::canonical_content()
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    assert!(matches!(
        parent.child(&renamed, &child_request),
        Err(OperationCheckpointProfileError::Association)
    ));
}

#[test]
fn supported_profile_rejects_validly_rehashed_unsupported_content_before_native_reads() {
    let source = SupportedOperationCheckpointProfile::canonical_content().unwrap();
    let fixture = profile_fixture();
    let checkpoints = profile_checkpoints();
    let calls = Cell::new(0);
    let mut store = PreparationOnly::new(
        |_: &OperationAccumulatorPreparation| {
            calls.set(calls.get() + 1);
            panic!("unsupported profile must not reach the snapshot owner")
        },
        ControlTransactionSequenceV1::new(1).unwrap(),
    );
    for field in [
        "native_implementation",
        "conflict_domain",
        "scope_encoding",
        "key_encoding",
        "observation_value_encoding",
        "request_value_encoding",
        "session_basis_value_encoding",
        "session_basis_shape_sha256",
        "handle_value_encoding",
        "initial_domain_epoch",
        "unknown_field",
    ] {
        let mut value = pale_core::canon::parse_strict(source.as_bytes()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert(field.into(), serde_json::json!("unsupported"));
        let bytes = pale_core::canon::to_canonical_string(&value).unwrap();
        let digest = CheckpointProfileDigest::hash_canonical(&value).unwrap();
        assert_ne!(
            digest,
            SupportedOperationCheckpointProfile::content_digest().unwrap()
        );
        // This is a valid new canonical identity, not a stale-digest rejection.
        let result = SupportedOperationCheckpointProfile::admit(
            record_ref("operation/checkpoint-profile"),
            &digest,
            bytes.as_bytes(),
        )
        .map(|admitted| {
            let selected = admitted
                .select(&store_coordinate(), &profile_request())
                .unwrap();
            prepare_operation_observation_append(fixture.transition(), &ResolveStore)
                .unwrap()
                .prepare_checkpoint(
                    selected,
                    &checkpoints.predecessor_ref,
                    &checkpoints.predecessor,
                    limits(),
                    &mut store,
                )
        });
        assert!(
            matches!(result, Err(OperationCheckpointProfileError::Unsupported)),
            "{field}: {result:?}"
        );
    }
    assert_eq!(calls.get(), 0);
}

#[test]
fn supported_profile_retains_original_request_identity_not_contents_or_handle() {
    let fixture = crate::effect_case_lineage_fixture::EffectFixture::new("profile");
    let original = SupportedOperationCheckpointProfile::conflict_domain(&fixture.request).unwrap();
    let mut changed = fixture.request_preimage.clone();
    changed.scope.purpose = BoundedResidentTextV1::parse("different purpose").unwrap();
    let changed = crate::effect_case_lineage_fixture::request(&changed);
    assert_ne!(
        fixture.request.canonical_request_sha256(),
        changed.canonical_request_sha256()
    );
    assert_eq!(
        original,
        SupportedOperationCheckpointProfile::conflict_domain(&changed).unwrap()
    );
    let selected = profile()
        .select(&store_coordinate(), &fixture.request)
        .unwrap();
    let foreign = OperationFixture::with_binding(
        "1",
        "operation-1",
        120,
        changed.local_request_binding().unwrap(),
    );
    assert!(matches!(
        selected.verify(
            &store_coordinate(),
            &foreign.handle,
            &profile_checkpoints().predecessor
        ),
        Err(OperationCheckpointProfileError::Association)
    ));
    let another_handle = OperationFixture::with_binding(
        "1",
        "another-handle",
        120,
        fixture.request.local_request_binding().unwrap(),
    );
    selected
        .verify(
            &store_coordinate(),
            &another_handle.handle,
            &profile_checkpoints().predecessor,
        )
        .expect("allocated handle does not split the request conflict domain");
    for change in [
        (|request: &mut CommonRequestEnvelopePreimageV1| {
            request.request_id = ResidentRequestIdV1::parse("other-request").unwrap()
        }) as fn(&mut CommonRequestEnvelopePreimageV1),
        |request| request.caller.identity = RequestCallerIdV1::parse("other-caller").unwrap(),
        |request| {
            request.caller.signer_key_epoch = SigningKeyEpochV1::new(2).unwrap();
            let authority = &request.signing_authority_binding;
            request.signing_authority_binding = SigningAuthorityBindingV1::integrated_runtime(
                authority.principal_id().clone(),
                ResidentKeyIdV1::parse(authority.signer_key_id()).unwrap(),
                request.caller.signer_key_epoch,
                authority.integrated_runtime_authority().unwrap().clone(),
            )
            .unwrap();
        },
    ] {
        let mut other = fixture.request_preimage.clone();
        change(&mut other);
        let other = crate::effect_case_lineage_fixture::request(&other);
        assert_ne!(
            original,
            SupportedOperationCheckpointProfile::conflict_domain(&other).unwrap()
        );
    }
}

#[test]
fn supported_profile_rejects_unlike_selection_before_snapshot_access() {
    let checkpoints = profile_checkpoints();
    let fixture = profile_fixture();
    let digest = SupportedOperationCheckpointProfile::content_digest().unwrap();
    let other_ref = SupportedOperationCheckpointProfile::admit(
        record_ref("another-profile"),
        &digest,
        SupportedOperationCheckpointProfile::canonical_content()
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    let other_request = crate::effect_case_lineage_fixture::EffectFixture::new("other").request;
    let selections = [
        other_ref
            .select(&store_coordinate(), &profile_request())
            .unwrap(),
        profile()
            .select(&store_coordinate(), &other_request)
            .unwrap(),
        profile()
            .select(
                &AuthorityStoreCoordinate::new(
                    ControlStoreIdV1::parse("other-store").unwrap(),
                    store_coordinate().incarnation_id().clone(),
                ),
                &profile_request(),
            )
            .unwrap(),
    ];
    for selection in selections {
        let mut store = PreparationOnly::new(
            |_: &OperationAccumulatorPreparation| panic!("unlike association must not read"),
            ControlTransactionSequenceV1::new(1).unwrap(),
        );
        let result = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
            .unwrap()
            .prepare_checkpoint(
                selection,
                &checkpoints.predecessor_ref,
                &checkpoints.predecessor,
                limits(),
                &mut store,
            );
        assert!(matches!(
            result,
            Err(OperationObservationCustodyProjectionError::Profile(
                OperationCheckpointProfileError::Association
            ))
        ));
    }
}

#[test]
fn child_parent_rejects_historical_selection_without_history_or_authority_calls() {
    use crate::resident::{
        prepare_local_child_parent, LocalOperationStartError, OperationCheckpointAncestryLimits,
        OperationObservationRecoveryAdmission, OperationObservationRecoveryInput,
        OperationObservationRecoveryLimits, OperationObservationSelection,
    };
    let (generation, profile) = super::local_start::source_generation();
    super::local_start::with_start(
        &generation,
        &profile,
        "child-history-refusal",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, parents| {
            let checkpoint = super::local_start::prepared_initial(prepared.unwrap());
            let (physical, pending, checkpoint) =
                super::local_start::issue_initial(checkpoint, request, |_| {}, &Cell::new(0))
                    .unwrap();
            let mut store = super::local_start::absent_start_store(request);
            let mut anchor = super::local_start::initial_anchor();
            let database = physical.commit(&mut store, &mut anchor, 1_048_576).unwrap();
            let operation = pending
                .finish(database.advance_anchor(&mut anchor).unwrap())
                .unwrap_or_else(|_| panic!("exact application custody fixture"));
            let mut port =
                PreparationOnly::new(
                    |_: &OperationAccumulatorPreparation| -> Result<
                        PreparedOperationAccumulator,
                        OperationAccumulatorPreparationError,
                    > {
                        panic!("history selection must not prepare native roots")
                    },
                    ControlTransactionSequenceV1::new(2).unwrap(),
                );
            let result = prepare_local_child_parent(
                OperationObservationRecoveryInput {
                    store: &store_coordinate(),
                    operation: &operation,
                    observation: OperationObservationSelection::HistoryMember {
                        reference: &record_ref("observation/child-history-refusal"),
                        digest: &OperationObservationDigest::parse("a".repeat(64)).unwrap(),
                    },
                    checkpoint_authority: &checkpoint_authority(&profile_checkpoint(1, None)),
                    limits: OperationObservationRecoveryLimits::new(4, 1_048_576, 262_144).unwrap(),
                    checkpoint_limits: OperationCheckpointAncestryLimits::new(4, 1_048_576, 4)
                        .unwrap(),
                    membership_limits: pale_ports::AccumulatorMembershipLimits::new(
                        1, 1024, 1_048_576,
                    )
                    .unwrap(),
                },
                parents.lease,
                &mut port,
                &mut anchor,
                OperationObservationRecoveryAdmission {
                    admit_checkpoint: |_: &ResidentRecordRefV1,
                                       _: ControlStateCheckpointWireV1|
                     -> Result<
                        ControlStateCheckpointV1,
                        InvalidResidentRecordV1,
                    > {
                        panic!("historical checkpoint admission")
                    },
                    resolve_signer: |_: &ResidentRecordRefV1,
                                     _: &OperationObservationWireV1|
                     -> Result<
                        SigningAuthorityBindingV1,
                        InvalidResidentRecordV1,
                    > {
                        panic!("historical signer resolution")
                    },
                    admit_state: |_: &OperationObservationStateV1| panic!("state admission"),
                    admit_rotation: |_: &ResidentRecordRefV1,
                                     _: &ArtifactDigest,
                                     _: &ResidentRecordRefV1,
                                     _: &ArtifactDigest| {
                        panic!("rotation admission")
                    },
                    verify_signature:
                        |_: &[u8],
                         _: &OperationObservationPreimageV1,
                         _: &OperationObservationDigest,
                         _: &CanonicalSignatureV1| {
                            panic!("observation signature")
                        },
                },
                |_: &[u8],
                 _: &ControlStateCheckpointPreimageV1,
                 _: &ControlStateCheckpointDigest,
                 _: &CanonicalSignatureV1| { panic!("checkpoint signature") },
            );
            assert!(matches!(result, Err(LocalOperationStartError::Coordinate)));
            assert_eq!(checkpoint.domain_sequence().get(), 1);
        },
    );
}

fn profile_request() -> CommonRequestEnvelopeV1 {
    crate::effect_case_lineage_fixture::EffectFixture::new("profile").request
}

pub(super) use crate::local_operation_start_fixture::profile;

pub(super) fn selection() -> crate::resident::OperationCheckpointSelection {
    profile()
        .select(&store_coordinate(), &profile_request())
        .unwrap()
}

pub(super) fn profile_fixture() -> OperationFixture {
    OperationFixture::with_binding(
        "1",
        "operation-1",
        120,
        profile_request().local_request_binding().unwrap(),
    )
}

fn profile_checkpoint(
    sequence: u64,
    prior: Option<(ResidentRecordRefV1, ControlStateCheckpointDigest)>,
) -> ControlStateCheckpointPreimageV1 {
    let mut value = checkpoint_preimage(sequence, prior);
    value.domain_epoch = ControlDomainEpochV1::new(1).unwrap();
    value.checkpoint_profile_sha256 =
        crate::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
    value.checkpoint_domain_id =
        crate::resident::SupportedOperationCheckpointProfile::conflict_domain(&profile_request())
            .unwrap();
    value
}

pub(super) fn profile_checkpoints() -> CheckpointPair {
    let predecessor_ref = record_ref("operation/checkpoint-1");
    let predecessor = admit_initial_checkpoint(&profile_checkpoint(1, None));
    let successor_ref = record_ref("operation/checkpoint-2");
    let successor = admit_successor_checkpoint(
        &profile_checkpoint(
            2,
            Some((
                predecessor_ref.clone(),
                predecessor.canonical_control_checkpoint_sha256().clone(),
            )),
        ),
        &predecessor_ref,
        &predecessor,
    );
    CheckpointPair {
        predecessor_ref,
        predecessor,
        successor_ref,
        successor,
    }
}

#[test]
fn initial_leaves_rebuild_actual_primary_indexes_and_preserve_observation_encoding() {
    use crate::resident::{checkpoint_profile::initial_operation_leaves, record_projection};
    use pale_ports::{
        OperationCustodyIndexMutation as Mutation, OperationCustodyIndexPredecessor as Expected,
    };
    let request = profile_request();
    let fixture = profile_fixture();
    let generation = compiled_generation('8');
    let (_, _, _, fence, active) = initial_active(&generation);
    let session = session_lease(&generation, &active, &fence);
    let request_ref = record_ref("request/initial-profile");
    let request_record = record_projection::request_record(&request_ref, &request).unwrap();
    let handle_coordinate = record_projection::coordinate(
        &fixture.handle_ref,
        fixture.handle.canonical_operation_handle_sha256().as_str(),
    )
    .unwrap();
    // This isolated projection fixture has opaque private bytes, not a session
    // admission; real start/recovery tests supply the session owner's encoding.
    let basis = pale_ports::OperationSessionBasis::new(
        handle_coordinate.clone(),
        request_record.coordinate().clone(),
        session.session_admission_lease_digest().clone(),
        b"{}".to_vec(),
        4096,
    )
    .unwrap();
    let leaves = initial_operation_leaves(
        &selection(),
        (&request_ref, &request),
        (&fixture.handle_ref, &fixture.handle),
        (&fixture.predecessor_ref, &fixture.predecessor),
        &basis,
        &session,
    )
    .unwrap();
    for (handle, request_coordinate, lease_digest) in [
        (
            record_projection::coordinate(
                &record_ref("other-handle"),
                basis.handle().record_sha256().as_str(),
            )
            .unwrap(),
            basis.request().clone(),
            basis.session_lease_digest().clone(),
        ),
        (
            basis.handle().clone(),
            record_projection::coordinate(
                &record_ref("other-request"),
                basis.request().record_sha256().as_str(),
            )
            .unwrap(),
            basis.session_lease_digest().clone(),
        ),
        (
            basis.handle().clone(),
            basis.request().clone(),
            SessionAdmissionLeaseDigest::parse(sha('f')).unwrap(),
        ),
    ] {
        let foreign = pale_ports::OperationSessionBasis::new(
            handle,
            request_coordinate,
            lease_digest,
            basis.canonical_basis_jcs().to_vec(),
            4096,
        )
        .unwrap();
        assert!(
            initial_operation_leaves(
                &selection(),
                (&request_ref, &request),
                (&fixture.handle_ref, &fixture.handle),
                (&fixture.predecessor_ref, &fixture.predecessor),
                &foreign,
                &session,
            )
            .is_err(),
            "projection cannot authenticate an unlike basis header"
        );
    }
    let observation_coordinate = record_projection::coordinate(
        &fixture.predecessor_ref,
        fixture
            .predecessor
            .canonical_operation_observation_sha256()
            .as_str(),
    )
    .unwrap();
    let request_key = crate::resident::operation_custody::operation_request_key(&request).unwrap();
    let handle_key = operation_handle_key(fixture.handle.operation_handle_id()).unwrap();
    // Independent record projections and existing lookup key owners establish
    // the four actual indexes; no expected leaf digest is supplied by the test.
    assert_eq!(
        leaves.indexes().unwrap(),
        vec![
            Mutation::RequestEvidence {
                key: request_key.clone(),
                expected: Expected::Absent,
                successor: request_record.coordinate().clone()
            },
            Mutation::RequestOperation {
                key: request_key,
                expected: Expected::Absent,
                successor: handle_coordinate.clone()
            },
            Mutation::OperationHandle {
                key: handle_key.clone(),
                expected: Expected::Absent,
                successor: handle_coordinate
            },
            Mutation::LatestObservation {
                key: handle_key,
                expected: Expected::Absent,
                successor: observation_coordinate
            },
        ]
    );
    let changes = leaves.changes().unwrap();
    assert_eq!(changes.len(), 4);
    let classes: std::collections::BTreeSet<_> = changes
        .iter()
        .map(|(key, _, _)| key.record_class.as_str())
        .collect();
    assert_eq!(
        classes,
        std::collections::BTreeSet::from([
            "operation-handle",
            "operation-observation",
            "operation-request",
            "operation-session-basis"
        ])
    );
    let append = prepare_operation_observation_append(fixture.transition(), &ResolveStore).unwrap();
    let successor =
        operation_observation_change(&append, &profile_checkpoints().predecessor).unwrap();
    let (_, hashed, value) = changes
        .iter()
        .find(|(key, _, _)| key == &successor.key)
        .unwrap();
    assert_eq!(hashed, &successor.key_sha256);
    assert_eq!(
        value, &successor.predecessor_sha256,
        "initial retained observation is byte-compatible with later replacement"
    );
}

pub(in crate::resident) struct PreparationOnly<F>(
    pub(in crate::resident) F,
    pub(in crate::resident) ControlTransactionSequenceV1,
    AuthorityStoreCoordinate,
);

impl<F> PreparationOnly<F> {
    pub(in crate::resident) fn new(prepare: F, transaction: ControlTransactionSequenceV1) -> Self {
        Self(prepare, transaction, store_coordinate())
    }

    pub(in crate::resident) fn at_store(
        prepare: F,
        transaction: ControlTransactionSequenceV1,
        store: AuthorityStoreCoordinate,
    ) -> Self {
        Self(prepare, transaction, store)
    }
}

impl<F> pale_ports::ControlTransactionPort for PreparationOnly<F> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        // Explicit synthetic phase-fixture frontier, not native store evidence.
        assert_eq!(store, &self.2);
        Ok(pale_ports::ControlTransactionPreparation::new(store.clone(), self.1).unwrap())
    }
}

impl<F> OperationCustodyPort for PreparationOnly<F>
where
    F: FnMut(
        &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError>,
{
    fn read_a2a_task_bindings(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationA2aBindingScope,
        _: Option<&pale_ports::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationA2aBindings, pale_ports::ControlStoreError> {
        panic!("checkpoint preparation must not discover remote tasks")
    }

    fn read_publication_sources(
        &mut self,
        _store: &pale_ports::AuthorityStoreCoordinate,
        _cursor: Option<&pale_ports::OperationPublicationCursor>,
        _maximum_sources: usize,
        _maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        panic!("publication discovery is outside this fixture's tested journey")
    }

    fn read_checkpoint_head(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_core::records::ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<
        Option<(pale_ports::ControlCheckpointHead, pale_ports::ControlRecord)>,
        pale_ports::ControlStoreError,
    > {
        panic!("this owning phase test does not perform locator recovery")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("this owning phase test does not perform membership recovery")
    }

    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        self.0(request)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("preparation must not retain a lease")
    }

    fn read_operation_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        panic!("preparation must not request lease custody")
    }

    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("preparation must not commit")
    }

    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        panic!("preparation must not read back a commit")
    }

    fn read_index(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &OperationCustodyIndexQuery,
        _: usize,
    ) -> Result<pale_ports::OperationCustodyIndexObservation, ControlStoreError> {
        panic!("preparation uses only its bounded native snapshot method")
    }
}

fn limits() -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536).expect("test work limits")
}

// Returns only a representation fixture. The adapter's real proof/overlay
// computation is qualified separately; this test checks the application joins.
fn response(
    request: &OperationAccumulatorPreparation,
    checkpoints: &CheckpointPair,
) -> PreparedOperationAccumulator {
    let batch = checkpoints.physical_batch();
    PreparedOperationAccumulator::new(
        request.clone(),
        batch.state().clone(),
        batch.unresolved().clone(),
    )
    .expect("same predecessor physical roots")
}

#[test]
fn observation_preparation_signs_exact_changes_without_another_storage_call() {
    let fixture = profile_fixture();
    let checkpoints = profile_checkpoints();
    let append = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
        .expect("admitted append");
    let leaf = operation_observation_change(&append, &checkpoints.predecessor)
        .expect("owning logical projection");
    let calls = Cell::new(0);
    let mut store = PreparationOnly::new(
        |request: &OperationAccumulatorPreparation| {
            calls.set(calls.get() + 1);
            assert_eq!(
                request.implementation(),
                pale_ports::AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1
            );
            let AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
                panic!("observation is a successor")
            };
            assert_eq!(prior.store(), &store_coordinate());
            assert_eq!(
                prior.profile_ref(),
                checkpoints.predecessor.checkpoint_profile_ref()
            );
            assert_eq!(
                prior.profile_sha256(),
                checkpoints.predecessor.checkpoint_profile_sha256()
            );
            assert_eq!(
                prior.checkpoint().coordinate().record_ref(),
                &checkpoints.predecessor_ref
            );
            assert_eq!(
                prior.checkpoint().coordinate().record_sha256().as_str(),
                checkpoints
                    .predecessor
                    .canonical_control_checkpoint_sha256()
                    .as_str()
            );
            assert_eq!(request.limits(), limits());
            let [change] = request.state().changes() else {
                panic!("one observation must produce one state replacement")
            };
            assert_eq!(change.key(), &leaf.key_sha256);
            assert_eq!(change.predecessor(), Some(&leaf.predecessor_sha256));
            assert_eq!(change.successor(), Some(&leaf.successor_sha256));
            assert!(request.unresolved().changes().is_empty());
            Ok(response(request, &checkpoints))
        },
        ControlTransactionSequenceV1::new(1).unwrap(),
    );
    let prepared = append
        .prepare_checkpoint(
            selection(),
            &checkpoints.predecessor_ref,
            &checkpoints.predecessor,
            limits(),
            &mut store,
        )
        .expect("prepared roots");
    assert_eq!(calls.get(), 1);
    assert_eq!(
        prepared.state_root(),
        checkpoints.successor.state_root_sha256()
    );
    assert_eq!(
        prepared.unresolved_root(),
        checkpoints.predecessor.unresolved_custody_root_sha256()
    );
    assert_eq!(prepared.observation_key(), &leaf.key);
    let mut successor = profile_checkpoint(
        2,
        Some((
            checkpoints.predecessor_ref.clone(),
            checkpoints
                .predecessor
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
    );
    successor.changed_leaf_keys = vec![prepared.observation_key().clone()];
    let expected = successor.clone();
    let signing_calls = Cell::new(0);
    let (custody, checkpoint) = prepared
        .issue_checkpoint(
            &checkpoints.successor_ref,
            successor,
            &expected.schema_sha256,
            &checkpoint_authority(&expected),
            |domain, preimage, digest| {
                signing_calls.set(signing_calls.get() + 1);
                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                assert_eq!(preimage, &expected);
                assert_eq!(
                    digest,
                    &ControlStateCheckpointDigest::hash_canonical(&expected).unwrap()
                );
                Ok(signature())
            },
        )
        .expect("matching roots sign and attach without another storage call");
    assert_eq!(
        checkpoint.canonical_control_checkpoint_sha256(),
        &ControlStateCheckpointDigest::hash_canonical(&expected).unwrap()
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(signing_calls.get(), 1);
    let mut store = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let committed = commit_observation(custody, &mut store).unwrap();
    assert_eq!(
        committed
            .candidate()
            .checkpoint()
            .successor_record()
            .canonical_record_jcs()
            .as_slice(),
        pale_core::canon::to_canonical_string(&checkpoint)
            .unwrap()
            .as_bytes()
    );
}

#[test]
fn preparation_preserves_limit_storage_and_wrong_predecessor_failures() {
    let fixture = profile_fixture();
    let checkpoints = profile_checkpoints();
    for source in [
        OperationAccumulatorPreparationError::Limit,
        OperationAccumulatorPreparationError::Store(ControlStoreError::new(
            ControlStoreErrorKind::Storage,
        )),
    ] {
        let was_limit = matches!(source, OperationAccumulatorPreparationError::Limit);
        let mut pending = Some(source);
        let mut store = PreparationOnly::new(
            |_: &OperationAccumulatorPreparation| {
                Err(pending.take().expect("one preparation call"))
            },
            ControlTransactionSequenceV1::new(1).unwrap(),
        );
        let error = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
            .expect("append")
            .prepare_checkpoint(
                selection(),
                &checkpoints.predecessor_ref,
                &checkpoints.predecessor,
                limits(),
                &mut store,
            )
            .expect_err("preparation failure does not produce signable roots");
        match error {
            OperationObservationCustodyProjectionError::AccumulatorPreparation(
                OperationAccumulatorPreparationError::Limit,
            ) => assert!(was_limit),
            OperationObservationCustodyProjectionError::AccumulatorPreparation(
                OperationAccumulatorPreparationError::Store(error),
            ) => {
                assert!(!was_limit);
                assert_eq!(error.kind(), ControlStoreErrorKind::Storage);
            }
            other => panic!("unexpected classification: {other:?}"),
        }
        assert!(pending.is_none());
    }
    let mut store = PreparationOnly::new(
        |request: &OperationAccumulatorPreparation| {
            let AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
                panic!("observation is a successor")
            };
            let different = OperationAccumulatorCheckpoint::new(
                prior.store().clone(),
                record_ref("other-profile"),
                prior.profile_sha256().clone(),
                prior.checkpoint().clone(),
                prior.state_root().clone(),
                prior.unresolved_root().clone(),
            );
            let batch = checkpoints.physical_batch();
            Ok(PreparedOperationAccumulator::new(
                OperationAccumulatorPreparation::new(
                    AccumulatorPreparationBasis::Successor(different),
                    request.state().clone(),
                    request.unresolved().clone(),
                    request.limits(),
                    request.implementation(),
                    request.transaction().clone(),
                )
                .unwrap(),
                batch.state().clone(),
                batch.unresolved().clone(),
            )
            .expect("structurally valid foreign response"))
        },
        ControlTransactionSequenceV1::new(1).unwrap(),
    );
    let error = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
        .expect("append")
        .prepare_checkpoint(
            selection(),
            &checkpoints.predecessor_ref,
            &checkpoints.predecessor,
            limits(),
            &mut store,
        )
        .expect_err("foreign preparation response");
    assert!(matches!(
        error,
        OperationObservationCustodyProjectionError::Checkpoint(_)
    ));
}

#[test]
fn observation_preparation_rejects_same_basis_changes_tree_role_and_limits() {
    use pale_ports::{AccumulatorLeafChange, StateLeafChanges, UnresolvedCustodyLeafChanges};
    let fixture = profile_fixture();
    let checkpoints = profile_checkpoints();
    for case in 0..7 {
        let mut store = PreparationOnly::new(
            |original: &OperationAccumulatorPreparation| {
                let mut state = original.state().changes().to_vec();
                let mut unresolved = original.unresolved().changes().to_vec();
                let mut bounds = [1, 1024, 1024, 512, 65_536];
                if case == 0 {
                    let old = &state[0];
                    let other = artifact_digest(if old.successor().unwrap().as_str() == sha('a') {
                        'b'
                    } else {
                        'a'
                    });
                    state[0] = AccumulatorLeafChange::new(
                        old.key().clone(),
                        old.predecessor().cloned(),
                        Some(other),
                    )
                    .unwrap();
                } else if case == 1 {
                    std::mem::swap(&mut state, &mut unresolved);
                } else {
                    bounds[case - 2] += 1;
                }
                let [leaves, reads, writes, positions, proof] = bounds;
                let swapped = OperationAccumulatorPreparation::new(
                    original.basis().clone(),
                    StateLeafChanges::new(state, leaves).unwrap(),
                    UnresolvedCustodyLeafChanges::new(unresolved, leaves).unwrap(),
                    AccumulatorPreparationLimits::new(leaves, reads, writes, positions, proof)
                        .unwrap(),
                    original.implementation(),
                    original.transaction().clone(),
                )
                .unwrap();
                assert_eq!(swapped.basis(), original.basis());
                assert_ne!(&swapped, original);
                Ok(response(&swapped, &checkpoints))
            },
            ControlTransactionSequenceV1::new(1).unwrap(),
        );
        let result = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
            .unwrap()
            .prepare_checkpoint(
                selection(),
                &checkpoints.predecessor_ref,
                &checkpoints.predecessor,
                limits(),
                &mut store,
            );
        assert!(
            matches!(
                result,
                Err(OperationObservationCustodyProjectionError::Checkpoint(_))
            ),
            "case {case}"
        );
        // PreparationOnly panics on all custody methods. Rejection returns no
        // prepared root to the later signing/attachment phase.
    }
}

#[test]
fn prepared_observation_rejects_substitutions_before_signing() {
    let fixture = profile_fixture();
    let checkpoints = profile_checkpoints();
    for change in [
        "state",
        "unresolved",
        "key",
        "count",
        "store",
        "incarnation",
        "domain",
        "epoch",
        "profile-ref",
        "profile-hash",
        "control-schema",
        "principal",
        "predecessor-ref",
        "predecessor-hash",
        "sequence",
        "time",
        "transaction",
        "schema",
        "signing-ref",
        "signing-hash",
        "signing-key",
        "signing-epoch",
    ] {
        let reads = Cell::new(0);
        let signs = Cell::new(0);
        let mut store = PreparationOnly::new(
            |request: &OperationAccumulatorPreparation| {
                reads.set(reads.get() + 1);
                Ok(response(request, &checkpoints))
            },
            ControlTransactionSequenceV1::new(1).unwrap(),
        );
        let prepared = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
            .unwrap()
            .prepare_checkpoint(
                selection(),
                &checkpoints.predecessor_ref,
                &checkpoints.predecessor,
                limits(),
                &mut store,
            )
            .unwrap();
        let mut successor = profile_checkpoint(
            2,
            Some((
                checkpoints.predecessor_ref.clone(),
                checkpoints
                    .predecessor
                    .canonical_control_checkpoint_sha256()
                    .clone(),
            )),
        );
        successor.changed_leaf_keys = vec![prepared.observation_key().clone()];
        let mut authority_source = successor.clone();
        let schema = successor.schema_sha256.clone();
        let retained_failure = match change {
            "state" => {
                successor.state_root_sha256 = artifact_digest('0');
                true
            }
            "unresolved" => {
                successor.unresolved_custody_root_sha256 = artifact_digest('0');
                true
            }
            "key" => {
                successor.changed_leaf_keys[0].logical_key =
                    ControlLogicalKeyV1::parse("another-operation").unwrap();
                true
            }
            "count" => {
                successor.leaf_count = ControlLeafCountV1::new(3).unwrap();
                true
            }
            "store" => {
                successor.control_store_id = ControlStoreIdV1::parse("other-store").unwrap();
                authority_source.control_store_id = successor.control_store_id.clone();
                true
            }
            "incarnation" => {
                successor.control_incarnation_id =
                    ControlIncarnationIdV1::parse("other-incarnation").unwrap();
                authority_source.control_incarnation_id = successor.control_incarnation_id.clone();
                true
            }
            "domain" => {
                successor.checkpoint_domain_id =
                    ControlCheckpointDomainIdV1::parse("operation:other").unwrap();
                authority_source.checkpoint_domain_id = successor.checkpoint_domain_id.clone();
                true
            }
            "epoch" => {
                successor.domain_epoch = ControlDomainEpochV1::new(2).unwrap();
                true
            }
            "profile-ref" => {
                successor.checkpoint_profile_ref = record_ref("other-profile");
                authority_source.checkpoint_profile_ref = successor.checkpoint_profile_ref.clone();
                true
            }
            "profile-hash" => {
                successor.checkpoint_profile_sha256 =
                    CheckpointProfileDigest::parse(sha('0')).unwrap();
                authority_source.checkpoint_profile_sha256 =
                    successor.checkpoint_profile_sha256.clone();
                true
            }
            "control-schema" => {
                successor.control_schema_version = 2;
                true
            }
            "principal" => {
                // A coordinated witness and preimage would agree with each other,
                // but must not replace the retained handle's actual issuer.
                successor.issuer_principal_id =
                    ResidentPrincipalIdV1::parse("other-principal").unwrap();
                authority_source.issuer_principal_id = successor.issuer_principal_id.clone();
                true
            }
            "predecessor-ref" => {
                successor.previous_control_checkpoint_ref = Some(record_ref("other-checkpoint"));
                false
            }
            "predecessor-hash" => {
                successor.previous_control_checkpoint_sha256 =
                    Some(ControlStateCheckpointDigest::parse(sha('0')).unwrap());
                false
            }
            "sequence" => {
                successor.domain_sequence = ControlDomainSequenceV1::new(3).unwrap();
                false
            }
            "time" => {
                successor.issued_at = UnixTimeV1::new(1).unwrap();
                false
            }
            "transaction" => {
                // Even core-valid equality with the predecessor cannot replace
                // this producer's exact retained transaction successor.
                successor.committed_transaction_sequence =
                    ControlTransactionSequenceV1::new(1).unwrap();
                true
            }
            "schema" => {
                successor.schema_sha256 = SchemaDigest::parse(sha('0')).unwrap();
                false
            }
            "signing-ref" => {
                successor.checkpoint_signing_authority_ref = record_ref("other-signer");
                false
            }
            "signing-hash" => {
                successor.checkpoint_signing_authority_sha256 =
                    CheckpointSigningAuthorityDigest::parse(sha('0')).unwrap();
                false
            }
            "signing-key" => {
                successor.issuer_key_id = ResidentKeyIdV1::parse("other-key").unwrap();
                false
            }
            "signing-epoch" => {
                successor.issuer_key_epoch = SigningKeyEpochV1::new(2).unwrap();
                false
            }
            _ => unreachable!(),
        };
        let error = prepared
            .issue_checkpoint(
                &checkpoints.successor_ref,
                successor,
                &schema,
                &checkpoint_authority(&authority_source),
                |_, _, _| {
                    signs.set(signs.get() + 1);
                    Ok(signature())
                },
            )
            .expect_err("substitution must fail before signing");
        if retained_failure {
            assert!(
                matches!(
                    error,
                    OperationObservationCustodyProjectionError::Checkpoint(_)
                ),
                "{change}: {error:?}"
            );
        } else {
            let expected = match change {
                "predecessor-ref" | "predecessor-hash" => (
                    "control checkpoint successor",
                    "exact predecessor reference",
                ),
                "sequence" => ("control checkpoint successor", "exact next domain sequence"),
                "time" | "transaction" => (
                    "control checkpoint successor",
                    "monotone transaction and issue time",
                ),
                _ => (
                    "control state checkpoint",
                    "exact schema and active checkpoint-signing authority",
                ),
            };
            let OperationObservationCustodyProjectionError::Semantic(
                InvalidResidentRecordV1::ContextualAdmission { record, invariant },
            ) = error
            else {
                panic!("{change}: {error:?}");
            };
            assert_eq!((record, invariant), expected, "{change}");
        }
        assert_eq!((reads.get(), signs.get()), (1, 0), "{change}");
        // Every non-preparation custody method on this test port panics.
    }
}

#[test]
fn observation_checkpoint_signer_failure_returns_no_custody() {
    let fixture = profile_fixture();
    let checkpoints = profile_checkpoints();
    let reads = Cell::new(0);
    let signs = Cell::new(0);
    let mut store = PreparationOnly::new(
        |request: &OperationAccumulatorPreparation| {
            reads.set(reads.get() + 1);
            Ok(response(request, &checkpoints))
        },
        ControlTransactionSequenceV1::new(1).unwrap(),
    );
    let prepared = prepare_operation_observation_append(fixture.transition(), &ResolveStore)
        .unwrap()
        .prepare_checkpoint(
            selection(),
            &checkpoints.predecessor_ref,
            &checkpoints.predecessor,
            limits(),
            &mut store,
        )
        .unwrap();
    let mut preimage = profile_checkpoint(
        2,
        Some((
            checkpoints.predecessor_ref.clone(),
            checkpoints
                .predecessor
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
    );
    preimage.changed_leaf_keys = vec![prepared.observation_key().clone()];
    let authority_source = preimage.clone();
    let error = prepared
        .issue_checkpoint(
            &checkpoints.successor_ref,
            preimage,
            &authority_source.schema_sha256,
            &checkpoint_authority(&authority_source),
            |domain, _, _| {
                signs.set(signs.get() + 1);
                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "test checkpoint signer",
                    invariant: "unavailable",
                })
            },
        )
        .expect_err("signer failure cannot return physical custody");
    assert!(matches!(
        error,
        OperationObservationCustodyProjectionError::Semantic(
            InvalidResidentRecordV1::ContextualAdmission {
                record: "test checkpoint signer",
                invariant: "unavailable"
            }
        )
    ));
    assert_eq!((reads.get(), signs.get()), (1, 1));
}
