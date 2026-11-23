//! Application ownership of the private consultation roster and its physical join.

use super::coordinator::{ResidentConsultationCoordinator, ResidentConsultationOpenInput};
use super::*;
use crate::effect_case_lineage_fixture as request_fixture;
use crate::local_operation_start_fixture::{
    artifact_digest, checkpoint_authority, issue_observation, record_ref, schema_digest, sha,
    signature, store_coordinate, with_consult_start, with_start_context_class, StartAdmission,
};
use crate::resident::local_operation_start_test_support::prepare_local_start;
use crate::resident::operation_dispatch::OperationCheckpointInput;
use crate::resident::operation_observation_custody::tests::{
    local_start, native_execution, RecordingAnchor, RecordingStore,
};
use crate::resident::operation_service::{
    CancellationAppendAdmission, CancellationAppendInput, ResidentAuthorizedCancellationRoute,
    ResidentConsultationCancellationFailure, ResidentOperationService, ResidentOperationTaskState,
};
use crate::resident::task_owner::{
    ResidentOperationTaskCancellationState, ResidentOperationTaskLimits,
};
use crate::resident::task_reservation::ResidentAggregateTaskLimits;
use crate::resident::CompletedOperationObservationCustody;
use crate::resident::{
    prepare_operation_observation_append, AdmittedAmbientSession, AmbientSessionAdmissionContext,
    LocalOperationPublicationPolicyInput, LocalOperationStartInput,
    OperationObservationAuthorityStoreRejection, OperationObservationTransition,
    PendingLocalOperationStart, PreparedLocalChildParent, PreparedLocalOperationStart,
    PreparedOperationStart, RecoveredLocalOperation, OPERATION_CANCEL_CAPABILITY_ID,
};
use crate::session_admission_fixture::{LocalStartContext, ProvingAuthority};
use pale_ports::{
    AccumulatorMutation, AccumulatorPosition, AccumulatorPreparationBasis,
    AccumulatorPreparationLimits, ControlRecordCoordinate, ControlRecordDigest,
    ControlTransactionPreparation, OperationAccumulatorPreparation, OperationCollectionJournal,
    OperationCustodyCandidate, PreparedAuthorityAppend, PreparedOperationAccumulator,
    SequencedAuthorityEntry, StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
    WallClockSourceError, WallClockSourcePort,
};
use std::{cell::Cell, collections::VecDeque, num::NonZeroUsize};

pub(in crate::resident) fn consultation_task_batch_fixture(
    parent: &str,
    frontier: &str,
    child_name: &str,
) -> ConsultationChildDispatchBatch {
    consultation_task_batch_fixture_in_store(
        pale_ports::AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("consultation-store").expect("store identity"),
            ControlIncarnationIdV1::parse("consultation-incarnation").expect("store incarnation"),
        ),
        parent,
        frontier,
        child_name,
    )
}

pub(in crate::resident) fn consultation_task_batch_fixture_in_store(
    store: pale_ports::AuthorityStoreCoordinate,
    parent: &str,
    frontier: &str,
    child_name: &str,
) -> ConsultationChildDispatchBatch {
    let immutable = |value: &str| ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(value).expect("record reference"),
        record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
    };
    ConsultationChildDispatchBatch {
        store,
        parent: immutable(parent),
        collection_frontier: immutable(frontier),
        deadline: UnixTimeV1::new(100).expect("deadline"),
        candidates: vec![ConsultationChildDispatchCandidate {
            slot_id: ConsultationSlotId::parse(format!("slot-{child_name}")).expect("slot"),
            child: immutable(&format!("operation/{child_name}")),
            requirement: ConsultationRequirement::Required,
            requirement_evidence: immutable(&format!("requirement/{child_name}")),
            participant: ParticipantRefV1::Application {
                application_id: format!("participant-{child_name}"),
            },
            provider_binding: None,
            context: Some(immutable(&format!("context/{child_name}"))),
        }],
    }
}

pub(in crate::resident) fn consultation_supervision_fixture(
    parent: &str,
    child_name: &str,
    deadline: u64,
    stopped: bool,
) -> RecoveredConsultationCollection {
    let immutable = |value: &str| ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(value).expect("fixture record reference"),
        record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
    };
    consultation_supervision_fixture_with_child(
        parent,
        child_name,
        immutable(&format!("operation/{child_name}")),
        deadline,
        stopped,
    )
}

pub(in crate::resident) fn consultation_supervision_fixture_in_store(
    parent: &str,
    child_name: &str,
    deadline: u64,
    stopped: bool,
    store: pale_ports::AuthorityStoreCoordinate,
) -> RecoveredConsultationCollection {
    let child = ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(format!("operation/{child_name}"))
            .expect("fixture child reference"),
        record_sha256: ArtifactDigest::hash_bytes(format!("operation/{child_name}").as_bytes()),
    };
    consultation_supervision_fixture_with_child_in_store(
        parent, child_name, child, deadline, stopped, store,
    )
}

pub(in crate::resident) fn consultation_supervision_fixture_with_child(
    parent: &str,
    child_name: &str,
    child: ImmutableRecordReferenceV1,
    deadline: u64,
    stopped: bool,
) -> RecoveredConsultationCollection {
    consultation_supervision_fixture_with_child_in_store(
        parent,
        child_name,
        child,
        deadline,
        stopped,
        pale_ports::AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("consultation-store").expect("fixture store"),
            ControlIncarnationIdV1::parse("consultation-incarnation").expect("fixture incarnation"),
        ),
    )
}

pub(in crate::resident) fn consultation_supervision_fixture_with_child_in_store(
    parent: &str,
    child_name: &str,
    child: ImmutableRecordReferenceV1,
    deadline: u64,
    stopped: bool,
    store: pale_ports::AuthorityStoreCoordinate,
) -> RecoveredConsultationCollection {
    let immutable = |value: &str| ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(value).expect("fixture record reference"),
        record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
    };
    consultation_supervision_fixture_for_parent_in_store(
        immutable(parent),
        child_name,
        child,
        deadline,
        stopped,
        store,
    )
}

pub(in crate::resident) fn consultation_supervision_fixture_for_parent_in_store(
    parent: ImmutableRecordReferenceV1,
    child_name: &str,
    child: ImmutableRecordReferenceV1,
    deadline: u64,
    stopped: bool,
    store: pale_ports::AuthorityStoreCoordinate,
) -> RecoveredConsultationCollection {
    consultation_supervision_fixture_with_times_in_store(
        parent,
        child_name,
        child,
        UnixTimeV1::new(1).expect("fixture opened time"),
        UnixTimeV1::new(deadline).expect("fixture deadline"),
        UnixTimeV1::new(200).expect("fixture expiry"),
        UnixTimeV1::new(300).expect("fixture retention"),
        stopped,
        store,
        true,
        None,
        None,
    )
}

pub(in crate::resident) fn consultation_supervision_fixture_for_operation_in_store(
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    child_name: &str,
    child: ImmutableRecordReferenceV1,
    stopped: bool,
    store: pale_ports::AuthorityStoreCoordinate,
) -> RecoveredConsultationCollection {
    let parent = ImmutableRecordReferenceV1 {
        record_ref: handle_ref.clone(),
        record_sha256: ArtifactDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .expect("fixture operation handle digest"),
    };
    let deadline = UnixTimeV1::new(handle.expires_at().get() - 1)
        .expect("fixture deadline before operation expiry");
    consultation_supervision_fixture_with_times_in_store(
        parent,
        child_name,
        child,
        *handle.created_at(),
        deadline,
        *handle.expires_at(),
        *handle.retain_until(),
        stopped,
        store,
        true,
        None,
        None,
    )
}

pub(in crate::resident) fn consultation_supervision_fixture_for_self_contained_operation_in_store(
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    child_name: &str,
    child: ImmutableRecordReferenceV1,
    stopped: bool,
    store: pale_ports::AuthorityStoreCoordinate,
) -> RecoveredConsultationCollection {
    let parent = ImmutableRecordReferenceV1 {
        record_ref: handle_ref.clone(),
        record_sha256: ArtifactDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .expect("fixture operation handle digest"),
    };
    let deadline = UnixTimeV1::new(handle.expires_at().get() - 1)
        .expect("fixture deadline before operation expiry");
    consultation_supervision_fixture_with_times_in_store(
        parent,
        child_name,
        child,
        *handle.created_at(),
        deadline,
        *handle.expires_at(),
        *handle.retain_until(),
        stopped,
        store,
        false,
        None,
        None,
    )
}

/// Build one exact self-contained roster whose slot already retains evidence.
///
/// The physical cold-recovery tests use this fixture to prove that a different
/// signed terminal result is a conflict, not replayable missing evidence.
pub(in crate::resident) fn consultation_supervision_fixture_with_retained_evidence_in_store(
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    child_name: &str,
    child: ImmutableRecordReferenceV1,
    evidence: ImmutableRecordReferenceV1,
    store: pale_ports::AuthorityStoreCoordinate,
) -> RecoveredConsultationCollection {
    let mut current = consultation_supervision_fixture_for_self_contained_operation_in_store(
        handle_ref,
        handle,
        child_name,
        child,
        false,
        store.clone(),
    );
    let recorded_at = UnixTimeV1::new(handle.created_at().get() + 1)
        .expect("retained evidence time follows parent creation");
    current.collection.updated_at = recorded_at;
    current.collection.slots[0].disposition = ConsultationSlotDisposition::Fulfilled {
        evidence,
        recorded_at,
    };
    recovered_consultation_fixture(current.collection, store)
}

pub(in crate::resident) fn consultation_supervision_fixture_for_provider_operation_in_store(
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    child_name: &str,
    child: ImmutableRecordReferenceV1,
    participant: ParticipantRefV1,
    provider_binding: ProviderBindingRefV1,
    store: pale_ports::AuthorityStoreCoordinate,
) -> RecoveredConsultationCollection {
    let parent = ImmutableRecordReferenceV1 {
        record_ref: handle_ref.clone(),
        record_sha256: ArtifactDigest::parse(handle.canonical_operation_handle_sha256().as_str())
            .expect("fixture operation handle digest"),
    };
    consultation_supervision_fixture_with_times_in_store(
        parent,
        child_name,
        child,
        *handle.created_at(),
        UnixTimeV1::new(handle.expires_at().get() - 1)
            .expect("fixture deadline before operation expiry"),
        *handle.expires_at(),
        *handle.retain_until(),
        false,
        store,
        false,
        Some(provider_binding),
        Some(participant),
    )
}

/// Build one recovered evidence-only collection for an exact root Consult.
pub(in crate::resident) fn consultation_evidence_fixture_for_operation_in_store(
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    evidence: ImmutableRecordReferenceV1,
    store: pale_ports::AuthorityStoreCoordinate,
) -> RecoveredConsultationCollection {
    let limits = ConsultationCollectionLimits::new(1, 65_536, 65_536)
        .expect("bounded evidence-only collection");
    let collection = ConsultationCollection::admit_initial(
        handle_ref,
        handle,
        ConsultationCollectionAdmission {
            completion_policy: evidence.clone(),
            opened_at: *handle.created_at(),
            deadline: *handle.expires_at(),
            slots: vec![ConsultationRosterSlot::new(
                ConsultationSlotId::parse("root-policy-evidence").expect("fixture slot"),
                ConsultationRequirement::Required,
                evidence.clone(),
                ParticipantRefV1::Application {
                    application_id: "root-policy-evidence".into(),
                },
                None,
                None,
                ConsultationContributionSource::EvidenceResolved { evidence },
            )],
        },
        &[],
        limits,
    )
    .expect("evidence-only collection matches the root Consult");
    recovered_consultation_fixture(collection, store)
}

#[allow(clippy::too_many_arguments)]
fn consultation_supervision_fixture_with_times_in_store(
    parent: ImmutableRecordReferenceV1,
    child_name: &str,
    child: ImmutableRecordReferenceV1,
    opened_at: UnixTimeV1,
    deadline: UnixTimeV1,
    expires_at: UnixTimeV1,
    retain_until: UnixTimeV1,
    stopped: bool,
    store: pale_ports::AuthorityStoreCoordinate,
    include_context: bool,
    provider_binding: Option<ProviderBindingRefV1>,
    participant: Option<ParticipantRefV1>,
) -> RecoveredConsultationCollection {
    let immutable = |value: &str| ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(value).expect("fixture record reference"),
        record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
    };
    let stopped_at = deadline;
    let mut collection = ConsultationCollection {
        initial_v1_domain: "PALE_PRIVATE_CONSULTATION_COLLECTION_V1".into(),
        parent: parent.clone(),
        completion_policy: immutable("policy/fixture"),
        opened_at,
        updated_at: if stopped { stopped_at } else { opened_at },
        deadline,
        expires_at,
        retain_until,
        predecessor_frontier: None,
        frozen_stop_frontier: None,
        slots: vec![ConsultationCollectionSlot {
            roster: ConsultationRosterSlot {
                slot_id: ConsultationSlotId::parse(format!("slot-{child_name}"))
                    .expect("fixture slot"),
                requirement: ConsultationRequirement::Required,
                requirement_evidence: immutable(&format!("requirement/{child_name}")),
                participant: participant.unwrap_or_else(|| ParticipantRefV1::Application {
                    application_id: format!("participant-{child_name}"),
                }),
                provider_binding,
                context: include_context.then(|| immutable(&format!("context/{child_name}"))),
                source: ConsultationContributionSource::FreshChild { child },
            },
            disposition: ConsultationSlotDisposition::Outstanding,
        }],
        disagreements: Vec::new(),
        stop: None,
        late_outcomes: Vec::new(),
    };
    if stopped {
        collection.stop = Some(ConsultationCollectionStop {
            cause: ConsultationStopCause::Deadline,
            reason: immutable("policy/fixture-stop"),
            stopped_at,
            summary: collection.requirement_summary(),
        });
    }
    recovered_consultation_fixture(collection, store)
}

fn recovered_consultation_fixture(
    collection: ConsultationCollection,
    store: pale_ports::AuthorityStoreCoordinate,
) -> RecoveredConsultationCollection {
    let parent = collection.parent().clone();
    let bytes = BoundedAuthorityEntryBytes::new(
        canon::to_canonical_string(&collection)
            .expect("fixture collection encoding")
            .into_bytes(),
        65_536,
    )
    .expect("fixture bounded collection");
    let handle = pale_ports::ControlRecordCoordinate::new(
        parent.record_ref.clone(),
        ControlRecordDigest::parse(parent.record_sha256.as_str()).expect("fixture handle digest"),
    );
    let transaction = pale_ports::ControlTransactionPreparation::new(
        store.clone(),
        ControlTransactionSequenceV1::new(1).expect("fixture transaction"),
    )
    .expect("fixture successor transaction");
    let append = PreparedAuthorityAppend::new(
        store.clone(),
        None,
        SequencedAuthorityEntry::new(
            JournalSequenceV1::new(1).expect("fixture journal sequence"),
            bytes,
        ),
    )
    .expect("fixture append");
    let journal = OperationCollectionJournal::prepare(&handle, transaction, append, 65_536, 65_536)
        .expect("fixture collection journal");
    let evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
        journal.frontier(),
        &store,
        &handle,
        65_536,
    )
    .expect("fixture frontier evidence");
    let snapshot = OperationCollectionSnapshot::new(
        journal.frontier().clone(),
        &evidence,
        pale_ports::OperationCollectionHeadObservation::for_handle(
            &handle,
            journal.frontier().coordinate().clone(),
        ),
        journal.append().entry().bytes().clone(),
    )
    .expect("fixture collection snapshot");
    RecoveredConsultationCollection {
        collection,
        snapshot,
        store,
    }
}

#[derive(Default)]
struct ConsultationTaskExecutor {
    accept_calls: usize,
    accepted: Vec<crate::resident::task_owner::ResidentOperationTask<u8>>,
    completions: VecDeque<crate::resident::task_owner::ResidentOperationTaskCompletion<u8, u16>>,
}

impl ConsultationTaskExecutor {
    fn complete(&mut self, index: usize, output: u16) {
        let task = self.accepted.remove(index);
        self.completions.push_back(
            crate::resident::task_owner::ResidentOperationTaskCompletion::Completed {
                task,
                output,
            },
        );
    }

    fn lose(&mut self, index: usize) {
        let task = self.accepted.remove(index);
        self.completions.push_back(task.into_lost());
    }
}

impl crate::resident::task_owner::ResidentOperationTaskExecutor<u8, u16>
    for ConsultationTaskExecutor
{
    fn try_accept(
        &mut self,
        task: crate::resident::task_owner::ResidentOperationTask<u8>,
    ) -> Result<(), crate::resident::task_owner::ResidentOperationTask<u8>> {
        self.accept_calls += 1;
        self.accepted.push(task);
        Ok(())
    }

    fn try_completion(
        &mut self,
    ) -> Option<crate::resident::task_owner::ResidentOperationTaskCompletion<u8, u16>> {
        self.completions.pop_front()
    }
}

struct ConsultationEvidenceExecutor<Output> {
    accept_calls: usize,
    accepted: Vec<crate::resident::task_owner::ResidentOperationTask<()>>,
    completions: VecDeque<crate::resident::task_owner::ResidentOperationTaskCompletion<(), Output>>,
}

impl<Output> Default for ConsultationEvidenceExecutor<Output> {
    fn default() -> Self {
        Self {
            accept_calls: 0,
            accepted: Vec::new(),
            completions: VecDeque::new(),
        }
    }
}

impl<Output> ConsultationEvidenceExecutor<Output> {
    fn complete(&mut self, index: usize, output: Output) {
        let task = self.accepted.remove(index);
        self.completions.push_back(
            crate::resident::task_owner::ResidentOperationTaskCompletion::Completed {
                task,
                output,
            },
        );
    }

    fn lose(&mut self, index: usize) {
        let task = self.accepted.remove(index);
        self.completions.push_back(task.into_lost());
    }
}

impl<Output> crate::resident::task_owner::ResidentOperationTaskExecutor<(), Output>
    for ConsultationEvidenceExecutor<Output>
{
    fn try_accept(
        &mut self,
        task: crate::resident::task_owner::ResidentOperationTask<()>,
    ) -> Result<(), crate::resident::task_owner::ResidentOperationTask<()>> {
        self.accept_calls += 1;
        self.accepted.push(task);
        Ok(())
    }

    fn try_completion(
        &mut self,
    ) -> Option<crate::resident::task_owner::ResidentOperationTaskCompletion<(), Output>> {
        self.completions.pop_front()
    }
}

#[derive(Clone, Copy)]
struct FixedClock(UnixTimeV1);

impl WallClockSourcePort for FixedClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(self.0)
    }
}

fn immutable(reference: &str, byte: char) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record_ref(reference),
        record_sha256: artifact_digest(byte),
    }
}

fn substituted_store_coordinate() -> pale_ports::AuthorityStoreCoordinate {
    pale_ports::AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").unwrap(),
        ControlIncarnationIdV1::parse("incarnation-2").unwrap(),
    )
}

fn cancellation_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
    issued_at: u64,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = original.to_preimage();
    preimage.request_id =
        ResidentRequestIdV1::parse(format!("cancel-consultation-{issued_at}")).unwrap();
    preimage.correlation_id =
        ResidentCorrelationIdV1::parse(format!("cancel-consultation-{issued_at}")).unwrap();
    preimage.capability_id = CapabilityIdV1::parse(OPERATION_CANCEL_CAPABILITY_ID).unwrap();
    preimage.idempotency_key =
        RequestIdempotencyKeyV1::parse(format!("cancel-consultation-{issued_at}")).unwrap();
    preimage.issued_at = UnixTimeV1::new(issued_at).unwrap();
    preimage.expires_at = *handle.expires_at();
    preimage.generation = Some(handle.compiled_generation_sha256().clone());
    preimage.fence_token = Some(handle.activation_fence_sha256().clone());
    preimage.body.schema_id = SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).unwrap();
    preimage.body.schema_sha256 = handle.schema_sha256().clone();
    preimage.body.payload = serde_json::to_value(handle).unwrap();
    preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&preimage.body.payload)
            .unwrap()
            .as_bytes(),
    );
    request_fixture::request(&preimage)
}

fn admitted_child(
    parent: &IssuedLocalOperation,
    signing: &SigningAuthorityBindingV1,
    suffix: &str,
) -> (ResidentRecordRefV1, OperationHandleV1) {
    let parent_lease = parent.historical_operation_lease();
    let child_lease = OperationLeaseV1::seal(
        schema_digest(OPERATION_LEASE_V1_SCHEMA_ID),
        parent.historical_session_lease(),
        OperationLeaseRequestV1 {
            operation_lease_id: OperationLeaseIdV1::parse(format!("consult-child-lease-{suffix}"))
                .unwrap(),
            operation: parent_lease.operation().clone(),
            capabilities: parent_lease.capabilities().to_vec(),
            effects: parent_lease.effects().to_vec(),
            read_set: parent_lease.read_set().to_vec(),
            evidence_obligations: parent_lease.evidence_obligations().to_vec(),
            budget_units: LeaseBudgetUnitsV1::new(parent_lease.budget_units().get() - 1).unwrap(),
            timeout_millis: LeaseTimeoutMillisV1::new(parent_lease.timeout_millis().get() - 1)
                .unwrap(),
            expires_at: UnixTimeV1::new(79).unwrap(),
            invocation_correlation: parent_lease.invocation_correlation().clone(),
        },
    )
    .unwrap();
    let (parent_ref, parent_handle) = parent.handle();
    let reference = record_ref(&format!("operation/consult-child-{suffix}"));
    let lease_ref = record_ref(&format!("operation/consult-child-lease-{suffix}"));
    let schema = schema_digest(OPERATION_HANDLE_V1_SCHEMA_ID);
    let preimage = OperationHandlePreimageV1 {
        schema_id: SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).unwrap(),
        schema_sha256: schema.clone(),
        operation_handle_id: OperationHandleIdV1::parse(format!("consult-child-{suffix}")).unwrap(),
        operation_class: OperationClassV1::Invoke,
        parent_operation: Some(ParentOperationReferenceV1 {
            operation_handle_id: parent_handle.operation_handle_id().clone(),
            operation_handle_ref: parent_ref.clone(),
            operation_handle_sha256: parent_handle.canonical_operation_handle_sha256().clone(),
        }),
        request_binding: parent_handle.request_binding().clone(),
        operation_lease_ref: lease_ref.clone(),
        operation_lease_sha256: child_lease.operation_lease_digest().clone(),
        compiled_generation_sha256: parent_handle.compiled_generation_sha256().clone(),
        activation_fence_sha256: parent_handle.activation_fence_sha256().clone(),
        instance_sha256: parent_handle.instance_sha256().clone(),
        created_at: UnixTimeV1::new(63).unwrap(),
        expires_at: UnixTimeV1::new(79).unwrap(),
        retain_until: UnixTimeV1::new(119).unwrap(),
        deployment_runtime_authority_ref: parent_handle.deployment_runtime_authority_ref().clone(),
        deployment_runtime_authority_sha256: parent_handle
            .deployment_runtime_authority_sha256()
            .clone(),
        issuer_principal_id: parent_handle.issuer_principal_id().clone(),
        issuer_key_id: parent_handle.issuer_key_id().clone(),
        issuer_key_epoch: *parent_handle.issuer_key_epoch(),
    };
    let handle = OperationHandleV1::issue_contextual(
        preimage,
        &OperationHandleAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: signing,
            },
            request_binding: parent_handle.request_binding(),
            operation_lease_ref: &lease_ref,
            operation_lease: &child_lease,
            parent_operation: Some(ParentOperationAdmissionContextV1 {
                operation_handle_ref: parent_ref,
                operation_handle: parent_handle,
                operation_lease: parent_lease,
            }),
            compiled_generation_sha256: parent_handle.compiled_generation_sha256(),
            activation_fence_sha256: parent_handle.activation_fence_sha256(),
            instance_sha256: parent_handle.instance_sha256(),
        },
        |_, _| Ok(()),
        |_, _, _| Ok(signature()),
    )
    .unwrap();
    (reference, handle)
}

/// Issue one actual child through semantic admission, checkpoint construction,
/// database custody, and independent anchoring. The protected-parent value is
/// assembled from an already-issued parent only because parent recovery has its
/// own exhaustive tests; no signed child handle is fabricated here.
pub(in crate::resident) fn issue_test_child(
    parent: &IssuedLocalOperation,
    parent_request: &CommonRequestEnvelopeV1,
    parent_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    generation: &crate::resident::AdmittedCompiledGeneration,
    retained: &crate::local_operation_start_fixture::RecoveryParents<'_>,
    operation_class: OperationClassV1,
    suffix: &str,
) -> (IssuedLocalOperation, ControlStateCheckpointV1) {
    let issued = issue_test_child_with_evidence(
        parent,
        parent_request,
        parent_checkpoint,
        generation,
        retained,
        operation_class,
        suffix,
    );
    (issued.operation, issued.checkpoint)
}

pub(in crate::resident) struct IssuedTestChild {
    pub(in crate::resident) operation: IssuedLocalOperation,
    pub(in crate::resident) parent_request:
        (pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1),
    pub(in crate::resident) request_ref: ResidentRecordRefV1,
    pub(in crate::resident) request: CommonRequestEnvelopeV1,
    pub(in crate::resident) checkpoint_ref: ResidentRecordRefV1,
    pub(in crate::resident) checkpoint: ControlStateCheckpointV1,
    pub(in crate::resident) initial_record: ControlRecord,
    pub(in crate::resident) initial_candidate: OperationCustodyCandidate,
}

pub(in crate::resident) struct IssuedTestChildLeaseCustody {
    pub(in crate::resident) parent: pale_ports::ControlRecord,
    pub(in crate::resident) child: pale_ports::ControlRecord,
}

/// Supply one complete child start without signing or retaining it.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn with_test_child_start_input<R>(
    parent: (&ResidentRecordRefV1, &OperationHandleV1),
    parent_lease: &OperationLeaseV1,
    parent_checkpoint: Option<(&ResidentRecordRefV1, &ControlStateCheckpointV1)>,
    parent_request: &CommonRequestEnvelopeV1,
    generation: &crate::resident::AdmittedCompiledGeneration,
    retained: &crate::local_operation_start_fixture::RecoveryParents<'_>,
    operation_class: OperationClassV1,
    suffix: &str,
    run: impl FnOnce(
        crate::resident::local_operation_coordinator::ResidentLocalOperationStartInput<'_>,
        ControlStateCheckpointPreimageV1,
    ) -> R,
) -> R {
    let lease = retained
        .session
        .admit_operation(
            schema_digest(OPERATION_LEASE_V1_SCHEMA_ID),
            OperationLeaseRequestV1 {
                operation_lease_id: OperationLeaseIdV1::parse(format!(
                    "child-operation-lease-{suffix}"
                ))
                .unwrap(),
                operation: parent_lease.operation().clone(),
                capabilities: parent_lease.capabilities().to_vec(),
                effects: parent_lease.effects().to_vec(),
                read_set: parent_lease.read_set().to_vec(),
                evidence_obligations: parent_lease.evidence_obligations().to_vec(),
                budget_units: LeaseBudgetUnitsV1::new(parent_lease.budget_units().get() - 1)
                    .unwrap(),
                timeout_millis: LeaseTimeoutMillisV1::new(parent_lease.timeout_millis().get() - 1)
                    .unwrap(),
                expires_at: UnixTimeV1::new(parent.1.expires_at().get() - 1).unwrap(),
                invocation_correlation: parent_lease.invocation_correlation().clone(),
            },
            retained.current,
            &mut ProvingAuthority,
        )
        .expect("planned child lease admits under the parent session");
    lease
        .verify_against_parent_operation(parent_lease)
        .expect("planned child lease narrows the parent operation");

    let mut request_preimage = parent_request.to_preimage();
    request_preimage.request_id =
        ResidentRequestIdV1::parse(format!("child-operation-request-{suffix}")).unwrap();
    request_preimage.correlation_id =
        ResidentCorrelationIdV1::parse(format!("child-operation-correlation-{suffix}")).unwrap();
    request_preimage.idempotency_key =
        RequestIdempotencyKeyV1::parse(format!("child-operation-idempotency-{suffix}")).unwrap();
    request_preimage.issued_at = retained.current.now;
    request_preimage.session_admission.checked_at = retained.current.now;
    request_preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
        canon::to_canonical_string(&request_preimage.body.payload)
            .unwrap()
            .as_bytes(),
    );
    let request = request_fixture::request(&request_preimage);
    let binding = request.local_request_binding().unwrap();
    let lease_ref = record_ref(&format!("operation/child-operation-lease-{suffix}"));
    let handle_ref = record_ref(&format!("operation/child-operation-{suffix}"));
    let handle_schema = schema_digest(OPERATION_HANDLE_V1_SCHEMA_ID);
    let context = OperationHandleAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &handle_schema,
            signing_authority: retained.signing,
        },
        request_binding: &binding,
        operation_lease_ref: &lease_ref,
        operation_lease: &lease,
        parent_operation: Some(ParentOperationAdmissionContextV1 {
            operation_handle_ref: parent.0,
            operation_handle: parent.1,
            operation_lease: parent_lease,
        }),
        compiled_generation_sha256: generation.record().compiled_generation_digest(),
        activation_fence_sha256: parent.1.activation_fence_sha256(),
        instance_sha256: generation.record().instance_digest(),
    };
    let handle = OperationHandlePreimageV1 {
        schema_id: SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).unwrap(),
        schema_sha256: handle_schema.clone(),
        operation_handle_id: OperationHandleIdV1::parse(format!("child-operation-{suffix}"))
            .unwrap(),
        operation_class,
        parent_operation: Some(ParentOperationReferenceV1 {
            operation_handle_id: parent.1.operation_handle_id().clone(),
            operation_handle_ref: parent.0.clone(),
            operation_handle_sha256: parent.1.canonical_operation_handle_sha256().clone(),
        }),
        request_binding: binding.clone(),
        operation_lease_ref: lease_ref.clone(),
        operation_lease_sha256: lease.operation_lease_digest().clone(),
        compiled_generation_sha256: parent.1.compiled_generation_sha256().clone(),
        activation_fence_sha256: parent.1.activation_fence_sha256().clone(),
        instance_sha256: parent.1.instance_sha256().clone(),
        created_at: retained.current.now,
        expires_at: lease.expires_at(),
        retain_until: *parent.1.retain_until(),
        deployment_runtime_authority_ref: parent.1.deployment_runtime_authority_ref().clone(),
        deployment_runtime_authority_sha256: parent.1.deployment_runtime_authority_sha256().clone(),
        issuer_principal_id: retained.signing.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(retained.signing.signer_key_id()).unwrap(),
        issuer_key_epoch: *retained.signing.signer_key_epoch(),
    };
    let scope = EventScopeV1 {
        principal_scope: request.scope().principal_scope.clone(),
        controller_scope: request.scope().controller_scope.clone(),
        relationship_scope: request.scope().relationship_scope.clone(),
        environment: DeploymentEnvironmentV1::parse(request.scope().environment.as_str()).unwrap(),
    };
    let classification =
        DataClassificationV1::parse(request.scope().data_classification.as_str()).unwrap();
    let visibility = InteractionVisibilityV1::parse("private").unwrap();
    let disclosure = InteractionDisclosureCeilingV1::parse("private").unwrap();
    let payload_policy = PayloadResolutionPolicyV1::parse("reference-only").unwrap();
    let actor = OwnerRecordCoordinateV1 {
        owner_system_id: OwnerSystemIdV1::parse("pale-test-owner").unwrap(),
        record_ref: record_ref(&format!("actor/child-operation-{suffix}")),
        record_schema_id: SchemaId::parse("test/actor.v1").unwrap(),
        record_schema_sha256: schema_digest(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID),
        record_content_sha256: artifact_digest('9'),
        native_revision: NativeRevisionV1 {
            native_revision_owner: NativeRevisionOwnerV1::parse("pale-test-owner").unwrap(),
            native_revision_token: NativeRevisionTokenV1::parse(format!(
                "actor-revision-child-operation-{suffix}"
            ))
            .unwrap(),
        },
    };
    let profile = crate::local_operation_start_fixture::profile();
    let mut authority_fields = match parent_checkpoint {
        Some((reference, checkpoint)) => crate::local_operation_start_fixture::checkpoint_preimage(
            checkpoint.domain_sequence().get() + 1,
            Some((
                reference.clone(),
                checkpoint.canonical_control_checkpoint_sha256().clone(),
            )),
        ),
        None => crate::local_operation_start_fixture::checkpoint_preimage(1, None),
    };
    if let Some((_, checkpoint)) = parent_checkpoint {
        authority_fields.checkpoint_domain_id = checkpoint.checkpoint_domain_id().clone();
        authority_fields.checkpoint_profile_ref = checkpoint.checkpoint_profile_ref().clone();
        authority_fields.checkpoint_profile_sha256 = checkpoint.checkpoint_profile_sha256().clone();
        authority_fields.domain_epoch = *checkpoint.domain_epoch();
    }
    let authority = checkpoint_authority(&authority_fields);
    let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
    let semantic = LocalOperationStartInput {
        checkpoint_profile: &profile,
        request_ref: record_ref(&format!("request/child-operation-{suffix}")),
        request: &request,
        handle_ref,
        handle,
        context: &context,
        session: retained.session,
        current: retained.current,
        generation,
        publication_policy: LocalOperationPublicationPolicyInput {
            target_scope: &scope,
            classification: &classification,
            visibility: &visibility,
            disclosure_ceiling: &disclosure,
            payload_resolution_policy: &payload_policy,
            actor_provenance: &actor,
        },
        checkpoint_authority: &authority,
        observation_ref: record_ref(&format!("observation/child-operation-{suffix}-1")),
        observation_schema: schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
    };
    let input = crate::resident::local_operation_coordinator::ResidentLocalOperationStartInput {
        semantic,
        accumulator_limits: AccumulatorPreparationLimits::new(8, 4096, 4096, 512, 65_536).unwrap(),
        checkpoint:
            crate::resident::local_operation_coordinator::ResidentLocalOperationCheckpointInput {
                id: ControlCheckpointIdV1::parse(format!("child-operation-{suffix}-checkpoint"))
                    .unwrap(),
                reference: record_ref(&format!("operation/child-operation-{suffix}-checkpoint")),
                schema: &checkpoint_schema,
                authority: &authority,
            },
        maximum_record_bytes: 1_048_576,
    };
    run(input, authority_fields.clone())
}

/// Capture a Consult-roster child identity without signing or retaining it.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn planned_test_child(
    parent: (&ResidentRecordRefV1, &OperationHandleV1),
    parent_lease: &OperationLeaseV1,
    parent_request: &CommonRequestEnvelopeV1,
    generation: &crate::resident::AdmittedCompiledGeneration,
    retained: &crate::local_operation_start_fixture::RecoveryParents<'_>,
    operation_class: OperationClassV1,
    suffix: &str,
) -> crate::resident::consultation::ConsultationFreshChildPlan {
    with_test_child_start_input(
        parent,
        parent_lease,
        None,
        parent_request,
        generation,
        retained,
        operation_class,
        suffix,
        |input, _| {
            crate::resident::consultation::ConsultationFreshChildPlan::capture(&input)
                .expect("planned child input has one exact recoverable identity")
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn issue_test_child_with_evidence(
    parent: &IssuedLocalOperation,
    parent_request: &CommonRequestEnvelopeV1,
    parent_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    generation: &crate::resident::AdmittedCompiledGeneration,
    retained: &crate::local_operation_start_fixture::RecoveryParents<'_>,
    operation_class: OperationClassV1,
    suffix: &str,
) -> IssuedTestChild {
    let mut custody =
        RecordingStore::committing(pale_ports::OperationCustodyCommitOutcome::Committed)
            .with_lease_retention(Ok(pale_ports::OperationCustodyCommitOutcome::Committed))
            .with_unlisted_indexes_absent();
    let mut anchor = local_start::successor_anchor();
    issue_test_child_with_evidence_in_custody(
        parent,
        parent_request,
        parent_checkpoint,
        generation,
        retained,
        operation_class,
        suffix,
        &mut custody,
        &mut anchor,
    )
}

#[allow(clippy::too_many_arguments)]
fn issue_test_child_with_evidence_in_custody(
    parent: &IssuedLocalOperation,
    parent_request: &CommonRequestEnvelopeV1,
    parent_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    generation: &crate::resident::AdmittedCompiledGeneration,
    retained: &crate::local_operation_start_fixture::RecoveryParents<'_>,
    operation_class: OperationClassV1,
    suffix: &str,
    custody: &mut dyn pale_ports::OperationCustodyPort,
    anchor: &mut dyn pale_ports::AntiRollbackAnchorPort,
) -> IssuedTestChild {
    let parent_lease = parent.historical_operation_lease();
    let lease = retained
        .session
        .admit_operation(
            schema_digest(OPERATION_LEASE_V1_SCHEMA_ID),
            OperationLeaseRequestV1 {
                operation_lease_id: OperationLeaseIdV1::parse(format!(
                    "child-operation-lease-{suffix}"
                ))
                .unwrap(),
                operation: parent_lease.operation().clone(),
                capabilities: parent_lease.capabilities().to_vec(),
                effects: parent_lease.effects().to_vec(),
                read_set: parent_lease.read_set().to_vec(),
                evidence_obligations: parent_lease.evidence_obligations().to_vec(),
                budget_units: LeaseBudgetUnitsV1::new(parent_lease.budget_units().get() - 1)
                    .unwrap(),
                timeout_millis: LeaseTimeoutMillisV1::new(parent_lease.timeout_millis().get() - 1)
                    .unwrap(),
                expires_at: UnixTimeV1::new(parent.handle().1.expires_at().get() - 1).unwrap(),
                invocation_correlation: parent_lease.invocation_correlation().clone(),
            },
            retained.current,
            &mut ProvingAuthority,
        )
        .expect("child lease admits under the current parent session");
    lease
        .verify_against_parent_operation(parent_lease)
        .expect("child lease narrows the parent operation");

    let mut request_preimage = parent_request.to_preimage();
    request_preimage.request_id =
        ResidentRequestIdV1::parse(format!("child-operation-request-{suffix}")).unwrap();
    request_preimage.correlation_id =
        ResidentCorrelationIdV1::parse(format!("child-operation-correlation-{suffix}")).unwrap();
    request_preimage.idempotency_key =
        RequestIdempotencyKeyV1::parse(format!("child-operation-idempotency-{suffix}")).unwrap();
    request_preimage.issued_at = retained.current.now;
    request_preimage.session_admission.checked_at = retained.current.now;
    request_preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
        canon::to_canonical_string(&request_preimage.body.payload)
            .unwrap()
            .as_bytes(),
    );
    let request = request_fixture::request(&request_preimage);
    let binding = request.local_request_binding().unwrap();
    let lease_ref = record_ref(&format!("operation/child-operation-lease-{suffix}"));
    let handle_ref = record_ref(&format!("operation/child-operation-{suffix}"));
    let handle_schema = schema_digest(OPERATION_HANDLE_V1_SCHEMA_ID);
    let context = OperationHandleAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &handle_schema,
            signing_authority: retained.signing,
        },
        request_binding: &binding,
        operation_lease_ref: &lease_ref,
        operation_lease: &lease,
        parent_operation: Some(ParentOperationAdmissionContextV1 {
            operation_handle_ref: parent.handle().0,
            operation_handle: parent.handle().1,
            operation_lease: parent_lease,
        }),
        compiled_generation_sha256: generation.record().compiled_generation_digest(),
        activation_fence_sha256: parent.handle().1.activation_fence_sha256(),
        instance_sha256: generation.record().instance_digest(),
    };
    let handle = OperationHandlePreimageV1 {
        schema_id: SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).unwrap(),
        schema_sha256: handle_schema.clone(),
        operation_handle_id: OperationHandleIdV1::parse(format!("child-operation-{suffix}"))
            .unwrap(),
        operation_class,
        parent_operation: Some(ParentOperationReferenceV1 {
            operation_handle_id: parent.handle().1.operation_handle_id().clone(),
            operation_handle_ref: parent.handle().0.clone(),
            operation_handle_sha256: parent
                .handle()
                .1
                .canonical_operation_handle_sha256()
                .clone(),
        }),
        request_binding: binding.clone(),
        operation_lease_ref: lease_ref.clone(),
        operation_lease_sha256: lease.operation_lease_digest().clone(),
        compiled_generation_sha256: parent.handle().1.compiled_generation_sha256().clone(),
        activation_fence_sha256: parent.handle().1.activation_fence_sha256().clone(),
        instance_sha256: parent.handle().1.instance_sha256().clone(),
        created_at: retained.current.now,
        expires_at: lease.expires_at(),
        retain_until: *parent.handle().1.retain_until(),
        deployment_runtime_authority_ref: parent
            .handle()
            .1
            .deployment_runtime_authority_ref()
            .clone(),
        deployment_runtime_authority_sha256: parent
            .handle()
            .1
            .deployment_runtime_authority_sha256()
            .clone(),
        issuer_principal_id: retained.signing.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(retained.signing.signer_key_id()).unwrap(),
        issuer_key_epoch: *retained.signing.signer_key_epoch(),
    };

    let publication_scope = EventScopeV1 {
        principal_scope: request.scope().principal_scope.clone(),
        controller_scope: request.scope().controller_scope.clone(),
        relationship_scope: request.scope().relationship_scope.clone(),
        environment: DeploymentEnvironmentV1::parse(request.scope().environment.as_str()).unwrap(),
    };
    let publication_classification =
        DataClassificationV1::parse(request.scope().data_classification.as_str()).unwrap();
    let publication_visibility = InteractionVisibilityV1::parse("private").unwrap();
    let publication_disclosure = InteractionDisclosureCeilingV1::parse("private").unwrap();
    let publication_payload_policy = PayloadResolutionPolicyV1::parse("reference-only").unwrap();
    let publication_actor = OwnerRecordCoordinateV1 {
        owner_system_id: OwnerSystemIdV1::parse("pale-test-owner").unwrap(),
        record_ref: record_ref(&format!("actor/child-operation-{suffix}")),
        record_schema_id: SchemaId::parse("test/actor.v1").unwrap(),
        record_schema_sha256: schema_digest(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID),
        record_content_sha256: artifact_digest('9'),
        native_revision: NativeRevisionV1 {
            native_revision_owner: NativeRevisionOwnerV1::parse("pale-test-owner").unwrap(),
            native_revision_token: NativeRevisionTokenV1::parse(format!(
                "actor-revision-child-operation-{suffix}"
            ))
            .unwrap(),
        },
    };
    let profile = crate::local_operation_start_fixture::profile();
    let mut authority_fields = crate::local_operation_start_fixture::checkpoint_preimage(
        parent_checkpoint.1.domain_sequence().get() + 1,
        Some((
            parent_checkpoint.0.clone(),
            parent_checkpoint
                .1
                .canonical_control_checkpoint_sha256()
                .clone(),
        )),
    );
    authority_fields.checkpoint_domain_id = parent_checkpoint.1.checkpoint_domain_id().clone();
    authority_fields.checkpoint_profile_ref = parent_checkpoint.1.checkpoint_profile_ref().clone();
    authority_fields.checkpoint_profile_sha256 =
        parent_checkpoint.1.checkpoint_profile_sha256().clone();
    authority_fields.domain_epoch = *parent_checkpoint.1.domain_epoch();
    let authority = checkpoint_authority(&authority_fields);
    let protected = PreparedLocalChildParent::fixture(
        parent,
        parent_checkpoint,
        ControlTransactionPreparation::new(
            store_coordinate(),
            ControlTransactionSequenceV1::new(parent_checkpoint.1.domain_sequence().get() + 1)
                .unwrap(),
        )
        .unwrap(),
        retained.current.now,
    );
    let request_ref = record_ref(&format!("request/child-operation-{suffix}"));
    let start_input = LocalOperationStartInput {
        checkpoint_profile: &profile,
        request_ref: request_ref.clone(),
        request: &request,
        handle_ref,
        handle,
        context: &context,
        session: retained.session,
        current: retained.current,
        generation,
        publication_policy: LocalOperationPublicationPolicyInput {
            target_scope: &publication_scope,
            classification: &publication_classification,
            visibility: &publication_visibility,
            disclosure_ceiling: &publication_disclosure,
            payload_resolution_policy: &publication_payload_policy,
            actor_provenance: &publication_actor,
        },
        checkpoint_authority: &authority,
        observation_ref: record_ref(&format!("observation/child-operation-{suffix}-1")),
        observation_schema: schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
    };
    let admit_owner =
        |_: &CommonRequestEnvelopeV1,
         _: &AdmittedAmbientSession,
         _: &AmbientSessionAdmissionContext<'_>,
         _: &OperationHandlePreimageV1,
         _: &OperationLeaseV1,
         _: &crate::semantic::ResolvedOperation,
         _: &LocalOperationPublicationPolicyInput<'_>,
         _: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>| Ok(());
    let prepared: PreparedLocalOperationStart = prepare_local_start(
        start_input,
        Some(protected),
        operation_class,
        None,
        &mut ProvingAuthority,
        admit_owner,
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
    )
    .expect("actual child semantic start");
    let mut preparation =
        RecordingStore::committing(pale_ports::OperationCustodyCommitOutcome::Committed)
            .with_preparation(
                prepare_collection_accumulator,
                ControlTransactionSequenceV1::new(parent_checkpoint.1.domain_sequence().get() + 1)
                    .unwrap(),
            );
    let prepared = prepared
        .prepare_checkpoint(
            AccumulatorPreparationLimits::new(8, 4096, 4096, 512, 65_536).unwrap(),
            &mut preparation,
        )
        .expect("actual child checkpoint roots");
    let checkpoint_ref = record_ref(&format!("operation/child-operation-{suffix}-checkpoint"));
    let mut checkpoint_fields = authority_fields.clone();
    checkpoint_fields.control_checkpoint_id =
        ControlCheckpointIdV1::parse(format!("child-operation-{suffix}-checkpoint")).unwrap();
    checkpoint_fields.committed_transaction_sequence = prepared.transaction_sequence();
    checkpoint_fields.state_root_sha256 = prepared.state_root().clone();
    checkpoint_fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    checkpoint_fields.changed_leaf_keys = prepared.changed_keys().to_vec();
    checkpoint_fields.leaf_count = prepared.leaf_count();
    checkpoint_fields.issued_at =
        UnixTimeV1::new(parent_checkpoint.1.issued_at().get() + 1).unwrap();
    let schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
    let (physical, pending): (PreparedOperationStart, PendingLocalOperationStart) = prepared
        .issue_checkpoint(
            &checkpoint_ref,
            checkpoint_fields.clone(),
            &schema,
            &authority,
            |_, _, _| Ok(signature()),
        )
        .expect("actual child checkpoint signature");
    crate::resident::retain_operation_lease_body(
        custody,
        &store_coordinate(),
        &lease_ref,
        &lease,
        1_048_576,
    )
    .expect("actual child lease custody");
    let database = physical
        .commit(custody, anchor, 1_048_576)
        .expect("actual child database custody");
    let initial_candidate = database.candidate().clone();
    let initial_record = database
        .candidate()
        .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
        .expect("actual child initial observation")
        .clone();
    let issued = pending
        .finish(
            database
                .advance_anchor(anchor)
                .expect("actual child anchor"),
        )
        .unwrap_or_else(|_| panic!("actual child locator"));
    let checkpoint =
        crate::resident::operation_observation_custody::tests::admit_successor_checkpoint(
            &checkpoint_fields,
            parent_checkpoint.0,
            parent_checkpoint.1,
        );
    IssuedTestChild {
        operation: issued,
        parent_request: (parent.request_coordinate().clone(), parent_request.clone()),
        request_ref,
        request,
        checkpoint_ref,
        checkpoint,
        initial_record,
        initial_candidate,
    }
}

/// Issue one real `Delegate` child under a durable `Consult` parent.
///
/// This private fixture keeps A2A owner tests on the same child-operation law
/// as production instead of fabricating a root `Delegate`.
pub(in crate::resident) fn with_issued_delegate_child_named<R>(
    generation: &crate::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    label: &str,
    run: impl FnOnce(
        &IssuedTestChild,
        &IssuedLocalOperation,
        OperationCustodyCandidate,
        ControlStateCheckpointV1,
        IssuedTestChildLeaseCustody,
        &crate::resident::AdmittedCompiledGeneration,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_issued_child_named(
        generation,
        profile,
        label,
        OperationClassV1::Delegate,
        LocalStartContext::Inherited,
        false,
        run,
    )
}

/// Issue one real `Invoke` child under a durable `Consult` parent.
///
/// Process-host tests use this fixture to prove child recovery through the
/// same semantic start, database custody, and anchor path as production.
pub(in crate::resident) fn with_issued_invoke_child_named<R>(
    generation: &crate::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    label: &str,
    run: impl FnOnce(
        &IssuedTestChild,
        &IssuedLocalOperation,
        OperationCustodyCandidate,
        ControlStateCheckpointV1,
        IssuedTestChildLeaseCustody,
        &crate::resident::AdmittedCompiledGeneration,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_issued_child_named(
        generation,
        profile,
        label,
        OperationClassV1::Invoke,
        LocalStartContext::SelfContained,
        true,
        run,
    )
}

/// Issue one provider-backed `Invoke` child under a durable `Consult` parent.
pub(in crate::resident) fn with_issued_provider_invoke_child_named<R>(
    generation: &crate::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    label: &str,
    run: impl FnOnce(
        &IssuedTestChild,
        &IssuedLocalOperation,
        OperationCustodyCandidate,
        ControlStateCheckpointV1,
        IssuedTestChildLeaseCustody,
        &crate::resident::AdmittedCompiledGeneration,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_issued_child_named(
        generation,
        profile,
        label,
        OperationClassV1::Invoke,
        LocalStartContext::SelfContained,
        false,
        run,
    )
}

fn with_issued_child_named<R>(
    generation: &crate::resident::AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    label: &str,
    operation_class: OperationClassV1,
    context_selection: LocalStartContext,
    native_invoke: bool,
    run: impl FnOnce(
        &IssuedTestChild,
        &IssuedLocalOperation,
        OperationCustodyCandidate,
        ControlStateCheckpointV1,
        IssuedTestChildLeaseCustody,
        &crate::resident::AdmittedCompiledGeneration,
        crate::local_operation_start_fixture::RecoveryParents<'_>,
    ) -> R,
) -> R {
    with_start_context_class(
        generation,
        profile,
        "a2a-task-owner-parent",
        |request| {
            if native_invoke {
                let descriptor = pale_contracts::schema_descriptor("action-input.v1")
                    .expect("action-input schema");
                request.body.schema_id = SchemaId::parse(descriptor.schema_id).unwrap();
                request.body.schema_sha256 = SchemaDigest::parse(descriptor.schema_sha256).unwrap();
                request.body.payload = serde_json::json!({
                    "kind": "path_inventory",
                    "payload": {
                        "style": "posix_relative",
                        "paths": ["src/./lib.rs"]
                    }
                });
                request.body.payload_sha256 = ArtifactDigest::hash_bytes(
                    canon::to_canonical_string(&request.body.payload)
                        .unwrap()
                        .as_bytes(),
                );
            }
        },
        StartAdmission::AcceptedOwnerModel,
        context_selection,
        OperationClassV1::Consult,
        false,
        None,
        |prepared, request, _, _, retained| {
            let prepared = prepared.expect("Consult parent start");
            let parent_store = store_coordinate();
            let prepared = if operation_class == OperationClassV1::Delegate {
                prepared
            } else {
                let planned = planned_test_child(
                    prepared.prepared_handle(),
                    retained.lease,
                    request,
                    generation,
                    &retained,
                    operation_class,
                    label,
                );
                let immutable = |value: &str| ImmutableRecordReferenceV1 {
                    record_ref: ResidentRecordRefV1::parse(value)
                        .expect("Consult fixture record reference"),
                    record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
                };
                let (participant, provider_binding, context) = if native_invoke {
                    (
                        ParticipantRefV1::Application {
                            application_id: format!("participant-{label}"),
                        },
                        None,
                        None,
                    )
                } else {
                    let provider = generation
                        .provider_binding("provider-binding-claude-code")
                        .expect("compiled provider binding");
                    (
                        provider.participant.clone(),
                        Some(ProviderBindingRefV1 {
                            provider_binding_id: provider.id.clone(),
                            provider_binding_identity: provider.binding_identity.clone(),
                        }),
                        None,
                    )
                };
                let (parent_ref, parent_handle) = prepared.prepared_handle();
                let intent = ConsultationStartIntent::admit_signed(
                    &parent_store,
                    parent_ref,
                    parent_handle,
                    ConsultationStartIntentAdmission {
                        completion_policy: immutable("policy/fixture"),
                        opened_at: *parent_handle.created_at(),
                        deadline: UnixTimeV1::new(parent_handle.expires_at().get() - 1)
                            .expect("Consult fixture deadline"),
                        slots: vec![ConsultationStartIntentSlot::new(
                            ConsultationSlotId::parse(format!("slot-{label}"))
                                .expect("Consult fixture slot"),
                            ConsultationRequirement::Required,
                            immutable(&format!("requirement/{label}")),
                            participant,
                            provider_binding,
                            context,
                            ConsultationStartIntentSource::FreshChild {
                                child: Box::new(planned),
                            },
                        )],
                        limits: ConsultationCollectionLimits::new(1, 65_536, 65_536)
                            .expect("Consult fixture limits"),
                    },
                )
                .expect("Consult fixture intent matches its signed parent");
                prepared.with_test_consultation_start_intent(intent)
            };
            let parent_lease_ref = crate::local_operation_start_fixture::operation_lease_ref();
            let parent_lease = retained.lease.clone();
            let prepared = local_start::prepared_initial(prepared);
            let (physical, pending, checkpoint) =
                local_start::issue_initial(prepared, request, |_| {}, &std::cell::Cell::new(0))
                    .expect("Consult parent checkpoint");
            let mut store = local_start::absent_start_store(request)
                .with_lease_retention(Ok(pale_ports::OperationCustodyCommitOutcome::Committed));
            let mut anchor = local_start::initial_anchor();
            crate::resident::retain_operation_lease_body(
                &mut store,
                &parent_store,
                &parent_lease_ref,
                &parent_lease,
                1_048_576,
            )
            .expect("Consult parent lease custody");
            let parent_lease_record = store
                .retained_lease()
                .expect("retained Consult parent lease")
                .clone();
            let database = physical
                .commit(&mut store, &mut anchor, 1_048_576)
                .expect("Consult parent database custody");
            let parent_candidate = database.candidate().clone();
            let parent = pending
                .finish(
                    database
                        .advance_anchor(&mut anchor)
                        .expect("Consult parent anchor"),
                )
                .unwrap_or_else(|_| panic!("issued Consult parent"));
            store.replace_commit_result(Ok(pale_ports::OperationCustodyCommitOutcome::Committed));
            store.replace_read_back_results([Ok(
                pale_ports::OperationCustodyReadBackOutcome::Predecessor,
            )]);
            let mut store = store
                .with_lease_retention(Ok(pale_ports::OperationCustodyCommitOutcome::Committed))
                .with_committed_candidate_index_recovery();
            anchor.allow_successor_preflight();
            anchor.allow_next_advance();
            let checkpoint_ref = parent_candidate
                .checkpoint()
                .successor_record()
                .coordinate()
                .record_ref()
                .clone();
            let child = issue_test_child_with_evidence_in_custody(
                &parent,
                request,
                (&checkpoint_ref, &checkpoint),
                generation,
                &retained,
                operation_class,
                label,
                &mut store,
                &mut anchor,
            );
            let child_lease_record = store
                .retained_lease()
                .expect("retained Delegate child lease")
                .clone();
            let child_parents = crate::local_operation_start_fixture::RecoveryParents {
                session: retained.session,
                current: retained.current,
                lease: child.operation.historical_operation_lease(),
                signing: retained.signing,
            };
            run(
                &child,
                &parent,
                parent_candidate,
                checkpoint,
                IssuedTestChildLeaseCustody {
                    parent: parent_lease_record,
                    child: child_lease_record,
                },
                generation,
                child_parents,
            )
        },
    )
}

fn successor_journal(
    collection: &ConsultationCollection,
    parent: &IssuedLocalOperation,
    transaction: u64,
    previous_journal: u64,
    limits: ConsultationCollectionLimits,
) -> OperationCollectionJournal {
    let transaction = ControlTransactionPreparation::new(
        store_coordinate(),
        ControlTransactionSequenceV1::new(transaction).unwrap(),
    )
    .unwrap();
    let append = PreparedAuthorityAppend::new(
        store_coordinate(),
        Some(JournalSequenceV1::new(previous_journal).unwrap()),
        SequencedAuthorityEntry::new(
            JournalSequenceV1::new(previous_journal + 1).unwrap(),
            collection.canonical_bytes(limits).unwrap(),
        ),
    )
    .unwrap();
    OperationCollectionJournal::prepare(
        &ControlRecordCoordinate::new(
            parent.handle().0.clone(),
            ControlRecordDigest::parse(
                parent
                    .handle()
                    .1
                    .canonical_operation_handle_sha256()
                    .as_str(),
            )
            .unwrap(),
        ),
        transaction,
        append,
        limits.maximum_frontier_bytes,
        limits.maximum_journal_bytes,
    )
    .unwrap()
}

pub(in crate::resident) fn prepare_collection_accumulator(
    request: &OperationAccumulatorPreparation,
) -> Result<PreparedOperationAccumulator, pale_ports::OperationAccumulatorPreparationError> {
    let AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
        panic!("collection follows durable start")
    };
    let next_root = artifact_digest('b');
    Ok(PreparedOperationAccumulator::new(
        request.clone(),
        StateAccumulatorBatch::new(
            prior.state_root().clone(),
            next_root,
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([3; 32]),
                Some(vec![4; 32]),
                Some(vec![5; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(
            prior.unresolved_root().clone(),
            prior.unresolved_root().clone(),
            vec![],
            1,
        )
        .unwrap(),
    )
    .expect("same predecessor physical roots"))
}

/// Re-admit one test-model collection commit through its exact retained bytes.
pub(in crate::resident) fn recover_completed_collection(
    completed: &CompletedConsultationCollection,
) -> RecoveredConsultationCollection {
    let journal = completed.journal();
    let store = journal.append().store().clone();
    let collection = ConsultationCollection::deserialize(
        canon::parse_strict(journal.append().entry().bytes().as_slice())
            .expect("committed collection JSON"),
    )
    .expect("committed collection shape");
    let evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
        journal.frontier(),
        &store,
        journal.operation_handle(),
        journal.maximum_journal_payload_bytes(),
    )
    .expect("committed collection frontier");
    let snapshot = OperationCollectionSnapshot::new(
        journal.frontier().clone(),
        &evidence,
        pale_ports::OperationCollectionHeadObservation::for_handle(
            journal.operation_handle(),
            journal.frontier().coordinate().clone(),
        ),
        journal.append().entry().bytes().clone(),
    )
    .expect("committed collection snapshot");
    RecoveredConsultationCollection {
        collection,
        snapshot,
        store,
    }
}

pub(in crate::resident) struct CollectionRecoveryOnly(
    pub(in crate::resident) Option<pale_ports::OperationCollectionSnapshot>,
);

impl pale_ports::ControlTransactionPort for CollectionRecoveryOnly {
    fn prepare_transaction(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        panic!("collection recovery performs no write preparation")
    }
}

pub(in crate::resident) struct CollectionCommitPort<F> {
    pub(in crate::resident) prepare: F,
    pub(in crate::resident) transaction: ControlTransactionSequenceV1,
    pub(in crate::resident) physical: RecordingStore,
}

impl<F> pale_ports::ControlTransactionPort for CollectionCommitPort<F> {
    fn prepare_transaction(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        assert_eq!(store, &store_coordinate());
        Ok(
            pale_ports::ControlTransactionPreparation::new(store.clone(), self.transaction)
                .unwrap(),
        )
    }
}

impl<F> pale_ports::OperationCustodyPort for CollectionCommitPort<F>
where
    F: FnMut(
        &OperationAccumulatorPreparation,
    ) -> Result<
        PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    >,
{
    fn read_collection_snapshot(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::ControlRecordCoordinate,
        _: usize,
        _: usize,
    ) -> Result<Option<pale_ports::OperationCollectionSnapshot>, pale_ports::ControlStoreError>
    {
        panic!("collection commitment does not recover a snapshot")
    }

    fn read_a2a_task_bindings(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationA2aBindingScope,
        _: Option<&pale_ports::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationA2aBindings, pale_ports::ControlStoreError> {
        panic!("collection commitment does not discover remote tasks")
    }

    fn read_publication_sources(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: Option<&pale_ports::OperationPublicationCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        panic!("collection commitment does not discover publications")
    }

    fn read_checkpoint_head(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<
        Option<(pale_ports::ControlCheckpointHead, pale_ports::ControlRecord)>,
        pale_ports::ControlStoreError,
    > {
        panic!("collection commitment does not recover a checkpoint head")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("collection commitment does not recover accumulator membership")
    }

    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, pale_ports::OperationAccumulatorPreparationError>
    {
        (self.prepare)(request)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::ControlRecord,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, pale_ports::ControlStoreError> {
        panic!("collection commitment does not retain an operation lease")
    }

    fn read_operation_record(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<pale_ports::ControlRecord>, pale_ports::ControlStoreError> {
        panic!("collection commitment does not read an operation record")
    }

    fn compare_and_commit(
        &mut self,
        candidate: &pale_ports::OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, pale_ports::ControlStoreError> {
        self.physical.compare_and_commit(candidate)
    }

    fn observe_exact(
        &mut self,
        candidate: &pale_ports::OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyReadBackOutcome, pale_ports::ControlStoreError> {
        self.physical.observe_exact(candidate)
    }

    fn read_index(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationCustodyIndexQuery,
        _: usize,
    ) -> Result<pale_ports::OperationCustodyIndexObservation, pale_ports::ControlStoreError> {
        panic!("collection commitment does not perform an index lookup")
    }
}

struct CollectionRecoverAndCommitPort<F> {
    snapshot: Option<pale_ports::OperationCollectionSnapshot>,
    commit: CollectionCommitPort<F>,
}

impl<F> pale_ports::ControlTransactionPort for CollectionRecoverAndCommitPort<F>
where
    F: FnMut(
        &OperationAccumulatorPreparation,
    ) -> Result<
        PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    >,
{
    fn prepare_transaction(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        self.commit.prepare_transaction(store)
    }
}

impl<F> pale_ports::OperationCustodyPort for CollectionRecoverAndCommitPort<F>
where
    F: FnMut(
        &OperationAccumulatorPreparation,
    ) -> Result<
        PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    >,
{
    fn read_collection_snapshot(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::ControlRecordCoordinate,
        _: usize,
        _: usize,
    ) -> Result<Option<pale_ports::OperationCollectionSnapshot>, pale_ports::ControlStoreError>
    {
        Ok(self.snapshot.clone())
    }

    fn read_a2a_task_bindings(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        scope: &pale_ports::OperationA2aBindingScope,
        cursor: Option<&pale_ports::OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_record_bytes: usize,
    ) -> Result<pale_ports::OperationA2aBindings, pale_ports::ControlStoreError> {
        self.commit.read_a2a_task_bindings(
            store,
            scope,
            cursor,
            maximum_bindings,
            maximum_record_bytes,
        )
    }

    fn read_publication_sources(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        cursor: Option<&pale_ports::OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_record_bytes: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        self.commit
            .read_publication_sources(store, cursor, maximum_sources, maximum_record_bytes)
    }

    fn read_checkpoint_head(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        domain: &ControlCheckpointDomainIdV1,
        maximum_record_bytes: usize,
    ) -> Result<
        Option<(pale_ports::ControlCheckpointHead, pale_ports::ControlRecord)>,
        pale_ports::ControlStoreError,
    > {
        self.commit
            .read_checkpoint_head(store, domain, maximum_record_bytes)
    }

    fn verify_accumulator_membership(
        &mut self,
        membership: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        self.commit.verify_accumulator_membership(membership)
    }

    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, pale_ports::OperationAccumulatorPreparationError>
    {
        self.commit.prepare_accumulator(request)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        lease: &pale_ports::ControlRecord,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, pale_ports::ControlStoreError> {
        self.commit.compare_and_retain_operation_lease(store, lease)
    }

    fn read_operation_record(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum_record_bytes: usize,
    ) -> Result<Option<pale_ports::ControlRecord>, pale_ports::ControlStoreError> {
        self.commit
            .read_operation_record(store, coordinate, maximum_record_bytes)
    }

    fn compare_and_commit(
        &mut self,
        candidate: &pale_ports::OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, pale_ports::ControlStoreError> {
        self.commit.compare_and_commit(candidate)
    }

    fn observe_exact(
        &mut self,
        candidate: &pale_ports::OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyReadBackOutcome, pale_ports::ControlStoreError> {
        self.commit.observe_exact(candidate)
    }

    fn read_index(
        &mut self,
        store: &pale_ports::AuthorityStoreCoordinate,
        query: &pale_ports::OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<pale_ports::OperationCustodyIndexObservation, pale_ports::ControlStoreError> {
        self.commit.read_index(store, query, maximum_record_bytes)
    }
}

impl pale_ports::OperationCustodyPort for CollectionRecoveryOnly {
    fn read_collection_snapshot(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::ControlRecordCoordinate,
        _: usize,
        _: usize,
    ) -> Result<Option<pale_ports::OperationCollectionSnapshot>, pale_ports::ControlStoreError>
    {
        Ok(self.0.clone())
    }

    fn read_a2a_task_bindings(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationA2aBindingScope,
        _: Option<&pale_ports::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationA2aBindings, pale_ports::ControlStoreError> {
        panic!("collection recovery does not discover A2A tasks")
    }

    fn read_publication_sources(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: Option<&pale_ports::OperationPublicationCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        panic!("collection recovery does not discover publication sources")
    }

    fn read_checkpoint_head(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<
        Option<(pale_ports::ControlCheckpointHead, pale_ports::ControlRecord)>,
        pale_ports::ControlStoreError,
    > {
        panic!("collection recovery does not read the checkpoint head")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("collection recovery does not verify accumulator membership")
    }

    fn prepare_accumulator(
        &mut self,
        _: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("collection recovery does not prepare an accumulator")
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::ControlRecord,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, pale_ports::ControlStoreError> {
        panic!("collection recovery does not retain a lease")
    }

    fn read_operation_record(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<pale_ports::ControlRecord>, pale_ports::ControlStoreError> {
        panic!("collection recovery does not read unrelated records")
    }

    fn compare_and_commit(
        &mut self,
        _: &pale_ports::OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, pale_ports::ControlStoreError> {
        panic!("collection recovery does not commit")
    }

    fn observe_exact(
        &mut self,
        _: &pale_ports::OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyReadBackOutcome, pale_ports::ControlStoreError> {
        panic!("collection recovery does not read back a commit")
    }

    fn read_index(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationCustodyIndexQuery,
        _: usize,
    ) -> Result<pale_ports::OperationCustodyIndexObservation, pale_ports::ControlStoreError> {
        panic!("collection recovery uses its dedicated point read")
    }
}

#[test]
fn consult_roster_and_frontier_share_the_signed_observation_transaction() {
    let (generation, profile) = local_start::source_generation();
    with_consult_start(
        &generation,
        &profile,
        "consult-collection",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, retained| {
            let prepared = local_start::prepared_initial(prepared.expect("Consult start"));
            let (physical, pending, checkpoint) =
                local_start::issue_initial(prepared, request, |_| {}, &std::cell::Cell::new(0))
                    .expect("initial checkpoint");
            let mut store = local_start::absent_start_store(request);
            let mut anchor = local_start::initial_anchor();
            let database = physical
                .commit(&mut store, &mut anchor, 1_048_576)
                .expect("initial custody");
            let initial_record = database
                .candidate()
                .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                .expect("initial observation")
                .clone();
            let issued = pending
                .finish(
                    database
                        .advance_anchor(&mut anchor)
                        .expect("initial anchor"),
                )
                .unwrap_or_else(|_| panic!("issued Consult"));
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = OperationObservationV1::admit_contextual_wire(
                serde_json::from_value(
                    canon::parse_strict(initial_record.canonical_record_jcs().as_slice())
                        .expect("initial observation JSON"),
                )
                .expect("initial observation wire"),
                &OperationObservationAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: retained.signing,
                    },
                    handle_ref: issued.handle().0,
                    handle: issued.handle().1,
                    predecessor: None,
                },
                |_| Ok(()),
                |_, _, _, _| panic!("revision one cannot rotate authority"),
                |_, _, _, _| Ok(()),
            )
            .expect("admitted initial observation");
            let limits = ConsultationCollectionLimits::new(8, 16_384, 4096).unwrap();
            let (child_ref, child) = admitted_child(&issued, retained.signing, "fresh");
            let child_coordinate = ImmutableRecordReferenceV1 {
                record_ref: child_ref.clone(),
                record_sha256: ArtifactDigest::parse(
                    child.canonical_operation_handle_sha256().as_str(),
                )
                .unwrap(),
            };
            let (optional_child_ref, optional_child) =
                admitted_child(&issued, retained.signing, "optional");
            let optional_child_coordinate = ImmutableRecordReferenceV1 {
                record_ref: optional_child_ref.clone(),
                record_sha256: ArtifactDigest::parse(
                    optional_child.canonical_operation_handle_sha256().as_str(),
                )
                .unwrap(),
            };
            let (uncertain_child_ref, uncertain_child) =
                admitted_child(&issued, retained.signing, "uncertain");
            let uncertain_child_coordinate = ImmutableRecordReferenceV1 {
                record_ref: uncertain_child_ref.clone(),
                record_sha256: ArtifactDigest::parse(
                    uncertain_child.canonical_operation_handle_sha256().as_str(),
                )
                .unwrap(),
            };
            let (terminal_child_ref, terminal_child) =
                admitted_child(&issued, retained.signing, "terminal");
            let terminal_child_coordinate = ImmutableRecordReferenceV1 {
                record_ref: terminal_child_ref.clone(),
                record_sha256: ArtifactDigest::parse(
                    terminal_child.canonical_operation_handle_sha256().as_str(),
                )
                .unwrap(),
            };
            let collection = ConsultationCollection::admit_initial(
                issued.handle().0,
                issued.handle().1,
                ConsultationCollectionAdmission {
                    completion_policy: immutable("policy/consult", '7'),
                    opened_at: UnixTimeV1::new(60).unwrap(),
                    deadline: UnixTimeV1::new(75).unwrap(),
                    slots: vec![
                        ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("existing-evidence").unwrap(),
                            ConsultationRequirement::Required,
                            immutable("requirement/existing", '8'),
                            ParticipantRefV1::Application {
                                application_id: "review-owner".into(),
                            },
                            None,
                            Some(immutable("context/existing", '9')),
                            ConsultationContributionSource::EvidenceResolved {
                                evidence: immutable("evidence/existing", 'a'),
                            },
                        ),
                        ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("fresh-assessment").unwrap(),
                            ConsultationRequirement::Required,
                            immutable("requirement/fresh", 'b'),
                            ParticipantRefV1::Application {
                                application_id: "fresh-reviewer".into(),
                            },
                            None,
                            Some(immutable("context/fresh", 'c')),
                            ConsultationContributionSource::FreshChild {
                                child: child_coordinate.clone(),
                            },
                        ),
                        ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("optional-perspective").unwrap(),
                            ConsultationRequirement::Optional,
                            immutable("requirement/optional", 'd'),
                            ParticipantRefV1::Application {
                                application_id: "optional-reviewer".into(),
                            },
                            None,
                            Some(immutable("context/optional", 'e')),
                            ConsultationContributionSource::FreshChild {
                                child: optional_child_coordinate.clone(),
                            },
                        ),
                        ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("uncertain-perspective").unwrap(),
                            ConsultationRequirement::Optional,
                            immutable("requirement/uncertain", '1'),
                            ParticipantRefV1::Application {
                                application_id: "uncertain-reviewer".into(),
                            },
                            None,
                            Some(immutable("context/uncertain", '2')),
                            ConsultationContributionSource::FreshChild {
                                child: uncertain_child_coordinate.clone(),
                            },
                        ),
                        ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("terminal-perspective").unwrap(),
                            ConsultationRequirement::Optional,
                            immutable("requirement/terminal", '3'),
                            ParticipantRefV1::Application {
                                application_id: "terminal-reviewer".into(),
                            },
                            None,
                            Some(immutable("context/terminal", '4')),
                            ConsultationContributionSource::FreshChild {
                                child: terminal_child_coordinate.clone(),
                            },
                        ),
                    ],
                },
                &[
                    (&child_ref, &child),
                    (&optional_child_ref, &optional_child),
                    (&uncertain_child_ref, &uncertain_child),
                    (&terminal_child_ref, &terminal_child),
                ],
                limits,
            )
            .expect("policy-admitted lookup-first roster");
            let mut collection_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(2).unwrap(),
                physical: local_start::absent_start_store(request),
            };
            let prepared = collection
                .prepare_observation(
                    &issued,
                    (&record_ref("operation/checkpoint-1"), &checkpoint),
                    (initial_record.coordinate().record_ref(), &initial),
                    ConsultationCollectionObservationInput {
                        observation_ref: record_ref("observation/consultation-2"),
                        predecessor_collection: None,
                        observed_at: UnixTimeV1::new(62).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                    },
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut collection_custody,
                    |actual| {
                        assert_eq!(
                            actual.completion_policy(),
                            &immutable("policy/consult", '7')
                        );
                        Ok(())
                    },
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .expect("signed collection observation");
            let OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::CollectingConsultation { evidence },
            } = prepared.observation().1.state()
            else {
                panic!("collection phase")
            };
            assert_eq!(
                &evidence.record_ref,
                prepared.journal().frontier().coordinate().record_ref()
            );
            assert_eq!(prepared.journal().transaction().expected().get(), 2);

            let frontier_evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
                prepared.journal().frontier(),
                &store_coordinate(),
                prepared.journal().operation_handle(),
                limits.maximum_frontier_bytes,
            )
            .expect("prepared frontier remains admissible after restart");
            let snapshot = pale_ports::OperationCollectionSnapshot::new(
                prepared.journal().frontier().clone(),
                &frontier_evidence,
                pale_ports::OperationCollectionHeadObservation::for_handle(
                    prepared.journal().operation_handle(),
                    prepared.journal().frontier().coordinate().clone(),
                ),
                prepared.journal().append().entry().bytes().clone(),
            )
            .expect("exact bounded roster snapshot");
            let mut missing_recovery = CollectionRecoveryOnly(None);
            assert!(matches!(
                ConsultationCollection::recover_current(
                    &issued,
                    prepared.observation().1,
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut missing_recovery,
                    |_| Ok(()),
                ),
                Err(ConsultationCollectionError::Association)
            ));
            let mut recovery = CollectionRecoveryOnly(Some(snapshot));
            let recovered = ConsultationCollection::recover_current(
                &issued,
                prepared.observation().1,
                limits,
                &local_start::StartStore(retained.session),
                &mut recovery,
                |actual| {
                    assert_eq!(actual.completion_policy(), collection.completion_policy());
                    assert_eq!(actual.slots().len(), 5);
                    Ok(())
                },
            )
            .expect("restart-safe roster recovery")
            .expect("collection phase requires a roster");
            assert_eq!(recovered.collection(), &collection);
            let dispatch_batch = recovered
                .plan_child_dispatch(UnixTimeV1::new(63).unwrap())
                .expect("four distinct child contributions can be released together");
            assert_eq!(dispatch_batch.candidates().len(), 4);
            assert_eq!(
                dispatch_batch.candidates()[0].context(),
                Some(&immutable("context/fresh", 'c'))
            );
            assert_eq!(
                dispatch_batch.candidates()[1].context(),
                Some(&immutable("context/optional", 'e'))
            );
            assert_ne!(
                dispatch_batch.candidates()[0].child(),
                dispatch_batch.candidates()[1].child()
            );
            assert_eq!(
                recovered
                    .revalidate_child_dispatch(&dispatch_batch, UnixTimeV1::new(64).unwrap(),)
                    .expect("the unchanged recovered head readmits its finite batch"),
                dispatch_batch
            );
            assert!(matches!(
                recovered.revalidate_child_dispatch(&dispatch_batch, UnixTimeV1::new(75).unwrap(),),
                Err(ConsultationCollectionError::Stopped)
            ));

            let optional_slot = ConsultationSlotId::parse("optional-perspective").unwrap();
            let sibling_joined_collection = recovered
                .admit_child_contribution(
                    ConsultationContribution {
                        slot_id: optional_slot,
                        child: optional_child_coordinate.clone(),
                        evidence: immutable("evidence/optional", 'f'),
                        recorded_at: UnixTimeV1::new(64).unwrap(),
                    },
                    limits,
                    |slot, _| {
                        assert_eq!(slot.slot_id().as_str(), "optional-perspective");
                        Ok(())
                    },
                )
                .expect("one sibling contribution advances the current roster");
            let sibling_joined = RecoveredConsultationCollection {
                collection: sibling_joined_collection,
                snapshot: recovered.snapshot().clone(),
                store: recovered.store().clone(),
            };
            assert!(matches!(
                sibling_joined
                    .revalidate_child_dispatch(&dispatch_batch, UnixTimeV1::new(65).unwrap(),),
                Err(ConsultationCollectionError::Association)
            ));
            assert!(matches!(
                service::revalidate_planned_child(
                    &sibling_joined,
                    &dispatch_batch,
                    &dispatch_batch.candidates()[0],
                    UnixTimeV1::new(65).unwrap(),
                ),
                Err(ConsultationCollectionError::Association)
            ));
            assert!(matches!(
                service::revalidate_planned_child(
                    &sibling_joined,
                    &dispatch_batch,
                    &dispatch_batch.candidates()[1],
                    UnixTimeV1::new(65).unwrap(),
                ),
                Err(ConsultationCollectionError::Association)
            ));

            let release_child = |candidate: &ConsultationChildDispatchCandidate| {
                let (reference, handle, suffix) = if candidate.child() == &child_coordinate {
                    (&child_ref, &child, "fresh")
                } else if candidate.child() == &optional_child_coordinate {
                    (&optional_child_ref, &optional_child, "optional")
                } else if candidate.child() == &uncertain_child_coordinate {
                    (&uncertain_child_ref, &uncertain_child, "uncertain")
                } else if candidate.child() == &terminal_child_coordinate {
                    (&terminal_child_ref, &terminal_child, "terminal")
                } else {
                    panic!("the planned consultation child must retain its admitted handle")
                };
                let observation_ref = record_ref(&format!("observation/consult-release-{suffix}"));
                let observation = issue_observation(
                    retained.signing,
                    reference,
                    handle,
                    observation_schema.clone(),
                    OperationObservationRevisionV1::new(1).unwrap(),
                    None,
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Resolving,
                    },
                    63,
                );
                RecoveredOperationObservation::fixture_for_handle(
                    ControlRecordCoordinate::new(
                        reference.clone(),
                        ControlRecordDigest::parse(
                            handle.canonical_operation_handle_sha256().as_str(),
                        )
                        .unwrap(),
                    ),
                    (observation_ref.clone(), observation.clone()),
                    (observation_ref, observation),
                    (
                        record_ref(&format!("operation/consult-release-{suffix}-checkpoint")),
                        checkpoint.clone(),
                    ),
                    store_coordinate(),
                )
            };
            let retain_child = |candidate: &ConsultationChildDispatchCandidate| {
                let (reference, handle, suffix) = if candidate.child() == &child_coordinate {
                    (&child_ref, &child, "fresh")
                } else if candidate.child() == &optional_child_coordinate {
                    (&optional_child_ref, &optional_child, "optional")
                } else if candidate.child() == &uncertain_child_coordinate {
                    (&uncertain_child_ref, &uncertain_child, "uncertain")
                } else if candidate.child() == &terminal_child_coordinate {
                    (&terminal_child_ref, &terminal_child, "terminal")
                } else {
                    panic!("retained child must remain in the admitted consultation batch")
                };
                let initial = release_child(candidate);
                let (initial_ref, initial_observation) = initial.selected();
                let dispatched_ref =
                    record_ref(&format!("observation/consult-task-retained-{suffix}"));
                let dispatched = issue_observation(
                    retained.signing,
                    reference,
                    handle,
                    observation_schema.clone(),
                    OperationObservationRevisionV1::new(2).unwrap(),
                    Some((initial_ref, initial_observation)),
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Dispatched {
                            attempt_id: CapabilityAttemptIdV1::parse(format!(
                                "consult-task-attempt-{suffix}"
                            ))
                            .unwrap(),
                            evidence: immutable(&format!("attempt/consult-task-{suffix}"), 'a'),
                        },
                    },
                    64,
                );
                RecoveredOperationObservation::fixture_for_handle(
                    ControlRecordCoordinate::new(
                        reference.clone(),
                        ControlRecordDigest::parse(
                            handle.canonical_operation_handle_sha256().as_str(),
                        )
                        .unwrap(),
                    ),
                    (dispatched_ref.clone(), dispatched.clone()),
                    (dispatched_ref, dispatched),
                    (
                        record_ref(&format!("operation/consult-task-{suffix}-checkpoint")),
                        checkpoint.clone(),
                    ),
                    store_coordinate(),
                )
            };
            let uncertain_initial = release_child(&dispatch_batch.candidates()[2]);
            let (uncertain_initial_ref, uncertain_initial_observation) =
                uncertain_initial.selected();
            let uncertain_observation_ref = record_ref("observation/consult-task-mixed-uncertain");
            let uncertain_observation = issue_observation(
                retained.signing,
                &uncertain_child_ref,
                &uncertain_child,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((uncertain_initial_ref, uncertain_initial_observation)),
                OperationObservationStateV1::OutcomeUnknown {
                    diagnostic: immutable("diagnostic/consult-task-uncertain", '5'),
                    reconciliation: ReconciliationFrontierReferenceV1 {
                        record_ref: record_ref("reconciliation/consult-task-uncertain"),
                        record_sha256: ReconciliationFrontierDigest::parse(sha('6')).unwrap(),
                    },
                    effect_receipt: None,
                },
                64,
            );
            let terminal_initial = release_child(&dispatch_batch.candidates()[3]);
            let (terminal_initial_ref, terminal_initial_observation) = terminal_initial.selected();
            let terminal_observation_ref = record_ref("observation/consult-task-mixed-terminal");
            let terminal_observation = issue_observation(
                retained.signing,
                &terminal_child_ref,
                &terminal_child,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((terminal_initial_ref, terminal_initial_observation)),
                OperationObservationStateV1::Failed {
                    result: None,
                    diagnostic: immutable("diagnostic/consult-task-terminal", '7'),
                },
                64,
            );
            let mixed_restart_child = |candidate: &ConsultationChildDispatchCandidate| {
                if candidate.child() == &child_coordinate {
                    release_child(candidate)
                } else if candidate.child() == &optional_child_coordinate {
                    retain_child(candidate)
                } else if candidate.child() == &uncertain_child_coordinate {
                    RecoveredOperationObservation::fixture_for_handle(
                        ControlRecordCoordinate::new(
                            uncertain_child_ref.clone(),
                            ControlRecordDigest::parse(
                                uncertain_child.canonical_operation_handle_sha256().as_str(),
                            )
                            .unwrap(),
                        ),
                        (
                            uncertain_observation_ref.clone(),
                            uncertain_observation.clone(),
                        ),
                        (
                            uncertain_observation_ref.clone(),
                            uncertain_observation.clone(),
                        ),
                        (
                            record_ref("operation/consult-task-mixed-uncertain-checkpoint"),
                            checkpoint.clone(),
                        ),
                        store_coordinate(),
                    )
                } else if candidate.child() == &terminal_child_coordinate {
                    RecoveredOperationObservation::fixture_for_handle(
                        ControlRecordCoordinate::new(
                            terminal_child_ref.clone(),
                            ControlRecordDigest::parse(
                                terminal_child.canonical_operation_handle_sha256().as_str(),
                            )
                            .unwrap(),
                        ),
                        (
                            terminal_observation_ref.clone(),
                            terminal_observation.clone(),
                        ),
                        (
                            terminal_observation_ref.clone(),
                            terminal_observation.clone(),
                        ),
                        (
                            record_ref("operation/consult-task-mixed-terminal-checkpoint"),
                            checkpoint.clone(),
                        ),
                        store_coordinate(),
                    )
                } else {
                    panic!("mixed restart must stay inside the frozen batch")
                }
            };
            let task_limits = service::task::ResidentConsultationTaskLimits::new(
                std::num::NonZeroUsize::new(1).unwrap(),
                std::num::NonZeroUsize::new(4).unwrap(),
            );
            let mut bounded_task_service = service::task::ResidentConsultationTaskService::new(
                dispatch_batch.clone(),
                task_limits,
            )
            .expect("finite two-member task service");
            let mut bounded_executor = ConsultationTaskExecutor::default();
            let mut outsider = dispatch_batch.candidates()[0].clone();
            outsider.slot_id = ConsultationSlotId::parse("not-in-frozen-batch").unwrap();
            assert!(matches!(
                bounded_task_service.retain_and_accept(
                    &outsider,
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || -> Result<_, std::convert::Infallible> {
                        panic!("an outsider must stop before parent lookup")
                    },
                    |_| -> Result<_, std::convert::Infallible> {
                        panic!("an outsider must stop before child recovery")
                    },
                    |_, _| -> service::task::ConsultationChildRetentionOutcome<
                        u8,
                        std::convert::Infallible,
                    > { panic!("an outsider must stop before retention") },
                    &mut bounded_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::NotPlanned(ref candidate)
                            if candidate == &outsider
                    )
            ));
            assert_eq!(bounded_task_service.in_flight(), 0);
            assert_eq!(bounded_executor.accept_calls, 0);

            let stale_recovery_calls = std::cell::Cell::new(0);
            let stale_retention_calls = std::cell::Cell::new(0);
            assert!(matches!(
                bounded_task_service.retain_and_accept(
                    &dispatch_batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(65).unwrap()),
                    || Ok::<_, std::convert::Infallible>(sibling_joined.clone()),
                    |_| {
                        stale_recovery_calls.set(stale_recovery_calls.get() + 1);
                        Ok::<_, std::convert::Infallible>(release_child(
                            &dispatch_batch.candidates()[0],
                        ))
                    },
                    |_, _| {
                        stale_retention_calls.set(stale_retention_calls.get() + 1);
                        service::task::ConsultationChildRetentionOutcome::<
                            u8,
                            std::convert::Infallible,
                        >::Retained {
                            child: Box::new(retain_child(&dispatch_batch.candidates()[0])),
                            work: 0,
                        }
                    },
                    &mut bounded_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::Release(
                            service::ConsultationChildDispatchReleaseFailure::Revalidation(
                                ConsultationCollectionError::Association
                            )
                        )
                    )
            ));
            assert_eq!(stale_recovery_calls.get(), 0);
            assert_eq!(stale_retention_calls.get(), 0);
            assert_eq!(bounded_task_service.in_flight(), 0);
            assert_eq!(bounded_executor.accept_calls, 0);
            assert!(bounded_task_service
                .retain_and_accept(
                    &dispatch_batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |candidate| { Ok::<_, std::convert::Infallible>(release_child(candidate)) },
                    |candidate, _| service::task::ConsultationChildRetentionOutcome::<
                        u8,
                        std::convert::Infallible,
                    >::Retained {
                        child: Box::new(retain_child(candidate)),
                        work: 1,
                    },
                    &mut bounded_executor,
                )
                .is_ok());
            assert!(matches!(
                bounded_task_service.retain_and_accept(
                    &dispatch_batch.candidates()[1],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || -> Result<_, std::convert::Infallible> {
                        panic!("saturation must stop before parent lookup")
                    },
                    |_| -> Result<_, std::convert::Infallible> {
                        panic!("saturation must stop before child recovery")
                    },
                    |_, _| -> service::task::ConsultationChildRetentionOutcome<
                        u8,
                        std::convert::Infallible,
                    > { panic!("saturation must stop before retention") },
                    &mut bounded_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::Saturated(ref candidate)
                            if candidate == &dispatch_batch.candidates()[1]
                    )
            ));
            assert_eq!(bounded_executor.accept_calls, 1);

            bounded_executor.complete(0, 11);
            assert!(matches!(
                bounded_task_service.try_completion(&mut bounded_executor),
                Some(service::task::ResidentConsultationTaskPoll::Completed {
                    output: 11,
                    cancellation: ResidentOperationTaskCancellationState::Unnotified,
                    ..
                })
            ));
            assert!(bounded_task_service
                .retain_and_accept(
                    &dispatch_batch.candidates()[1],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |candidate| { Ok::<_, std::convert::Infallible>(release_child(candidate)) },
                    |candidate, _| service::task::ConsultationChildRetentionOutcome::<
                        u8,
                        std::convert::Infallible,
                    >::Retained {
                        child: Box::new(retain_child(candidate)),
                        work: 2,
                    },
                    &mut bounded_executor,
                )
                .is_ok());
            assert_eq!(bounded_executor.accept_calls, 2);

            let restarted_task_service = service::task::ResidentConsultationTaskService::new(
                dispatch_batch.clone(),
                task_limits,
            )
            .expect("cold-restarted two-member task service");
            for candidate in dispatch_batch.candidates() {
                assert_eq!(
                    restarted_task_service
                        .classify_after_restart(
                            candidate,
                            &FixedClock(UnixTimeV1::new(64).unwrap()),
                            || Ok::<_, std::convert::Infallible>(recovered.clone()),
                            |candidate| {
                                Ok::<_, std::convert::Infallible>(retain_child(candidate))
                            },
                        )
                        .expect("every released roster child must classify after restart"),
                    ProtectedConsultationChildPhase::ResumeOnly,
                );
            }

            let mixed_restart_service = service::task::ResidentConsultationTaskService::new(
                dispatch_batch.clone(),
                task_limits,
            )
            .expect("mixed cold-restart service");
            let expected_mixed_phases = [
                ProtectedConsultationChildPhase::EligibleForFirstDispatch,
                ProtectedConsultationChildPhase::ResumeOnly,
                ProtectedConsultationChildPhase::ResumeOnly,
                ProtectedConsultationChildPhase::TerminalJoin,
            ];
            for (candidate, expected) in dispatch_batch
                .candidates()
                .iter()
                .zip(expected_mixed_phases)
            {
                assert_eq!(
                    mixed_restart_service
                        .classify_after_restart(
                            candidate,
                            &FixedClock(UnixTimeV1::new(64).unwrap()),
                            || Ok::<_, std::convert::Infallible>(recovered.clone()),
                            |candidate| {
                                Ok::<_, std::convert::Infallible>(mixed_restart_child(candidate))
                            },
                        )
                        .expect("each mixed roster child classifies independently"),
                    expected,
                );
            }

            let parallel_limits = service::task::ResidentConsultationTaskLimits::new(
                std::num::NonZeroUsize::new(2).unwrap(),
                std::num::NonZeroUsize::new(4).unwrap(),
            );
            let mut partial_task_service = service::task::ResidentConsultationTaskService::new(
                dispatch_batch.clone(),
                parallel_limits,
            )
            .expect("two-member partial-completion service");
            let mut partial_executor = ConsultationTaskExecutor::default();
            for (index, candidate) in dispatch_batch.candidates().iter().take(2).enumerate() {
                assert!(partial_task_service
                    .retain_and_accept(
                        candidate,
                        &FixedClock(UnixTimeV1::new(64).unwrap()),
                        || Ok::<_, std::convert::Infallible>(recovered.clone()),
                        |candidate| { Ok::<_, std::convert::Infallible>(release_child(candidate)) },
                        |candidate, _| service::task::ConsultationChildRetentionOutcome::<
                            u8,
                            std::convert::Infallible,
                        >::Retained {
                            child: Box::new(retain_child(candidate)),
                            work: u8::try_from(index).unwrap(),
                        },
                        &mut partial_executor,
                    )
                    .is_ok());
            }
            assert_eq!(partial_task_service.in_flight(), 2);
            partial_executor.complete(0, 21);
            assert!(matches!(
                partial_task_service.try_completion(&mut partial_executor),
                Some(service::task::ResidentConsultationTaskPoll::Completed {
                    ref candidate,
                    output: 21,
                    ..
                }) if candidate == &dispatch_batch.candidates()[0]
            ));
            assert_eq!(partial_task_service.in_flight(), 1);

            let recovered_predecessor = ConsultationCollectionPredecessor::recovered(&recovered);
            assert_eq!(
                recovered_predecessor.frontier.coordinate(),
                prepared.journal().frontier().coordinate()
            );

            let child_handle_coordinate = ControlRecordCoordinate::new(
                child_ref.clone(),
                ControlRecordDigest::parse(child.canonical_operation_handle_sha256().as_str())
                    .unwrap(),
            );
            let child_initial_ref = record_ref("observation/consult-child-1");
            let child_initial = issue_observation(
                retained.signing,
                &child_ref,
                &child,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                63,
            );
            let child_result_ref = record_ref("result/consult-child");
            let child_result_sha256 = CapabilityResultDigest::parse(sha('f')).unwrap();
            let child_terminal_ref = record_ref("observation/consult-child-2");
            let child_terminal = issue_observation(
                retained.signing,
                &child_ref,
                &child,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&child_initial_ref, &child_initial)),
                OperationObservationStateV1::Succeeded {
                    result: CapabilityResultReferenceV1 {
                        record_ref: child_result_ref.clone(),
                        record_sha256: child_result_sha256.clone(),
                    },
                    effect_receipt: None,
                },
                64,
            );
            let recovered_child = RecoveredOperationObservation::fixture_for_handle(
                child_handle_coordinate.clone(),
                (child_terminal_ref.clone(), child_terminal.clone()),
                (child_terminal_ref.clone(), child_terminal.clone()),
                (
                    record_ref("operation/consult-child-checkpoint"),
                    checkpoint.clone(),
                ),
                store_coordinate(),
            );
            let joined = recovered
                .admit_recovered_child_terminal(
                    &ConsultationSlotId::parse("fresh-assessment").unwrap(),
                    &recovered_child,
                    UnixTimeV1::new(65).unwrap(),
                    limits,
                    |slot, evidence| {
                        assert_eq!(slot.slot_id().as_str(), "fresh-assessment");
                        assert_eq!(evidence.record_ref, child_result_ref);
                        assert_eq!(
                            evidence.record_sha256.as_str(),
                            child_result_sha256.as_str()
                        );
                        Ok(())
                    },
                )
                .expect("protected successful child joins its exact roster slot");
            assert!(matches!(
                joined.slots()[1].disposition(),
                ConsultationSlotDisposition::Fulfilled { evidence, recorded_at }
                    if evidence.record_ref == child_result_ref
                        && *recorded_at == UnixTimeV1::new(65).unwrap()
            ));

            let child_failure_ref = record_ref("observation/consult-child-failed");
            let child_failure = issue_observation(
                retained.signing,
                &child_ref,
                &child,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&child_initial_ref, &child_initial)),
                OperationObservationStateV1::Failed {
                    result: None,
                    diagnostic: immutable("diagnostic/consult-child-failed", 'e'),
                },
                64,
            );
            let recovered_failure = RecoveredOperationObservation::fixture_for_handle(
                child_handle_coordinate.clone(),
                (child_failure_ref.clone(), child_failure.clone()),
                (child_failure_ref, child_failure),
                (
                    record_ref("operation/consult-child-failed-checkpoint"),
                    checkpoint.clone(),
                ),
                store_coordinate(),
            );
            let unavailable = recovered
                .admit_recovered_child_terminal(
                    &ConsultationSlotId::parse("fresh-assessment").unwrap(),
                    &recovered_failure,
                    UnixTimeV1::new(65).unwrap(),
                    limits,
                    |_, reason| {
                        assert_eq!(reason, &immutable("diagnostic/consult-child-failed", 'e'));
                        Ok(())
                    },
                )
                .expect("protected failed child records exact missing coverage");
            assert!(matches!(
                unavailable.slots()[1].disposition(),
                ConsultationSlotDisposition::Unfulfillable { reason, recorded_at }
                    if reason == &immutable("diagnostic/consult-child-failed", 'e')
                        && *recorded_at == UnixTimeV1::new(65).unwrap()
            ));

            let cancelled_diagnostic = immutable("diagnostic/consult-child-cancelled", 'd');
            let cancelled_ref = record_ref("observation/consult-child-cancelled");
            let cancelled = issue_observation(
                retained.signing,
                &child_ref,
                &child,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&child_initial_ref, &child_initial)),
                OperationObservationStateV1::Cancelled {
                    diagnostic: cancelled_diagnostic.clone(),
                },
                64,
            );
            let recovered_cancelled = RecoveredOperationObservation::fixture_for_handle(
                child_handle_coordinate.clone(),
                (cancelled_ref.clone(), cancelled.clone()),
                (cancelled_ref, cancelled),
                (
                    record_ref("operation/consult-child-cancelled-checkpoint"),
                    checkpoint.clone(),
                ),
                store_coordinate(),
            );
            let cancelled_slot = recovered
                .admit_recovered_child_terminal(
                    &ConsultationSlotId::parse("fresh-assessment").unwrap(),
                    &recovered_cancelled,
                    UnixTimeV1::new(65).unwrap(),
                    limits,
                    |_, reason| {
                        assert_eq!(reason, &cancelled_diagnostic);
                        Ok(())
                    },
                )
                .expect("protected cancelled child records its exact diagnostic");
            assert!(matches!(
                cancelled_slot.slots()[1].disposition(),
                ConsultationSlotDisposition::Unfulfillable { reason, recorded_at }
                    if reason == &cancelled_diagnostic
                        && *recorded_at == UnixTimeV1::new(65).unwrap()
            ));

            let still_open_states = [
                OperationObservationStateV1::CancellationRequested {
                    request: immutable("cancellation/consult-child", '1'),
                },
                OperationObservationStateV1::Recovering {
                    reconciliation: ReconciliationFrontierReferenceV1 {
                        record_ref: record_ref("reconciliation/consult-child"),
                        record_sha256: ReconciliationFrontierDigest::parse(sha('2')).unwrap(),
                    },
                },
                OperationObservationStateV1::OutcomeUnknown {
                    diagnostic: immutable("diagnostic/consult-child-unknown", '3'),
                    reconciliation: ReconciliationFrontierReferenceV1 {
                        record_ref: record_ref("reconciliation/consult-child-unknown"),
                        record_sha256: ReconciliationFrontierDigest::parse(sha('4')).unwrap(),
                    },
                    effect_receipt: None,
                },
                OperationObservationStateV1::Expired {
                    diagnostic: immutable("diagnostic/consult-child-expired", '5'),
                },
            ];
            for (offset, state) in still_open_states.into_iter().enumerate() {
                let open_ref = record_ref(&format!("observation/consult-child-open-{offset}"));
                let open = issue_observation(
                    retained.signing,
                    &child_ref,
                    &child,
                    observation_schema.clone(),
                    OperationObservationRevisionV1::new(2).unwrap(),
                    Some((&child_initial_ref, &child_initial)),
                    state,
                    64,
                );
                let recovered_open = RecoveredOperationObservation::fixture_for_handle(
                    child_handle_coordinate.clone(),
                    (open_ref.clone(), open.clone()),
                    (open_ref, open),
                    (
                        record_ref(&format!("operation/consult-child-open-{offset}-checkpoint")),
                        checkpoint.clone(),
                    ),
                    store_coordinate(),
                );
                let policy_calls = std::cell::Cell::new(0);
                assert!(matches!(
                    recovered.admit_recovered_child_terminal(
                        &ConsultationSlotId::parse("fresh-assessment").unwrap(),
                        &recovered_open,
                        UnixTimeV1::new(65).unwrap(),
                        limits,
                        |_, _| {
                            policy_calls.set(policy_calls.get() + 1);
                            Ok(())
                        },
                    ),
                    Err(ConsultationCollectionError::Contribution)
                ));
                assert_eq!(
                    policy_calls.get(),
                    0,
                    "an open child history cannot reach consultation policy admission"
                );
            }

            let other_store_child = RecoveredOperationObservation::fixture_for_handle(
                ControlRecordCoordinate::new(
                    child_ref.clone(),
                    ControlRecordDigest::parse(child.canonical_operation_handle_sha256().as_str())
                        .unwrap(),
                ),
                (
                    record_ref("observation/other-store-child"),
                    child_initial.clone(),
                ),
                (
                    record_ref("observation/other-store-child"),
                    child_initial.clone(),
                ),
                (
                    record_ref("operation/other-store-child-checkpoint"),
                    checkpoint.clone(),
                ),
                substituted_store_coordinate(),
            );
            assert!(matches!(
                recovered.admit_recovered_child_terminal(
                    &ConsultationSlotId::parse("fresh-assessment").unwrap(),
                    &other_store_child,
                    UnixTimeV1::new(65).unwrap(),
                    limits,
                    |_, _| panic!("another store cannot contribute to this roster"),
                ),
                Err(ConsultationCollectionError::Child)
            ));
            assert!(matches!(
                recovered.admit_recovered_child_terminal(
                    &ConsultationSlotId::parse("fresh-assessment").unwrap(),
                    &RecoveredOperationObservation::fixture_for_handle(
                        ControlRecordCoordinate::new(
                            record_ref("operation/substituted-child"),
                            ControlRecordDigest::parse(
                                child.canonical_operation_handle_sha256().as_str()
                            )
                            .unwrap(),
                        ),
                        (record_ref("observation/substituted"), child_initial.clone()),
                        (record_ref("observation/substituted"), child_initial.clone()),
                        (
                            record_ref("operation/substituted-checkpoint"),
                            checkpoint.clone()
                        ),
                        store_coordinate(),
                    ),
                    UnixTimeV1::new(65).unwrap(),
                    limits,
                    |_, _| panic!("substituted child must be rejected before policy admission"),
                ),
                Err(ConsultationCollectionError::Child)
            ));

            let stopped = collection
                .stop(
                    prepared.journal(),
                    ConsultationStopCause::Deadline,
                    immutable("stop/deadline", 'd'),
                    UnixTimeV1::new(75).unwrap(),
                    limits,
                    |_, summary| {
                        assert!(!summary.requirements_met());
                        assert_eq!(
                            summary.required_outstanding(),
                            &[ConsultationSlotId::parse("fresh-assessment").unwrap()]
                        );
                        Ok(())
                    },
                )
                .expect("deadline stops but does not satisfy missing evidence");
            let stopped_summary = stopped.stopped().unwrap().summary().clone();
            let stopped_journal = successor_journal(&stopped, &issued, 3, 1, limits);
            let late = stopped
                .admit_child_contribution(
                    &stopped_journal,
                    ConsultationContribution {
                        slot_id: ConsultationSlotId::parse("fresh-assessment").unwrap(),
                        child: child_coordinate.clone(),
                        evidence: immutable("evidence/late", 'e'),
                        recorded_at: UnixTimeV1::new(76).unwrap(),
                    },
                    limits,
                    |_, _| Ok(()),
                )
                .expect("late outcome remains attributable");
            assert_eq!(late.stopped().unwrap().summary(), &stopped_summary);
            assert_eq!(late.late_outcomes().len(), 1);
            assert!(matches!(
                late.late_outcomes()[0].disposition(),
                LateConsultationDisposition::Fulfilled { evidence }
                    if evidence == &immutable("evidence/late", 'e')
            ));

            let late_failure = stopped
                .mark_unfulfillable(
                    &stopped_journal,
                    &ConsultationSlotId::parse("fresh-assessment").unwrap(),
                    immutable("diagnostic/late-child-failed", '6'),
                    UnixTimeV1::new(76).unwrap(),
                    limits,
                    |slot, reason| {
                        assert_eq!(slot.slot_id().as_str(), "fresh-assessment");
                        assert_eq!(reason, &immutable("diagnostic/late-child-failed", '6'));
                        Ok(())
                    },
                )
                .expect("late child failure remains attributable");
            assert_eq!(
                late_failure.stopped().unwrap().summary(),
                &stopped_summary,
                "a late failure cannot rewrite the frozen summary"
            );
            assert!(matches!(
                late_failure.slots()[1].disposition(),
                ConsultationSlotDisposition::Outstanding
            ));
            assert!(matches!(
                late_failure.late_outcomes()[0].disposition(),
                LateConsultationDisposition::Unfulfillable { reason }
                    if reason == &immutable("diagnostic/late-child-failed", '6')
            ));
            assert!(matches!(
                late_failure.mark_unfulfillable(
                    &successor_journal(&late_failure, &issued, 4, 3, limits),
                    &ConsultationSlotId::parse("fresh-assessment").unwrap(),
                    immutable("diagnostic/late-child-duplicate", '7'),
                    UnixTimeV1::new(77).unwrap(),
                    limits,
                    |_, _| Ok(()),
                ),
                Err(ConsultationCollectionError::Contribution)
            ));

            let stopped_at_expiry = collection
                .stop(
                    prepared.journal(),
                    ConsultationStopCause::OperationExpiry,
                    immutable("stop/operation-expiry", '3'),
                    UnixTimeV1::new(80).unwrap(),
                    limits,
                    |_, summary| {
                        assert!(!summary.requirements_met());
                        assert_eq!(
                            summary.required_outstanding(),
                            &[ConsultationSlotId::parse("fresh-assessment").unwrap()]
                        );
                        Ok(())
                    },
                )
                .expect("operation expiry can be recorded at its boundary");
            assert!(!stopped_at_expiry
                .stopped()
                .unwrap()
                .summary()
                .requirements_met());

            let stopped_after_expiry = collection
                .stop(
                    prepared.journal(),
                    ConsultationStopCause::OperationExpiry,
                    immutable("stop/observed-operation-expiry", '4'),
                    UnixTimeV1::new(81).unwrap(),
                    limits,
                    |_, summary| {
                        assert!(!summary.requirements_met());
                        Ok(())
                    },
                )
                .expect("operation expiry can be recorded during retention");
            let expiry_summary = stopped_after_expiry.stopped().unwrap().summary().clone();
            let mut expiry_stop_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                physical: local_start::absent_start_store(request),
            };
            let prepared_expiry_stop = stopped_after_expiry
                .prepare_observation(
                    &issued,
                    (&record_ref("operation/checkpoint-1"), &checkpoint),
                    prepared.observation(),
                    ConsultationCollectionObservationInput {
                        observation_ref: record_ref("observation/consultation-expired-stop"),
                        predecessor_collection: Some(ConsultationCollectionPredecessor::prepared(
                            prepared.journal(),
                        )),
                        observed_at: UnixTimeV1::new(81).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                    },
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut expiry_stop_custody,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .expect("retained expiry stop observation");
            let retained_late = stopped_after_expiry
                .admit_child_contribution(
                    prepared_expiry_stop.journal(),
                    ConsultationContribution {
                        slot_id: ConsultationSlotId::parse("fresh-assessment").unwrap(),
                        child: child_coordinate.clone(),
                        evidence: immutable("evidence/retained-late", '5'),
                        recorded_at: UnixTimeV1::new(119).unwrap(),
                    },
                    limits,
                    |_, _| Ok(()),
                )
                .expect("late outcome remains attributable before retention ends");
            assert_eq!(retained_late.stopped().unwrap().summary(), &expiry_summary);
            assert_eq!(retained_late.late_outcomes().len(), 1);
            assert!(matches!(
                retained_late.slots()[1].disposition(),
                ConsultationSlotDisposition::Outstanding
            ));
            let mut retained_late_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(4).unwrap(),
                physical: local_start::absent_start_store(request),
            };
            let prepared_retained_late = retained_late
                .prepare_observation(
                    &issued,
                    (&record_ref("operation/checkpoint-1"), &checkpoint),
                    prepared_expiry_stop.observation(),
                    ConsultationCollectionObservationInput {
                        observation_ref: record_ref("observation/consultation-retained-late"),
                        predecessor_collection: Some(ConsultationCollectionPredecessor::prepared(
                            prepared_expiry_stop.journal(),
                        )),
                        observed_at: UnixTimeV1::new(119).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                    },
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut retained_late_custody,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .expect("late outcome retains the stopped operation phase");
            assert_eq!(
                prepared_retained_late
                    .journal()
                    .append()
                    .entry()
                    .sequence()
                    .get(),
                3
            );
            let OperationObservationStateV1::Active {
                phase:
                    OperationActivePhaseV1::ConsultationCollected {
                        evidence: stopped_evidence,
                    },
            } = prepared_expiry_stop.observation().1.state()
            else {
                panic!("expiry stop phase")
            };
            let OperationObservationStateV1::Active {
                phase:
                    OperationActivePhaseV1::ConsultationCollected {
                        evidence: retained_evidence,
                    },
            } = prepared_retained_late.observation().1.state()
            else {
                panic!("retained late phase")
            };
            assert_eq!(retained_evidence, stopped_evidence);
            assert_ne!(
                &retained_evidence.record_ref,
                prepared_retained_late
                    .journal()
                    .frontier()
                    .coordinate()
                    .record_ref(),
                "late custody advances privately without rewriting the stop result"
            );
            let substituted_stop = issue_observation(
                retained.signing,
                issued.handle().0,
                issued.handle().1,
                observation_schema.clone(),
                *prepared_expiry_stop.observation().1.revision(),
                Some(prepared.observation()),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ConsultationCollected {
                        evidence: ImmutableRecordReferenceV1 {
                            record_ref: record_ref("consultation/substituted-frozen-stop"),
                            record_sha256: stopped_evidence.record_sha256.clone(),
                        },
                    },
                },
                81,
            );
            assert!(matches!(
                retained_late.prepare_observation(
                    &issued,
                    (&record_ref("operation/checkpoint-1"), &checkpoint),
                    (
                        &record_ref("observation/substituted-frozen-stop"),
                        &substituted_stop,
                    ),
                    ConsultationCollectionObservationInput {
                        observation_ref: record_ref(
                            "observation/consultation-substituted-frozen-stop"
                        ),
                        predecessor_collection: Some(ConsultationCollectionPredecessor::prepared(
                            prepared_expiry_stop.journal(),
                        )),
                        observed_at: UnixTimeV1::new(119).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                    },
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut retained_late_custody,
                    |_| panic!("frozen stop substitution must reject before semantic admission"),
                    |_, _, _, _| panic!("frozen stop substitution must reject before rotation"),
                    |_, _, _| panic!("frozen stop substitution must reject before signing"),
                ),
                Err(ConsultationCollectionError::Association)
            ));
            let retained_frontier_evidence =
                pale_ports::OperationCollectionFrontierEvidence::admit(
                    prepared_retained_late.journal().frontier(),
                    &store_coordinate(),
                    prepared_retained_late.journal().operation_handle(),
                    limits.maximum_frontier_bytes,
                )
                .expect("retained late frontier");
            let stopped_frontier_evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
                prepared_expiry_stop.journal().frontier(),
                &store_coordinate(),
                prepared_expiry_stop.journal().operation_handle(),
                limits.maximum_frontier_bytes,
            )
            .expect("historical frozen stop frontier");
            let stale_stop_snapshot = pale_ports::OperationCollectionSnapshot::new(
                prepared_expiry_stop.journal().frontier().clone(),
                &stopped_frontier_evidence,
                pale_ports::OperationCollectionHeadObservation::for_handle(
                    prepared_expiry_stop.journal().operation_handle(),
                    prepared_retained_late
                        .journal()
                        .frontier()
                        .coordinate()
                        .clone(),
                ),
                prepared_expiry_stop
                    .journal()
                    .append()
                    .entry()
                    .bytes()
                    .clone(),
            )
            .expect("historical stop bytes remain physically self-consistent");
            let mut stale_stop_recovery = CollectionRecoveryOnly(Some(stale_stop_snapshot));
            assert!(matches!(
                ConsultationCollection::recover_current(
                    &issued,
                    prepared_retained_late.observation().1,
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut stale_stop_recovery,
                    |_| panic!("stale stop must reject before semantic admission"),
                ),
                Err(ConsultationCollectionError::Association)
            ));
            let mut substituted_late_payload = pale_core::canon::parse_strict(
                prepared_retained_late
                    .journal()
                    .append()
                    .entry()
                    .bytes()
                    .as_slice(),
            )
            .unwrap();
            substituted_late_payload["late_outcomes"][0]["disposition"]["evidence"]["record_ref"] =
                serde_json::Value::String("evidence/substituted-late".into());
            let substituted_late_payload = pale_ports::BoundedAuthorityEntryBytes::new(
                pale_core::canon::to_canonical_string(&substituted_late_payload)
                    .unwrap()
                    .into_bytes(),
                limits.maximum_journal_bytes,
            )
            .unwrap();
            assert_eq!(
                pale_ports::OperationCollectionSnapshot::new(
                    prepared_retained_late.journal().frontier().clone(),
                    &retained_frontier_evidence,
                    pale_ports::OperationCollectionHeadObservation::for_handle(
                        prepared_retained_late.journal().operation_handle(),
                        prepared_retained_late
                            .journal()
                            .frontier()
                            .coordinate()
                            .clone(),
                    ),
                    substituted_late_payload,
                ),
                Err(pale_ports::OperationCollectionJournalError::JournalPayload),
                "changed late outcome must fail its physical digest before semantic admission"
            );
            let retained_snapshot = pale_ports::OperationCollectionSnapshot::new(
                prepared_retained_late.journal().frontier().clone(),
                &retained_frontier_evidence,
                pale_ports::OperationCollectionHeadObservation::for_handle(
                    prepared_retained_late.journal().operation_handle(),
                    prepared_retained_late
                        .journal()
                        .frontier()
                        .coordinate()
                        .clone(),
                ),
                prepared_retained_late
                    .journal()
                    .append()
                    .entry()
                    .bytes()
                    .clone(),
            )
            .expect("retained late snapshot");
            let mut retained_recovery = CollectionRecoveryOnly(Some(retained_snapshot));
            let recovered_retained = ConsultationCollection::recover_current(
                &issued,
                prepared_retained_late.observation().1,
                limits,
                &local_start::StartStore(retained.session),
                &mut retained_recovery,
                |_| Ok(()),
            )
            .expect("retained late state recovers against frozen stop evidence")
            .expect("collected consultation has retained state");
            assert_eq!(recovered_retained.collection(), &retained_late);
            assert_eq!(
                recovered_retained.collection().stopped().unwrap().summary(),
                &expiry_summary
            );

            assert!(matches!(
                collection.admit_child_contribution(
                    prepared.journal(),
                    ConsultationContribution {
                        slot_id: ConsultationSlotId::parse("fresh-assessment").unwrap(),
                        child: child_coordinate.clone(),
                        evidence: immutable("evidence/after-execution", '6'),
                        recorded_at: UnixTimeV1::new(81).unwrap(),
                    },
                    limits,
                    |_, _| panic!("expired collection must reject before admission")
                ),
                Err(ConsultationCollectionError::Stopped)
            ));
            assert!(matches!(
                stopped_after_expiry.admit_child_contribution(
                    prepared_expiry_stop.journal(),
                    ConsultationContribution {
                        slot_id: ConsultationSlotId::parse("fresh-assessment").unwrap(),
                        child: child_coordinate.clone(),
                        evidence: immutable("evidence/clock-rewind", '7'),
                        recorded_at: UnixTimeV1::new(80).unwrap(),
                    },
                    limits,
                    |_, _| panic!("clock rewind must reject before admission")
                ),
                Err(ConsultationCollectionError::Association)
            ));
            assert!(matches!(
                stopped_after_expiry.admit_child_contribution(
                    prepared.journal(),
                    ConsultationContribution {
                        slot_id: ConsultationSlotId::parse("fresh-assessment").unwrap(),
                        child: child_coordinate.clone(),
                        evidence: immutable("evidence/wrong-frontier", '8'),
                        recorded_at: UnixTimeV1::new(82).unwrap(),
                    },
                    limits,
                    |_, _| panic!("unlike frontier must reject before admission")
                ),
                Err(ConsultationCollectionError::Association)
            ));
            for retained_at in [120, 121] {
                assert!(matches!(
                    stopped_after_expiry.admit_child_contribution(
                        prepared_expiry_stop.journal(),
                        ConsultationContribution {
                            slot_id: ConsultationSlotId::parse("fresh-assessment").unwrap(),
                            child: child_coordinate.clone(),
                            evidence: immutable("evidence/after-retention", '9'),
                            recorded_at: UnixTimeV1::new(retained_at).unwrap(),
                        },
                        limits,
                        |_, _| panic!("retention expiry must reject before admission")
                    ),
                    Err(ConsultationCollectionError::Association)
                ));
            }
            assert!(matches!(
                collection.stop(
                    prepared.journal(),
                    ConsultationStopCause::CompletionPolicy,
                    immutable("stop/policy-after-expiry", 'a'),
                    UnixTimeV1::new(81).unwrap(),
                    limits,
                    |_, _| panic!("policy collection cannot continue after expiry")
                ),
                Err(ConsultationCollectionError::Stopped)
            ));
            assert!(matches!(
                collection.stop(
                    prepared.journal(),
                    ConsultationStopCause::OperationExpiry,
                    immutable("stop/after-retention", 'b'),
                    UnixTimeV1::new(120).unwrap(),
                    limits,
                    |_, _| panic!("retention expiry must reject before admission")
                ),
                Err(ConsultationCollectionError::Association)
            ));

            let fulfilled = collection
                .admit_child_contribution(
                    prepared.journal(),
                    ConsultationContribution {
                        slot_id: ConsultationSlotId::parse("fresh-assessment").unwrap(),
                        child: child_coordinate.clone(),
                        evidence: immutable("evidence/fresh", 'f'),
                        recorded_at: UnixTimeV1::new(64).unwrap(),
                    },
                    limits,
                    |_, _| Ok(()),
                )
                .expect("fresh attributable contribution");
            let fulfilled_journal = successor_journal(&fulfilled, &issued, 3, 1, limits);
            let disagreement = fulfilled
                .record_disagreement(
                    &fulfilled_journal,
                    vec![
                        ConsultationSlotId::parse("fresh-assessment").unwrap(),
                        ConsultationSlotId::parse("existing-evidence").unwrap(),
                    ],
                    immutable("evidence/disagreement", '1'),
                    UnixTimeV1::new(65).unwrap(),
                    limits,
                    |slots, _| {
                        assert_eq!(slots.len(), 2);
                        Ok(())
                    },
                )
                .expect("disagreement remains explicit");
            assert_eq!(disagreement.disagreements().len(), 1);
            assert!(matches!(
                collection.stop(
                    &stopped_journal,
                    ConsultationStopCause::CompletionPolicy,
                    immutable("stop/unrelated", '2'),
                    UnixTimeV1::new(66).unwrap(),
                    limits,
                    |_, _| Ok(())
                ),
                Err(ConsultationCollectionError::Association)
            ));

            let identity = OperationCheckpointInput {
                reference: record_ref("operation/checkpoint-2"),
                id: ControlCheckpointIdV1::parse("operation-checkpoint-2").unwrap(),
            };
            let mut authority_source = crate::local_operation_start_fixture::checkpoint_preimage(
                2,
                Some((
                    record_ref("operation/checkpoint-1"),
                    checkpoint.canonical_control_checkpoint_sha256().clone(),
                )),
            );
            authority_source.checkpoint_domain_id = checkpoint.checkpoint_domain_id().clone();
            authority_source.checkpoint_profile_sha256 =
                crate::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
            let authority = checkpoint_authority(&authority_source);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let prepared = match commit_consultation_collection(
                prepared,
                ConsultationCollectionCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .unwrap(),
                    predecessor: (&record_ref("operation/checkpoint-substituted"), &checkpoint),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/checkpoint-substituted-successor"),
                        id: ControlCheckpointIdV1::parse(
                            "operation-checkpoint-substituted-successor",
                        )
                        .unwrap(),
                    },
                    issued_at: UnixTimeV1::new(63).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &authority,
                },
                &local_start::StartStore(retained.session),
                &mut collection_custody,
                &mut local_start::successor_anchor(),
                |_, _, _| panic!("a substituted predecessor cannot sign a checkpoint"),
            )
            .expect_err("the exact predecessor reference is part of preparation custody")
            {
                ConsultationCollectionCommitFailure::Issue { prepared, .. } => *prepared,
                other => panic!("unexpected substituted-checkpoint phase: {other:?}"),
            };
            assert_eq!(collection_custody.physical.mutation_counts(), (0, 0));
            let mut unavailable_custody = CollectionCommitPort {
                prepare: |_: &OperationAccumulatorPreparation| {
                    Err(pale_ports::OperationAccumulatorPreparationError::Limit)
                },
                transaction: ControlTransactionSequenceV1::new(2).unwrap(),
                physical: local_start::absent_start_store(request),
            };
            let prepared = match commit_consultation_collection(
                prepared,
                ConsultationCollectionCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .unwrap(),
                    predecessor: (&record_ref("operation/checkpoint-1"), &checkpoint),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/checkpoint-unused"),
                        id: ControlCheckpointIdV1::parse("operation-checkpoint-unused").unwrap(),
                    },
                    issued_at: UnixTimeV1::new(63).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &authority,
                },
                &local_start::StartStore(retained.session),
                &mut unavailable_custody,
                &mut local_start::successor_anchor(),
                |_, _, _| panic!("accumulator refusal precedes checkpoint signing"),
            )
            .expect_err("failed native preparation retains the signed collection")
            {
                ConsultationCollectionCommitFailure::Preparation { prepared, reason } => {
                    assert!(matches!(
                        reason,
                        ConsultationCollectionError::Projection(
                            OperationObservationCustodyProjectionError::AccumulatorPreparation(
                                pale_ports::OperationAccumulatorPreparationError::Limit
                            )
                        )
                    ));
                    *prepared
                }
                other => panic!("unexpected stopped phase: {other:?}"),
            };
            let mut observation_anchor = local_start::successor_anchor();
            let completed = commit_consultation_collection(
                prepared,
                ConsultationCollectionCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .unwrap(),
                    predecessor: (&record_ref("operation/checkpoint-1"), &checkpoint),
                    identity,
                    issued_at: UnixTimeV1::new(63).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &authority,
                },
                &local_start::StartStore(retained.session),
                &mut collection_custody,
                &mut observation_anchor,
                |_, _, _| Ok(signature()),
            )
            .expect("collection checkpoint reaches database and anchor custody");
            let pair = completed
                .custody()
                .candidate()
                .collection_journal()
                .expect("collection pair retained");
            assert_eq!(pair, completed.journal());
            assert_eq!(
                pair.frontier(),
                prepared_journal_frontier(completed.custody())
            );
            assert_eq!(
                completed.checkpoint().0,
                &record_ref("operation/checkpoint-2")
            );

            let completed_snapshot = {
                let evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
                    completed.journal().frontier(),
                    &store_coordinate(),
                    completed.journal().operation_handle(),
                    limits.maximum_frontier_bytes,
                )
                .unwrap();
                pale_ports::OperationCollectionSnapshot::new(
                    completed.journal().frontier().clone(),
                    &evidence,
                    pale_ports::OperationCollectionHeadObservation::for_handle(
                        completed.journal().operation_handle(),
                        completed.journal().frontier().coordinate().clone(),
                    ),
                    completed.journal().append().entry().bytes().clone(),
                )
                .unwrap()
            };
            let mut completed_recovery = CollectionRecoveryOnly(Some(completed_snapshot.clone()));
            let recovered_for_cancellation = ConsultationCollection::recover_current(
                &issued,
                completed.observation().1,
                limits,
                &local_start::StartStore(retained.session),
                &mut completed_recovery,
                |_| Ok(()),
            )
            .unwrap()
            .expect("current collection recovers before cancellation");
            let recovered_operation =
                RecoveredLocalOperation::fixture(&issued, request.clone(), store_coordinate());
            let recovered_observation = RecoveredOperationObservation::fixture(
                &issued,
                (
                    completed.observation().0.clone(),
                    completed.observation().1.clone(),
                ),
                (
                    completed.observation().0.clone(),
                    completed.observation().1.clone(),
                ),
                (
                    completed.checkpoint().0.clone(),
                    completed.checkpoint().1.clone(),
                ),
                store_coordinate(),
            );
            let cancel_request = cancellation_request(request, issued.handle().1, 66);
            let authorized = AuthorizedOperationCancellation::fixture(
                &cancel_request,
                &recovered_operation,
                &recovered_observation,
                UnixTimeV1::new(66).unwrap(),
            );
            assert!(matches!(
                authorized.disposition(),
                OperationCancellationDisposition::CollectingConsultation { .. }
            ));
            let mut cancellation_authority_source =
                crate::local_operation_start_fixture::checkpoint_preimage(
                    3,
                    Some((
                        completed.checkpoint().0.clone(),
                        completed
                            .checkpoint()
                            .1
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                );
            cancellation_authority_source.checkpoint_domain_id =
                completed.checkpoint().1.checkpoint_domain_id().clone();
            cancellation_authority_source.checkpoint_profile_sha256 =
                crate::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
            let cancellation_authority = checkpoint_authority(&cancellation_authority_source);

            let integrated_authorized = AuthorizedOperationCancellation::fixture(
                &cancel_request,
                &recovered_operation,
                &recovered_observation,
                UnixTimeV1::new(66).unwrap(),
            );
            let mut integrated_custody = CollectionRecoverAndCommitPort {
                snapshot: Some(completed_snapshot.clone()),
                commit: CollectionCommitPort {
                    prepare: prepare_collection_accumulator,
                    transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                    physical: RecordingStore::committing(
                        pale_ports::OperationCustodyCommitOutcome::Committed,
                    ),
                },
            };
            let mut integrated_anchor = local_start::successor_anchor();
            let mut integrated_tasks = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).unwrap(),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .expect("bounded task state");
            let mut ambient = ProvingAuthority;
            let mut integrated_service = ResidentOperationService::new(
                &mut ambient,
                &mut integrated_custody,
                &mut integrated_anchor,
                &mut integrated_tasks,
            );
            let ResidentAuthorizedCancellationRoute::Consultation(integrated_route) =
                integrated_service.route_authorized_cancellation(integrated_authorized)
            else {
                panic!("collecting consultation must preserve its authorized custody route")
            };
            let mut rotate = |_: &ResidentRecordRefV1,
                              _: &ArtifactDigest,
                              _: &ResidentRecordRefV1,
                              _: &ArtifactDigest| { Ok(()) };
            let mut sign_observation =
                |_: &[u8], _: &OperationObservationPreimageV1, _: &OperationObservationDigest| {
                    Ok(signature())
                };
            let mut sign_checkpoint =
                |_: &[u8],
                 _: &ControlStateCheckpointPreimageV1,
                 _: &ControlStateCheckpointDigest| { Ok(signature()) };
            let mut integrated_admission = CancellationAppendAdmission {
                resolve_store: &local_start::StartStore(retained.session),
                admit_rotation: &mut rotate,
                sign_observation: &mut sign_observation,
                sign_checkpoint: &mut sign_checkpoint,
            };
            let integrated = integrated_service
                .cancel_authorized_consultation(
                    integrated_route,
                    CancellationAppendInput {
                        request_ref: record_ref("request/cancel-consultation-integrated"),
                        observation_ref: record_ref(
                            "observation/consultation-cancellation-integrated",
                        ),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                        checkpoint: OperationCheckpointInput {
                            reference: record_ref("operation/checkpoint-cancel-integrated"),
                            id: ControlCheckpointIdV1::parse(
                                "operation-checkpoint-cancel-integrated",
                            )
                            .unwrap(),
                        },
                        checkpoint_issued_at: UnixTimeV1::new(66).unwrap(),
                        checkpoint_schema: &checkpoint_schema,
                        checkpoint_authority: &cancellation_authority,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            4, 4096, 4096, 512, 65_536,
                        )
                        .unwrap(),
                    },
                    limits,
                    &mut integrated_admission,
                    |_, summary| {
                        assert!(!summary.requirements_met());
                        Ok(())
                    },
                    |_| Ok(()),
                )
                .unwrap_or_else(|failure| {
                    panic!("service-owned consultation cancellation commits: {failure:?}")
                });
            assert_eq!(
                integrated.notification,
                ResidentConsultationCancellationNotification::NoLocalOwner,
                "restart without process-local work cannot block durable cancellation"
            );
            assert_eq!(
                integrated
                    .custody
                    .candidate()
                    .records()
                    .iter()
                    .filter(|record| {
                        record.kind()
                            == pale_ports::OperationCustodyRecordKind::OperationObservation
                    })
                    .count(),
                1
            );
            assert_eq!(integrated_custody.commit.physical.mutation_counts(), (1, 1));
            assert_eq!(integrated_anchor.access_counts(), (1, 1));

            #[derive(Clone, Copy, Debug)]
            enum LateCancellationCase {
                Lost,
                Failed,
                Cancelled,
            }

            for case in [
                LateCancellationCase::Lost,
                LateCancellationCase::Failed,
                LateCancellationCase::Cancelled,
            ] {
                let suffix = match case {
                    LateCancellationCase::Lost => "lost",
                    LateCancellationCase::Failed => "failed",
                    LateCancellationCase::Cancelled => "cancelled",
                };
                let authorized = AuthorizedOperationCancellation::fixture(
                    &cancel_request,
                    &recovered_operation,
                    &recovered_observation,
                    UnixTimeV1::new(66).unwrap(),
                );
                let mut custody = CollectionRecoverAndCommitPort {
                    snapshot: Some(completed_snapshot.clone()),
                    commit: CollectionCommitPort {
                        prepare: prepare_collection_accumulator,
                        transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                        physical: RecordingStore::committing(
                            pale_ports::OperationCustodyCommitOutcome::Committed,
                        ),
                    },
                };
                let mut anchor = local_start::successor_anchor();
                let mut tasks = ResidentOperationTaskState::new(
                    ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                    NonZeroUsize::new(1).unwrap(),
                    ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                )
                .expect("bounded live cancellation task state");
                let mut ambient = ProvingAuthority;
                let mut executor =
                    ConsultationEvidenceExecutor::<RecoveredOperationObservation>::default();
                {
                    let mut service = ResidentOperationService::new(
                        &mut ambient,
                        &mut custody,
                        &mut anchor,
                        &mut tasks,
                    );
                    let coordinate = service
                        .open_consultation_batch(
                            dispatch_batch.clone(),
                            ResidentConsultationTaskLimits::new(
                                NonZeroUsize::new(1).unwrap(),
                                NonZeroUsize::new(4).unwrap(),
                            ),
                        )
                        .expect("exact live cancellation batch");
                    let candidate = &dispatch_batch.candidates()[0];
                    let retained_child = retain_child(candidate);
                    let (retained_ref, retained_observation) = retained_child.selected();
                    let retained_ref = retained_ref.clone();
                    let retained_observation = retained_observation.clone();
                    service
                        .retain_and_accept_consultation_task(
                            &coordinate,
                            candidate,
                            &FixedClock(UnixTimeV1::new(64).unwrap()),
                            || {
                                Ok::<_, std::convert::Infallible>(
                                    recovered_for_cancellation.clone(),
                                )
                            },
                            |_| Ok::<_, std::convert::Infallible>(release_child(candidate)),
                            |_, _| service::task::ConsultationChildRetentionOutcome::<
                                (),
                                std::convert::Infallible,
                            >::Retained {
                                child: Box::new(retained_child),
                                work: (),
                            },
                            &mut executor,
                        )
                        .expect("live child reaches its executor once");
                    let ResidentAuthorizedCancellationRoute::Consultation(route) =
                        service.route_authorized_cancellation(authorized)
                    else {
                        panic!("live consultation must retain its exact cancellation route")
                    };
                    let mut rotate =
                        |_: &ResidentRecordRefV1,
                         _: &ArtifactDigest,
                         _: &ResidentRecordRefV1,
                         _: &ArtifactDigest| { Ok(()) };
                    let mut sign_observation =
                        |_: &[u8],
                         _: &OperationObservationPreimageV1,
                         _: &OperationObservationDigest| {
                            Ok(signature())
                        };
                    let mut sign_checkpoint =
                        |_: &[u8],
                         _: &ControlStateCheckpointPreimageV1,
                         _: &ControlStateCheckpointDigest| {
                            Ok(signature())
                        };
                    let mut admission = CancellationAppendAdmission {
                        resolve_store: &local_start::StartStore(retained.session),
                        admit_rotation: &mut rotate,
                        sign_observation: &mut sign_observation,
                        sign_checkpoint: &mut sign_checkpoint,
                    };
                    let cancelled = service
                        .cancel_authorized_consultation(
                            route,
                            CancellationAppendInput {
                                request_ref: record_ref(&format!(
                                    "request/cancel-consultation-live-{suffix}"
                                )),
                                observation_ref: record_ref(&format!(
                                    "observation/cancel-consultation-live-{suffix}"
                                )),
                                signing: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: &observation_schema,
                                    signing_authority: retained.signing,
                                },
                                checkpoint: OperationCheckpointInput {
                                    reference: record_ref(&format!(
                                        "operation/checkpoint-cancel-live-{suffix}"
                                    )),
                                    id: ControlCheckpointIdV1::parse(format!(
                                        "operation-checkpoint-cancel-live-{suffix}"
                                    ))
                                    .unwrap(),
                                },
                                checkpoint_issued_at: UnixTimeV1::new(66).unwrap(),
                                checkpoint_schema: &checkpoint_schema,
                                checkpoint_authority: &cancellation_authority,
                                accumulator_limits: AccumulatorPreparationLimits::new(
                                    4, 4096, 4096, 512, 65_536,
                                )
                                .unwrap(),
                            },
                            limits,
                            &mut admission,
                            |_, summary| {
                                assert!(!summary.requirements_met());
                                Ok(())
                            },
                            |_| Ok(()),
                        )
                        .unwrap_or_else(|failure| {
                            panic!("live consultation cancellation commits first: {failure:?}")
                        });
                    assert_eq!(
                        cancelled.notification,
                        ResidentConsultationCancellationNotification::BatchMarked {
                            newly_marked: 1,
                            already_marked: 0,
                            no_longer_accepted: 0,
                        },
                        "only completed database and anchor custody can mark local work"
                    );

                    match case {
                        LateCancellationCase::Lost => executor.lose(0),
                        LateCancellationCase::Failed | LateCancellationCase::Cancelled => {
                            let state = match case {
                                LateCancellationCase::Failed => {
                                    OperationObservationStateV1::Failed {
                                        result: None,
                                        diagnostic: immutable(
                                            "diagnostic/consult-live-late-failed",
                                            'f',
                                        ),
                                    }
                                }
                                LateCancellationCase::Cancelled => {
                                    OperationObservationStateV1::Cancelled {
                                        diagnostic: immutable(
                                            "diagnostic/consult-live-late-cancelled",
                                            'c',
                                        ),
                                    }
                                }
                                LateCancellationCase::Lost => unreachable!(),
                            };
                            let late_ref =
                                record_ref(&format!("observation/consult-live-late-{suffix}"));
                            let late = issue_observation(
                                retained.signing,
                                &child_ref,
                                &child,
                                observation_schema.clone(),
                                OperationObservationRevisionV1::new(3).unwrap(),
                                Some((&retained_ref, &retained_observation)),
                                state,
                                67,
                            );
                            executor.complete(
                                0,
                                RecoveredOperationObservation::fixture_for_handle(
                                    child_handle_coordinate.clone(),
                                    (late_ref.clone(), late.clone()),
                                    (late_ref, late),
                                    (
                                        record_ref(&format!(
                                            "operation/consult-live-late-{suffix}-checkpoint"
                                        )),
                                        checkpoint.clone(),
                                    ),
                                    store_coordinate(),
                                ),
                            );
                        }
                    }
                    let Some(Ok(late)) = service.try_consultation_completion(&mut executor) else {
                        panic!("late {suffix} outcome remains retained by the exact batch")
                    };
                    assert_eq!(late.coordinate, coordinate);
                    match (case, late.outcome) {
                        (
                            LateCancellationCase::Lost,
                            service::task::ResidentConsultationTaskPoll::Lost(lost),
                        ) => {
                            assert_eq!(lost.candidate(), candidate);
                            assert_eq!(
                                lost.cancellation_state(),
                                ResidentOperationTaskCancellationState::Notified
                            );
                        }
                        (
                            LateCancellationCase::Failed,
                            service::task::ResidentConsultationTaskPoll::Completed {
                                candidate: actual,
                                output,
                                cancellation: ResidentOperationTaskCancellationState::Notified,
                                ..
                            },
                        ) => {
                            assert_eq!(&actual, candidate);
                            assert!(matches!(
                                output.selected().1.state(),
                                OperationObservationStateV1::Failed { .. }
                            ));
                        }
                        (
                            LateCancellationCase::Cancelled,
                            service::task::ResidentConsultationTaskPoll::Completed {
                                candidate: actual,
                                output,
                                cancellation: ResidentOperationTaskCancellationState::Notified,
                                ..
                            },
                        ) => {
                            assert_eq!(&actual, candidate);
                            assert!(matches!(
                                output.selected().1.state(),
                                OperationObservationStateV1::Cancelled { .. }
                            ));
                        }
                        (case, outcome) => {
                            panic!("late {case:?} outcome lost its attribution: {outcome:?}")
                        }
                    }
                    assert_eq!(
                        service.outstanding_task_reservations(),
                        1,
                        "late evidence remains retained until its durable join or recovery"
                    );
                }
                assert_eq!(custody.commit.physical.mutation_counts(), (1, 1));
                assert_eq!(anchor.access_counts(), (1, 1));
            }

            #[derive(Clone, Copy, Debug)]
            enum CancellationCustodyFailureCase {
                Storage,
                Anchor,
            }

            for failure_case in [
                CancellationCustodyFailureCase::Storage,
                CancellationCustodyFailureCase::Anchor,
            ] {
                let suffix = match failure_case {
                    CancellationCustodyFailureCase::Storage => "storage",
                    CancellationCustodyFailureCase::Anchor => "anchor",
                };
                let physical = match failure_case {
                    CancellationCustodyFailureCase::Storage => RecordingStore::failing_commit(
                        pale_ports::ControlStoreError::new(
                            pale_ports::ControlStoreErrorKind::Storage,
                        ),
                        [
                            Err(pale_ports::ControlStoreError::new(
                                pale_ports::ControlStoreErrorKind::Storage,
                            )),
                            Err(pale_ports::ControlStoreError::new(
                                pale_ports::ControlStoreErrorKind::Storage,
                            )),
                            Ok(pale_ports::OperationCustodyReadBackOutcome::ExactSuccessor),
                        ],
                    ),
                    CancellationCustodyFailureCase::Anchor => RecordingStore::committing(
                        pale_ports::OperationCustodyCommitOutcome::Committed,
                    ),
                };
                let mut custody = CollectionRecoverAndCommitPort {
                    snapshot: Some(completed_snapshot.clone()),
                    commit: CollectionCommitPort {
                        prepare: prepare_collection_accumulator,
                        transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                        physical,
                    },
                };
                let mut anchor = match failure_case {
                    CancellationCustodyFailureCase::Storage => local_start::successor_anchor(),
                    CancellationCustodyFailureCase::Anchor => {
                        RecordingAnchor::unavailable_after_preflight()
                    }
                };
                let mut tasks = ResidentOperationTaskState::new(
                    ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                    NonZeroUsize::new(1).unwrap(),
                    ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                )
                .expect("bounded failed-custody task state");
                let mut ambient = ProvingAuthority;
                let mut executor = ConsultationEvidenceExecutor::<()>::default();
                let candidate = &dispatch_batch.candidates()[0];
                let local_coordinate =
                    if matches!(failure_case, CancellationCustodyFailureCase::Storage) {
                        let mut service = ResidentOperationService::new(
                            &mut ambient,
                            &mut custody,
                            &mut anchor,
                            &mut tasks,
                        );
                        let coordinate = service
                            .open_consultation_batch(
                                dispatch_batch.clone(),
                                ResidentConsultationTaskLimits::new(
                                    NonZeroUsize::new(1).unwrap(),
                                    NonZeroUsize::new(4).unwrap(),
                                ),
                            )
                            .expect("exact failed-custody batch");
                        service
                            .retain_and_accept_consultation_task(
                                &coordinate,
                                candidate,
                                &FixedClock(UnixTimeV1::new(64).unwrap()),
                                || {
                                    Ok::<_, std::convert::Infallible>(
                                        recovered_for_cancellation.clone(),
                                    )
                                },
                                |_| Ok::<_, std::convert::Infallible>(release_child(candidate)),
                                |_, _| service::task::ConsultationChildRetentionOutcome::<
                                    (),
                                    std::convert::Infallible,
                                >::Retained {
                                    child: Box::new(retain_child(candidate)),
                                    work: (),
                                },
                                &mut executor,
                            )
                            .expect("failed-custody child reaches its executor once");
                        Some(coordinate)
                    } else {
                        None
                    };
                let failure_issued = issued.fixture_clone();
                let failure_recovered_operation = RecoveredLocalOperation::fixture(
                    &failure_issued,
                    request.clone(),
                    store_coordinate(),
                );
                let failure_recovered_observation = RecoveredOperationObservation::fixture(
                    &failure_issued,
                    (
                        completed.observation().0.clone(),
                        completed.observation().1.clone(),
                    ),
                    (
                        completed.observation().0.clone(),
                        completed.observation().1.clone(),
                    ),
                    (
                        completed.checkpoint().0.clone(),
                        completed.checkpoint().1.clone(),
                    ),
                    store_coordinate(),
                );
                let failure_cancel_request =
                    cancellation_request(request, failure_issued.handle().1, 66);
                let observation_signatures = Cell::new(0);
                let checkpoint_signatures = Cell::new(0);
                let pending = {
                    let mut service = ResidentOperationService::new(
                        &mut ambient,
                        &mut custody,
                        &mut anchor,
                        &mut tasks,
                    );
                    let authorized = AuthorizedOperationCancellation::fixture(
                        &failure_cancel_request,
                        &failure_recovered_operation,
                        &failure_recovered_observation,
                        UnixTimeV1::new(66).unwrap(),
                    );
                    let ResidentAuthorizedCancellationRoute::Consultation(route) =
                        service.route_authorized_cancellation(authorized)
                    else {
                        panic!("failed-custody case must retain the Consult route")
                    };
                    let mut rotate =
                        |_: &ResidentRecordRefV1,
                         _: &ArtifactDigest,
                         _: &ResidentRecordRefV1,
                         _: &ArtifactDigest| { Ok(()) };
                    let mut sign_observation =
                        |_: &[u8],
                         _: &OperationObservationPreimageV1,
                         _: &OperationObservationDigest| {
                            observation_signatures.set(observation_signatures.get() + 1);
                            Ok(signature())
                        };
                    let mut sign_checkpoint =
                        |_: &[u8],
                         _: &ControlStateCheckpointPreimageV1,
                         _: &ControlStateCheckpointDigest| {
                            checkpoint_signatures.set(checkpoint_signatures.get() + 1);
                            Ok(signature())
                        };
                    let mut admission = CancellationAppendAdmission {
                        resolve_store: &local_start::StartStore(retained.session),
                        admit_rotation: &mut rotate,
                        sign_observation: &mut sign_observation,
                        sign_checkpoint: &mut sign_checkpoint,
                    };
                    match service.cancel_authorized_consultation(
                        route,
                        CancellationAppendInput {
                            request_ref: record_ref(&format!(
                                "request/cancel-consultation-{suffix}-failure"
                            )),
                            observation_ref: record_ref(&format!(
                                "observation/cancel-consultation-{suffix}-failure"
                            )),
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: retained.signing,
                            },
                            checkpoint: OperationCheckpointInput {
                                reference: record_ref(&format!(
                                    "operation/checkpoint-cancel-{suffix}-failure"
                                )),
                                id: ControlCheckpointIdV1::parse(format!(
                                    "operation-checkpoint-cancel-{suffix}-failure"
                                ))
                                .unwrap(),
                            },
                            checkpoint_issued_at: UnixTimeV1::new(66).unwrap(),
                            checkpoint_schema: &checkpoint_schema,
                            checkpoint_authority: &cancellation_authority,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                4, 4096, 4096, 512, 65_536,
                            )
                            .unwrap(),
                        },
                        limits,
                        &mut admission,
                        |_, summary| {
                            assert!(!summary.requirements_met());
                            Ok(())
                        },
                        |_| Ok(()),
                    ) {
                        Err(ResidentConsultationCancellationFailure::Custody(pending)) => pending,
                        Ok(_) => panic!("{failure_case:?} cannot complete before recovery"),
                        Err(failure) => {
                            panic!("{failure_case:?} must retain custody: {failure:?}")
                        }
                    }
                };
                drop(failure_recovered_observation);
                drop(failure_recovered_operation);
                drop(failure_issued);
                if matches!(failure_case, CancellationCustodyFailureCase::Anchor) {
                    assert_eq!(anchor.access_counts(), (1, 1));
                    anchor = local_start::successor_anchor();
                }
                let pending = if matches!(failure_case, CancellationCustodyFailureCase::Storage) {
                    let mut service = ResidentOperationService::new(
                        &mut ambient,
                        &mut custody,
                        &mut anchor,
                        &mut tasks,
                    );
                    match service.continue_consultation_cancellation_custody(pending) {
                        Err(pending) => pending,
                        Ok(_) => panic!("storage uncertainty cannot complete on failed read-back"),
                    }
                } else {
                    pending
                };
                let completed_cancellation = {
                    let mut service = ResidentOperationService::new(
                        &mut ambient,
                        &mut custody,
                        &mut anchor,
                        &mut tasks,
                    );
                    service
                        .continue_consultation_cancellation_custody(pending)
                        .unwrap_or_else(|failure| {
                            panic!("{suffix} custody continuation must complete: {failure:?}")
                        })
                };
                match failure_case {
                    CancellationCustodyFailureCase::Storage => {
                        assert_eq!(
                            completed_cancellation.notification,
                            ResidentConsultationCancellationNotification::BatchMarked {
                                newly_marked: 1,
                                already_marked: 0,
                                no_longer_accepted: 0,
                            }
                        );
                        assert_eq!(custody.commit.physical.mutation_counts(), (0, 3));
                        assert_eq!(anchor.access_counts(), (1, 1));
                    }
                    CancellationCustodyFailureCase::Anchor => {
                        assert_eq!(
                            completed_cancellation.notification,
                            ResidentConsultationCancellationNotification::NoLocalOwner
                        );
                        assert_eq!(custody.commit.physical.mutation_counts(), (1, 1));
                        assert_eq!(anchor.access_counts(), (1, 0));
                    }
                }
                assert_eq!(observation_signatures.get(), 1);
                assert_eq!(checkpoint_signatures.get(), 1);
                if let Some(coordinate) = local_coordinate {
                    executor.complete(0, ());
                    let mut service = ResidentOperationService::new(
                        &mut ambient,
                        &mut custody,
                        &mut anchor,
                        &mut tasks,
                    );
                    let Some(Ok(late)) = service.try_consultation_completion(&mut executor) else {
                        panic!("{suffix} recovery must leave local work attributable")
                    };
                    assert_eq!(late.coordinate, coordinate);
                    assert!(matches!(
                        late.outcome,
                        service::task::ResidentConsultationTaskPoll::Completed {
                            candidate: actual,
                            cancellation: ResidentOperationTaskCancellationState::Notified,
                            ..
                        } if actual == *candidate
                    ));
                }
            }

            let mut cancellation_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut cancellation_anchor = local_start::successor_anchor();
            let substituted_store = |_observation: &OperationObservationV1| {
                Ok::<_, OperationObservationAuthorityStoreRejection>(substituted_store_coordinate())
            };
            let mut rejected_stop_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut rejected_stop_anchor = local_start::successor_anchor();
            assert!(matches!(
                ResidentConsultationCoordinator::new(
                    &mut rejected_stop_custody,
                    &mut rejected_stop_anchor,
                )
                .stop(
                    &recovered_for_cancellation,
                    ConsultationStopInput {
                        cause: ConsultationStopCause::CompletionPolicy,
                        reason: immutable("stop/substituted-store", 'c'),
                        stopped_at: UnixTimeV1::new(65).unwrap(),
                        parent: &issued,
                        parent_observation: completed.observation(),
                        parent_checkpoint: completed.checkpoint(),
                        observation_ref: record_ref("observation/stop-substituted-store"),
                        observed_at: UnixTimeV1::new(65).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                        limits,
                    },
                    ConsultationSuccessorCommitInput {
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            4, 4096, 4096, 512, 65_536,
                        )
                        .unwrap(),
                        identity: OperationCheckpointInput {
                            reference: record_ref("operation/checkpoint-stop-substituted-store"),
                            id: ControlCheckpointIdV1::parse(
                                "operation-checkpoint-stop-substituted-store",
                            )
                            .unwrap(),
                        },
                        issued_at: UnixTimeV1::new(65).unwrap(),
                        schema: &checkpoint_schema,
                        authority: &cancellation_authority,
                    },
                    &substituted_store,
                    |_, _| Ok(()),
                    |_| panic!("a substituted store must fail before collection admission"),
                    |_, _, _, _| panic!("a substituted store cannot rotate authority"),
                    |_, _, _| panic!("a substituted store cannot sign an observation"),
                    |_, _, _| panic!("a substituted store cannot sign a checkpoint"),
                ),
                Err(ConsultationStopFailure::Admission(
                    ConsultationCollectionError::Association
                ))
            ));
            assert_eq!(rejected_stop_custody.physical.mutation_counts(), (0, 0));
            assert_eq!(rejected_stop_anchor.access_counts(), (0, 0));

            let mut stop_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut stop_anchor = local_start::successor_anchor();
            let stopped = ResidentConsultationCoordinator::new(&mut stop_custody, &mut stop_anchor)
                .stop(
                    &recovered_for_cancellation,
                    ConsultationStopInput {
                        cause: ConsultationStopCause::Deadline,
                        reason: immutable("stop/coordinator-deadline", 'd'),
                        stopped_at: UnixTimeV1::new(75).unwrap(),
                        parent: &issued,
                        parent_observation: completed.observation(),
                        parent_checkpoint: completed.checkpoint(),
                        observation_ref: record_ref("observation/coordinator-deadline-stop"),
                        observed_at: UnixTimeV1::new(75).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                        limits,
                    },
                    ConsultationSuccessorCommitInput {
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            4, 4096, 4096, 512, 65_536,
                        )
                        .unwrap(),
                        identity: OperationCheckpointInput {
                            reference: record_ref("operation/checkpoint-coordinator-deadline-stop"),
                            id: ControlCheckpointIdV1::parse(
                                "operation-checkpoint-coordinator-deadline-stop",
                            )
                            .unwrap(),
                        },
                        issued_at: UnixTimeV1::new(75).unwrap(),
                        schema: &checkpoint_schema,
                        authority: &cancellation_authority,
                    },
                    &local_start::StartStore(retained.session),
                    |_, summary| {
                        assert!(!summary.requirements_met());
                        assert_eq!(summary.required_outstanding().len(), 1);
                        Ok(())
                    },
                    |admitted| {
                        assert_eq!(
                            admitted.stopped().map(ConsultationCollectionStop::cause),
                            Some(ConsultationStopCause::Deadline)
                        );
                        Ok(())
                    },
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|failure| panic!("coordinator stop must commit: {failure}"));
            assert_eq!(stop_custody.physical.mutation_counts(), (1, 1));
            assert_eq!(stop_anchor.access_counts(), (1, 1));
            let stopped_collection: ConsultationCollection = ConsultationCollection::deserialize(
                pale_core::canon::parse_strict(
                    stopped.journal().append().entry().bytes().as_slice(),
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(
                stopped_collection
                    .stopped()
                    .map(ConsultationCollectionStop::cause),
                Some(ConsultationStopCause::Deadline)
            );

            let mut rejected_cancellation_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut rejected_cancellation_anchor = local_start::successor_anchor();
            assert!(matches!(
                ResidentConsultationCoordinator::new(
                    &mut rejected_cancellation_custody,
                    &mut rejected_cancellation_anchor,
                )
                .cancel(
                    &authorized,
                    &recovered_for_cancellation,
                    ConsultationCancellationInput {
                        request_ref: record_ref("request/cancel-substituted-store"),
                        observation_ref: record_ref("observation/cancel-substituted-store"),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                        limits,
                    },
                    ConsultationSuccessorCommitInput {
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            4, 4096, 4096, 512, 65_536,
                        )
                        .unwrap(),
                        identity: OperationCheckpointInput {
                            reference: record_ref("operation/checkpoint-cancel-substituted-store"),
                            id: ControlCheckpointIdV1::parse(
                                "operation-checkpoint-cancel-substituted-store",
                            )
                            .unwrap(),
                        },
                        issued_at: UnixTimeV1::new(66).unwrap(),
                        schema: &checkpoint_schema,
                        authority: &cancellation_authority,
                    },
                    &substituted_store,
                    |_, _| Ok(()),
                    |_| panic!("a substituted store must fail before collection admission"),
                    |_, _, _, _| panic!("a substituted store cannot rotate authority"),
                    |_, _, _| panic!("a substituted store cannot sign an observation"),
                    |_, _, _| panic!("a substituted store cannot sign a checkpoint"),
                ),
                Err(ConsultationCancellationFailure::Admission(
                    ConsultationCollectionError::Association
                ))
            ));
            assert_eq!(
                rejected_cancellation_custody.physical.mutation_counts(),
                (0, 0)
            );
            assert_eq!(rejected_cancellation_anchor.access_counts(), (0, 0));

            let fresh_assessment = ConsultationSlotId::parse("fresh-assessment").unwrap();
            let late_candidate = dispatch_batch
                .candidates()
                .iter()
                .find(|candidate| candidate.slot_id() == &fresh_assessment)
                .expect("late child remains in the exact frozen batch");
            let mut late_task_service = service::task::ResidentConsultationTaskService::new(
                dispatch_batch.clone(),
                service::task::ResidentConsultationTaskLimits::new(
                    std::num::NonZeroUsize::new(1).unwrap(),
                    std::num::NonZeroUsize::new(4).unwrap(),
                ),
            )
            .expect("late-evidence task service");
            let mut late_executor =
                ConsultationEvidenceExecutor::<&RecoveredOperationObservation>::default();
            let late_dispatch = late_task_service
                .retain_and_accept(
                    late_candidate,
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered_for_cancellation.clone()),
                    |candidate| Ok::<_, std::convert::Infallible>(release_child(candidate)),
                    |candidate, _| service::task::ConsultationChildRetentionOutcome::<
                        (),
                        std::convert::Infallible,
                    >::Retained {
                        child: Box::new(retain_child(candidate)),
                        work: (),
                    },
                    &mut late_executor,
                )
                .expect("the fresh child reaches retained process-local custody");

            let cancelled = ResidentConsultationCoordinator::new(
                &mut cancellation_custody,
                &mut cancellation_anchor,
            )
            .cancel(
                &authorized,
                &recovered_for_cancellation,
                ConsultationCancellationInput {
                    request_ref: record_ref("request/cancel-consultation"),
                    observation_ref: record_ref("observation/consultation-cancellation-requested"),
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: retained.signing,
                    },
                    limits,
                },
                ConsultationSuccessorCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        4, 4096, 4096, 512, 65_536,
                    )
                    .unwrap(),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/checkpoint-cancel-consultation"),
                        id: ControlCheckpointIdV1::parse(
                            "operation-checkpoint-cancel-consultation",
                        )
                        .unwrap(),
                    },
                    issued_at: UnixTimeV1::new(66).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &cancellation_authority,
                },
                &local_start::StartStore(retained.session),
                |_, summary| {
                    assert!(!summary.requirements_met());
                    assert_eq!(summary.required_outstanding().len(), 1);
                    Ok(())
                },
                |actual| {
                    assert_eq!(
                        actual.stopped().map(ConsultationCollectionStop::cause),
                        Some(ConsultationStopCause::Cancellation)
                    );
                    Ok(())
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_, _, _| Ok(signature()),
            )
            .unwrap_or_else(|failure| panic!("consultation cancellation commits: {failure}"));
            let cancellation_evidence = match cancelled.observation().1.state() {
                OperationObservationStateV1::CancellationRequested { request } => request,
                state => panic!("cancellation remains nonterminal, got {state:?}"),
            };
            assert_eq!(
                cancellation_evidence.record_ref,
                record_ref("request/cancel-consultation")
            );
            assert_eq!(
                cancelled
                    .custody()
                    .candidate()
                    .record(pale_ports::OperationCustodyRecordKind::CommonRequest)
                    .expect("signed cancellation request shares custody")
                    .coordinate()
                    .record_ref(),
                &cancellation_evidence.record_ref
            );
            let cancelled_collection: ConsultationCollection = ConsultationCollection::deserialize(
                pale_core::canon::parse_strict(
                    cancelled.journal().append().entry().bytes().as_slice(),
                )
                .unwrap(),
            )
            .unwrap();
            let stop = cancelled_collection
                .stopped()
                .expect("frozen cancellation stop");
            assert_eq!(stop.cause(), ConsultationStopCause::Cancellation);
            assert_eq!(stop.reason(), cancellation_evidence);
            assert_eq!(stop.summary().required_outstanding().len(), 1);

            let cancelled_snapshot = {
                let evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
                    cancelled.journal().frontier(),
                    &store_coordinate(),
                    cancelled.journal().operation_handle(),
                    limits.maximum_frontier_bytes,
                )
                .unwrap();
                pale_ports::OperationCollectionSnapshot::new(
                    cancelled.journal().frontier().clone(),
                    &evidence,
                    pale_ports::OperationCollectionHeadObservation::for_handle(
                        cancelled.journal().operation_handle(),
                        cancelled.journal().frontier().coordinate().clone(),
                    ),
                    cancelled.journal().append().entry().bytes().clone(),
                )
                .unwrap()
            };
            let mut cancelled_recovery = CollectionRecoveryOnly(Some(cancelled_snapshot));
            let recovered_cancelled = ConsultationCollection::recover_current(
                &issued,
                cancelled.observation().1,
                limits,
                &local_start::StartStore(retained.session),
                &mut cancelled_recovery,
                |_| Ok(()),
            )
            .unwrap()
            .expect("cancelled collection recovers from its current head");
            let cancelled_observation = RecoveredOperationObservation::fixture(
                &issued,
                (
                    cancelled.observation().0.clone(),
                    cancelled.observation().1.clone(),
                ),
                (
                    cancelled.observation().0.clone(),
                    cancelled.observation().1.clone(),
                ),
                (
                    cancelled.checkpoint().0.clone(),
                    cancelled.checkpoint().1.clone(),
                ),
                store_coordinate(),
            );
            let already_requested = AuthorizedOperationCancellation::fixture(
                &cancel_request,
                &recovered_operation,
                &cancelled_observation,
                UnixTimeV1::new(66).unwrap(),
            );
            assert!(matches!(
                ResidentConsultationCoordinator::new(
                    &mut cancellation_custody,
                    &mut cancellation_anchor,
                )
                .cancel(
                    &already_requested,
                    &recovered_cancelled,
                    ConsultationCancellationInput {
                        request_ref: record_ref("request/cancel-consultation-repeated"),
                        observation_ref: record_ref(
                            "observation/consultation-cancellation-repeated",
                        ),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                        limits,
                    },
                    ConsultationSuccessorCommitInput {
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            4, 4096, 4096, 512, 65_536,
                        )
                        .unwrap(),
                        identity: OperationCheckpointInput {
                            reference: record_ref("operation/checkpoint-cancel-repeated"),
                            id: ControlCheckpointIdV1::parse(
                                "operation-checkpoint-cancel-repeated",
                            )
                            .unwrap(),
                        },
                        issued_at: UnixTimeV1::new(66).unwrap(),
                        schema: &checkpoint_schema,
                        authority: &cancellation_authority,
                    },
                    &local_start::StartStore(retained.session),
                    |_, _| panic!("duplicate cancellation must not re-admit a stop"),
                    |_| panic!("duplicate cancellation must not prepare a collection"),
                    |_, _, _, _| panic!("duplicate cancellation must not rotate authority"),
                    |_, _, _| panic!("duplicate cancellation must not sign an observation"),
                    |_, _, _| panic!("duplicate cancellation must not sign a checkpoint"),
                ),
                Err(ConsultationCancellationFailure::Authorization(
                    OperationCancellationRequestError::Routing
                ))
            ));
            assert_eq!(
                late_task_service.note_cancellation_notification(&late_dispatch),
                crate::resident::task_owner::ResidentOperationTaskCancellationNotification::Notified
            );
            late_executor.complete(0, &recovered_child);
            let Some(service::task::ResidentConsultationTaskJoinPoll::Joined {
                candidate,
                joined: late,
                cancellation: ResidentOperationTaskCancellationState::Notified,
                ..
            }) = late_task_service.try_completion_and_join(
                &mut late_executor,
                || Ok::<_, std::convert::Infallible>(recovered_cancelled.clone()),
                |current, candidate, child| {
                    current.admit_recovered_child_terminal(
                        candidate.slot_id(),
                        child,
                        UnixTimeV1::new(67).unwrap(),
                        limits,
                        |_, evidence| {
                            assert_eq!(evidence.record_ref, child_result_ref);
                            Ok(())
                        },
                    )
                },
            )
            else {
                panic!("late terminal child remains attributable after cancellation")
            };
            assert_eq!(candidate.slot_id(), &fresh_assessment);
            let late = late.expect("late child remains admitted collection evidence");
            let mut late_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(4).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let late_prepared = late
                .prepare_observation(
                    &issued,
                    cancelled.checkpoint(),
                    cancelled.observation(),
                    ConsultationCollectionObservationInput {
                        observation_ref: record_ref(
                            "observation/consultation-cancellation-late-child",
                        ),
                        predecessor_collection: Some(ConsultationCollectionPredecessor::recovered(
                            &recovered_cancelled,
                        )),
                        observed_at: UnixTimeV1::new(67).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                    },
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut late_custody,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .expect("late retained evidence prepares a collection successor");
            assert_eq!(
                late_prepared.observation().1.state(),
                cancelled.observation().1.state()
            );
            let mut late_authority_source =
                crate::local_operation_start_fixture::checkpoint_preimage(
                    4,
                    Some((
                        cancelled.checkpoint().0.clone(),
                        cancelled
                            .checkpoint()
                            .1
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                );
            late_authority_source.checkpoint_domain_id =
                cancelled.checkpoint().1.checkpoint_domain_id().clone();
            late_authority_source.checkpoint_profile_sha256 =
                crate::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
            let late_authority = checkpoint_authority(&late_authority_source);
            let late_completed = commit_consultation_collection(
                late_prepared,
                ConsultationCollectionCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        4, 4096, 4096, 512, 65_536,
                    )
                    .unwrap(),
                    predecessor: cancelled.checkpoint(),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/checkpoint-cancel-late-child"),
                        id: ControlCheckpointIdV1::parse("operation-checkpoint-cancel-late-child")
                            .unwrap(),
                    },
                    issued_at: UnixTimeV1::new(67).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &late_authority,
                },
                &local_start::StartStore(retained.session),
                &mut late_custody,
                &mut local_start::successor_anchor(),
                |_, _, _| Ok(signature()),
            )
            .unwrap_or_else(|failure| panic!("late child evidence reaches custody: {failure}"));
            assert_eq!(
                late_completed.observation().1.state(),
                cancelled.observation().1.state()
            );
            let retained_late: ConsultationCollection = ConsultationCollection::deserialize(
                pale_core::canon::parse_strict(
                    late_completed.journal().append().entry().bytes().as_slice(),
                )
                .unwrap(),
            )
            .unwrap();
            let retained_stop = retained_late.stopped().unwrap();
            assert_eq!(retained_stop.reason(), cancellation_evidence);
            assert_eq!(retained_stop.summary(), stop.summary());
            assert_eq!(retained_late.late_outcomes().len(), 1);

            let mut join_authority_source =
                crate::local_operation_start_fixture::checkpoint_preimage(
                    3,
                    Some((
                        completed.checkpoint().0.clone(),
                        completed
                            .checkpoint()
                            .1
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                );
            join_authority_source.checkpoint_domain_id =
                completed.checkpoint().1.checkpoint_domain_id().clone();
            join_authority_source.checkpoint_profile_sha256 =
                crate::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
            let join_authority = checkpoint_authority(&join_authority_source);
            let mut join_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut join_anchor = local_start::successor_anchor();
            let mut rejected_join_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut rejected_join_anchor = local_start::successor_anchor();
            assert!(matches!(
                ResidentConsultationCoordinator::new(
                    &mut rejected_join_custody,
                    &mut rejected_join_anchor,
                )
                .join_child(
                    &recovered,
                    ConsultationChildJoinInput {
                        slot_id: &fresh_assessment,
                        child: &recovered_child,
                        recorded_at: UnixTimeV1::new(65).unwrap(),
                        parent: &issued,
                        parent_observation: completed.observation(),
                        observation_ref: record_ref("observation/join-substituted-store"),
                        observed_at: UnixTimeV1::new(65).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                        limits,
                    },
                    ConsultationSuccessorCommitInput {
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            1, 1024, 1024, 512, 65_536,
                        )
                        .unwrap(),
                        identity: OperationCheckpointInput {
                            reference: record_ref("operation/checkpoint-join-substituted-store"),
                            id: ControlCheckpointIdV1::parse(
                                "operation-checkpoint-join-substituted-store",
                            )
                            .unwrap(),
                        },
                        issued_at: UnixTimeV1::new(65).unwrap(),
                        schema: &checkpoint_schema,
                        authority: &join_authority,
                    },
                    &|_observation: &OperationObservationV1| {
                        Ok::<_, OperationObservationAuthorityStoreRejection>(
                            substituted_store_coordinate(),
                        )
                    },
                    |_, _| Ok(()),
                    |_| panic!("a substituted store must fail before collection admission"),
                    |_, _, _, _| panic!("a substituted store cannot rotate authority"),
                    |_, _, _| panic!("a substituted store cannot sign an observation"),
                    |_, _, _| panic!("a substituted store cannot sign a checkpoint"),
                ),
                Err(ConsultationChildJoinFailure::Admission(
                    ConsultationCollectionError::Association
                ))
            ));
            assert_eq!(rejected_join_custody.physical.mutation_counts(), (0, 0));
            assert_eq!(rejected_join_anchor.access_counts(), (0, 0));

            let fresh_candidate = dispatch_batch
                .candidates()
                .iter()
                .find(|candidate| candidate.slot_id() == &fresh_assessment)
                .expect("fresh child remains in the exact frozen batch");
            let remaining_candidate = dispatch_batch
                .candidates()
                .iter()
                .find(|candidate| candidate.slot_id().as_str() == "optional-perspective")
                .expect("remaining child stays in the exact frozen batch");
            let mut join_task_service = service::task::ResidentConsultationTaskService::new(
                dispatch_batch.clone(),
                service::task::ResidentConsultationTaskLimits::new(
                    std::num::NonZeroUsize::new(2).unwrap(),
                    std::num::NonZeroUsize::new(4).unwrap(),
                ),
            )
            .expect("join task service");
            let mut join_executor =
                ConsultationEvidenceExecutor::<&RecoveredOperationObservation>::default();
            assert!(join_task_service
                .retain_and_accept(
                    fresh_candidate,
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |candidate| { Ok::<_, std::convert::Infallible>(release_child(candidate)) },
                    |candidate, _| service::task::ConsultationChildRetentionOutcome::<
                        (),
                        std::convert::Infallible,
                    >::Retained {
                        child: Box::new(retain_child(candidate)),
                        work: (),
                    },
                    &mut join_executor,
                )
                .is_ok());
            assert!(join_task_service
                .retain_and_accept(
                    remaining_candidate,
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |candidate| Ok::<_, std::convert::Infallible>(release_child(candidate)),
                    |candidate, _| service::task::ConsultationChildRetentionOutcome::<
                        (),
                        std::convert::Infallible,
                    >::Retained {
                        child: Box::new(retain_child(candidate)),
                        work: (),
                    },
                    &mut join_executor,
                )
                .is_ok());
            join_executor.complete(0, &recovered_child);
            let Some(service::task::ResidentConsultationTaskJoinPoll::Joined {
                candidate,
                output,
                joined,
                ..
            }) = join_task_service.try_completion_and_join(
                &mut join_executor,
                || Ok::<_, std::convert::Infallible>(recovered.clone()),
                |current, candidate, child| {
                    assert_eq!(candidate.slot_id(), &fresh_assessment);
                    ResidentConsultationCoordinator::new(&mut join_custody, &mut join_anchor)
                        .join_child(
                            current,
                            ConsultationChildJoinInput {
                                slot_id: &fresh_assessment,
                                child,
                                recorded_at: UnixTimeV1::new(65).unwrap(),
                                parent: &issued,
                                parent_observation: completed.observation(),
                                observation_ref: record_ref(
                                    "observation/consultation-child-joined",
                                ),
                                observed_at: UnixTimeV1::new(65).unwrap(),
                                signing: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: &observation_schema,
                                    signing_authority: retained.signing,
                                },
                                limits,
                            },
                            ConsultationSuccessorCommitInput {
                                accumulator_limits: AccumulatorPreparationLimits::new(
                                    1, 1024, 1024, 512, 65_536,
                                )
                                .unwrap(),
                                identity: OperationCheckpointInput {
                                    reference: record_ref("operation/checkpoint-3"),
                                    id: ControlCheckpointIdV1::parse("operation-checkpoint-3")
                                        .unwrap(),
                                },
                                issued_at: UnixTimeV1::new(65).unwrap(),
                                schema: &checkpoint_schema,
                                authority: &join_authority,
                            },
                            &local_start::StartStore(retained.session),
                            |_, evidence| {
                                assert_eq!(evidence.record_ref, child_result_ref);
                                Ok(())
                            },
                            |actual| {
                                assert!(matches!(
                                    actual.slots()[1].disposition(),
                                    ConsultationSlotDisposition::Fulfilled { evidence, .. }
                                        if evidence.record_ref == child_result_ref
                                ));
                                Ok(())
                            },
                            |_, _, _, _| Ok(()),
                            |_, _, _| Ok(signature()),
                            |_, _, _| Ok(signature()),
                        )
                },
            )
            else {
                panic!("completed child must reach the coordinator join once")
            };
            assert_eq!(candidate, *fresh_candidate);
            assert_eq!(output.selected_head().0, recovered_child.selected_head().0);
            let joined = joined.unwrap_or_else(|failure| {
                panic!("protected child join reaches exact next custody state: {failure}")
            });
            assert_eq!(
                joined.journal().append().entry().sequence().get(),
                2,
                "one recovered child advances the private collection exactly once"
            );
            assert_eq!(joined.checkpoint().0, &record_ref("operation/checkpoint-3"));
            let joined_collection = ConsultationCollection::deserialize(
                canon::parse_strict(joined.journal().append().entry().bytes().as_slice()).unwrap(),
            )
            .unwrap();
            assert!(matches!(
                joined_collection.slots()[1].disposition(),
                ConsultationSlotDisposition::Fulfilled { evidence, recorded_at }
                    if evidence.record_ref == child_result_ref
                        && *recorded_at == UnixTimeV1::new(65).unwrap()
            ));
            let joined_frontier_evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
                joined.journal().frontier(),
                &store_coordinate(),
                joined.journal().operation_handle(),
                limits.maximum_frontier_bytes,
            )
            .expect("joined frontier remains admissible after restart");
            let joined_snapshot = pale_ports::OperationCollectionSnapshot::new(
                joined.journal().frontier().clone(),
                &joined_frontier_evidence,
                pale_ports::OperationCollectionHeadObservation::for_handle(
                    joined.journal().operation_handle(),
                    joined.journal().frontier().coordinate().clone(),
                ),
                joined.journal().append().entry().bytes().clone(),
            )
            .expect("joined roster snapshot");
            let mut joined_recovery = CollectionRecoveryOnly(Some(joined_snapshot));
            let recovered_joined = ConsultationCollection::recover_current(
                &issued,
                joined.observation().1,
                limits,
                &local_start::StartStore(retained.session),
                &mut joined_recovery,
                |_| Ok(()),
            )
            .expect("joined roster recovery")
            .expect("joined observation retains a roster");
            assert!(matches!(
                recovered_joined
                    .revalidate_child_dispatch(&dispatch_batch, UnixTimeV1::new(66).unwrap(),),
                Err(ConsultationCollectionError::Association)
            ));
            assert!(matches!(
                join_task_service.classify_after_restart(
                    fresh_candidate,
                    &FixedClock(UnixTimeV1::new(66).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered_joined.clone()),
                    |candidate| Ok::<_, std::convert::Infallible>(retain_child(candidate)),
                ),
                Err(
                    service::task::ResidentConsultationTaskRecoveryFailure::Revalidation(
                        ConsultationCollectionError::Association
                    )
                )
            ));

            join_executor.lose(0);
            let Some(service::task::ResidentConsultationTaskPoll::Lost(remaining_lost)) =
                join_task_service.try_completion(&mut join_executor)
            else {
                panic!("the remaining accepted child keeps exact lost-task custody")
            };
            assert_eq!(remaining_lost.candidate(), remaining_candidate);
            assert_eq!(join_task_service.in_flight(), 0);
            assert_eq!(
                join_task_service
                    .classify_lost(
                        &remaining_lost,
                        &FixedClock(UnixTimeV1::new(66).unwrap()),
                        || Ok::<_, std::convert::Infallible>(recovered_joined.clone()),
                        |candidate| Ok::<_, std::convert::Infallible>(retain_child(candidate)),
                    )
                    .expect("a sibling join cannot erase an already released child"),
                ProtectedConsultationChildPhase::ResumeOnly
            );

            let substituted_remaining = {
                let retained_remaining = retain_child(remaining_candidate);
                let (_, retained_observation) = retained_remaining.selected();
                let substituted_ref =
                    record_ref("observation/consult-remaining-substituted-dispatch");
                RecoveredOperationObservation::fixture_for_handle(
                    ControlRecordCoordinate::new(
                        optional_child_ref.clone(),
                        ControlRecordDigest::parse(
                            optional_child.canonical_operation_handle_sha256().as_str(),
                        )
                        .unwrap(),
                    ),
                    (substituted_ref.clone(), retained_observation.clone()),
                    (substituted_ref, retained_observation.clone()),
                    (
                        record_ref("operation/consult-remaining-substituted-checkpoint"),
                        checkpoint.clone(),
                    ),
                    store_coordinate(),
                )
            };
            assert!(matches!(
                join_task_service.classify_lost(
                    &remaining_lost,
                    &FixedClock(UnixTimeV1::new(66).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered_joined.clone()),
                    |_| Ok::<_, std::convert::Infallible>(substituted_remaining),
                ),
                Err(service::task::ResidentConsultationTaskRecoveryFailure::DispatchAssociation)
            ));

            let rolled_back_remaining = {
                let retained_remaining = retain_child(remaining_candidate);
                let (retained_ref, retained_observation) = retained_remaining.selected();
                let initial_remaining = release_child(remaining_candidate);
                let (initial_ref, initial_observation) = initial_remaining.selected();
                RecoveredOperationObservation::history_fixture_for_handle(
                    ControlRecordCoordinate::new(
                        optional_child_ref.clone(),
                        ControlRecordDigest::parse(
                            optional_child.canonical_operation_handle_sha256().as_str(),
                        )
                        .unwrap(),
                    ),
                    (retained_ref.clone(), retained_observation.clone()),
                    Some((initial_ref.clone(), initial_observation.clone())),
                    None,
                    (initial_ref.clone(), initial_observation.clone()),
                    (
                        record_ref("operation/consult-remaining-rolled-back-checkpoint"),
                        checkpoint.clone(),
                    ),
                    store_coordinate(),
                )
            };
            assert!(matches!(
                join_task_service.classify_lost(
                    &remaining_lost,
                    &FixedClock(UnixTimeV1::new(66).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered_joined.clone()),
                    |_| Ok::<_, std::convert::Infallible>(rolled_back_remaining),
                ),
                Err(service::task::ResidentConsultationTaskRecoveryFailure::DispatchRolledBack)
            ));

            let mut disagreement_authority_source =
                crate::local_operation_start_fixture::checkpoint_preimage(
                    4,
                    Some((
                        joined.checkpoint().0.clone(),
                        joined
                            .checkpoint()
                            .1
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                );
            disagreement_authority_source.checkpoint_domain_id =
                joined.checkpoint().1.checkpoint_domain_id().clone();
            disagreement_authority_source.checkpoint_profile_sha256 =
                crate::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
            let disagreement_authority = checkpoint_authority(&disagreement_authority_source);
            let mut disagreement_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(4).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let disagreed = commit_recovered_consultation_disagreement(
                &recovered_joined,
                ConsultationDisagreementCommitInput {
                    slots: vec![
                        fresh_assessment.clone(),
                        ConsultationSlotId::parse("existing-evidence").unwrap(),
                    ],
                    evidence: immutable("evidence/disagreement-committed", 'd'),
                    recorded_at: UnixTimeV1::new(66).unwrap(),
                    parent: &issued,
                    parent_observation: joined.observation(),
                    parent_checkpoint: joined.checkpoint(),
                    observation_ref: record_ref("observation/consultation-disagreement"),
                    observed_at: UnixTimeV1::new(66).unwrap(),
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: retained.signing,
                    },
                    limits,
                },
                ConsultationSuccessorCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .unwrap(),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/checkpoint-disagreement"),
                        id: ControlCheckpointIdV1::parse("operation-checkpoint-disagreement")
                            .unwrap(),
                    },
                    issued_at: UnixTimeV1::new(66).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &disagreement_authority,
                },
                &local_start::StartStore(retained.session),
                &mut disagreement_custody,
                &mut local_start::successor_anchor(),
                |slots, evidence| {
                    assert_eq!(slots.len(), 2);
                    assert_eq!(
                        evidence.record_ref,
                        record_ref("evidence/disagreement-committed")
                    );
                    Ok(())
                },
                |actual| {
                    assert_eq!(actual.disagreements().len(), 1);
                    Ok(())
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_, _, _| Ok(signature()),
            )
            .unwrap_or_else(|failure| {
                panic!("attributable disagreement reaches parent custody: {failure}")
            });
            assert_eq!(disagreed.journal().append().entry().sequence().get(), 3);
            let OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::CollectingConsultation { evidence },
            } = disagreed.observation().1.state()
            else {
                panic!("disagreement keeps the consultation in its collection phase")
            };
            assert_eq!(
                &evidence.record_ref,
                disagreed.journal().frontier().coordinate().record_ref()
            );
            assert_eq!(
                evidence.record_sha256.as_str(),
                disagreed
                    .journal()
                    .frontier()
                    .coordinate()
                    .record_sha256()
                    .as_str()
            );
            let disagreed_collection = ConsultationCollection::deserialize(
                canon::parse_strict(disagreed.journal().append().entry().bytes().as_slice())
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(disagreed_collection.disagreements().len(), 1);
            assert_eq!(
                disagreed_collection.disagreements()[0]
                    .evidence()
                    .record_ref,
                record_ref("evidence/disagreement-committed")
            );

            let mut anchor_failure_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(2).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let observation_signatures = Cell::new(0);
            let anchor_failure_prepared = collection
                .prepare_observation(
                    &issued,
                    (&record_ref("operation/checkpoint-1"), &checkpoint),
                    (initial_record.coordinate().record_ref(), &initial),
                    ConsultationCollectionObservationInput {
                        observation_ref: record_ref("observation/consultation-2"),
                        predecessor_collection: None,
                        observed_at: UnixTimeV1::new(62).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                    },
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut anchor_failure_custody,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        observation_signatures.set(observation_signatures.get() + 1);
                        Ok(signature())
                    },
                )
                .expect("second signed collection observation");
            let mut unavailable_anchor = RecordingAnchor::unavailable_after_preflight();
            let checkpoint_signatures = Cell::new(0);
            let anchor_failure = commit_consultation_collection(
                anchor_failure_prepared,
                ConsultationCollectionCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .unwrap(),
                    predecessor: (&record_ref("operation/checkpoint-1"), &checkpoint),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/checkpoint-2"),
                        id: ControlCheckpointIdV1::parse("operation-checkpoint-2").unwrap(),
                    },
                    issued_at: UnixTimeV1::new(63).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &authority,
                },
                &local_start::StartStore(retained.session),
                &mut anchor_failure_custody,
                &mut unavailable_anchor,
                |_, _, _| {
                    checkpoint_signatures.set(checkpoint_signatures.get() + 1);
                    Ok(signature())
                },
            )
            .expect_err("database custody cannot hide an unavailable anchor");

            let mut uncertain_commit_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(2).unwrap(),
                physical: RecordingStore::failing_commit(
                    pale_ports::ControlStoreError::new(pale_ports::ControlStoreErrorKind::Storage),
                    [
                        Err(pale_ports::ControlStoreError::new(
                            pale_ports::ControlStoreErrorKind::Storage,
                        )),
                        Ok(pale_ports::OperationCustodyReadBackOutcome::ExactSuccessor),
                    ],
                ),
            };
            let uncertain_commit_prepared = collection
                .prepare_observation(
                    &issued,
                    (&record_ref("operation/checkpoint-1"), &checkpoint),
                    (initial_record.coordinate().record_ref(), &initial),
                    ConsultationCollectionObservationInput {
                        observation_ref: record_ref("observation/consultation-2"),
                        predecessor_collection: None,
                        observed_at: UnixTimeV1::new(62).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                    },
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut uncertain_commit_custody,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .expect("uncertain database collection preparation");
            let mut uncertain_commit_anchor = local_start::successor_anchor();
            let uncertain_commit = commit_consultation_collection(
                uncertain_commit_prepared,
                ConsultationCollectionCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .unwrap(),
                    predecessor: (&record_ref("operation/checkpoint-1"), &checkpoint),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/checkpoint-2"),
                        id: ControlCheckpointIdV1::parse("operation-checkpoint-2").unwrap(),
                    },
                    issued_at: UnixTimeV1::new(63).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &authority,
                },
                &local_start::StartStore(retained.session),
                &mut uncertain_commit_custody,
                &mut uncertain_commit_anchor,
                |_, _, _| Ok(signature()),
            )
            .expect_err("an uncertain database commit retains the exact candidate");

            let mut conflict_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(2).unwrap(),
                physical: RecordingStore::failing_commit(
                    pale_ports::ControlStoreError::new(
                        pale_ports::ControlStoreErrorKind::CandidateConflict,
                    ),
                    [Ok(pale_ports::OperationCustodyReadBackOutcome::Predecessor)],
                ),
            };
            let conflict_prepared = collection
                .prepare_observation(
                    &issued,
                    (&record_ref("operation/checkpoint-1"), &checkpoint),
                    (initial_record.coordinate().record_ref(), &initial),
                    ConsultationCollectionObservationInput {
                        observation_ref: record_ref("observation/consultation-2"),
                        predecessor_collection: None,
                        observed_at: UnixTimeV1::new(62).unwrap(),
                        signing: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: &observation_schema,
                            signing_authority: retained.signing,
                        },
                    },
                    limits,
                    &local_start::StartStore(retained.session),
                    &mut conflict_custody,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .expect("third signed collection observation");
            let mut conflict_anchor = local_start::successor_anchor();
            let conflict = commit_consultation_collection(
                conflict_prepared,
                ConsultationCollectionCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .unwrap(),
                    predecessor: (&record_ref("operation/checkpoint-1"), &checkpoint),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/checkpoint-2"),
                        id: ControlCheckpointIdV1::parse("operation-checkpoint-2").unwrap(),
                    },
                    issued_at: UnixTimeV1::new(63).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &authority,
                },
                &local_start::StartStore(retained.session),
                &mut conflict_custody,
                &mut conflict_anchor,
                |_, _, _| Ok(signature()),
            )
            .expect_err("a competing database candidate cannot be retried blindly");
            match conflict
                .continue_custody(&mut conflict_custody, &mut conflict_anchor)
                .expect_err("candidate conflict retains the complete collection continuation")
            {
                ConsultationCollectionCommitFailure::Commit { .. } => {}
                other => panic!("unexpected stopped phase: {other:?}"),
            }
            assert_eq!(conflict_custody.physical.mutation_counts(), (1, 1));
            assert_eq!(
                conflict_anchor.access_counts(),
                (0, 1),
                "database failure cannot advance the anchor"
            );

            drop(issued);
            let uncertain_commit_completed = uncertain_commit
                .continue_custody(&mut uncertain_commit_custody, &mut uncertain_commit_anchor)
                .expect("exact read-back resumes after operation recovery leaves scope");
            assert_eq!(
                uncertain_commit_completed.checkpoint().0,
                &record_ref("operation/checkpoint-2")
            );
            assert_eq!(
                uncertain_commit_custody.physical.mutation_counts(),
                (0, 2),
                "exact-successor read-back completes custody without a duplicate append"
            );
            let anchor_completed = anchor_failure
                .continue_custody(
                    &mut anchor_failure_custody,
                    &mut local_start::successor_anchor(),
                )
                .expect("owned preparation resumes after its recovered operation leaves scope");
            assert_eq!(
                anchor_completed.checkpoint().0,
                &record_ref("operation/checkpoint-2")
            );
            assert_eq!(
                anchor_failure_custody.physical.mutation_counts(),
                (1, 1),
                "anchor recovery cannot repeat database custody"
            );
            assert_eq!(unavailable_anchor.access_counts(), (1, 1));
            assert_eq!(observation_signatures.get(), 1);
            assert_eq!(checkpoint_signatures.get(), 1);
        },
    );
}

fn prepared_journal_frontier(
    physical: &CompletedOperationObservationCustody,
) -> &pale_ports::ControlRecord {
    physical
        .candidate()
        .record(pale_ports::OperationCustodyRecordKind::CollectionFrontier)
        .expect("frontier record")
}

#[test]
fn private_coordinator_opens_recovers_and_never_releases_a_dispatched_child() {
    let (generation, profile) = local_start::source_generation();
    with_consult_start(
        &generation,
        &profile,
        "consult-coordinator",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, retained| {
            let prepared = local_start::prepared_initial(prepared.expect("Consult start"));
            let (physical, pending, checkpoint) =
                local_start::issue_initial(prepared, request, |_| {}, &std::cell::Cell::new(0))
                    .expect("initial checkpoint");
            let mut start_store = local_start::absent_start_store(request);
            let mut start_anchor = local_start::initial_anchor();
            let database = physical
                .commit(&mut start_store, &mut start_anchor, 1_048_576)
                .expect("parent database custody");
            let initial_record = database
                .candidate()
                .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                .expect("parent resolving observation")
                .clone();
            let parent = pending
                .finish(
                    database
                        .advance_anchor(&mut start_anchor)
                        .expect("parent anchor"),
                )
                .unwrap_or_else(|_| panic!("issued Consult parent"));
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = OperationObservationV1::admit_contextual_wire(
                serde_json::from_value(
                    canon::parse_strict(initial_record.canonical_record_jcs().as_slice()).unwrap(),
                )
                .unwrap(),
                &OperationObservationAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: retained.signing,
                    },
                    handle_ref: parent.handle().0,
                    handle: parent.handle().1,
                    predecessor: None,
                },
                |_| Ok(()),
                |_, _, _, _| panic!("revision one cannot rotate authority"),
                |_, _, _, _| Ok(()),
            )
            .expect("parent resolving observation admits");

            let parent_checkpoint = (record_ref("operation/checkpoint-1"), checkpoint);
            let (child, child_checkpoint) = issue_test_child(
                &parent,
                request,
                (&parent_checkpoint.0, &parent_checkpoint.1),
                &generation,
                &retained,
                OperationClassV1::Invoke,
                "invoke",
            );
            let (delegate, _) = issue_test_child(
                &parent,
                request,
                (&parent_checkpoint.0, &parent_checkpoint.1),
                &generation,
                &retained,
                OperationClassV1::Delegate,
                "delegate",
            );
            let (nested_consult, _) = issue_test_child(
                &parent,
                request,
                (&parent_checkpoint.0, &parent_checkpoint.1),
                &generation,
                &retained,
                OperationClassV1::Consult,
                "nested-consult",
            );
            assert_eq!(
                nested_consult.handle().1.operation_class(),
                &OperationClassV1::Consult
            );
            let nested_parent = nested_consult
                .handle()
                .1
                .parent_operation()
                .as_ref()
                .expect("nested Consult retains its exact parent");
            assert_eq!(
                &nested_parent.operation_handle_id,
                parent.handle().1.operation_handle_id()
            );
            assert_eq!(&nested_parent.operation_handle_ref, parent.handle().0);
            assert_eq!(
                nested_consult.handle().1.compiled_generation_sha256(),
                parent.handle().1.compiled_generation_sha256()
            );
            let limits = ConsultationCollectionLimits::new(2, 16_384, 4096).unwrap();
            let coordinate = |operation: &IssuedLocalOperation| ImmutableRecordReferenceV1 {
                record_ref: operation.handle().0.clone(),
                record_sha256: ArtifactDigest::parse(
                    operation
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .unwrap(),
            };
            let delegate_coordinate = coordinate(&delegate);
            assert!(matches!(
                ConsultationCollection::admit_initial(
                    parent.handle().0,
                    parent.handle().1,
                    ConsultationCollectionAdmission {
                        completion_policy: immutable("policy/consult", '7'),
                        opened_at: UnixTimeV1::new(60).unwrap(),
                        deadline: UnixTimeV1::new(75).unwrap(),
                        slots: vec![ConsultationRosterSlot::new(
                            ConsultationSlotId::parse("delegate-is-not-consultation").unwrap(),
                            ConsultationRequirement::Required,
                            immutable("requirement/delegate", '8'),
                            ParticipantRefV1::Application {
                                application_id: "delegate".into(),
                            },
                            None,
                            None,
                            ConsultationContributionSource::FreshChild {
                                child: delegate_coordinate,
                            },
                        )],
                    },
                    &[(delegate.handle().0, delegate.handle().1)],
                    limits,
                ),
                Err(ConsultationCollectionError::Child)
            ));

            let nested_coordinate = coordinate(&nested_consult);
            let nested_collection = ConsultationCollection::admit_initial(
                parent.handle().0,
                parent.handle().1,
                ConsultationCollectionAdmission {
                    completion_policy: immutable("policy/nested-consult", '6'),
                    opened_at: UnixTimeV1::new(60).unwrap(),
                    deadline: UnixTimeV1::new(75).unwrap(),
                    slots: vec![ConsultationRosterSlot::new(
                        ConsultationSlotId::parse("nested-consult").unwrap(),
                        ConsultationRequirement::Required,
                        immutable("requirement/nested-consult", '5'),
                        ParticipantRefV1::Application {
                            application_id: "nested-review".into(),
                        },
                        None,
                        Some(immutable("context/nested-consult", '4')),
                        ConsultationContributionSource::FreshChild {
                            child: nested_coordinate,
                        },
                    )],
                },
                &[(nested_consult.handle().0, nested_consult.handle().1)],
                limits,
            )
            .expect("actual nested Consult child enters the parent roster");
            assert_eq!(nested_collection.slots().len(), 1);

            let child_coordinate = coordinate(&child);
            let collection = ConsultationCollection::admit_initial(
                parent.handle().0,
                parent.handle().1,
                ConsultationCollectionAdmission {
                    completion_policy: immutable("policy/consult", '7'),
                    opened_at: UnixTimeV1::new(60).unwrap(),
                    deadline: UnixTimeV1::new(75).unwrap(),
                    slots: vec![ConsultationRosterSlot::new(
                        ConsultationSlotId::parse("issued-child").unwrap(),
                        ConsultationRequirement::Required,
                        immutable("requirement/issued-child", '8'),
                        ParticipantRefV1::Application {
                            application_id: "reviewer".into(),
                        },
                        None,
                        Some(immutable("context/issued-child", '9')),
                        ConsultationContributionSource::FreshChild {
                            child: child_coordinate.clone(),
                        },
                    )],
                },
                &[(child.handle().0, child.handle().1)],
                limits,
            )
            .expect("actual issued Invoke child enters the consultation roster");

            let mut authority_fields = crate::local_operation_start_fixture::checkpoint_preimage(
                child_checkpoint.domain_sequence().get() + 1,
                Some((
                    record_ref("operation/child-operation-coordinator-child-checkpoint"),
                    child_checkpoint
                        .canonical_control_checkpoint_sha256()
                        .clone(),
                )),
            );
            authority_fields.checkpoint_domain_id = child_checkpoint.checkpoint_domain_id().clone();
            authority_fields.checkpoint_profile_ref =
                child_checkpoint.checkpoint_profile_ref().clone();
            authority_fields.checkpoint_profile_sha256 =
                child_checkpoint.checkpoint_profile_sha256().clone();
            authority_fields.domain_epoch = *child_checkpoint.domain_epoch();
            let authority = checkpoint_authority(&authority_fields);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let mut custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(
                    child_checkpoint.committed_transaction_sequence().get() + 1,
                )
                .unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut anchor = local_start::successor_anchor();
            let completed = ResidentConsultationCoordinator::new(&mut custody, &mut anchor)
                .open(
                    ResidentConsultationOpenInput {
                        collection,
                        parent: &parent,
                        parent_observation: (initial_record.coordinate().record_ref(), &initial),
                        parent_checkpoint: (
                            &record_ref("operation/child-operation-coordinator-child-checkpoint"),
                            &child_checkpoint,
                        ),
                        observation: ConsultationCollectionObservationInput {
                            observation_ref: record_ref("observation/consult-coordinator-open"),
                            predecessor_collection: None,
                            observed_at: UnixTimeV1::new(62).unwrap(),
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: retained.signing,
                            },
                        },
                        limits,
                        checkpoint: ConsultationSuccessorCommitInput {
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                4, 4096, 4096, 512, 65_536,
                            )
                            .unwrap(),
                            identity: OperationCheckpointInput {
                                reference: record_ref("operation/consult-coordinator-checkpoint"),
                                id: ControlCheckpointIdV1::parse("consult-coordinator-checkpoint")
                                    .unwrap(),
                            },
                            issued_at: UnixTimeV1::new(63).unwrap(),
                            schema: &checkpoint_schema,
                            authority: &authority,
                        },
                    },
                    &local_start::StartStore(retained.session),
                    |admitted| {
                        assert_eq!(
                            admitted.completion_policy(),
                            &immutable("policy/consult", '7')
                        );
                        assert_eq!(admitted.slots().len(), 1);
                        Ok(())
                    },
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|failure| panic!("coordinator opening failed: {failure:?}"));
            assert_eq!(custody.physical.mutation_counts(), (1, 1));
            assert_eq!(anchor.access_counts(), (1, 1));

            let frontier = completed.journal().frontier();
            let frontier_evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
                frontier,
                &store_coordinate(),
                completed.journal().operation_handle(),
                limits.maximum_frontier_bytes,
            )
            .unwrap();
            let snapshot = pale_ports::OperationCollectionSnapshot::new(
                frontier.clone(),
                &frontier_evidence,
                pale_ports::OperationCollectionHeadObservation::for_handle(
                    completed.journal().operation_handle(),
                    frontier.coordinate().clone(),
                ),
                completed.journal().append().entry().bytes().clone(),
            )
            .unwrap();
            let open_observation_ref = completed.observation().0.clone();
            let open_observation = completed.observation().1.clone();
            let open_checkpoint = (
                completed.checkpoint().0.clone(),
                completed.checkpoint().1.clone(),
            );
            drop(completed);

            let protected_parent = RecoveredOperationObservation::fixture(
                &parent,
                (open_observation_ref.clone(), open_observation.clone()),
                (open_observation_ref.clone(), open_observation.clone()),
                open_checkpoint.clone(),
                store_coordinate(),
            );
            let substituted_store_parent = RecoveredOperationObservation::fixture(
                &parent,
                (open_observation_ref.clone(), open_observation.clone()),
                (open_observation_ref.clone(), open_observation.clone()),
                open_checkpoint.clone(),
                substituted_store_coordinate(),
            );
            let mut substituted_store_port = CollectionRecoveryOnly(Some(snapshot.clone()));
            let mut substituted_store_anchor = local_start::successor_anchor();
            assert!(matches!(
                ResidentConsultationCoordinator::new(
                    &mut substituted_store_port,
                    &mut substituted_store_anchor,
                )
                .recover(
                    &parent,
                    &substituted_store_parent,
                    limits,
                    &local_start::StartStore(retained.session),
                    |_| panic!("a substituted protected store cannot reach semantic admission"),
                ),
                Err(ConsultationCollectionError::RuntimeAuthority(
                    OperationObservationAuthorityStoreRejection
                ))
            ));

            let substituted_handle_parent = RecoveredOperationObservation::fixture_for_handle(
                ControlRecordCoordinate::new(
                    child.handle().0.clone(),
                    ControlRecordDigest::parse(
                        child
                            .handle()
                            .1
                            .canonical_operation_handle_sha256()
                            .as_str(),
                    )
                    .unwrap(),
                ),
                (open_observation_ref.clone(), open_observation.clone()),
                (open_observation_ref, open_observation),
                open_checkpoint.clone(),
                store_coordinate(),
            );
            let mut substituted_handle_port = CollectionRecoveryOnly(Some(snapshot.clone()));
            let mut substituted_handle_anchor = local_start::successor_anchor();
            assert!(matches!(
                ResidentConsultationCoordinator::new(
                    &mut substituted_handle_port,
                    &mut substituted_handle_anchor,
                )
                .recover(
                    &parent,
                    &substituted_handle_parent,
                    limits,
                    &local_start::StartStore(retained.session),
                    |_| panic!("a substituted protected handle cannot reach semantic admission"),
                ),
                Err(ConsultationCollectionError::Association)
            ));

            let mut rejected_generation_port = CollectionRecoveryOnly(Some(snapshot.clone()));
            let mut rejected_generation_anchor = local_start::successor_anchor();
            assert!(matches!(
                ResidentConsultationCoordinator::new(
                    &mut rejected_generation_port,
                    &mut rejected_generation_anchor,
                )
                .recover(
                    &parent,
                    &protected_parent,
                    limits,
                    &local_start::StartStore(retained.session),
                    |_| Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "private consultation collection",
                        invariant: "current compiled generation no longer admits its policy",
                    }),
                ),
                Err(ConsultationCollectionError::Semantic(
                    InvalidResidentRecordV1::ContextualAdmission { .. }
                ))
            ));

            let mut recovered_port = CollectionRecoveryOnly(Some(snapshot));
            let mut recovery_anchor = local_start::successor_anchor();
            let recovered =
                ResidentConsultationCoordinator::new(&mut recovered_port, &mut recovery_anchor)
                    .recover(
                        &parent,
                        &protected_parent,
                        limits,
                        &local_start::StartStore(retained.session),
                        |admitted| {
                            assert_eq!(
                                admitted.completion_policy(),
                                &immutable("policy/consult", '7')
                            );
                            assert_eq!(admitted.slots().len(), 1);
                            Ok(())
                        },
                    )
                    .expect("bounded cold recovery")
                    .expect("collecting observation requires a roster");
            let batch = recovered
                .plan_child_dispatch(UnixTimeV1::new(64).unwrap())
                .expect("one exact finite batch");
            assert_eq!(batch.candidates().len(), 1);

            let child_initial_ref = record_ref("observation/child-operation-classification-1");
            let child_initial = issue_observation(
                retained.signing,
                child.handle().0,
                child.handle().1,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                61,
            );
            let recovered_child =
                |reference: ResidentRecordRefV1, observation: OperationObservationV1| {
                    RecoveredOperationObservation::fixture_for_handle(
                        ControlRecordCoordinate::new(
                            child.handle().0.clone(),
                            ControlRecordDigest::parse(
                                child
                                    .handle()
                                    .1
                                    .canonical_operation_handle_sha256()
                                    .as_str(),
                            )
                            .unwrap(),
                        ),
                        (reference.clone(), observation.clone()),
                        (reference, observation),
                        (
                            record_ref("operation/child-operation-classification-checkpoint"),
                            child_checkpoint.clone(),
                        ),
                        store_coordinate(),
                    )
                };
            let mut classification_port = CollectionRecoveryOnly(None);
            let mut classification_anchor = local_start::successor_anchor();
            let coordinator = ResidentConsultationCoordinator::new(
                &mut classification_port,
                &mut classification_anchor,
            );
            assert_eq!(
                coordinator
                    .classify_child(
                        &recovered,
                        &batch,
                        &batch.candidates()[0],
                        UnixTimeV1::new(64).unwrap(),
                        &recovered_child(child_initial_ref.clone(), child_initial.clone()),
                    )
                    .unwrap(),
                ProtectedConsultationChildPhase::EligibleForFirstDispatch
            );
            let dispatched_ref = record_ref("observation/child-operation-dispatched");
            let dispatched = issue_observation(
                retained.signing,
                child.handle().0,
                child.handle().1,
                observation_schema,
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&child_initial_ref, &child_initial)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched {
                        attempt_id: CapabilityAttemptIdV1::parse("consult-child-attempt").unwrap(),
                        evidence: immutable("attempt/consult-child", 'a'),
                    },
                },
                62,
            );
            assert_eq!(
                coordinator
                    .classify_child(
                        &recovered,
                        &batch,
                        &batch.candidates()[0],
                        UnixTimeV1::new(64).unwrap(),
                        &recovered_child(dispatched_ref.clone(), dispatched.clone()),
                    )
                    .unwrap(),
                ProtectedConsultationChildPhase::ResumeOnly,
                "a child that crossed dispatch can never receive another fresh release"
            );

            let task_limits = service::task::ResidentConsultationTaskLimits::new(
                std::num::NonZeroUsize::new(1).unwrap(),
                std::num::NonZeroUsize::new(1).unwrap(),
            );
            let correct_handle = ControlRecordCoordinate::new(
                child.handle().0.clone(),
                ControlRecordDigest::parse(
                    child
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .unwrap(),
            );
            let invalid_preflight_children = [
                RecoveredOperationObservation::fixture_for_handle(
                    correct_handle.clone(),
                    (child_initial_ref.clone(), child_initial.clone()),
                    (child_initial_ref.clone(), child_initial.clone()),
                    (
                        record_ref("operation/consult-task-wrong-store-checkpoint"),
                        child_checkpoint.clone(),
                    ),
                    substituted_store_coordinate(),
                ),
                RecoveredOperationObservation::fixture_for_handle(
                    ControlRecordCoordinate::new(
                        record_ref("operation/consult-task-wrong-handle"),
                        correct_handle.record_sha256().clone(),
                    ),
                    (child_initial_ref.clone(), child_initial.clone()),
                    (child_initial_ref.clone(), child_initial.clone()),
                    (
                        record_ref("operation/consult-task-wrong-handle-checkpoint"),
                        child_checkpoint.clone(),
                    ),
                    store_coordinate(),
                ),
                RecoveredOperationObservation::fixture_for_handle(
                    ControlRecordCoordinate::new(
                        child.handle().0.clone(),
                        ControlRecordDigest::parse(sha('8')).unwrap(),
                    ),
                    (child_initial_ref.clone(), child_initial.clone()),
                    (child_initial_ref.clone(), child_initial.clone()),
                    (
                        record_ref("operation/consult-task-wrong-digest-checkpoint"),
                        child_checkpoint.clone(),
                    ),
                    store_coordinate(),
                ),
            ];
            for invalid_child in invalid_preflight_children {
                let mut invalid_service =
                    service::task::ResidentConsultationTaskService::new(batch.clone(), task_limits)
                        .expect("invalid preflight service");
                let mut invalid_executor = ConsultationTaskExecutor::default();
                assert!(matches!(
                    invalid_service.retain_and_accept(
                        &batch.candidates()[0],
                        &FixedClock(UnixTimeV1::new(64).unwrap()),
                        || Ok::<_, std::convert::Infallible>(recovered.clone()),
                        |_| Ok::<_, std::convert::Infallible>(invalid_child),
                        |_, _| -> service::task::ConsultationChildRetentionOutcome<
                            u8,
                            std::convert::Infallible,
                        > { panic!("invalid protected child cannot reach retention") },
                        &mut invalid_executor,
                    ),
                    Err(failure)
                        if matches!(
                            *failure,
                            service::task::ResidentConsultationTaskAcceptanceFailure::Release(
                                service::ConsultationChildDispatchReleaseFailure::Revalidation(
                                    ConsultationCollectionError::Child
                                )
                            )
                        )
                ));
                assert_eq!(invalid_executor.accept_calls, 0);
                assert_eq!(invalid_service.in_flight(), 0);
            }

            let mut wrong_phase_service =
                service::task::ResidentConsultationTaskService::new(batch.clone(), task_limits)
                    .expect("wrong retained phase service");
            let mut wrong_phase_executor = ConsultationTaskExecutor::default();
            assert!(matches!(
                wrong_phase_service.retain_and_accept(
                    &batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |_| {
                        Ok::<_, std::convert::Infallible>(recovered_child(
                            child_initial_ref.clone(),
                            child_initial.clone(),
                        ))
                    },
                    |_, _| service::task::ConsultationChildRetentionOutcome::<
                        u8,
                        std::convert::Infallible,
                    >::Retained {
                        child: Box::new(recovered_child(
                            child_initial_ref.clone(),
                            child_initial.clone(),
                        )),
                        work: 0,
                    },
                    &mut wrong_phase_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::RetainedPhase {
                            phase: ProtectedConsultationChildPhase::EligibleForFirstDispatch,
                            ..
                        }
                    )
            ));
            assert_eq!(wrong_phase_executor.accept_calls, 0);
            assert!(matches!(
                wrong_phase_service.retain_and_accept(
                    &batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || -> Result<_, std::convert::Infallible> {
                        panic!("wrong retained phase crossed release once")
                    },
                    |_| -> Result<_, std::convert::Infallible> {
                        panic!("wrong retained phase cannot recover twice")
                    },
                    |_, _| -> service::task::ConsultationChildRetentionOutcome<
                        u8,
                        std::convert::Infallible,
                    > { panic!("wrong retained phase cannot retain twice") },
                    &mut wrong_phase_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::AlreadyReleased(_)
                    )
            ));
            let mut task_service =
                service::task::ResidentConsultationTaskService::new(batch.clone(), task_limits)
                    .expect("finite task service");
            let mut task_executor = ConsultationTaskExecutor::default();
            let Ok(task_key) = task_service.retain_and_accept(
                &batch.candidates()[0],
                &FixedClock(UnixTimeV1::new(64).unwrap()),
                || Ok::<_, std::convert::Infallible>(recovered.clone()),
                |_| {
                    Ok::<_, std::convert::Infallible>(recovered_child(
                        child_initial_ref.clone(),
                        child_initial.clone(),
                    ))
                },
                |_, _| service::task::ConsultationChildRetentionOutcome::<
                    u8,
                    std::convert::Infallible,
                >::Retained {
                    child: Box::new(recovered_child(dispatched_ref.clone(), dispatched.clone())),
                    work: 7,
                },
                &mut task_executor,
            ) else {
                panic!("fresh durable dispatch must reach the executor once")
            };
            assert_eq!(task_executor.accept_calls, 1);
            assert_eq!(task_service.in_flight(), 1);
            let duplicate_retention_calls = std::cell::Cell::new(0);
            assert!(matches!(
                task_service.retain_and_accept(
                    &batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || -> Result<_, std::convert::Infallible> {
                        panic!("duplicate release must stop before parent lookup")
                    },
                    |_| -> Result<_, std::convert::Infallible> {
                        panic!("duplicate release must stop before child recovery")
                    },
                    |_, _| {
                        duplicate_retention_calls.set(duplicate_retention_calls.get() + 1);
                        service::task::ConsultationChildRetentionOutcome::<
                            u8,
                            std::convert::Infallible,
                        >::Retained {
                            child: Box::new(recovered_child(
                                dispatched_ref.clone(),
                                dispatched.clone(),
                            )),
                            work: 8,
                        }
                    },
                    &mut task_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::AlreadyReleased(_)
                    )
            ));
            assert_eq!(duplicate_retention_calls.get(), 0);
            assert_eq!(task_executor.accept_calls, 1);
            assert_eq!(
                task_service.note_cancellation_notification(&task_key),
                crate::resident::task_owner::ResidentOperationTaskCancellationNotification::Notified
            );
            task_executor.lose(0);
            let Some(service::task::ResidentConsultationTaskPoll::Lost(lost)) =
                task_service.try_completion(&mut task_executor)
            else {
                panic!("lost executor work must retain its exact roster basis")
            };
            assert_eq!(
                lost.cancellation_state(),
                ResidentOperationTaskCancellationState::Notified
            );
            assert_eq!(lost.candidate(), &batch.candidates()[0]);
            assert_eq!(lost.dispatch(), &task_key);
            let Ok(lost_phase) = task_service.classify_lost(
                &lost,
                &FixedClock(UnixTimeV1::new(64).unwrap()),
                || Ok::<_, std::convert::Infallible>(recovered.clone()),
                |_| {
                    Ok::<_, std::convert::Infallible>(recovered_child(
                        dispatched_ref.clone(),
                        dispatched.clone(),
                    ))
                },
            ) else {
                panic!("lost work must classify through its retained dispatch")
            };
            assert_eq!(
                lost_phase,
                ProtectedConsultationChildPhase::ResumeOnly,
                "lost work remains recovery-only and is never replayed"
            );
            assert_eq!(task_executor.accept_calls, 1);
            assert!(matches!(
                task_service.classify_lost(
                    &lost,
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |_| {
                        Ok::<_, std::convert::Infallible>(recovered_child(
                            record_ref("observation/consult-substituted-dispatch"),
                            dispatched.clone(),
                        ))
                    },
                ),
                Err(service::task::ResidentConsultationTaskRecoveryFailure::DispatchAssociation)
            ));
            let rolled_back = RecoveredOperationObservation::history_fixture_for_handle(
                correct_handle.clone(),
                (dispatched_ref.clone(), dispatched.clone()),
                Some((child_initial_ref.clone(), child_initial.clone())),
                None,
                (child_initial_ref.clone(), child_initial.clone()),
                (
                    record_ref("operation/consult-task-rolled-back-checkpoint"),
                    child_checkpoint.clone(),
                ),
                store_coordinate(),
            );
            assert!(matches!(
                task_service.classify_lost(
                    &lost,
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |_| Ok::<_, std::convert::Infallible>(rolled_back),
                ),
                Err(service::task::ResidentConsultationTaskRecoveryFailure::DispatchRolledBack)
            ));

            let mut unopened_task_service =
                service::task::ResidentConsultationTaskService::new(batch.clone(), task_limits)
                    .expect("unopened retention service");
            assert!(matches!(
                unopened_task_service.retain_and_accept(
                    &batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |_| {
                        Ok::<_, std::convert::Infallible>(recovered_child(
                            child_initial_ref.clone(),
                            child_initial.clone(),
                        ))
                    },
                    |_, _| service::task::ConsultationChildRetentionOutcome::Unopened(
                        "dispatch stayed unopened",
                    ),
                    &mut task_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::RetentionUnopened {
                            ref candidate,
                            reason: "dispatch stayed unopened",
                        } if candidate == &batch.candidates()[0]
                    )
            ));
            assert_eq!(task_executor.accept_calls, 1);

            let mut uncertain_task_service =
                service::task::ResidentConsultationTaskService::new(batch.clone(), task_limits)
                    .expect("uncertain retention service");
            assert!(matches!(
                uncertain_task_service.retain_and_accept(
                    &batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |_| {
                        Ok::<_, std::convert::Infallible>(recovered_child(
                            child_initial_ref.clone(),
                            child_initial.clone(),
                        ))
                    },
                    |_, _| service::task::ConsultationChildRetentionOutcome::Uncertain(
                        "dispatch certainty was lost",
                    ),
                    &mut task_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::RetentionUncertain {
                            ref candidate,
                            reason: "dispatch certainty was lost",
                        } if candidate == &batch.candidates()[0]
                    )
            ));
            assert_eq!(task_executor.accept_calls, 1);
            assert!(matches!(
                uncertain_task_service.retain_and_accept(
                    &batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || -> Result<_, std::convert::Infallible> {
                        panic!("uncertain dispatch must stop before parent lookup")
                    },
                    |_| -> Result<_, std::convert::Infallible> {
                        panic!("uncertain dispatch must stop before child recovery")
                    },
                    |_, _| -> service::task::ConsultationChildRetentionOutcome<
                        u8,
                        std::convert::Infallible,
                    > {
                        panic!("uncertain dispatch must not be retained a second time")
                    },
                    &mut task_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::AlreadyReleased(ref candidate)
                            if candidate == &batch.candidates()[0]
                    )
            ));

            let restarted_task_service =
                service::task::ResidentConsultationTaskService::new(batch.clone(), task_limits)
                    .expect("restarted task service");
            let Ok(restarted_phase) = restarted_task_service.classify_after_restart(
                &batch.candidates()[0],
                &FixedClock(UnixTimeV1::new(64).unwrap()),
                || Ok::<_, std::convert::Infallible>(recovered.clone()),
                |_| {
                    Ok::<_, std::convert::Infallible>(recovered_child(
                        dispatched_ref.clone(),
                        dispatched.clone(),
                    ))
                },
            ) else {
                panic!("restart must classify the protected dispatched child")
            };
            assert_eq!(restarted_phase, ProtectedConsultationChildPhase::ResumeOnly);
            let restart_retention_calls = std::cell::Cell::new(0);
            let mut restarted_task_service = restarted_task_service;
            assert!(matches!(
                restarted_task_service.retain_and_accept(
                    &batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |_| {
                        Ok::<_, std::convert::Infallible>(recovered_child(
                            dispatched_ref.clone(),
                            dispatched.clone(),
                        ))
                    },
                    |_, _| {
                        restart_retention_calls.set(restart_retention_calls.get() + 1);
                        service::task::ConsultationChildRetentionOutcome::<u8, std::convert::Infallible>::Retained {
                            child: Box::new(recovered_child(
                                dispatched_ref.clone(),
                                dispatched.clone(),
                            )),
                            work: 8,
                        }
                    },
                    &mut task_executor,
                ),
                Err(failure)
                    if matches!(
                        *failure,
                        service::task::ResidentConsultationTaskAcceptanceFailure::Release(
                            service::ConsultationChildDispatchReleaseFailure::ResumeOnly
                        )
                    )
            ));
            assert_eq!(restart_retention_calls.get(), 0);
            assert_eq!(task_executor.accept_calls, 1);

            let terminal_ref = record_ref("observation/child-operation-terminal");
            let terminal = issue_observation(
                retained.signing,
                child.handle().0,
                child.handle().1,
                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&child_initial_ref, &child_initial)),
                OperationObservationStateV1::Failed {
                    result: None,
                    diagnostic: immutable("diagnostic/child-operation", 'b'),
                },
                62,
            );
            assert_eq!(
                coordinator
                    .classify_child(
                        &recovered,
                        &batch,
                        &batch.candidates()[0],
                        UnixTimeV1::new(64).unwrap(),
                        &recovered_child(terminal_ref.clone(), terminal.clone()),
                    )
                    .unwrap(),
                ProtectedConsultationChildPhase::TerminalJoin
            );

            let mut task_state = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).unwrap(),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .expect("bounded resident task state");
            let mut terminal_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(
                    open_checkpoint.1.committed_transaction_sequence().get() + 3,
                )
                .unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut terminal_anchor = RecordingAnchor::unavailable_after_preflight();
            let mut ambient = ProvingAuthority;
            let mut terminal_executor = ConsultationTaskExecutor::default();
            let mut operation_service = ResidentOperationService::new(
                &mut ambient,
                &mut terminal_custody,
                &mut terminal_anchor,
                &mut task_state,
            );
            let task_coordinate = operation_service
                .open_consultation_batch(batch.clone(), task_limits)
                .expect("one retained Consult batch");
            operation_service
                .retain_and_accept_consultation_task(
                    &task_coordinate,
                    &batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |_| {
                        Ok::<_, std::convert::Infallible>(recovered_child(
                            child_initial_ref.clone(),
                            child_initial.clone(),
                        ))
                    },
                    |_, _| service::task::ConsultationChildRetentionOutcome::<
                        u8,
                        std::convert::Infallible,
                    >::Retained {
                        child: Box::new(recovered_child(
                            dispatched_ref.clone(),
                            dispatched.clone(),
                        )),
                        work: 15,
                    },
                    &mut terminal_executor,
                )
                .expect("fresh child reaches its only executor release");
            terminal_executor.lose(0);
            let Some(Ok(poll)) =
                operation_service.try_consultation_completion(&mut terminal_executor)
            else {
                panic!("lost work must retain its exact recovery continuation")
            };
            assert_eq!(operation_service.outstanding_task_reservations(), 1);

            let resume_only_child = RecoveredOperationObservation::history_fixture_for_handle(
                correct_handle.clone(),
                (dispatched_ref.clone(), dispatched.clone()),
                Some((child_initial_ref.clone(), child_initial.clone())),
                None,
                (dispatched_ref.clone(), dispatched.clone()),
                (
                    record_ref("operation/consult-resume-only-checkpoint"),
                    child_checkpoint.clone(),
                ),
                store_coordinate(),
            );
            let uncertain_ref = record_ref("observation/child-operation-outcome-unknown");
            let uncertain = issue_observation(
                retained.signing,
                child.handle().0,
                child.handle().1,
                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                OperationObservationRevisionV1::new(3).unwrap(),
                Some((&dispatched_ref, &dispatched)),
                OperationObservationStateV1::OutcomeUnknown {
                    diagnostic: immutable("diagnostic/child-operation-unknown", 'd'),
                    reconciliation: ReconciliationFrontierReferenceV1 {
                        record_ref: record_ref("reconciliation/child-operation-unknown"),
                        record_sha256: ReconciliationFrontierDigest::parse(sha('e')).unwrap(),
                    },
                    effect_receipt: None,
                },
                65,
            );
            let protected_uncertain = RecoveredOperationObservation::history_fixture_for_handle(
                correct_handle.clone(),
                (dispatched_ref.clone(), dispatched.clone()),
                Some((child_initial_ref.clone(), child_initial.clone())),
                None,
                (uncertain_ref, uncertain),
                (
                    record_ref("operation/consult-unknown-checkpoint"),
                    child_checkpoint.clone(),
                ),
                store_coordinate(),
            );

            for completed_locally in [true, false] {
                let mut uncertain_state = Box::new(
                    ResidentOperationTaskState::new(
                        ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                        NonZeroUsize::new(1).unwrap(),
                        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                    )
                    .expect("bounded uncertainty task state"),
                );
                let mut unused_custody = Box::new(RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ));
                let mut unused_anchor = Box::new(local_start::successor_anchor());
                let mut uncertain_ambient = ProvingAuthority;
                let mut uncertain_executor = ConsultationTaskExecutor::default();
                let poll = {
                    let mut uncertainty_service = ResidentOperationService::new(
                        &mut uncertain_ambient,
                        &mut *unused_custody,
                        &mut *unused_anchor,
                        &mut uncertain_state,
                    );
                    let coordinate = uncertainty_service
                        .open_consultation_batch(batch.clone(), task_limits)
                        .expect("uncertain provider child batch");
                    uncertainty_service
                        .retain_and_accept_consultation_task(
                            &coordinate,
                            &batch.candidates()[0],
                            &FixedClock(UnixTimeV1::new(64).unwrap()),
                            || Ok::<_, std::convert::Infallible>(recovered.clone()),
                            |_| {
                                Ok::<_, std::convert::Infallible>(recovered_child(
                                    child_initial_ref.clone(),
                                    child_initial.clone(),
                                ))
                            },
                            |_, _| service::task::ConsultationChildRetentionOutcome::<
                                u8,
                                std::convert::Infallible,
                            >::Retained {
                                child: Box::new(recovered_child(
                                    dispatched_ref.clone(),
                                    dispatched.clone(),
                                )),
                                work: 16,
                            },
                            &mut uncertain_executor,
                        )
                        .expect("uncertain provider child reaches one executor release");
                    if completed_locally {
                        uncertain_executor.complete(0, 160);
                    } else {
                        uncertain_executor.lose(0);
                    }
                    let Some(Ok(poll)) =
                        uncertainty_service.try_consultation_completion(&mut uncertain_executor)
                    else {
                        panic!("local provider return retains one settlement continuation")
                    };
                    poll
                };
                let rejected = match uncertain_state.settle_consultation_nonterminal(
                    ResidentConsultationCompletionReady::from_parts(recovered.clone(), poll),
                    &resume_only_child,
                ) {
                    Ok(_) => panic!("a dispatched head without durable uncertainty cannot release"),
                    Err(rejected) => rejected,
                };
                let ready = match rejected {
                    ResidentConsultationNonterminalSettlementFailure::Preflight {
                        ready,
                        reason:
                            service::task::ResidentConsultationTerminalJoinPreflightFailure::DispatchMismatch,
                    } => *ready,
                    other => panic!("unexpected nonterminal rejection: {other:?}"),
                };
                let settled = uncertain_state
                    .settle_consultation_nonterminal(ready, &protected_uncertain)
                    .unwrap_or_else(|failure| {
                        panic!("durable provider uncertainty releases capacity: {failure:?}")
                    });
                assert!(
                    matches!(
                        settled.outcome(),
                        service::task::ResidentConsultationTaskPoll::Completed {
                            work: 16,
                            output: 160,
                            ..
                        } if completed_locally
                    ) || matches!(
                        settled.outcome(),
                        service::task::ResidentConsultationTaskPoll::Lost(_)
                            if !completed_locally
                    )
                );
                assert_eq!(uncertain_state.outstanding_task_reservations(), 0);
                assert!(matches!(
                    recovered.collection().slots()[0].disposition(),
                    ConsultationSlotDisposition::Outstanding
                ));
            }
            let (parent_observation_ref, parent_observation) = protected_parent.selected();
            let terminal_after_dispatch_ref =
                record_ref("observation/child-operation-terminal-after-dispatch");
            let terminal_after_dispatch = issue_observation(
                retained.signing,
                child.handle().0,
                child.handle().1,
                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                OperationObservationRevisionV1::new(3).unwrap(),
                Some((&dispatched_ref, &dispatched)),
                OperationObservationStateV1::Failed {
                    result: None,
                    diagnostic: immutable("diagnostic/consult-proof-terminal", 'c'),
                },
                65,
            );
            let resolve_store = |_observation: &OperationObservationV1| {
                Ok::<_, OperationObservationAuthorityStoreRejection>(store_coordinate())
            };
            let dispatched_append = prepare_operation_observation_append(
                OperationObservationTransition::new(
                    child.handle().0,
                    child.handle().1,
                    &child_initial_ref,
                    &child_initial,
                    &dispatched_ref,
                    &dispatched,
                ),
                &resolve_store,
            )
            .expect("dispatched child observation projects");
            let (dispatched_custody, dispatched_checkpoint) =
                native_execution::complete_with_selection(
                    dispatched_append,
                    child.checkpoint_selection(),
                    (&open_checkpoint.0, &open_checkpoint.1),
                    crate::resident::OperationCustodyDatabaseEvidence::New,
                    64,
                );
            let dispatched_checkpoint_ref = dispatched_custody
                .candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref()
                .clone();
            let terminal_append = prepare_operation_observation_append(
                OperationObservationTransition::new(
                    child.handle().0,
                    child.handle().1,
                    &dispatched_ref,
                    &dispatched,
                    &terminal_after_dispatch_ref,
                    &terminal_after_dispatch,
                ),
                &resolve_store,
            )
            .expect("terminal child observation projects");
            let (terminal_child_custody, terminal_child_checkpoint) =
                native_execution::complete_with_selection(
                    terminal_append,
                    child.checkpoint_selection(),
                    (&dispatched_checkpoint_ref, &dispatched_checkpoint),
                    crate::resident::OperationCustodyDatabaseEvidence::New,
                    65,
                );
            let terminal_child_checkpoint_ref = terminal_child_custody
                .candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref()
                .clone();
            let mut terminal_authority_source =
                crate::local_operation_start_fixture::checkpoint_preimage(
                    terminal_child_checkpoint.domain_sequence().get() + 1,
                    Some((
                        terminal_child_checkpoint_ref.clone(),
                        terminal_child_checkpoint
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                );
            terminal_authority_source.checkpoint_domain_id =
                terminal_child_checkpoint.checkpoint_domain_id().clone();
            terminal_authority_source.checkpoint_profile_ref =
                terminal_child_checkpoint.checkpoint_profile_ref().clone();
            terminal_authority_source.checkpoint_profile_sha256 = terminal_child_checkpoint
                .checkpoint_profile_sha256()
                .clone();
            terminal_authority_source.domain_epoch = *terminal_child_checkpoint.domain_epoch();
            let terminal_authority = checkpoint_authority(&terminal_authority_source);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let terminal_observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);

            let resume_failure = match operation_service.join_consultation_terminal(
                ResidentConsultationCompletionReady::from_parts(recovered.clone(), poll),
                ConsultationChildJoinInput {
                    slot_id: batch.candidates()[0].slot_id(),
                    child: &resume_only_child,
                    recorded_at: UnixTimeV1::new(65).unwrap(),
                    parent: &parent,
                    parent_observation: (parent_observation_ref, parent_observation),
                    observation_ref: record_ref("observation/consult-resume-only-rejected"),
                    observed_at: UnixTimeV1::new(65).unwrap(),
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &terminal_observation_schema,
                        signing_authority: retained.signing,
                    },
                    limits,
                },
                ConsultationSuccessorCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .unwrap(),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/consult-resume-only-checkpoint"),
                        id: ControlCheckpointIdV1::parse("consult-resume-only-checkpoint").unwrap(),
                    },
                    issued_at: UnixTimeV1::new(65).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &terminal_authority,
                },
                &local_start::StartStore(retained.session),
                |_, _| panic!("resume-only work cannot reach evidence admission"),
                |_| panic!("resume-only work cannot prepare a collection"),
                |_, _, _, _| panic!("resume-only work cannot rotate authority"),
                |_, _, _| panic!("resume-only work cannot sign an observation"),
                |_, _, _| panic!("resume-only work cannot sign a checkpoint"),
            ) {
                Ok(_) => panic!("resume-only work keeps its exact recovery owner"),
                Err(failure) => failure,
            };
            let ready = match resume_failure {
                ResidentConsultationTerminalJoinFailure::Preflight {
                    ready,
                    reason:
                        service::task::ResidentConsultationTerminalJoinPreflightFailure::ChildPhase(
                            ProtectedConsultationChildPhase::ResumeOnly,
                        ),
                } => *ready,
                other => panic!("unexpected resume-only failure: {other:?}"),
            };
            assert_eq!(operation_service.outstanding_task_reservations(), 1);

            let protected_terminal = RecoveredOperationObservation::history_fixture_for_handle(
                correct_handle,
                (dispatched_ref.clone(), dispatched.clone()),
                Some((child_initial_ref, child_initial)),
                None,
                (terminal_after_dispatch_ref, terminal_after_dispatch),
                (terminal_child_checkpoint_ref, terminal_child_checkpoint),
                store_coordinate(),
            );
            let custody_failure = operation_service.join_consultation_terminal(
                ready,
                ConsultationChildJoinInput {
                    slot_id: batch.candidates()[0].slot_id(),
                    child: &protected_terminal,
                    recorded_at: UnixTimeV1::new(66).unwrap(),
                    parent: &parent,
                    parent_observation: (parent_observation_ref, parent_observation),
                    observation_ref: record_ref("observation/consult-proof-terminal-joined"),
                    observed_at: UnixTimeV1::new(66).unwrap(),
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &terminal_observation_schema,
                        signing_authority: retained.signing,
                    },
                    limits,
                },
                ConsultationSuccessorCommitInput {
                    accumulator_limits: AccumulatorPreparationLimits::new(
                        1, 1024, 1024, 512, 65_536,
                    )
                    .unwrap(),
                    identity: OperationCheckpointInput {
                        reference: record_ref("operation/consult-proof-terminal-checkpoint"),
                        id: ControlCheckpointIdV1::parse("consult-proof-terminal-checkpoint")
                            .unwrap(),
                    },
                    issued_at: UnixTimeV1::new(66).unwrap(),
                    schema: &checkpoint_schema,
                    authority: &terminal_authority,
                },
                &local_start::StartStore(retained.session),
                |slot, evidence| {
                    assert_eq!(slot.slot_id(), batch.candidates()[0].slot_id());
                    assert_eq!(
                        evidence.record_ref,
                        record_ref("diagnostic/consult-proof-terminal")
                    );
                    Ok(())
                },
                |_| Ok(()),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_, _, _| Ok(signature()),
            );
            let pending = match custody_failure {
                Err(ResidentConsultationTerminalJoinFailure::Custody(pending)) => *pending,
                Ok(_) => panic!("an unavailable anchor cannot complete parent custody"),
                Err(other) => panic!("unexpected terminal custody failure: {other:?}"),
            };
            assert_eq!(operation_service.outstanding_task_reservations(), 1);
            #[allow(
                clippy::drop_non_drop,
                reason = "the facade must release its exclusive borrows before custody inspection"
            )]
            drop(operation_service);
            assert_eq!(terminal_custody.physical.mutation_counts(), (1, 1));
            assert_eq!(terminal_anchor.access_counts(), (1, 1));

            let mut recovery_anchor = local_start::successor_anchor();
            {
                let mut operation_service = ResidentOperationService::new(
                    &mut ambient,
                    &mut terminal_custody,
                    &mut recovery_anchor,
                    &mut task_state,
                );
                let completed = operation_service
                    .continue_consultation_terminal_custody(pending)
                    .unwrap_or_else(|failure| {
                        panic!("exact retained terminal custody resumes: {failure:?}")
                    });
                assert!(matches!(
                    completed.outcome(),
                    service::task::ResidentConsultationTaskPoll::Lost(_)
                ));
                assert_eq!(
                    completed
                        .collection()
                        .journal()
                        .append()
                        .entry()
                        .sequence()
                        .get(),
                    2
                );
                assert_eq!(operation_service.outstanding_task_reservations(), 0);
            }
            assert_eq!(terminal_custody.physical.mutation_counts(), (1, 1));
            assert_eq!(recovery_anchor.access_counts(), (1, 0));
        },
    );
}

#[derive(Clone, Copy)]
enum TerminalFacadeProofCase {
    DirectRelease,
    RejectedReleaseRetry,
    StoppedLateEvidence,
    ForeignCompletion,
}

fn prove_terminal_facade_release(case: TerminalFacadeProofCase) {
    let (generation, profile) = local_start::source_generation();
    with_consult_start(
        &generation,
        &profile,
        "consult-terminal-facade",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, retained| {
            let prepared = local_start::prepared_initial(prepared.expect("Consult start"));
            let (physical, pending, checkpoint) =
                local_start::issue_initial(prepared, request, |_| {}, &std::cell::Cell::new(0))
                    .expect("initial checkpoint");
            let mut start_store = local_start::absent_start_store(request);
            let mut start_anchor = local_start::initial_anchor();
            let database = physical
                .commit(&mut start_store, &mut start_anchor, 1_048_576)
                .expect("parent database custody");
            let initial_record = database
                .candidate()
                .record(pale_ports::OperationCustodyRecordKind::OperationObservation)
                .expect("parent resolving observation")
                .clone();
            let parent = pending
                .finish(
                    database
                        .advance_anchor(&mut start_anchor)
                        .expect("parent anchor"),
                )
                .unwrap_or_else(|_| panic!("issued Consult parent"));
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            let initial = OperationObservationV1::admit_contextual_wire(
                serde_json::from_value(
                    canon::parse_strict(initial_record.canonical_record_jcs().as_slice()).unwrap(),
                )
                .unwrap(),
                &OperationObservationAdmissionContextV1 {
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: retained.signing,
                    },
                    handle_ref: parent.handle().0,
                    handle: parent.handle().1,
                    predecessor: None,
                },
                |_| Ok(()),
                |_, _, _, _| panic!("revision one cannot rotate authority"),
                |_, _, _, _| Ok(()),
            )
            .expect("parent resolving observation admits");
            let parent_checkpoint = (record_ref("operation/checkpoint-1"), checkpoint);
            let (child, child_checkpoint) = issue_test_child(
                &parent,
                request,
                (&parent_checkpoint.0, &parent_checkpoint.1),
                &generation,
                &retained,
                OperationClassV1::Invoke,
                "terminal-facade",
            );
            let limits = ConsultationCollectionLimits::new(1, 16_384, 4096).unwrap();
            let child_coordinate = ImmutableRecordReferenceV1 {
                record_ref: child.handle().0.clone(),
                record_sha256: ArtifactDigest::parse(
                    child
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .unwrap(),
            };
            let collection = ConsultationCollection::admit_initial(
                parent.handle().0,
                parent.handle().1,
                ConsultationCollectionAdmission {
                    completion_policy: immutable("policy/consult-terminal-facade", '7'),
                    opened_at: UnixTimeV1::new(60).unwrap(),
                    deadline: UnixTimeV1::new(75).unwrap(),
                    slots: vec![ConsultationRosterSlot::new(
                        ConsultationSlotId::parse("terminal-child").unwrap(),
                        ConsultationRequirement::Required,
                        immutable("requirement/terminal-child", '8'),
                        ParticipantRefV1::Application {
                            application_id: "reviewer".into(),
                        },
                        None,
                        Some(immutable("context/terminal-child", '9')),
                        ConsultationContributionSource::FreshChild {
                            child: child_coordinate,
                        },
                    )],
                },
                &[(child.handle().0, child.handle().1)],
                limits,
            )
            .expect("issued child enters its parent roster");
            let mut open_authority_source =
                crate::local_operation_start_fixture::checkpoint_preimage(
                    2,
                    Some((
                        parent_checkpoint.0.clone(),
                        parent_checkpoint
                            .1
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                );
            open_authority_source.checkpoint_domain_id =
                parent_checkpoint.1.checkpoint_domain_id().clone();
            open_authority_source.checkpoint_profile_ref =
                parent_checkpoint.1.checkpoint_profile_ref().clone();
            open_authority_source.checkpoint_profile_sha256 =
                parent_checkpoint.1.checkpoint_profile_sha256().clone();
            open_authority_source.domain_epoch = *parent_checkpoint.1.domain_epoch();
            let open_authority = checkpoint_authority(&open_authority_source);
            let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
            let mut open_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(2).unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut open_anchor = local_start::successor_anchor();
            let opened = ResidentConsultationCoordinator::new(&mut open_custody, &mut open_anchor)
                .open(
                    ResidentConsultationOpenInput {
                        collection,
                        parent: &parent,
                        parent_observation: (initial_record.coordinate().record_ref(), &initial),
                        parent_checkpoint: (&parent_checkpoint.0, &parent_checkpoint.1),
                        observation: ConsultationCollectionObservationInput {
                            observation_ref: record_ref("observation/consult-terminal-open"),
                            predecessor_collection: None,
                            observed_at: UnixTimeV1::new(62).unwrap(),
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: retained.signing,
                            },
                        },
                        limits,
                        checkpoint: ConsultationSuccessorCommitInput {
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                4, 4096, 4096, 512, 65_536,
                            )
                            .unwrap(),
                            identity: OperationCheckpointInput {
                                reference: record_ref("operation/consult-terminal-open-checkpoint"),
                                id: ControlCheckpointIdV1::parse(
                                    "consult-terminal-open-checkpoint",
                                )
                                .unwrap(),
                            },
                            issued_at: UnixTimeV1::new(63).unwrap(),
                            schema: &checkpoint_schema,
                            authority: &open_authority,
                        },
                    },
                    &local_start::StartStore(retained.session),
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|failure| panic!("collection opening failed: {failure:?}"));
            let frontier = opened.journal().frontier();
            let frontier_evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
                frontier,
                &store_coordinate(),
                opened.journal().operation_handle(),
                limits.maximum_frontier_bytes,
            )
            .unwrap();
            let snapshot = pale_ports::OperationCollectionSnapshot::new(
                frontier.clone(),
                &frontier_evidence,
                pale_ports::OperationCollectionHeadObservation::for_handle(
                    opened.journal().operation_handle(),
                    frontier.coordinate().clone(),
                ),
                opened.journal().append().entry().bytes().clone(),
            )
            .unwrap();
            let recovered = RecoveredConsultationCollection {
                collection: ConsultationCollection::deserialize(
                    canon::parse_strict(opened.journal().append().entry().bytes().as_slice())
                        .unwrap(),
                )
                .unwrap(),
                snapshot,
                store: store_coordinate(),
            };
            let open_observation = (
                opened.observation().0.clone(),
                opened.observation().1.clone(),
            );
            let open_checkpoint = (opened.checkpoint().0.clone(), opened.checkpoint().1.clone());
            let protected_parent = RecoveredOperationObservation::fixture(
                &parent,
                open_observation.clone(),
                open_observation,
                open_checkpoint.clone(),
                store_coordinate(),
            );
            let batch = recovered
                .plan_child_dispatch(UnixTimeV1::new(64).unwrap())
                .expect("one terminal child batch");

            let child_initial_ref = record_ref("observation/consult-terminal-child-1");
            let child_initial = issue_observation(
                retained.signing,
                child.handle().0,
                child.handle().1,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
                61,
            );
            let dispatched_ref = record_ref("observation/consult-terminal-child-2");
            let dispatched = issue_observation(
                retained.signing,
                child.handle().0,
                child.handle().1,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&child_initial_ref, &child_initial)),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Dispatched {
                        attempt_id: CapabilityAttemptIdV1::parse("consult-terminal-attempt")
                            .unwrap(),
                        evidence: immutable("attempt/consult-terminal", 'a'),
                    },
                },
                62,
            );
            let terminal_ref = record_ref("observation/consult-terminal-child-3");
            let terminal = issue_observation(
                retained.signing,
                child.handle().0,
                child.handle().1,
                observation_schema.clone(),
                OperationObservationRevisionV1::new(3).unwrap(),
                Some((&dispatched_ref, &dispatched)),
                OperationObservationStateV1::Succeeded {
                    result: CapabilityResultReferenceV1 {
                        record_ref: record_ref("result/consult-terminal-child"),
                        record_sha256: CapabilityResultDigest::parse(sha('f')).unwrap(),
                    },
                    effect_receipt: None,
                },
                64,
            );
            let child_coordinate = ControlRecordCoordinate::new(
                child.handle().0.clone(),
                ControlRecordDigest::parse(
                    child
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str(),
                )
                .unwrap(),
            );
            let recovered_child =
                |reference: ResidentRecordRefV1, observation: OperationObservationV1| {
                    RecoveredOperationObservation::fixture_for_handle(
                        child_coordinate.clone(),
                        (reference.clone(), observation.clone()),
                        (reference, observation),
                        (
                            record_ref("operation/consult-terminal-child-checkpoint"),
                            child_checkpoint.clone(),
                        ),
                        store_coordinate(),
                    )
                };
            let protected_terminal = RecoveredOperationObservation::history_fixture_for_handle(
                child_coordinate.clone(),
                (dispatched_ref.clone(), dispatched.clone()),
                Some((child_initial_ref.clone(), child_initial.clone())),
                None,
                (terminal_ref, terminal),
                (
                    record_ref("operation/consult-terminal-child-checkpoint"),
                    child_checkpoint.clone(),
                ),
                store_coordinate(),
            );

            let mut task_state = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(
                    if matches!(case, TerminalFacadeProofCase::ForeignCompletion) {
                        2
                    } else {
                        1
                    },
                )
                .unwrap(),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .expect("bounded task state");
            let mut join_custody = CollectionCommitPort {
                prepare: prepare_collection_accumulator,
                transaction: ControlTransactionSequenceV1::new(
                    if matches!(case, TerminalFacadeProofCase::StoppedLateEvidence) {
                        4
                    } else {
                        3
                    },
                )
                .unwrap(),
                physical: RecordingStore::committing(
                    pale_ports::OperationCustodyCommitOutcome::Committed,
                ),
            };
            let mut join_anchor = local_start::successor_anchor();
            let mut ambient = ProvingAuthority;
            let mut executor = ConsultationTaskExecutor::default();
            let mut service = ResidentOperationService::new(
                &mut ambient,
                &mut join_custody,
                &mut join_anchor,
                &mut task_state,
            );
            let coordinate = service
                .open_consultation_batch(
                    batch.clone(),
                    ResidentConsultationTaskLimits::new(
                        NonZeroUsize::new(1).unwrap(),
                        NonZeroUsize::new(1).unwrap(),
                    ),
                )
                .expect("one retained terminal batch");
            service
                .retain_and_accept_consultation_task(
                    &coordinate,
                    &batch.candidates()[0],
                    &FixedClock(UnixTimeV1::new(64).unwrap()),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    |_| {
                        Ok::<_, std::convert::Infallible>(recovered_child(
                            child_initial_ref.clone(),
                            child_initial.clone(),
                        ))
                    },
                    |_, _| service::task::ConsultationChildRetentionOutcome::<
                        u8,
                        std::convert::Infallible,
                    >::Retained {
                        child: Box::new(recovered_child(
                            dispatched_ref.clone(),
                            dispatched.clone(),
                        )),
                        work: 17,
                    },
                    &mut executor,
                )
                .expect("terminal child reaches its executor once");
            let selected_stop = if matches!(case, TerminalFacadeProofCase::StoppedLateEvidence) {
                match service.supervise_consultation_turn(
                    ResidentConsultationTaskLimits::new(
                        NonZeroUsize::new(1).unwrap(),
                        NonZeroUsize::new(1).unwrap(),
                    ),
                    UnixTimeV1::new(65).unwrap(),
                    || Ok::<_, std::convert::Infallible>(recovered.clone()),
                    &mut executor,
                    |_, _| {
                        ResidentConsultationSupervisionDecision::Stop(ConsultationStopInput {
                            cause: ConsultationStopCause::CompletionPolicy,
                            reason: immutable("policy/consult-terminal-stop", 'b'),
                            stopped_at: UnixTimeV1::new(65).unwrap(),
                            parent: &parent,
                            parent_observation: protected_parent.selected(),
                            parent_checkpoint: protected_parent.selected_checkpoint_record(),
                            observation_ref: record_ref("observation/consult-terminal-stop"),
                            observed_at: UnixTimeV1::new(65).unwrap(),
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: retained.signing,
                            },
                            limits,
                        })
                    },
                    |_| -> Result<RecoveredOperationObservation, std::convert::Infallible> {
                        panic!("a selected stop performs no child recovery")
                    },
                    |_,
                     _|
                     -> service::task::ConsultationChildRetentionOutcome<
                        u8,
                        std::convert::Infallible,
                    > { panic!("a selected stop performs no child retention") },
                ) {
                    ResidentConsultationTurnDisposition::Stopped(
                        ResidentConsultationTurnStopped::Selected(selected),
                    ) => Some(*selected),
                    _ => panic!("the facade turn must retain the selected stop phase"),
                }
            } else {
                None
            };
            #[allow(
                clippy::drop_non_drop,
                reason = "the facade must release its exclusive task-state and port borrows"
            )]
            drop(service);
            let stopped = if let Some(selected) = selected_stop {
                let mut stop_authority_source =
                    crate::local_operation_start_fixture::checkpoint_preimage(
                        3,
                        Some((
                            open_checkpoint.0.clone(),
                            open_checkpoint
                                .1
                                .canonical_control_checkpoint_sha256()
                                .clone(),
                        )),
                    );
                stop_authority_source.checkpoint_domain_id =
                    open_checkpoint.1.checkpoint_domain_id().clone();
                stop_authority_source.checkpoint_profile_ref =
                    open_checkpoint.1.checkpoint_profile_ref().clone();
                stop_authority_source.checkpoint_profile_sha256 =
                    open_checkpoint.1.checkpoint_profile_sha256().clone();
                stop_authority_source.domain_epoch = *open_checkpoint.1.domain_epoch();
                let stop_authority = checkpoint_authority(&stop_authority_source);
                let mut stop_custody = CollectionCommitPort {
                    prepare: prepare_collection_accumulator,
                    transaction: ControlTransactionSequenceV1::new(3).unwrap(),
                    physical: RecordingStore::committing(
                        pale_ports::OperationCustodyCommitOutcome::Committed,
                    ),
                };
                let mut unavailable_anchor = RecordingAnchor::unavailable_after_preflight();
                let selected = {
                    let mut stop_service = ResidentOperationService::new(
                        &mut ambient,
                        &mut stop_custody,
                        &mut unavailable_anchor,
                        &mut task_state,
                    );
                    let failure = stop_service.commit_selected_consultation_stop(
                        selected,
                        ConsultationSuccessorCommitInput {
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                1, 1024, 1024, 512, 65_536,
                            )
                            .unwrap(),
                            identity: OperationCheckpointInput {
                                reference: record_ref("operation/consult-terminal-stop-checkpoint"),
                                id: ControlCheckpointIdV1::parse(
                                    "consult-terminal-stop-checkpoint",
                                )
                                .unwrap(),
                            },
                            issued_at: UnixTimeV1::new(65).unwrap(),
                            schema: &checkpoint_schema,
                            authority: &stop_authority,
                        },
                        &local_start::StartStore(retained.session),
                        |_, _| {
                            Err(InvalidResidentRecordV1::ContextualAdmission {
                                record: "private consultation collection",
                                invariant: "test policy rejection",
                            })
                        },
                        |_| panic!("a rejected stop cannot admit the collection"),
                        |_, _, _, _| panic!("a rejected stop cannot rotate authority"),
                        |_, _, _| panic!("a rejected stop cannot sign an observation"),
                        |_, _, _| panic!("a rejected stop cannot sign a checkpoint"),
                    );
                    match failure {
                        Err(ResidentConsultationStopCommitFailure::Admission {
                            selected,
                            reason:
                                ConsultationCollectionError::Semantic(
                                    InvalidResidentRecordV1::ContextualAdmission { .. },
                                ),
                        }) => *selected,
                        Ok(_) => panic!("rejected stop admission cannot commit"),
                        Err(other) => panic!("unexpected stop admission result: {other:?}"),
                    }
                };
                assert_eq!(stop_custody.physical.mutation_counts(), (0, 0));
                assert_eq!(unavailable_anchor.access_counts(), (0, 0));
                let pending = {
                    let mut stop_service = ResidentOperationService::new(
                        &mut ambient,
                        &mut stop_custody,
                        &mut unavailable_anchor,
                        &mut task_state,
                    );
                    let failure = stop_service.commit_selected_consultation_stop(
                        selected,
                        ConsultationSuccessorCommitInput {
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                1, 1024, 1024, 512, 65_536,
                            )
                            .unwrap(),
                            identity: OperationCheckpointInput {
                                reference: record_ref("operation/consult-terminal-stop-checkpoint"),
                                id: ControlCheckpointIdV1::parse(
                                    "consult-terminal-stop-checkpoint",
                                )
                                .unwrap(),
                            },
                            issued_at: UnixTimeV1::new(65).unwrap(),
                            schema: &checkpoint_schema,
                            authority: &stop_authority,
                        },
                        &local_start::StartStore(retained.session),
                        |_, summary| {
                            assert_eq!(summary.required_outstanding().len(), 1);
                            Ok(())
                        },
                        |_| Ok(()),
                        |_, _, _, _| Ok(()),
                        |_, _, _| Ok(signature()),
                        |_, _, _| Ok(signature()),
                    );
                    match failure {
                        Err(ResidentConsultationStopCommitFailure::Custody(pending)) => *pending,
                        Ok(_) => panic!("the unavailable anchor must retain stop custody"),
                        Err(other) => panic!("unexpected selected-stop failure: {other:?}"),
                    }
                };
                assert_eq!(stop_custody.physical.mutation_counts(), (1, 1));
                assert_eq!(unavailable_anchor.access_counts(), (1, 1));
                let mut recovery_anchor = local_start::successor_anchor();
                let stopped = {
                    let mut stop_service = ResidentOperationService::new(
                        &mut ambient,
                        &mut stop_custody,
                        &mut recovery_anchor,
                        &mut task_state,
                    );
                    stop_service
                        .continue_consultation_stop_custody(pending)
                        .unwrap_or_else(|failure| {
                            panic!("the exact selected-stop custody must resume: {failure:?}")
                        })
                };
                assert_eq!(
                    stop_custody.physical.mutation_counts(),
                    (1, 1),
                    "stop anchor recovery cannot repeat database custody"
                );
                Some(stopped)
            } else {
                None
            };
            executor.complete(0, 170);
            let mut service = ResidentOperationService::new(
                &mut ambient,
                &mut join_custody,
                &mut join_anchor,
                &mut task_state,
            );
            let recovered_stopped = stopped.as_ref().map(|stopped| {
                let frontier = stopped.journal().frontier();
                let evidence = pale_ports::OperationCollectionFrontierEvidence::admit(
                    frontier,
                    &store_coordinate(),
                    stopped.journal().operation_handle(),
                    limits.maximum_frontier_bytes,
                )
                .unwrap();
                let snapshot = pale_ports::OperationCollectionSnapshot::new(
                    frontier.clone(),
                    &evidence,
                    pale_ports::OperationCollectionHeadObservation::for_handle(
                        stopped.journal().operation_handle(),
                        frontier.coordinate().clone(),
                    ),
                    stopped.journal().append().entry().bytes().clone(),
                )
                .unwrap();
                RecoveredConsultationCollection {
                    collection: ConsultationCollection::deserialize(
                        canon::parse_strict(stopped.journal().append().entry().bytes().as_slice())
                            .unwrap(),
                    )
                    .unwrap(),
                    snapshot,
                    store: store_coordinate(),
                }
            });
            let join_current = recovered_stopped.as_ref().unwrap_or(&recovered);
            let foreign_current =
                matches!(case, TerminalFacadeProofCase::ForeignCompletion).then(|| {
                    consultation_supervision_fixture(
                        "operation/consult-terminal-foreign-parent",
                        "consult-terminal-foreign-child",
                        100,
                        false,
                    )
                });
            let turn_current = foreign_current.as_ref().unwrap_or(join_current);
            let join_parent_observation = stopped
                .as_ref()
                .map_or_else(|| protected_parent.selected(), |value| value.observation());
            let join_parent_checkpoint = stopped.as_ref().map_or_else(
                || protected_parent.selected_checkpoint_record(),
                |value| value.checkpoint(),
            );
            let frozen_stop_summary = join_current
                .collection()
                .stopped()
                .map(|stop| stop.summary().clone());
            let policy_calls = std::cell::Cell::new(0);
            let ready = match service.supervise_consultation_turn(
                ResidentConsultationTaskLimits::new(
                    NonZeroUsize::new(1).unwrap(),
                    NonZeroUsize::new(1).unwrap(),
                ),
                UnixTimeV1::new(if stopped.is_some() { 66 } else { 65 }).unwrap(),
                || Ok::<_, std::convert::Infallible>(turn_current.clone()),
                &mut executor,
                |_, _| {
                    policy_calls.set(policy_calls.get() + 1);
                    ResidentConsultationSupervisionDecision::<()>::Continue(None)
                },
                |_| -> Result<RecoveredOperationObservation, std::convert::Infallible> {
                    panic!("ready completion must precede child recovery")
                },
                |_,
                 _|
                 -> service::task::ConsultationChildRetentionOutcome<
                    u8,
                    std::convert::Infallible,
                > { panic!("ready completion must precede child retention") },
            ) {
                ResidentConsultationTurnDisposition::Progress(
                    ResidentConsultationTurnProgress::CompletionReady(ready),
                ) => *ready,
                ResidentConsultationTurnDisposition::NeedsRecovery(reason)
                    if matches!(case, TerminalFacadeProofCase::ForeignCompletion) =>
                {
                    let ResidentConsultationTurnNeedsRecovery::ForeignCompletion(poll) = *reason
                    else {
                        panic!("foreign-parent turn must retain the routed completion")
                    };
                    assert_ne!(poll.coordinate.parent(), turn_current.collection().parent());
                    let owner_recoveries = std::cell::Cell::new(0);
                    let separately_recovered_owner = || {
                        owner_recoveries.set(owner_recoveries.get() + 1);
                        join_current.clone()
                    };
                    let ready = ResidentConsultationCompletionReady::from_parts(
                        separately_recovered_owner(),
                        *poll,
                    );
                    assert_eq!(owner_recoveries.get(), 1);
                    ready
                }
                _ => panic!("the facade turn must return the exact ready completion"),
            };
            assert_eq!(policy_calls.get(), 0);
            let mut join_authority_source =
                crate::local_operation_start_fixture::checkpoint_preimage(
                    if stopped.is_some() { 4 } else { 3 },
                    Some((
                        join_parent_checkpoint.0.clone(),
                        join_parent_checkpoint
                            .1
                            .canonical_control_checkpoint_sha256()
                            .clone(),
                    )),
                );
            join_authority_source.checkpoint_domain_id =
                join_parent_checkpoint.1.checkpoint_domain_id().clone();
            join_authority_source.checkpoint_profile_ref =
                join_parent_checkpoint.1.checkpoint_profile_ref().clone();
            join_authority_source.checkpoint_profile_sha256 =
                join_parent_checkpoint.1.checkpoint_profile_sha256().clone();
            join_authority_source.domain_epoch = *join_parent_checkpoint.1.domain_epoch();
            let join_authority = checkpoint_authority(&join_authority_source);
            let signs = std::cell::Cell::new(0);
            let join_input = ConsultationChildJoinInput {
                slot_id: batch.candidates()[0].slot_id(),
                child: &protected_terminal,
                recorded_at: UnixTimeV1::new(if stopped.is_some() { 66 } else { 65 }).unwrap(),
                parent: &parent,
                parent_observation: join_parent_observation,
                observation_ref: record_ref("observation/consult-terminal-joined"),
                observed_at: UnixTimeV1::new(if stopped.is_some() { 66 } else { 65 }).unwrap(),
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: retained.signing,
                },
                limits,
            };
            let successor_checkpoint = ConsultationSuccessorCommitInput {
                accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536)
                    .unwrap(),
                identity: OperationCheckpointInput {
                    reference: record_ref("operation/consult-terminal-checkpoint"),
                    id: ControlCheckpointIdV1::parse("consult-terminal-checkpoint").unwrap(),
                },
                issued_at: UnixTimeV1::new(if stopped.is_some() { 66 } else { 65 }).unwrap(),
                schema: &checkpoint_schema,
                authority: &join_authority,
            };
            crate::resident::process_host::tests::assert_consultation_terminal_association_fences(
                retained.signing,
                join_current.store(),
                &join_authority,
                &join_input,
                &successor_checkpoint,
            );
            let result = service
                .join_consultation_terminal_with_optional_release_rejection_for_test(
                    matches!(case, TerminalFacadeProofCase::RejectedReleaseRetry),
                    ready,
                    join_input,
                    successor_checkpoint,
                    &local_start::StartStore(retained.session),
                    |slot, result| {
                        assert_eq!(slot.slot_id(), batch.candidates()[0].slot_id());
                        assert_eq!(
                            result.record_ref,
                            record_ref("result/consult-terminal-child")
                        );
                        Ok(())
                    },
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        signs.set(signs.get() + 1);
                        Ok(signature())
                    },
                    |_, _, _| {
                        signs.set(signs.get() + 1);
                        Ok(signature())
                    },
                );
            match case {
                TerminalFacadeProofCase::DirectRelease => {
                    let completed = result.unwrap_or_else(|failure| {
                        panic!("durable terminal join must release exactly once: {failure:?}")
                    });
                    assert!(matches!(
                        completed.outcome(),
                        service::task::ResidentConsultationTaskPoll::Completed {
                            work: 17,
                            output: 170,
                            ..
                        }
                    ));
                    assert_eq!(service.outstanding_task_reservations(), 0);
                }
                TerminalFacadeProofCase::ForeignCompletion => {
                    let completed = result.unwrap_or_else(|failure| {
                        panic!(
                            "foreign completion must join through its recovered owner: {failure:?}"
                        )
                    });
                    assert!(matches!(
                        completed.outcome(),
                        service::task::ResidentConsultationTaskPoll::Completed {
                            work: 17,
                            output: 170,
                            ..
                        }
                    ));
                    assert_eq!(service.outstanding_task_reservations(), 0);
                    assert_eq!(join_custody.physical.mutation_counts(), (1, 1));
                    assert_eq!(join_anchor.access_counts(), (1, 1));
                    assert_eq!(signs.get(), 2);
                }
                TerminalFacadeProofCase::RejectedReleaseRetry => {
                    let joined = match result {
                        Err(ResidentConsultationTerminalJoinFailure::ReleasePreflight {
                            joined,
                            reason:
                                service::task::ResidentConsultationTerminalJoinPreflightFailure::OwnerMismatch,
                        }) => *joined,
                        Ok(_) => panic!("a replacement owner must retain joined custody"),
                        Err(other) => panic!("unexpected release rejection: {other:?}"),
                    };
                    assert_eq!(service.outstanding_task_reservations(), 1);
                    assert_eq!(signs.get(), 2);
                    let completed = service
                        .retry_consultation_terminal_release(joined)
                        .unwrap_or_else(|failure| {
                            panic!("exact joined continuation must release: {failure:?}")
                        });
                    assert!(matches!(
                        completed.outcome(),
                        service::task::ResidentConsultationTaskPoll::Completed {
                            work: 17,
                            output: 170,
                            ..
                        }
                    ));
                    assert_eq!(service.outstanding_task_reservations(), 0);
                    assert_eq!(join_custody.physical.mutation_counts(), (1, 1));
                    assert_eq!(join_anchor.access_counts(), (1, 1));
                    assert_eq!(signs.get(), 2);
                }
                TerminalFacadeProofCase::StoppedLateEvidence => {
                    let completed = result.unwrap_or_else(|failure| {
                        panic!("late terminal evidence must reach custody: {failure:?}")
                    });
                    let retained: ConsultationCollection = ConsultationCollection::deserialize(
                        canon::parse_strict(
                            completed
                                .collection()
                                .journal()
                                .append()
                                .entry()
                                .bytes()
                                .as_slice(),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                    assert_eq!(
                        retained.stopped().map(ConsultationCollectionStop::summary),
                        frozen_stop_summary.as_ref(),
                        "late evidence cannot rewrite the frozen stop summary"
                    );
                    assert_eq!(retained.late_outcomes().len(), 1);
                    assert_eq!(service.outstanding_task_reservations(), 0);
                    assert_eq!(join_custody.physical.mutation_counts(), (1, 1));
                    assert_eq!(join_anchor.access_counts(), (1, 1));
                    assert_eq!(signs.get(), 2);
                }
            }
        },
    );
}

#[test]
fn operation_facade_releases_successful_child_only_after_exact_durable_parent_join() {
    prove_terminal_facade_release(TerminalFacadeProofCase::DirectRelease);
}

#[test]
fn operation_facade_retries_only_release_after_post_custody_rejection() {
    prove_terminal_facade_release(TerminalFacadeProofCase::RejectedReleaseRetry);
}

#[test]
fn operation_facade_retains_late_terminal_evidence_without_rewriting_the_frozen_stop() {
    prove_terminal_facade_release(TerminalFacadeProofCase::StoppedLateEvidence);
}

#[test]
fn operation_facade_rebinds_foreign_completion_to_its_recovered_parent_before_join() {
    prove_terminal_facade_release(TerminalFacadeProofCase::ForeignCompletion);
}
