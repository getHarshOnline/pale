//! Bounded traversal laws independent of native storage implementation.

use super::*;
use crate::local_operation_start_fixture::{
    checkpoint_preimage, record_ref, signature, store_coordinate,
};
use crate::resident::application::tests::empty_application;
use crate::resident::event_custody::local_operation_projection::tests::{
    producer, source, writer, ProjectionFixture, PublicationStore,
};
use crate::resident::journal::tests::MemoryAuthorityStore;
use crate::resident::operation_observation_custody::tests::{
    admit_initial_checkpoint, OperationFixture,
};
use crate::resident::record_projection;
use pale_ports::{
    ControlRecord, ControlRecordBytes, ControlStoreError, ControlStoreErrorKind,
    ControlTransactionPort, ControlTransactionPreparation, OperationA2aBindingCursor,
    OperationA2aBindingScope, OperationA2aBindings, OperationAccumulatorMembership,
    OperationAccumulatorPreparation, OperationAccumulatorPreparationError,
    OperationCustodyCandidate, OperationCustodyCommitOutcome, OperationCustodyIndexObservation,
    OperationCustodyIndexQuery, OperationCustodyReadBackOutcome, OperationCustodyRecordKind,
    OperationPublicationSources, PreparedOperationAccumulator,
    VerifiedOperationAccumulatorMembership,
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

fn sequence(value: u64) -> ControlTransactionSequenceV1 {
    ControlTransactionSequenceV1::new(value).expect("positive transaction")
}

fn opaque_record(schema: &str, reference: &str, body_bytes: usize) -> ControlRecord {
    ControlRecord::new(
        ControlRecordCoordinate::new(
            record_ref(reference),
            ControlRecordDigest::parse("a".repeat(64)).expect("record digest"),
        ),
        SchemaId::parse(schema).expect("schema"),
        SchemaDigest::parse("b".repeat(64)).expect("schema digest"),
        ControlRecordBytes::new(vec![b'x'; body_bytes]).expect("non-empty body"),
    )
}

fn opaque_source(
    kind: OperationPublicationSourceKind,
    transaction: u64,
    label: &str,
    observation_bytes: usize,
    checkpoint_bytes: usize,
) -> OperationPublicationSource {
    let schema = match kind {
        OperationPublicationSourceKind::OperationObservation => OPERATION_OBSERVATION_V1_SCHEMA,
        OperationPublicationSourceKind::A2aTaskObservation => A2A_TASK_OBSERVATION_V1_SCHEMA,
    };
    OperationPublicationSource::new(
        sequence(transaction),
        opaque_record(schema, &format!("{label}/observation"), observation_bytes),
        opaque_record(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            &format!("{label}/checkpoint"),
            checkpoint_bytes,
        ),
    )
    .expect("supported source family")
}

fn source_bytes(source: &OperationPublicationSource) -> usize {
    source.observation().canonical_record_jcs().as_slice().len()
        + source.checkpoint().canonical_record_jcs().as_slice().len()
}

struct SourceStore {
    store: AuthorityStoreCoordinate,
    through: ControlTransactionSequenceV1,
    sources: Vec<OperationPublicationSource>,
    reads: usize,
    requested_body_limits: Vec<usize>,
}

impl SourceStore {
    fn new(sources: Vec<OperationPublicationSource>, through: u64) -> Self {
        Self {
            store: store_coordinate(),
            through: sequence(through),
            sources,
            reads: 0,
            requested_body_limits: Vec::new(),
        }
    }
}

impl ControlTransactionPort for SourceStore {
    fn prepare_transaction(
        &mut self,
        _: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        panic!("source enumeration never prepares a write transaction")
    }
}

impl OperationCustodyPort for SourceStore {
    fn read_a2a_task_bindings(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &OperationA2aBindingScope,
        _: Option<&OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<OperationA2aBindings, ControlStoreError> {
        panic!("A2A binding discovery is outside publication enumeration")
    }

    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<OperationPublicationSources, OperationPublicationReadError> {
        self.reads += 1;
        self.requested_body_limits.push(maximum_body_bytes);
        if store != &self.store || maximum_sources != 1 || maximum_body_bytes == 0 {
            return Err(OperationPublicationReadError::Storage(
                ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired),
            ));
        }
        let start = match cursor {
            None => 0,
            Some(cursor) if cursor.store() == store && cursor.through() == self.through => self
                .sources
                .iter()
                .position(|source| {
                    source.transaction() == cursor.after_transaction()
                        && source.observation().coordinate().record_ref()
                            == cursor.after_reference()
                })
                .map(|index| index + 1)
                .ok_or_else(|| {
                    OperationPublicationReadError::Storage(ControlStoreError::new(
                        ControlStoreErrorKind::RecoveryRequired,
                    ))
                })?,
            Some(_) => {
                return Err(OperationPublicationReadError::Storage(
                    ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired),
                ));
            }
        };
        let Some(source) = self.sources.get(start).cloned() else {
            return Ok(OperationPublicationSources::new(
                self.through,
                Vec::new(),
                None,
            ));
        };
        if source_bytes(&source) > maximum_body_bytes {
            return Err(OperationPublicationReadError::Limit);
        }
        let continuation = (start + 1 < self.sources.len())
            .then(|| {
                OperationPublicationCursor::new(
                    self.store.clone(),
                    self.through,
                    source.transaction(),
                    source.observation().coordinate().record_ref().clone(),
                )
            })
            .transpose()
            .map_err(OperationPublicationReadError::Storage)?;
        Ok(OperationPublicationSources::new(
            self.through,
            vec![source],
            continuation,
        ))
    }

    fn read_checkpoint_head(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<Option<(pale_ports::ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        panic!("source enumeration does not recover checkpoints")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &OperationAccumulatorMembership,
    ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
        panic!("source enumeration does not verify membership")
    }

    fn prepare_accumulator(
        &mut self,
        _: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        panic!("source enumeration does not prepare accumulators")
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("source enumeration does not retain leases")
    }

    fn read_operation_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        panic!("source enumeration does not recover operation records")
    }

    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("source enumeration does not commit operation custody")
    }

    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        panic!("source enumeration does not read back operation custody")
    }

    fn read_index(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &OperationCustodyIndexQuery,
        _: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        panic!("source enumeration does not read operation indexes")
    }
}

#[allow(clippy::too_many_arguments)]
fn recover_consult_publication_source(
    request: &LocalOperationPublicationRecoveryRequest<'_>,
    child: &crate::resident::consultation::tests::IssuedTestChild,
    parent: &IssuedLocalOperation,
    sessions: &crate::resident::process_host::tests::restart::ColdRestartSessions,
    historical: &crate::resident_fixture::RecoveryResolver,
    checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) -> Result<RecoveredOperationControl, LocalOperationPublicationSourceRecoveryError> {
    let parent_handle = parent.handle().1;
    let child_handle = child.operation.handle().1;
    let mut admit_request = |reference: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
        let expected = if reference == &child.request_ref {
            &child.request
        } else if reference == child.parent_request.0.record_ref() {
            &child.parent_request.1
        } else {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "common request envelope",
                invariant: "known retained Consult publication request coordinate",
            });
        };
        let preimage = expected.to_preimage();
        CommonRequestEnvelopeV1::admit_contextual_wire(
            wire,
            &crate::resident::process_host::tests::restart::request_admission_context(&preimage),
            |_, _, actual| {
                if actual != expected.caller_signature() {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "common request envelope",
                        invariant: "retained caller signature",
                    });
                }
                Ok(())
            },
        )
    };
    let mut admit_handle_authority =
        |preimage: &OperationHandlePreimageV1, recovered_parent: Option<&OperationHandleV1>| {
            if preimage.operation_handle_id == *child_handle.operation_handle_id() {
                if recovered_parent != Some(parent_handle) {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "operation handle",
                        invariant: "exact recovered Consult parent",
                    });
                }
            } else if preimage.operation_handle_id != *parent_handle.operation_handle_id()
                || recovered_parent.is_some()
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "operation handle",
                    invariant: "one admitted root Consult parent",
                });
            }
            Ok(())
        };
    let mut verify_handle_signature =
        |_: &[u8],
         _: &OperationHandlePreimageV1,
         _: &OperationHandleDigest,
         actual: &CanonicalSignatureV1| {
            if actual != &signature() {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "operation handle",
                    invariant: "retained handle signature",
                });
            }
            Ok(())
        };
    let mut admit_checkpoint = |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
        ControlStateCheckpointV1::admit_signed_wire(
            wire,
            child.checkpoint.schema_sha256(),
            checkpoint_authority,
            |_, _, _, actual| {
                if actual != &signature() {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "control state checkpoint",
                        invariant: "retained checkpoint signature",
                    });
                }
                Ok(())
            },
        )
    };
    let signing = child.request.to_preimage().signing_authority_binding;
    let mut resolve_observation_signer =
        |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| Ok(signing.clone());
    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
    let mut admit_rotation = |_: &ResidentRecordRefV1,
                              _: &ArtifactDigest,
                              _: &ResidentRecordRefV1,
                              _: &ArtifactDigest| { Ok(()) };
    let mut verify_observation_signature =
        |_: &[u8],
         _: &OperationObservationPreimageV1,
         _: &OperationObservationDigest,
         actual: &CanonicalSignatureV1| {
            if actual != &signature() {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "operation observation",
                    invariant: "retained observation signature",
                });
            }
            Ok(())
        };
    let mut verify_checkpoint_signature =
        |_: &[u8],
         _: &ControlStateCheckpointPreimageV1,
         _: &ControlStateCheckpointDigest,
         actual: &CanonicalSignatureV1| {
            if actual != &signature() {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "control state checkpoint",
                    invariant: "retained checkpoint signature",
                });
            }
            Ok(())
        };
    let limits = request.limits();
    let selected_digest =
        OperationObservationDigest::parse(request.observation().record_sha256().as_str())
            .map_err(|_| LocalOperationPublicationSourceRecoveryError::Unavailable)?;
    let profile = crate::local_operation_start_fixture::profile();
    crate::resident::recover_operation_control(
        crate::resident::OperationControlRecoveryInput {
            operation: crate::resident::LocalOperationHandleRecoveryInput {
                context: crate::resident::LocalOperationRecoveryContext {
                    store: request.store(),
                    profile: &profile,
                    handle_signing_authority: &signing,
                    checkpoint_authority,
                    historical,
                    maximum_record_bytes: 1 << 20,
                    maximum_operation_lease_bytes: 1 << 20,
                    maximum_session_lease_bytes: 1 << 20,
                    membership_limits: limits.membership(),
                    observation_limits: limits.observation(),
                    checkpoint_limits: limits.checkpoint(),
                },
                expected_handle: request.operation_handle(),
                maximum_parent_depth: limits.maximum_parent_depth(),
            },
            observation: crate::resident::OperationObservationSelection::HistoryMember {
                reference: request.observation().record_ref(),
                digest: &selected_digest,
            },
            observation_limits: limits.observation(),
            checkpoint_limits: limits.checkpoint(),
            membership_limits: limits.membership(),
        },
        custody,
        sessions,
        anchor,
        crate::resident::OperationControlRecoveryAdmission {
            admit_request: &mut admit_request,
            admit_handle_authority: &mut admit_handle_authority,
            verify_handle_signature: &mut verify_handle_signature,
            admit_checkpoint: &mut admit_checkpoint,
            resolve_observation_signer: &mut resolve_observation_signer,
            admit_observation_state: &mut admit_observation_state,
            admit_rotation: &mut admit_rotation,
            verify_observation_signature: &mut verify_observation_signature,
            verify_checkpoint_signature: &mut verify_checkpoint_signature,
        },
    )
    .map_err(Into::into)
}

fn sweep_limits(
    pages: usize,
    sources: usize,
    body_bytes: usize,
) -> LocalOperationPublicationSweepLimits {
    LocalOperationPublicationSweepLimits::new(pages, sources, body_bytes, 64 * 1024)
        .expect("positive sweep limits")
}

fn recovery_limits() -> LocalOperationPublicationRecoveryLimits {
    LocalOperationPublicationRecoveryLimits::new(
        7,
        OperationObservationRecoveryLimits::new(11, 12_000, 13_000).expect("observation limits"),
        OperationCheckpointAncestryLimits::new(17, 18_000, 19).expect("checkpoint limits"),
        AccumulatorMembershipLimits::new(23, 29, 31_000).expect("membership limits"),
    )
}

fn assert_complete(
    progress: LocalOperationPublicationSweepProgress,
    expected_through: u64,
    expected_sources: usize,
) {
    let LocalOperationPublicationSweepProgress::SupportedLocalPrefixComplete {
        store,
        through,
        sources_examined,
    } = progress
    else {
        panic!("expected exact supported-local-prefix completion")
    };
    assert_eq!(store, store_coordinate());
    assert_eq!(through, sequence(expected_through));
    assert_eq!(sources_examined, expected_sources);
}

#[test]
fn pending_source_blocks_cursor_advance_and_preserves_transaction_order() {
    let first = opaque_source(
        OperationPublicationSourceKind::OperationObservation,
        1,
        "first",
        32,
        32,
    );
    let second = opaque_source(
        OperationPublicationSourceKind::OperationObservation,
        2,
        "second",
        32,
        32,
    );
    let mut port = SourceStore::new(vec![first, second], 2);
    let mut attempt = LocalOperationPublicationSweepAttempt::new(store_coordinate());
    let limits = sweep_limits(2, 2, 256);

    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        LocalOperationPublicationSweepProgress::ContinueScan
    ));
    let PublicationSweepPosition::Pending(pending) = &attempt.position else {
        panic!("first source remains pending")
    };
    assert_eq!(pending.source.transaction(), sequence(1));
    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port),
        Err(LocalOperationPublicationSweepError::InvalidSource)
    ));
    assert_eq!(port.reads, 1, "a pending source prevents another page read");

    finish_pending_source(&mut attempt);
    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        LocalOperationPublicationSweepProgress::ContinueScan
    ));
    let PublicationSweepPosition::Pending(pending) = &attempt.position else {
        panic!("second source follows only after the first completes")
    };
    assert_eq!(pending.source.transaction(), sequence(2));
}

#[test]
fn empty_prefix_is_exact_and_completion_is_idempotent_at_the_page_ceiling() {
    let mut port = SourceStore::new(Vec::new(), 9);
    let mut attempt = LocalOperationPublicationSweepAttempt::new(store_coordinate());
    let limits = sweep_limits(1, 1, 1);

    assert_complete(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        9,
        0,
    );
    assert_complete(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        9,
        0,
    );
    assert_eq!(
        port.reads, 1,
        "terminal replay performs no second source read"
    );
}

#[test]
fn remote_sources_hold_the_cursor_until_their_projection_finishes() {
    let first = opaque_source(
        OperationPublicationSourceKind::A2aTaskObservation,
        1,
        "remote-first",
        19,
        23,
    );
    let second = opaque_source(
        OperationPublicationSourceKind::A2aTaskObservation,
        2,
        "remote-second",
        29,
        31,
    );
    let exact_bytes = source_bytes(&first) + source_bytes(&second);
    let mut port = SourceStore::new(vec![first, second], 2);
    let mut attempt = LocalOperationPublicationSweepAttempt::new(store_coordinate());
    let limits = sweep_limits(2, 2, exact_bytes);

    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        LocalOperationPublicationSweepProgress::ContinueScan
    ));
    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port),
        Err(LocalOperationPublicationSweepError::InvalidSource)
    ));
    finish_pending_source(&mut attempt);
    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        LocalOperationPublicationSweepProgress::ContinueScan
    ));
    finish_pending_source(&mut attempt);
    assert_complete(completion_or_continue(&attempt).unwrap(), 2, 2);
    assert_complete(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        2,
        2,
    );
    assert_eq!(port.reads, 2);
}

#[test]
fn aggregate_page_source_and_body_exhaustion_are_not_completion() {
    let remote = |transaction, label| {
        opaque_source(
            OperationPublicationSourceKind::A2aTaskObservation,
            transaction,
            label,
            32,
            32,
        )
    };
    for limits in [sweep_limits(1, 2, 256), sweep_limits(2, 1, 256)] {
        let mut port = SourceStore::new(
            vec![remote(1, "bounded-first"), remote(2, "bounded-second")],
            2,
        );
        let mut attempt = LocalOperationPublicationSweepAttempt::new(store_coordinate());
        assert!(matches!(
            load_next_source(&mut attempt, limits, &mut port).unwrap(),
            LocalOperationPublicationSweepProgress::ContinueScan
        ));
        finish_pending_source(&mut attempt);
        assert!(matches!(
            load_next_source(&mut attempt, limits, &mut port).unwrap(),
            LocalOperationPublicationSweepProgress::BudgetExhausted
        ));
    }

    let first = remote(1, "byte-first");
    let first_bytes = source_bytes(&first);
    let mut port = SourceStore::new(vec![first, remote(2, "byte-second")], 2);
    let mut attempt = LocalOperationPublicationSweepAttempt::new(store_coordinate());
    let limits = sweep_limits(2, 2, first_bytes);
    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        LocalOperationPublicationSweepProgress::ContinueScan
    ));
    finish_pending_source(&mut attempt);
    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        LocalOperationPublicationSweepProgress::BudgetExhausted
    ));
}

#[test]
fn aggregate_body_budget_can_span_multiple_ordinary_pages_above_eight_mebibytes() {
    let body = MAX_OPERATION_PUBLICATION_BODY_BYTES / 2 + 4096;
    let first = opaque_source(
        OperationPublicationSourceKind::OperationObservation,
        1,
        "large-first",
        body,
        1,
    );
    let second = opaque_source(
        OperationPublicationSourceKind::OperationObservation,
        2,
        "large-second",
        body,
        1,
    );
    let aggregate = source_bytes(&first) + source_bytes(&second);
    assert!(aggregate > MAX_OPERATION_PUBLICATION_BODY_BYTES);
    let mut port = SourceStore::new(vec![first, second], 2);
    let mut attempt = LocalOperationPublicationSweepAttempt::new(store_coordinate());
    let limits = sweep_limits(2, 2, aggregate);

    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        LocalOperationPublicationSweepProgress::ContinueScan
    ));
    finish_pending_source(&mut attempt);
    assert!(matches!(
        load_next_source(&mut attempt, limits, &mut port).unwrap(),
        LocalOperationPublicationSweepProgress::ContinueScan
    ));
    assert_eq!(attempt.source_body_bytes_examined(), aggregate);
    assert_eq!(
        port.requested_body_limits,
        vec![MAX_OPERATION_PUBLICATION_BODY_BYTES, body + 1]
    );
}

#[test]
fn recovery_request_preserves_exact_store_handle_observation_and_limits() {
    let fixture = OperationFixture::exact();
    let checkpoint_ref = record_ref("sweep/checkpoint-1");
    let checkpoint = admit_initial_checkpoint(&checkpoint_preimage(1, None));
    let observation = record_projection::record(
        record_projection::coordinate(
            fixture.predecessor().0,
            fixture
                .predecessor()
                .1
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .unwrap(),
        fixture.predecessor().1,
    )
    .unwrap();
    let checkpoint_record = record_projection::record(
        record_projection::coordinate(
            &checkpoint_ref,
            checkpoint.canonical_control_checkpoint_sha256().as_str(),
        )
        .unwrap(),
        &checkpoint,
    )
    .unwrap();
    let source = OperationPublicationSource::new(sequence(1), observation, checkpoint_record)
        .expect("typed source");
    let selected_limits = recovery_limits();
    let store = store_coordinate();
    let request = recovery_request(&store, &source, selected_limits).unwrap();

    assert_eq!(request.store(), &store_coordinate());
    assert_eq!(request.operation_handle().record_ref(), fixture.handle().0);
    assert_eq!(
        request.operation_handle().record_sha256().as_str(),
        fixture
            .handle()
            .1
            .canonical_operation_handle_sha256()
            .as_str()
    );
    assert_eq!(request.observation(), source.observation().coordinate());
    assert_eq!(request.limits().maximum_parent_depth(), 7);
    assert_eq!(
        request.limits().observation(),
        OperationObservationRecoveryLimits::new(11, 12_000, 13_000).unwrap()
    );
    assert_eq!(
        request.limits().checkpoint(),
        OperationCheckpointAncestryLimits::new(17, 18_000, 19).unwrap()
    );
    assert_eq!(
        request.limits().membership(),
        AccumulatorMembershipLimits::new(23, 29, 31_000).unwrap()
    );
}

#[test]
fn a2a_recovery_request_preserves_binding_observation_checkpoint_and_limits() {
    let schema = pale_contracts::record_schema_identity(A2A_TASK_OBSERVATION_V1_SCHEMA_ID).unwrap();
    let binding_ref = record_ref("sweep/a2a-binding");
    let binding_digest = A2aTaskBindingDigest::parse("c".repeat(64)).unwrap();
    let observation_preimage = A2aTaskObservationPreimageV1 {
        schema_id: schema.schema.clone(),
        schema_sha256: schema.schema_sha256.clone(),
        task_binding_id: A2aTaskBindingIdV1::parse("binding-1").unwrap(),
        task_binding_ref: binding_ref.clone(),
        task_binding_sha256: binding_digest.clone(),
        revision: A2aTaskObservationRevisionV1::new(1).unwrap(),
        predecessor_ref: None,
        predecessor_sha256: None,
        source: A2aTaskObservationSourceV1::InitialSendMessageResponse {
            evidence: ImmutableRecordReferenceV1 {
                record_ref: record_ref("sweep/a2a-response"),
                record_sha256: ArtifactDigest::parse("d".repeat(64)).unwrap(),
            },
        },
        remote_state: A2aRemoteTaskStateV1::Submitted,
        reconciliation: None,
        observed_at: UnixTimeV1::new(20).unwrap(),
        deployment_runtime_authority_ref: record_ref("sweep/runtime-authority"),
        deployment_runtime_authority_sha256: ArtifactDigest::parse("e".repeat(64)).unwrap(),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").unwrap(),
        issuer_key_epoch: SigningKeyEpochV1::new(1).unwrap(),
    };
    let observation_digest =
        A2aTaskObservationDigest::hash_canonical(&observation_preimage).unwrap();
    let mut observation_value = serde_json::to_value(&observation_preimage).unwrap();
    let observation_object = observation_value.as_object_mut().unwrap();
    observation_object.insert(
        "canonical_a2a_task_observation_sha256".into(),
        serde_json::to_value(&observation_digest).unwrap(),
    );
    observation_object.insert(
        "signature".into(),
        serde_json::to_value(signature()).unwrap(),
    );
    let observation: A2aTaskObservationWireV1 = serde_json::from_value(observation_value).unwrap();
    let observation_record = ControlRecord::new(
        record_projection::coordinate(
            &record_ref("sweep/a2a-observation"),
            observation_digest.as_str(),
        )
        .unwrap(),
        schema.schema,
        schema.schema_sha256,
        ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&observation)
                .unwrap()
                .into_bytes(),
        )
        .unwrap(),
    );
    let checkpoint = opaque_record(
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        "sweep/a2a-checkpoint",
        17,
    );
    let source =
        OperationPublicationSource::new(sequence(2), observation_record, checkpoint).unwrap();
    let local = recovery_limits();
    let limits = A2aTaskPublicationRecoveryLimits::new(
        local,
        A2aTaskRecoveryLimits::new(7, 8_000, 9_000, 10, 11_000).unwrap(),
    );
    let store = store_coordinate();
    let request = a2a_recovery_request(&store, &source, limits).unwrap();

    assert_eq!(request.store(), &store);
    assert_eq!(request.task_binding().record_ref(), &binding_ref);
    assert_eq!(
        request.task_binding().record_sha256().as_str(),
        binding_digest.as_str()
    );
    assert_eq!(request.observation(), source.observation().coordinate());
    assert_eq!(request.checkpoint(), Some(source.checkpoint().coordinate()));
    assert_eq!(request.limits().local().maximum_parent_depth(), 7);
}

#[test]
fn store_mismatch_stops_before_reads_recovery_signing_or_writes() {
    let authority = MemoryAuthorityStore::default();
    let mut application = empty_application(&authority, RingCursorV1::new(0));
    let mut event_port = PublicationStore::new(&authority);
    let mut source_port = SourceStore::new(Vec::new(), 1);
    let other_store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("other-store").unwrap(),
        ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
    );
    let mut attempt = LocalOperationPublicationSweepAttempt::new(other_store);
    let config = ProjectionFixture::new();
    let producer = producer();
    let writer = writer();
    let producer_ref = record_ref("producer-allocation-1");
    let writer_ref = record_ref("writer-fence-1");
    let verifier = |_: &super::super::EventPublicationRaceClosure<'_>| Ok(());
    let signed = Cell::new(0);
    let recovered = Cell::new(0);
    let mut sign_occurrence =
        |_: &[u8], _: &InteractionOccurrencePreimageV1, _: &InteractionOccurrenceDigest| {
            signed.set(signed.get() + 1);
            Ok(signature())
        };
    let mut sign_edge = |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| {
        signed.set(signed.get() + 1);
        Ok(signature())
    };
    let mut sign_frontier =
        |_: &[u8], _: &ReconciliationFrontierPreimageV1, _: &ReconciliationFrontierDigest| {
            signed.set(signed.get() + 1);
            Ok(signature())
        };
    let mut issuance = LocalOperationPublicationIssuance {
        sign_occurrence: &mut sign_occurrence,
        sign_edge: &mut sign_edge,
        sign_frontier: &mut sign_frontier,
    };
    let historical = config.contexts().occurrence.signing_authority.clone();
    let mut admit_event_source = |_: &EventReferencePreimageV1| Ok(());
    let mut resolve_occurrence = |_: &ControlRecord| Ok(historical.clone());
    let mut resolve_edge = |_: &ControlRecord| Ok(historical.clone());
    let mut resolve_frontier = |_: &ControlRecord| Ok(historical.clone());
    let mut verify_occurrence =
        |_: &[u8],
         _: &InteractionOccurrencePreimageV1,
         _: &InteractionOccurrenceDigest,
         _: &CanonicalSignatureV1| { Ok(()) };
    let mut verify_edge = |_: &[u8],
                           _: &GenealogyEdgePreimageV1,
                           _: &GenealogyEdgeDigest,
                           _: &CanonicalSignatureV1| { Ok(()) };
    let mut verify_frontier = |_: &[u8],
                               _: &ReconciliationFrontierPreimageV1,
                               _: &ReconciliationFrontierDigest,
                               _: &CanonicalSignatureV1| { Ok(()) };
    let mut readmission = LocalOperationPublicationReadmission {
        admit_event_source: &mut admit_event_source,
        resolve_occurrence_authority: &mut resolve_occurrence,
        verify_occurrence_signature: &mut verify_occurrence,
        resolve_edge_authority: &mut resolve_edge,
        verify_edge_signature: &mut verify_edge,
        resolve_frontier_authority: &mut resolve_frontier,
        verify_frontier_signature: &mut verify_frontier,
    };
    let mut recover = |_: &LocalOperationPublicationRecoveryRequest<'_>| {
        recovered.set(recovered.get() + 1);
        Err(LocalOperationPublicationSourceRecoveryError::Unavailable)
    };
    let mut recover_a2a = |_: &A2aTaskPublicationRecoveryRequest<'_>| {
        Err(A2aTaskPublicationSourceRecoveryError::Unavailable)
    };
    let a2a_limits = A2aTaskPublicationRecoveryLimits::new(
        recovery_limits(),
        A2aTaskRecoveryLimits::new(8, 65_536, 1 << 20, 16, 1 << 20).unwrap(),
    );

    let result = sweep_local_operation_publications(
        &mut attempt,
        sweep_limits(1, 1, 1),
        recovery_limits(),
        a2a_limits,
        &mut source_port,
        &mut application,
        &mut event_port,
        &mut recover,
        &mut recover_a2a,
        LocalOperationPublicationCurrent {
            producer_allocation_ref: &producer_ref,
            producer: &producer,
            writer_fence_ref: &writer_ref,
            writer_fence: &writer,
            publication_state: EventPublicationState::Active,
            race_verifier: &verifier,
            signing_contexts: config.contexts(),
        },
        &mut issuance,
        &mut readmission,
    );
    assert!(matches!(
        result,
        Err(LocalOperationPublicationSweepError::Association)
    ));
    assert_eq!(source_port.reads, 0);
    assert_eq!(recovered.get(), 0);
    assert_eq!(signed.get(), 0);
    assert!(application.journal_head().is_none());
}

#[test]
fn child_sweep_readmits_parent_and_retries_one_interrupted_structural_edge() {
    let (mut generations, profile) =
        crate::resident::operation_observation_custody::tests::local_start::
            host_source_generation_copies(1);
    let generation = generations.pop().expect("one source generation");
    crate::resident::consultation::tests::with_issued_delegate_child_named(
        &generation,
        &profile,
        "publication-sweep-lineage",
        |child,
         parent,
         parent_candidate,
         parent_checkpoint,
         lease_custody,
         _generation,
         parents| {
            let observation_schema =
                pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID)
                    .expect("observation schema");
            let admit_initial = |record: &ControlRecord, operation: &IssuedLocalOperation| {
                let wire: OperationObservationWireV1 = serde_json::from_value(
                    pale_core::canon::parse_strict(record.canonical_record_jcs().as_slice())
                        .expect("canonical initial observation"),
                )
                .expect("initial observation wire");
                OperationObservationV1::admit_contextual_wire(
                    wire,
                    &OperationObservationAdmissionContextV1 {
                        signed: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema.schema_sha256,
                            signing_authority: parents.signing,
                        },
                        handle_ref: operation.handle().0,
                        handle: operation.handle().1,
                        predecessor: None,
                    },
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _, _| Ok(()),
                )
                .expect("admitted initial observation")
            };

            let parent_record = parent_candidate
                .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                .expect("parent initial observation")
                .clone();
            let parent_observation = admit_initial(&parent_record, parent);
            let parent_observation_ref = parent_record.coordinate().record_ref().clone();
            let parent_checkpoint_ref = parent_candidate
                .checkpoint()
                .successor_record()
                .coordinate()
                .record_ref()
                .clone();
            let child_observation = admit_initial(&child.initial_record, &child.operation);
            let child_observation_ref = child.initial_record.coordinate().record_ref().clone();

            let parent_source = source(
                (&parent_observation_ref, &parent_observation),
                (&parent_checkpoint_ref, &parent_checkpoint),
            );
            let child_source = source(
                (&child_observation_ref, &child_observation),
                (&child.checkpoint_ref, &child.checkpoint),
            );
            assert!(parent_source.transaction() < child_source.transaction());
            let through = child_source.transaction().get();
            let mut source_port = SourceStore::new(vec![parent_source, child_source], through);

            let parent_references = LocalOperationPublicationReferences::derive_for_operation(
                &store_coordinate(),
                &parent_observation_ref,
                &parent_observation,
                parent.handle().1,
            )
            .expect("parent references");
            let child_references = LocalOperationPublicationReferences::derive_for_operation(
                &store_coordinate(),
                &child_observation_ref,
                &child_observation,
                child.operation.handle().1,
            )
            .expect("child references");
            assert!(parent_references.edge().is_none());
            assert!(child_references.edge().is_some());

            let recovered_store = child.initial_candidate.store().clone();
            let (sessions, historical) =
                crate::resident::process_host::tests::restart::cold_restart_sessions(
                    parents,
                    &recovered_store,
                );
            let checkpoint_authority =
                crate::resident::process_host::tests::a2a_cancel_task::checkpoint_signer(
                    &child.checkpoint,
                );
            let supported_profile = crate::local_operation_start_fixture::profile();
            let recovery_calls = Cell::new(0);
            crate::resident::operation_observation_custody::tests::native_execution::with_service_custody_faults(
                &parent_checkpoint,
                Some(&child.initial_candidate),
                vec![parent_candidate],
                None,
                vec![lease_custody.parent, lease_custody.child],
                0,
                0,
                0,
                0,
                0,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                Vec::new(),
                false,
                None,
                std::rc::Rc::new(RefCell::new(None)),
                |custody, anchor| {
            let mut recover_local = |request: &LocalOperationPublicationRecoveryRequest<'_>| {
                recovery_calls.set(recovery_calls.get() + 1);
                let (selected_ref, selected_observation) =
                    if request.observation() == parent_record.coordinate() {
                        (&parent_observation_ref, &parent_observation)
                    } else if request.observation() == child.initial_record.coordinate() {
                        (&child_observation_ref, &child_observation)
                    } else {
                        return Err(LocalOperationPublicationSourceRecoveryError::Unavailable);
                    };
                let limits = request.limits();
                let parent_handle = parent.handle().1;
                let child_handle = child.operation.handle().1;
                let mut admit_request =
                    |reference: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
                        let expected = if reference == &child.request_ref {
                            &child.request
                        } else if reference == child.parent_request.0.record_ref() {
                            &child.parent_request.1
                        } else {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "common request envelope",
                                invariant: "known retained request coordinate",
                            });
                        };
                        let preimage = expected.to_preimage();
                        CommonRequestEnvelopeV1::admit_contextual_wire(
                            wire,
                            &crate::resident::process_host::tests::restart::request_admission_context(
                                &preimage,
                            ),
                            |_, _, actual| {
                                if actual != expected.caller_signature() {
                                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                                        record: "common request envelope",
                                        invariant: "retained caller signature",
                                    });
                                }
                                Ok(())
                            },
                        )
                    };
                let mut admit_handle_authority =
                    |preimage: &OperationHandlePreimageV1,
                     recovered_parent: Option<&OperationHandleV1>| {
                        if preimage.operation_handle_id == *child_handle.operation_handle_id() {
                            if recovered_parent != Some(parent_handle) {
                                return Err(InvalidResidentRecordV1::ContextualAdmission {
                                    record: "operation handle",
                                    invariant: "exact recovered Delegate parent",
                                });
                            }
                        } else if preimage.operation_handle_id
                            != *parent_handle.operation_handle_id()
                            || recovered_parent.is_some()
                        {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "operation handle",
                                invariant: "one admitted Consult parent",
                            });
                        }
                        Ok(())
                    };
                let mut verify_handle_signature =
                    |_: &[u8],
                     _: &OperationHandlePreimageV1,
                     _: &OperationHandleDigest,
                     actual: &CanonicalSignatureV1| {
                        if actual != &signature() {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "operation handle",
                                invariant: "retained handle signature",
                            });
                        }
                        Ok(())
                    };
                let mut admit_checkpoint =
                    |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                        ControlStateCheckpointV1::admit_signed_wire(
                            wire,
                            child.checkpoint.schema_sha256(),
                            &checkpoint_authority,
                            |_, _, _, actual| {
                                if actual != &signature() {
                                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                                        record: "control state checkpoint",
                                        invariant: "retained checkpoint signature",
                                    });
                                }
                                Ok(())
                            },
                        )
                    };
                let mut resolve_observation_signer =
                    |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                        Ok(parents.signing.clone())
                    };
                let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
                let mut admit_rotation =
                    |_: &ResidentRecordRefV1,
                     _: &ArtifactDigest,
                     _: &ResidentRecordRefV1,
                     _: &ArtifactDigest| { Ok(()) };
                let mut verify_observation_signature =
                    |_: &[u8],
                     _: &OperationObservationPreimageV1,
                     _: &OperationObservationDigest,
                     actual: &CanonicalSignatureV1| {
                        if actual != &signature() {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "operation observation",
                                invariant: "retained observation signature",
                            });
                        }
                        Ok(())
                    };
                let mut verify_checkpoint_signature =
                    |_: &[u8],
                     _: &ControlStateCheckpointPreimageV1,
                     _: &ControlStateCheckpointDigest,
                     actual: &CanonicalSignatureV1| {
                        if actual != &signature() {
                            return Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "control state checkpoint",
                                invariant: "retained checkpoint signature",
                            });
                        }
                        Ok(())
                    };
                let recovered = crate::resident::recover_operation_control(
                    crate::resident::OperationControlRecoveryInput {
                        operation: crate::resident::LocalOperationHandleRecoveryInput {
                            context: crate::resident::LocalOperationRecoveryContext {
                                store: request.store(),
                                profile: &supported_profile,
                                handle_signing_authority: parents.signing,
                                checkpoint_authority: &checkpoint_authority,
                                historical: &historical,
                                maximum_record_bytes: 1 << 20,
                                maximum_operation_lease_bytes: 1 << 20,
                                maximum_session_lease_bytes: 1 << 20,
                                membership_limits: limits.membership(),
                                observation_limits: limits.observation(),
                                checkpoint_limits: limits.checkpoint(),
                            },
                            expected_handle: request.operation_handle(),
                            maximum_parent_depth: limits.maximum_parent_depth(),
                        },
                        observation: crate::resident::OperationObservationSelection::HistoryMember {
                            reference: selected_ref,
                            digest: selected_observation.canonical_operation_observation_sha256(),
                        },
                        observation_limits: limits.observation(),
                        checkpoint_limits: limits.checkpoint(),
                        membership_limits: limits.membership(),
                    },
                    custody,
                    &sessions,
                    anchor,
                    crate::resident::OperationControlRecoveryAdmission {
                        admit_request: &mut admit_request,
                        admit_handle_authority: &mut admit_handle_authority,
                        verify_handle_signature: &mut verify_handle_signature,
                        admit_checkpoint: &mut admit_checkpoint,
                        resolve_observation_signer: &mut resolve_observation_signer,
                        admit_observation_state: &mut admit_observation_state,
                        admit_rotation: &mut admit_rotation,
                        verify_observation_signature: &mut verify_observation_signature,
                        verify_checkpoint_signature: &mut verify_checkpoint_signature,
                    },
                )?;
                if request.observation() == child.initial_record.coordinate() {
                    assert!(recovered.operation().initial_observation().is_some());
                    assert!(recovered
                        .operation()
                        .recovered_parent()
                        .is_some_and(|value| value.initial_observation().is_some()));
                }
                Ok(recovered)
            };
            let a2a_recoveries = Cell::new(0);
            let mut recover_a2a = |_: &A2aTaskPublicationRecoveryRequest<'_>| {
                a2a_recoveries.set(a2a_recoveries.get() + 1);
                Err(A2aTaskPublicationSourceRecoveryError::Unavailable)
            };

            let authority = MemoryAuthorityStore::default();
            let mut application = empty_application(&authority, RingCursorV1::new(0));
            let mut event_port = PublicationStore::new(&authority);
            let config = ProjectionFixture::new();
            let producer = producer();
            let writer = writer();
            let producer_ref = record_ref("sweep/producer-allocation");
            let writer_ref = record_ref("sweep/writer-fence");
            let verifier = |_: &super::super::EventPublicationRaceClosure<'_>| Ok(());
            let occurrence_reads = RefCell::new(BTreeMap::<ResidentRecordRefV1, usize>::new());
            let occurrence_signatures = Cell::new(0);
            let edge_signatures = Cell::new(0);
            let frontier_signatures = Cell::new(0);
            let mut sign_occurrence =
                |_: &[u8], _: &InteractionOccurrencePreimageV1, _: &InteractionOccurrenceDigest| {
                    occurrence_signatures.set(occurrence_signatures.get() + 1);
                    Ok(signature())
                };
            let mut sign_edge = |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| {
                edge_signatures.set(edge_signatures.get() + 1);
                Ok(signature())
            };
            let mut sign_frontier =
                |_: &[u8],
                 _: &ReconciliationFrontierPreimageV1,
                 _: &ReconciliationFrontierDigest| {
                    frontier_signatures.set(frontier_signatures.get() + 1);
                    Ok(signature())
                };
            let mut issuance = LocalOperationPublicationIssuance {
                sign_occurrence: &mut sign_occurrence,
                sign_edge: &mut sign_edge,
                sign_frontier: &mut sign_frontier,
            };
            let historical = parents.signing.clone();
            let mut admit_event_source = |_: &EventReferencePreimageV1| Ok(());
            let mut resolve_occurrence = |record: &ControlRecord| {
                *occurrence_reads
                    .borrow_mut()
                    .entry(record.coordinate().record_ref().clone())
                    .or_default() += 1;
                Ok(historical.clone())
            };
            let mut resolve_edge = |_: &ControlRecord| Ok(historical.clone());
            let mut resolve_frontier = |_: &ControlRecord| Ok(historical.clone());
            let mut verify_occurrence =
                |_: &[u8],
                 _: &InteractionOccurrencePreimageV1,
                 _: &InteractionOccurrenceDigest,
                 _: &CanonicalSignatureV1| { Ok(()) };
            let mut verify_edge =
                |_: &[u8],
                 _: &GenealogyEdgePreimageV1,
                 _: &GenealogyEdgeDigest,
                 _: &CanonicalSignatureV1| { Ok(()) };
            let mut verify_frontier =
                |_: &[u8],
                 _: &ReconciliationFrontierPreimageV1,
                 _: &ReconciliationFrontierDigest,
                 _: &CanonicalSignatureV1| { Ok(()) };
            let mut readmission = LocalOperationPublicationReadmission {
                admit_event_source: &mut admit_event_source,
                resolve_occurrence_authority: &mut resolve_occurrence,
                verify_occurrence_signature: &mut verify_occurrence,
                resolve_edge_authority: &mut resolve_edge,
                verify_edge_signature: &mut verify_edge,
                resolve_frontier_authority: &mut resolve_frontier,
                verify_frontier_signature: &mut verify_frontier,
            };
            let limits = sweep_limits(8, 8, 1 << 20);
            let local_limits = recovery_limits();
            let a2a_limits = A2aTaskPublicationRecoveryLimits::new(
                local_limits,
                A2aTaskRecoveryLimits::new(8, 65_536, 1 << 20, 16, 1 << 20).unwrap(),
            );
            macro_rules! sweep {
                ($attempt:expr) => {
                    sweep_local_operation_publications(
                        $attempt,
                        limits,
                        local_limits,
                        a2a_limits,
                        &mut source_port,
                        &mut application,
                        &mut event_port,
                        &mut recover_local,
                        &mut recover_a2a,
                        LocalOperationPublicationCurrent {
                            producer_allocation_ref: &producer_ref,
                            producer: &producer,
                            writer_fence_ref: &writer_ref,
                            writer_fence: &writer,
                            publication_state: EventPublicationState::Active,
                            race_verifier: &verifier,
                            signing_contexts: config.contexts_with(parents.signing),
                        },
                        &mut issuance,
                        &mut readmission,
                    )
                };
            }

            let mut first_attempt = LocalOperationPublicationSweepAttempt::new(store_coordinate());
            for expected in [
                LocalOperationPublicationStage::Event,
                LocalOperationPublicationStage::Occurrence,
                LocalOperationPublicationStage::ReconciliationFrontier,
            ] {
                assert!(matches!(
                    sweep!(&mut first_attempt).expect("parent publication stage"),
                    LocalOperationPublicationSweepProgress::StageEstablished { stage, .. }
                        if stage == expected
                ));
            }
            assert_eq!(
                event_port.record_count_for_schema(GENEALOGY_EDGE_V1_SCHEMA),
                0,
                "an unparented root retains no structural edge"
            );
            assert!(matches!(
                sweep!(&mut first_attempt).expect("finish parent source"),
                LocalOperationPublicationSweepProgress::ContinueScan
            ));

            let (parent_occurrence_query, parent_occurrence_record) = event_port
                .remove_only_occurrence_index()
                .expect("one parent occurrence index");
            let journal_before_parent_failures = authority.entry_count();
            let missing = sweep!(&mut first_attempt);
            assert!(matches!(
                missing,
                Err(LocalOperationPublicationSweepError::Advance(
                    LocalOperationPublicationAdvanceError::MissingPredecessor
                ))
            ));
            assert_eq!(first_attempt.sources_examined(), 2);
            assert_eq!(authority.entry_count(), journal_before_parent_failures);
            event_port.restore_index(
                parent_occurrence_query.clone(),
                ControlRecord::new(
                    ControlRecordCoordinate::new(
                        record_ref("sweep/conflicting-parent-occurrence"),
                        parent_occurrence_record
                            .coordinate()
                            .record_sha256()
                            .clone(),
                    ),
                    parent_occurrence_record.schema_id().clone(),
                    parent_occurrence_record.schema_sha256().clone(),
                    parent_occurrence_record.canonical_record_jcs().clone(),
                ),
            );
            assert!(sweep!(&mut first_attempt).is_err());
            assert_eq!(authority.entry_count(), journal_before_parent_failures);
            event_port.restore_index(parent_occurrence_query, parent_occurrence_record);

            for expected in [
                LocalOperationPublicationStage::Event,
                LocalOperationPublicationStage::Occurrence,
            ] {
                assert!(matches!(
                    sweep!(&mut first_attempt).expect("child start publication stage"),
                    LocalOperationPublicationSweepProgress::StageEstablished { stage, .. }
                        if stage == expected
                ));
            }

            let mut interrupted = LocalOperationPublicationSweepAttempt::new(store_coordinate());
            assert!(matches!(
                sweep!(&mut interrupted).expect("re-scan complete parent"),
                LocalOperationPublicationSweepProgress::ContinueScan
            ));
            event_port.fail_next_commit_for_schema(GENEALOGY_EDGE_V1_SCHEMA);
            assert!(matches!(
                sweep!(&mut interrupted),
                Err(LocalOperationPublicationSweepError::Advance(
                    LocalOperationPublicationAdvanceError::Resident(_)
                        | LocalOperationPublicationAdvanceError::Store(_)
                ))
            ));
            assert_eq!(
                event_port.record_count_for_schema(GENEALOGY_EDGE_V1_SCHEMA),
                0
            );

            let mut retry = LocalOperationPublicationSweepAttempt::new(store_coordinate());
            assert!(matches!(
                sweep!(&mut retry).expect("retry re-scans complete parent"),
                LocalOperationPublicationSweepProgress::ContinueScan
            ));
            assert!(matches!(
                sweep!(&mut retry).expect("retry one structural edge"),
                LocalOperationPublicationSweepProgress::StageEstablished {
                    stage: LocalOperationPublicationStage::GenealogyEdge,
                    ..
                }
            ));
            assert_eq!(
                event_port.record_count_for_schema(GENEALOGY_EDGE_V1_SCHEMA),
                1,
                "retry retains exactly one structural edge"
            );
            assert_eq!(edge_signatures.get(), 2, "one failed attempt and one retry");
            assert!(occurrence_reads
                .borrow()
                .get(parent_references.occurrence())
                .is_some_and(|count| *count > 0));
            assert!(occurrence_reads
                .borrow()
                .get(child_references.occurrence())
                .is_some_and(|count| *count > 0));
            assert_eq!(
                a2a_recoveries.get(),
                0,
                "local sources do not enter A2A recovery"
            );
                },
            );
            assert!(
                recovery_calls.get() > 0,
                "the sweep uses protected production recovery"
            );
        },
    );
}

#[test]
fn cold_consult_child_join_publishes_only_structural_successors_after_custody() {
    crate::resident::process_host::tests::consult_child_recovery_sweep_hostile::with_completed_cold_consult_child_database_retry(
        "cold-consult-child-publication",
        |child,
         parent,
         selected_store,
         sessions,
         historical,
         mut candidates,
         retained_records,
         completed| {
            let recovered_collection =
                crate::resident::consultation::tests::recover_completed_collection(&completed);
            assert!(matches!(
                completed.observation().1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::CollectingConsultation { .. }
                }
            ));
            assert!(recovered_collection.collection().stopped().is_none());
            assert_eq!(recovered_collection.collection().slots().len(), 1);
            let expected_result = ImmutableRecordReferenceV1 {
                record_ref: record_ref("result/cold-consult-child-success"),
                record_sha256: ArtifactDigest::hash_bytes(b"cold consult child result"),
            };
            assert!(matches!(
                recovered_collection.collection().slots()[0].disposition(),
                crate::resident::consultation::ConsultationSlotDisposition::Fulfilled {
                    evidence,
                    ..
                } if evidence == &expected_result
            ));
            assert!(recovered_collection.collection().disagreements().is_empty());
            assert!(recovered_collection.collection().late_outcomes().is_empty());
            assert_eq!(
                completed.custody().database_evidence(),
                crate::resident::OperationCustodyDatabaseEvidence::ExactReplay,
            );
            assert_eq!(
                completed.custody().anchor_evidence(),
                crate::resident::OperationCustodyAnchorEvidence::ExactReplay,
            );
            let joined_candidate = completed.custody().candidate();
            assert_eq!(
                joined_candidate.collection_journal(),
                Some(completed.journal()),
            );
            assert_eq!(
                joined_candidate
                    .record(OperationCustodyRecordKind::OperationObservation)
                    .expect("joined parent observation")
                    .coordinate()
                    .record_ref(),
                completed.observation().0,
            );
            assert_eq!(
                joined_candidate
                    .checkpoint()
                    .successor_record()
                    .coordinate()
                    .record_ref(),
                completed.checkpoint().0,
            );
            assert_eq!(
                recovered_collection.snapshot().frontier(),
                completed.journal().frontier(),
            );

            assert_eq!(candidates.len(), 5, "exact retained publication history");
            candidates.sort_by_key(|candidate| {
                candidate
                    .checkpoint()
                    .successor()
                    .committed_transaction_sequence()
                    .get()
            });
            let mut sources = candidates
                .iter()
                .map(|candidate| {
                    OperationPublicationSource::new(
                        candidate
                            .checkpoint()
                            .successor()
                            .committed_transaction_sequence(),
                        candidate
                            .record(OperationCustodyRecordKind::OperationObservation)
                            .expect("one observation per retained transition")
                            .clone(),
                        candidate.checkpoint().successor_record().clone(),
                    )
                    .expect("operation observation publication source")
                })
                .collect::<Vec<_>>();
            sources.sort_by_key(|source| source.transaction().get());
            assert!(sources
                .windows(2)
                .all(|pair| pair[0].transaction() < pair[1].transaction()));
            let through = sources.last().expect("joined parent source").transaction().get();
            let references = sources
                .iter()
                .enumerate()
                .map(|(index, source)| {
                    LocalOperationPublicationReferences::derive_for_source(
                        "pale-local-operation",
                        "operation-publication",
                        selected_store,
                        source.observation().coordinate().record_ref(),
                        source.observation().coordinate().record_sha256().as_str(),
                        index != 0,
                    )
                    .expect("deterministic publication references")
                })
                .collect::<Vec<_>>();
            assert!(references[0].edge().is_none());
            assert!(references[1..].iter().all(|value| value.edge().is_some()));
            let mut source_port = SourceStore::new(sources, through);
            assert_eq!(&source_port.store, selected_store);

            let latest = candidates.pop().expect("joined parent candidate");
            assert_eq!(&latest, joined_candidate);
            crate::resident::operation_observation_custody::tests::native_execution::with_service_custody_faults_and_collection(
                completed.checkpoint().1,
                Some(&latest),
                candidates,
                None,
                retained_records,
                0,
                0,
                0,
                0,
                0,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                Vec::new(),
                false,
                None,
                std::rc::Rc::new(RefCell::new(None)),
                Some(recovered_collection.snapshot().clone()),
                |custody, anchor| {
                    let checkpoint_authority =
                        crate::resident::process_host::tests::a2a_cancel_task::checkpoint_signer(
                            &child.checkpoint,
                        );
                    let recovery_calls = Cell::new(0);
                    let mut recover_local =
                        |request: &LocalOperationPublicationRecoveryRequest<'_>| {
                            recovery_calls.set(recovery_calls.get() + 1);
                            recover_consult_publication_source(
                                request,
                                child,
                                parent,
                                sessions,
                                historical,
                                &checkpoint_authority,
                                custody,
                                anchor,
                            )
                        };
                    let a2a_recoveries = Cell::new(0);
                    let mut recover_a2a = |_: &A2aTaskPublicationRecoveryRequest<'_>| {
                        a2a_recoveries.set(a2a_recoveries.get() + 1);
                        Err(A2aTaskPublicationSourceRecoveryError::Unavailable)
                    };

                    let authority = MemoryAuthorityStore::default();
                    let mut application = empty_application(&authority, RingCursorV1::new(0));
                    let mut event_port = PublicationStore::new(&authority);
                    let config = ProjectionFixture::new();
                    let producer = producer();
                    let writer = writer();
                    let producer_ref = record_ref("sweep/consult-producer-allocation");
                    let writer_ref = record_ref("sweep/consult-writer-fence");
                    let verifier = |_: &super::super::EventPublicationRaceClosure<'_>| Ok(());
                    let occurrence_signatures = Cell::new(0);
                    let edge_signatures = Cell::new(0);
                    let frontier_signatures = Cell::new(0);
                    let mut sign_occurrence =
                        |_: &[u8],
                         _: &InteractionOccurrencePreimageV1,
                         _: &InteractionOccurrenceDigest| {
                            occurrence_signatures.set(occurrence_signatures.get() + 1);
                            Ok(signature())
                        };
                    let mut sign_edge =
                        |_: &[u8], _: &GenealogyEdgePreimageV1, _: &GenealogyEdgeDigest| {
                            edge_signatures.set(edge_signatures.get() + 1);
                            Ok(signature())
                        };
                    let mut sign_frontier =
                        |_: &[u8],
                         _: &ReconciliationFrontierPreimageV1,
                         _: &ReconciliationFrontierDigest| {
                            frontier_signatures.set(frontier_signatures.get() + 1);
                            Ok(signature())
                        };
                    let mut issuance = LocalOperationPublicationIssuance {
                        sign_occurrence: &mut sign_occurrence,
                        sign_edge: &mut sign_edge,
                        sign_frontier: &mut sign_frontier,
                    };
                    let signing = child.request.to_preimage().signing_authority_binding;
                    let mut admit_event_source = |_: &EventReferencePreimageV1| Ok(());
                    let mut resolve_occurrence = |_: &ControlRecord| Ok(signing.clone());
                    let mut resolve_edge = |_: &ControlRecord| Ok(signing.clone());
                    let mut resolve_frontier = |_: &ControlRecord| Ok(signing.clone());
                    let mut verify_occurrence =
                        |_: &[u8],
                         _: &InteractionOccurrencePreimageV1,
                         _: &InteractionOccurrenceDigest,
                         _: &CanonicalSignatureV1| { Ok(()) };
                    let mut verify_edge =
                        |_: &[u8],
                         _: &GenealogyEdgePreimageV1,
                         _: &GenealogyEdgeDigest,
                         _: &CanonicalSignatureV1| { Ok(()) };
                    let mut verify_frontier =
                        |_: &[u8],
                         _: &ReconciliationFrontierPreimageV1,
                         _: &ReconciliationFrontierDigest,
                         _: &CanonicalSignatureV1| { Ok(()) };
                    let mut readmission = LocalOperationPublicationReadmission {
                        admit_event_source: &mut admit_event_source,
                        resolve_occurrence_authority: &mut resolve_occurrence,
                        verify_occurrence_signature: &mut verify_occurrence,
                        resolve_edge_authority: &mut resolve_edge,
                        verify_edge_signature: &mut verify_edge,
                        resolve_frontier_authority: &mut resolve_frontier,
                        verify_frontier_signature: &mut verify_frontier,
                    };
                    let sweep_limits = sweep_limits(8, 8, 1 << 20);
                    let local_limits = recovery_limits();
                    let a2a_limits = A2aTaskPublicationRecoveryLimits::new(
                        local_limits,
                        A2aTaskRecoveryLimits::new(8, 65_536, 1 << 20, 16, 1 << 20).unwrap(),
                    );
                    macro_rules! sweep {
                        ($attempt:expr) => {
                            sweep_local_operation_publications(
                                $attempt,
                                sweep_limits,
                                local_limits,
                                a2a_limits,
                                &mut source_port,
                                &mut application,
                                &mut event_port,
                                &mut recover_local,
                                &mut recover_a2a,
                                LocalOperationPublicationCurrent {
                                    producer_allocation_ref: &producer_ref,
                                    producer: &producer,
                                    writer_fence_ref: &writer_ref,
                                    writer_fence: &writer,
                                    publication_state: EventPublicationState::Active,
                                    race_verifier: &verifier,
                                    signing_contexts: config.contexts_with(&signing),
                                },
                                &mut issuance,
                                &mut readmission,
                            )
                        };
                    }

                    let complete = |progress: &LocalOperationPublicationSweepProgress| {
                        matches!(
                            progress,
                            LocalOperationPublicationSweepProgress::SupportedLocalPrefixComplete {
                                through: actual,
                                sources_examined: 5,
                                ..
                            } if actual.get() == through
                        )
                    };
                    let mut attempt = LocalOperationPublicationSweepAttempt::new(selected_store.clone());
                    for _ in 0..64 {
                        let progress = sweep!(&mut attempt).expect("publication sweep turn");
                        if complete(&progress) {
                            break;
                        }
                        assert!(!matches!(
                            progress,
                            LocalOperationPublicationSweepProgress::BudgetExhausted
                        ));
                    }
                    assert_eq!(attempt.sources_examined(), 5);
                    assert_eq!(occurrence_signatures.get(), 5);
                    assert_eq!(edge_signatures.get(), 4);
                    assert_eq!(frontier_signatures.get(), 5);
                    assert_eq!(event_port.record_count_for_schema(EVENT_REFERENCE_V1_SCHEMA), 5);
                    assert_eq!(event_port.record_count_for_schema(INTERACTION_OCCURRENCE_V1_SCHEMA), 5);
                    assert_eq!(event_port.record_count_for_schema(GENEALOGY_EDGE_V1_SCHEMA), 4);
                    assert_eq!(event_port.record_count_for_schema(RECONCILIATION_FRONTIER_V1_SCHEMA), 5);
                    assert_eq!(event_port.record_count_for_schema(OPERATION_OBSERVATION_V1_SCHEMA), 0);
                    assert_eq!(event_port.record_count_for_schema(CAPABILITY_RESULT_V1_SCHEMA), 0);
                    assert_eq!(a2a_recoveries.get(), 0);
                    assert!(recovery_calls.get() >= 5);

                    let edge_context = config.contexts_with(&signing).edge;
                    let mut actual_edges = event_port
                        .records_for_schema(GENEALOGY_EDGE_V1_SCHEMA)
                        .into_iter()
                        .map(|record| {
                            let edge = GenealogyEdgeV1::admit_contextual_wire(
                                serde_json::from_value(
                                    pale_core::canon::parse_strict(
                                        record.canonical_record_jcs().as_slice(),
                                    )
                                    .expect("canonical genealogy edge"),
                                )
                                .expect("genealogy edge wire"),
                                &edge_context,
                                |_| Ok(()),
                                |_, _, _, _| Ok(()),
                            )
                            .expect("admitted genealogy edge");
                            let GenealogyProofV1::NativeStructural { relation, .. } = edge.proof()
                            else {
                                panic!("operation publication emits only native structural edges")
                            };
                            (
                                edge.source_occurrence_ref().as_str().to_owned(),
                                edge.destination_occurrence_ref().as_str().to_owned(),
                                relation.as_str().to_owned(),
                            )
                        })
                        .collect::<Vec<_>>();
                    actual_edges.sort();
                    let mut expected_edges = vec![
                        (
                            references[0].occurrence().as_str().to_owned(),
                            references[1].occurrence().as_str().to_owned(),
                            "invoke".to_owned(),
                        ),
                        (
                            references[0].occurrence().as_str().to_owned(),
                            references[2].occurrence().as_str().to_owned(),
                            "successor".to_owned(),
                        ),
                        (
                            references[1].occurrence().as_str().to_owned(),
                            references[3].occurrence().as_str().to_owned(),
                            "successor".to_owned(),
                        ),
                        (
                            references[2].occurrence().as_str().to_owned(),
                            references[4].occurrence().as_str().to_owned(),
                            "successor".to_owned(),
                        ),
                    ];
                    expected_edges.sort();
                    assert_eq!(actual_edges, expected_edges);
                    assert!(!actual_edges.iter().any(|(source, destination, _)| {
                        source == references[3].occurrence().as_str()
                            && destination == references[4].occurrence().as_str()
                    }));

                    let signatures_before_replay = (
                        occurrence_signatures.get(),
                        edge_signatures.get(),
                        frontier_signatures.get(),
                    );
                    let event_commits_before_replay = event_port.committed_transition_count();
                    let journal_entries_before_replay = authority.entry_count();
                    let mut replay =
                        LocalOperationPublicationSweepAttempt::new(selected_store.clone());
                    for _ in 0..16 {
                        let progress = sweep!(&mut replay).expect("recreated publisher replay");
                        if complete(&progress) {
                            break;
                        }
                        assert!(matches!(
                            progress,
                            LocalOperationPublicationSweepProgress::ContinueScan
                        ));
                    }
                    assert_eq!(replay.sources_examined(), 5);
                    assert_eq!(
                        (
                            occurrence_signatures.get(),
                            edge_signatures.get(),
                            frontier_signatures.get(),
                        ),
                        signatures_before_replay,
                    );
                    assert_eq!(
                        event_port.committed_transition_count(),
                        event_commits_before_replay,
                    );
                    assert_eq!(authority.entry_count(), journal_entries_before_replay);
                },
            );
        },
    );
}
